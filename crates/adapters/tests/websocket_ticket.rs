//! Real WS/WSS access-ticket upgrades (websocket-relay-v2 §1.9).

use std::net::SocketAddr;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use aws_lc_rs::signature::ECDSA_P256_SHA256_ASN1_SIGNING;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use serde_json::{Value, json};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::time::timeout;
use tokio_rustls::{TlsAcceptor, TlsConnector};
use vnc_relay_adapters::crypto::{RelayIdKey, SessionIdKey};
use vnc_relay_adapters::net::{AcceptPolicy, AllowSource};
use vnc_relay_adapters::ops_log::format_line;
use vnc_relay_adapters::target::TcpTargetConnector;
use vnc_relay_adapters::tls::{self, TlsSource};
use vnc_relay_adapters::websocket::{WsEntryPoint, WsPolicy};
use vnc_relay_app::events::{Event, RejectReason};
use vnc_relay_app::ports::{Clock, EventSink};
use vnc_relay_app::relay::{RelayRuntime, RelayRuntimeConfig, RelayService};
use vnc_relay_app::reload::ConfigCandidate;
use vnc_relay_app::reload::ports::RuntimeSettingsSource;
use vnc_relay_app::reload::snapshot::{ReloadCell, ReloadSnapshot};
use vnc_relay_app::settings::{Settings, TicketRoleSettings};
use vnc_relay_domain::allow_list::{AllowList, IpItem};
use vnc_relay_domain::budget::ConnectionBudget;
use vnc_relay_domain::entry_point::{Listener, Role};
use vnc_relay_domain::handshake::HandshakePolicy;
use vnc_relay_domain::ticket::{TicketError, TicketUseTable};

#[path = "common/ticket.rs"]
mod ticket;
use ticket::Signer;

const LIMIT: Duration = Duration::from_secs(10);
const NOW: u64 = 1_000;

fn claims(role: Role, jti: &str) -> Value {
    json!({"iss":format!("issuer.example/{}",role.as_str()),"aud":format!("vnc-relay/{}",role.as_str()),"rh":"relay.example","rid":"0123","jti":jti,"iat":NOW,"exp":NOW+60})
}

fn role_settings(role: Role, signer: &Signer) -> TicketRoleSettings {
    TicketRoleSettings {
        enabled: true,
        issuer: Some(format!("issuer.example/{}", role.as_str())),
        relay_host: Some("relay.example".into()),
        public_keys_file: Some("keys.json".into()),
        max_lifetime: Duration::from_secs(60),
        keys: signer.keys.clone(),
    }
}

struct TestClock {
    wall: AtomicU64,
    start: tokio::time::Instant,
}

impl Clock for TestClock {
    fn monotonic(&self) -> Duration {
        self.start.elapsed()
    }
    fn wall(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(self.wall.load(Ordering::SeqCst))
    }
}

#[derive(Default)]
struct Recorder(Mutex<Vec<Event>>);

#[expect(
    clippy::unwrap_used,
    reason = "test recorder: a poisoned lock fails the test"
)]
impl Recorder {
    fn events(&self) -> Vec<Event> {
        self.0.lock().unwrap().clone()
    }
}

#[expect(
    clippy::unwrap_used,
    reason = "test recorder: a poisoned lock fails the test"
)]
impl EventSink for Recorder {
    fn record(&self, event: &Event) {
        self.0.lock().unwrap().push(event.clone());
    }
}

struct Running {
    addr: SocketAddr,
    wss_addr: SocketAddr,
    events: Arc<Recorder>,
    cell: Arc<ReloadCell<TlsAcceptor>>,
    clock: Arc<TestClock>,
    uses: Arc<Mutex<TicketUseTable>>,
    stop: watch::Sender<bool>,
    runtime: Arc<RelayRuntime<TcpTargetConnector>>,
    service: Arc<RelayService>,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.service.request_shutdown(&self.runtime, Duration::ZERO);
        self.stop.send_replace(true);
    }
}

#[expect(
    clippy::unwrap_used,
    reason = "test fixture: constant addresses always parse"
)]
fn allow(text: &str) -> AllowList {
    AllowList::new(vec![IpItem::parse(text).unwrap()])
}

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/tls")
        .join(name)
}

