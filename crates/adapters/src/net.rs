//! TCP listeners and TCP entry points. Startup binding:
//! docs/architectures/serve-startup.md. Entry points:
//! docs/architectures/tcp-entry-point.md. Limits, H, and keepalive:
//! docs/architectures/connection-limits.md. The status listener first:
//! docs/architectures/status-listener.md.

use std::future::Future;
use std::io::{self, ErrorKind};
use std::sync::Arc;
use std::time::Duration;

use socket2::{SockRef, TcpKeepalive};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::task::JoinSet;
use tokio::time::Instant;
use vnc_relay_app::events::{AbortReason, ConnectionErrorKind, Event, RejectReason, Stage};
use vnc_relay_app::ports::EventSink;
use vnc_relay_app::settings::{KeepaliveSettings, ListenerPlan};
use vnc_relay_domain::allow_list::AllowList;
use vnc_relay_domain::budget::{BudgetPermit, ConnectionBudget};
use vnc_relay_domain::conn_id::ConnId;
use vnc_relay_domain::entry_point::EntryPoint;
use vnc_relay_domain::peer_ip::PeerIp;

/// Wait after a failed `accept`, so that a lasting error such as running
/// out of file descriptors does not spin the accept loop.
pub(crate) const ACCEPT_RETRY_DELAY: Duration = Duration::from_millis(100);

/// A listener bound for its plan.
#[derive(Debug)]
pub struct BoundListener {
    pub plan: ListenerPlan,
    pub listener: TcpListener,
}

/// A plan that could not be bound. `cause` is one Korean sentence without
/// the address, so the operation log can carry it (ops-info-v1 §1.11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindFailure {
    pub plan: ListenerPlan,
    pub cause: String,
}

/// Binds every plan in order. After a failure it still tries the rest so
/// that every failure is reported (cli-v1 §1.5). When any plan fails, the
/// listeners already bound are closed before returning.
pub async fn bind(plans: &[ListenerPlan]) -> Result<Vec<BoundListener>, Vec<BindFailure>> {
    let mut bound = Vec::with_capacity(plans.len());
    let mut failures = Vec::new();
    for plan in plans {
        match bind_one(*plan).await {
            Ok(listener) => bound.push(listener),
            Err(failure) => failures.push(failure),
        }
    }
    if failures.is_empty() {
        Ok(bound)
    } else {
        drop(bound);
        Err(failures)
    }
}

/// Binds one plan. The status listener is bound alone first so that it
/// answers while the others are bound (ops-info-v1 §1.12).
pub async fn bind_one(plan: ListenerPlan) -> Result<BoundListener, BindFailure> {
    match TcpListener::bind(plan.addr).await {
        Ok(listener) => Ok(BoundListener { plan, listener }),
        Err(error) => Err(BindFailure {
            plan,
            cause: bind_cause(&error),
        }),
    }
}

fn bind_cause(error: &io::Error) -> String {
    match error.kind() {
        ErrorKind::AddrInUse => "이미 사용 중이다.".to_owned(),
        ErrorKind::AddrNotAvailable => "이 호스트의 주소가 아니다.".to_owned(),
        ErrorKind::PermissionDenied => "권한이 없다.".to_owned(),
        _ => format!("{error}."),
    }
}

/// What a TCP entry point applies to each connection it accepts.
#[derive(Debug, Clone)]
pub struct AcceptPolicy {
    /// Allow list of this entry point.
    pub allow: AllowList,
    /// L, shared by every relay entry point.
    pub relay_budget: Arc<ConnectionBudget>,
    /// H, counted from the accept: a TCP connection enters the initial
    /// exchange as soon as it is accepted.
    pub handshake: Duration,
    /// K, set on every accepted relay socket.
    pub keepalive: KeepaliveSettings,
}

/// The TCP Viewer or TCP Server entry point: accepts connections, checks
/// each source IP against the entry point's allow list before reading any
/// byte (relay spec §3.1 step 1, RLY-R16), then takes a place of L.
pub struct TcpEntryPoint {
    entry_point: EntryPoint,
    listener: TcpListener,
    policy: AcceptPolicy,
}

