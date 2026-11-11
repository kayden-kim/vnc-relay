// Contract tests of the management listener (docs/architectures/management-listener.md,
// docs/contracts/management-api-v1.md §1.3, §1.4.1, §1.11).
//
// Fixtures under tests/fixtures/tls are shared with tls.rs. Each test starts its
// own listener on loopback. Connections that hold Lm stay open until the test ends.

use std::io;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rustls::crypto::aws_lc_rs;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::version::{TLS12, TLS13};
use rustls::{ClientConfig, RootCertStore, SupportedProtocolVersion};
use serde_json::{Value, json};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_rustls::{TlsAcceptor, TlsConnector};
use vnc_relay_adapters::accounts_file::{Argon2Hasher, Argon2Verifier};
use vnc_relay_adapters::clock::SystemClock;
use vnc_relay_adapters::config_edit::{FsConfigFileStore, TomlConfigDocument};
use vnc_relay_adapters::config_file;
use vnc_relay_adapters::crypto::{RelayIdKey, SessionIdKey, SystemTokenSource};
use vnc_relay_adapters::management_http::{DashboardHints, ManagementListener, ManagementServices};
use vnc_relay_adapters::net::{AcceptPolicy, AllowSource, TcpEntryPoint, TcpRelayStream};
use vnc_relay_adapters::ops_log::{JsonLinesEventSink, RecentLogs, format_line};
use vnc_relay_adapters::target::TcpTargetConnector;
use vnc_relay_adapters::tls;
use vnc_relay_app::events::{AdminTerminateResult, Event, Level, RejectReason, Stage};
use vnc_relay_app::management::config::{
    ConfigActor, ConfigEditError, ConfigEditResult, ConfigEditService, ConfigFileStore,
    ConfigGetError, ConfigManagement, ConfigPatch, ConfigReplaceError, ConfigView,
};
use vnc_relay_app::management::login::dummy_hash;
use vnc_relay_app::management::query::ManagementQueryService;
use vnc_relay_app::management::terminate::{ManagementTerminateService, TerminationTicket};
use vnc_relay_app::management::{AdminAuth, AdminPorts};
use vnc_relay_app::ports::{
    Clock, EventSink, HashError, PasswordHasher, PasswordVerifier, RelayCloseAction,
    RelayLifecycle, RelaySessionControl, RelayStateSource, RelayStream,
};
use vnc_relay_app::relay::{
    ConnectionContext, RelayRuntime, RelayRuntimeConfig, RelayService, RelaySnapshot,
};
use vnc_relay_app::reload::SharedReload;
use vnc_relay_app::reload::ports::{ReloadFileStore, ReloadTlsStore, TlsBuildError};
use vnc_relay_app::reload::service::ReloadService;
use vnc_relay_app::reload::snapshot::{ReloadCell, ReloadSnapshot};
use vnc_relay_domain::account::{Account, AccountName, Argon2Params, Password, PasswordHash};
use vnc_relay_domain::admin_session::AdminSessionPolicy;
use vnc_relay_domain::budget::ConnectionBudget;
use vnc_relay_domain::conn_id::ConnId;
use vnc_relay_domain::entry_point::EntryPoint;
use vnc_relay_domain::handshake::{HANDSHAKE_REQUEST_LEN, HandshakePolicy, VIEWER_GREETING};
use vnc_relay_domain::login_limit::{LimitScope, LoginLimitPolicy};
use vnc_relay_domain::peer_ip::PeerIp;
use vnc_relay_domain::session::Transport;

/// Upper bound for one exchange. A hang fails the test instead of blocking it.
const LIMIT: Duration = Duration::from_secs(10);

/// Lm for tests that send several requests in a row. Each request uses a new
/// connection, so a small limit could still be held by a closing one.
const ROOMY_LM: u32 = 8;

/// U of the default configuration (config-v4 §1.2, `tls_upgrade_secs` = 10).
const UPGRADE: Duration = Duration::from_secs(10);

/// A short U for tests that wait for the deadline to end a connection.
const SHORT_UPGRADE: Duration = Duration::from_millis(300);

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/tls")
        .join(name)
}

fn tls_acceptor() -> TlsAcceptor {
    tls::load_acceptor(
        &fixture("server-chain.pem"),
        &fixture("server-key-p256.pem"),
    )
    .unwrap_or_else(|error| panic!("server identity loads: {error}"))
}

/// A client that trusts only the test root CA and offers the given versions.
fn connector(versions: &[&'static SupportedProtocolVersion]) -> TlsConnector {
    let mut roots = RootCertStore::empty();
    let certs = CertificateDer::pem_file_iter(fixture("root-ca.pem"))
        .unwrap_or_else(|error| panic!("root CA opens: {error}"));
    for cert in certs {
        let cert = cert.unwrap_or_else(|error| panic!("root CA is PEM: {error}"));
        roots
            .add(cert)
            .unwrap_or_else(|error| panic!("root CA is a certificate: {error}"));
    }
    let config = ClientConfig::builder_with_provider(Arc::new(aws_lc_rs::default_provider()))
        .with_protocol_versions(versions)
        .unwrap_or_else(|error| panic!("client protocol versions: {error}"))
        .with_root_certificates(roots)
        .with_no_client_auth();
    TlsConnector::from(Arc::new(config))
}

/// Collects every event the listener records.
#[derive(Default)]
struct Recorder(Mutex<Vec<Event>>);

impl Recorder {
    fn lock(&self) -> MutexGuard<'_, Vec<Event>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn snapshot(&self) -> Vec<Event> {
        self.lock().clone()
    }
}

impl EventSink for Recorder {
    fn record(&self, event: &Event) {
        self.lock().push(event.clone());
    }
}

/// Waits until an event that `matches` is recorded and returns it.
async fn wait_for(events: &Recorder, matches: impl Fn(&Event) -> bool) -> Event {
    let found = async {
        loop {
            if let Some(event) = events.snapshot().into_iter().find(&matches) {
                return event;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    };
    timeout(LIMIT, found)
        .await
        .unwrap_or_else(|error| panic!("event is recorded in time: {error}"))
}

struct Running {
    addr: SocketAddr,
    events: Arc<Recorder>,
    task: JoinHandle<()>,
    logs: Arc<RecentLogs>,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Starts a management listener on loopback with the given Lm and U of 10 s.
async fn start(tls: Option<TlsAcceptor>, admin_connections: u32) -> Running {
    start_with_upgrade(tls, admin_connections, UPGRADE).await
}

/// Starts a management listener on loopback with the given Lm and U.
async fn start_with_upgrade(
    tls: Option<TlsAcceptor>,
    admin_connections: u32,
    upgrade: Duration,
) -> Running {
    start_with(
        tls,
        admin_connections,
        upgrade,
        session_policy(),
        limit_policy(),
    )
    .await
}

/// Starts a management listener on loopback with its own admin state.
async fn start_with(
    tls: Option<TlsAcceptor>,
    admin_connections: u32,
    upgrade: Duration,
    session: AdminSessionPolicy,
    limit: LoginLimitPolicy,
) -> Running {
    start_with_ports(
        tls,
        admin_connections,
        upgrade,
        session,
        limit,
        Arc::new(Argon2Verifier),
        Arc::new(SystemClock::new()),
    )
    .await
}

/// Like `start_with`, but the admin state uses `verifier` and `clock`.
async fn start_with_ports(
    tls: Option<TlsAcceptor>,
    admin_connections: u32,
    upgrade: Duration,
    session: AdminSessionPolicy,
    limit: LoginLimitPolicy,
    verifier: Arc<dyn PasswordVerifier>,
    clock: Arc<dyn Clock>,
) -> Running {
    start_with_ports_and_logs(
        tls,
        admin_connections,
        upgrade,
        session,
        limit,
        verifier,
        (clock, Arc::new(RecentLogs::new(1000))),
    )
    .await
}

async fn start_with_ports_and_logs(
    tls: Option<TlsAcceptor>,
    admin_connections: u32,
    upgrade: Duration,
    session: AdminSessionPolicy,
    limit: LoginLimitPolicy,
    verifier: Arc<dyn PasswordVerifier>,
    state: (Arc<dyn Clock>, Arc<RecentLogs>),
) -> Running {
    let (clock, logs) = state;
    let events = Arc::new(Recorder::default());
    let log: Arc<dyn EventSink> = events.clone();
    let budget = ConnectionBudget::new();
    let runtime = management_runtime(admin_connections, upgrade, Duration::from_secs(10));
    let auth = admin_auth(
        Arc::clone(&log),
        session,
        limit,
        verifier,
        Arc::clone(&clock),
    );
    let relay_budget = ConnectionBudget::new();
    let query = Arc::new(ManagementQueryService::new(
        Arc::new(EmptyRelaySource),
        relay_budget,
        Arc::clone(&budget),
        Arc::clone(&clock),
        runtime.clone(),
    ));
    let terminate = Arc::new(ManagementTerminateService::new(
        Arc::new(EmptyRelaySource),
        clock,
        Arc::clone(&log),
    ));
    start_services(
        tls,
        budget,
        runtime,
        ManagementServices {
            auth,
            query,
            terminate,
            config: config_fixture(false, Arc::new(SystemClock::new()), Arc::clone(&log)).service,
            logs,
        },
        events,
    )
    .await
}

async fn start_services(
    tls: Option<TlsAcceptor>,
    budget: Arc<ConnectionBudget>,
    runtime: Arc<ReloadCell<TlsAcceptor>>,
    services: ManagementServices,
    events: Arc<Recorder>,
) -> Running {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap_or_else(|error| panic!("loopback bind: {error}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|error| panic!("loopback address: {error}"));
    let dashboard = DashboardHints::new(
        Duration::from_secs(2),
        Duration::from_secs(5),
        runtime.clone(),
    );
    let tls = tls.map(|acceptor| {
        let current = runtime.current();
        runtime.replace(ReloadSnapshot {
            settings: current.settings.clone(),
            presence: current.presence.clone(),
            wss_tls: None,
            management_tls: Some(acceptor),
        });
        tls::TlsSource::management(runtime.clone())
    });
    let logs = Arc::clone(&services.logs);
    let listener = ManagementListener::new(listener, tls, budget, runtime, dashboard, services);
    let task = tokio::spawn(listener.run(events.clone()));
    Running {
        addr,
        events,
        task,
        logs,
    }
}

/// The same runtime backs TCP relay traffic, operational queries and termination.
struct RunningRelay {
    management: Running,
    viewer: SocketAddr,
    server: SocketAddr,
    stop: watch::Sender<bool>,
    tasks: Vec<JoinHandle<()>>,
    runtime: Arc<RelayRuntime<TcpTargetConnector>>,
    service: Arc<RelayService>,
    budget: Arc<ConnectionBudget>,
    clock: Arc<dyn Clock>,
    source: Arc<ObservedRelaySource>,
}

impl RunningRelay {
    async fn start(session: AdminSessionPolicy) -> Self {
        Self::start_with_control(session, false, Arc::new(Argon2Verifier)).await
    }

    async fn start_with_control(
        session: AdminSessionPolicy,
        fail_first: bool,
        verifier: Arc<dyn PasswordVerifier>,
    ) -> Self {
        Self::start_with_clock(session, fail_first, verifier, Arc::new(SystemClock::new())).await
    }

    async fn start_with_clock(
        session: AdminSessionPolicy,
        fail_first: bool,
        verifier: Arc<dyn PasswordVerifier>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        let events = Arc::new(Recorder::default());
        let log: Arc<dyn EventSink> = events.clone();
        let relay_budget = ConnectionBudget::new();
        let admin_budget = ConnectionBudget::new();
        let cell = management_runtime(ROOMY_LM, UPGRADE, Duration::from_secs(1));
        let runtime = Arc::new(RelayRuntime::new(
            TcpTargetConnector::new(cell.clone()),
            Arc::new(RelayIdKey::generate().unwrap_or_else(|error| panic!("relay key: {error:?}"))),
            Arc::new(
                SessionIdKey::generate().unwrap_or_else(|error| panic!("session key: {error:?}")),
            ),
            Arc::clone(&relay_budget),
            ConnectionBudget::new(),
            RelayRuntimeConfig { recent_closed: 16 },
            cell.clone(),
        ));
        let auth = admin_auth(
            Arc::clone(&log),
            session,
            limit_policy(),
            verifier,
            Arc::clone(&clock),
        );
        let source = Arc::new(ObservedRelaySource {
            runtime: runtime.clone(),
            after_snapshot: Mutex::new(None),
            last: Mutex::new(None),
        });
        let query = Arc::new(ManagementQueryService::new(
            source.clone(),
            Arc::clone(&relay_budget),
            Arc::clone(&admin_budget),
            Arc::clone(&clock),
            cell.clone(),
        ));
        let terminate = Arc::new(ManagementTerminateService::new(
            Arc::new(FailFirstTermination {
                runtime: runtime.clone(),
                fail_next: AtomicBool::new(fail_first),
            }),
            Arc::clone(&clock),
            Arc::clone(&log),
        ));
        let management = start_services(
            None,
            admin_budget,
            cell.clone(),
            ManagementServices {
                auth,
                query,
                terminate,
                config: config_fixture(false, Arc::clone(&clock), Arc::clone(&log)).service,
                logs: Arc::new(RecentLogs::new(1000)),
            },
            events,
        )
        .await;
        let service = Arc::new(RelayService::new(
            HandshakePolicy {
                mode1_enabled: false,
                mode2_enabled: true,
            },
            cell.clone(),
            Arc::clone(&clock),
            Arc::clone(&log),
        ));
        let mut addresses = Vec::new();
        let mut tasks = Vec::new();
        let (stop, stopped) = watch::channel(false);
        for (kind, allow) in [
            (
                EntryPoint::TcpViewer,
                AllowSource::tcp_viewer(Arc::clone(&cell)),
            ),
            (
                EntryPoint::TcpServer,
                AllowSource::tcp_server(Arc::clone(&cell)),
            ),
        ] {
            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .unwrap_or_else(|error| panic!("relay bind: {error}"));
            addresses.push(
                listener
                    .local_addr()
                    .unwrap_or_else(|error| panic!("relay address: {error}")),
            );
            let entry = TcpEntryPoint::new(
                kind,
                listener,
                AcceptPolicy {
                    allow,
                    relay_budget: Arc::clone(&relay_budget),
                },
                Arc::clone(&service),
                Arc::clone(&runtime),
                Arc::clone(&clock),
            );
            tasks.push(tokio::spawn(entry.run(Arc::clone(&log), stopped.clone())));
        }
        Self {
            management,
            viewer: addresses[0],
            server: addresses[1],
            stop,
            tasks,
            runtime,
            service,
            budget: relay_budget,
            clock,
            source,
        }
    }

    async fn pair(&self) -> RelayPair {
        self.pair_with_payload(true).await
    }

    async fn pair_with_payload(&self, exchange_payload: bool) -> RelayPair {
        self.pair_with_id("123", exchange_payload).await
    }

    async fn pair_with_id(&self, id: &str, exchange_payload: bool) -> RelayPair {
        let old_ids: Vec<_> = self
            .management
            .events
            .snapshot()
            .into_iter()
            .filter_map(|event| {
                if let Event::SessionStarted { session_id, .. } = event {
                    Some(session_id)
                } else {
                    None
                }
            })
            .collect();
        timeout(LIMIT, async {
            let mut viewer = TcpStream::connect(self.viewer)
                .await
                .unwrap_or_else(|error| panic!("viewer connect: {error}"));
            let mut greeting = [0; VIEWER_GREETING.len()];
            viewer
                .read_exact(&mut greeting)
                .await
                .unwrap_or_else(|error| panic!("viewer greeting: {error}"));
            assert_eq!(&greeting, VIEWER_GREETING);
            let mut initial = [0; HANDSHAKE_REQUEST_LEN];
            let request = format!("ID:{id}");
            assert!(request.len() <= HANDSHAKE_REQUEST_LEN, "test relay ID fits handshake");
            initial[..request.len()].copy_from_slice(request.as_bytes());
            viewer
                .write_all(&initial)
                .await
                .unwrap_or_else(|error| panic!("viewer handshake: {error}"));
            wait_for(&self.management.events, |event| {
                matches!(event, Event::WaitingRegistered { .. })
            })
            .await;
            let mut server = TcpStream::connect(self.server)
                .await
                .unwrap_or_else(|error| panic!("server connect: {error}"));
            server
                .write_all(&initial)
                .await
                .unwrap_or_else(|error| panic!("server handshake: {error}"));
            let started = wait_for(&self.management.events, |event| {
                matches!(event, Event::SessionStarted { session_id, .. } if !old_ids.contains(session_id))
            })
            .await;
            let Event::SessionStarted { session_id, .. } = started else {
                panic!("session-start event identifies the real relay");
            };
            let mut pair = RelayPair {
                viewer,
                server,
                session_id: session_id.to_string(),
            };
            if exchange_payload {
                pair.assert_bidirectional().await;
            }
            pair
        })
        .await
        .unwrap_or_else(|error| panic!("relay pair starts in time: {error}"))
    }
}

impl Drop for RunningRelay {
    fn drop(&mut self) {
        self.stop.send_replace(true);
        for task in &self.tasks {
            task.abort();
        }
    }
}

struct RelayPair {
    viewer: TcpStream,
    server: TcpStream,
    session_id: String,
}

impl RelayPair {
    async fn assert_bidirectional(&mut self) {
        timeout(LIMIT, async {
            async fn transfer(source: &mut TcpStream, destination: &mut TcpStream, bytes: &[u8]) {
                source
                    .write_all(bytes)
                    .await
                    .unwrap_or_else(|error| panic!("relay payload sent: {error}"));
                let mut received = vec![0; bytes.len()];
                destination
                    .read_exact(&mut received)
                    .await
                    .unwrap_or_else(|error| panic!("relay payload received: {error}"));
                assert_eq!(received, bytes);
            }
            transfer(&mut self.viewer, &mut self.server, b"viewer-to-server").await;
            transfer(&mut self.server, &mut self.viewer, b"server-to-viewer").await;
        })
        .await
        .unwrap_or_else(|error| panic!("relay carries both directions in time: {error}"));
    }

    async fn assert_listed_and_terminate(&self, running: &Running) {
        let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
        assert_eq!(login.status, 200);
        let cookie = cookie_line(&login);
        let csrf = csrf_line(&login);
        let listed = plain_exchange(running.addr, &get("/api/v1/sessions", &[&cookie])).await;
        assert_eq!(listed.status, 200);
        let body = json_body(&listed);
        let items = body["items"]
            .as_array()
            .unwrap_or_else(|| panic!("sessions is an array: {body}"));
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["session_id"], self.session_id);
        assert_eq!(items[0]["state"], "active");
        let terminated = plain_exchange(
            running.addr,
            &request(
                "POST",
                &format!("/api/v1/sessions/{}/terminate", self.session_id),
                &[&cookie, &csrf],
            ),
        )
        .await;
        assert_eq!(terminated.status, 200);
        let body = json_body(&terminated);
        assert_eq!(body["result"], "terminated");
        assert_eq!(body["closed_session"]["session_id"], self.session_id);
        assert_eq!(body["closed_session"]["close_cause"], "admin_terminated");
    }
}

/// Only the first accepted control request loses its completion. Query state
/// and all later termination requests still use the real relay runtime.
struct FailFirstTermination {
    runtime: Arc<RelayRuntime<TcpTargetConnector>>,
    fail_next: AtomicBool,
}

impl RelaySessionControl for FailFirstTermination {
    fn request_terminate(
        &self,
        session_id: &str,
        account: AccountName,
        peer: PeerIp,
        clock: &dyn Clock,
        events: &dyn EventSink,
    ) -> Option<TerminationTicket> {
        if self.fail_next.swap(false, Ordering::SeqCst) {
            assert!(
                self.runtime
                    .snapshot(clock)
                    .active
                    .iter()
                    .any(|session| session.session_id.as_str() == session_id),
                "the failing control targets a real active session"
            );
            let (sender, completion) = watch::channel(None);
            drop(sender);
            // A joining request owns its failure audit in the real service.
            return Some(TerminationTicket {
                initiated: false,
                completion,
            });
        }
        self.runtime
            .request_terminate(session_id, account, peer, clock, events)
    }
}

/// Retains the actual account hash verified during login, not a synthetic PHC.
#[derive(Default)]
struct HashRecordingVerifier(Mutex<String>);

impl PasswordVerifier for HashRecordingVerifier {
    fn verify(&self, typed: &str, hash: &PasswordHash) -> Result<bool, HashError> {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = hash.as_str().to_owned();
        Argon2Verifier.verify(typed, hash)
    }
}

struct EmptyRelaySource;

impl RelaySessionControl for EmptyRelaySource {
    fn request_terminate(
        &self,
        _session_id: &str,
        _account: AccountName,
        _peer: PeerIp,
        _clock: &dyn Clock,
        _events: &dyn EventSink,
    ) -> Option<TerminationTicket> {
        None
    }
}

impl RelayStateSource for EmptyRelaySource {
    fn snapshot(&self, clock: &dyn Clock) -> RelaySnapshot {
        RelaySnapshot {
            lifecycle: RelayLifecycle::Running,
            observed_at: clock.monotonic(),
            handshaking: 0,
            target_connecting: 0,
            max_entries: 0,
            waiting: Vec::new(),
            active: Vec::new(),
            recent_closed: Vec::new(),
        }
    }
}

/// Session times that the login tests never reach by accident.
fn session_policy() -> AdminSessionPolicy {
    AdminSessionPolicy::new(Duration::from_secs(600), Duration::from_secs(3600))
}

/// Limits that the login tests never reach. The limit tests pass their own.
fn limit_policy() -> LoginLimitPolicy {
    LoginLimitPolicy::new(50, 500, Duration::from_secs(60), Duration::from_secs(60))
}

/// The loopback peer that the test client uses. The port does not matter for
/// `PeerIp`, which keeps only the address.
fn loopback() -> PeerIp {
    PeerIp::from(SocketAddr::from(([127, 0, 0, 1], 0)))
}

/// A parsed HTTP/1.1 reply. Header names are lowercase.
struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Reply {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.as_str() == name)
            .map(|(_, value)| value.as_str())
    }
}

fn parse(raw: &[u8]) -> Reply {
    let end = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .unwrap_or_else(|| panic!("reply has a header block: {raw:02x?}"));
    let head = String::from_utf8_lossy(&raw[..end]).into_owned();
    let mut lines = head.split("\r\n");
    let status_line = lines.next().unwrap_or_default();
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("status line has a code: {status_line}"));
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_owned()))
        .collect();
    Reply {
        status,
        headers,
        body: raw[end + 4..].to_vec(),
    }
}

/// An HTTP/1.1 request that asks the server to close after the reply.
fn request(method: &str, path: &str, extra_headers: &[&str]) -> Vec<u8> {
    let mut text = format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n");
    for header in extra_headers {
        text.push_str(header);
        text.push_str("\r\n");
    }
    text.push_str("\r\n");
    text.into_bytes()
}

fn get(path: &str, extra_headers: &[&str]) -> Vec<u8> {
    request("GET", path, extra_headers)
}

/// Sends `request` and returns the bytes that the server sends before it closes.
async fn exchange<S>(stream: &mut S, request: &[u8]) -> Vec<u8>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    stream
        .write_all(request)
        .await
        .unwrap_or_else(|error| panic!("request is sent: {error}"));
    let mut reply = Vec::new();
    // A TLS peer may close without close_notify. The bytes read so far are the
    // reply, so a read error here is not a failure of its own.
    let _ = timeout(LIMIT, stream.read_to_end(&mut reply)).await;
    reply
}

async fn plain_exchange(addr: SocketAddr, request: &[u8]) -> Reply {
    let mut tcp = TcpStream::connect(addr)
        .await
        .unwrap_or_else(|error| panic!("loopback connect: {error}"));
    parse(&exchange(&mut tcp, request).await)
}

async fn https_exchange(
    addr: SocketAddr,
    versions: &[&'static SupportedProtocolVersion],
    request: &[u8],
) -> Reply {
    let tcp = TcpStream::connect(addr)
        .await
        .unwrap_or_else(|error| panic!("loopback connect: {error}"));
    let name = ServerName::try_from("localhost")
        .unwrap_or_else(|error| panic!("server name is valid: {error}"));
    let mut stream = connector(versions)
        .connect(name, tcp)
        .await
        .unwrap_or_else(|error| panic!("TLS handshake succeeds: {error}"));
    parse(&exchange(&mut stream, request).await)
}

/// The `error.code` of a management error body (management-api-v1 §2.1).
fn error_code(reply: &Reply) -> String {
    let value: Value = serde_json::from_slice(&reply.body)
        .unwrap_or_else(|error| panic!("error body is JSON: {error}"));
    value["error"]["code"]
        .as_str()
        .unwrap_or_else(|| panic!("error code is a string: {value}"))
        .to_owned()
}

/// A TLS record with a ClientHello of one cipher suite. It offers
/// `legacy_version`, and `versions` in `supported_versions` when `versions` is
/// not empty. It has no `signature_algorithms`, as TLS 1.0 and 1.1 clients send none.
fn client_hello(legacy_version: [u8; 2], versions: &[u16]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&legacy_version);
    body.extend_from_slice(&[0x5A; 32]); // random
    body.push(0); // legacy_session_id: empty
    body.extend_from_slice(&[0x00, 0x02, 0x00, 0x2F]); // TLS_RSA_WITH_AES_128_CBC_SHA
    body.extend_from_slice(&[0x01, 0x00]); // compression methods: null
    let mut extensions = Vec::new();
    if !versions.is_empty() {
        let list: Vec<u8> = versions
            .iter()
            .flat_map(|version| version.to_be_bytes())
            .collect();
        extensions.extend_from_slice(&[0x00, 0x2B]); // supported_versions
        extensions.extend_from_slice(&be_bytes(list.len() + 1, 2));
        extensions.extend_from_slice(&be_bytes(list.len(), 1));
        extensions.extend_from_slice(&list);
    }
    body.extend_from_slice(&be_bytes(extensions.len(), 2));
    body.extend_from_slice(&extensions);
    let mut handshake = vec![0x01]; // msg_type: client_hello
    handshake.extend_from_slice(&be_bytes(body.len(), 3));
    handshake.extend_from_slice(&body);
    let mut record = vec![0x16, 0x03, 0x01]; // handshake record
    record.extend_from_slice(&be_bytes(handshake.len(), 2));
    record.extend_from_slice(&handshake);
    record
}

/// The lowest `width` big-endian bytes of `value`.
fn be_bytes(value: usize, width: usize) -> Vec<u8> {
    value.to_be_bytes()[std::mem::size_of::<usize>() - width..].to_vec()
}