#[expect(
    clippy::unwrap_used,
    reason = "test fixture: a failed setup or I/O step fails the test"
)]
impl Running {
    async fn start(viewer: Option<&Signer>, server: Option<&Signer>, started: u64) -> Self {
        let mut settings = vnc_relay_adapters::config_file::parse(
            "[admin]\naccounts_file='accounts.toml'\n[management]\nlisten='127.0.0.1:8443'\n[status]\nlisten='127.0.0.1:9090'\n", Path::new("."),
        ).unwrap();
        for listener in [&mut settings.relay.ws, &mut settings.relay.wss] {
            listener.viewer.enabled = true;
            listener.server.enabled = true;
            listener.viewer.allow = Some(allow("127.0.0.1"));
            listener.server.allow = Some(allow("127.0.0.1"));
        }
        if let Some(signer) = viewer {
            settings.relay.ticket.viewer = role_settings(Role::Viewer, signer);
        }
        if let Some(signer) = server {
            settings.relay.ticket.server = role_settings(Role::Server, signer);
        }
        settings.limits.relay_connections = 128;
        settings.limits.waiting = 64;
        settings.timeouts.close = Duration::from_millis(50);
        let tls = tls::load_acceptor(
            &fixture("server-chain.pem"),
            &fixture("server-key-p256.pem"),
        )
        .unwrap();
        let cell = Arc::new(ReloadCell::new(ReloadSnapshot::new(
            ConfigCandidate {
                settings,
                presence: Default::default(),
            },
            Some(tls),
            None,
        )));
        let source: Arc<dyn RuntimeSettingsSource> = cell.clone();
        let budget = ConnectionBudget::new();
        let runtime = Arc::new(RelayRuntime::new(
            TcpTargetConnector::new(source.clone()),
            Arc::new(RelayIdKey::generate().unwrap()),
            Arc::new(SessionIdKey::generate().unwrap()),
            budget.clone(),
            ConnectionBudget::new(),
            RelayRuntimeConfig { recent_closed: 16 },
            source.clone(),
        ));
        let clock = Arc::new(TestClock {
            wall: AtomicU64::new(NOW),
            start: tokio::time::Instant::now(),
        });
        let events = Arc::new(Recorder::default());
        let service = Arc::new(RelayService::new(
            HandshakePolicy {
                mode1_enabled: true,
                mode2_enabled: true,
            },
            source,
            clock.clone(),
            events.clone(),
        ));
        let uses = Arc::new(Mutex::new(TicketUseTable::new()));
        let (stop, stopped) = watch::channel(false);
        let ws = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let wss = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = ws.local_addr().unwrap();
        let wss_addr = wss.local_addr().unwrap();
        for (listener, socket) in [(Listener::Ws, ws), (Listener::Wss, wss)] {
            let policy = WsPolicy {
                accept: AcceptPolicy {
                    allow: if listener == Listener::Ws {
                        AllowSource::ws(cell.clone())
                    } else {
                        AllowSource::wss(cell.clone())
                    },
                    relay_budget: budget.clone(),
                },
                ticket_uses: uses.clone(),
                started_at: UNIX_EPOCH + Duration::from_secs(started),
            };
            tokio::spawn(
                WsEntryPoint::new(
                    listener,
                    socket,
                    (listener == Listener::Wss).then(|| TlsSource::wss(cell.clone())),
                    policy,
                    service.clone(),
                    runtime.clone(),
                    clock.clone(),
                )
                .run(events.clone(), stopped.clone()),
            );
        }
        Self {
            addr,
            wss_addr,
            events,
            cell,
            clock,
            uses,
            stop,
            runtime,
            service,
        }
    }

    fn replace(&self, change: impl FnOnce(&mut Settings)) {
        let previous = self.cell.current();
        let mut settings = (*previous.settings).clone();
        change(&mut settings);
        self.cell.replace(ReloadSnapshot::new(
            ConfigCandidate {
                settings,
                presence: Default::default(),
            },
            previous.wss_tls.clone(),
            None,
        ));
    }

    async fn connect(&self, tls: bool) -> Box<dyn Io> {
        let stream = TcpStream::connect(if tls { self.wss_addr } else { self.addr })
            .await
            .unwrap();
        if !tls {
            return Box::new(stream);
        }
        let mut roots = rustls::RootCertStore::empty();
        for cert in CertificateDer::pem_file_iter(fixture("root-ca.pem")).unwrap() {
            roots.add(cert.unwrap()).unwrap();
        }
        let config = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::aws_lc_rs::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(roots)
        .with_no_client_auth();
        Box::new(
            TlsConnector::from(Arc::new(config))
                .connect(ServerName::try_from("localhost").unwrap(), stream)
                .await
                .unwrap(),
        )
    }

