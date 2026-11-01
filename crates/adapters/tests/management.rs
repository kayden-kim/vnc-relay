// Contract tests of the management listener (docs/architectures/management-listener.md,
// docs/contracts/management-api-v1.md §1.3, §1.4.1, §1.11).
//
// Fixtures under tests/fixtures/tls are shared with tls.rs. Each test starts its
// own listener on loopback. Connections that hold Lm stay open until the test ends.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rustls::crypto::aws_lc_rs;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::version::{TLS12, TLS13};
use rustls::{ClientConfig, RootCertStore, SupportedProtocolVersion};
use serde_json::{Value, json};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use tokio_rustls::{TlsAcceptor, TlsConnector};
use vnc_relay_adapters::accounts_file::{Argon2Hasher, Argon2Verifier};
use vnc_relay_adapters::clock::SystemClock;
use vnc_relay_adapters::crypto::SystemTokenSource;
use vnc_relay_adapters::management_http::{DashboardHints, ManagementListener};
use vnc_relay_adapters::tls;
use vnc_relay_app::events::{Event, RejectReason, Stage};
use vnc_relay_app::management::login::dummy_hash;
use vnc_relay_app::management::query::ManagementQueryService;
use vnc_relay_app::management::{AdminAuth, AdminPorts};
use vnc_relay_app::ports::{
    Clock, EventSink, HashError, PasswordHasher, PasswordVerifier, RelayLifecycle, RelayStateSource,
};
use vnc_relay_app::relay::RelaySnapshot;
use vnc_relay_app::reload::snapshot::{RelayAllow, ReloadCell, ReloadSnapshot};
use vnc_relay_domain::account::{Account, AccountName, Argon2Params, Password, PasswordHash};
use vnc_relay_domain::admin_session::AdminSessionPolicy;
use vnc_relay_domain::budget::ConnectionBudget;
use vnc_relay_domain::login_limit::{LimitScope, LoginLimitPolicy};
use vnc_relay_domain::peer_ip::PeerIp;

/// Upper bound for one exchange. A hang fails the test instead of blocking it.
const LIMIT: Duration = Duration::from_secs(10);

/// Lm for tests that send several requests in a row. Each request uses a new
/// connection, so a small limit could still be held by a closing one.
const ROOMY_LM: u32 = 8;

