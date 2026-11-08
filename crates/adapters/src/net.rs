//! TCP listeners and TCP entry points. Startup binding:
//! docs/architectures/serve-startup.md. Entry points:
//! docs/architectures/tcp-entry-point.md. Limits, H, and keepalive:
//! docs/architectures/connection-limits.md. The status listener first:
//! docs/architectures/status-listener.md.

use std::fmt;
use std::future::Future;
use std::io::{self, ErrorKind};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use socket2::{SockRef, TcpKeepalive};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::task::JoinSet;
use tokio::time::Instant;
use tokio_rustls::TlsAcceptor;
use vnc_relay_app::events::{AbortReason, ConnectionErrorKind, Event, RejectReason, Stage};
use vnc_relay_app::ports::{Clock, EventSink};
use vnc_relay_app::relay::{ConnectionContext, RelayRuntime, RelayService};
use vnc_relay_app::reload::ports::RuntimeSettingsSource;
use vnc_relay_app::reload::snapshot::ReloadCell;
use vnc_relay_app::settings::{KeepaliveSettings, ListenerPlan, Settings};
use vnc_relay_domain::budget::{BudgetPermit, ConnectionBudget};
use vnc_relay_domain::conn_id::ConnId;
use vnc_relay_domain::entry_point::EntryPoint;
use vnc_relay_domain::peer_ip::PeerIp;

use crate::target::TcpTargetConnector;
/// TCP payload stream used after admission or an exact-address target connect.
/// Terminal close intents shut down writes; the owner then drops the socket
/// before returning its connection permit (relay-session.md §§3.2, 6).
#[derive(Debug)]
pub struct TcpRelayStream {
    inner: TcpStream,
}

impl TcpRelayStream {
    /// Wraps a socket whose entry point has already applied K.
    pub fn new(stream: TcpStream) -> Self {
        Self { inner: stream }
    }

    /// Applies K before exposing a newly connected target socket.
    /// On failure the owned socket is dropped before returning the error.
    pub fn with_keepalive(stream: TcpStream, keepalive: &KeepaliveSettings) -> io::Result<Self> {
        set_keepalive(&stream, keepalive)?;
        Ok(Self::new(stream))
    }

    /// Returns ownership of the socket, without splitting or cloning it.
    pub fn into_inner(self) -> TcpStream {
        self.inner
    }
}

/// Socket ETIMEDOUT, EHOSTUNREACH, or ENETUNREACH means keepalive or retransmission
/// gave up on an unresponsive peer. Application H/C/W timers never pass through this mapper.
/// Windows remains unclassified until its socket failure is observed (Q86).
pub(crate) fn classify_socket_error(error: io::Error) -> io::Error {
    if cfg!(target_os = "linux")
        && matches!(
            error.kind(),
            ErrorKind::TimedOut | ErrorKind::HostUnreachable | ErrorKind::NetworkUnreachable
        )
    {
        vnc_relay_app::ports::RelayStreamError::DeadConnection.into_io_error()
    } else {
        error
    }
}

#[cfg(all(test, target_os = "linux"))]
mod socket_error_tests {
    use super::classify_socket_error;
    use std::io;
    use std::task::Poll;
    use vnc_relay_app::events::ConnectionErrorKind;
    use vnc_relay_app::ports::RelayStreamError;

    #[test]
    fn classifies_only_linux_socket_timeouts_as_dead_connections() {
        for kind in [
            io::ErrorKind::TimedOut,
            io::ErrorKind::HostUnreachable,
            io::ErrorKind::NetworkUnreachable,
            io::ErrorKind::ConnectionReset,
            io::ErrorKind::BrokenPipe,
        ] {
            // Inject the same poll result produced by the underlying TCP socket.
            let result: Poll<io::Result<()>> = Poll::Ready(Err(io::Error::from(kind)));
            let error = match result.map_err(classify_socket_error) {
                Poll::Ready(Err(error)) => error,
                other => panic!("injected socket error disappeared: {other:?}"),
            };
            let is_dead = matches!(
                kind,
                io::ErrorKind::TimedOut
                    | io::ErrorKind::HostUnreachable
                    | io::ErrorKind::NetworkUnreachable
            );
            assert_eq!(RelayStreamError::is_dead_connection(&error), is_dead);
            assert_eq!(
                RelayStreamError::classify(&error),
                ConnectionErrorKind::IoError
            );
            if !is_dead {
                assert_eq!(error.kind(), kind);
            }
        }
    }
}