/// An accepted connection while it waits for the next stage.
pub(crate) struct Accepted {
    pub(crate) conn_id: ConnId,
    pub(crate) entry_point: EntryPoint,
    pub(crate) peer_ip: PeerIp,
    pub(crate) at: Instant,
}

impl TcpEntryPoint {
    pub fn new(entry_point: EntryPoint, listener: TcpListener, policy: AcceptPolicy) -> Self {
        Self {
            entry_point,
            listener,
            policy,
        }
    }

    /// Accepts connections until `stop` holds true or its sender is gone.
    /// Then it closes the listener and returns once every accepted
    /// connection is closed and logged.
    pub async fn run(self, log: Arc<dyn EventSink>, stop: watch::Receiver<bool>) {
        let handshake = self.policy.handshake;
        let hold_log = Arc::clone(&log);
        accept_connections(
            self.entry_point,
            self.listener,
            self.policy,
            log,
            stop,
            move |stream, conn, permit, stop| {
                hold(stream, conn, permit, handshake, Arc::clone(&hold_log), stop)
            },
        )
        .await;
    }
}

/// The accept loop of a relay entry point, shared by the TCP and WebSocket
/// entry points. Until `stop` holds true it accepts each connection, admits
/// it by source IP and then by L, and logs a refusal or the accept. Each
/// admitted connection runs `serve` with its place of L. When `stop` holds
/// true, the listener closes and the function returns after every served
/// connection has ended.
pub(crate) async fn accept_connections<Serve, Fut>(
    entry_point: EntryPoint,
    listener: TcpListener,
    policy: AcceptPolicy,
    log: Arc<dyn EventSink>,
    mut stop: watch::Receiver<bool>,
    serve: Serve,
) where
    Serve: Fn(TcpStream, Accepted, BudgetPermit, watch::Receiver<bool>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    let mut held = JoinSet::new();
    loop {
        let accepted = tokio::select! {
            _ = stop.wait_for(|stop| *stop) => break,
            // Reaps ended holds so that their results do not pile up.
            Some(_) = held.join_next() => continue,
            accepted = listener.accept() => accepted,
        };
        let (stream, addr) = match accepted {
            Ok(accepted) => accepted,
            Err(_) => {
                // ops-info-v1 has no event for a failed accept.
                tokio::time::sleep(ACCEPT_RETRY_DELAY).await;
                continue;
            }
        };
        let peer_ip = PeerIp::from(addr);
        let permit = match admit(&policy, peer_ip) {
            Ok(permit) => permit,
            Err(reason) => {
                // Logged before the close, so a peer that sees the
                // close also finds the line.
                log.record(&Event::ConnectionRejected {
                    listener: entry_point.name(),
                    stage: Stage::Accept,
                    reason,
                    peer_ip,
                });
                drop(stream);
                continue;
            }
        };
        // A socket that cannot take the options is already broken; the
        // serve step sees that as an error and ends the connection.
        let _ = set_keepalive(&stream, &policy.keepalive);
        let conn = Accepted {
            conn_id: ConnId::next(),
            entry_point,
            peer_ip,
            at: Instant::now(),
        };
        log.record(&Event::ConnectionAccepted {
            conn_id: conn.conn_id,
            entry_point: conn.entry_point,
            peer_ip,
        });
        held.spawn(serve(stream, conn, permit, stop.clone()));
    }
    drop(listener);
    while held.join_next().await.is_some() {}
}

/// The source IP check, then a place of L (relay spec §3.1 step 1, RLY-R16).
/// A refused source neither takes a place of L nor learns that L is full.
fn admit(policy: &AcceptPolicy, peer_ip: PeerIp) -> Result<BudgetPermit, RejectReason> {
    if !policy.allow.permits(peer_ip) {
        return Err(RejectReason::IpNotAllowed);
    }
    policy
        .relay_budget
        .try_acquire()
        .ok_or(RejectReason::RelayConnectionLimit {
            limit: policy.relay_budget.limit(),
        })
}

