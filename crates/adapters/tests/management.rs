// Contract tests of the management listener (docs/architectures/management-listener.md,
// docs/contracts/management-api-v1.md §1.3, §1.4.1, §1.11).
//
// Fixtures under tests/fixtures/tls are shared with tls.rs. Each test starts its
// own listener on loopback. Connections that hold Lm stay open until the test ends.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use rustls::crypto::aws_lc_rs;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::version::{TLS12, TLS13};
use rustls::{ClientConfig, RootCertStore, SupportedProtocolVersion};
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use tokio_rustls::{TlsAcceptor, TlsConnector};
use vnc_relay_adapters::management_http::ManagementListener;
use vnc_relay_adapters::tls;
use vnc_relay_app::events::{Event, RejectReason, Stage};
use vnc_relay_app::ports::EventSink;
use vnc_relay_domain::budget::ConnectionBudget;
use vnc_relay_domain::peer_ip::PeerIp;

/// Upper bound for one exchange. A hang fails the test instead of blocking it.
const LIMIT: Duration = Duration::from_secs(10);

/// Lm for tests that send several requests in a row. Each request uses a new
/// connection, so a small limit could still be held by a closing one.
const ROOMY_LM: u32 = 8;

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

/// Starts a management listener on loopback with the given Lm.
async fn start(tls: Option<TlsAcceptor>, admin_connections: u32) -> Running {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap_or_else(|error| panic!("loopback bind: {error}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|error| panic!("loopback address: {error}"));
    let events = Arc::new(Recorder::default());
    let log: Arc<dyn EventSink> = events.clone();
    let budget = ConnectionBudget::new(admin_connections);
    tokio::spawn(ManagementListener::new(listener, tls, budget).run(log));
    Running { addr, events }
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
async fn login_paths_have_no_page_before_static_files_arrive() {
    let running = start(None, ROOMY_LM).await;

    for path in ["/login/", "/login/app.js"] {
        let reply = plain_exchange(running.addr, &get(path, &[])).await;
        assert_eq!(reply.status, 404, "GET {path}");
        assert_eq!(error_code(&reply), "not_found", "GET {path}");
    }
}