impl tokio::io::AsyncRead for TcpRelayStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buffer: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        tokio::io::AsyncRead::poll_read(std::pin::Pin::new(&mut self.get_mut().inner), cx, buffer)
            .map_err(classify_socket_error)
    }
}

impl tokio::io::AsyncWrite for TcpRelayStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bytes: &[u8],
    ) -> std::task::Poll<io::Result<usize>> {
        tokio::io::AsyncWrite::poll_write(std::pin::Pin::new(&mut self.get_mut().inner), cx, bytes)
            .map_err(classify_socket_error)
    }

    fn poll_write_vectored(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buffers: &[io::IoSlice<'_>],
    ) -> std::task::Poll<io::Result<usize>> {
        tokio::io::AsyncWrite::poll_write_vectored(
            std::pin::Pin::new(&mut self.get_mut().inner),
            cx,
            buffers,
        )
        .map_err(classify_socket_error)
    }

    fn is_write_vectored(&self) -> bool {
        tokio::io::AsyncWrite::is_write_vectored(&self.inner)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        tokio::io::AsyncWrite::poll_flush(std::pin::Pin::new(&mut self.get_mut().inner), cx)
            .map_err(classify_socket_error)
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        tokio::io::AsyncWrite::poll_shutdown(std::pin::Pin::new(&mut self.get_mut().inner), cx)
            .map_err(classify_socket_error)
    }
}

impl vnc_relay_app::ports::RelayStream for TcpRelayStream {
    fn transport(&self) -> vnc_relay_domain::session::ViewerTransport {
        vnc_relay_domain::session::ViewerTransport::Tcp
    }

    fn pending_write_len(&self) -> usize {
        0
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        _action: vnc_relay_app::ports::RelayCloseAction,
    ) -> std::task::Poll<io::Result<()>> {
        tokio::io::AsyncWrite::poll_shutdown(self, cx)
    }
}

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
    /// Allow list of this entry point, read from the live snapshot at each check.
    pub allow: AllowSource,
    /// L, shared by every relay entry point.
    pub relay_budget: Arc<ConnectionBudget>,
}

/// Where an entry point takes its allow list. Each check reads the list from
/// the live snapshot, so a reload applies to new connections and to the
/// pre-upgrade IP recheck (file-watch.md §4).
#[derive(Clone)]
pub struct AllowSource {
    runtime: Arc<dyn RuntimeSettingsSource>,
    entry_point: EntryPoint,
}

impl AllowSource {
    /// The `tcp_viewer` allow list of the snapshot.
    pub fn tcp_viewer(cell: Arc<ReloadCell<TlsAcceptor>>) -> Self {
        Self::new(cell, EntryPoint::TcpViewer)
    }

    /// The `tcp_server` allow list of the snapshot.
    pub fn tcp_server(cell: Arc<ReloadCell<TlsAcceptor>>) -> Self {
        Self::new(cell, EntryPoint::TcpServer)
    }

    /// The `ws_viewer` allow list of the snapshot.
    pub fn ws_viewer(cell: Arc<ReloadCell<TlsAcceptor>>) -> Self {
        Self::new(cell, EntryPoint::WsViewer)
    }

    /// The `wss_viewer` allow list of the snapshot.
    pub fn wss_viewer(cell: Arc<ReloadCell<TlsAcceptor>>) -> Self {
        Self::new(cell, EntryPoint::WssViewer)
    }

    fn new(cell: Arc<ReloadCell<TlsAcceptor>>, entry_point: EntryPoint) -> Self {
        Self {
            runtime: cell,
            entry_point,
        }
    }

    /// Whether the allow list current now permits `peer_ip`.
    pub fn permits(&self, peer_ip: PeerIp) -> bool {
        let mut permitted = false;
        self.runtime.with_current(&mut |settings| {
            permitted = self.permits_in(settings, peer_ip);
        });
        permitted
    }

    pub(crate) fn permits_in(&self, settings: &Settings, peer_ip: PeerIp) -> bool {
        settings
            .relay
            .get(self.entry_point)
            .allow
            .as_ref()
            .is_some_and(|allow| allow.permits(peer_ip))
    }