/// spec: operations.md:OPS-034 partial
#[tokio::test]
async fn plaintext_api_requests_are_refused_without_a_session() {
    let running = start(None, ROOMY_LM).await;

    let reply = plain_exchange(running.addr, &get("/api/v1/session", &[])).await;
    assert_eq!(reply.status, 401);
    assert_eq!(
        reply.header("www-authenticate"),
        Some(r#"Session realm="vnc-relay""#)
    );
    assert_eq!(reply.header("cache-control"), Some("no-store"));
    assert_eq!(
        reply.header("content-type"),
        Some("application/json; charset=utf-8")
    );
    assert_eq!(reply.header("set-cookie"), None);
    assert_eq!(error_code(&reply), "unauthenticated");

    // Path and method are not judged before the session check (§1.3 step 2).
    let reply = plain_exchange(running.addr, &request("POST", "/api/v1/unknown", &[])).await;
    assert_eq!(reply.status, 401);
    assert_eq!(error_code(&reply), "unauthenticated");
}

/// spec: management.md:MGT-017 partial
#[tokio::test]
async fn stale_session_cookie_is_cleared_without_secure_over_plain_http() {
    let running = start(None, ROOMY_LM).await;
    let cookie = ["Cookie: relay_admin_session=stale"];

    let reply = plain_exchange(running.addr, &get("/api/v1/session", &cookie)).await;
    assert_eq!(reply.status, 401);
    assert_eq!(
        reply.header("set-cookie"),
        Some("relay_admin_session=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict")
    );
}

/// spec: operations.md:OPS-036 partial
#[tokio::test]
async fn https_clients_on_tls12_and_tls13_get_the_unauthenticated_answer() {
    let running = start(Some(tls_acceptor()), ROOMY_LM).await;
    let cookie = ["Cookie: relay_admin_session=stale"];

    for versions in [&[&TLS12][..], &[&TLS13][..]] {
        let reply = https_exchange(running.addr, versions, &get("/api/v1/session", &cookie)).await;
        assert_eq!(reply.status, 401);
        assert_eq!(
            reply.header("set-cookie"),
            Some("relay_admin_session=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict; Secure")
        );
        assert_eq!(error_code(&reply), "unauthenticated");
    }
}

/// spec: operations.md:OPS-036 partial
#[tokio::test]
async fn tls10_and_tls11_hellos_are_refused_without_plaintext_fallback() {
    for legacy_version in [[0x03, 0x01], [0x03, 0x02]] {
        let running = start(Some(tls_acceptor()), ROOMY_LM).await;
        let mut tcp = TcpStream::connect(running.addr)
            .await
            .unwrap_or_else(|error| panic!("loopback connect: {error}"));
        tcp.write_all(&client_hello(legacy_version, &[]))
            .await
            .unwrap_or_else(|error| panic!("hello is sent: {error}"));
        let mut reply = Vec::new();
        timeout(LIMIT, tcp.read_to_end(&mut reply))
            .await
            .unwrap_or_else(|error| panic!("server closes in time: {error}"))
            .unwrap_or_else(|error| panic!("reply is read: {error}"));
        // A refusal is an alert record (0x15) or a closed connection. A ServerHello
        // (0x16) or an HTTP answer is a failure.
        assert!(
            reply.first().is_none_or(|&record| record == 0x15),
            "reply to {legacy_version:02x?} must not start a handshake: {reply:02x?}"
        );
        let event = wait_for(&running.events, |event| {
            matches!(
                event,
                Event::ConnectionRejected {
                    reason: RejectReason::TlsVersionUnsupported,
                    ..
                }
            )
        })
        .await;
        assert_eq!(
            event,
            Event::ConnectionRejected {
                listener: "management",
                stage: Stage::Tls,
                reason: RejectReason::TlsVersionUnsupported,
                peer_ip: loopback(),
            }
        );
    }
}

/// spec: operations.md:OPS-036 partial
#[tokio::test]
async fn version_refusal_follows_supported_versions_when_present() {
    // (legacy_version, supported_versions, recorded as a TLS version refusal)
    let cases: [([u8; 2], &[u16], bool); 3] = [
        ([0x03, 0x01], &[0x0304], false),
        ([0x03, 0x03], &[0x0301], true),
        ([0x03, 0x03], &[], false),
    ];
    for (legacy_version, versions, refused_by_version) in cases {
        let running = start(Some(tls_acceptor()), ROOMY_LM).await;
        let mut tcp = TcpStream::connect(running.addr)
            .await
            .unwrap_or_else(|error| panic!("loopback connect: {error}"));
        tcp.write_all(&client_hello(legacy_version, versions))
            .await
            .unwrap_or_else(|error| panic!("hello is sent: {error}"));
        // The server closes after the failed handshake. An event is recorded
        // before that close, so it is in place once the reply ends.
        let mut reply = Vec::new();
        timeout(LIMIT, tcp.read_to_end(&mut reply))
            .await
            .unwrap_or_else(|error| panic!("server closes in time: {error}"))
            .unwrap_or_else(|error| panic!("reply is read: {error}"));
        let recorded = running.events.snapshot().iter().any(|event| {
            matches!(
                event,
                Event::ConnectionRejected {
                    reason: RejectReason::TlsVersionUnsupported,
                    ..
                }
            )
        });
        assert_eq!(
            recorded, refused_by_version,
            "version refusal for {legacy_version:02x?} with {versions:04x?}"
        );
    }
}

/// spec: operations.md:OPS-033 partial
#[tokio::test]
async fn management_connection_beyond_lm_is_refused_at_accept() {
    let running = start(None, 1).await;
    // Holds the one place of Lm while the test runs.
    let _held = TcpStream::connect(running.addr)
        .await
        .unwrap_or_else(|error| panic!("first connection: {error}"));
    let mut refused = TcpStream::connect(running.addr)
        .await
        .unwrap_or_else(|error| panic!("second connection: {error}"));

    let mut reply = Vec::new();
    timeout(LIMIT, refused.read_to_end(&mut reply))
        .await
        .unwrap_or_else(|error| panic!("refused connection closes in time: {error}"))
        .unwrap_or_else(|error| panic!("refused connection is read: {error}"));
    assert!(reply.is_empty(), "a refused connection gets no HTTP answer");

    let event = wait_for(&running.events, |event| {
        matches!(
            event,
            Event::ConnectionRejected {
                reason: RejectReason::AdminConnectionLimit { .. },
                ..
            }
        )
    })
    .await;
    assert_eq!(
        event,
        Event::ConnectionRejected {
            listener: "management",
            stage: Stage::Accept,
            reason: RejectReason::AdminConnectionLimit { limit: 1 },
            peer_ip: loopback(),
        }
    );
}

/// spec: operations.md:OPS-006 partial
#[tokio::test]
async fn stalled_tls_handshake_is_closed_after_u_without_an_answer() {
    let running = start_with_upgrade(Some(tls_acceptor()), ROOMY_LM, SHORT_UPGRADE).await;
    // Connects and sends no ClientHello.
    let mut stalled = TcpStream::connect(running.addr)
        .await
        .unwrap_or_else(|error| panic!("loopback connect: {error}"));
    let started = Instant::now();
    let mut reply = Vec::new();
    timeout(LIMIT, stalled.read_to_end(&mut reply))
        .await
        .unwrap_or_else(|error| panic!("stalled handshake is closed in time: {error}"))
        .unwrap_or_else(|error| panic!("stalled handshake is read: {error}"));
    assert!(started.elapsed() >= SHORT_UPGRADE, "closed before U");
    assert!(reply.is_empty(), "a stalled handshake gets no HTTP answer");
}

/// spec: operations.md:OPS-033 partial
#[tokio::test]
async fn stalled_request_frees_its_lm_place_after_u() {
    let running = start_with_upgrade(None, 1, SHORT_UPGRADE).await;
    // Sends a request line and part of the head, then stops.
    let mut stalled = TcpStream::connect(running.addr)
        .await
        .unwrap_or_else(|error| panic!("loopback connect: {error}"));
    stalled
        .write_all(b"GET /api/v1/session HTTP/1.1\r\nHost: localhost\r\n")
        .await
        .unwrap_or_else(|error| panic!("partial head is sent: {error}"));
    let mut reply = Vec::new();
    timeout(LIMIT, stalled.read_to_end(&mut reply))
        .await
        .unwrap_or_else(|error| panic!("stalled request is closed in time: {error}"))
        .unwrap_or_else(|error| panic!("stalled request is read: {error}"));
    assert!(reply.is_empty(), "a stalled request gets no HTTP answer");

    // Lm had one place. Once the stalled connection is closed, the place is free.
    let reply = plain_exchange(running.addr, &get("/api/v1/session", &[])).await;
    assert_eq!(reply.status, 401);
    assert_eq!(error_code(&reply), "unauthenticated");
}

/// spec: operations.md:OPS-006 partial
#[tokio::test]
async fn idle_connection_after_a_reply_is_closed_after_u() {
    let running = start_with_upgrade(None, ROOMY_LM, SHORT_UPGRADE).await;
    let mut idle = TcpStream::connect(running.addr)
        .await
        .unwrap_or_else(|error| panic!("loopback connect: {error}"));
    let started = Instant::now();
    // No `Connection: close`, so the server keeps the connection after the reply.
    idle.write_all(b"GET /api/v1/session HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .await
        .unwrap_or_else(|error| panic!("request is sent: {error}"));
    let mut raw = Vec::new();
    timeout(LIMIT, idle.read_to_end(&mut raw))
        .await
        .unwrap_or_else(|error| panic!("idle connection is closed in time: {error}"))
        .unwrap_or_else(|error| panic!("idle connection is read: {error}"));
    assert!(
        started.elapsed() >= SHORT_UPGRADE,
        "closed before U of idle time"
    );
    let reply = parse(&raw);
    assert_eq!(reply.status, 401);
    assert_eq!(error_code(&reply), "unauthenticated");
}

#[tokio::test]
async fn static_paths_redirect_to_login_without_a_session() {
    let running = start(None, ROOMY_LM).await;

    for path in ["/", "/app/main.js", "/elsewhere"] {
        let reply = plain_exchange(running.addr, &get(path, &[])).await;
        assert_eq!(reply.status, 303, "GET {path}");
        assert_eq!(reply.header("location"), Some("/login/"), "GET {path}");
        assert!(reply.body.is_empty(), "GET {path} has no body");
        assert_eq!(
            reply.header("cache-control"),
            Some("no-store"),
            "GET {path}"
        );
    }

    // Other paths redirect for any method (management-api-v1 §1.11).
    let reply = plain_exchange(running.addr, &request("POST", "/elsewhere", &[])).await;
    assert_eq!(reply.status, 303);
    assert_eq!(reply.header("location"), Some("/login/"));
}

#[tokio::test]
async fn static_paths_accept_only_get_and_head() {
    let running = start(None, ROOMY_LM).await;

    for (method, path) in [
        ("POST", "/"),
        ("DELETE", "/app/main.js"),
        ("PUT", "/login/"),
    ] {
        let reply = plain_exchange(running.addr, &request(method, path, &[])).await;
        assert_eq!(reply.status, 405, "{method} {path}");
        assert_eq!(reply.header("allow"), Some("GET, HEAD"), "{method} {path}");
        assert_eq!(error_code(&reply), "invalid_request", "{method} {path}");
    }

    // HEAD takes the same redirect as GET, without a body.
    let reply = plain_exchange(running.addr, &request("HEAD", "/", &[])).await;
    assert_eq!(reply.status, 303);
    assert!(reply.body.is_empty());
}

#[tokio::test]
async fn login_page_is_served_without_a_session() {
    let running = start(None, ROOMY_LM).await;
    let disk = dashboard_file("login/index.html");

    let reply = plain_exchange(running.addr, &get("/login/", &[])).await;
    assert_eq!(reply.status, 200);
    check_file_headers(&reply, "text/html; charset=utf-8", disk.len());
    assert_eq!(reply.body, disk);
}

#[tokio::test]
async fn login_paths_without_a_file_are_not_found() {
    let running = start(None, ROOMY_LM).await;

    let reply = plain_exchange(running.addr, &get("/login/app.js", &[])).await;
    assert_eq!(reply.status, 404, "GET /login/app.js");
    assert_eq!(error_code(&reply), "not_found", "GET /login/app.js");
}

/// The bytes of the dashboard file `disk`, relative to `web/dashboard/`.
fn dashboard_file(disk: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../web/dashboard")
        .join(disk);
    std::fs::read(&path).unwrap_or_else(|error| panic!("reads {}: {error}", path.display()))
}

/// Checks the headers of a dashboard file reply (management-api-v1 §1.11).
fn check_file_headers(reply: &Reply, content_type: &str, length: usize) {
    let length = length.to_string();
    assert_eq!(reply.header("content-type"), Some(content_type));
    assert_eq!(reply.header("content-length"), Some(length.as_str()));
    assert_eq!(reply.header("cache-control"), Some("no-store"));
    assert_eq!(reply.header("x-content-type-options"), Some("nosniff"));
    assert_eq!(reply.header("referrer-policy"), Some("no-referrer"));
    assert_eq!(
        reply.header("content-security-policy"),
        Some("default-src 'self'; frame-ancestors 'none'")
    );
}

#[tokio::test]
async fn login_files_are_served_without_a_session() {
    let running = start(None, ROOMY_LM).await;
    let disk = dashboard_file("login/tokens.css");

    let reply = plain_exchange(running.addr, &get("/login/tokens.css", &[])).await;
    assert_eq!(reply.status, 200);
    check_file_headers(&reply, "text/css; charset=utf-8", disk.len());
    assert_eq!(reply.body, disk);

    // HEAD takes the same headers, `Content-Length` included, and no body.
    let reply = plain_exchange(running.addr, &request("HEAD", "/login/tokens.css", &[])).await;
    assert_eq!(reply.status, 200);
    check_file_headers(&reply, "text/css; charset=utf-8", disk.len());
    assert!(reply.body.is_empty());

    // Only GET and HEAD reach a file (management-api-v1 §1.11).
    let reply = plain_exchange(running.addr, &request("POST", "/login/tokens.css", &[])).await;
    assert_eq!(reply.status, 405);
    assert_eq!(reply.header("allow"), Some("GET, HEAD"));
}

/// spec: dashboard.md:DASH-014 partial
#[tokio::test]
async fn protected_files_redirect_to_login_without_a_session() {
    let running = start(None, ROOMY_LM).await;

    // A missing file redirects too, so the reply does not show whether it exists.
    for path in [
        "/",
        "/app/tokens.css",
        "/app/styles/base.css",
        "/app/components/button.js",
        "/app/missing.js",
    ] {
        let reply = plain_exchange(running.addr, &get(path, &[])).await;
        assert_eq!(reply.status, 303, "GET {path}");
        assert_eq!(reply.header("location"), Some("/login/"), "GET {path}");
        assert!(reply.body.is_empty(), "GET {path} has no body");
        assert_eq!(
            reply.header("cache-control"),
            Some("no-store"),
            "GET {path}"
        );
        assert_eq!(reply.header("content-security-policy"), None, "GET {path}");
    }

    // HEAD takes the same redirect.
    let reply = plain_exchange(running.addr, &request("HEAD", "/app/tokens.css", &[])).await;
    assert_eq!(reply.status, 303);
    assert_eq!(reply.header("location"), Some("/login/"));
}

#[tokio::test]
async fn protected_files_are_served_with_a_session() {
    let running = start(None, ROOMY_LM).await;
    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    assert_eq!(login.status, 200);
    let cookie = cookie_line(&login);

    for (url, disk, content_type) in [
        (
            "/app/tokens.css",
            "app/tokens.css",
            "text/css; charset=utf-8",
        ),
        (
            "/app/styles/base.css",
            "app/styles/base.css",
            "text/css; charset=utf-8",
        ),
        (
            "/app/components/button.js",
            "app/components/button.js",
            "text/javascript; charset=utf-8",
        ),
    ] {
        let reply = plain_exchange(running.addr, &get(url, &[cookie.as_str()])).await;
        let expected = dashboard_file(disk);
        assert_eq!(reply.status, 200, "GET {url}");
        check_file_headers(&reply, content_type, expected.len());
        assert_eq!(reply.body, expected, "GET {url}");
    }

    // The app index is served at `/`. A session does not make a missing file exist.
    let index = dashboard_file("app/index.html");
    let reply = plain_exchange(running.addr, &get("/", &[cookie.as_str()])).await;
    assert_eq!(reply.status, 200, "GET /");
    check_file_headers(&reply, "text/html; charset=utf-8", index.len());
    assert_eq!(reply.body, index, "GET /");

    let reply = plain_exchange(running.addr, &get("/app/missing.js", &[cookie.as_str()])).await;
    assert_eq!(reply.status, 404, "GET /app/missing.js");
    assert_eq!(error_code(&reply), "not_found", "GET /app/missing.js");
}

#[tokio::test]
async fn preview_and_test_pages_are_never_served() {
    let running = start(None, ROOMY_LM).await;
    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    let cookie = cookie_line(&login);

    // Outside the served paths, every request redirects, with or without a session.
    for path in ["/preview/index.html", "/tests/components.html"] {
        let reply = plain_exchange(running.addr, &get(path, &[])).await;
        assert_eq!(reply.status, 303, "GET {path}");
        let reply = plain_exchange(running.addr, &get(path, &[cookie.as_str()])).await;
        assert_eq!(reply.status, 303, "GET {path} with a session");
        assert_eq!(reply.header("location"), Some("/login/"), "GET {path}");
    }
    // Under `/app/` the session decides, and the page is not in the table.
    let reply = plain_exchange(
        running.addr,
        &get("/app/preview/index.html", &[cookie.as_str()]),
    )
    .await;
    assert_eq!(reply.status, 404);
    assert_eq!(error_code(&reply), "not_found");
}

#[tokio::test]
async fn static_paths_match_the_table_exactly() {
    let running = start(None, ROOMY_LM).await;
    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    let cookie = cookie_line(&login);

    // Case, a trailing slash, and an extra segment name no file.
    for path in [
        "/login/TOKENS.CSS",
        "/login/tokens.css/",
        "/login/styles/missing.css",
    ] {
        let reply = plain_exchange(running.addr, &get(path, &[])).await;
        assert_eq!(reply.status, 404, "GET {path}");
        assert_eq!(error_code(&reply), "not_found", "GET {path}");
    }
    for path in [
        "/app/TOKENS.CSS",
        "/app/styles/",
        "/app/components/button.js/",
    ] {
        let reply = plain_exchange(running.addr, &get(path, &[cookie.as_str()])).await;
        assert_eq!(reply.status, 404, "GET {path}");
        assert_eq!(error_code(&reply), "not_found", "GET {path}");
    }
}

// Admin login and session API (management-api-v1 §1.2–§1.5).

/// Accounts of the login tests. Both are administrators.
const ACCOUNT: &str = "alice";
const PASSWORD: &str = "correct-horse-battery";
const OTHER_ACCOUNT: &str = "bob";
const OTHER_PASSWORD: &str = "another-long-password";

/// Minimal Argon2id parameters. The tests log in often, so each hash must be cheap.
const TEST_PARAMS: Argon2Params = Argon2Params {
    memory_kib: 8,
    iterations: 1,
    parallelism: 1,
};

/// The cookie that a refused or ended session clears over plain HTTP.
const CLEARED_COOKIE: &str = "relay_admin_session=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict";

/// Admin state of one listener: `alice` and `bob` with their test passwords.
/// `verifier` and `clock` are the ports that the test chooses.
fn admin_auth(
    log: Arc<dyn EventSink>,
    session: AdminSessionPolicy,
    limit: LoginLimitPolicy,
    verifier: Arc<dyn PasswordVerifier>,
    clock: Arc<dyn Clock>,
) -> Arc<AdminAuth> {
    let accounts: Vec<Account> = [(ACCOUNT, PASSWORD), (OTHER_ACCOUNT, OTHER_PASSWORD)]
        .into_iter()
        .map(|(name, password)| {
            let name = AccountName::parse(name)
                .unwrap_or_else(|error| panic!("test account name is valid: {error}"));
            let password = Password::parse(password.to_owned())
                .unwrap_or_else(|error| panic!("test password is valid: {error:?}"));
            let password_hash = Argon2Hasher
                .hash(&password, TEST_PARAMS)
                .unwrap_or_else(|error| panic!("test hash is made: {error}"));
            Account {
                name,
                password_hash,
            }
        })
        .collect();
    let dummy = dummy_hash(&Argon2Hasher, TEST_PARAMS)
        .unwrap_or_else(|error| panic!("dummy hash is made: {error}"));
    let ports = AdminPorts {
        verifier,
        tokens: Arc::new(SystemTokenSource),
        clock,
        events: log,
    };
    Arc::new(AdminAuth::new(accounts, dummy, ports, session, limit))
}

/// A clock that moves only when the test sets it. Its wall clock follows the
/// monotonic one from a fixed start.
#[derive(Default)]
struct FakeClock(Mutex<Duration>);

impl FakeClock {
    fn set(&self, now: Duration) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = now;
    }
}

impl Clock for FakeClock {
    fn monotonic(&self) -> Duration {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn wall(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(1_700_000_000) + self.monotonic()
    }
}

/// The real Argon2 check, with a count of the checks that ran.
#[derive(Default)]
struct CountingVerifier(Mutex<usize>);

impl CountingVerifier {
    fn calls(&self) -> usize {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl PasswordVerifier for CountingVerifier {
    fn verify(&self, typed: &str, hash: &PasswordHash) -> Result<bool, HashError> {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) += 1;
        Argon2Verifier.verify(typed, hash)
    }
}

/// Sends one login and checks that it ran exactly one password check. A refused
/// attempt runs the check too (management-api-v1 §6).
async fn checked_login(
    running: &Running,
    verifier: &CountingVerifier,
    account: &str,
    password: &str,
) -> Reply {
    let before = verifier.calls();
    let reply = plain_exchange(running.addr, &login_request(account, password)).await;
    assert_eq!(
        verifier.calls(),
        before + 1,
        "one password check for {account}"
    );
    reply
}

/// How many `admin_login_limited` events the listener recorded for `scope`.
fn limited_events(running: &Running, scope: LimitScope) -> usize {
    running
        .events
        .snapshot()
        .iter()
        .filter(|event| {
            matches!(event, Event::AdminLoginLimited { scope: found, .. } if *found == scope)
        })
        .count()
}

/// Asserts that two replies match in status, headers, and body. The `date`
/// header is left out, because the server stamps each reply with its own time.
fn assert_same_reply(first: &Reply, second: &Reply) {
    assert_eq!(first.status, second.status);
    let headers = |reply: &Reply| -> Vec<(String, String)> {
        reply
            .headers
            .iter()
            .filter(|(name, _)| name.as_str() != "date")
            .cloned()
            .collect()
    };
    assert_eq!(headers(first), headers(second));
    assert_eq!(first.body, second.body);
}

/// A `POST` request with a JSON body. Its length comes from the body.
fn post_json(path: &str, body: &Value, extra_headers: &[&str]) -> Vec<u8> {
    let text = body.to_string();
    let length = format!("Content-Length: {}", text.len());
    let mut headers = vec!["Content-Type: application/json", length.as_str()];
    headers.extend_from_slice(extra_headers);
    let mut bytes = request("POST", path, &headers);
    bytes.extend_from_slice(text.as_bytes());
    bytes
}

/// The login request for `account` with `password`.
fn login_request(account: &str, password: &str) -> Vec<u8> {
    post_json(
        "/api/v1/session",
        &json!({ "account": account, "password": password }),
        &[],
    )
}

/// The `Cookie` header that sends the session cookie of a login reply.
fn cookie_line(login: &Reply) -> String {
    let set_cookie = login
        .header("set-cookie")
        .unwrap_or_else(|| panic!("login sets the session cookie"));
    let pair = set_cookie.split(';').next().unwrap_or_default();
    format!("Cookie: {pair}")
}

/// The `X-CSRF-Token` header with the token of a session view.
fn csrf_line(view: &Reply) -> String {
    let body = json_body(view);
    let token = body["csrf_token"]
        .as_str()
        .unwrap_or_else(|| panic!("view has csrf_token: {body}"));
    format!("X-CSRF-Token: {token}")
}

fn json_body(reply: &Reply) -> Value {
    serde_json::from_slice(&reply.body).unwrap_or_else(|error| panic!("body is JSON: {error}"))
}

/// Checks the `Set-Cookie` of a login reply: its attributes and the token form.
fn check_session_cookie(login: &Reply, secure: bool) {
    let set_cookie = login
        .header("set-cookie")
        .unwrap_or_else(|| panic!("login sets the session cookie"));
    let (pair, attributes) = set_cookie
        .split_once(';')
        .unwrap_or_else(|| panic!("cookie has attributes: {set_cookie}"));
    let expected = if secure {
        " Path=/; HttpOnly; SameSite=Strict; Secure"
    } else {
        " Path=/; HttpOnly; SameSite=Strict"
    };
    assert_eq!(attributes, expected, "cookie attributes");
    let token = pair
        .strip_prefix("relay_admin_session=")
        .unwrap_or_else(|| panic!("cookie has the session name: {pair}"));
    assert_eq!(token.len(), 64, "token has 64 characters");
    assert!(
        token
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "token is lowercase hex"
    );
}

/// spec: management.md:MGT-017
#[tokio::test]
async fn https_and_plain_loopback_logins_set_transport_cookies_and_allow_queries() {
    for secure in [true, false] {
        let running = start(secure.then(tls_acceptor), ROOMY_LM).await;
        let exchange = async |bytes: &[u8]| {
            if secure {
                https_exchange(running.addr, &[&TLS13], bytes).await
            } else {
                plain_exchange(running.addr, bytes).await
            }
        };
        let login = exchange(&login_request(ACCOUNT, PASSWORD)).await;
        assert_eq!(login.status, 200);
        check_session_cookie(&login, secure);
        let view = json_body(&login);
        assert_eq!(view["account"], ACCOUNT);
        assert_eq!(view["dashboard"]["poll_secs"], 2);
        assert_eq!(view["dashboard"]["query_timeout_secs"], 5);
        assert_eq!(view["dashboard"]["terminate_timeout_secs"], 15);

        let cookie = cookie_line(&login);
        let reply = exchange(&get("/api/v1/session", &[&cookie])).await;
        assert_eq!(reply.status, 200);
        assert_eq!(json_body(&reply)["account"], ACCOUNT);
        let summary = exchange(&get("/api/v1/summary", &[&cookie])).await;
        assert_eq!(summary.status, 200);
        assert_eq!(json_body(&summary)["server"]["state"], "ready");
        assert_eq!(json_body(&summary)["counts"]["active"], 0);
        let sessions = exchange(&get("/api/v1/sessions", &[&cookie])).await;
        assert_eq!(sessions.status, 200);
        assert_eq!(json_body(&sessions)["items"], json!([]));
    }
}

// Covers the refusal of a wrong password and of an unknown account (MGT-018). The
// session view is the only operational data path in 4.3. The queries (4.4) take the
// same session check, so a refused login cannot open them either.
/// spec: management.md:MGT-018
#[tokio::test]
async fn wrong_password_and_unknown_account_get_the_same_refusal() {
    let verifier = Arc::new(CountingVerifier::default());
    let running = start_with_ports(
        None,
        ROOMY_LM,
        UPGRADE,
        session_policy(),
        limit_policy(),
        verifier.clone(),
        Arc::new(SystemClock::new()),
    )
    .await;

    let wrong = checked_login(&running, &verifier, ACCOUNT, "wrong-password").await;
    let unknown = checked_login(&running, &verifier, "nobody", PASSWORD).await;

    for reply in [&wrong, &unknown] {
        assert_eq!(reply.status, 401);
        assert_eq!(error_code(reply), "login_failed");
        assert_eq!(reply.header("set-cookie"), None);
    }
    assert_same_reply(&wrong, &unknown);
    let text = String::from_utf8_lossy(&wrong.body);
    for name in [ACCOUNT, "nobody"] {
        assert!(!text.contains(name), "the refusal names no account: {text}");
    }

    // With no session, the session view and the query paths of 4.4 are refused
    // by the same check, before routing.
    for path in [
        "/api/v1/session",
        "/api/v1/summary",
        "/api/v1/waiting",
        "/api/v1/sessions",
        "/api/v1/closed-sessions",
    ] {
        let reply = plain_exchange(running.addr, &get(path, &[])).await;
        assert_eq!(reply.status, 401, "{path}");
        assert_eq!(error_code(&reply), "unauthenticated", "{path}");
    }
}

// Covers the account limit, the source limit, and the end of both locks (MGT-022).
// A limited login gets the same refusal as a wrong password, for an existing and for
// an unknown account. Every attempt runs one password check. The fake clock ends the
// locks at a set time, and then only the right password gets a session. The contract's
// delay variant is not used.
/// spec: management.md:MGT-022
#[tokio::test]
async fn limited_logins_refuse_alike_and_the_right_password_works_after_the_locks() {
    // Two failures per account are allowed, and six per source. Each lock lasts 60 s.
    let limit = LoginLimitPolicy::new(2, 6, Duration::from_secs(900), Duration::from_secs(60));
    let verifier = Arc::new(CountingVerifier::default());
    let clock = Arc::new(FakeClock::default());
    let running = start_with_ports(
        None,
        ROOMY_LM,
        UPGRADE,
        session_policy(),
        limit,
        verifier.clone(),
        clock.clone(),
    )
    .await;

    // The third failure for alice starts her lock. Every failure gets the same refusal.
    let ordinary = checked_login(&running, &verifier, ACCOUNT, "wrong-password").await;
    assert_eq!(ordinary.status, 401);
    assert_eq!(error_code(&ordinary), "login_failed");
    for _ in 0..2 {
        let reply = checked_login(&running, &verifier, ACCOUNT, "wrong-password").await;
        assert_same_reply(&ordinary, &reply);
    }
    assert_eq!(limited_events(&running, LimitScope::Account), 1);

    // The right password is refused during the lock, and the refusal is not counted.
    let locked = checked_login(&running, &verifier, ACCOUNT, PASSWORD).await;
    assert_same_reply(&ordinary, &locked);

    // An unknown name is limited the same way: its third failure starts its lock.
    for _ in 0..3 {
        let reply = checked_login(&running, &verifier, "nobody", "wrong-password").await;
        assert_same_reply(&ordinary, &reply);
    }
    assert_eq!(limited_events(&running, LimitScope::Account), 2);
    // Six failures from this source do not exceed the limit of six.
    assert_eq!(limited_events(&running, LimitScope::Source), 0);
    let nobody_locked = checked_login(&running, &verifier, "nobody", PASSWORD).await;
    assert_same_reply(&ordinary, &nobody_locked);

    // The seventh failure from this source starts the source lock. bob has no
    // failure of his own.
    let tipped = checked_login(&running, &verifier, OTHER_ACCOUNT, "wrong-password").await;
    assert_same_reply(&ordinary, &tipped);
    assert_eq!(limited_events(&running, LimitScope::Source), 1);

    // The source lock refuses bob's right password too.
    let refused = checked_login(&running, &verifier, OTHER_ACCOUNT, OTHER_PASSWORD).await;
    assert_same_reply(&ordinary, &refused);

    // Both locks end at 60 s. Then only the right password gets a session.
    clock.set(Duration::from_secs(61));
    let wrong = checked_login(&running, &verifier, ACCOUNT, "wrong-password").await;
    assert_same_reply(&ordinary, &wrong);
    let unknown = checked_login(&running, &verifier, "nobody", PASSWORD).await;
    assert_same_reply(&ordinary, &unknown);
    let bob = checked_login(&running, &verifier, OTHER_ACCOUNT, OTHER_PASSWORD).await;
    assert_eq!(bob.status, 200);
    check_session_cookie(&bob, false);
    let alice = checked_login(&running, &verifier, ACCOUNT, PASSWORD).await;
    assert_eq!(alice.status, 200);
    check_session_cookie(&alice, false);

    // No new lock started in the meantime.
    assert_eq!(limited_events(&running, LimitScope::Account), 2);
    assert_eq!(limited_events(&running, LimitScope::Source), 1);
}

fn assert_unauthenticated(reply: &Reply) {
    assert_eq!(reply.status, 401);
    assert_eq!(error_code(reply), "unauthenticated");
    assert_eq!(
        reply.header("www-authenticate"),
        Some(r#"Session realm="vnc-relay""#)
    );
    assert_eq!(reply.header("set-cookie"), Some(CLEARED_COOKIE));
}

/// spec: management.md:MGT-019
#[tokio::test]
async fn expired_admin_cookie_cannot_query_or_terminate_and_the_relay_survives() {
    let session = AdminSessionPolicy::new(Duration::from_millis(300), Duration::from_secs(3600));
    let relay = RunningRelay::start(session).await;
    let mut pair = relay.pair().await;
    let running = &relay.management;
    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    assert_eq!(login.status, 200);
    let cookie = cookie_line(&login);
    let csrf = csrf_line(&login);
    tokio::time::sleep(Duration::from_millis(500)).await;

    for bytes in [
        get("/api/v1/session", &[&cookie]),
        get("/api/v1/summary", &[&cookie]),
        get("/api/v1/sessions", &[&cookie]),
        request(
            "POST",
            &format!("/api/v1/sessions/{}/terminate", pair.session_id),
            &[&cookie, &csrf],
        ),
        request("DELETE", "/api/v1/session", &[&cookie, &csrf]),
    ] {
        let reply = plain_exchange(running.addr, &bytes).await;
        assert_unauthenticated(&reply);
    }
    pair.assert_bidirectional().await;
    pair.assert_listed_and_terminate(running).await;
}

/// spec: management.md:MGT-025
#[tokio::test]
async fn idle_polling_and_absolute_activity_expiry_leave_existing_relays_alive() {
    let idle = Duration::from_millis(400);
    let max_lifetime = Duration::from_millis(1200);
    for automatic in [true, false] {
        let relay = RunningRelay::start(AdminSessionPolicy::new(idle, max_lifetime)).await;
        let mut pair = relay.pair().await;
        let running = &relay.management;
        let started = Instant::now();
        let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
        assert_eq!(login.status, 200);
        let initial = json_body(&login);
        let cookie = cookie_line(&login);
        let mut headers = vec![cookie.as_str()];
        if automatic {
            headers.push("X-Relay-Poll: 1");
        }
        let mut refused = None;
        let mut last_success = Instant::now();
        let mut successes = 0;
        let mut active_past_idle = false;
        // The dashboard polls the query paths, so every request kind takes a turn.
        // Only the session view reports the deadlines.
        let paths = [
            "/api/v1/session",
            "/api/v1/summary",
            "/api/v1/waiting",
            "/api/v1/sessions",
            "/api/v1/closed-sessions",
        ];
        for round in 0..500 {
            tokio::time::sleep(Duration::from_millis(10)).await;
            let path = paths[round % paths.len()];
            let reply = plain_exchange(running.addr, &get(path, &headers)).await;
            assert!(
                last_success.elapsed() < idle,
                "requests stay frequent enough to avoid an idle gap"
            );
            if reply.status == 401 {
                refused = Some(reply);
                break;
            }
            assert_eq!(reply.status, 200, "{path} succeeds before expiry");
            successes += 1;
            last_success = Instant::now();
            if path != "/api/v1/session" {
                continue;
            }
            let view = json_body(&reply);
            assert_eq!(view["absolute_expires_at"], initial["absolute_expires_at"]);
            if automatic {
                assert_eq!(view["idle_expires_at"], initial["idle_expires_at"]);
            } else if started.elapsed() > idle {
                active_past_idle = true;
                assert_ne!(view["idle_expires_at"], initial["idle_expires_at"]);
            }
        }
        let refused = refused.unwrap_or_else(|| panic!("the admin session expires in time"));
        assert_unauthenticated(&refused);
        assert!(successes > 1, "several requests succeeded before expiry");
        if automatic {
            assert!(started.elapsed() >= idle, "polls expire after Si");
            assert!(started.elapsed() < max_lifetime, "polls expire before Sa");
        } else {
            assert!(active_past_idle, "user actions extend the idle deadline");
            assert!(
                started.elapsed() >= max_lifetime,
                "activity still expires at Sa"
            );
        }
        pair.assert_bidirectional().await;
        pair.assert_listed_and_terminate(running).await;
    }
}

// Covers the logout of one admin session. The relay sessions that stay open need a
// running relay, which this listener test does not start.
/// spec: management.md:MGT-020 partial
#[tokio::test]
async fn logout_ends_only_its_own_session() {
    let running = start(None, ROOMY_LM).await;

    let first = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    let second = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    assert_eq!(first.status, 200);
    assert_eq!(second.status, 200);
    let first_cookie = cookie_line(&first);
    let first_csrf = csrf_line(&first);
    let second_cookie = cookie_line(&second);

    let logout = plain_exchange(
        running.addr,
        &request(
            "DELETE",
            "/api/v1/session",
            &[first_cookie.as_str(), first_csrf.as_str()],
        ),
    )
    .await;
    assert_eq!(logout.status, 204);
    assert_eq!(logout.header("set-cookie"), Some(CLEARED_COOKIE));
    assert_eq!(logout.header("cache-control"), Some("no-store"));
    assert!(logout.body.is_empty());

    let reused = plain_exchange(
        running.addr,
        &get("/api/v1/session", &[first_cookie.as_str()]),
    )
    .await;
    assert_eq!(reused.status, 401);
    let kept = plain_exchange(
        running.addr,
        &get("/api/v1/session", &[second_cookie.as_str()]),
    )
    .await;
    assert_eq!(kept.status, 200);
}

// Covers logout CSRF only. Termination CSRF is checked by its route tests.
/// spec: management.md:MGT-014 partial
#[tokio::test]
async fn logout_without_the_csrf_token_is_refused_and_keeps_the_session() {
    let running = start(None, ROOMY_LM).await;
    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    let cookie = cookie_line(&login);
    let csrf = csrf_line(&login);

    let missing = plain_exchange(
        running.addr,
        &request("DELETE", "/api/v1/session", &[cookie.as_str()]),
    )
    .await;
    assert_eq!(missing.status, 403);
    assert_eq!(error_code(&missing), "csrf_invalid");

    let wrong = plain_exchange(
        running.addr,
        &request(
            "DELETE",
            "/api/v1/session",
            &[cookie.as_str(), "X-CSRF-Token: not-the-token"],
        ),
    )
    .await;
    assert_eq!(wrong.status, 403);
    assert_eq!(error_code(&wrong), "csrf_invalid");

    let view = plain_exchange(running.addr, &get("/api/v1/session", &[cookie.as_str()])).await;
    assert_eq!(view.status, 200, "refused logouts keep the session");

    let logout = plain_exchange(
        running.addr,
        &request(
            "DELETE",
            "/api/v1/session",
            &[cookie.as_str(), csrf.as_str()],
        ),
    )
    .await;
    assert_eq!(logout.status, 204);
}

// Simulates the restart with a second listener and fresh admin state. A real process
// restart is not tested here.
/// spec: management.md:MGT-027 partial
#[tokio::test]
async fn cookie_from_before_a_restart_is_refused_and_login_works_again() {
    let before = start(None, ROOMY_LM).await;
    let login = plain_exchange(before.addr, &login_request(ACCOUNT, PASSWORD)).await;
    let old_cookie = cookie_line(&login);
    let old_csrf = csrf_line(&login);

    // The restarted listener starts with no sessions.
    let after = start(None, ROOMY_LM).await;
    let stale = plain_exchange(after.addr, &get("/api/v1/session", &[old_cookie.as_str()])).await;
    assert_eq!(stale.status, 401);
    assert_eq!(error_code(&stale), "unauthenticated");
    let stale_logout = plain_exchange(
        after.addr,
        &request(
            "DELETE",
            "/api/v1/session",
            &[old_cookie.as_str(), old_csrf.as_str()],
        ),
    )
    .await;
    assert_eq!(stale_logout.status, 401);

    let relogin = plain_exchange(after.addr, &login_request(ACCOUNT, PASSWORD)).await;
    assert_eq!(relogin.status, 200);
    let new_cookie = cookie_line(&relogin);
    let view = plain_exchange(after.addr, &get("/api/v1/session", &[new_cookie.as_str()])).await;
    assert_eq!(view.status, 200);
}

// Covers the logins of both admin accounts. The local CLI account setup and the query
// and terminate permissions (4.4, 4.5) are not covered here.
/// spec: operations.md:OPS-029 partial
#[tokio::test]
async fn each_admin_account_logs_in_to_the_same_session_view() {
    let running = start(None, ROOMY_LM).await;

    for (account, password) in [(ACCOUNT, PASSWORD), (OTHER_ACCOUNT, OTHER_PASSWORD)] {
        let login = plain_exchange(running.addr, &login_request(account, password)).await;
        assert_eq!(login.status, 200, "{account} logs in");
        assert_eq!(json_body(&login)["account"], account);
        let cookie = cookie_line(&login);
        let view = plain_exchange(running.addr, &get("/api/v1/session", &[cookie.as_str()])).await;
        assert_eq!(view.status, 200, "{account} opens the session view");
        assert_eq!(json_body(&view)["account"], account);
    }
}

#[tokio::test]
async fn login_body_must_be_a_json_account_and_password() {
    let running = start(None, ROOMY_LM).await;

    let mut text_plain = request(
        "POST",
        "/api/v1/session",
        &["Content-Type: text/plain", "Content-Length: 2"],
    );
    text_plain.extend_from_slice(b"{}");
    let reply = plain_exchange(running.addr, &text_plain).await;
    assert_eq!(reply.status, 415);
    assert_eq!(error_code(&reply), "invalid_request");

    for body in [
        json!({ "account": ACCOUNT }),
        json!({ "account": ACCOUNT, "password": "" }),
        json!("alice"),
    ] {
        let reply = plain_exchange(running.addr, &post_json("/api/v1/session", &body, &[])).await;
        assert_eq!(reply.status, 400, "body {body}");
        assert_eq!(error_code(&reply), "invalid_request");
    }

    // A charset parameter does not change the media type.
    let body = json!({ "account": ACCOUNT, "password": PASSWORD }).to_string();
    let length = format!("Content-Length: {}", body.len());
    let mut bytes = request(
        "POST",
        "/api/v1/session",
        &[
            "Content-Type: application/json; charset=utf-8",
            length.as_str(),
        ],
    );
    bytes.extend_from_slice(body.as_bytes());
    let reply = plain_exchange(running.addr, &bytes).await;
    assert_eq!(reply.status, 200);
}

#[tokio::test]
async fn valid_session_gets_not_found_or_refusal_for_other_api_paths() {
    let running = start(None, ROOMY_LM).await;
    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    let cookie = cookie_line(&login);

    let unknown = plain_exchange(running.addr, &get("/api/v1/unknown", &[cookie.as_str()])).await;
    assert_eq!(unknown.status, 404);
    assert_eq!(error_code(&unknown), "not_found");

    let method = plain_exchange(
        running.addr,
        &request("PUT", "/api/v1/session", &[cookie.as_str()]),
    )
    .await;
    assert_eq!(method.status, 405);
    assert_eq!(method.header("allow"), Some("GET, POST, DELETE"));

    let poll = plain_exchange(
        running.addr,
        &get("/api/v1/session", &[cookie.as_str(), "X-Relay-Poll: 0"]),
    )
    .await;
    assert_eq!(poll.status, 400);
    assert_eq!(error_code(&poll), "invalid_request");
}

/// The body of a chunked request: one chunk, then the last chunk.
fn chunked(body: &str) -> Vec<u8> {
    format!("{:x}\r\n{body}\r\n0\r\n\r\n", body.len()).into_bytes()
}

/// A login request whose body is sent with `Content-Length`.
fn sized_login(text: &str) -> Vec<u8> {
    let length = format!("Content-Length: {}", text.len());
    let mut bytes = request(
        "POST",
        "/api/v1/session",
        &["Content-Type: application/json", length.as_str()],
    );
    bytes.extend_from_slice(text.as_bytes());
    bytes
}

/// A login request whose body is sent with chunked framing.
fn chunked_login(text: &str) -> Vec<u8> {
    let mut bytes = request(
        "POST",
        "/api/v1/session",
        &[
            "Content-Type: application/json",
            "Transfer-Encoding: chunked",
        ],
    );
    bytes.extend_from_slice(&chunked(text));
    bytes
}

/// A login body of exactly `size` bytes. The `pad` field fills the rest.
fn login_body_of(size: usize) -> String {
    let bare = json!({ "account": ACCOUNT, "password": PASSWORD, "pad": "" }).to_string();
    let text = json!({
        "account": ACCOUNT,
        "password": PASSWORD,
        "pad": "x".repeat(size - bare.len()),
    })
    .to_string();
    assert_eq!(text.len(), size);
    text
}

// Covers the session view's `as_of` (management-api-v1 §1.1). The aggregate
// consistency of the operational queries is checked in 4.4.
/// spec: management.md:MGT-017 partial
#[tokio::test]
async fn session_view_carries_as_of_as_a_timestamp() {
    let running = start(None, ROOMY_LM).await;
    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    let cookie = cookie_line(&login);
    let view = plain_exchange(running.addr, &get("/api/v1/session", &[cookie.as_str()])).await;
    assert_eq!(view.status, 200);
    let body = json_body(&view);
    let as_of = body["as_of"]
        .as_str()
        .unwrap_or_else(|| panic!("view has as_of: {body}"));
    // Same form as the other timestamps: UTC, milliseconds, and `Z`.
    assert_eq!(as_of.len(), 24, "{as_of}");
    assert!(as_of.ends_with('Z'), "{as_of}");
}

// Covers the media type of bodies on GET and DELETE (management-api-v1 §1.1).
// A body that is not `application/json` is refused with 415 and changes nothing.
#[tokio::test]
async fn get_and_delete_with_a_body_must_send_application_json() {
    let running = start(None, ROOMY_LM).await;
    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    let cookie = cookie_line(&login);
    let view = plain_exchange(running.addr, &get("/api/v1/session", &[cookie.as_str()])).await;
    let csrf = csrf_line(&view);
    let text = "{}";
    let length = format!("Content-Length: {}", text.len());

    let bare = request(
        "GET",
        "/api/v1/session",
        &[cookie.as_str(), "Content-Type: text/plain"],
    );
    let reply = plain_exchange(running.addr, &bare).await;
    assert_eq!(reply.status, 200, "text/plain without a body is accepted");

    let mut sized = request(
        "GET",
        "/api/v1/session",
        &[cookie.as_str(), "Content-Type: text/plain", length.as_str()],
    );
    sized.extend_from_slice(text.as_bytes());
    let reply = plain_exchange(running.addr, &sized).await;
    assert_eq!(reply.status, 415);
    assert_eq!(error_code(&reply), "invalid_request");

    let mut streamed = request(
        "GET",
        "/api/v1/session",
        &[
            cookie.as_str(),
            "Content-Type: text/plain",
            "Transfer-Encoding: chunked",
        ],
    );
    streamed.extend_from_slice(&chunked(text));
    let reply = plain_exchange(running.addr, &streamed).await;
    assert_eq!(reply.status, 415);
    assert_eq!(error_code(&reply), "invalid_request");

    let mut delete = request(
        "DELETE",
        "/api/v1/session",
        &[
            cookie.as_str(),
            csrf.as_str(),
            "Content-Type: text/plain",
            length.as_str(),
        ],
    );
    delete.extend_from_slice(text.as_bytes());
    let reply = plain_exchange(running.addr, &delete).await;
    assert_eq!(reply.status, 415);
    assert_eq!(error_code(&reply), "invalid_request");

    // A refused request changes nothing: the session still answers.
    let still = plain_exchange(running.addr, &get("/api/v1/session", &[cookie.as_str()])).await;
    assert_eq!(still.status, 200);
}

// Covers the 400 for a name that cannot be an account (management-api-v1 §1.5
// and §2.1). The refusal for names that follow the grammar is checked by the
// app login tests.
/// spec: management.md:MGT-018 partial
#[tokio::test]
async fn login_account_outside_the_name_grammar_is_a_bad_request() {
    let running = start(None, ROOMY_LM).await;
    let long = "x".repeat(33);
    for account in ["Alice", "al ice", "bob!", long.as_str()] {
        let reply = plain_exchange(running.addr, &login_request(account, PASSWORD)).await;
        assert_eq!(reply.status, 400, "{account}");
        assert_eq!(error_code(&reply), "invalid_request");
        assert!(reply.header("set-cookie").is_none(), "{account}");
    }
}

// Covers the 16 KiB limit on the login body (management-api-v1 §1.5) for both
// framings. A body one byte over the limit is refused with 400.
#[tokio::test]
async fn login_body_limit_is_16_kib_for_sized_and_chunked_bodies() {
    let running = start(None, ROOMY_LM).await;
    let fitting = login_body_of(16_384);
    let over = login_body_of(16_385);

    let reply = plain_exchange(running.addr, &sized_login(&fitting)).await;
    assert_eq!(reply.status, 200);
    let reply = plain_exchange(running.addr, &chunked_login(&fitting)).await;
    assert_eq!(reply.status, 200);

    for bytes in [sized_login(&over), chunked_login(&over)] {
        let reply = plain_exchange(running.addr, &bytes).await;
        assert_eq!(reply.status, 400);
        assert_eq!(error_code(&reply), "invalid_request");
    }
}

// A 400 for the body is not counted (management-api-v1 §2.1). With a limit of
// 2 counted failures per source, three malformed bodies must not lock it.
// Covers the not-counted rule only; response timing is not covered.
/// spec: management.md:MGT-022 partial
#[tokio::test]
async fn malformed_login_bodies_are_not_counted_against_the_limit() {
    let limit = LoginLimitPolicy::new(2, 2, Duration::from_secs(60), Duration::from_secs(60));
    let running = start_with(None, ROOMY_LM, UPGRADE, session_policy(), limit).await;
    for _ in 0..3 {
        let reply = plain_exchange(running.addr, &sized_login("{")).await;
        assert_eq!(reply.status, 400);
        assert_eq!(error_code(&reply), "invalid_request");
    }
    let reply = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    assert_eq!(reply.status, 200);
}

#[tokio::test]
async fn x_relay_poll_is_refused_on_login_and_logout() {
    let running = start(None, ROOMY_LM).await;
    let body = json!({ "account": ACCOUNT, "password": PASSWORD });
    let reply = plain_exchange(
        running.addr,
        &post_json("/api/v1/session", &body, &["X-Relay-Poll: 1"]),
    )
    .await;
    assert_eq!(reply.status, 400);
    assert_eq!(error_code(&reply), "invalid_request");
    assert!(
        reply.header("set-cookie").is_none(),
        "a refused login sets no cookie"
    );

    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    let cookie = cookie_line(&login);
    let view = plain_exchange(running.addr, &get("/api/v1/session", &[cookie.as_str()])).await;
    let csrf = csrf_line(&view);
    let reply = plain_exchange(
        running.addr,
        &request(
            "DELETE",
            "/api/v1/session",
            &[cookie.as_str(), csrf.as_str(), "X-Relay-Poll: 1"],
        ),
    )
    .await;
    assert_eq!(reply.status, 400);
    assert_eq!(error_code(&reply), "invalid_request");
    let still = plain_exchange(running.addr, &get("/api/v1/session", &[cookie.as_str()])).await;
    assert_eq!(still.status, 200, "a refused logout keeps the session");
}

#[tokio::test]
async fn a_login_with_a_valid_prior_cookie_ends_that_session() {
    let running = start(None, ROOMY_LM).await;
    let first = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    let old = cookie_line(&first);
    let second = plain_exchange(
        running.addr,
        &post_json(
            "/api/v1/session",
            &json!({ "account": ACCOUNT, "password": PASSWORD }),
            &[old.as_str()],
        ),
    )
    .await;
    assert_eq!(second.status, 200);
    let new = cookie_line(&second);

    let refused = plain_exchange(running.addr, &get("/api/v1/session", &[old.as_str()])).await;
    assert_eq!(refused.status, 401);
    assert_eq!(error_code(&refused), "unauthenticated");
    let kept = plain_exchange(running.addr, &get("/api/v1/session", &[new.as_str()])).await;
    assert_eq!(kept.status, 200);
}

#[tokio::test]
async fn terminate_dispatch_checks_auth_csrf_path_method_then_poll_and_id() {
    let running = start(None, ROOMY_LM).await;
    let target = "/api/v1/sessions/unknown/terminate";
    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    let cookie = cookie_line(&login);
    let csrf = csrf_line(&login);
    for method in ["POST", "DELETE"] {
        for path in [target, "/api/v1/unknown"] {
            let unauthenticated = plain_exchange(running.addr, &request(method, path, &[])).await;
            assert_eq!(unauthenticated.status, 401);
            let missing =
                plain_exchange(running.addr, &request(method, path, &[cookie.as_str()])).await;
            assert_eq!(missing.status, 403);
            assert_eq!(error_code(&missing), "csrf_invalid");
            let repeated = plain_exchange(
                running.addr,
                &request(
                    method,
                    path,
                    &[cookie.as_str(), csrf.as_str(), csrf.as_str()],
                ),
            )
            .await;
            assert_eq!(repeated.status, 403);
        }
    }
    for method in ["GET", "HEAD", "PUT", "DELETE"] {
        let reply = plain_exchange(
            running.addr,
            &request(
                method,
                target,
                &[cookie.as_str(), csrf.as_str(), "X-Relay-Poll: invalid"],
            ),
        )
        .await;
        assert_eq!(reply.status, 405);
        assert_eq!(reply.header("allow"), Some("POST"));
    }
    let unknown = plain_exchange(
        running.addr,
        &request("POST", "/api/v1/unknown", &[cookie.as_str(), csrf.as_str()]),
    )
    .await;
    assert_eq!(unknown.status, 404);
    let extra_segment = plain_exchange(
        running.addr,
        &request(
            "POST",
            "/api/v1/sessions/unknown/terminate/",
            &[cookie.as_str(), csrf.as_str()],
        ),
    )
    .await;
    assert_eq!(extra_segment.status, 404);
    let poll = plain_exchange(
        running.addr,
        &request(
            "POST",
            "/api/v1/sessions/!/terminate",
            &[cookie.as_str(), csrf.as_str(), "X-Relay-Poll: 1"],
        ),
    )
    .await;
    assert_eq!(poll.status, 400);
    assert!(
        json_body(&poll)["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("X-Relay-Poll"))
    );
    let invalid_id = plain_exchange(
        running.addr,
        &request(
            "POST",
            "/api/v1/sessions/!/terminate",
            &[cookie.as_str(), csrf.as_str()],
        ),
    )
    .await;
    assert_eq!(invalid_id.status, 400);
    assert!(
        running
            .events
            .snapshot()
            .iter()
            .all(|event| !matches!(event, Event::AdminTerminate { .. }))
    );
}

#[tokio::test]
async fn terminate_rejects_all_actual_bodies_but_allows_empty_content_types_without_touching_auth()
{
    let clock = Arc::new(FakeClock::default());
    let running = start_with_ports(
        None,
        ROOMY_LM,
        UPGRADE,
        session_policy(),
        limit_policy(),
        Arc::new(Argon2Verifier),
        clock.clone(),
    )
    .await;
    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    let cookie = cookie_line(&login);
    let csrf = csrf_line(&login);
    clock.set(Duration::from_secs(599));
    let path = "/api/v1/sessions/unknown/terminate";
    for content_type in [
        None,
        Some("Content-Type: application/json"),
        Some("Content-Type: text/plain"),
    ] {
        for chunked in [false, true] {
            let mut headers = vec![cookie.as_str(), csrf.as_str()];
            if let Some(content_type) = content_type {
                headers.push(content_type);
            }
            headers.push(if chunked {
                "Transfer-Encoding: chunked"
            } else {
                "Content-Length: 1"
            });
            let mut bytes = request("POST", path, &headers);
            bytes.extend_from_slice(if chunked {
                b"1\r\nx\r\n0\r\n\r\n"
            } else {
                b"x"
            });
            let reply = plain_exchange(running.addr, &bytes).await;
            assert_eq!(
                reply.status, 400,
                "body {content_type:?}, chunked={chunked}"
            );
            assert_eq!(error_code(&reply), "invalid_request");
        }
    }
    assert!(
        running
            .events
            .snapshot()
            .iter()
            .all(|event| !matches!(event, Event::AdminTerminate { .. }))
    );
    for content_type in [
        None,
        Some("Content-Type: application/json"),
        Some("Content-Type: text/plain"),
    ] {
        let mut headers = vec![cookie.as_str(), csrf.as_str(), "Content-Length: 0"];
        if let Some(content_type) = content_type {
            headers.push(content_type);
        }
        let reply = plain_exchange(running.addr, &request("POST", path, &headers)).await;
        assert_eq!(
            reply.status, 404,
            "empty {content_type:?} is accepted then missing"
        );
        assert_eq!(error_code(&reply), "not_found");
    }
    clock.set(Duration::from_secs(600));
    let expired = plain_exchange(running.addr, &get("/api/v1/session", &[cookie.as_str()])).await;
    assert_eq!(
        expired.status, 401,
        "neither rejection nor missing target touches Si"
    );
}

async fn acknowledge_pair_close(pair: &mut RelayPair) {
    async fn acknowledge(stream: &mut TcpStream) {
        let mut remaining = Vec::new();
        stream
            .read_to_end(&mut remaining)
            .await
            .unwrap_or_else(|error| panic!("relay sends terminal EOF: {error}"));
        assert!(
            remaining.is_empty(),
            "all test payload was already received"
        );
        stream
            .shutdown()
            .await
            .unwrap_or_else(|error| panic!("relay peer acknowledges close: {error}"));
    }
    timeout(LIMIT, async {
        tokio::join!(acknowledge(&mut pair.viewer), acknowledge(&mut pair.server));
    })
    .await
    .unwrap_or_else(|error| panic!("both relay peers close in time: {error}"));
}

/// spec: management.md:MGT-012
#[tokio::test]
async fn completion_mgt012_failed_control_and_lost_response_never_invent_completion() {
    let relay =
        RunningRelay::start_with_control(session_policy(), true, Arc::new(Argon2Verifier)).await;
    let running = &relay.management;
    let mut pair = relay.pair().await;
    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    assert_eq!(login.status, 200);
    let cookie = cookie_line(&login);
    let csrf = csrf_line(&login);
    let path = format!("/api/v1/sessions/{}/terminate", pair.session_id);
    let command = request("POST", &path, &[&cookie, &csrf]);

    // The fake loses only the completion channel. The real listener, auth,
    // terminate service, error serializer, query source and event sink run.
    let failed = plain_exchange(running.addr, &command).await;
    assert_eq!(failed.status, 500);
    assert_eq!(error_code(&failed), "terminate_failed");
    assert_eq!(failed.header("cache-control"), Some("no-store"));
    let body = json_body(&failed);
    assert!(body.get("closed_session").is_none());
    assert!(body.get("result").is_none());
    let audit = wait_for(&running.events, |event| {
        matches!(event, Event::AdminTerminate {
            admin_account, peer_ip, session_id: Some(id), result: AdminTerminateResult::Failed
        } if admin_account.as_str() == ACCOUNT && *peer_ip == loopback()
            && id.as_str() == pair.session_id)
    })
    .await;
    assert_eq!(audit.level(), Level::Error);
    let detail = plain_exchange(
        running.addr,
        &get(&format!("/api/v1/sessions/{}", pair.session_id), &[&cookie]),
    )
    .await;
    assert_eq!(detail.status, 200);
    assert_eq!(json_body(&detail)["session"]["state"], "active");
    let closed = plain_exchange(running.addr, &get("/api/v1/closed-sessions", &[&cookie])).await;
    assert_eq!(closed.status, 200);
    assert_eq!(json_body(&closed)["items"], json!([]));
    assert!(
        running.events.snapshot().iter().all(|event| !matches!(
            event,
            Event::SessionClosed { .. }
                | Event::AdminTerminate {
                    result: AdminTerminateResult::Terminated | AdminTerminateResult::AlreadyClosed,
                    ..
                }
        )),
        "failed completion neither closes nor audits success"
    );
    pair.assert_bidirectional().await;

    // The second request delegates to the real runtime. Drop HTTP before any
    // response read, while relay peers still prevent completion. Their EOF
    // acknowledgements below release cleanup without an HTTP consumer.
    let mut lost = TcpStream::connect(running.addr)
        .await
        .unwrap_or_else(|error| panic!("lost-response client connects: {error}"));
    lost.write_all(&command)
        .await
        .unwrap_or_else(|error| panic!("lost-response request is sent: {error}"));
    // Observe relay-side EOF before disconnecting HTTP: write_all alone does
    // not prove the listener accepted the command on every operating system.
    let mut unexpected = [0; 1];
    let eof = timeout(LIMIT, pair.viewer.read(&mut unexpected))
        .await
        .unwrap_or_else(|error| panic!("accepted termination closes relay write side: {error}"))
        .unwrap_or_else(|error| panic!("relay termination EOF: {error}"));
    assert_eq!(
        eof, 0,
        "the relay accepted termination without an HTTP read"
    );
    lost.shutdown()
        .await
        .unwrap_or_else(|error| panic!("lost-response client closes write side: {error}"));
    drop(lost);
    acknowledge_pair_close(&mut pair).await;
    wait_for(&running.events, |event| {
        matches!(event, Event::AdminTerminate {
            admin_account, peer_ip, session_id: Some(id), result: AdminTerminateResult::Terminated
        } if admin_account.as_str() == ACCOUNT && *peer_ip == loopback()
            && id.as_str() == pair.session_id)
    })
    .await;
    let active = plain_exchange(running.addr, &get("/api/v1/sessions", &[&cookie])).await;
    assert_eq!(active.status, 200);
    assert_eq!(json_body(&active)["items"], json!([]));
    let detail = plain_exchange(
        running.addr,
        &get(&format!("/api/v1/sessions/{}", pair.session_id), &[&cookie]),
    )
    .await;
    assert_eq!(detail.status, 404);
    assert_eq!(error_code(&detail), "not_found");
    let closed = plain_exchange(running.addr, &get("/api/v1/closed-sessions", &[&cookie])).await;
    assert_eq!(closed.status, 200);
    let body = json_body(&closed);
    assert_eq!(body["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(body["items"][0]["session_id"], pair.session_id);
    assert_eq!(body["items"][0]["close_cause"], "admin_terminated");
    let events = running.events.snapshot();
    let closures: Vec<_> = events
        .iter()
        .enumerate()
        .filter(|(_, event)| matches!(event, Event::SessionClosed { .. }))
        .collect();
    assert_eq!(closures.len(), 1);
    assert!(matches!(closures[0].1, Event::SessionClosed {
        session_id, admin_account: Some(account), close_cause, ..
    } if session_id.as_str() == pair.session_id && account.as_str() == ACCOUNT
        && *close_cause == vnc_relay_domain::session::CloseCause::AdminTerminated));
    let successes: Vec<_> = events
        .iter()
        .enumerate()
        .filter(|(_, event)| {
            matches!(
                event,
                Event::AdminTerminate {
                    result: AdminTerminateResult::Terminated | AdminTerminateResult::AlreadyClosed,
                    ..
                }
            )
        })
        .collect();
    assert_eq!(
        successes.len(),
        1,
        "only confirmed cleanup is audited successful"
    );
    assert!(
        closures[0].0 < successes[0].0,
        "cleanup precedes success audit"
    );
}

/// spec: management.md:MGT-013
#[tokio::test]
async fn completion_mgt013_query_auth_and_failed_termination_responses_keep_secrets_private() {
    const PAYLOAD: &[u8] = b"MGT013_UNIQUE_RAW_RFB_PAYLOAD_95_738291";
    let verifier = Arc::new(HashRecordingVerifier::default());
    let relay = RunningRelay::start_with_control(session_policy(), true, verifier.clone()).await;
    let running = &relay.management;
    let mut pair = relay.pair().await;
    timeout(LIMIT, async {
        pair.viewer
            .write_all(PAYLOAD)
            .await
            .unwrap_or_else(|error| panic!("secret payload sent: {error}"));
        let mut received = vec![0; PAYLOAD.len()];
        pair.server
            .read_exact(&mut received)
            .await
            .unwrap_or_else(|error| panic!("secret payload relayed: {error}"));
        assert_eq!(received, PAYLOAD);
    })
    .await
    .unwrap_or_else(|error| panic!("secret payload crosses real relay: {error}"));
    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    assert_eq!(login.status, 200);
    let hash = verifier
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    assert!(
        hash.starts_with("$argon2id$"),
        "the actual password hash is known"
    );
    let cookie = cookie_line(&login);
    let csrf = csrf_line(&login);
    let session_token = cookie
        .strip_prefix("Cookie: relay_admin_session=")
        .unwrap_or_else(|| panic!("known session cookie token"));
    let csrf_token = csrf
        .strip_prefix("X-CSRF-Token: ")
        .unwrap_or_else(|| panic!("known CSRF token"));
    assert_eq!(session_token.len(), 64);
    assert_eq!(csrf_token.len(), 64);
    let detail_path = format!("/api/v1/sessions/{}", pair.session_id);
    let terminate_path = format!("{detail_path}/terminate");
    let session = plain_exchange(running.addr, &get("/api/v1/session", &[&cookie])).await;
    assert_eq!(session.status, 200);
    assert_eq!(json_body(&session)["csrf_token"], csrf_token);
    let mut responses = vec![("login", login), ("session", session)];
    for path in [
        "/api/v1/summary",
        "/api/v1/waiting",
        "/api/v1/sessions",
        detail_path.as_str(),
        "/api/v1/closed-sessions",
    ] {
        let reply = plain_exchange(running.addr, &get(path, &[&cookie])).await;
        assert_eq!(reply.status, 200, "{path}");
        responses.push(("query", reply));
    }
    for (label, wire, status, code) in [
        (
            "login_failed",
            login_request(ACCOUNT, OTHER_PASSWORD),
            401,
            "login_failed",
        ),
        (
            "unauthenticated",
            get("/api/v1/summary", &[]),
            401,
            "unauthenticated",
        ),
        (
            "csrf_invalid",
            request("POST", &terminate_path, &[&cookie]),
            403,
            "csrf_invalid",
        ),
        (
            "invalid_request",
            get("/api/v1/summary", &[&cookie, "X-Relay-Poll: invalid"]),
            400,
            "invalid_request",
        ),
        (
            "not_found",
            get("/api/v1/not-a-query", &[&cookie]),
            404,
            "not_found",
        ),
        (
            "missing_detail",
            get(
                "/api/v1/sessions/ses_aaaaaaaaaaaaaaaaaaaaaaaaaa",
                &[&cookie],
            ),
            404,
            "not_found",
        ),
        (
            "method",
            request("POST", "/api/v1/summary", &[&cookie, &csrf]),
            405,
            "invalid_request",
        ),
        (
            "terminate_failed",
            request("POST", &terminate_path, &[&cookie, &csrf]),
            500,
            "terminate_failed",
        ),
    ] {
        let reply = plain_exchange(running.addr, &wire).await;
        assert_eq!(reply.status, status, "{label}");
        assert_eq!(error_code(&reply), code, "{label}");
        responses.push((label, reply));
    }
    pair.assert_bidirectional().await;

    // Also inspect a populated closure result, not only the initially empty list.
    let command = request("POST", &terminate_path, &[&cookie, &csrf]);
    let (terminated, ()) = tokio::join!(
        plain_exchange(running.addr, &command),
        acknowledge_pair_close(&mut pair)
    );
    assert_eq!(terminated.status, 200);
    responses.push(("terminated", terminated));
    let closed = plain_exchange(running.addr, &get("/api/v1/closed-sessions", &[&cookie])).await;
    assert_eq!(closed.status, 200);
    assert_eq!(
        json_body(&closed)["items"][0]["session_id"],
        pair.session_id
    );
    responses.push(("closed_query", closed));

    for (label, reply) in responses {
        assert_eq!(reply.header("cache-control"), Some("no-store"), "{label}");
        let mut body = json_body(&reply);
        // Only these two session views may contain csrf_token, exactly at this
        // field. The cookie token is legal only in the login Set-Cookie header.
        if matches!(label, "login" | "session") {
            let object = body
                .as_object_mut()
                .unwrap_or_else(|| panic!("session object"));
            assert_eq!(object.remove("csrf_token"), Some(json!(csrf_token)));
        }
        let body_bytes = serde_json::to_vec(&body)
            .unwrap_or_else(|error| panic!("response remains JSON: {error}"));
        let forbidden = [
            PAYLOAD,
            PASSWORD.as_bytes(),
            OTHER_PASSWORD.as_bytes(),
            hash.as_bytes(),
            b"$argon2id$",
            session_token.as_bytes(),
            csrf_token.as_bytes(),
        ];
        for secret in forbidden {
            assert!(
                !body_bytes
                    .windows(secret.len())
                    .any(|window| window == secret),
                "{label}: response body discloses a secret"
            );
        }
        for (name, value) in &reply.headers {
            for secret in forbidden {
                if label == "login" && name == "set-cookie" && secret == session_token.as_bytes() {
                    assert_eq!(value.split(';').next(), Some(&cookie["Cookie: ".len()..]));
                    continue;
                }
                assert!(
                    !name
                        .as_bytes()
                        .windows(secret.len())
                        .any(|window| window == secret)
                        && !value
                            .as_bytes()
                            .windows(secret.len())
                            .any(|window| window == secret),
                    "{label}: response header discloses a secret"
                );
            }
        }
    }
}

struct ObservedRelaySource {
    runtime: Arc<RelayRuntime<TcpTargetConnector>>,
    after_snapshot: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    last: Mutex<Option<RelaySnapshot>>,
}

impl RelayStateSource for ObservedRelaySource {
    fn snapshot(&self, clock: &dyn Clock) -> RelaySnapshot {
        let snapshot = self.runtime.snapshot(clock);
        *self.last.lock().unwrap_or_else(PoisonError::into_inner) = Some(snapshot.clone());
        let hook = self
            .after_snapshot
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(hook) = hook {
            hook();
        }
        snapshot
    }
}

struct SteppingClock(Mutex<(Duration, SystemTime)>);

impl SteppingClock {
    fn new() -> Self {
        Self(Mutex::new((
            Duration::ZERO,
            UNIX_EPOCH + Duration::from_secs(1_700_000_000),
        )))
    }

    fn set(&self, mono_millis: u64, wall_offset_secs: i64) {
        let base = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let wall = if wall_offset_secs >= 0 {
            base + Duration::from_secs(wall_offset_secs.unsigned_abs())
        } else {
            base - Duration::from_secs(wall_offset_secs.unsigned_abs())
        };
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) =
            (Duration::from_millis(mono_millis), wall);
    }
}

impl Clock for SteppingClock {
    fn monotonic(&self) -> Duration {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).0
    }

    fn wall(&self) -> SystemTime {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).1
    }
}

struct PartialWrites {
    enabled: AtomicBool,
    chunk: usize,
    fail_after: std::sync::atomic::AtomicUsize,
    bytes: std::sync::atomic::AtomicUsize,
    calls: std::sync::atomic::AtomicUsize,
    failures: std::sync::atomic::AtomicUsize,
}

impl PartialWrites {
    fn new(chunk: usize) -> Arc<Self> {
        Arc::new(Self {
            enabled: AtomicBool::new(false),
            chunk,
            fail_after: std::sync::atomic::AtomicUsize::new(usize::MAX),
            bytes: std::sync::atomic::AtomicUsize::new(0),
            calls: std::sync::atomic::AtomicUsize::new(0),
            failures: std::sync::atomic::AtomicUsize::new(0),
        })
    }
}

/// Only payload write completions are controlled; the socket, handshake,
/// session owner, forwarding engine and management HTTP stack remain real.
struct PartialTcpStream {
    inner: TcpRelayStream,
    writes: Arc<PartialWrites>,
}

impl AsyncRead for PartialTcpStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buffer)
    }
}

impl AsyncWrite for PartialTcpStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        if !self.writes.enabled.load(Ordering::SeqCst) {
            return Pin::new(&mut self.inner).poll_write(cx, bytes);
        }
        let written = self.writes.bytes.load(Ordering::SeqCst);
        let remaining = self
            .writes
            .fail_after
            .load(Ordering::SeqCst)
            .saturating_sub(written);
        if remaining == 0 {
            self.writes.failures.fetch_add(1, Ordering::SeqCst);
            return Poll::Ready(Err(io::Error::from(io::ErrorKind::BrokenPipe)));
        }
        let count = bytes.len().min(self.writes.chunk).min(remaining);
        match Pin::new(&mut self.inner).poll_write(cx, &bytes[..count]) {
            Poll::Ready(Ok(count)) => {
                self.writes.bytes.fetch_add(count, Ordering::SeqCst);
                self.writes.calls.fetch_add(1, Ordering::SeqCst);
                Poll::Ready(Ok(count))
            }
            other => other,
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

impl RelayStream for PartialTcpStream {
    fn transport(&self) -> Transport {
        Transport::Tcp
    }

    fn pending_write_len(&self) -> usize {
        0
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        action: RelayCloseAction,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_close(cx, action)
    }
}

impl RunningRelay {
    async fn partial_connection(
        &self,
        entry_point: EntryPoint,
        writes: Arc<PartialWrites>,
    ) -> (TcpStream, JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|error| panic!("partial stream bind: {error}"));
        let address = listener
            .local_addr()
            .unwrap_or_else(|error| panic!("partial address: {error}"));
        let (client, accepted) = tokio::join!(TcpStream::connect(address), listener.accept());
        let mut client = client.unwrap_or_else(|error| panic!("partial client: {error}"));
        let (socket, peer_addr) =
            accepted.unwrap_or_else(|error| panic!("partial accept: {error}"));
        let context = ConnectionContext {
            conn_id: ConnId::next(),
            entry_point,
            peer_ip: PeerIp::from(peer_addr),
            forwarded_ip: None,
            peer_addr,
            accepted_wall: self.clock.wall(),
            accepted_at: self.clock.monotonic(),
            exchange_at: self.clock.monotonic(),
            buffer_bytes: 16_384,
            handshake: LIMIT,
        };
        let permit = self
            .budget
            .try_acquire(10)
            .unwrap_or_else(|| panic!("partial stream L permit"));
        let prepare = self.service.handle_connection(
            PartialTcpStream {
                inner: TcpRelayStream::new(socket),
                writes: writes.clone(),
            },
            context,
            None,
            permit,
            self.runtime.lifecycle(),
        );
        let handshake = async {
            if entry_point == EntryPoint::TcpViewer {
                let mut greeting = [0; VIEWER_GREETING.len()];
                client
                    .read_exact(&mut greeting)
                    .await
                    .unwrap_or_else(|error| panic!("partial greeting: {error}"));
                assert_eq!(&greeting, VIEWER_GREETING);
            }
            let mut initial = [0; HANDSHAKE_REQUEST_LEN];
            initial[..6].copy_from_slice(b"ID:505");
            client
                .write_all(&initial)
                .await
                .unwrap_or_else(|error| panic!("partial initial request: {error}"));
        };
        let (prepared, ()) = timeout(LIMIT, async { tokio::join!(prepare, handshake) })
            .await
            .unwrap_or_else(|error| panic!("partial stream preparation deadline: {error}"));
        let prepared = prepared.unwrap_or_else(|error| panic!("real stream preparation: {error}"));
        writes.enabled.store(true, Ordering::SeqCst);
        let service = self.service.clone();
        let runtime = self.runtime.clone();
        let task = tokio::spawn(async move {
            service
                .dispatch(prepared, &runtime)
                .await
                .unwrap_or_else(|error| panic!("real partial-stream dispatch: {error}"));
        });
        (client, task)
    }
}

async fn authenticated_query(running: &Running, cookie: &str, path: &str) -> Value {
    let reply = plain_exchange(running.addr, &get(path, &[cookie])).await;
    assert_eq!(reply.status, 200, "{path}");
    assert_eq!(reply.header("cache-control"), Some("no-store"));
    assert_eq!(
        reply
            .body
            .windows(b"\"as_of\"".len())
            .filter(|window| *window == b"\"as_of\"")
            .count(),
        1,
        "{path}: one response-level reference time"
    );
    json_body(&reply)
}

/// spec: management.md:MGT-005
#[tokio::test]
async fn completion_mgt005_partial_success_and_write_failure_count_only_delivered_bytes() {
    let relay = RunningRelay::start(session_policy()).await;
    let running = &relay.management;
    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    assert_eq!(login.status, 200);
    let cookie = cookie_line(&login);
    let viewer_writes = PartialWrites::new(2);
    let server_writes = PartialWrites::new(3);
    let (mut viewer, viewer_task) = relay
        .partial_connection(EntryPoint::TcpViewer, viewer_writes.clone())
        .await;
    wait_for(&running.events, |event| {
        matches!(event, Event::WaitingRegistered { .. })
    })
    .await;
    let (mut server, server_task) = relay
        .partial_connection(EntryPoint::TcpServer, server_writes.clone())
        .await;
    let started = wait_for(&running.events, |event| {
        matches!(event, Event::SessionStarted { .. })
    })
    .await;
    let Event::SessionStarted { session_id, .. } = started else {
        panic!("real session started");
    };
    let forward = b"seventeen-bytes!!!";
    let reverse = b"a-different-reverse-payload-29";
    timeout(LIMIT, async {
        viewer
            .write_all(forward)
            .await
            .unwrap_or_else(|error| panic!("forward send: {error}"));
        let mut received = vec![0; forward.len()];
        server
            .read_exact(&mut received)
            .await
            .unwrap_or_else(|error| panic!("forward delivery: {error}"));
        assert_eq!(received, forward);
        server
            .write_all(reverse)
            .await
            .unwrap_or_else(|error| panic!("reverse send: {error}"));
        let mut received = vec![0; reverse.len()];
        viewer
            .read_exact(&mut received)
            .await
            .unwrap_or_else(|error| panic!("reverse delivery: {error}"));
        assert_eq!(received, reverse);
    })
    .await
    .unwrap_or_else(|error| panic!("partial payload deadline: {error}"));
    let detail =
        authenticated_query(running, &cookie, &format!("/api/v1/sessions/{session_id}")).await;
    assert_eq!(
        detail["session"]["bytes_viewer_to_target"].as_u64(),
        Some(forward.len() as u64)
    );
    assert_eq!(
        detail["session"]["bytes_target_to_viewer"].as_u64(),
        Some(reverse.len() as u64)
    );
    assert!(server_writes.calls.load(Ordering::SeqCst) > 1);
    assert!(viewer_writes.calls.load(Ordering::SeqCst) > 1);
    assert_eq!(server_writes.bytes.load(Ordering::SeqCst), forward.len());
    assert_eq!(viewer_writes.bytes.load(Ordering::SeqCst), reverse.len());

    // The next write delivers only five of thirteen bytes before BrokenPipe.
    server_writes
        .fail_after
        .store(forward.len() + 5, Ordering::SeqCst);
    viewer
        .write_all(b"thirteen-more")
        .await
        .unwrap_or_else(|error| panic!("failing payload send: {error}"));
    let mut final_bytes = Vec::new();
    timeout(LIMIT, server.read_to_end(&mut final_bytes))
        .await
        .unwrap_or_else(|error| panic!("failed transport closes: {error}"))
        .unwrap_or_else(|error| panic!("read partial accepted prefix: {error}"));
    assert_eq!(final_bytes, b"thirt");
    server
        .shutdown()
        .await
        .unwrap_or_else(|error| panic!("server EOF: {error}"));
    viewer
        .shutdown()
        .await
        .unwrap_or_else(|error| panic!("viewer EOF: {error}"));
    wait_for(
        &running.events,
        |event| matches!(event, Event::SessionClosed { session_id: id, .. } if id == &session_id),
    )
    .await;
    for task in [viewer_task, server_task] {
        timeout(LIMIT, task)
            .await
            .unwrap_or_else(|error| panic!("dispatch deadline: {error}"))
            .unwrap_or_else(|error| panic!("dispatch joins: {error}"));
    }
    assert!(server_writes.failures.load(Ordering::SeqCst) > 0);
    let closed = authenticated_query(running, &cookie, "/api/v1/closed-sessions").await;
    assert_eq!(closed["items"].as_array().map(Vec::len), Some(1));
    let item = &closed["items"][0];
    assert_eq!(item["session_id"], session_id.to_string());
    assert_eq!(item["close_cause"], "connection_error");
    assert_eq!(
        item["bytes_viewer_to_target"].as_u64(),
        Some(server_writes.bytes.load(Ordering::SeqCst) as u64)
    );
    assert_eq!(
        item["bytes_viewer_to_target"].as_u64(),
        Some((forward.len() + 5) as u64)
    );
    assert_eq!(
        item["bytes_target_to_viewer"].as_u64(),
        Some(viewer_writes.bytes.load(Ordering::SeqCst) as u64)
    );
    assert_eq!(
        item["bytes_target_to_viewer"].as_u64(),
        Some(reverse.len() as u64)
    );
}

async fn terminate_pair(running: &Running, login: &Reply, pair: &mut RelayPair) -> Value {
    let cookie = cookie_line(login);
    let csrf = csrf_line(login);
    let command = request(
        "POST",
        &format!("/api/v1/sessions/{}/terminate", pair.session_id),
        &[&cookie, &csrf],
    );
    let (reply, ()) = tokio::join!(
        plain_exchange(running.addr, &command),
        acknowledge_pair_close(pair)
    );
    assert_eq!(reply.status, 200);
    let body = json_body(&reply);
    assert_eq!(body["result"], "terminated");
    assert_eq!(body["closed_session"]["session_id"], pair.session_id);
    body
}

/// A fresh runtime/listener represents a process lifecycle boundary here; only
/// the public Clock port is controlled. Actual process startup is not injected.
/// spec: management.md:MGT-007
#[tokio::test]
async fn completion_mgt007_reconnect_fresh_runtime_and_wall_steps_keep_counters_separate() {
    let clock = Arc::new(SteppingClock::new());
    let relay = RunningRelay::start_with_clock(
        session_policy(),
        false,
        Arc::new(Argon2Verifier),
        clock.clone(),
    )
    .await;
    let running = &relay.management;
    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    assert_eq!(login.status, 200);
    let cookie = cookie_line(&login);
    let mut first = relay.pair_with_payload(false).await;
    let first_id = first.session_id.clone();
    let detail =
        authenticated_query(running, &cookie, &format!("/api/v1/sessions/{first_id}")).await;
    for direction in ["bytes_viewer_to_target", "bytes_target_to_viewer"] {
        assert_eq!(detail["session"][direction].as_u64(), Some(0));
    }
    first.assert_bidirectional().await;
    let first_closed = terminate_pair(running, &login, &mut first).await;
    assert_eq!(
        first_closed["closed_session"]["bytes_viewer_to_target"].as_u64(),
        Some(b"viewer-to-server".len() as u64)
    );
    assert_eq!(
        first_closed["closed_session"]["bytes_target_to_viewer"].as_u64(),
        Some(b"server-to-viewer".len() as u64)
    );

    // Reusing exactly the same Mode II ID never reuses a session counter.
    let mut second = relay.pair_with_payload(false).await;
    let second_id = second.session_id.clone();
    assert_ne!(first_id, second_id);
    let detail =
        authenticated_query(running, &cookie, &format!("/api/v1/sessions/{second_id}")).await;
    for direction in ["bytes_viewer_to_target", "bytes_target_to_viewer"] {
        assert_eq!(detail["session"][direction].as_u64(), Some(0));
    }
    let history = authenticated_query(running, &cookie, "/api/v1/closed-sessions").await;
    assert_eq!(history["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(history["items"][0], first_closed["closed_session"]);

    // Keep a real unmatched socket so API elapsed time is observable as
    // waited_ms. Active duration_ms is an ops event, not an API session field.
    let mut waiting = TcpStream::connect(relay.viewer)
        .await
        .unwrap_or_else(|error| panic!("clock-step waiter connects: {error}"));
    let mut greeting = [0; VIEWER_GREETING.len()];
    waiting
        .read_exact(&mut greeting)
        .await
        .unwrap_or_else(|error| panic!("clock waiter greeting: {error}"));
    let mut initial = [0; HANDSHAKE_REQUEST_LEN];
    initial[..6].copy_from_slice(b"ID:789");
    waiting
        .write_all(&initial)
        .await
        .unwrap_or_else(|error| panic!("clock waiter handshake: {error}"));
    timeout(LIMIT, async {
        loop {
            if relay.runtime.snapshot(clock.as_ref()).waiting.len() == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|error| panic!("clock waiter is registered: {error}"));
    let mut last_uptime = 0;
    let mut last_waited = 0;
    let mut previous_as_of = String::new();
    for (mono_millis, wall_seconds) in [(1000, -3600), (2000, 7200), (3000, -7200)] {
        clock.set(mono_millis, wall_seconds);
        let summary = authenticated_query(running, &cookie, "/api/v1/summary").await;
        let uptime = summary["server"]["uptime_ms"]
            .as_u64()
            .unwrap_or_else(|| panic!("integer uptime"));
        assert_eq!(uptime, mono_millis);
        assert!(uptime >= last_uptime);
        last_uptime = uptime;
        let waiting = authenticated_query(running, &cookie, "/api/v1/waiting").await;
        assert_eq!(waiting["items"].as_array().map(Vec::len), Some(1));
        let waited = waiting["items"][0]["waited_ms"]
            .as_u64()
            .unwrap_or_else(|| panic!("integer elapsed time"));
        assert_eq!(waited, mono_millis);
        assert!(waited >= last_waited);
        last_waited = waited;
        let active = relay.runtime.snapshot(clock.as_ref());
        let session = active
            .active
            .iter()
            .find(|session| session.session_id.as_str() == second_id)
            .unwrap_or_else(|| panic!("clock-stepped session remains active"));
        assert_eq!(session.duration.as_millis(), u128::from(mono_millis));
        let as_of = summary["as_of"]
            .as_str()
            .unwrap_or_else(|| panic!("UTC as_of"))
            .to_owned();
        assert_eq!(waiting["as_of"], as_of);
        if mono_millis == 2000 {
            assert!(as_of > previous_as_of, "forward wall step is visible");
        } else if mono_millis == 3000 {
            assert!(
                as_of < previous_as_of,
                "backward wall step does not alter elapsed time"
            );
        }
        previous_as_of = as_of;
        let detail =
            authenticated_query(running, &cookie, &format!("/api/v1/sessions/{second_id}")).await;
        assert_eq!(
            detail["session"]["bytes_viewer_to_target"].as_u64(),
            Some(0)
        );
        assert_eq!(
            detail["session"]["bytes_target_to_viewer"].as_u64(),
            Some(0)
        );
    }
    let closed = terminate_pair(running, &login, &mut second).await;
    assert!(
        closed["closed_session"]["ended_at"].as_str()
            >= closed["closed_session"]["started_at"].as_str()
    );
    let duration = wait_for(&running.events, |event| {
        matches!(event,
            Event::SessionClosed { session_id, .. } if session_id.as_str() == second_id
        )
    })
    .await;
    assert!(matches!(
        duration,
        Event::SessionClosed {
            duration_ms: 3000,
            ..
        }
    ));
    drop(waiting);
    timeout(LIMIT, async {
        loop {
            if relay.runtime.snapshot(clock.as_ref()).waiting.is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|error| panic!("clock waiter releases before restart: {error}"));
    drop(first);
    drop(second);
    drop(relay);

    // No runtime, auth state, history or keyed session-ID source is reused.
    let restarted = RunningRelay::start(session_policy()).await;
    let running = &restarted.management;
    let refused = plain_exchange(running.addr, &get("/api/v1/sessions", &[&cookie])).await;
    assert_eq!(refused.status, 401);
    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    assert_eq!(login.status, 200);
    let cookie = cookie_line(&login);
    for path in [
        "/api/v1/waiting",
        "/api/v1/sessions",
        "/api/v1/closed-sessions",
    ] {
        let response = authenticated_query(running, &cookie, path).await;
        assert_eq!(response["items"], json!([]), "{path}: restart clears state");
    }
    let summary = authenticated_query(running, &cookie, "/api/v1/summary").await;
    assert_eq!(summary["counts"]["active"].as_u64(), Some(0));
    assert_eq!(
        summary["usage"]["relay_connections"]["current"].as_u64(),
        Some(0)
    );
    let mut fresh = restarted.pair_with_payload(false).await;
    assert_ne!(fresh.session_id, first_id);
    assert_ne!(fresh.session_id, second_id);
    let detail = authenticated_query(
        running,
        &cookie,
        &format!("/api/v1/sessions/{}", fresh.session_id),
    )
    .await;
    for direction in ["bytes_viewer_to_target", "bytes_target_to_viewer"] {
        assert_eq!(detail["session"][direction].as_u64(), Some(0));
    }
    terminate_pair(running, &login, &mut fresh).await;
}

/// Counts and list items follow each response's captured registry and one as_of.
/// Summary usage is a separately sampled resource gauge and may lead counts
/// during admission (management.md §2.2, management-api-v1 §1.1; Q100).
/// spec: management.md:MGT-016
#[tokio::test]
async fn completion_mgt016_counts_lists_and_as_of_follow_one_capture_while_usage_is_sampled() {
    let clock = Arc::new(SteppingClock::new());
    let relay = RunningRelay::start_with_clock(
        session_policy(),
        false,
        Arc::new(Argon2Verifier),
        clock.clone(),
    )
    .await;
    let running = &relay.management;
    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    assert_eq!(login.status, 200);
    let cookie = cookie_line(&login);
    let mut pair = relay.pair().await;
    let runtime = relay.runtime.clone();
    let clock_for_hook = clock.clone();
    let events = running.events.clone();
    let id = pair.session_id.clone();
    *relay
        .source
        .after_snapshot
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = Some(Box::new(move || {
        let ticket = runtime.request_terminate(
            &id,
            AccountName::parse(ACCOUNT)
                .unwrap_or_else(|error| panic!("snapshot initiator: {error}")),
            loopback(),
            clock_for_hook.as_ref(),
            events.as_ref(),
        );
        assert!(
            ticket.is_some(),
            "real termination starts strictly after capture"
        );
    }));
    let sessions = authenticated_query(running, &cookie, "/api/v1/sessions").await;
    let captured = relay
        .source
        .last
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
        .unwrap_or_else(|| panic!("query captured a runtime snapshot"));
    assert_eq!(captured.active.len(), 1);
    assert_eq!(
        sessions["items"].as_array().map(Vec::len),
        Some(captured.active.len())
    );
    assert_eq!(sessions["items"][0]["session_id"], pair.session_id);
    assert_eq!(
        sessions["items"][0]["state"], "active",
        "the response retains pre-mutation state"
    );
    timeout(LIMIT, async {
        loop {
            let now = relay.runtime.snapshot(clock.as_ref());
            if now.active.is_empty()
                || now.active.first().is_some_and(|session| {
                    session.state == vnc_relay_domain::session::SessionState::Closing
                })
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|error| panic!("real session leaves active state: {error}"));
    let next = authenticated_query(running, &cookie, "/api/v1/sessions").await;
    let items = next["items"]
        .as_array()
        .unwrap_or_else(|| panic!("next session items"));
    assert!(
        items.is_empty() || items[0]["state"] == "closing",
        "mutation is visible on the next capture"
    );
    assert_eq!(
        sessions["as_of"], next["as_of"],
        "one controlled wall time per response"
    );
    acknowledge_pair_close(&mut pair).await;
    wait_for(&running.events, |event| {
        matches!(event, Event::SessionClosed { .. })
    })
    .await;

    // After the empty registry is copied, admit a real TCP viewer and withhold
    // its handshake. The source returns the old snapshot; summary reads new L.
    let held_socket = Arc::new(Mutex::new(None));
    let held_for_hook = held_socket.clone();
    let service = relay.service.clone();
    let budget = relay.budget.clone();
    let runtime = relay.runtime.clone();
    let clock_for_hook = clock.clone();
    *relay
        .source
        .after_snapshot
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = Some(Box::new(move || {
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap_or_else(|error| panic!("post-snapshot listener: {error}"));
        let mut socket = std::net::TcpStream::connect(
            listener
                .local_addr()
                .unwrap_or_else(|error| panic!("snapshot listener address: {error}")),
        )
        .unwrap_or_else(|error| panic!("post-snapshot real TCP admission: {error}"));
        let (accepted, peer_addr) = listener
            .accept()
            .unwrap_or_else(|error| panic!("post-snapshot accept: {error}"));
        accepted
            .set_nonblocking(true)
            .unwrap_or_else(|error| panic!("snapshot accepted socket mode: {error}"));
        socket
            .set_read_timeout(Some(LIMIT))
            .unwrap_or_else(|error| panic!("snapshot socket timeout: {error}"));
        // A separate current-thread reactor lets the synchronous snapshot port
        // advance real socket preparation without adding Tokio feature flags.
        let worker = std::thread::spawn(move || {
            let reactor = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap_or_else(|error| panic!("snapshot admission reactor: {error}"));
            reactor.block_on(async {
                let stream = TcpStream::from_std(accepted)
                    .unwrap_or_else(|error| panic!("snapshot Tokio socket: {error}"));
                let _handshaking = runtime.track_handshaking();
                let permit = budget
                    .try_acquire(10)
                    .unwrap_or_else(|| panic!("snapshot admission L permit"));
                let context = ConnectionContext {
                    conn_id: ConnId::next(),
                    entry_point: EntryPoint::TcpViewer,
                    peer_ip: PeerIp::from(peer_addr),
                    forwarded_ip: None,
                    peer_addr,
                    accepted_wall: clock_for_hook.wall(),
                    accepted_at: clock_for_hook.monotonic(),
                    exchange_at: clock_for_hook.monotonic(),
                    buffer_bytes: 16_384,
                    handshake: LIMIT,
                };
                let result = service
                    .handle_connection(
                        TcpRelayStream::new(stream),
                        context,
                        None,
                        permit,
                        runtime.lifecycle(),
                    )
                    .await;
                assert!(
                    result.is_err(),
                    "withheld handshake ends when its real peer drops"
                );
            });
        });
        let mut greeting = [0; VIEWER_GREETING.len()];
        std::io::Read::read_exact(&mut socket, &mut greeting)
            .unwrap_or_else(|error| panic!("real post-snapshot viewer greeting: {error}"));
        assert_eq!(&greeting, VIEWER_GREETING);
        *held_for_hook.lock().unwrap_or_else(PoisonError::into_inner) = Some((socket, worker));
    }));
    let summary = authenticated_query(running, &cookie, "/api/v1/summary").await;
    let captured = relay
        .source
        .last
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
        .unwrap_or_else(|| panic!("summary captured registry"));
    assert_eq!(captured.active.len(), 0);
    assert_eq!(captured.handshaking, 0);
    assert_eq!(summary["counts"]["active"].as_u64(), Some(0));
    assert_eq!(summary["counts"]["handshaking"].as_u64(), Some(0));
    assert_eq!(
        summary["usage"]["relay_connections"]["current"].as_u64(),
        Some(1),
        "Q100: separately sampled L usage may lead the response's captured counts"
    );
    assert_eq!(relay.runtime.snapshot(clock.as_ref()).handshaking, 1);
    let next = authenticated_query(running, &cookie, "/api/v1/summary").await;
    assert_eq!(next["counts"]["handshaking"].as_u64(), Some(1));
    assert_eq!(
        next["usage"]["relay_connections"]["current"].as_u64(),
        Some(1)
    );
    assert_eq!(
        next["counts"]["handshaking"], next["usage"]["relay_connections"]["current"],
        "after admission stabilizes, the next capture and resource gauge agree"
    );
    assert_eq!(
        summary["as_of"], next["as_of"],
        "one controlled reference time per response"
    );
    let (socket, worker) = held_socket
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take()
        .unwrap_or_else(|| panic!("post-snapshot admission is retained"));
    drop(socket);
    worker
        .join()
        .unwrap_or_else(|error| panic!("post-snapshot admission joins: {error:?}"));
    timeout(LIMIT, async {
        loop {
            if relay.runtime.snapshot(clock.as_ref()).handshaking == 0
                && relay.budget.current() == 0
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|error| panic!("post-snapshot socket releases: {error}"));

    // During concurrent real registration/removal, compare each response only
    // to its own capture; different requests need not describe the same state.
    let churn = async {
        for index in 0..8 {
            let mut socket = TcpStream::connect(relay.viewer)
                .await
                .unwrap_or_else(|error| panic!("churn viewer: {error}"));
            let mut greeting = [0; VIEWER_GREETING.len()];
            socket
                .read_exact(&mut greeting)
                .await
                .unwrap_or_else(|error| panic!("churn greeting: {error}"));
            let id = format!("ID:9{index:02}");
            let mut initial = [0; HANDSHAKE_REQUEST_LEN];
            initial[..id.len()].copy_from_slice(id.as_bytes());
            socket
                .write_all(&initial)
                .await
                .unwrap_or_else(|error| panic!("churn handshake: {error}"));
            timeout(LIMIT, async {
                loop {
                    if !relay.runtime.snapshot(clock.as_ref()).waiting.is_empty() {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap_or_else(|error| panic!("churn registration: {error}"));
            drop(socket);
            timeout(LIMIT, async {
                loop {
                    if relay.runtime.snapshot(clock.as_ref()).waiting.is_empty() {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap_or_else(|error| panic!("churn removal: {error}"));
        }
    };
    let queries = async {
        for _ in 0..12 {
            for path in ["/api/v1/summary", "/api/v1/waiting", "/api/v1/sessions"] {
                let response = authenticated_query(running, &cookie, path).await;
                let captured = relay
                    .source
                    .last
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone()
                    .unwrap_or_else(|| panic!("concurrent query snapshot"));
                match path {
                    "/api/v1/summary" => {
                        assert_eq!(
                            response["counts"]["waiting"].as_u64(),
                            Some(captured.waiting.len() as u64)
                        );
                        assert_eq!(
                            response["counts"]["active"].as_u64(),
                            Some(captured.active.len() as u64)
                        );
                        assert_eq!(
                            response["counts"]["handshaking"].as_u64(),
                            Some(captured.handshaking as u64)
                        );
                        assert_eq!(
                            response["counts"]["target_connecting"].as_u64(),
                            Some(captured.target_connecting as u64)
                        );
                    }
                    "/api/v1/waiting" => assert_eq!(
                        response["items"].as_array().map(Vec::len),
                        Some(captured.waiting.len())
                    ),
                    "/api/v1/sessions" => assert_eq!(
                        response["items"].as_array().map(Vec::len),
                        Some(captured.active.len())
                    ),
                    _ => unreachable!(),
                }
                assert_eq!(response["as_of"], sessions["as_of"]);
            }
        }
    };
    timeout(LIMIT, async {
        tokio::join!(churn, queries);
    })
    .await
    .unwrap_or_else(|error| panic!("concurrent snapshot exercise: {error}"));
}

/// spec: management.md:MGT-023 partial
#[tokio::test]
async fn completion_mgt023_shutdown_history_is_queryable_and_preserves_prior_eof_cause() {
    let relay = RunningRelay::start(session_policy()).await;
    let running = &relay.management;
    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    assert_eq!(login.status, 200);
    let cookie = cookie_line(&login);
    let mut forced = relay.pair_with_id("82301", true).await;
    let mut prior_eof = relay.pair_with_id("82302", true).await;
    let service = relay.service.clone();
    let runtime = relay.runtime.clone();
    let supervisor = tokio::spawn(async move {
        service.drive_shutdown(&runtime).await;
    });

    // Production's first request enters Grace while management HTTP remains
    // available; existing sessions can still carry payload during this phase.
    relay
        .service
        .request_shutdown(&relay.runtime, Duration::from_secs(60));
    let summary = authenticated_query(running, &cookie, "/api/v1/summary").await;
    assert_eq!(summary["server"]["state"], "shutdown_grace");
    assert_eq!(summary["counts"]["active"].as_u64(), Some(2));
    assert_eq!(
        summary["usage"]["relay_connections"]["current"].as_u64(),
        Some(4)
    );
    forced.assert_bidirectional().await;

    // Observe relay-propagated EOF before Force, while the opposite write half
    // stays open. This proves cleanup already has its first cause (Q82).
    prior_eof
        .viewer
        .shutdown()
        .await
        .unwrap_or_else(|error| panic!("prior EOF write half closes: {error}"));
    let mut unexpected = [0; 1];
    let eof = timeout(LIMIT, prior_eof.server.read(&mut unexpected))
        .await
        .unwrap_or_else(|error| panic!("relay propagates prior EOF: {error}"))
        .unwrap_or_else(|error| panic!("read propagated prior EOF: {error}"));
    assert_eq!(eof, 0);
    let detail = authenticated_query(
        running,
        &cookie,
        &format!("/api/v1/sessions/{}", prior_eof.session_id),
    )
    .await;
    assert_eq!(detail["session"]["state"], "closing");

    // Like serve.rs's second shutdown request, Force cleans up both owners
    // without shutting down the independently owned management listener.
    relay
        .service
        .request_shutdown(&relay.runtime, Duration::ZERO);
    timeout(LIMIT, supervisor)
        .await
        .unwrap_or_else(|error| panic!("shutdown supervisor completes: {error}"))
        .unwrap_or_else(|error| panic!("shutdown supervisor joins: {error}"));
    for id in [&forced.session_id, &prior_eof.session_id] {
        wait_for(&running.events, |event| {
            matches!(event, Event::SessionClosed { session_id, .. } if session_id.as_str() == id)
        }).await;
    }
    let history = authenticated_query(running, &cookie, "/api/v1/closed-sessions").await;
    let items = history["items"]
        .as_array()
        .unwrap_or_else(|| panic!("shutdown history items"));
    assert_eq!(items.len(), 2);
    for (id, cause, transfers) in [
        (&forced.session_id, "server_shutdown", 2_u64),
        (&prior_eof.session_id, "peer_eof", 1_u64),
    ] {
        let item = items
            .iter()
            .find(|item| item["session_id"].as_str() == Some(id.as_str()))
            .unwrap_or_else(|| panic!("shutdown history contains session {id}"));
        assert_eq!(item["close_cause"], cause);
        assert_eq!(item["mode"], "mode2");
        assert_eq!(item["viewer_transport"], "tcp");
        assert_eq!(
            item["bytes_viewer_to_target"].as_u64(),
            Some(transfers * b"viewer-to-server".len() as u64),
            "{cause}: integer final forward counter"
        );
        assert_eq!(
            item["bytes_target_to_viewer"].as_u64(),
            Some(transfers * b"server-to-viewer".len() as u64),
            "{cause}: integer final reverse counter"
        );
        assert!(item["ended_at"].as_str() >= item["started_at"].as_str());
        let detail = plain_exchange(
            running.addr,
            &get(&format!("/api/v1/sessions/{id}"), &[&cookie]),
        )
        .await;
        assert_eq!(detail.status, 404);
        assert_eq!(error_code(&detail), "not_found");
    }
    let summary = authenticated_query(running, &cookie, "/api/v1/summary").await;
    assert_eq!(summary["server"]["state"], "shutdown_grace");
    assert_eq!(summary["counts"]["active"].as_u64(), Some(0));
    assert_eq!(
        summary["usage"]["relay_connections"]["current"].as_u64(),
        Some(0)
    );
    let sessions = authenticated_query(running, &cookie, "/api/v1/sessions").await;
    assert_eq!(sessions["items"], json!([]));
    let waiting = authenticated_query(running, &cookie, "/api/v1/waiting").await;
    assert_eq!(waiting["items"], json!([]));
}

// Config HTTP tests keep the app transaction and document adapter real. Only
// filesystem failures and TLS content checks are controlled at their ports.
const CONFIG_TEXT: &str = r#"
[admin]
accounts_file = "accounts.toml"
[management]
listen = "127.0.0.1:8443"
config_edit = true
[status]
listen = "127.0.0.1:8081"
[relay.tcp_viewer]
enabled = true
listen = "127.0.0.1:5901"
allow = ["127.0.0.1"]
[relay.tcp_server]
enabled = true
listen = "127.0.0.1:5500"
allow = ["127.0.0.1"]
[relay.ws]
listen = "127.0.0.1:6080"
[relay.ws.viewer]
enabled = true
allow = ["127.0.0.1"]
allowed_origins = ["https://example.test"]
[relay.wss]
listen = "127.0.0.1:6081"
cert_file = "cert.pem"
key_file = "key.pem"
[relay.wss.viewer]
enabled = true
allow = ["127.0.0.1"]
allowed_origins = ["https://example.test"]
[mode1]
enabled = true
allow_targets = ["127.0.0.1:5900"]
[mode2]
enabled = true
"#;

struct ConfigMemory {
    bytes: Mutex<Vec<u8>>,
    unreadable: AtomicBool,
    fail_replace: AtomicBool,
    fail_write: AtomicBool,
    temporary_writes: std::sync::atomic::AtomicUsize,
    replaces: std::sync::atomic::AtomicUsize,
    replacement_gate: Mutex<Option<Arc<ConfigReplacementGate>>>,
}

#[derive(Default)]
struct ConfigReplacementGate {
    entered: AtomicBool,
    released: Mutex<bool>,
    ready: std::sync::Condvar,
}

struct ReleaseConfigReplacement(Arc<ConfigReplacementGate>);
impl Drop for ReleaseConfigReplacement {
    fn drop(&mut self) {
        *self
            .0
            .released
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = true;
        self.0.ready.notify_all();
    }
}

impl ConfigMemory {
    fn bytes(&self) -> Vec<u8> {
        self.bytes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
    fn set(&self, bytes: Vec<u8>) {
        *self.bytes.lock().unwrap_or_else(PoisonError::into_inner) = bytes;
    }
}

impl ConfigFileStore for ConfigMemory {
    fn read(&self, _path: &Path) -> io::Result<Vec<u8>> {
        if self.unreadable.load(Ordering::SeqCst) {
            return Err(io::Error::from(io::ErrorKind::PermissionDenied));
        }
        Ok(self.bytes())
    }
    fn revision(&self, bytes: &[u8]) -> String {
        FsConfigFileStore.revision(bytes)
    }
    fn replace(&self, _path: &Path, expected: &str, next: &[u8]) -> Result<(), ConfigReplaceError> {
        self.replaces.fetch_add(1, Ordering::SeqCst);
        if self.fail_write.load(Ordering::SeqCst) {
            return Err(ConfigReplaceError::WriteFailed);
        }
        self.temporary_writes.fetch_add(1, Ordering::SeqCst);
        let gate = self
            .replacement_gate
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        if let Some(gate) = gate {
            gate.entered.store(true, Ordering::SeqCst);
            let mut released = gate.released.lock().unwrap_or_else(PoisonError::into_inner);
            while !*released {
                released = gate
                    .ready
                    .wait(released)
                    .unwrap_or_else(PoisonError::into_inner);
            }
        }
        if self.fail_replace.load(Ordering::SeqCst) {
            return Err(ConfigReplaceError::WriteFailed);
        }
        let mut bytes = self.bytes.lock().unwrap_or_else(PoisonError::into_inner);
        if self.revision(&bytes) != expected {
            return Err(ConfigReplaceError::Conflict);
        }
        *bytes = next.to_vec();
        Ok(())
    }
}

#[derive(Default)]
struct ConfigTls {
    invalid: AtomicBool,
}
impl ReloadFileStore for ConfigTls {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        if path
            .extension()
            .is_some_and(|extension| extension == "jwks")
        {
            return std::fs::read(path);
        }
        Ok(b"TLS-FILE-SECRET".to_vec())
    }
}
impl ReloadTlsStore<()> for ConfigTls {
    fn build(&self, _cert: &[u8], _key: &[u8]) -> Result<(), TlsBuildError> {
        if self.invalid.load(Ordering::SeqCst) {
            Err(TlsBuildError {
                detail: "certificate format",
            })
        } else {
            Ok(())
        }
    }
}

struct ConfigFixture {
    service: Arc<ConfigEditService<()>>,
    files: Arc<ConfigMemory>,
    tls: Arc<ConfigTls>,
    reload: SharedReload<()>,
}

fn config_fixture(
    enabled: bool,
    clock: Arc<dyn Clock>,
    events: Arc<dyn EventSink>,
) -> ConfigFixture {
    config_fixture_text(
        &CONFIG_TEXT.replace("config_edit = true", &format!("config_edit = {enabled}")),
        clock,
        events,
    )
}

fn config_fixture_text(
    text: &str,
    clock: Arc<dyn Clock>,
    events: Arc<dyn EventSink>,
) -> ConfigFixture {
    let base = std::env::temp_dir();
    let candidate = config_file::parse_candidate(text.as_bytes(), &base)
        .unwrap_or_else(|error| panic!("config fixture: {error:?}"));
    let cell = Arc::new(ReloadCell::new(ReloadSnapshot::new(
        candidate.clone(),
        None,
        None,
    )));
    let tls = Arc::new(ConfigTls::default());
    let reload = Arc::new(Mutex::new(ReloadService::new(
        candidate,
        cell,
        tls.clone(),
        tls.clone(),
        Arc::new(vnc_relay_adapters::ticket::TicketJwksStore),
        events.clone(),
    )));
    let files = Arc::new(ConfigMemory {
        bytes: Mutex::new(text.as_bytes().to_vec()),
        unreadable: AtomicBool::new(false),
        fail_write: AtomicBool::new(false),
        temporary_writes: std::sync::atomic::AtomicUsize::new(0),
        fail_replace: AtomicBool::new(false),
        replaces: std::sync::atomic::AtomicUsize::new(0),
        replacement_gate: Mutex::new(None),
    });
    let service = Arc::new(ConfigEditService::new(
        reload.clone(),
        files.clone(),
        Arc::new(TomlConfigDocument),
        base.join("relay.toml"),
        base,
        clock,
        events,
    ));
    ConfigFixture {
        service,
        files,
        tls,
        reload,
    }
}

async fn start_config_services(
    config: Arc<dyn ConfigManagement>,
    events: Arc<Recorder>,
    clock: Arc<dyn Clock>,
) -> Running {
    let log: Arc<dyn EventSink> = events.clone();
    let budget = ConnectionBudget::new();
    let runtime = management_runtime(ROOMY_LM, UPGRADE, Duration::from_secs(10));
    let auth = admin_auth(
        log.clone(),
        session_policy(),
        limit_policy(),
        Arc::new(Argon2Verifier),
        clock.clone(),
    );
    let query = Arc::new(ManagementQueryService::new(
        Arc::new(EmptyRelaySource),
        ConnectionBudget::new(),
        budget.clone(),
        clock.clone(),
        runtime.clone(),
    ));
    let terminate = Arc::new(ManagementTerminateService::new(
        Arc::new(EmptyRelaySource),
        clock,
        log,
    ));
    start_services(
        None,
        budget,
        runtime,
        ManagementServices {
            auth,
            query,
            terminate,
            config,
            logs: Arc::new(RecentLogs::new(1000)),
        },
        events,
    )
    .await
}

async fn config_login(running: &Running) -> (String, String) {
    let reply = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    assert_eq!(reply.status, 200);
    let body: Value =
        serde_json::from_slice(&reply.body).unwrap_or_else(|error| panic!("login JSON: {error}"));
    (
        cookie_line(&reply),
        format!(
            "X-CSRF-Token: {}",
            body["csrf_token"].as_str().unwrap_or_default()
        ),
    )
}

fn patch_bytes(body: &str, headers: &[&str], content_type: Option<&str>) -> Vec<u8> {
    let length = format!("Content-Length: {}", body.len());
    let mut all = vec![length.as_str()];
    if let Some(content_type) = content_type {
        all.push(content_type);
    }
    all.extend_from_slice(headers);
    let mut bytes = request("PATCH", "/api/v1/config", &all);
    bytes.extend_from_slice(body.as_bytes());
    bytes
}

async fn config_patch(
    running: &Running,
    cookie: &str,
    csrf: &str,
    revision: &str,
    changes: Value,
) -> Reply {
    let body = json!({"revision": revision, "changes": changes}).to_string();
    plain_exchange(
        running.addr,
        &patch_bytes(
            &body,
            &[cookie, csrf],
            Some("Content-Type: application/json"),
        ),
    )
    .await
}

fn config_revision(fixture: &ConfigFixture) -> String {
    fixture.files.revision(&fixture.files.bytes())
}

fn config_json(reply: &Reply) -> Value {
    serde_json::from_slice(&reply.body)
        .unwrap_or_else(|error| panic!("config JSON: {error}: {:?}", reply.body))
}

fn config_audits(events: &Recorder) -> Vec<Value> {
    events
        .snapshot()
        .iter()
        .filter(|event| {
            matches!(
                event,
                Event::ConfigEdited { .. }
                    | Event::ConfigEditRejected { .. }
                    | Event::ReloadApplied { .. }
            )
        })
        .map(|event| {
            serde_json::from_str(&format_line(event, UNIX_EPOCH))
                .unwrap_or_else(|error| panic!("audit JSON: {error}"))
        })
        .collect()
}

fn assert_config_unchanged(
    fixture: &ConfigFixture,
    bytes: &[u8],
    running: &vnc_relay_app::settings::Settings,
) {
    assert_eq!(fixture.files.bytes(), bytes);
    let reload = fixture
        .reload
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    assert_eq!(reload.running(), running);
    assert!(reload.last_reload().is_none());
}

fn assert_rejection(events: &Recorder, reason: &str, key: &str) {
    let audits = config_audits(events);
    assert_eq!(audits.len(), 1, "{audits:?}");
    assert_eq!(audits[0]["event"], "config_edit_rejected");
    assert_eq!(audits[0]["admin_account"], ACCOUNT);
    assert_eq!(audits[0]["peer_ip"], "127.0.0.1");
    assert_eq!(audits[0]["reason"], reason);
    assert_eq!(audits[0]["config_keys"], json!([key]));
    assert!(audits[0].get("revision").is_none());
    for forbidden in [
        "changes",
        "value",
        "password",
        "password_hash",
        "origin",
        "target",
    ] {
        assert!(audits[0].get(forbidden).is_none(), "{audits:?}");
    }
}

/// spec: management.md:MGT-029
#[tokio::test]
async fn config_routes_refuse_unauthenticated_requests_before_details() {
    let events = Arc::new(Recorder::default());
    let clock = Arc::new(FakeClock::default());
    let fixture = config_fixture(true, clock.clone(), events.clone());
    fixture
        .files
        .set(b"not TOML; PRIVATE-CONFIG-VALUE".to_vec());
    let original = fixture.files.bytes();
    let settings = fixture
        .reload
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .running()
        .clone();
    let running = start_config_services(fixture.service.clone(), events.clone(), clock).await;
    for method in ["GET", "PATCH", "HEAD", "PUT"] {
        let reply = plain_exchange(running.addr, &request(method, "/api/v1/config", &[])).await;
        assert_eq!(reply.status, 401);
        if method != "HEAD" {
            assert_eq!(error_code(&reply), "unauthenticated");
            assert!(config_json(&reply)["error"].get("diagnostics").is_none());
        }
        assert!(!String::from_utf8_lossy(&reply.body).contains("PRIVATE-CONFIG-VALUE"));
    }
    assert_config_unchanged(&fixture, &original, &settings);
    assert_eq!(fixture.files.replaces.load(Ordering::SeqCst), 0);
    assert!(config_audits(&events).is_empty());
}

/// spec: management.md:MGT-030
#[tokio::test]
async fn disabled_config_edit_keeps_file_and_runtime_and_audits_rejection() {
    let events = Arc::new(Recorder::default());
    let clock = Arc::new(FakeClock::default());
    let fixture = config_fixture(false, clock.clone(), events.clone());
    let original = fixture.files.bytes();
    let settings = fixture
        .reload
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .running()
        .clone();
    let revision = config_revision(&fixture);
    let running = start_config_services(fixture.service.clone(), events.clone(), clock).await;
    let (cookie, csrf) = config_login(&running).await;
    let view = plain_exchange(running.addr, &get("/api/v1/config", &[&cookie])).await;
    assert_eq!(view.status, 200);
    assert_eq!(config_json(&view)["edit_enabled"], false);
    // Disabled is checked before the unreadable file and stale revision.
    fixture.files.unreadable.store(true, Ordering::SeqCst);
    let reply = config_patch(
        &running,
        &cookie,
        &csrf,
        &revision,
        json!({"timeouts.connect_secs": 17}),
    )
    .await;
    assert_eq!(reply.status, 403);
    assert_eq!(error_code(&reply), "config_edit_disabled");
    assert_config_unchanged(&fixture, &original, &settings);
    assert_eq!(fixture.files.replaces.load(Ordering::SeqCst), 0);
    assert_rejection(&events, "config_edit_disabled", "timeouts.connect_secs");
}

/// spec: management.md:MGT-032
#[tokio::test]
async fn config_patch_csrf_failure_changes_nothing_and_emits_no_edit_event() {
    let events = Arc::new(Recorder::default());
    let clock = Arc::new(FakeClock::default());
    let fixture = config_fixture(true, clock.clone(), events.clone());
    let original = fixture.files.bytes();
    let settings = fixture
        .reload
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .running()
        .clone();
    let running = start_config_services(fixture.service.clone(), events.clone(), clock).await;
    let (cookie, csrf) = config_login(&running).await;
    for headers in [
        vec![cookie.as_str()],
        vec![cookie.as_str(), "X-CSRF-Token: wrong"],
        vec![cookie.as_str(), csrf.as_str(), csrf.as_str()],
    ] {
        // A malformed body must not win over CSRF.
        let reply = plain_exchange(
            running.addr,
            &patch_bytes("{", &headers, Some("Content-Type: text/plain")),
        )
        .await;
        assert_eq!(reply.status, 403);
        assert_eq!(error_code(&reply), "csrf_invalid");
    }
    let session = plain_exchange(
        running.addr,
        &get("/api/v1/session", &[&cookie, "X-Relay-Poll: 1"]),
    )
    .await;
    assert_eq!(session.status, 200, "CSRF rejection preserves the session");
    assert_config_unchanged(&fixture, &original, &settings);
    assert_eq!(fixture.files.replaces.load(Ordering::SeqCst), 0);
    assert!(config_audits(&events).is_empty());
}

/// spec: management.md:MGT-035
#[tokio::test]
async fn invalid_or_unreadable_current_file_refuses_patch_with_safe_diagnostics() {
    for state in ["read", "utf8", "syntax", "tls"] {
        let events = Arc::new(Recorder::default());
        let clock = Arc::new(FakeClock::default());
        let text = if state == "tls" {
            CONFIG_TEXT.replace(
                "config_edit = true",
                "config_edit = true\ncert_file = \"cert.pem\"\nkey_file = \"key.pem\"",
            )
        } else {
            CONFIG_TEXT.to_owned()
        };
        let fixture = config_fixture_text(&text, clock.clone(), events.clone());
        match state {
            "read" => fixture.files.unreadable.store(true, Ordering::SeqCst),
            "utf8" => fixture.files.set(vec![0xff, 0xfe]),
            "syntax" => fixture
                .files
                .set(b"[management\nPRIVATE-CONFIG-VALUE".to_vec()),
            "tls" => fixture.tls.invalid.store(true, Ordering::SeqCst),
            _ => unreachable!(),
        }
        let original = fixture.files.bytes();
        let settings = fixture
            .reload
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .running()
            .clone();
        let running = start_config_services(fixture.service.clone(), events.clone(), clock).await;
        let (cookie, csrf) = config_login(&running).await;
        let reply = config_patch(
            &running,
            &cookie,
            &csrf,
            &config_revision(&fixture),
            json!({"timeouts.connect_secs": 17}),
        )
        .await;
        assert_eq!(reply.status, 409, "{state}: {:?}", config_json(&reply));
        assert_eq!(error_code(&reply), "config_file_invalid");
        let body = config_json(&reply);
        let diagnostics = body["error"]["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("diagnostics array"));
        if matches!(state, "read" | "utf8") {
            assert!(diagnostics.is_empty());
        } else {
            assert!(!diagnostics.is_empty());
        }
        if let Some(diagnostic) = diagnostics.first() {
            assert_eq!(
                diagnostic["code"],
                if state == "tls" {
                    "tls_file_invalid"
                } else {
                    "config_syntax_error"
                }
            );
            assert!(diagnostic["message"].is_string());
        }
        for secret in ["PRIVATE-CONFIG-VALUE", "TLS-FILE-SECRET"] {
            assert!(!String::from_utf8_lossy(&reply.body).contains(secret));
        }
        assert_config_unchanged(&fixture, &original, &settings);
        assert_eq!(fixture.files.replaces.load(Ordering::SeqCst), 0);
        assert_rejection(&events, "config_file_invalid", "timeouts.connect_secs");
    }
}

/// spec: management.md:MGT-036
#[tokio::test]
async fn invalid_config_candidate_returns_indexed_safe_diagnostics_without_writes() {
    for (changes, key, code, secret) in [
        (
            json!({"timeouts.connect_secs": 0}),
            "timeouts.connect_secs",
            "config_value_out_of_range",
            "not-present",
        ),
        (
            json!({"timeouts.connect_secs": u64::MAX}),
            "timeouts.connect_secs",
            "config_value_out_of_range",
            "not-present",
        ),
        (
            json!({"limits.relay_connections": 2, "limits.waiting": 3}),
            "limits.waiting",
            "config_value_out_of_range",
            "not-present",
        ),
        (
            json!({"relay.tcp_viewer.allow": ["127.0.0.1", "::1", "PRIVATE-BAD-IP"]}),
            "relay.tcp_viewer.allow[2]",
            "config_ip_item_invalid",
            "PRIVATE-BAD-IP",
        ),
        (
            json!({"relay.ws.viewer.allowed_origins": ["https://example.test", "PRIVATE-BAD-ORIGIN"]}),
            "relay.ws.viewer.allowed_origins[1]",
            "config_origin_invalid",
            "PRIVATE-BAD-ORIGIN",
        ),
        (
            json!({"mode1.allow_targets": ["127.0.0.1:5900", "PRIVATE-BAD-TARGET"]}),
            "mode1.allow_targets[1]",
            "config_target_item_invalid",
            "PRIVATE-BAD-TARGET",
        ),
    ] {
        let events = Arc::new(Recorder::default());
        let clock = Arc::new(FakeClock::default());
        let fixture = config_fixture(true, clock.clone(), events.clone());
        let original = fixture.files.bytes();
        let settings = fixture
            .reload
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .running()
            .clone();
        let running = start_config_services(fixture.service.clone(), events.clone(), clock).await;
        let (cookie, csrf) = config_login(&running).await;
        let reply = config_patch(
            &running,
            &cookie,
            &csrf,
            &config_revision(&fixture),
            changes,
        )
        .await;
        assert_eq!(reply.status, 400, "{key}: {:?}", config_json(&reply));
        assert_eq!(error_code(&reply), "config_invalid");
        assert!(
            config_json(&reply)["error"]["diagnostics"]
                .as_array()
                .unwrap_or_else(|| panic!("diagnostics"))
                .iter()
                .any(|diagnostic| diagnostic["key"] == key
                    && diagnostic["code"] == code
                    && diagnostic["message"].is_string())
        );
        assert!(!String::from_utf8_lossy(&reply.body).contains(secret));
        assert_config_unchanged(&fixture, &original, &settings);
        assert_eq!(fixture.files.replaces.load(Ordering::SeqCst), 0);
        let audits = config_audits(&events);
        assert_eq!(audits.len(), 1);
        assert_eq!(audits[0]["reason"], "config_invalid");
        assert!(!audits[0].to_string().contains(secret));
    }
}

/// spec: management.md:MGT-037
#[tokio::test]
async fn config_patch_rejects_uneditable_empty_mistyped_and_poll_requests_without_audit() {
    let events = Arc::new(Recorder::default());
    let clock = Arc::new(FakeClock::default());
    let all_roles = format!(
        "{CONFIG_TEXT}\n[relay.ws.server]\nenabled=true\nallow=['127.0.0.1']\n\
         [relay.wss.server]\nenabled=true\nallow=['127.0.0.1']\n"
    );
    let fixture = config_fixture_text(&all_roles, clock.clone(), events.clone());
    let original = fixture.files.bytes();
    let settings = fixture
        .reload
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .running()
        .clone();
    let running = start_config_services(fixture.service.clone(), events.clone(), clock).await;
    let (cookie, csrf) = config_login(&running).await;
    let revision = config_revision(&fixture);
    let view = plain_exchange(running.addr, &get("/api/v1/config", &[&cookie])).await;
    let items = config_json(&view)["items"]
        .as_array()
        .unwrap_or_else(|| panic!("items"))
        .clone();
    // Every restart key and all TLS/JWKS paths are rejected.
    for item in items.iter().filter(|item| item["editable"] == false) {
        let key = item["key"].as_str().unwrap_or_default();
        let mut changes = serde_json::Map::new();
        changes.insert(key.to_owned(), item["value"].clone());
        let reply = config_patch(&running, &cookie, &csrf, &revision, Value::Object(changes)).await;
        assert_eq!(reply.status, 400, "{key}");
        assert_eq!(error_code(&reply), "invalid_request");
    }
    assert_eq!(
        items.iter().filter(|item| item["editable"] == true).count(),
        37
    );
    for changes in [
        json!({}),
        json!({"unknown.key": 1}),
        json!({"timeouts.connect_secs": "17"}),
        json!({"timeouts.connect_secs": 1.5}),
        json!({"timeouts.connect_secs": true}),
        json!({"timeouts.connect_secs": null}),
        json!({"relay.tcp_viewer.allow": "127.0.0.1"}),
        json!({"relay.tcp_viewer.allow": [1]}),
        serde_json::from_str(r#"{"timeouts.connect_secs":18446744073709551616}"#)
            .unwrap_or_else(|error| panic!("JSON: {error}")),
    ] {
        let reply = config_patch(&running, &cookie, &csrf, &revision, changes).await;
        assert_eq!(reply.status, 400);
        assert_eq!(error_code(&reply), "invalid_request");
    }
    let body = json!({"revision": revision, "changes": {"timeouts.connect_secs": 17}}).to_string();
    let reply = plain_exchange(
        running.addr,
        &patch_bytes(
            &body,
            &[&cookie, &csrf, "X-Relay-Poll: 1"],
            Some("Content-Type: application/json"),
        ),
    )
    .await;
    assert_eq!(reply.status, 400);
    assert_eq!(error_code(&reply), "invalid_request");
    assert_config_unchanged(&fixture, &original, &settings);
    assert_eq!(fixture.files.replaces.load(Ordering::SeqCst), 0);
    assert!(config_audits(&events).is_empty());
    drop(running);
    let events = Arc::new(Recorder::default());
    let clock = Arc::new(FakeClock::default());
    let text = CONFIG_TEXT
        .replace(
            "[relay.wss.viewer]\nenabled = true",
            "[relay.wss.viewer]\nenabled = false",
        )
        .replace("[mode1]\nenabled = true", "[mode1]\nenabled = false");
    let fixture = config_fixture_text(&text, clock.clone(), events.clone());
    let original = fixture.files.bytes();
    let settings = fixture
        .reload
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .running()
        .clone();
    let running = start_config_services(fixture.service.clone(), events.clone(), clock).await;
    let (cookie, csrf) = config_login(&running).await;
    for changes in [
        json!({"relay.wss.viewer.allow": ["127.0.0.1"]}),
        json!({"relay.wss.viewer.allowed_origins": ["https://example.test"]}),
        json!({"relay.ws.server.allow": ["127.0.0.1"]}),
        json!({"relay.ws.server.allowed_origins": ["https://example.test"]}),
        json!({"relay.wss.server.allow": ["127.0.0.1"]}),
        json!({"relay.wss.server.allowed_origins": ["https://example.test"]}),
        json!({"relay.wss.trusted_proxies": ["127.0.0.1"]}),
        json!({"mode1.allow_targets": ["127.0.0.1:5900"]}),
    ] {
        let reply = config_patch(
            &running,
            &cookie,
            &csrf,
            &config_revision(&fixture),
            changes,
        )
        .await;
        assert_eq!(reply.status, 400);
        assert_eq!(error_code(&reply), "invalid_request");
    }
    assert_config_unchanged(&fixture, &original, &settings);
    assert_eq!(fixture.files.replaces.load(Ordering::SeqCst), 0);
    assert!(config_audits(&events).is_empty());
}

struct TicketConfigFiles(std::path::PathBuf);

impl TicketConfigFiles {
    fn new() -> Self {
        use ::aws_lc_rs::rand::SystemRandom;
        use ::aws_lc_rs::signature::{ECDSA_P256_SHA256_FIXED_SIGNING, EcdsaKeyPair, KeyPair};

        let dir = std::env::temp_dir().join(format!(
            "vnc-relay-ticket-management-{}-{}",
            std::process::id(),
            ConnId::next()
        ));
        std::fs::create_dir(&dir).unwrap_or_else(|error| panic!("ticket directory: {error}"));
        let files = Self(dir);
        for role in ["viewer", "server"] {
            let rng = SystemRandom::new();
            let document = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng)
                .unwrap_or_else(|error| panic!("generate ticket key: {error:?}"));
            let pair =
                EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, document.as_ref())
                    .unwrap_or_else(|error| panic!("load ticket key: {error:?}"));
            let public = pair.public_key().as_ref();
            let jwks = json!({"keys": [{
                "kty": "EC", "crv": "P-256", "kid": role,
                "x": ticket_coordinate(&public[1..33]),
                "y": ticket_coordinate(&public[33..65]),
            }]})
            .to_string();
            std::fs::write(files.0.join(format!("{role}.jwks")), jwks)
                .unwrap_or_else(|error| panic!("write ticket JWKS: {error}"));
        }
        files
    }

    fn table(&self, role: &str, missing: Option<&str>) -> String {
        let path = self.0.join(format!("{role}.jwks"));
        let mut table = format!("\n[relay.ticket.{role}]\nenabled = false\n");
        for (key, value) in [
            ("issuer", "issuer.example".to_owned()),
            ("relay_host", "relay.example".to_owned()),
            ("public_keys_file", path.to_string_lossy().into_owned()),
        ] {
            if missing != Some(key) {
                table.push_str(&format!("{key} = {}\n", toml_edit::Value::from(value)));
            }
        }
        table
    }
}

impl Drop for TicketConfigFiles {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0)
            .unwrap_or_else(|error| panic!("remove ticket directory: {error}"));
    }
}

fn ticket_coordinate(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut text = String::new();
    for chunk in bytes.chunks(3) {
        let mut block = [0u8; 3];
        block[..chunk.len()].copy_from_slice(chunk);
        let value = u32::from(block[0]) << 16 | u32::from(block[1]) << 8 | u32::from(block[2]);
        for index in 0..=chunk.len() {
            text.push(char::from(
                TABLE[(value >> (18 - 6 * index) & 0x3f) as usize],
            ));
        }
    }
    text
}

/// spec: management.md:MGT-053
/// spec: operations.md:OPS-072 partial
#[tokio::test]
async fn ticket_config_view_edits_and_refusals_follow_management_contract() {
    let key_files = TicketConfigFiles::new();
    let events = Arc::new(Recorder::default());
    let clock = Arc::new(FakeClock::default());
    let text = format!(
        "{CONFIG_TEXT}\n[relay.ws.server]\nenabled = true\nallow = [\"127.0.0.1\"]\n[relay.wss.server]\nenabled = true\nallow = [\"127.0.0.1\"]\n{}{}",
        key_files.table("viewer", None),
        key_files.table("server", None)
    );
    let fixture = config_fixture_text(&text, clock.clone(), events.clone());
    let running = start_config_services(fixture.service.clone(), events.clone(), clock).await;
    let (cookie, csrf) = config_login(&running).await;
    let view = plain_exchange(running.addr, &get("/api/v1/config", &[&cookie])).await;
    assert_eq!(view.status, 200);
    let body = config_json(&view);
    let items = body["items"].as_array().unwrap_or_else(|| panic!("items"));
    assert_eq!(items.len(), 75);
    assert_eq!(
        items
            .iter()
            .filter(|item| item["reload"] == "dynamic")
            .count(),
        43
    );
    assert_eq!(
        items.iter().filter(|item| item["editable"] == true).count(),
        37
    );
    let ticket_items = items
        .iter()
        .filter(|item| {
            item["key"]
                .as_str()
                .unwrap_or_default()
                .starts_with("relay.ticket.")
        })
        .collect::<Vec<_>>();
    assert_eq!(ticket_items.len(), 10);
    for (role_index, role) in ["viewer", "server"].into_iter().enumerate() {
        for (offset, key) in [
            "enabled",
            "issuer",
            "relay_host",
            "public_keys_file",
            "max_lifetime_secs",
        ]
        .into_iter()
        .enumerate()
        {
            let item = ticket_items[role_index * 5 + offset];
            assert_eq!(item["key"], format!("relay.ticket.{role}.{key}"));
            assert_eq!(item["reload"], "dynamic");
            assert_eq!(item["editable"], key != "public_keys_file");
        }
        assert_eq!(ticket_items[role_index * 5]["value"], false);
        let lifetime = ticket_items[role_index * 5 + 4];
        assert_eq!(lifetime["value"], 60);
        assert_eq!(lifetime["min"], 10);
        assert_eq!(lifetime["max"], 600);
        assert_eq!(ticket_items[role_index * 5 + 1]["value"], "issuer.example");
        assert_eq!(ticket_items[role_index * 5 + 2]["value"], "relay.example");
        assert_eq!(
            ticket_items[role_index * 5 + 3]["value"],
            key_files
                .0
                .join(format!("{role}.jwks"))
                .to_string_lossy()
                .as_ref()
        );
    }
    for role in ["viewer", "server"] {
        for (key, value, code) in [
            ("enabled", json!("true"), "invalid_request"),
            ("issuer", json!(true), "invalid_request"),
            ("relay_host", json!(null), "invalid_request"),
            ("max_lifetime_secs", json!("60"), "invalid_request"),
            ("public_keys_file", json!("other.jwks"), "invalid_request"),
            ("issuer", json!(""), "config_invalid"),
            ("relay_host", json!("é".repeat(129)), "config_invalid"),
            ("max_lifetime_secs", json!(9), "config_invalid"),
            ("max_lifetime_secs", json!(601), "config_invalid"),
        ] {
            let original = fixture.files.bytes();
            let reply = config_patch(
                &running,
                &cookie,
                &csrf,
                &config_revision(&fixture),
                json!({format!("relay.ticket.{role}.{key}"): value}),
            )
            .await;
            assert_eq!(reply.status, 400, "{role}.{key}: {:?}", config_json(&reply));
            assert_eq!(error_code(&reply), code);
            assert_eq!(fixture.files.bytes(), original);
            if code == "config_invalid" {
                assert!(
                    config_json(&reply)["error"]["diagnostics"]
                        .as_array()
                        .unwrap_or_else(|| panic!("diagnostics"))
                        .iter()
                        .any(
                            |diagnostic| diagnostic["code"] == "config_value_out_of_range"
                                && diagnostic["key"] == format!("relay.ticket.{role}.{key}")
                        )
                );
            }
        }
    }
    for (enabled, lifetime) in [(true, 10), (false, 600), (true, 90)] {
        let reply = config_patch(
            &running,
            &cookie,
            &csrf,
            &config_revision(&fixture),
            json!({
                "relay.ticket.viewer.enabled": enabled,
                "relay.ticket.viewer.issuer": " issuer.example ",
                "relay.ticket.viewer.relay_host": "é".repeat(128),
                "relay.ticket.viewer.max_lifetime_secs": lifetime,
                "relay.ticket.server.enabled": enabled,
                "relay.ticket.server.issuer": "i",
                "relay.ticket.server.relay_host": "relay.example",
                "relay.ticket.server.max_lifetime_secs": lifetime,
            }),
        )
        .await;
        assert_eq!(reply.status, 200, "{:?}", config_json(&reply));
        let applied = config_json(&reply);
        assert!(
            applied["applied"]
                .as_array()
                .unwrap_or_else(|| panic!("applied"))
                .contains(&json!("relay.ticket.viewer.enabled"))
        );
        let reload = fixture
            .reload
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        for settings in [
            &reload.running().relay.ticket.viewer,
            &reload.running().relay.ticket.server,
        ] {
            assert_eq!(settings.enabled, enabled);
            assert_eq!(settings.max_lifetime.as_secs(), lifetime);
            assert_eq!(settings.keys.len(), usize::from(enabled));
        }
        assert_eq!(
            reload.running().relay.ticket.viewer.issuer.as_deref(),
            Some(" issuer.example ")
        );
        assert_eq!(
            reload.running().relay.ticket.viewer.relay_host.as_deref(),
            Some("é".repeat(128).as_str())
        );
    }
    drop(running);

    // Both transports disabled must reject every editable ticket kind.
    for role in ["viewer", "server"] {
        let events = Arc::new(Recorder::default());
        let clock = Arc::new(FakeClock::default());
        let text = CONFIG_TEXT
            .replace(
                "[relay.ws.viewer]\nenabled = true",
                "[relay.ws.viewer]\nenabled = false",
            )
            .replace(
                "[relay.wss.viewer]\nenabled = true",
                "[relay.wss.viewer]\nenabled = false",
            );
        let fixture = config_fixture_text(&text, clock.clone(), events.clone());
        let running = start_config_services(fixture.service.clone(), events.clone(), clock).await;
        let (cookie, csrf) = config_login(&running).await;
        let view = plain_exchange(running.addr, &get("/api/v1/config", &[&cookie])).await;
        for item in config_json(&view)["items"]
            .as_array()
            .unwrap_or_else(|| panic!("items"))
        {
            if item["key"]
                .as_str()
                .unwrap_or_default()
                .starts_with(&format!("relay.ticket.{role}."))
            {
                assert_eq!(item["editable"], false);
                if item["key"] == format!("relay.ticket.{role}.issuer")
                    || item["key"] == format!("relay.ticket.{role}.relay_host")
                {
                    assert!(item["value"].is_null());
                }
            }
        }
        let original = fixture.files.bytes();
        for (key, value) in [
            ("enabled", json!(true)),
            ("issuer", json!("issuer.example")),
            ("relay_host", json!("relay.example")),
            ("max_lifetime_secs", json!(60)),
        ] {
            let reply = config_patch(
                &running,
                &cookie,
                &csrf,
                &config_revision(&fixture),
                json!({format!("relay.ticket.{role}.{key}"): value}),
            )
            .await;
            assert_eq!(reply.status, 400);
            assert_eq!(error_code(&reply), "invalid_request");
            assert_eq!(fixture.files.bytes(), original);
        }
    }
    for role in ["viewer", "server"] {
        for missing in ["issuer", "relay_host", "public_keys_file"] {
            let events = Arc::new(Recorder::default());
            let clock = Arc::new(FakeClock::default());
            let text = format!(
                "{CONFIG_TEXT}\n[relay.ws.server]\nenabled = true\nallow = [\"127.0.0.1\"]\n{}",
                key_files.table(role, Some(missing))
            );
            let fixture = config_fixture_text(&text, clock.clone(), events.clone());
            let original = fixture.files.bytes();
            let settings = fixture
                .reload
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .running()
                .clone();
            let running =
                start_config_services(fixture.service.clone(), events.clone(), clock).await;
            let (cookie, csrf) = config_login(&running).await;
            let reply = config_patch(
                &running,
                &cookie,
                &csrf,
                &config_revision(&fixture),
                json!({format!("relay.ticket.{role}.enabled"): true}),
            )
            .await;
            assert_eq!(reply.status, 400, "{:?}", config_json(&reply));
            assert_eq!(error_code(&reply), "config_invalid");
            assert!(
                config_json(&reply)["error"]["diagnostics"]
                    .as_array()
                    .unwrap_or_else(|| panic!("diagnostics"))
                    .iter()
                    .any(|diagnostic| diagnostic["code"] == "config_missing_key"
                        && diagnostic["key"] == format!("relay.ticket.{role}.{missing}"))
            );
            assert_config_unchanged(&fixture, &original, &settings);
            assert_eq!(fixture.files.replaces.load(Ordering::SeqCst), 0);
        }
    }
}

/// spec: management.md:MGT-038
/// spec: operations.md:OPS-061
#[tokio::test]
async fn temporary_write_and_replace_failures_keep_original_bytes_and_running_values() {
    for stage in ["temporary_write", "atomic_replace"] {
        let events = Arc::new(Recorder::default());
        let clock = Arc::new(FakeClock::default());
        let fixture = config_fixture(true, clock.clone(), events.clone());
        fixture
            .files
            .fail_write
            .store(stage == "temporary_write", Ordering::SeqCst);
        fixture
            .files
            .fail_replace
            .store(stage == "atomic_replace", Ordering::SeqCst);
        let original = fixture.files.bytes();
        let settings = fixture
            .reload
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .running()
            .clone();
        let running = start_config_services(fixture.service.clone(), events.clone(), clock).await;
        let (cookie, csrf) = config_login(&running).await;
        let reply = config_patch(
            &running,
            &cookie,
            &csrf,
            &config_revision(&fixture),
            json!({"relay.tcp_viewer.allow": ["198.51.100.219"]}),
        )
        .await;
        assert_eq!(reply.status, 500);
        assert_eq!(error_code(&reply), "config_write_failed");
        assert_config_unchanged(&fixture, &original, &settings);
        assert_eq!(fixture.files.replaces.load(Ordering::SeqCst), 1);
        assert_eq!(
            fixture.files.temporary_writes.load(Ordering::SeqCst),
            usize::from(stage == "atomic_replace")
        );
        assert_rejection(&events, "config_write_failed", "relay.tcp_viewer.allow");
        assert!(
            !config_audits(&events)[0]
                .to_string()
                .contains("198.51.100.219")
        );
    }
}

/// spec: management.md:MGT-045
/// spec: operations.md:OPS-062
#[tokio::test]
async fn explicit_config_retry_after_write_repair_commits_and_audits_once() {
    let events = Arc::new(Recorder::default());
    let clock = Arc::new(FakeClock::default());
    let fixture = config_fixture(true, clock.clone(), events.clone());
    fixture.files.fail_replace.store(true, Ordering::SeqCst);
    let revision = config_revision(&fixture);
    let running = start_config_services(fixture.service.clone(), events.clone(), clock).await;
    let (cookie, csrf) = config_login(&running).await;
    let reply = config_patch(
        &running,
        &cookie,
        &csrf,
        &revision,
        json!({"timeouts.connect_secs": 17}),
    )
    .await;
    assert_eq!(reply.status, 500);
    assert_rejection(&events, "config_write_failed", "timeouts.connect_secs");
    fixture.files.fail_replace.store(false, Ordering::SeqCst);
    let view = plain_exchange(running.addr, &get("/api/v1/config", &[&cookie])).await;
    assert_eq!(config_json(&view)["revision"], revision);
    assert!(config_json(&view)["last_reload"].is_null());
    let reply = config_patch(
        &running,
        &cookie,
        &csrf,
        &revision,
        json!({"timeouts.connect_secs": 17}),
    )
    .await;
    assert_eq!(reply.status, 200, "{:?}", config_json(&reply));
    assert_eq!(
        config_json(&reply)["applied"],
        json!(["timeouts.connect_secs"])
    );
    assert_ne!(config_revision(&fixture), revision);
    assert_eq!(config_json(&reply)["revision"], config_revision(&fixture));
    assert_eq!(fixture.files.replaces.load(Ordering::SeqCst), 2);
    let reload = fixture
        .reload
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    assert_eq!(reload.running().timeouts.connect, Duration::from_secs(17));
    assert!(reload.last_reload().is_some());
    drop(reload);
    let audits = config_audits(&events);
    assert_eq!(audits.len(), 3, "{audits:?}");
    assert_eq!(audits[1]["event"], "reload_applied");
    assert_eq!(audits[1]["trigger"], "config_edit");
    assert_eq!(audits[2]["event"], "config_edited");
    assert_eq!(audits[2]["admin_account"], ACCOUNT);
    assert_eq!(audits[2]["peer_ip"], "127.0.0.1");
    assert_eq!(audits[2]["config_keys"], json!(["timeouts.connect_secs"]));
    assert_eq!(audits[2]["revision"], config_revision(&fixture));
}

struct FailedConfigQuery;
impl ConfigManagement for FailedConfigQuery {
    fn get(&self) -> Result<ConfigView, ConfigGetError> {
        Err(ConfigGetError::Internal)
    }
    fn patch(
        &self,
        _actor: ConfigActor,
        _request: ConfigPatch,
    ) -> Result<ConfigEditResult, ConfigEditError> {
        Err(ConfigEditError::Internal)
    }
}

/// spec: management.md:MGT-042 partial
#[tokio::test]
async fn config_query_internal_error_is_not_an_empty_view() {
    let events = Arc::new(Recorder::default());
    let running = start_config_services(
        Arc::new(FailedConfigQuery),
        events.clone(),
        Arc::new(FakeClock::default()),
    )
    .await;
    let (cookie, _) = config_login(&running).await;
    let reply = plain_exchange(running.addr, &get("/api/v1/config", &[&cookie])).await;
    assert_eq!(reply.status, 500);
    assert_eq!(error_code(&reply), "internal");
    assert!(config_json(&reply).get("items").is_none());
    assert!(config_audits(&events).is_empty());
}

/// spec: management.md:MGT-040 partial
#[tokio::test]
async fn disconnected_config_client_does_not_cancel_commit() {
    let events = Arc::new(Recorder::default());
    let clock = Arc::new(FakeClock::default());
    let fixture = config_fixture(true, clock.clone(), events.clone());
    let gate = Arc::new(ConfigReplacementGate::default());
    let _release_on_failure = ReleaseConfigReplacement(gate.clone());
    *fixture
        .files
        .replacement_gate
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = Some(gate.clone());
    let running = start_config_services(fixture.service.clone(), events.clone(), clock).await;
    let (cookie, csrf) = config_login(&running).await;
    let body =
        json!({"revision": config_revision(&fixture), "changes": {"timeouts.connect_secs": 17}})
            .to_string();
    let mut stream = TcpStream::connect(running.addr)
        .await
        .unwrap_or_else(|error| panic!("connect: {error}"));
    stream
        .write_all(&patch_bytes(
            &body,
            &[&cookie, &csrf],
            Some("Content-Type: application/json"),
        ))
        .await
        .unwrap_or_else(|error| panic!("write PATCH: {error}"));
    timeout(LIMIT, async {
        while !gate.entered.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap_or_else(|error| panic!("replacement starts: {error}"));
    drop(stream);
    *gate.released.lock().unwrap_or_else(PoisonError::into_inner) = true;
    gate.ready.notify_all();
    wait_for(&events, |event| matches!(event, Event::ConfigEdited { .. })).await;
    assert_eq!(
        fixture
            .reload
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .running()
            .timeouts
            .connect,
        Duration::from_secs(17)
    );
    assert_eq!(fixture.files.replaces.load(Ordering::SeqCst), 1);
    let view = plain_exchange(running.addr, &get("/api/v1/config", &[&cookie])).await;
    assert_eq!(view.status, 200);
    assert_eq!(config_json(&view)["revision"], config_revision(&fixture));
    assert_eq!(config_audits(&events).len(), 2);
}

/// spec: management.md:MGT-025 partial
#[tokio::test]
async fn config_user_actions_extend_idle_but_failures_do_not() {
    for action in [
        "get",
        "patch",
        "noop",
        "poll",
        "invalid",
        "internal",
        "write",
        "csrf",
        "disabled",
        "conflict",
        "file",
        "candidate",
        "restart",
    ] {
        let events = Arc::new(Recorder::default());
        let clock = Arc::new(FakeClock::default());
        let fixture = config_fixture(action != "disabled", clock.clone(), events.clone());
        let revision = config_revision(&fixture);
        let config: Arc<dyn ConfigManagement> = if action == "internal" {
            Arc::new(FailedConfigQuery)
        } else {
            fixture.service.clone()
        };
        let running = start_config_services(config, events, clock.clone()).await;
        let (cookie, csrf) = config_login(&running).await;
        clock.set(Duration::from_secs(400));
        let reply = match action {
            "get" | "internal" => {
                plain_exchange(running.addr, &get("/api/v1/config", &[&cookie])).await
            }
            "poll" => {
                plain_exchange(
                    running.addr,
                    &get("/api/v1/config", &[&cookie, "X-Relay-Poll: 1"]),
                )
                .await
            }
            "invalid" => {
                config_patch(
                    &running,
                    &cookie,
                    &csrf,
                    &config_revision(&fixture),
                    json!({}),
                )
                .await
            }
            "noop" => {
                config_patch(
                    &running,
                    &cookie,
                    &csrf,
                    &revision,
                    json!({"relay.tcp_viewer.allow": ["127.0.0.1"]}),
                )
                .await
            }
            "conflict" => {
                let mut bytes = fixture.files.bytes();
                bytes.extend_from_slice(b"\n# external edit\n");
                fixture.files.set(bytes);
                config_patch(
                    &running,
                    &cookie,
                    &csrf,
                    &revision,
                    json!({"timeouts.connect_secs": 17}),
                )
                .await
            }
            "file" => {
                fixture.files.unreadable.store(true, Ordering::SeqCst);
                config_patch(
                    &running,
                    &cookie,
                    &csrf,
                    &revision,
                    json!({"timeouts.connect_secs": 17}),
                )
                .await
            }
            "candidate" => {
                config_patch(
                    &running,
                    &cookie,
                    &csrf,
                    &revision,
                    json!({"timeouts.connect_secs": 0}),
                )
                .await
            }
            "restart" => {
                fixture.files.set(
                    CONFIG_TEXT
                        .replace("127.0.0.1:8443", "127.0.0.1:8444")
                        .into_bytes(),
                );
                config_patch(
                    &running,
                    &cookie,
                    &csrf,
                    &config_revision(&fixture),
                    json!({"timeouts.connect_secs": 17}),
                )
                .await
            }
            "csrf" => {
                config_patch(
                    &running,
                    &cookie,
                    "X-CSRF-Token: wrong",
                    &config_revision(&fixture),
                    json!({"timeouts.connect_secs": 17}),
                )
                .await
            }
            _ => {
                fixture
                    .files
                    .fail_replace
                    .store(action == "write", Ordering::SeqCst);
                config_patch(
                    &running,
                    &cookie,
                    &csrf,
                    &config_revision(&fixture),
                    json!({"timeouts.connect_secs": 17}),
                )
                .await
            }
        };
        assert_eq!(
            reply.status,
            match action {
                "invalid" | "candidate" => 400,
                "csrf" | "disabled" => 403,
                "conflict" | "file" | "restart" => 409,
                "internal" | "write" => 500,
                _ => 200,
            },
            "{action}"
        );
        clock.set(Duration::from_secs(650));
        let session = plain_exchange(
            running.addr,
            &get("/api/v1/session", &[&cookie, "X-Relay-Poll: 1"]),
        )
        .await;
        assert_eq!(
            session.status,
            if matches!(action, "get" | "patch" | "noop") {
                200
            } else {
                401
            },
            "{action}"
        );
    }
}

#[tokio::test]
async fn config_methods_media_type_body_limit_and_unknown_fields_follow_common_order() {
    let events = Arc::new(Recorder::default());
    let clock = Arc::new(FakeClock::default());
    let fixture = config_fixture(true, clock.clone(), events.clone());
    let running = start_config_services(fixture.service.clone(), events.clone(), clock).await;
    let (cookie, csrf) = config_login(&running).await;
    for method in ["HEAD", "PUT", "POST", "DELETE"] {
        let reply = plain_exchange(
            running.addr,
            &request(method, "/api/v1/config", &[&cookie, &csrf]),
        )
        .await;
        assert_eq!(reply.status, 405, "{method}");
        assert_eq!(reply.header("allow"), Some("GET, PATCH"));
    }
    for path in ["/api/v1/config", "/api/v1/no-such-path"] {
        let reply = plain_exchange(running.addr, &request("PATCH", path, &[&cookie])).await;
        assert_eq!(reply.status, 403);
        assert_eq!(error_code(&reply), "csrf_invalid");
    }
    let revision = config_revision(&fixture);
    let basic = json!({"revision": revision, "changes": {"timeouts.connect_secs": 17}});
    for content_type in [None, Some("Content-Type: text/plain")] {
        let reply = plain_exchange(
            running.addr,
            &patch_bytes(&basic.to_string(), &[&cookie, &csrf], content_type),
        )
        .await;
        assert_eq!(reply.status, 415);
    }
    let mut wrong_get_body = request(
        "GET",
        "/api/v1/config",
        &[&cookie, "Content-Type: text/plain", "Content-Length: 1"],
    );
    wrong_get_body.push(b'x');
    let reply = plain_exchange(running.addr, &wrong_get_body).await;
    assert_eq!(reply.status, 415);
    for body in [
        "{".to_owned(),
        "[]".to_owned(),
        json!({"changes": {"timeouts.connect_secs": 17}}).to_string(),
        json!({"revision": "bad", "changes": {"timeouts.connect_secs": 17}}).to_string(),
        json!({"revision": revision, "changes": []}).to_string(),
    ] {
        let reply = plain_exchange(
            running.addr,
            &patch_bytes(
                &body,
                &[&cookie, &csrf],
                Some("Content-Type: application/json"),
            ),
        )
        .await;
        assert_eq!(reply.status, 400);
        assert_eq!(error_code(&reply), "invalid_request");
    }
    let padded = |size: usize| {
        let bare =
            json!({"revision": revision, "changes": {"timeouts.connect_secs": 17}, "future": ""})
                .to_string();
        let body = json!({"revision": revision, "changes": {"timeouts.connect_secs": 17}, "future": "x".repeat(size - bare.len())}).to_string();
        assert_eq!(body.len(), size);
        body
    };
    let oversized = padded(65_537);
    let reply = plain_exchange(
        running.addr,
        &patch_bytes(
            &oversized,
            &[&cookie, &csrf],
            Some("Content-Type: application/json"),
        ),
    )
    .await;
    assert_eq!(reply.status, 400);
    assert_eq!(error_code(&reply), "invalid_request");
    let mut chunked = request(
        "PATCH",
        "/api/v1/config",
        &[
            &cookie,
            &csrf,
            "Content-Type: application/json",
            "Transfer-Encoding: chunked",
        ],
    );
    chunked
        .extend_from_slice(format!("{:x}\r\n{oversized}\r\n0\r\n\r\n", oversized.len()).as_bytes());
    let reply = plain_exchange(running.addr, &chunked).await;
    assert_eq!(reply.status, 400);
    assert_eq!(error_code(&reply), "invalid_request");
    assert!(config_audits(&events).is_empty());
    assert_eq!(fixture.files.replaces.load(Ordering::SeqCst), 0);
    let fitting = padded(65_536);
    let reply = plain_exchange(
        running.addr,
        &patch_bytes(
            &fitting,
            &[&cookie, &csrf],
            Some("Content-Type: application/json"),
        ),
    )
    .await;
    assert_eq!(reply.status, 200, "{:?}", config_json(&reply));
    assert_eq!(fixture.files.replaces.load(Ordering::SeqCst), 1);
}

fn management_runtime(
    admin_connections: u32,
    upgrade: Duration,
    close: Duration,
) -> Arc<ReloadCell<TlsAcceptor>> {
    let mut candidate = config_file::parse_candidate(CONFIG_TEXT.as_bytes(), &std::env::temp_dir())
        .unwrap_or_else(|error| panic!("runtime fixture: {error:?}"));
    candidate.settings.limits.admin_connections = admin_connections;
    candidate.settings.limits.relay_connections = 10;
    candidate.settings.limits.waiting = 10;
    candidate.settings.limits.buffer_bytes = 16_384;
    candidate.settings.timeouts.tls_upgrade = upgrade;
    candidate.settings.timeouts.handshake = LIMIT;
    candidate.settings.timeouts.wait = LIMIT;
    candidate.settings.timeouts.connect = LIMIT;
    candidate.settings.timeouts.close = close;
    candidate.settings.mode1.enabled = false;
    Arc::new(ReloadCell::new(ReloadSnapshot::new(candidate, None, None)))
}

/// spec: management.md:MGT-051 partial
#[tokio::test]
async fn session_queries_and_termination_provide_full_transport_and_forwarded_ip_coverage() {
    use std::collections::HashMap;
    use vnc_relay_app::relay::{ActivePeer, ActiveSessionSnapshot, ClosedSessionSnapshot};
    use vnc_relay_domain::handshake::Mode;
    use vnc_relay_domain::session::{CloseCause, SessionId, SessionState, Transport};

    struct ControllableRelayState {
        snapshot: Mutex<RelaySnapshot>,
        terminated: Mutex<HashMap<String, Arc<ClosedSessionSnapshot>>>,
    }

    impl RelayStateSource for ControllableRelayState {
        fn snapshot(&self, _clock: &dyn Clock) -> RelaySnapshot {
            self.snapshot.lock().unwrap().clone()
        }
    }

    impl RelaySessionControl for ControllableRelayState {
        fn request_terminate(
            &self,
            session_id: &str,
            _account: AccountName,
            _peer: PeerIp,
            _clock: &dyn Clock,
            _events: &dyn EventSink,
        ) -> Option<TerminationTicket> {
            let closed = self.terminated.lock().unwrap().get(session_id).cloned()?;
            let (sender, completion) = watch::channel(Some(Ok(closed)));
            drop(sender);
            Some(TerminationTicket {
                initiated: true,
                completion,
            })
        }
    }

    let fwd_v4_viewer = PeerIp::new("198.51.100.7".parse().unwrap());
    let fwd_v4_server = PeerIp::new("203.0.113.8".parse().unwrap());
    let fwd_v6_viewer = PeerIp::new("2001:db8::7".parse().unwrap());

    struct TestCase {
        id: &'static str,
        mode: Mode,
        viewer_transport: Transport,
        target_transport: Transport,
        viewer_fwd: Option<PeerIp>,
        target_fwd: Option<PeerIp>,
        viewer_sock: SocketAddr,
        target_sock: SocketAddr,
        peer: ActivePeer,
    }

    let cases = [
        TestCase {
            id: "ses_aaaaaaaaaaaaaaaaaaaaaaaaae",
            mode: Mode::Mode1,
            viewer_transport: Transport::Tcp,
            target_transport: Transport::Tcp,
            viewer_fwd: None,
            target_fwd: None,
            viewer_sock: "127.0.0.1:5001".parse().unwrap(),
            target_sock: "192.0.2.1:5900".parse().unwrap(),
            peer: ActivePeer::Target("192.0.2.1:5900".parse().unwrap()),
        },
        TestCase {
            id: "ses_aaaaaaaaaaaaaaaaaaaaaaaaai",
            mode: Mode::Mode1,
            viewer_transport: Transport::Ws,
            target_transport: Transport::Tcp,
            viewer_fwd: Some(fwd_v4_viewer),
            target_fwd: None,
            viewer_sock: "127.0.0.1:5002".parse().unwrap(),
            target_sock: "192.0.2.2:5901".parse().unwrap(),
            peer: ActivePeer::Target("192.0.2.2:5901".parse().unwrap()),
        },
        TestCase {
            id: "ses_aaaaaaaaaaaaaaaaaaaaaaaaam",
            mode: Mode::Mode2,
            viewer_transport: Transport::Tcp,
            target_transport: Transport::Tcp,
            viewer_fwd: None,
            target_fwd: None,
            viewer_sock: "127.0.0.1:5003".parse().unwrap(),
            target_sock: "127.0.0.1:5004".parse().unwrap(),
            peer: ActivePeer::Server(ConnectionContext {
                conn_id: ConnId::next(),
                entry_point: EntryPoint::TcpServer,
                peer_ip: PeerIp::from("127.0.0.1:5004".parse::<SocketAddr>().unwrap()),
                forwarded_ip: None,
                peer_addr: "127.0.0.1:5004".parse().unwrap(),
                accepted_wall: UNIX_EPOCH + Duration::from_secs(1_700_000_000),
                accepted_at: Duration::from_secs(1),
                exchange_at: Duration::from_secs(1),
                buffer_bytes: 16_384,
                handshake: LIMIT,
            }),
        },
        TestCase {
            id: "ses_aaaaaaaaaaaaaaaaaaaaaaaaaq",
            mode: Mode::Mode2,
            viewer_transport: Transport::Ws,
            target_transport: Transport::Wss,
            viewer_fwd: Some(fwd_v4_viewer),
            target_fwd: Some(fwd_v4_server),
            viewer_sock: "127.0.0.1:5005".parse().unwrap(),
            target_sock: "127.0.0.1:5006".parse().unwrap(),
            peer: ActivePeer::Server(ConnectionContext {
                conn_id: ConnId::next(),
                entry_point: EntryPoint::WssServer,
                peer_ip: PeerIp::from("127.0.0.1:5006".parse::<SocketAddr>().unwrap()),
                forwarded_ip: Some(fwd_v4_server),
                peer_addr: "127.0.0.1:5006".parse().unwrap(),
                accepted_wall: UNIX_EPOCH + Duration::from_secs(1_700_000_000),
                accepted_at: Duration::from_secs(2),
                exchange_at: Duration::from_secs(2),
                buffer_bytes: 16_384,
                handshake: LIMIT,
            }),
        },
        TestCase {
            id: "ses_aaaaaaaaaaaaaaaaaaaaaaaaau",
            mode: Mode::Mode2,
            viewer_transport: Transport::Wss,
            target_transport: Transport::Ws,
            viewer_fwd: Some(fwd_v6_viewer),
            target_fwd: None,
            viewer_sock: "[::1]:5007".parse().unwrap(),
            target_sock: "127.0.0.1:5008".parse().unwrap(),
            peer: ActivePeer::Server(ConnectionContext {
                conn_id: ConnId::next(),
                entry_point: EntryPoint::WsServer,
                peer_ip: PeerIp::from("127.0.0.1:5008".parse::<SocketAddr>().unwrap()),
                forwarded_ip: None,
                peer_addr: "127.0.0.1:5008".parse().unwrap(),
                accepted_wall: UNIX_EPOCH + Duration::from_secs(1_700_000_000),
                accepted_at: Duration::from_secs(3),
                exchange_at: Duration::from_secs(3),
                buffer_bytes: 16_384,
                handshake: LIMIT,
            }),
        },
    ];
    let mut active = Vec::new();
    let mut terminated_map = HashMap::new();
    let mut recent_closed = Vec::new();

    for tc in &cases {
        let session_id = SessionId::parse(tc.id).unwrap();
        let active_entry = ActiveSessionSnapshot {
            session_id: session_id.clone(),
            mode: tc.mode,
            viewer_transport: tc.viewer_transport,
            target_transport: tc.target_transport,
            viewer_forwarded_ip: tc.viewer_fwd,
            target_forwarded_ip: tc.target_fwd,
            started_at: UNIX_EPOCH + Duration::from_secs(1_700_000_000),
            started_mono: Duration::from_secs(1),
            duration: Duration::from_secs(5),
            state: SessionState::Active,
            viewer: ConnectionContext {
                conn_id: ConnId::next(),
                entry_point: match tc.viewer_transport {
                    Transport::Tcp => EntryPoint::TcpViewer,
                    Transport::Ws => EntryPoint::WsViewer,
                    Transport::Wss => EntryPoint::WssViewer,
                },
                peer_ip: PeerIp::from(tc.viewer_sock),
                forwarded_ip: tc.viewer_fwd,
                peer_addr: tc.viewer_sock,
                accepted_wall: UNIX_EPOCH + Duration::from_secs(1_700_000_000),
                accepted_at: Duration::from_secs(1),
                exchange_at: Duration::from_secs(1),
                buffer_bytes: 16_384,
                handshake: LIMIT,
            },
            peer: tc.peer.clone(),
            relay_id_ref: None,
            bytes_viewer_to_target: 100,
            bytes_target_to_viewer: 200,
        };
        active.push(active_entry);

        let closed_entry = Arc::new(ClosedSessionSnapshot {
            session_id: session_id.clone(),
            mode: tc.mode,
            viewer_transport: tc.viewer_transport,
            target_transport: tc.target_transport,
            started_at: UNIX_EPOCH + Duration::from_secs(1_700_000_000),
            ended_at: UNIX_EPOCH + Duration::from_secs(1_700_000_010),
            duration: Duration::from_secs(10),
            bytes_viewer_to_target: 100,
            bytes_target_to_viewer: 200,
            close_cause: CloseCause::AdminTerminated,
            relay_id_ref: None,
            side: None,
        });
        terminated_map.insert(tc.id.to_owned(), closed_entry.clone());
        recent_closed.push((*closed_entry).clone());
    }

    let relay_state = Arc::new(ControllableRelayState {
        snapshot: Mutex::new(RelaySnapshot {
            lifecycle: RelayLifecycle::Running,
            observed_at: Duration::from_secs(10),
            handshaking: 0,
            target_connecting: 0,
            max_entries: 16,
            waiting: Vec::new(),
            active,
            recent_closed,
        }),
        terminated: Mutex::new(terminated_map),
    });

    let cell = management_runtime(ROOMY_LM, UPGRADE, Duration::from_secs(1));
    let clock = Arc::new(SystemClock::new());
    let events = Arc::new(Recorder::default());
    let auth = admin_auth(
        events.clone(),
        session_policy(),
        limit_policy(),
        Arc::new(Argon2Verifier),
        clock.clone(),
    );
    let query = Arc::new(ManagementQueryService::new(
        relay_state.clone(),
        ConnectionBudget::new(),
        ConnectionBudget::new(),
        clock.clone(),
        cell.clone(),
    ));
    let terminate = Arc::new(ManagementTerminateService::new(
        relay_state.clone(),
        clock.clone(),
        events.clone(),
    ));
    let running = start_services(
        None,
        ConnectionBudget::new(),
        cell,
        ManagementServices {
            auth,
            query,
            terminate,
            config: config_fixture(false, clock.clone(), events.clone()).service,
            logs: Arc::new(RecentLogs::new(1000)),
        },
        events,
    )
    .await;

    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    assert_eq!(login.status, 200);
    let cookie = cookie_line(&login);
    let csrf = csrf_line(&login);

    // 1. GET /api/v1/sessions
    let list = authenticated_query(&running, &cookie, "/api/v1/sessions").await;
    let items = list["items"].as_array().expect("items array");
    assert_eq!(items.len(), cases.len());
    for tc in &cases {
        let item = items
            .iter()
            .find(|it| it["session_id"].as_str() == Some(tc.id))
            .unwrap_or_else(|| panic!("missing session {}", tc.id));
        assert_eq!(item["mode"], tc.mode.as_str());
        assert_eq!(item["viewer_transport"], tc.viewer_transport.as_str());
        assert_eq!(item["target_transport"], tc.target_transport.as_str());
        assert_eq!(
            item["viewer_forwarded_ip"],
            json!(tc.viewer_fwd.map(|ip| ip.to_string()))
        );
        assert_eq!(
            item["target_forwarded_ip"],
            json!(tc.target_fwd.map(|ip| ip.to_string()))
        );
        assert_eq!(
            item["viewer_address"]["ip"],
            tc.viewer_sock.ip().to_string()
        );
        assert_eq!(item["viewer_address"]["port"], tc.viewer_sock.port());
        assert_eq!(
            item["target_address"]["ip"],
            tc.target_sock.ip().to_string()
        );
        assert_eq!(item["target_address"]["port"], tc.target_sock.port());
        assert!(item.get("viewer_forwarded_ip_unavailable_reason").is_none());
        assert!(item.get("target_forwarded_ip_unavailable_reason").is_none());

        // 2. GET /api/v1/sessions/{session_id}
        let detail =
            authenticated_query(&running, &cookie, &format!("/api/v1/sessions/{}", tc.id)).await;
        let session = &detail["session"];
        assert_eq!(session, item);

        // 3. POST /api/v1/sessions/{session_id}/terminate
        let terminate_cmd = request(
            "POST",
            &format!("/api/v1/sessions/{}/terminate", tc.id),
            &[&cookie, &csrf],
        );
        let reply = plain_exchange(running.addr, &terminate_cmd).await;
        assert_eq!(reply.status, 200, "terminate {}", tc.id);
        let body = json_body(&reply);
        assert_eq!(body["result"], "terminated");
        let closed = &body["closed_session"];
        assert_eq!(closed["session_id"], tc.id);
        assert_eq!(closed["mode"], tc.mode.as_str());
        assert_eq!(closed["viewer_transport"], tc.viewer_transport.as_str());
        assert_eq!(closed["target_transport"], tc.target_transport.as_str());
        assert!(closed.get("viewer_forwarded_ip").is_none());
        assert!(closed.get("target_forwarded_ip").is_none());
        assert!(
            closed
                .get("viewer_forwarded_ip_unavailable_reason")
                .is_none()
        );
        assert!(
            closed
                .get("target_forwarded_ip_unavailable_reason")
                .is_none()
        );
    }

    // 4. GET /api/v1/closed-sessions
    let closed_list = authenticated_query(&running, &cookie, "/api/v1/closed-sessions").await;
    let closed_items = closed_list["items"].as_array().expect("closed items array");
    assert_eq!(closed_items.len(), cases.len());
    for tc in &cases {
        let item = closed_items
            .iter()
            .find(|it| it["session_id"].as_str() == Some(tc.id))
            .unwrap_or_else(|| panic!("missing closed session {}", tc.id));
        assert_eq!(item["mode"], tc.mode.as_str());
        assert_eq!(item["viewer_transport"], tc.viewer_transport.as_str());
        assert_eq!(item["target_transport"], tc.target_transport.as_str());
        assert!(item.get("viewer_forwarded_ip").is_none());
        assert!(item.get("target_forwarded_ip").is_none());
        assert_eq!(item["close_cause"], "admin_terminated");
        assert!(item.get("viewer_forwarded_ip_unavailable_reason").is_none());
        assert!(item.get("target_forwarded_ip_unavailable_reason").is_none());
    }
}

/// spec: management.md:MGT-046
#[tokio::test]
async fn recent_logs_query_basics_limit_order_and_bounds() {
    let running = start(None, ROOMY_LM).await;
    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    let cookie = cookie_line(&login);

    let path = std::env::temp_dir().join(format!(
        "vnc-relay-mgt046-{}-{}.jsonl",
        std::process::id(),
        ConnId::next()
    ));
    let sink = JsonLinesEventSink::open(
        &vnc_relay_app::settings::LogOutput::File(path.clone()),
        |_| {},
        |_| {},
    )
    .unwrap()
    .with_recent_logs(Arc::clone(&running.logs));
    for _ in 0..5 {
        sink.record(&Event::UpgradeRejected {
            conn_id: ConnId::next(),
            listener: "wss",
            role: Some(vnc_relay_domain::entry_point::Role::Server),
            reason: RejectReason::IpNotAllowed,
            peer_ip: PeerIp::new("127.0.0.1".parse().unwrap()),
            forwarded_ip: Some(PeerIp::new("198.51.100.8".parse().unwrap())),
            origin: None,
            ticket_jti: None,
        });
    }
    drop(sink);
    let expected: Vec<Value> = std::fs::read_to_string(&path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    std::fs::remove_file(path).unwrap();

    // 1. limit omitted -> defaults to 200, returns all 5 in ascending seq order
    let res = authenticated_query(&running, &cookie, "/api/v1/logs").await;
    assert_eq!(res["max_entries"], 1000);
    assert_eq!(res["oldest_seq"], 1);
    assert_eq!(res["latest_seq"], 5);
    assert_eq!(res["has_more"], false);
    let items = res["items"].as_array().unwrap();
    assert_eq!(items.len(), 5);
    for (i, item) in items.iter().enumerate() {
        assert_eq!(item["seq"], (i + 1) as u64);
        assert_eq!(item["entry"], expected[i]);
    }

    // 2. limit specified (limit=2) -> newest 2 matches in ascending order, has_more is true
    let res = authenticated_query(&running, &cookie, "/api/v1/logs?limit=2").await;
    assert_eq!(res["max_entries"], 1000);
    assert_eq!(res["oldest_seq"], 1);
    assert_eq!(res["latest_seq"], 5);
    assert_eq!(res["has_more"], true);
    let items = res["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["seq"], 4);
    assert_eq!(items[1]["seq"], 5);
}

/// spec: management.md:MGT-047
#[tokio::test]
async fn recent_logs_filters_alone_and_combined_and_session_relation() {
    let running = start(None, ROOMY_LM).await;
    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    let cookie = cookie_line(&login);

    let sink = JsonLinesEventSink::stdout(|_| {}).with_recent_logs(Arc::clone(&running.logs));
    let conn1 = ConnId::next();
    let conn2 = ConnId::next();
    let sid1 = vnc_relay_domain::session::SessionId::from_block([1; 16]);
    let sid2 = vnc_relay_domain::session::SessionId::from_block([2; 16]);
    let ip = PeerIp::new(std::net::IpAddr::from([127, 0, 0, 1]));

    // seq 1
    sink.record(&Event::ConnectionAccepted {
        conn_id: conn1,
        listener: vnc_relay_domain::entry_point::Listener::TcpViewer,
        peer_ip: ip,
    });
    // seq 2
    sink.record(&Event::SessionStarted {
        session_id: sid1.clone(),
        mode: vnc_relay_domain::handshake::Mode::Mode1,
        viewer_transport: Transport::Tcp,
        server_transport: None,
        viewer_conn_id: conn1,
        viewer_peer_ip: ip,
        viewer_forwarded_ip: None,
        server_conn_id: None,
        server_peer_ip: None,
        server_forwarded_ip: None,
        relay_id_ref: None,
        waited_ms: None,
        target_ip: None,
        target_port: None,
        target_host: None,
    });
    // seq 3
    sink.record(&Event::ConnectionAccepted {
        conn_id: conn2,
        listener: vnc_relay_domain::entry_point::Listener::TcpServer,
        peer_ip: ip,
    });
    // seq 4
    sink.record(&Event::SessionStarted {
        session_id: sid2,
        mode: vnc_relay_domain::handshake::Mode::Mode2,
        viewer_transport: Transport::Tcp,
        server_transport: Some(Transport::Tcp),
        viewer_conn_id: conn2,
        viewer_peer_ip: ip,
        viewer_forwarded_ip: None,
        server_conn_id: Some(conn2),
        server_peer_ip: Some(ip),
        server_forwarded_ip: None,
        relay_id_ref: Some(vnc_relay_app::events::RelayIdRef::from_hash_prefix([
            0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89,
        ])),
        waited_ms: Some(100),
        target_ip: None,
        target_port: None,
        target_host: None,
    });
    // seq 5
    sink.record(&Event::ConfigWarning {
        warning: vnc_relay_app::settings::ConfigWarning::TrustAllProxies {
            listener: vnc_relay_domain::entry_point::Listener::Ws,
            item: vnc_relay_domain::allow_list::IpItem::parse("0.0.0.0/0").unwrap(),
        },
        phase: vnc_relay_app::events::Phase::Startup,
    });

    // 1. level filter
    let res = authenticated_query(&running, &cookie, "/api/v1/logs?level=warn").await;
    let items = res["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["entry"]["event"], "trust_all_proxies");

    // level with multiple values
    let res = authenticated_query(&running, &cookie, "/api/v1/logs?level=warn,info").await;
    assert_eq!(res["items"].as_array().unwrap().len(), 5);

    // 2. event filter alone
    let res = authenticated_query(&running, &cookie, "/api/v1/logs?event=session_started").await;
    assert_eq!(res["items"].as_array().unwrap().len(), 2);

    // unknown event name -> empty result, bounds kept
    let res = authenticated_query(&running, &cookie, "/api/v1/logs?event=unknown_event").await;
    assert_eq!(res["items"].as_array().unwrap().len(), 0);
    assert_eq!(res["oldest_seq"], 1);
    assert_eq!(res["latest_seq"], 5);

    // 3. q search: ASCII case-folding partial search on values
    let res = authenticated_query(&running, &cookie, "/api/v1/logs?q=ABCDEF0123456789").await;
    assert_eq!(res["items"].as_array().unwrap().len(), 1);
    assert_eq!(res["items"][0]["seq"], 4);

    // q matches numbers: 100
    let res = authenticated_query(&running, &cookie, "/api/v1/logs?q=100").await;
    assert!(
        res["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["seq"] == 4)
    );

    // q does not match field names: "viewer_conn_id" is a key name only
    let res = authenticated_query(&running, &cookie, "/api/v1/logs?q=viewer_conn_id").await;
    assert_eq!(res["items"].as_array().unwrap().len(), 0);

    // 4. session_id with relation to connection events
    let res = authenticated_query(
        &running,
        &cookie,
        &format!("/api/v1/logs?session_id={sid1}"),
    )
    .await;
    let items = res["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["entry"]["event"], "connection_accepted");
    assert_eq!(items[0]["entry"]["conn_id"], conn1.to_string());
    assert_eq!(items[1]["entry"]["event"], "session_started");

    // 5. combined filters: level + event + q
    let res = authenticated_query(
        &running,
        &cookie,
        "/api/v1/logs?level=info&event=connection_accepted&q=tcp_viewer",
    )
    .await;
    assert_eq!(res["items"].as_array().unwrap().len(), 1);
    assert_eq!(res["items"][0]["seq"], 1);
}

/// spec: management.md:MGT-048
#[tokio::test]
async fn recent_logs_cursors_has_more_and_validation() {
    let running = start(None, ROOMY_LM).await;
    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    let cookie = cookie_line(&login);

    let sink = JsonLinesEventSink::stdout(|_| {}).with_recent_logs(Arc::clone(&running.logs));
    for _ in 0..10 {
        sink.record(&Event::ServerStarting);
    }

    // Multi-page with before:
    let p1 = authenticated_query(&running, &cookie, "/api/v1/logs?limit=3").await;
    assert_eq!(p1["has_more"], true);
    let items = p1["items"].as_array().unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["seq"], 8);
    assert_eq!(items[1]["seq"], 9);
    assert_eq!(items[2]["seq"], 10);

    let p2 = authenticated_query(&running, &cookie, "/api/v1/logs?before=8&limit=3").await;
    assert_eq!(p2["has_more"], true);
    let items = p2["items"].as_array().unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["seq"], 5);
    assert_eq!(items[1]["seq"], 6);
    assert_eq!(items[2]["seq"], 7);

    let p3 = authenticated_query(&running, &cookie, "/api/v1/logs?before=5&limit=3").await;
    assert_eq!(p3["has_more"], true);
    let items = p3["items"].as_array().unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["seq"], 2);
    assert_eq!(items[1]["seq"], 3);
    assert_eq!(items[2]["seq"], 4);

    let p4 = authenticated_query(&running, &cookie, "/api/v1/logs?before=2&limit=3").await;
    assert_eq!(p4["has_more"], false);
    let items = p4["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["seq"], 1);

    // Multi-page with after:
    let a1 = authenticated_query(&running, &cookie, "/api/v1/logs?after=0&limit=3").await;
    assert_eq!(a1["has_more"], true);
    let items = a1["items"].as_array().unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["seq"], 1);
    assert_eq!(items[1]["seq"], 2);
    assert_eq!(items[2]["seq"], 3);

    let a2 = authenticated_query(&running, &cookie, "/api/v1/logs?after=3&limit=3").await;
    assert_eq!(a2["has_more"], true);
    let items = a2["items"].as_array().unwrap();
    assert_eq!(items[0]["seq"], 4);
    assert_eq!(items[1]["seq"], 5);
    assert_eq!(items[2]["seq"], 6);

    let a4 = authenticated_query(&running, &cookie, "/api/v1/logs?after=9&limit=3").await;
    assert_eq!(a4["has_more"], false);
    let items = a4["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["seq"], 10);

    // Cursors outside retained range
    let out = authenticated_query(&running, &cookie, "/api/v1/logs?before=1").await;
    assert_eq!(out["items"].as_array().unwrap().len(), 0);
    assert_eq!(out["has_more"], false);
    let out = authenticated_query(&running, &cookie, "/api/v1/logs?after=100").await;
    assert_eq!(out["items"].as_array().unwrap().len(), 0);
    assert_eq!(out["has_more"], false);

    // Refusals (400 invalid_request)
    // Unknown parameters are ignored (contract §3 Producer rule)
    let ign1 = authenticated_query(&running, &cookie, "/api/v1/logs?unknown_param=1&foo=bar").await;
    assert_eq!(ign1["items"].as_array().unwrap().len(), 10);
    let ign2 = authenticated_query(&running, &cookie, "/api/v1/logs?unknown_flag").await;
    assert_eq!(ign2["items"].as_array().unwrap().len(), 10);

    // Repeated level and event are combined
    let comb1 = authenticated_query(&running, &cookie, "/api/v1/logs?level=info&level=warn").await;
    assert_eq!(comb1["items"].as_array().unwrap().len(), 10);
    let comb2 = authenticated_query(
        &running,
        &cookie,
        "/api/v1/logs?event=server_starting&event=unknown_event",
    )
    .await;
    assert_eq!(comb2["items"].as_array().unwrap().len(), 10);

    // + and %20 are equivalent spaces in q
    sink.record(&Event::LogReopenFailed {
        detail: "broken pipe error".to_owned(),
    });
    let q_plus = authenticated_query(&running, &cookie, "/api/v1/logs?q=broken+pipe").await;
    let q_pct = authenticated_query(&running, &cookie, "/api/v1/logs?q=broken%20pipe").await;
    assert_eq!(q_plus["items"], q_pct["items"]);
    assert_eq!(q_plus["items"].as_array().unwrap().len(), 1);
    assert_eq!(q_plus["items"][0]["entry"]["event"], "log_reopen_failed");
    // Refusals (400 invalid_request)
    for bad_path in [
        "/api/v1/logs?before=5&after=3",
        "/api/v1/logs?before=0",
        "/api/v1/logs?before=-1",
        "/api/v1/logs?before=1.5",
        "/api/v1/logs?after=-1",
        "/api/v1/logs?limit=0",
        "/api/v1/logs?limit=1001",
        "/api/v1/logs?limit=abc",
        "/api/v1/logs?q=",
        "/api/v1/logs?q=%ZZ",
        "/api/v1/logs?level=",
        "/api/v1/logs?level=invalid",
        "/api/v1/logs?event=",
        "/api/v1/logs?event=UPPERCASE",
        "/api/v1/logs?session_id=bad%20id",
        "/api/v1/logs?limit=10&limit=20",
        "/api/v1/logs?before=10&before=20",
        "/api/v1/logs?after=10&after=20",
        "/api/v1/logs?q=a&q=b",
        "/api/v1/logs?session_id=ses_a&session_id=ses_b",
        // Known parameter without = has empty value -> 400
        "/api/v1/logs?level",
        "/api/v1/logs?limit",
        "/api/v1/logs?q",
        "/api/v1/logs?session_id",
        "/api/v1/logs?before",
        "/api/v1/logs?after",
    ] {
        let reply = plain_exchange(running.addr, &get(bad_path, &[&cookie])).await;
        assert_eq!(reply.status, 400, "{bad_path}");
        assert_eq!(error_code(&reply), "invalid_request", "{bad_path}");
    }
}

/// spec: management.md:MGT-049
#[tokio::test]
async fn recent_logs_zero_max_entries_and_empty_buffer_responses() {
    // 1. Empty buffer with max_entries = 1000
    let running = start(None, ROOMY_LM).await;
    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    let cookie = cookie_line(&login);

    let res = authenticated_query(&running, &cookie, "/api/v1/logs").await;
    assert_eq!(res["max_entries"], 1000);
    assert!(res["oldest_seq"].is_null());
    assert!(res["latest_seq"].is_null());
    assert_eq!(res["has_more"], false);
    assert_eq!(res["items"].as_array().unwrap().len(), 0);

    // 2. Capacity zero still uses the authenticated HTTP route.
    let zero_logs = Arc::new(RecentLogs::new(0));
    let zero_sink = JsonLinesEventSink::stdout(|_| {}).with_recent_logs(Arc::clone(&zero_logs));
    zero_sink.record(&Event::ServerStarting);
    let running = start_with_ports_and_logs(
        None,
        ROOMY_LM,
        UPGRADE,
        session_policy(),
        limit_policy(),
        Arc::new(Argon2Verifier),
        (Arc::new(SystemClock::new()), zero_logs),
    )
    .await;
    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    let cookie = cookie_line(&login);
    let res = authenticated_query(&running, &cookie, "/api/v1/logs").await;
    assert_eq!(res["max_entries"], 0);
    assert!(res["oldest_seq"].is_null());
    assert!(res["latest_seq"].is_null());
    assert_eq!(res["has_more"], false);
    assert_eq!(res["items"], json!([]));
}

/// spec: management.md:MGT-050
#[tokio::test]
async fn recent_logs_polling_idle_timeout_and_unauthenticated_guard() {
    let idle = Duration::from_secs(400);
    let clock = Arc::new(FakeClock::default());
    let session = AdminSessionPolicy::new(idle, Duration::from_secs(3600));
    let running = start_with_ports(
        None,
        ROOMY_LM,
        UPGRADE,
        session,
        limit_policy(),
        Arc::new(Argon2Verifier),
        clock.clone(),
    )
    .await;
    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    let cookie = cookie_line(&login);

    let sink = JsonLinesEventSink::stdout(|_| {}).with_recent_logs(Arc::clone(&running.logs));
    sink.record(&Event::ServerStarting);

    // 1. Unauthenticated request -> 401 unauthenticated, no log exposed
    let unauth = plain_exchange(running.addr, &get("/api/v1/logs", &[])).await;
    assert_eq!(unauth.status, 401);
    assert_eq!(error_code(&unauth), "unauthenticated");
    assert_eq!(
        unauth.header("www-authenticate"),
        Some(r#"Session realm="vnc-relay""#)
    );
    assert!(!String::from_utf8_lossy(&unauth.body).contains("server_starting"));

    // 2. Method not allowed: POST, DELETE, PATCH -> 405 invalid_request with Allow: GET
    let session_view = authenticated_query(&running, &cookie, "/api/v1/session").await;
    let csrf = session_view["csrf_token"].as_str().unwrap();
    for method in ["POST", "DELETE", "PATCH"] {
        let req = request(
            method,
            "/api/v1/logs",
            &[&cookie, &format!("X-CSRF-Token: {csrf}")],
        );
        let reply = plain_exchange(running.addr, &req).await;
        assert_eq!(reply.status, 405, "{method}");
        assert_eq!(reply.header("allow"), Some("GET"), "{method}");
        assert_eq!(error_code(&reply), "invalid_request", "{method}");
    }

    // 3. Media type check: GET with non-empty non-JSON body -> 415 invalid_request
    let mut sized = request(
        "GET",
        "/api/v1/logs",
        &[
            cookie.as_str(),
            "Content-Type: text/plain",
            "Content-Length: 5",
        ],
    );
    sized.extend_from_slice(b"hello");
    let reply = plain_exchange(running.addr, &sized).await;
    assert_eq!(reply.status, 415);
    assert_eq!(error_code(&reply), "invalid_request");

    // 4. Polling vs user request idle timer behavior:
    clock.set(Duration::from_secs(250));
    let poll_req = get("/api/v1/logs", &[&cookie, "X-Relay-Poll: 1"]);
    let reply = plain_exchange(running.addr, &poll_req).await;
    assert_eq!(reply.status, 200);

    // Move past the original idle deadline without a real-time race.
    clock.set(Duration::from_secs(500));
    let expired = plain_exchange(running.addr, &get("/api/v1/logs", &[&cookie])).await;
    assert_eq!(
        expired.status, 401,
        "session must expire because X-Relay-Poll did not extend idle"
    );
    assert_eq!(error_code(&expired), "unauthenticated");

    // 5. User request DOES extend idle timer:
    let login2 = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    let cookie2 = cookie_line(&login2);
    clock.set(Duration::from_secs(750));
    let user_req = get("/api/v1/logs", &[&cookie2]);
    let reply = plain_exchange(running.addr, &user_req).await;
    assert_eq!(reply.status, 200);

    clock.set(Duration::from_secs(1000));
    let still_valid = plain_exchange(running.addr, &get("/api/v1/logs", &[&cookie2])).await;
    assert_eq!(
        still_valid.status, 200,
        "session must remain valid because user request extended idle"
    );
}