    async fn request(
        &self,
        tls: bool,
        role: Role,
        protocols: Option<&str>,
        extra: &str,
    ) -> (String, Box<dyn Io>) {
        let mut stream = self.connect(tls).await;
        let protocol = protocols
            .map(|value| format!("Sec-WebSocket-Protocol: {value}\r\n"))
            .unwrap_or_default();
        let request = format!(
            "GET /relay/{} HTTP/1.1\r\nHost: ignored.example\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n{protocol}{extra}\r\n",
            role.as_str()
        );
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut head = Vec::new();
        timeout(LIMIT, async {
            while !head.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                if stream.read(&mut byte).await.unwrap() == 0 {
                    break;
                }
                head.push(byte[0]);
            }
        })
        .await
        .unwrap();
        (String::from_utf8(head).unwrap(), stream)
    }

    async fn ticket(&self, tls: bool, role: Role, ticket: &str) -> (String, Box<dyn Io>) {
        self.request(tls, role, Some(&format!("binary, ticket.{ticket}")), "")
            .await
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "each refusal case names its request and expected log"
    )]
    async fn rejected(
        &self,
        tls: bool,
        role: Role,
        protocols: Option<&str>,
        extra: &str,
        status: u16,
        reason: RejectReason,
        jti: Option<&str>,
    ) {
        let (head, mut stream) = self.request(tls, role, protocols, extra).await;
        assert_status(&head, status);
        assert!(head.contains("Content-Length: 0\r\n"));
        assert_eq!(
            head.contains("WWW-Authenticate: VncRelayTicket\r\n"),
            status == 401
        );
        let mut remainder = Vec::new();
        timeout(LIMIT, stream.read_to_end(&mut remainder))
            .await
            .unwrap()
            .unwrap();
        assert!(remainder.is_empty());
        let event = self
            .events
            .events()
            .into_iter()
            .rev()
            .find(|event| matches!(event, Event::UpgradeRejected { .. }))
            .unwrap();
        match event {
            Event::UpgradeRejected {
                reason: actual,
                role: actual_role,
                ticket_jti,
                ..
            } => {
                assert_eq!(actual, reason);
                assert_eq!(actual_role, Some(role));
                assert_eq!(ticket_jti.as_deref(), jti);
            }
            _ => unreachable!(),
        }
    }
}

trait Io: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Io for T {}

fn assert_status(head: &str, status: u16) {
    assert!(head.starts_with(&format!("HTTP/1.1 {status} ")), "{head}");
}

#[expect(
    clippy::unwrap_used,
    reason = "test client: a failed read fails the test"
)]
async fn frame(stream: &mut Box<dyn Io>) -> (u8, Vec<u8>) {
    timeout(LIMIT, async {
        let mut header = [0; 2];
        stream.read_exact(&mut header).await.unwrap();
        assert!(header[1] < 126);
        let mut payload = vec![0; usize::from(header[1])];
        stream.read_exact(&mut payload).await.unwrap();
        (header[0] & 15, payload)
    })
    .await
    .unwrap()
}

#[expect(
    clippy::unwrap_used,
    reason = "test client: a failed write fails the test"
)]
async fn send(stream: &mut Box<dyn Io>, data: &[u8]) {
    assert!(data.len() < 126);
    let mut message = vec![0x82, 0x80 | data.len() as u8, 1, 2, 3, 4];
    message.extend(
        data.iter()
            .enumerate()
            .map(|(i, byte)| byte ^ [1, 2, 3, 4][i % 4]),
    );
    stream.write_all(&message).await.unwrap();
}

async fn assert_data(stream: &mut Box<dyn Io>, expected: &[u8]) {
    let mut received = Vec::new();
    while received.len() < expected.len() {
        let (opcode, payload) = frame(stream).await;
        assert_eq!(opcode, 2);
        received.extend_from_slice(&payload);
    }
    assert_eq!(received, expected);
}

async fn initial_request(stream: &mut Box<dyn Io>, request: &str) {
    let mut padded = [0; 250];
    padded[..request.len()].copy_from_slice(request.as_bytes());
    for chunk in padded.chunks(125) {
        send(stream, chunk).await;
    }
}