    pub(crate) fn entry_point(&self) -> EntryPoint {
        self.entry_point
    }

    pub(crate) fn with_current(&self, read: &mut dyn FnMut(&Settings)) {
        self.runtime.with_current(read);
    }
}

impl fmt::Debug for AllowSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AllowSource").finish_non_exhaustive()
    }
}

/// The TCP Viewer or TCP Server entry point: accepts connections, checks
/// each source IP against the entry point's allow list before reading any
/// byte (relay spec §3.1 step 1, RLY-R16), then takes a place of L.
pub struct TcpEntryPoint<L = TcpListener> {
    entry_point: EntryPoint,
    listener: L,
    policy: AcceptPolicy,
    service: Arc<RelayService>,
    runtime: Arc<RelayRuntime<TcpTargetConnector>>,
    clock: Arc<dyn Clock>,
}

/// An accepted connection while it waits for the next stage.
pub(crate) struct Accepted {
    pub(crate) conn_id: ConnId,
    pub(crate) entry_point: EntryPoint,
    pub(crate) peer_ip: PeerIp,
    pub(crate) at: Instant,
    pub(crate) peer_addr: SocketAddr,
    pub(crate) accepted_wall: SystemTime,
    pub(crate) settings: AcceptedSettings,
}

/// Values fixed at TCP accept, including B even when WS upgrades much later.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AcceptedSettings {
    pub(crate) buffer_bytes: usize,
    pub(crate) handshake: Duration,
    pub(crate) upgrade: Duration,
    pub(crate) keepalive: KeepaliveSettings,
}

