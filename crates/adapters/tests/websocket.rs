// Contract tests of the WS and WSS viewer entry points, upgrade step only
// (docs/contracts/websocket-relay-v1.md §1.1–§1.3, §3; docs/specs/websocket.md §7).
// After 101 the relay waits for the Repeater initial exchange (TODO 2.7), so
// these tests stop at the reply to the upgrade request.
//
// Each test starts its own entry point on loopback. Connections that reached
// 101 stay open until the test ends.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use rustls::crypto::aws_lc_rs;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::version::{TLS12, TLS13};
use rustls::{ClientConfig, RootCertStore, SupportedProtocolVersion};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpSocket, TcpStream};
use tokio::sync::watch;
use tokio::time::timeout;
use tokio_rustls::{TlsAcceptor, TlsConnector};
use vnc_relay_adapters::net::AcceptPolicy;
use vnc_relay_adapters::tls;
use vnc_relay_adapters::websocket::{WsEntryPoint, WsPolicy};
use vnc_relay_app::events::{AbortReason, Event, RejectReason, Stage};
use vnc_relay_app::ports::EventSink;
use vnc_relay_app::settings::KeepaliveSettings;
use vnc_relay_domain::allow_list::{AllowList, IpItem};
use vnc_relay_domain::budget::ConnectionBudget;
use vnc_relay_domain::entry_point::EntryPoint;
use vnc_relay_domain::origin::Origin;

/// Upper bound for one exchange. A hang fails the test instead of blocking it.
const LIMIT: Duration = Duration::from_secs(10);

/// Upper bound for a refused connection to close. A refusal closes at once,
/// far below U, so this bound tells "closed at once" from "waited for U".
const PROMPT: Duration = Duration::from_secs(2);

/// U of the default configuration (config-v1 §1.2, `tls_upgrade_secs` = 10).
const UPGRADE: Duration = Duration::from_secs(10);

/// A short U for tests that wait for the deadline to end a connection.
const SHORT_UPGRADE: Duration = Duration::from_millis(300);

/// H of the default configuration; only used for the hold after 101.
const HANDSHAKE: Duration = Duration::from_secs(10);

const KEEPALIVE: KeepaliveSettings = KeepaliveSettings {
    tcp_idle: Duration::from_secs(60),
    tcp_interval: Duration::from_secs(10),
    tcp_probes: 6,
    ws_ping: Duration::from_secs(30),
    ws_pong_timeout: Duration::from_secs(30),
};

/// The RFC 6455 sample key and its accept value.
const KEY_LINE: &str = "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==";
const ACCEPT: &str = "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=";
const HOST_LINE: &str = "Host: localhost";
const UPGRADE_LINE: &str = "Upgrade: websocket";
const CONNECTION_LINE: &str = "Connection: Upgrade";
const VERSION_LINE: &str = "Sec-WebSocket-Version: 13";

const ALLOWED_ORIGIN: &str = "https://a.example:8443";

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

/// Collects every event the entry point records.
#[derive(Default)]
struct Recorder(Mutex<Vec<Event>>);

impl Recorder {
    fn lock(&self) -> MutexGuard<'_, Vec<Event>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl EventSink for Recorder {
    fn record(&self, event: &Event) {
        self.lock().push(event.clone());
    }
}