/// Sets TCP keepalive with the idle time, probe interval, and probe count
/// of K (config-v1 §1.2 `[keepalive]`).
fn set_keepalive(stream: &TcpStream, keepalive: &KeepaliveSettings) -> io::Result<()> {
    let params = TcpKeepalive::new()
        .with_time(keepalive.tcp_idle)
        .with_interval(keepalive.tcp_interval)
        .with_retries(keepalive.tcp_probes);
    SockRef::from(stream).set_tcp_keepalive(&params)
}

/// Keeps an accepted connection unread until H passes, the peer closes it
/// first, or the server stops. The initial exchange (task 2.7) takes this
/// place once the handshake contract is adopted; until then no byte is
/// read, so no format is guessed (user decision 2026-10-20). The place of
/// L goes back before the line is logged, so a client that sees the line
/// can connect again at once.
async fn hold(
    stream: TcpStream,
    conn: Accepted,
    permit: BudgetPermit,
    handshake: Duration,
    log: Arc<dyn EventSink>,
    mut stop: watch::Receiver<bool>,
) {
    let reason = tokio::select! {
        // An error means the sender is gone, which also ends the server.
        _ = stop.wait_for(|stop| *stop) => AbortReason::ServerShutdown,
        () = tokio::time::sleep_until(conn.at + handshake) => AbortReason::HandshakeTimeout,
        reason = closed_before_data(&stream) => reason,
    };
    drop(stream);
    drop(permit);
    log.record(&Event::ConnectionAborted {
        conn_id: conn.conn_id,
        entry_point: conn.entry_point,
        stage: Stage::InitialExchange,
        reason,
        peer_ip: conn.peer_ip,
        elapsed_ms: u64::try_from(conn.at.elapsed().as_millis()).unwrap_or(u64::MAX),
    });
}

/// Resolves when the peer closes or breaks the connection before it sends
/// a byte. `peek` leaves bytes in the socket for the initial exchange. Once
/// a byte waits, this never resolves: only H or the stop ends the hold.
async fn closed_before_data(stream: &TcpStream) -> AbortReason {
    let mut byte = [0; 1];
    match stream.peek(&mut byte).await {
        Ok(0) => AbortReason::PeerEof,
        Ok(_) => std::future::pending().await,
        Err(_) => AbortReason::ConnectionError(ConnectionErrorKind::IoError),
    }
}

// Reading the options back needs getters that socket2 offers on Linux but
// not on Windows.
#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sets_keepalive_idle_interval_and_probes_from_the_settings() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|error| panic!("bind: {error}"));
        let addr = listener
            .local_addr()
            .unwrap_or_else(|error| panic!("local addr: {error}"));
        let _client = TcpStream::connect(addr)
            .await
            .unwrap_or_else(|error| panic!("connect: {error}"));
        let (server, _) = listener
            .accept()
            .await
            .unwrap_or_else(|error| panic!("accept: {error}"));
        let settings = KeepaliveSettings {
            tcp_idle: Duration::from_secs(70),
            tcp_interval: Duration::from_secs(11),
            tcp_probes: 7,
            ws_ping: Duration::from_secs(30),
            ws_pong_timeout: Duration::from_secs(30),
        };
        set_keepalive(&server, &settings).unwrap_or_else(|error| panic!("set: {error}"));

        let socket = SockRef::from(&server);
        fn got<T>(result: io::Result<T>) -> T {
            result.unwrap_or_else(|error| panic!("get: {error}"))
        }
        assert!(got(socket.keepalive()));
        assert_eq!(got(socket.tcp_keepalive_time()), Duration::from_secs(70));
        assert_eq!(
            got(socket.tcp_keepalive_interval()),
            Duration::from_secs(11)
        );
        assert_eq!(got(socket.tcp_keepalive_retries()), 7);
    }
}