impl AcceptedSettings {
    pub(crate) fn from_settings(settings: &Settings) -> Self {
        Self {
            buffer_bytes: settings.limits.buffer_bytes as usize,
            handshake: settings.timeouts.handshake,
            upgrade: settings.timeouts.tls_upgrade,
            keepalive: settings.keepalive,
        }
    }
}
impl<L: crate::listener::Listener> TcpEntryPoint<L> {
    pub fn new(
        entry_point: EntryPoint,
        listener: L,
        policy: AcceptPolicy,
        service: Arc<RelayService>,
        runtime: Arc<RelayRuntime<TcpTargetConnector>>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            entry_point,
            listener,
            policy,
            service,
            runtime,
            clock,
        }
    }

    /// Accepts connections until `stop` holds true or its sender is gone.
    /// Then it closes the listener and returns once every accepted
    /// connection is closed and logged.
    pub async fn run(self, log: Arc<dyn EventSink>, stop: watch::Receiver<bool>) {
        let service = self.service;
        let runtime = self.runtime;
        let clock = self.clock;
        accept_connections(
            self.entry_point,
            self.listener,
            self.policy,
            log,
            stop,
            move |stream, conn, permit, _stop| {
                let service = Arc::clone(&service);
                let runtime = Arc::clone(&runtime);
                let clock = Arc::clone(&clock);
                let elapsed = conn.at.elapsed();
                let accepted_at = clock.monotonic().saturating_sub(elapsed);
                let exchange_at = accepted_at;
                let context = ConnectionContext {
                    conn_id: conn.conn_id,
                    entry_point: conn.entry_point,
                    peer_ip: conn.peer_ip,
                    peer_addr: conn.peer_addr,
                    accepted_wall: conn.accepted_wall,
                    accepted_at,
                    exchange_at,
                    buffer_bytes: conn.settings.buffer_bytes,
                    handshake: conn.settings.handshake,
                };
                async move {
                    let stream = TcpRelayStream::new(stream);
                    let lifecycle = runtime.lifecycle();
                    let _handshaking = runtime.track_handshaking();
                    let prepared = service
                        .handle_connection(stream, context, permit, lifecycle)
                        .await;
                    drop(_handshaking);
                    if let Ok(prepared) = prepared {
                        let _ = service.dispatch(prepared, &runtime).await;
                    }
                }
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
pub(crate) async fn accept_connections<L: crate::listener::Listener, Serve, Fut>(
    entry_point: EntryPoint,
    listener: L,
    policy: AcceptPolicy,
    log: Arc<dyn EventSink>,
    stop: watch::Receiver<bool>,
    serve: Serve,
) where
    Serve: Fn(TcpStream, Accepted, BudgetPermit, watch::Receiver<bool>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    accept_connections_with_keepalive(
        entry_point,
        listener,
        policy,
        log,
        stop,
        serve,
        set_keepalive,
    )
    .await;
}

// The socket-option operation is an explicit parameter so tests can fail it
// deterministically without relying on platform-specific option limits.
async fn accept_connections_with_keepalive<L: crate::listener::Listener, Serve, Fut, Keepalive>(
    entry_point: EntryPoint,
    mut listener: L,
    policy: AcceptPolicy,
    log: Arc<dyn EventSink>,
    mut stop: watch::Receiver<bool>,
    serve: Serve,
    keepalive: Keepalive,
) where
    Serve: Fn(TcpStream, Accepted, BudgetPermit, watch::Receiver<bool>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
    Keepalive: Fn(&TcpStream, &KeepaliveSettings) -> io::Result<()>,
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
            Err(error) => {
                if crate::listener::is_fatal_accept_error(&error) {
                    break;
                }
                tokio::time::sleep(ACCEPT_RETRY_DELAY).await;
                continue;
            }
        };
        let at = Instant::now();
        let accepted_wall = SystemTime::now();
        let peer_ip = PeerIp::from(addr);
        let (permit, settings) = match admit(&policy, peer_ip) {
            Ok(admitted) => admitted,
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
        let conn = Accepted {
            conn_id: ConnId::next(),
            entry_point,
            peer_ip,
            at,
            peer_addr: addr,
            accepted_wall,
            settings,
        };
        log.record(&Event::ConnectionAccepted {
            conn_id: conn.conn_id,
            entry_point: conn.entry_point,
            peer_ip,
        });
        if keepalive(&stream, &conn.settings.keepalive).is_err() {
            let stage = match entry_point {
                EntryPoint::TcpViewer | EntryPoint::TcpServer => Stage::InitialExchange,
                EntryPoint::WsViewer => Stage::Upgrade,
                EntryPoint::WssViewer => Stage::Tls,
            };
            log.record(&Event::ConnectionAborted {
                conn_id: conn.conn_id,
                entry_point,
                stage,
                reason: AbortReason::ConnectionError(ConnectionErrorKind::IoError),
                peer_ip,
                elapsed_ms: u64::try_from(conn.at.elapsed().as_millis()).unwrap_or(u64::MAX),
            });
            drop(stream);
            drop(permit);
            continue;
        }
        held.spawn(serve(stream, conn, permit, stop.clone()));
    }
    drop(listener);
    while held.join_next().await.is_some() {}
}

/// The source IP check, then a place of L (relay spec §3.1 step 1, RLY-R16).
/// A refused source neither takes a place of L nor learns that L is full.
fn admit(
    policy: &AcceptPolicy,
    peer_ip: PeerIp,
) -> Result<(BudgetPermit, AcceptedSettings), RejectReason> {
    let mut admitted = Err(RejectReason::IpNotAllowed);
    policy.allow.with_current(&mut |settings| {
        if policy.allow.permits_in(settings, peer_ip) {
            let limit = settings.limits.relay_connections;
            admitted = policy
                .relay_budget
                .try_acquire(limit)
                .map(|permit| (permit, AcceptedSettings::from_settings(settings)))
                .ok_or(RejectReason::RelayConnectionLimit { limit });
        }
    });
    admitted
}

/// Sets the socket options of a relay connection: TCP keepalive with the idle
/// time, probe interval, and probe count of K (config-v3 §1.2 `[keepalive]`),
/// and `TCP_NODELAY`. A relay forwards each read as it arrives; with Nagle on,
/// a small write followed by a larger one waits for the peer's delayed ACK.
fn set_keepalive(stream: &TcpStream, keepalive: &KeepaliveSettings) -> io::Result<()> {
    let params = TcpKeepalive::new()
        .with_time(keepalive.tcp_idle)
        .with_interval(keepalive.tcp_interval)
        .with_retries(keepalive.tcp_probes);
    SockRef::from(stream).set_tcp_keepalive(&params)?;
    stream.set_nodelay(true)
}

// Reading the options back needs getters that socket2 offers on Linux but
// not on Windows.
#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sets_keepalive_from_the_settings_and_turns_nagle_off() {
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
        assert!(got(server.nodelay()));
    }
}