/// Waits until an event that `matches` is recorded.
async fn wait_for(events: &Recorder, matches: impl Fn(&Event) -> bool) {
    let found = async {
        loop {
            if events.lock().iter().any(&matches) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    };
    timeout(LIMIT, found)
        .await
        .unwrap_or_else(|_| panic!("the expected event is recorded within {LIMIT:?}"));
}

/// A running entry point and what it recorded.
struct Running {
    addr: SocketAddr,
    events: Arc<Recorder>,
    /// Keeps the stop channel open. Dropping the sender would read as shutdown.
    _stop: watch::Sender<bool>,
}

/// Starts a WS or WSS viewer entry point on loopback with the given policy
/// and 8 relay places.
async fn start(
    entry: EntryPoint,
    tls: Option<TlsAcceptor>,
    allow: &[&str],
    origins: &[&str],
    upgrade: Duration,
) -> Running {
    start_with_budget(entry, tls, allow, origins, upgrade, 8).await
}

/// Like `start`, with `places` relay places (L) instead of 8.
async fn start_with_budget(
    entry: EntryPoint,
    tls: Option<TlsAcceptor>,
    allow: &[&str],
    origins: &[&str],
    upgrade: Duration,
    places: u32,
) -> Running {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap_or_else(|error| panic!("loopback bind: {error}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|error| panic!("local address: {error}"));
    let events = Arc::new(Recorder::default());
    let (stop, stopped) = watch::channel(false);
    let policy = WsPolicy {
        accept: AcceptPolicy {
            allow: AllowList::new(
                allow
                    .iter()
                    .map(|text| {
                        IpItem::parse(text).unwrap_or_else(|_| panic!("allow item {text} parses"))
                    })
                    .collect(),
            ),
            relay_budget: ConnectionBudget::new(places),
            handshake: HANDSHAKE,
            keepalive: KEEPALIVE,
        },
        upgrade,
        origins: origins
            .iter()
            .map(|text| Origin::parse(text).unwrap_or_else(|_| panic!("origin {text} parses")))
            .collect(),
    };
    let log: Arc<dyn EventSink> = events.clone();
    tokio::spawn(WsEntryPoint::new(entry, listener, tls, policy).run(log, stopped));
    Running {
        addr,
        events,
        _stop: stop,
    }
}

/// The header lines of a valid upgrade request.
fn valid_headers<'a>() -> Vec<&'a str> {
    vec![
        HOST_LINE,
        UPGRADE_LINE,
        CONNECTION_LINE,
        KEY_LINE,
        VERSION_LINE,
    ]
}

/// A GET request for `path` with exactly the given header lines.
fn request(path: &str, headers: &[&str]) -> Vec<u8> {
    let mut text = format!("GET {path} HTTP/1.1\r\n");
    for line in headers {
        text.push_str(line);
        text.push_str("\r\n");
    }
    text.push_str("\r\n");
    text.into_bytes()
}

/// A GET request for `path` with the valid headers plus `extra`.
fn upgrade_request(path: &str, extra: &[&str]) -> Vec<u8> {
    let mut headers = valid_headers();
    headers.extend_from_slice(extra);
    request(path, &headers)
}

/// The status code on the first line of a reply.
fn status_of(head: &[u8]) -> u16 {
    String::from_utf8_lossy(head)
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("a status line in {:?}", String::from_utf8_lossy(head)))
}

/// Whether the reply has a header line equal to `line`, ignoring case.
fn has_line(head: &[u8], line: &str) -> bool {
    String::from_utf8_lossy(head)
        .lines()
        .any(|candidate| candidate.eq_ignore_ascii_case(line))
}

/// Reads a reply up to its blank line, or until the server closes. A stall
/// past LIMIT fails the test.
async fn read_head<S: AsyncRead + Unpin>(stream: &mut S) -> Vec<u8> {
    let mut head = Vec::new();
    let mut chunk = [0_u8; 1024];
    let reading = async {
        while !head.windows(4).any(|window| window == b"\r\n\r\n") {
            match stream.read(&mut chunk).await {
                Ok(0) | Err(_) => break,
                Ok(count) => head.extend_from_slice(&chunk[..count]),
            }
        }
    };
    timeout(LIMIT, reading)
        .await
        .unwrap_or_else(|_| panic!("the reply ends within {LIMIT:?}"));
    head
}

/// Sends a plain request and reads the reply head.
async fn plain_head(addr: SocketAddr, request: &[u8]) -> Vec<u8> {
    let mut stream = TcpStream::connect(addr)
        .await
        .unwrap_or_else(|error| panic!("connect: {error}"));
    stream
        .write_all(request)
        .await
        .unwrap_or_else(|error| panic!("request is sent: {error}"));
    read_head(&mut stream).await
}

/// Sends a request over TLS and reads the reply head.
async fn tls_head(addr: SocketAddr, request: &[u8]) -> Vec<u8> {
    let tcp = TcpStream::connect(addr)
        .await
        .unwrap_or_else(|error| panic!("connect: {error}"));
    let name =
        ServerName::try_from("localhost").unwrap_or_else(|error| panic!("server name: {error}"));
    let mut stream = connector(&[&TLS12, &TLS13])
        .connect(name, tcp)
        .await
        .unwrap_or_else(|error| panic!("TLS handshake: {error}"));
    stream
        .write_all(request)
        .await
        .unwrap_or_else(|error| panic!("request is sent: {error}"));
    read_head(&mut stream).await
}

/// Sends `request` and returns what the server sends before it closes. The
/// server may close before the request arrives; the failed write is part of
/// the refusal.
async fn refused(addr: SocketAddr, request: &[u8]) -> Vec<u8> {
    let mut stream = TcpStream::connect(addr)
        .await
        .unwrap_or_else(|error| panic!("connect: {error}"));
    let _ = stream.write_all(request).await;
    let mut reply = Vec::new();
    let closed = timeout(PROMPT, stream.read_to_end(&mut reply)).await;
    assert!(
        closed.is_ok(),
        "the refused connection closes within {PROMPT:?}"
    );
    reply
}

/// Opens a loopback connection whose client address is `source`. Every
/// 127.0.0.0/8 address is local, so the server sees `source` as the peer.
async fn connect_from(source: [u8; 4], addr: SocketAddr) -> TcpStream {
    let socket = TcpSocket::new_v4().unwrap_or_else(|error| panic!("socket: {error}"));
    socket
        .bind(SocketAddr::from((source, 0)))
        .unwrap_or_else(|error| panic!("bind {source:?}: {error}"));
    socket
        .connect(addr)
        .await
        .unwrap_or_else(|error| panic!("connect from {source:?}: {error}"))
}

/// A plain request whose Origin header has the given value.
async fn with_origin(addr: SocketAddr, origin: &str) -> Vec<u8> {
    let line = format!("Origin: {origin}");
    plain_head(addr, &upgrade_request("/relay", &[line.as_str()])).await
}

/// spec: websocket.md:WS-021 partial
#[tokio::test]
async fn allowed_client_without_origin_or_session_gets_switching_protocols() {
    let running = start(
        EntryPoint::WsViewer,
        None,
        &["127.0.0.1/32"],
        &[ALLOWED_ORIGIN],
        UPGRADE,
    )
    .await;
    let head = plain_head(
        running.addr,
        &upgrade_request("/relay", &["Sec-WebSocket-Protocol: binary"]),
    )
    .await;
    assert_eq!(status_of(&head), 101);
    assert!(has_line(&head, &format!("Sec-WebSocket-Accept: {ACCEPT}")));
    assert!(has_line(&head, "Sec-WebSocket-Protocol: binary"));
}

/// spec: websocket.md:WS-027
#[tokio::test]
async fn origin_must_equal_the_allowed_value() {
    let running = start(
        EntryPoint::WsViewer,
        None,
        &["127.0.0.1/32"],
        &[ALLOWED_ORIGIN],
        UPGRADE,
    )
    .await;
    let same = with_origin(running.addr, ALLOWED_ORIGIN).await;
    assert_eq!(status_of(&same), 101, "the same origin upgrades");
    for origin in [
        "http://a.example:8443",
        "https://a.example:9443",
        "https://x.a.example:8443",
    ] {
        let head = with_origin(running.addr, origin).await;
        assert_eq!(status_of(&head), 403, "origin {origin} is refused");
    }
}

/// spec: websocket.md:WS-015
#[tokio::test]
async fn allowed_source_with_disallowed_origin_is_refused_with_403() {
    let running = start(
        EntryPoint::WsViewer,
        None,
        &["127.0.0.1/32"],
        &[ALLOWED_ORIGIN],
        UPGRADE,
    )
    .await;
    let head = with_origin(running.addr, "https://evil.example").await;
    assert_eq!(status_of(&head), 403);
    wait_for(&running.events, |event| {
        matches!(
            event,
            Event::UpgradeRejected {
                reason: RejectReason::OriginNotAllowed,
                origin: Some(origin),
                ..
            } if origin.as_str() == "https://evil.example"
        )
    })
    .await;

    // An empty origin list refuses every Origin header.
    let empty = start(EntryPoint::WsViewer, None, &["127.0.0.1/32"], &[], UPGRADE).await;
    let head = with_origin(empty.addr, ALLOWED_ORIGIN).await;
    assert_eq!(
        status_of(&head),
        403,
        "an empty origin list refuses an Origin"
    );
}

/// A TLS request whose Origin header has the given value.
async fn tls_with_origin(addr: SocketAddr, origin: &str) -> Vec<u8> {
    let line = format!("Origin: {origin}");
    tls_head(addr, &upgrade_request("/relay", &[line.as_str()])).await
}

/// spec: websocket.md:WS-027
#[tokio::test]
async fn origin_must_equal_the_allowed_value_on_wss() {
    let running = start(
        EntryPoint::WssViewer,
        Some(tls_acceptor()),
        &["127.0.0.1/32"],
        &[ALLOWED_ORIGIN],
        UPGRADE,
    )
    .await;
    let same = tls_with_origin(running.addr, ALLOWED_ORIGIN).await;
    assert_eq!(status_of(&same), 101, "the same origin upgrades");
    for origin in [
        "http://a.example:8443",
        "https://a.example:9443",
        "https://x.a.example:8443",
    ] {
        let head = tls_with_origin(running.addr, origin).await;
        assert_eq!(status_of(&head), 403, "origin {origin} is refused");
    }
}

/// spec: websocket.md:WS-015
#[tokio::test]
async fn allowed_source_with_disallowed_origin_is_refused_with_403_on_wss() {
    let running = start(
        EntryPoint::WssViewer,
        Some(tls_acceptor()),
        &["127.0.0.1/32"],
        &[ALLOWED_ORIGIN],
        UPGRADE,
    )
    .await;
    let head = tls_with_origin(running.addr, "https://evil.example").await;
    assert_eq!(status_of(&head), 403);
    wait_for(&running.events, |event| {
        matches!(
            event,
            Event::UpgradeRejected {
                reason: RejectReason::OriginNotAllowed,
                origin: Some(origin),
                ..
            } if origin.as_str() == "https://evil.example"
        )
    })
    .await;
}

/// spec: websocket.md:WS-014
#[tokio::test]
async fn disallowed_source_is_closed_before_tls_or_http_even_with_session_hints() {
    let hints = [
        "Cookie: session=valid",
        "X-Forwarded-For: 10.0.0.1",
        "Forwarded: for=10.0.0.1",
    ];
    let bytes = upgrade_request("/relay", &hints);

    let plain = start(EntryPoint::WsViewer, None, &["10.0.0.1/32"], &[], UPGRADE).await;
    assert!(
        refused(plain.addr, &bytes).await.is_empty(),
        "a refused plain source gets no HTTP reply"
    );
    wait_for(&plain.events, |event| {
        matches!(
            event,
            Event::ConnectionRejected {
                listener: "ws_viewer",
                stage: Stage::Accept,
                reason: RejectReason::IpNotAllowed,
                ..
            }
        )
    })
    .await;

    let secure = start(
        EntryPoint::WssViewer,
        Some(tls_acceptor()),
        &["10.0.0.1/32"],
        &[],
        UPGRADE,
    )
    .await;
    assert!(
        refused(secure.addr, &bytes).await.is_empty(),
        "a refused WSS source gets no TLS record"
    );
    wait_for(&secure.events, |event| {
        matches!(
            event,
            Event::ConnectionRejected {
                listener: "wss_viewer",
                stage: Stage::Accept,
                reason: RejectReason::IpNotAllowed,
                ..
            }
        )
    })
    .await;
}

/// spec: websocket.md:WS-023
#[tokio::test]
async fn disallowed_source_on_wss_is_closed_before_tls_and_holds_no_relay_place() {
    // One relay place. A refused source that took it would leave none for
    // the allowed client below.
    let running = start_with_budget(
        EntryPoint::WssViewer,
        Some(tls_acceptor()),
        &["127.0.0.1/32"],
        &[],
        UPGRADE,
        1,
    )
    .await;
    let name =
        ServerName::try_from("localhost").unwrap_or_else(|error| panic!("server name: {error}"));

    // The refused client starts a real TLS handshake. The server must close
    // before ServerHello within PROMPT, far below U.
    let tcp = connect_from([127, 0, 0, 2], running.addr).await;
    let attempt = timeout(
        PROMPT,
        connector(&[&TLS12, &TLS13]).connect(name.clone(), tcp),
    )
    .await;
    assert!(
        matches!(attempt, Ok(Err(_))),
        "a refused source sees the close before ServerHello within {PROMPT:?}"
    );
    wait_for(&running.events, |event| {
        matches!(
            event,
            Event::ConnectionRejected {
                listener: "wss_viewer",
                stage: Stage::Accept,
                reason: RejectReason::IpNotAllowed,
                ..
            }
        )
    })
    .await;

    // The refusal left the one place free. The allowed client completes TLS
    // and the upgrade.
    let tcp = TcpStream::connect(running.addr)
        .await
        .unwrap_or_else(|error| panic!("connect: {error}"));
    let mut stream = connector(&[&TLS12, &TLS13])
        .connect(name, tcp)
        .await
        .unwrap_or_else(|error| panic!("allowed client gets a TLS session: {error}"));
    stream
        .write_all(&upgrade_request("/relay", &[]))
        .await
        .unwrap_or_else(|error| panic!("request is sent: {error}"));
    let head = read_head(&mut stream).await;
    assert_eq!(status_of(&head), 101);
}

/// spec: operations.md:OPS-037 partial
#[tokio::test]
async fn ws_and_wss_check_only_their_own_allow_lists() {
    let ws = start(EntryPoint::WsViewer, None, &["127.0.0.2/32"], &[], UPGRADE).await;
    let wss = start(
        EntryPoint::WssViewer,
        Some(tls_acceptor()),
        &["127.0.0.3/32"],
        &[],
        UPGRADE,
    )
    .await;
    let request = upgrade_request("/relay", &[]);
    let name =
        ServerName::try_from("localhost").unwrap_or_else(|error| panic!("server name: {error}"));

    // Each address is allowed on one entry point only. The server decides at
    // accept, so the refused WS client gets no HTTP reply.
    let mut plain = connect_from([127, 0, 0, 3], ws.addr).await;
    let _ = plain.write_all(&request).await;
    let mut reply = Vec::new();
    let closed = timeout(PROMPT, plain.read_to_end(&mut reply)).await;
    assert!(
        closed.is_ok(),
        "ws_viewer closes 127.0.0.3 within {PROMPT:?}"
    );
    assert!(reply.is_empty(), "ws_viewer gives 127.0.0.3 no HTTP reply");
    wait_for(&ws.events, |event| {
        matches!(
            event,
            Event::ConnectionRejected {
                listener: "ws_viewer",
                stage: Stage::Accept,
                reason: RejectReason::IpNotAllowed,
                ..
            }
        )
    })
    .await;

    let tcp = connect_from([127, 0, 0, 2], wss.addr).await;
    let attempt = timeout(
        PROMPT,
        connector(&[&TLS12, &TLS13]).connect(name.clone(), tcp),
    )
    .await;
    assert!(
        matches!(attempt, Ok(Err(_))),
        "wss_viewer closes 127.0.0.2 before ServerHello within {PROMPT:?}"
    );
    wait_for(&wss.events, |event| {
        matches!(
            event,
            Event::ConnectionRejected {
                listener: "wss_viewer",
                stage: Stage::Accept,
                reason: RejectReason::IpNotAllowed,
                ..
            }
        )
    })
    .await;

    // Each address gets the upgrade on its own entry point.
    let mut plain = connect_from([127, 0, 0, 2], ws.addr).await;
    plain
        .write_all(&request)
        .await
        .unwrap_or_else(|error| panic!("request is sent: {error}"));
    let head = read_head(&mut plain).await;
    assert_eq!(status_of(&head), 101);

    let tcp = connect_from([127, 0, 0, 3], wss.addr).await;
    let mut secure = connector(&[&TLS12, &TLS13])
        .connect(name, tcp)
        .await
        .unwrap_or_else(|error| panic!("TLS handshake: {error}"));
    secure
        .write_all(&request)
        .await
        .unwrap_or_else(|error| panic!("request is sent: {error}"));
    let head = read_head(&mut secure).await;
    assert_eq!(status_of(&head), 101);
}

/// spec: websocket.md:WS-012 partial
#[tokio::test]
async fn management_paths_and_malformed_upgrades_are_refused_without_upgrade() {
    let running = start(
        EntryPoint::WsViewer,
        None,
        &["127.0.0.1/32"],
        &[ALLOWED_ORIGIN],
        UPGRADE,
    )
    .await;
    let without_key = [HOST_LINE, UPGRADE_LINE, CONNECTION_LINE, VERSION_LINE];
    let two_origins = [
        HOST_LINE,
        UPGRADE_LINE,
        CONNECTION_LINE,
        KEY_LINE,
        VERSION_LINE,
        "Origin: https://a.example:8443",
        "Origin: https://a.example:8443",
    ];
    let post = format!(
        "POST /relay HTTP/1.1\r\n{}\r\n\r\n",
        valid_headers().join("\r\n")
    );
    let wrong_version = request(
        "/relay",
        &[
            HOST_LINE,
            UPGRADE_LINE,
            CONNECTION_LINE,
            KEY_LINE,
            "Sec-WebSocket-Version: 8",
        ],
    );
    let cases: [(Vec<u8>, u16); 4] = [
        (upgrade_request("/api/v1/session", &[]), 404),
        (upgrade_request("/relay/", &[]), 404),
        (request("/relay", &without_key), 400),
        (request("/relay", &two_origins), 400),
    ];
    for (index, (bytes, expected)) in cases.into_iter().enumerate() {
        let head = plain_head(running.addr, &bytes).await;
        assert_eq!(status_of(&head), expected, "case {index}");
    }

    let method = plain_head(running.addr, post.as_bytes()).await;
    assert_eq!(status_of(&method), 405, "POST is not allowed");
    assert!(
        has_line(&method, "Allow: GET"),
        "405 names the allowed method"
    );

    let version = plain_head(running.addr, &wrong_version).await;
    assert_eq!(status_of(&version), 426, "version 8 is not supported");
    assert!(
        has_line(&version, VERSION_LINE),
        "426 names the supported version"
    );
}

/// Unmarked: a subprotocol other than `binary` is refused (websocket-relay-v1 §1.2).
#[tokio::test]
async fn subprotocol_without_binary_is_refused_with_400() {
    let running = start(EntryPoint::WsViewer, None, &["127.0.0.1/32"], &[], UPGRADE).await;
    let head = plain_head(
        running.addr,
        &upgrade_request("/relay", &["Sec-WebSocket-Protocol: chat"]),
    )
    .await;
    assert_eq!(status_of(&head), 400);
}

/// Unmarked: an allowed client upgrades over TLS on the WSS listener.
#[tokio::test]
async fn wss_allowed_client_gets_switching_protocols_after_tls() {
    let running = start(
        EntryPoint::WssViewer,
        Some(tls_acceptor()),
        &["127.0.0.1/32"],
        &[],
        UPGRADE,
    )
    .await;
    let head = tls_head(running.addr, &upgrade_request("/relay", &[])).await;
    assert_eq!(status_of(&head), 101);
    assert!(has_line(&head, &format!("Sec-WebSocket-Accept: {ACCEPT}")));
}

/// spec: operations.md:OPS-006 partial
#[tokio::test]
async fn stalled_upgrade_request_is_closed_at_u_without_a_reply() {
    let running = start(
        EntryPoint::WsViewer,
        None,
        &["127.0.0.1/32"],
        &[],
        SHORT_UPGRADE,
    )
    .await;
    let mut stream = TcpStream::connect(running.addr)
        .await
        .unwrap_or_else(|error| panic!("connect: {error}"));
    // The head never ends: no blank line arrives.
    stream
        .write_all(b"GET /relay HTTP/1.1\r\nHost: localhost\r\n")
        .await
        .unwrap_or_else(|error| panic!("partial head is sent: {error}"));
    let mut reply = Vec::new();
    let closed = timeout(LIMIT, stream.read_to_end(&mut reply)).await;
    assert!(
        closed.is_ok(),
        "the server closes the stalled upgrade within {LIMIT:?}"
    );
    assert!(reply.is_empty(), "no HTTP reply before the upgrade");
    wait_for(&running.events, |event| {
        matches!(
            event,
            Event::ConnectionAborted {
                reason: AbortReason::TlsUpgradeTimeout,
                stage: Stage::Upgrade,
                ..
            }
        )
    })
    .await;
}

/// spec: operations.md:OPS-006 partial
#[tokio::test]
async fn stalled_tls_handshake_on_wss_is_closed_at_u_without_a_reply() {
    let running = start(
        EntryPoint::WssViewer,
        Some(tls_acceptor()),
        &["127.0.0.1/32"],
        &[],
        SHORT_UPGRADE,
    )
    .await;
    let mut stream = TcpStream::connect(running.addr)
        .await
        .unwrap_or_else(|error| panic!("connect: {error}"));
    // No ClientHello is sent.
    let mut reply = Vec::new();
    let closed = timeout(LIMIT, stream.read_to_end(&mut reply)).await;
    assert!(
        closed.is_ok(),
        "the server closes the stalled handshake within {LIMIT:?}"
    );
    assert!(reply.is_empty(), "no TLS record before the handshake");
    wait_for(&running.events, |event| {
        matches!(
            event,
            Event::ConnectionAborted {
                reason: AbortReason::TlsUpgradeTimeout,
                stage: Stage::Tls,
                ..
            }
        )
    })
    .await;
}

/// spec: operations.md:OPS-006 partial
#[tokio::test]
async fn head_completed_after_u_gets_no_answer() {
    // The test thread blocks in `std::thread::sleep`, so the single-threaded
    // runtime cannot run the server task. The server reads the head only after
    // U has passed. Neither the 101 nor a refusal may be sent.
    for path in ["/relay", "/api/v1/session"] {
        let running = start(
            EntryPoint::WsViewer,
            None,
            &["127.0.0.1/32"],
            &[ALLOWED_ORIGIN],
            SHORT_UPGRADE,
        )
        .await;
        let mut stream = TcpStream::connect(running.addr)
            .await
            .unwrap_or_else(|error| panic!("connect: {error}"));
        wait_for(&running.events, |event| {
            matches!(event, Event::ConnectionAccepted { .. })
        })
        .await;
        stream
            .write_all(&upgrade_request(path, &[]))
            .await
            .unwrap_or_else(|error| panic!("request is sent: {error}"));
        std::thread::sleep(SHORT_UPGRADE + Duration::from_millis(200));
        let reply = read_head(&mut stream).await;
        assert!(
            reply.is_empty(),
            "{path}: no reply once U has passed, got {:?}",
            String::from_utf8_lossy(&reply)
        );
        wait_for(&running.events, |event| {
            matches!(
                event,
                Event::ConnectionAborted {
                    reason: AbortReason::TlsUpgradeTimeout,
                    stage: Stage::Upgrade,
                    ..
                }
            )
        })
        .await;
    }
}