#[expect(
    clippy::unwrap_used,
    reason = "test helper: a missed event fails the test"
)]
async fn wait_for(running: &Running, predicate: impl Fn(&Event) -> bool) {
    timeout(LIMIT, async {
        while !running.events.events().iter().any(&predicate) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

fn assert_no_registration(running: &Running) {
    let snapshot = running.runtime.snapshot(running.clock.as_ref());
    assert!(snapshot.waiting.is_empty());
    assert!(snapshot.active.is_empty());
    assert_eq!(snapshot.target_connecting, 0);
    assert!(
        !running
            .events
            .events()
            .iter()
            .any(|event| matches!(event, Event::TicketAccepted { .. }))
    );
}

/// spec: websocket.md:WS-037
#[tokio::test]
async fn disabled_ticket_roles_keep_the_previous_subprotocol_behavior() {
    let running = Running::start(None, None, 900).await;
    for tls in [false, true] {
        for protocols in [Some("binary, ticket.not-a-jws"), None, Some("binary")] {
            let (head, _) = running.request(tls, Role::Viewer, protocols, "").await;
            assert_status(&head, 101);
            assert_eq!(
                head.contains("Sec-WebSocket-Protocol: binary\r\n"),
                protocols.is_some()
            );
            assert!(!head.contains("ticket."));
        }
    }
    assert!(running.uses.lock().unwrap().is_empty());
    assert!(
        !running
            .events
            .events()
            .iter()
            .any(|event| matches!(event, Event::TicketAccepted { .. }))
    );
}

/// spec: websocket.md:WS-038 partial
#[tokio::test]
async fn valid_viewer_tickets_match_over_ws_and_wss_without_exposing_ticket_text() {
    // This fixture has no authenticated management HTTP listener; API secrecy is
    // checked separately by the management-contract owner.
    let signer = Signer::new();
    let running = Running::start(Some(&signer), None, 900).await;
    for tls in [false, true] {
        let jti = if tls { "valid-wss" } else { "valid-ws" };
        let mut payload = claims(Role::Viewer, jti);
        payload["sub"] = json!("session-1");
        payload["sx"] = json!(NOW + 60);
        let ticket = signer.sign(&payload);
        let (head, mut viewer) = running.ticket(tls, Role::Viewer, &ticket).await;
        assert_status(&head, 101);
        assert!(head.contains("Sec-WebSocket-Protocol: binary\r\n"));
        assert!(!head.contains(&ticket));
        assert_eq!(frame(&mut viewer).await, (2, b"RFB 000.000\n".to_vec()));
        let (head, mut server) = running.request(tls, Role::Server, Some("binary"), "").await;
        assert_status(&head, 101);
        initial_request(&mut viewer, "ID:0123").await;
        initial_request(&mut server, "ID:0123").await;
        send(&mut server, b"server-data").await;
        assert_data(&mut viewer, b"server-data").await;
        send(&mut viewer, b"viewer-data").await;
        assert_data(&mut server, b"viewer-data").await;
        let events = running.events.events();
        let accepted = events.iter().find(|event| matches!(event, Event::TicketAccepted { ticket_jti, .. } if ticket_jti == jti)).unwrap();
        match accepted {
            Event::TicketAccepted {
                listener,
                role,
                ticket_sub,
                session_expires_at,
                ..
            } => {
                assert_eq!(*listener, if tls { Listener::Wss } else { Listener::Ws });
                assert_eq!(*role, Role::Viewer);
                assert_eq!(ticket_sub.as_deref(), Some("session-1"));
                assert_eq!(
                    *session_expires_at,
                    Some(UNIX_EPOCH + Duration::from_secs(NOW + 60))
                );
            }
            _ => unreachable!(),
        }
        for event in events {
            assert!(!format_line(&event, UNIX_EPOCH).contains(&ticket));
        }
        // Avoid reusing the relay id before the previous pair has finished.
        drop(viewer);
        drop(server);
        timeout(LIMIT, async {
            while !running
                .runtime
                .snapshot(running.clock.as_ref())
                .active
                .is_empty()
            {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }
}

/// spec: websocket.md:WS-039
#[tokio::test]
async fn enabled_roles_require_ticket_and_binary_in_contract_order() {
    let signer = Signer::new();
    let running = Running::start(Some(&signer), None, 900).await;
    for tls in [false, true] {
        for protocols in [None, Some("binary")] {
            running
                .rejected(
                    tls,
                    Role::Viewer,
                    protocols,
                    "",
                    401,
                    RejectReason::Ticket(TicketError::Missing),
                    None,
                )
                .await;
        }
        running
            .rejected(
                tls,
                Role::Viewer,
                Some("ticket.invalid"),
                "",
                400,
                RejectReason::SubprotocolNotSupported,
                None,
            )
            .await;
    }
    assert!(running.uses.lock().unwrap().is_empty());
    assert_no_registration(&running);
}

/// spec: websocket.md:WS-040
#[tokio::test]
async fn invalid_ticket_forms_headers_signatures_and_claims_are_unauthorized() {
    let signer = Signer::new();
    let wrong = Signer::new();
    let der = Signer::with_algorithm(&ECDSA_P256_SHA256_ASN1_SIGNING);
    let running = Running::start(Some(&signer), None, 900).await;
    let valid = claims(Role::Viewer, "invalid-case");
    let mut cases: Vec<(String, Option<&str>)> = vec![
        ("a.b".into(), None),
        ("e30=.e30.AA".into(), None),
        ("e31.e30.AA".into(), None),
        (wrong.sign(&valid), None),
    ];
    for header in [
        r#"{"alg":"none","kid":"key"}"#,
        r#"{"alg":"ES256"}"#,
        r#"{"alg":"ES256","kid":""}"#,
        r#"{"alg":"ES256","kid":"unknown"}"#,
        r#"{"alg":"ES256","kid":"key","crit":[]}"#,
        r#"{"alg":"ES256","kid":"key","\u006bid":"key"}"#,
    ] {
        cases.push((signer.sign_text(header, &valid.to_string()), None));
    }
    let header = json!({"alg":"ES256","kid":"x".repeat(129)});
    cases.push((
        signer.sign_text(&header.to_string(), &valid.to_string()),
        None,
    ));
    for name in ["iss", "aud", "rh", "rid", "iat", "exp"] {
        let mut value = valid.clone();
        value.as_object_mut().unwrap().remove(name);
        cases.push((signer.sign(&value), Some("invalid-case")));
    }
    let mut value = valid.clone();
    value.as_object_mut().unwrap().remove("jti");
    cases.push((signer.sign(&value), None));
    for (name, value) in [
        ("iss", json!("other.example")),
        ("aud", json!("vnc-relay/server")),
        ("rh", json!("other.example")),
        ("aud", json!(["vnc-relay/viewer"])),
        ("rid", json!("123x")),
        ("rid", json!(123)),
        ("iat", json!(-1)),
        ("iat", json!(1000.0)),
        ("iat", json!("1000")),
        ("iat", json!(NOW + 6)),
        ("exp", json!(NOW)),
        ("exp", json!(NOW - 1)),
        ("exp", json!(NOW + 61)),
        ("exp", json!(1060.0)),
        ("sx", json!(null)),
        ("sub", json!("")),
        ("sub", json!("a".repeat(129))),
    ] {
        let mut payload = valid.clone();
        payload[name] = value;
        cases.push((signer.sign(&payload), Some("invalid-case")));
    }
    for value in [json!(""), json!("a".repeat(129)), json!(1)] {
        let mut payload = valid.clone();
        payload["jti"] = value;
        cases.push((signer.sign(&payload), None));
    }
    cases.push((
        signer.sign_text(
            r#"{"alg":"ES256","kid":"key"}"#,
            r#"{"jti":"id","jti":"id"}"#,
        ),
        None,
    ));
    for tls in [false, true] {
        running
            .rejected(
                tls,
                Role::Viewer,
                Some("binary, ticket.a, ticket.b"),
                "",
                401,
                RejectReason::Ticket(TicketError::Invalid),
                None,
            )
            .await;
        running
            .rejected(
                tls,
                Role::Viewer,
                Some("binary, ticket.a"),
                "Sec-WebSocket-Protocol: ticket.b\r\n",
                401,
                RejectReason::Ticket(TicketError::Invalid),
                None,
            )
            .await;
        for (ticket, jti) in &cases {
            running
                .rejected(
                    tls,
                    Role::Viewer,
                    Some(&format!("binary, ticket.{ticket}")),
                    "",
                    401,
                    RejectReason::Ticket(TicketError::Invalid),
                    *jti,
                )
                .await;
        }
        running.replace(|settings| settings.relay.ticket.viewer.keys = der.keys.clone());
        running
            .rejected(
                tls,
                Role::Viewer,
                Some(&format!("binary, ticket.{}", der.sign(&valid))),
                "",
                401,
                RejectReason::Ticket(TicketError::Invalid),
                None,
            )
            .await;
        running.replace(|settings| settings.relay.ticket.viewer.keys = signer.keys.clone());
    }
    assert!(running.uses.lock().unwrap().is_empty());
    assert_no_registration(&running);
}

/// spec: websocket.md:WS-041
#[tokio::test]
async fn expiry_boundaries_use_floored_wall_seconds_and_no_session_skew() {
    let signer = Signer::new();
    let running = Running::start(Some(&signer), None, 900).await;
    for tls in [false, true] {
        for (suffix, exp, sx, status) in [
            ("exp-before", NOW - 4, None, 101),
            ("exp-at", NOW - 5, None, 401),
            ("sx-after", NOW + 60, Some(NOW + 1), 101),
            ("sx-at", NOW + 60, Some(NOW), 401),
            ("sx-before", NOW + 60, Some(NOW - 1), 401),
        ] {
            let mut value = claims(Role::Viewer, &format!("{tls}-{suffix}"));
            value["iat"] = json!(exp - 60);
            value["exp"] = json!(exp);
            if let Some(sx) = sx {
                value["sx"] = json!(sx);
            }
            let ticket = signer.sign(&value);
            if status == 101 {
                assert_status(&running.ticket(tls, Role::Viewer, &ticket).await.0, 101);
            } else {
                running
                    .rejected(
                        tls,
                        Role::Viewer,
                        Some(&format!("binary, ticket.{ticket}")),
                        "",
                        401,
                        RejectReason::Ticket(TicketError::Expired),
                        value["jti"].as_str(),
                    )
                    .await;
            }
        }
    }
}

/// spec: websocket.md:WS-042
#[tokio::test]
async fn use_records_are_atomic_shared_by_ws_wss_and_expire_at_exp_plus_five() {
    let signer = Signer::new();
    let running = Running::start(Some(&signer), None, 900).await;
    let ticket = signer.sign(&claims(Role::Viewer, "once"));
    assert_status(&running.ticket(false, Role::Viewer, &ticket).await.0, 101);
    running
        .rejected(
            true,
            Role::Viewer,
            Some(&format!("binary, ticket.{ticket}")),
            "",
            401,
            RejectReason::Ticket(TicketError::Reused),
            Some("once"),
        )
        .await;
    let concurrent = signer.sign(&claims(Role::Viewer, "concurrent"));
    let (a, b) = tokio::join!(
        running.ticket(false, Role::Viewer, &concurrent),
        running.ticket(true, Role::Viewer, &concurrent)
    );
    let statuses = [
        a.0.starts_with("HTTP/1.1 101 "),
        b.0.starts_with("HTTP/1.1 101 "),
    ];
    assert_eq!(statuses.iter().filter(|status| **status).count(), 1);
    let rejected = if statuses[0] { &b.0 } else { &a.0 };
    assert_status(rejected, 401);
    assert!(rejected.contains("WWW-Authenticate: VncRelayTicket\r\n"));
    assert!(running.events.events().iter().any(|event| matches!(event, Event::UpgradeRejected { reason: RejectReason::Ticket(TicketError::Reused), ticket_jti: Some(jti), .. } if jti == "concurrent")));
    drop(a);
    drop(b);
    // The server role keeps the socket IP in the accept-time union. Viewer IP
    // refusal therefore occurs at the second, role-specific check.
    running.replace(|settings| {
        settings.relay.ws.viewer.allow = Some(allow("192.0.2.1"));
    });
    let retry = signer.sign(&claims(Role::Viewer, "retry"));
    running
        .rejected(
            false,
            Role::Viewer,
            Some(&format!("binary, ticket.{retry}")),
            "",
            403,
            RejectReason::IpNotAllowed,
            Some("retry"),
        )
        .await;
    running.replace(|settings| settings.relay.ws.viewer.allow = Some(allow("127.0.0.1")));
    assert_status(&running.ticket(false, Role::Viewer, &retry).await.0, 101);
    assert_eq!(running.uses.lock().unwrap().len(), 3);
    running.clock.wall.store(NOW + 64, Ordering::SeqCst);
    let mut before = claims(Role::Viewer, "once");
    before["iat"] = json!(NOW + 64);
    before["exp"] = json!(NOW + 124);
    running
        .rejected(
            false,
            Role::Viewer,
            Some(&format!("binary, ticket.{}", signer.sign(&before))),
            "",
            401,
            RejectReason::Ticket(TicketError::Reused),
            Some("once"),
        )
        .await;
    running.clock.wall.store(NOW + 65, Ordering::SeqCst);
    running
        .rejected(
            false,
            Role::Viewer,
            Some(&format!("binary, ticket.{ticket}")),
            "",
            401,
            RejectReason::Ticket(TicketError::Expired),
            Some("once"),
        )
        .await;
    assert_status(
        &running
            .ticket(false, Role::Viewer, &signer.sign(&before))
            .await
            .0,
        101,
    );
    assert_eq!(running.uses.lock().unwrap().len(), 1);
}

/// spec: websocket.md:WS-043
#[tokio::test]
async fn restart_rejects_tickets_issued_at_or_before_start_plus_five() {
    let signer = Signer::new();
    let running = Running::start(Some(&signer), None, 990).await;
    for tls in [false, true] {
        for iat in [994, 995, 996] {
            let mut value = claims(Role::Viewer, &format!("{tls}-{iat}"));
            value["iat"] = json!(iat);
            value["exp"] = json!(iat + 60);
            let ticket = signer.sign(&value);
            if iat > 995 {
                assert_status(&running.ticket(tls, Role::Viewer, &ticket).await.0, 101);
            } else {
                running
                    .rejected(
                        tls,
                        Role::Viewer,
                        Some(&format!("binary, ticket.{ticket}")),
                        "",
                        401,
                        RejectReason::Ticket(TicketError::IssuedBeforeStart),
                        value["jti"].as_str(),
                    )
                    .await;
            }
        }
    }
}

/// spec: websocket.md:WS-044
#[tokio::test]
async fn origin_then_subprotocol_then_ticket_then_ip_decide_without_consuming_ticket() {
    let signer = Signer::new();
    let running = Running::start(Some(&signer), None, 900).await;
    running.replace(|settings| {
        settings.relay.ws.viewer.allow = Some(allow("192.0.2.1"));
        settings.relay.wss.viewer.allow = Some(allow("192.0.2.1"));
    });
    for tls in [false, true] {
        let used_before = running.uses.lock().unwrap().len();
        let ticket = signer.sign(&claims(Role::Viewer, &format!("order-{tls}")));
        running
            .rejected(
                tls,
                Role::Viewer,
                Some("ticket.invalid"),
                "Origin: https://denied.example\r\n",
                403,
                RejectReason::OriginNotAllowed,
                None,
            )
            .await;
        running
            .rejected(
                tls,
                Role::Viewer,
                Some("ticket.invalid"),
                "",
                400,
                RejectReason::SubprotocolNotSupported,
                None,
            )
            .await;
        running
            .rejected(
                tls,
                Role::Viewer,
                Some("binary, ticket.invalid"),
                "",
                401,
                RejectReason::Ticket(TicketError::Invalid),
                None,
            )
            .await;
        running
            .rejected(
                tls,
                Role::Viewer,
                Some(&format!("binary, ticket.{ticket}")),
                "",
                403,
                RejectReason::IpNotAllowed,
                Some(format!("order-{tls}").as_str()),
            )
            .await;
        assert_eq!(running.uses.lock().unwrap().len(), used_before);
        running.replace(|settings| {
            if tls {
                settings.relay.wss.viewer.allow = Some(allow("127.0.0.1"));
            } else {
                settings.relay.ws.viewer.allow = Some(allow("127.0.0.1"));
            }
        });
        // Also prove Origin refusal of a valid ticket does not consume it.
        running
            .rejected(
                tls,
                Role::Viewer,
                Some(&format!("binary, ticket.{ticket}")),
                "Origin: https://denied.example\r\n",
                403,
                RejectReason::OriginNotAllowed,
                None,
            )
            .await;
        assert_status(&running.ticket(tls, Role::Viewer, &ticket).await.0, 101);
    }
}

/// spec: websocket.md:WS-045
#[tokio::test]
async fn role_specific_issuers_keys_and_audiences_cannot_cross_paths() {
    let viewer = Signer::new();
    let server = Signer::new();
    let running = Running::start(Some(&viewer), Some(&server), 900).await;
    for tls in [false, true] {
        let viewer_ticket = viewer.sign(&claims(Role::Viewer, &format!("viewer-{tls}")));
        let server_ticket = server.sign(&claims(Role::Server, &format!("server-{tls}")));
        for (role, ticket) in [
            (Role::Viewer, &server_ticket),
            (Role::Server, &viewer_ticket),
        ] {
            running
                .rejected(
                    tls,
                    role,
                    Some(&format!("binary, ticket.{ticket}")),
                    "",
                    401,
                    RejectReason::Ticket(TicketError::Invalid),
                    None,
                )
                .await;
        }
        for (role, ticket) in [
            (Role::Viewer, &viewer_ticket),
            (Role::Server, &server_ticket),
        ] {
            assert_status(&running.ticket(tls, role, ticket).await.0, 101);
            assert!(running.events.events().iter().any(|event| matches!(event, Event::TicketAccepted { role: actual, ticket_jti, .. } if *actual==role && ticket_jti==&format!("{}-{tls}",role.as_str()))));
        }
    }
}

/// spec: websocket.md:WS-046 partial
#[tokio::test]
async fn replacement_settings_apply_to_new_upgrades_and_preserve_bindings_and_use_records() {
    // Snapshot replacement exercises the cutover itself, not its management-API trigger.
    let old = Signer::new();
    let new = Signer::new();
    let running = Running::start(Some(&old), None, 900).await;
    let mut value = claims(Role::Viewer, "existing");
    value["sx"] = json!(NOW + 2);
    let old_ticket = old.sign(&value);
    let (head, mut existing) = running.ticket(false, Role::Viewer, &old_ticket).await;
    assert_status(&head, 101);
    assert_eq!(frame(&mut existing).await.0, 2);
    running.replace(|settings| settings.relay.ticket.viewer.enabled = false);
    assert_status(&running.request(false, Role::Viewer, None, "").await.0, 101);
    running.replace(|settings| settings.relay.ticket.viewer.enabled = true);
    running
        .rejected(
            false,
            Role::Viewer,
            Some(&format!("binary, ticket.{old_ticket}")),
            "",
            401,
            RejectReason::Ticket(TicketError::Reused),
            Some("existing"),
        )
        .await;
    running.replace(|settings| {
        settings.relay.ticket.viewer.keys = new.keys.clone();
        settings.relay.ticket.viewer.issuer = Some("issuer.example/new".into());
        settings.relay.ticket.viewer.relay_host = Some("new-relay.example".into());
        settings.relay.ticket.viewer.max_lifetime = Duration::from_secs(10);
    });
    running
        .rejected(
            false,
            Role::Viewer,
            Some(&format!("binary, ticket.{old_ticket}")),
            "",
            401,
            RejectReason::Ticket(TicketError::Invalid),
            None,
        )
        .await;
    let mut fresh = claims(Role::Viewer, "fresh");
    fresh["iss"] = json!("issuer.example/new");
    fresh["rh"] = json!("new-relay.example");
    running
        .rejected(
            false,
            Role::Viewer,
            Some(&format!("binary, ticket.{}", new.sign(&fresh))),
            "",
            401,
            RejectReason::Ticket(TicketError::Invalid),
            Some("fresh"),
        )
        .await;
    fresh["exp"] = json!(NOW + 10);
    assert_status(
        &running
            .ticket(true, Role::Viewer, &new.sign(&fresh))
            .await
            .0,
        101,
    );
    fresh["jti"] = json!("existing");
    running
        .rejected(
            true,
            Role::Viewer,
            Some(&format!("binary, ticket.{}", new.sign(&fresh))),
            "",
            401,
            RejectReason::Ticket(TicketError::Reused),
            Some("existing"),
        )
        .await;
    // The pre-cutover connection still accepts its original rid and retains sx.
    initial_request(&mut existing, "ID:0123").await;
    wait_for(&running, |event| {
        matches!(event, Event::WaitingRegistered { .. })
    })
    .await;
    let (head, mut server) = running
        .request(false, Role::Server, Some("binary"), "")
        .await;
    assert_status(&head, 101);
    initial_request(&mut server, "ID:0123").await;
    send(&mut server, b"bound-before-reload").await;
    assert_data(&mut existing, b"bound-before-reload").await;
    // A wall-clock jump after 101 must not retime the captured monotonic deadline.
    running.clock.wall.store(NOW + 10_000, Ordering::SeqCst);
    let (opcode, payload) = frame(&mut existing).await;
    assert_eq!(opcode, 8);
    assert_eq!(&payload[..2], &4013_u16.to_be_bytes());
    assert_eq!(&payload[2..], b"session_expired");
}