#[cfg(test)]
mod accept_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::sync::mpsc;
    use vnc_relay_app::reload::snapshot::ReloadSnapshot;
    use vnc_relay_domain::allow_list::AllowList;
    use vnc_relay_domain::allow_list::IpItem;

    use crate::ops_log::format_line;

    struct LogSender(mpsc::UnboundedSender<serde_json::Value>);

    impl EventSink for LogSender {
        fn record(&self, event: &Event) {
            let line = format_line(event, SystemTime::UNIX_EPOCH);
            let value = serde_json::from_str(&line)
                .unwrap_or_else(|error| panic!("parse log line: {error}"));
            self.0
                .send(value)
                .unwrap_or_else(|error| panic!("send log line: {error}"));
        }
    }

    #[tokio::test]
    async fn keepalive_failure_aborts_accepted_connections_and_releases_l() {
        for (entry_point, stage) in [
            (EntryPoint::TcpViewer, "initial_exchange"),
            (EntryPoint::TcpServer, "initial_exchange"),
            (EntryPoint::WsViewer, "upgrade"),
            (EntryPoint::WssViewer, "tls"),
        ] {
            tokio::time::timeout(Duration::from_secs(5), check_failure(entry_point, stage))
                .await
                .unwrap_or_else(|error| panic!("{} timed out: {error}", entry_point.name()));
        }
    }

    async fn check_failure(entry_point: EntryPoint, stage: &str) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|error| panic!("bind: {error}"));
        let address = listener
            .local_addr()
            .unwrap_or_else(|error| panic!("listener address: {error}"));
        let allow = AllowList::new(vec![
            IpItem::parse("127.0.0.1").unwrap_or_else(|error| panic!("allow: {error:?}")),
        ]);
        let mut settings = crate::config_file::parse(
            "[admin]\naccounts_file='accounts.toml'\n[management]\nlisten='127.0.0.1:8443'\n[status]\nlisten='127.0.0.1:9090'\n",
            std::path::Path::new("."),
        ).unwrap_or_else(|error| panic!("settings: {error:?}"));
        settings.relay.tcp_viewer.allow = Some(allow.clone());
        settings.relay.tcp_server.allow = Some(allow.clone());
        settings.relay.ws_viewer.allow = Some(allow.clone());
        settings.relay.wss_viewer.allow = Some(allow);
        settings.limits.relay_connections = 1;
        settings.timeouts.handshake = Duration::from_secs(5);
        let snapshot = ReloadSnapshot::new(
            vnc_relay_app::reload::ConfigCandidate {
                settings,
                presence: Default::default(),
            },
            None,
            None,
        );
        let cell = Arc::new(ReloadCell::new(snapshot));
        let allow = match entry_point {
            EntryPoint::TcpViewer => AllowSource::tcp_viewer(cell),
            EntryPoint::TcpServer => AllowSource::tcp_server(cell),
            EntryPoint::WsViewer => AllowSource::ws_viewer(cell),
            EntryPoint::WssViewer => AllowSource::wss_viewer(cell),
        };
        let budget = ConnectionBudget::new();
        let policy = AcceptPolicy {
            allow,
            relay_budget: Arc::clone(&budget),
        };
        let (events, mut received) = mpsc::unbounded_channel();
        let log: Arc<dyn EventSink> = Arc::new(LogSender(events));
        let (stop, stopped) = watch::channel(false);
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_in_accept = Arc::clone(&attempts);
        let served = Arc::new(AtomicUsize::new(0));
        let served_in_accept = Arc::clone(&served);
        let task = tokio::spawn(accept_connections_with_keepalive(
            entry_point,
            listener,
            policy,
            log,
            stopped,
            move |mut stream, _conn, permit, mut stopped| {
                served_in_accept.fetch_add(1, Ordering::SeqCst);
                async move {
                    stream
                        .write_all(b"served")
                        .await
                        .unwrap_or_else(|error| panic!("write served: {error}"));
                    stopped
                        .wait_for(|stop| *stop)
                        .await
                        .unwrap_or_else(|error| panic!("wait for stop: {error}"));
                    drop(stream);
                    drop(permit);
                }
            },
            move |stream, settings| {
                if attempts_in_accept.fetch_add(1, Ordering::SeqCst) == 0 {
                    Err(io::Error::other("injected socket option failure"))
                } else {
                    set_keepalive(stream, settings)
                }
            },
        ));
        let started = Instant::now();
        let mut first = TcpStream::connect(address)
            .await
            .unwrap_or_else(|error| panic!("first connect: {error}"));
        let accepted = received
            .recv()
            .await
            .unwrap_or_else(|| panic!("accept log"));
        let aborted = received.recv().await.unwrap_or_else(|| panic!("abort log"));
        let elapsed_ms = aborted["elapsed_ms"]
            .as_u64()
            .unwrap_or_else(|| panic!("elapsed_ms must be an unsigned integer: {aborted}"));
        assert!(u128::from(elapsed_ms) <= started.elapsed().as_millis());
        let conn_id = accepted["conn_id"]
            .as_str()
            .unwrap_or_else(|| panic!("conn_id must be a string: {accepted}"));
        assert!(!conn_id.is_empty());
        assert_eq!(
            accepted,
            serde_json::json!({
                "ts": "1970-01-01T00:00:00.000Z",
                "level": "info", "event": "connection_accepted",
                "conn_id": conn_id, "listener": entry_point.name(),
                "role": entry_point.role().as_str(), "peer_ip": "127.0.0.1"
            })
        );
        assert_eq!(
            aborted,
            serde_json::json!({
                "ts": "1970-01-01T00:00:00.000Z",
                "level": "warn", "event": "connection_aborted",
                "conn_id": conn_id, "listener": entry_point.name(),
                "stage": stage, "reason": "connection_error", "error_kind": "io_error",
                "role": entry_point.role().as_str(), "peer_ip": "127.0.0.1",
                "elapsed_ms": elapsed_ms
            })
        );
        let mut byte = [0; 1];
        match first.read(&mut byte).await {
            Ok(0) => {}
            Err(error) if error.kind() == ErrorKind::ConnectionReset => {}
            other => panic!("failed socket must close without serving: {other:?}"),
        }
        assert_eq!(served.load(Ordering::SeqCst), 0);
        assert_eq!(budget.current(), 0);

        let mut second = TcpStream::connect(address)
            .await
            .unwrap_or_else(|error| panic!("second connect: {error}"));
        let next = received
            .recv()
            .await
            .unwrap_or_else(|| panic!("next accept log"));
        assert_eq!(next["event"], "connection_accepted");
        assert_eq!(next["listener"], entry_point.name());
        assert_ne!(next["conn_id"], accepted["conn_id"]);
        let mut payload = [0; 6];
        second
            .read_exact(&mut payload)
            .await
            .unwrap_or_else(|error| panic!("following connection must be served: {error}"));
        assert_eq!(&payload, b"served");
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
        assert_eq!(served.load(Ordering::SeqCst), 1);
        assert_eq!(budget.current(), 1);
        assert!(matches!(
            received.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
        stop.send(true)
            .unwrap_or_else(|error| panic!("stop accept: {error}"));
        task.await
            .unwrap_or_else(|error| panic!("accept task: {error}"));
        assert_eq!(budget.current(), 0);
    }

    // Reading K back needs socket2 getters that exist on Linux only.
    /// spec: operations.md:OPS-050 partial
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn reloaded_keepalive_applies_to_new_accepts_and_keeps_accepted_sockets() {
        tokio::time::timeout(Duration::from_secs(5), check_reloaded_keepalive())
            .await
            .unwrap_or_else(|error| panic!("keepalive reload timed out: {error}"));
    }

    #[cfg(target_os = "linux")]
    async fn check_reloaded_keepalive() {
        type Keepalive = (Duration, Duration, u32);
        fn keepalive_of(stream: &TcpStream) -> Keepalive {
            fn got<T>(result: io::Result<T>) -> T {
                result.unwrap_or_else(|error| panic!("get: {error}"))
            }
            let socket = SockRef::from(stream);
            (
                got(socket.tcp_keepalive_time()),
                got(socket.tcp_keepalive_interval()),
                got(socket.tcp_keepalive_retries()),
            )
        }
        fn snapshot(settings: Settings) -> ReloadSnapshot<TlsAcceptor> {
            ReloadSnapshot::new(
                vnc_relay_app::reload::ConfigCandidate {
                    settings,
                    presence: Default::default(),
                },
                None,
                None,
            )
        }

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|error| panic!("bind: {error}"));
        let address = listener
            .local_addr()
            .unwrap_or_else(|error| panic!("listener address: {error}"));
        let mut settings = crate::config_file::parse(
            "[admin]\naccounts_file='accounts.toml'\n[management]\nlisten='127.0.0.1:8443'\n[status]\nlisten='127.0.0.1:9090'\n",
            std::path::Path::new("."),
        ).unwrap_or_else(|error| panic!("settings: {error:?}"));
        settings.relay.tcp_viewer.allow = Some(AllowList::new(vec![
            IpItem::parse("127.0.0.1").unwrap_or_else(|error| panic!("allow: {error:?}")),
        ]));
        settings.keepalive.tcp_idle = Duration::from_secs(70);
        settings.keepalive.tcp_interval = Duration::from_secs(11);
        settings.keepalive.tcp_probes = 7;
        let old: Keepalive = (Duration::from_secs(70), Duration::from_secs(11), 7);
        let new: Keepalive = (Duration::from_secs(90), Duration::from_secs(13), 9);
        let cell = Arc::new(ReloadCell::new(snapshot(settings.clone())));
        let policy = AcceptPolicy {
            allow: AllowSource::tcp_viewer(Arc::clone(&cell)),
            relay_budget: ConnectionBudget::new(),
        };
        let (events, _log_lines) = mpsc::unbounded_channel();
        let log: Arc<dyn EventSink> = Arc::new(LogSender(events));
        let (stop, stopped) = watch::channel(false);
        let (observed, mut seen) = mpsc::unbounded_channel();
        let task = tokio::spawn(accept_connections(
            EntryPoint::TcpViewer,
            listener,
            policy,
            log,
            stopped,
            move |stream, conn, permit, mut stopped| {
                let observed = observed.clone();
                async move {
                    let report = |when| {
                        observed
                            .send((when, conn.peer_addr, keepalive_of(&stream)))
                            .unwrap_or_else(|error| panic!("send keepalive: {error}"));
                    };
                    report("accepted");
                    stopped
                        .wait_for(|stop| *stop)
                        .await
                        .unwrap_or_else(|error| panic!("wait for stop: {error}"));
                    report("stopping");
                    drop(permit);
                }
            },
        ));

        let first = TcpStream::connect(address)
            .await
            .unwrap_or_else(|error| panic!("first connect: {error}"));
        let first_addr = first
            .local_addr()
            .unwrap_or_else(|error| panic!("first address: {error}"));
        let accepted = seen.recv().await.unwrap_or_else(|| panic!("first accept"));
        assert_eq!(accepted, ("accepted", first_addr, old));

        settings.keepalive.tcp_idle = Duration::from_secs(90);
        settings.keepalive.tcp_interval = Duration::from_secs(13);
        settings.keepalive.tcp_probes = 9;
        cell.replace(snapshot(settings));
        let second = TcpStream::connect(address)
            .await
            .unwrap_or_else(|error| panic!("second connect: {error}"));
        let second_addr = second
            .local_addr()
            .unwrap_or_else(|error| panic!("second address: {error}"));
        let accepted = seen.recv().await.unwrap_or_else(|| panic!("second accept"));
        assert_eq!(accepted, ("accepted", second_addr, new));

        stop.send(true)
            .unwrap_or_else(|error| panic!("stop accept: {error}"));
        task.await
            .unwrap_or_else(|error| panic!("accept task: {error}"));
        let mut kept = vec![
            seen.try_recv()
                .unwrap_or_else(|error| panic!("first kept: {error}")),
            seen.try_recv()
                .unwrap_or_else(|error| panic!("second kept: {error}")),
        ];
        kept.sort_by_key(|(_, peer, _)| *peer != first_addr);
        assert_eq!(
            kept,
            vec![
                ("stopping", first_addr, old),
                ("stopping", second_addr, new)
            ],
            "an accepted socket keeps the K it was accepted with"
        );
    }
}
