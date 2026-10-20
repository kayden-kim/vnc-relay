//! TCP listeners and TCP entry points. Startup binding:
//! docs/architectures/serve-startup.md. Entry points:
//! docs/architectures/tcp-entry-point.md.

use std::io::{self, ErrorKind};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::task::JoinSet;
use vnc_relay_app::events::{AbortReason, Event, RejectReason, Stage};
use vnc_relay_app::ports::EventSink;
use vnc_relay_app::settings::ListenerPlan;
use vnc_relay_domain::allow_list::AllowList;
use vnc_relay_domain::conn_id::ConnId;
use vnc_relay_domain::entry_point::EntryPoint;
use vnc_relay_domain::peer_ip::PeerIp;

/// Wait after a failed `accept`, so that a lasting error such as running
/// out of file descriptors does not spin the accept loop.
const ACCEPT_RETRY_DELAY: Duration = Duration::from_millis(100);

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
        match TcpListener::bind(plan.addr).await {
            Ok(listener) => bound.push(BoundListener {
                plan: *plan,
                listener,
            }),
            Err(error) => failures.push(BindFailure {
                plan: *plan,
                cause: bind_cause(&error),
            }),
        }
    }
    if failures.is_empty() {
        Ok(bound)
    } else {
        drop(bound);
        Err(failures)
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

/// The TCP Viewer or TCP Server entry point: accepts connections and checks
/// each source IP against the entry point's allow list before reading any
/// byte (relay spec §3.1 step 1, RLY-R16).
pub struct TcpEntryPoint {
    entry_point: EntryPoint,
    listener: TcpListener,
    allow: AllowList,
}

/// An accepted connection while it waits for the next stage.
struct Accepted {
    conn_id: ConnId,
    entry_point: EntryPoint,
    peer_ip: PeerIp,
    at: Instant,
}

impl TcpEntryPoint {
    pub fn new(entry_point: EntryPoint, listener: TcpListener, allow: AllowList) -> Self {
        Self {
            entry_point,
            listener,
            allow,
        }
    }

    /// Accepts connections until `stop` holds true or its sender is gone.
    /// Then it closes the listener and returns once every accepted
    /// connection is closed and logged.
    pub async fn run(self, log: Arc<dyn EventSink>, mut stop: watch::Receiver<bool>) {
        let mut held = JoinSet::new();
        loop {
            let accepted = tokio::select! {
                _ = stop.wait_for(|stop| *stop) => break,
                accepted = self.listener.accept() => accepted,
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
            if !self.allow.permits(peer_ip) {
                // Logged before the close, so a peer that sees the close
                // also finds the line.
                log.record(&Event::ConnectionRejected {
                    entry_point: self.entry_point,
                    stage: Stage::Accept,
                    reason: RejectReason::IpNotAllowed,
                    peer_ip,
                });
                drop(stream);
                continue;
            }
            let conn = Accepted {
                conn_id: ConnId::next(),
                entry_point: self.entry_point,
                peer_ip,
                at: Instant::now(),
            };
            log.record(&Event::ConnectionAccepted {
                conn_id: conn.conn_id,
                entry_point: conn.entry_point,
                peer_ip,
            });
            held.spawn(hold(stream, conn, Arc::clone(&log), stop.clone()));
        }
        drop(self.listener);
        while held.join_next().await.is_some() {}
    }
}

/// Keeps an accepted connection unread until the server stops, then closes
/// it. The initial exchange (task 2.7) takes this place once the handshake
/// contract is adopted; until then no byte is read, so no format is guessed
/// (user decision 2026-10-20).
async fn hold(
    stream: TcpStream,
    conn: Accepted,
    log: Arc<dyn EventSink>,
    mut stop: watch::Receiver<bool>,
) {
    // An error means the sender is gone, which also ends the server.
    let _ = stop.wait_for(|stop| *stop).await;
    drop(stream);
    log.record(&Event::ConnectionAborted {
        conn_id: conn.conn_id,
        entry_point: conn.entry_point,
        stage: Stage::InitialExchange,
        reason: AbortReason::ServerShutdown,
        peer_ip: conn.peer_ip,
        elapsed_ms: u64::try_from(conn.at.elapsed().as_millis()).unwrap_or(u64::MAX),
    });
}