/// U of the default configuration (config-v2 §1.2, `tls_upgrade_secs` = 10).
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
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap_or_else(|error| panic!("loopback bind: {error}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|error| panic!("loopback address: {error}"));
    let events = Arc::new(Recorder::default());
    let log: Arc<dyn EventSink> = events.clone();
    let budget = ConnectionBudget::new(admin_connections);
    let auth = admin_auth(
        Arc::clone(&log),
        session,
        limit,
        verifier,
        Arc::clone(&clock),
    );
    let relay_budget = ConnectionBudget::new(10);
    let query = Arc::new(ManagementQueryService::new(
        Arc::new(EmptyRelaySource),
        relay_budget,
        Arc::clone(&budget),
        clock,
    ));
    let dashboard = DashboardHints::new(
        Duration::from_secs(2),
        Duration::from_secs(5),
        Duration::from_secs(10),
    );
    let tls = tls.map(|acceptor| {
        let snapshot = ReloadSnapshot::new(RelayAllow::default(), None, Some(acceptor));
        tls::TlsSource::management(Arc::new(ReloadCell::new(snapshot)))
    });
    let listener = ManagementListener::new(listener, tls, budget, upgrade, auth, dashboard, query);
    tokio::spawn(listener.run(log));
    Running { addr, events }
}

struct EmptyRelaySource;

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

// Covers the login and the session view over HTTPS. The operational queries
// (4.4) are not built yet, so the session view stands in for them here.
/// spec: management.md:MGT-017 partial
#[tokio::test]
async fn https_login_sets_a_secure_cookie_and_opens_the_session_view() {
    let running = start(Some(tls_acceptor()), ROOMY_LM).await;

    let login = https_exchange(running.addr, &[&TLS13], &login_request(ACCOUNT, PASSWORD)).await;
    assert_eq!(login.status, 200);
    check_session_cookie(&login, true);
    let view = json_body(&login);
    assert_eq!(view["account"], ACCOUNT);
    assert_eq!(view["dashboard"]["poll_secs"], 2);
    assert_eq!(view["dashboard"]["query_timeout_secs"], 5);
    assert_eq!(view["dashboard"]["terminate_timeout_secs"], 15);

    let cookie = cookie_line(&login);
    let reply = https_exchange(
        running.addr,
        &[&TLS13],
        &get("/api/v1/session", &[cookie.as_str()]),
    )
    .await;
    assert_eq!(reply.status, 200);
    assert_eq!(json_body(&reply)["account"], ACCOUNT);
}

// Covers the login and the session view over plain loopback HTTP. The operational
// queries (4.4) are not built yet, so the session view stands in for them here.
/// spec: management.md:MGT-017 partial
#[tokio::test]
async fn plain_http_login_sets_a_cookie_without_secure() {
    let running = start(None, ROOMY_LM).await;

    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    assert_eq!(login.status, 200);
    check_session_cookie(&login, false);

    let cookie = cookie_line(&login);
    let reply = plain_exchange(running.addr, &get("/api/v1/session", &[cookie.as_str()])).await;
    assert_eq!(reply.status, 200);
    assert_eq!(json_body(&reply)["account"], ACCOUNT);
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

// Covers the expired session for the session view and for logout. The query (4.4)
// and terminate (4.5) paths are not built yet, and relay sessions are not checked here.
/// spec: management.md:MGT-019 partial
#[tokio::test]
async fn expired_session_cookie_is_refused_by_the_view_and_by_logout() {
    let session = AdminSessionPolicy::new(Duration::from_millis(300), Duration::from_secs(3600));
    let running = start_with(None, ROOMY_LM, UPGRADE, session, limit_policy()).await;

    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    assert_eq!(login.status, 200);
    let cookie = cookie_line(&login);
    let csrf = csrf_line(&login);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let view = plain_exchange(running.addr, &get("/api/v1/session", &[cookie.as_str()])).await;
    assert_eq!(view.status, 401);
    assert_eq!(error_code(&view), "unauthenticated");
    assert_eq!(view.header("set-cookie"), Some(CLEARED_COOKIE));

    let logout = plain_exchange(
        running.addr,
        &request(
            "DELETE",
            "/api/v1/session",
            &[cookie.as_str(), csrf.as_str()],
        ),
    )
    .await;
    assert_eq!(logout.status, 401);
    assert_eq!(error_code(&logout), "unauthenticated");
}

// Covers the idle deadline: automatic polls (X-Relay-Poll: 1) do not extend it. Relay
// sessions are not checked here.
/// spec: management.md:MGT-025 partial
#[tokio::test]
async fn automatic_polls_do_not_extend_the_idle_deadline() {
    let idle = Duration::from_millis(500);
    let session = AdminSessionPolicy::new(idle, Duration::from_secs(3600));
    let running = start_with(None, ROOMY_LM, UPGRADE, session, limit_policy()).await;

    let started = Instant::now();
    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    let cookie = cookie_line(&login);
    let mut refused = None;
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let poll = plain_exchange(
            running.addr,
            &get("/api/v1/session", &[cookie.as_str(), "X-Relay-Poll: 1"]),
        )
        .await;
        if poll.status == 401 {
            refused = Some(poll);
            break;
        }
        assert_eq!(poll.status, 200, "poll before the idle deadline");
    }
    let refused = refused.unwrap_or_else(|| panic!("polls do not extend the idle deadline"));
    assert_eq!(error_code(&refused), "unauthenticated");
    assert!(
        started.elapsed() >= idle,
        "refused only after the idle time"
    );
}

// Covers the absolute deadline: user actions extend the idle deadline, but the
// absolute deadline still ends the session. Relay sessions are not checked here.
/// spec: management.md:MGT-025 partial
#[tokio::test]
async fn user_actions_keep_the_session_until_the_absolute_deadline() {
    let max_lifetime = Duration::from_millis(1200);
    let session = AdminSessionPolicy::new(Duration::from_millis(400), max_lifetime);
    let running = start_with(None, ROOMY_LM, UPGRADE, session, limit_policy()).await;

    let started = Instant::now();
    let login = plain_exchange(running.addr, &login_request(ACCOUNT, PASSWORD)).await;
    let cookie = cookie_line(&login);
    let mut refused = None;
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(150)).await;
        let view = plain_exchange(running.addr, &get("/api/v1/session", &[cookie.as_str()])).await;
        if view.status == 401 {
            refused = Some(view);
            break;
        }
        assert_eq!(view.status, 200, "user action before the absolute deadline");
    }
    let refused = refused.unwrap_or_else(|| panic!("the absolute deadline ends the session"));
    assert_eq!(error_code(&refused), "unauthenticated");
    assert!(
        started.elapsed() >= max_lifetime,
        "refused only after the absolute lifetime"
    );
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

// Covers the CSRF check of logout. The terminate path (4.5) is not built yet, so the
// check of a terminate request is not covered here.
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
