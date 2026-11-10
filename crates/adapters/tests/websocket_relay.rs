//! Real WS/WSS listeners, the common relay service, and real TCP peers.
//! Raw RFC 6455 frames keep fragmentation, large frames, and client data after
//! a server Close observable without a client library's automatic close logic.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, SystemTime};

use rustls::crypto::aws_lc_rs;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::version::TLS13;
use rustls::{ClientConfig, RootCertStore};
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_rustls::TlsConnector;
use vnc_relay_adapters::clock::SystemClock;
use vnc_relay_adapters::crypto::{RelayIdKey, SessionIdKey};
use vnc_relay_adapters::net::{AcceptPolicy, AllowSource, TcpEntryPoint};
use vnc_relay_adapters::ops_log::format_line;
use vnc_relay_adapters::target::TcpTargetConnector;
use vnc_relay_adapters::tls::{self, TlsSource};
use vnc_relay_adapters::websocket::{WsEntryPoint, WsPolicy};
use vnc_relay_app::events::Event;
use vnc_relay_app::ports::{Clock, EventSink};
use vnc_relay_app::relay::{RelayRuntime, RelayRuntimeConfig, RelayService};
use vnc_relay_app::reload::snapshot::{ReloadCell, ReloadSnapshot};
use vnc_relay_app::settings::KeepaliveSettings;
use vnc_relay_domain::allow_list::{AllowList, IpItem};
use vnc_relay_domain::budget::ConnectionBudget;
use vnc_relay_domain::entry_point::EntryPoint;
use vnc_relay_domain::handshake::{HANDSHAKE_REQUEST_LEN, HandshakePolicy, VIEWER_GREETING};
use vnc_relay_domain::target_policy::{TargetItem, TargetPolicy};

const LIMIT: Duration = Duration::from_secs(10);
const B: usize = 16 * 1024;
const M: usize = 64 * 1024;
const KEEPALIVE: KeepaliveSettings = KeepaliveSettings {
    tcp_idle: Duration::from_secs(60),
    tcp_interval: Duration::from_secs(10),
    tcp_probes: 6,
    ws_ping: Duration::from_secs(30),
    ws_pong_timeout: Duration::from_secs(30),
};

async fn bounded<T>(future: impl Future<Output = T>) -> T {
    timeout(LIMIT, future)
        .await
        .unwrap_or_else(|error| panic!(concat!("exchange finishes", ": {:?}"), error))
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/tls")
        .join(name)
}

fn connector() -> TlsConnector {
    let mut roots = RootCertStore::empty();
    for cert in CertificateDer::pem_file_iter(fixture("root-ca.pem"))
        .unwrap_or_else(|error| panic!(concat!("test root opens", ": {:?}"), error))
    {
        roots
            .add(cert.unwrap_or_else(|error| panic!(concat!("test root PEM", ": {:?}"), error)))
            .unwrap_or_else(|error| panic!("test CA: {error}"));
    }
    let config = ClientConfig::builder_with_provider(Arc::new(aws_lc_rs::default_provider()))
        .with_protocol_versions(&[&TLS13])
        .unwrap_or_else(|error| panic!(concat!("TLS 1.3 supported", ": {:?}"), error))
        .with_root_certificates(roots)
        .with_no_client_auth();
    TlsConnector::from(Arc::new(config))
}

#[derive(Default)]
struct Recorder(Mutex<Vec<Value>>);

impl Recorder {
    fn lock(&self) -> MutexGuard<'_, Vec<Value>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    async fn wait(&self, matches: impl Fn(&Value) -> bool) -> Value {
        bounded(async {
            loop {
                if let Some(line) = self.lock().iter().find(|line| matches(line)).cloned() {
                    return line;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
    }
}

impl EventSink for Recorder {
    fn record(&self, event: &Event) {
        let line = format_line(event, SystemTime::UNIX_EPOCH);
        assert_eq!(line.lines().count(), 1, "one JSON log line per event");
        self.lock().push(
            serde_json::from_str(&line)
                .unwrap_or_else(|error| panic!(concat!("operation log is JSON", ": {:?}"), error)),
        );
    }
}

struct Settings {
    places: u32,
    handshake: Duration,
    close: Duration,
    modes: HandshakePolicy,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            places: 8,
            handshake: LIMIT,
            close: Duration::from_secs(3),
            modes: HandshakePolicy {
                mode1_enabled: true,
                mode2_enabled: true,
            },
        }
    }
}

struct Running {
    addr: SocketAddr,
    server_addr: SocketAddr,
    secure: bool,
    events: Arc<Recorder>,
    service: Arc<RelayService>,
    runtime: Arc<RelayRuntime<TcpTargetConnector>>,
    clock: Arc<dyn Clock>,
    budget: Arc<ConnectionBudget>,
    limit: u32,
    stop: watch::Sender<bool>,
    tasks: Vec<JoinHandle<()>>,
}

impl Running {
    async fn start(secure: bool, settings: Settings) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|error| panic!(concat!("WS bind", ": {:?}"), error));
        let addr = listener
            .local_addr()
            .unwrap_or_else(|error| panic!(concat!("WS address", ": {:?}"), error));
        let server = TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|error| panic!(concat!("TCP Server bind", ": {:?}"), error));
        let server_addr = server
            .local_addr()
            .unwrap_or_else(|error| panic!(concat!("TCP Server address", ": {:?}"), error));
        let allow = AllowList::new(vec![
            IpItem::parse("127.0.0.1").unwrap_or_else(|error| panic!("loopback allow: {error:?}")),
        ]);
        let acceptor = secure.then(|| {
            tls::load_acceptor(
                &fixture("server-chain.pem"),
                &fixture("server-key-p256.pem"),
            )
            .unwrap_or_else(|error| panic!(concat!("test TLS identity", ": {:?}"), error))
        });
        let mut running = vnc_relay_adapters::config_file::parse(
            "[admin]\naccounts_file='accounts.toml'\n[management]\nlisten='127.0.0.1:8443'\n[status]\nlisten='127.0.0.1:9090'\n",
            Path::new("."),
        ).unwrap_or_else(|error| panic!("settings: {error:?}"));
        running.relay.tcp_server.allow = Some(allow.clone());
        running.relay.ws.viewer.enabled = true;
        running.relay.wss.viewer.enabled = true;
        running.relay.ws.viewer.allow = Some(allow.clone());
        running.relay.wss.viewer.allow = Some(allow);
        running.limits.relay_connections = settings.places;
        running.limits.waiting = 8;
        running.limits.buffer_bytes = B as u32;
        running.limits.ws_message_bytes = M as u32;
        running.timeouts.handshake = settings.handshake;
        running.timeouts.tls_upgrade = LIMIT;
        running.timeouts.wait = LIMIT;
        running.timeouts.connect = LIMIT;
        running.timeouts.close = settings.close;
        running.keepalive = KEEPALIVE;
        running.mode1.allow_targets = Some(TargetPolicy::new(vec![
            TargetItem::parse("127.0.0.1:1-65535")
                .unwrap_or_else(|error| panic!("loopback targets: {error:?}")),
        ]));
        let cell = Arc::new(ReloadCell::new(ReloadSnapshot::new(
            vnc_relay_app::reload::ConfigCandidate {
                settings: running,
                presence: Default::default(),
            },
            acceptor,
            None,
        )));
        let source: Arc<dyn vnc_relay_app::reload::ports::RuntimeSettingsSource> = cell.clone();
        let clock: Arc<dyn Clock> = Arc::new(SystemClock::new());
        let events = Arc::new(Recorder::default());
        let log: Arc<dyn EventSink> = events.clone();
        let budget = ConnectionBudget::new();
        let runtime =
            Arc::new(RelayRuntime::new(
                TcpTargetConnector::new(Arc::clone(&source)),
                Arc::new(RelayIdKey::generate().unwrap_or_else(|error| {
                    panic!(concat!("relay reference key", ": {:?}"), error)
                })),
                Arc::new(SessionIdKey::from_bytes(&[0x37; 32])),
                Arc::clone(&budget),
                ConnectionBudget::new(),
                RelayRuntimeConfig { recent_closed: 16 },
                Arc::clone(&source),
            ));
        let service = Arc::new(RelayService::new(
            settings.modes,
            source,
            Arc::clone(&clock),
            Arc::clone(&log),
        ));
        let (stop, stopped) = watch::channel(false);
        let accept = |allow| AcceptPolicy {
            allow,
            relay_budget: Arc::clone(&budget),
        };
        let entry = if secure {
            EntryPoint::WssViewer
        } else {
            EntryPoint::WsViewer
        };
        let ws = WsEntryPoint::new(
            entry.listener(),
            listener,
            secure.then(|| TlsSource::wss(Arc::clone(&cell))),
            WsPolicy {
                accept: accept(if secure {
                    AllowSource::wss(Arc::clone(&cell))
                } else {
                    AllowSource::ws(Arc::clone(&cell))
                }),
            },
            Arc::clone(&service),
            Arc::clone(&runtime),
            Arc::clone(&clock),
        );
        let tcp = TcpEntryPoint::new(
            EntryPoint::TcpServer,
            server,
            accept(AllowSource::tcp_server(cell)),
            Arc::clone(&service),
            Arc::clone(&runtime),
            Arc::clone(&clock),
        );
        let supervisor = {
            let service = Arc::clone(&service);
            let runtime = Arc::clone(&runtime);
            tokio::spawn(async move { service.drive_shutdown(&runtime).await })
        };
        let tasks = vec![
            tokio::spawn(ws.run(Arc::clone(&log), stopped.clone())),
            tokio::spawn(tcp.run(log, stopped)),
            supervisor,
        ];
        Self {
            addr,
            server_addr,
            secure,
            events,
            service,
            runtime,
            clock,
            budget,
            limit: settings.places,
            stop,
            tasks,
        }
    }

    async fn connect(&self) -> Client {
        Client::connect(self.addr, self.secure).await
    }

    async fn mode_i(&self, target: &TcpListener) -> (Client, TcpStream, Value) {
        let mut client = self.connect().await;
        client
            .binary(&request(
                &target
                    .local_addr()
                    .unwrap_or_else(|error| panic!(concat!("target address", ": {:?}"), error))
                    .to_string(),
            ))
            .await;
        let (peer, _) = bounded(target.accept())
            .await
            .unwrap_or_else(|error| panic!(concat!("target accepted", ": {:?}"), error));
        let started = self
            .events
            .wait(|line| {
                line["event"] == "session_started"
                    && line["viewer_transport"] == if self.secure { "wss" } else { "ws" }
                    && line["target_port"]
                        == target
                            .local_addr()
                            .unwrap_or_else(|error| {
                                panic!(concat!("target address", ": {:?}"), error)
                            })
                            .port()
            })
            .await;
        (client, peer, started)
    }

    fn listener_name(&self) -> &'static str {
        if self.secure { "wss" } else { "ws" }
    }

    async fn closed(&self, started: &Value, cause: &str) -> Value {
        let closed = self
            .events
            .wait(|line| {
                line["event"] == "session_closed" && line["session_id"] == started["session_id"]
            })
            .await;
        assert_eq!(closed["close_cause"], cause);
        assert_eq!(
            closed["viewer_transport"],
            if self.secure { "wss" } else { "ws" }
        );
        assert!(
            !self
                .runtime
                .snapshot(self.clock.as_ref())
                .active
                .iter()
                .any(|entry| {
                    entry.session_id.as_str()
                        == started["session_id"]
                            .as_str()
                            .unwrap_or_else(|| panic!("session ID"))
                })
        );
        assert_eq!(
            self.events
                .lock()
                .iter()
                .filter(|line| {
                    line["event"] == "session_closed" && line["session_id"] == started["session_id"]
                })
                .count(),
            1
        );
        closed
    }

    fn assert_empty(&self) {
        let snapshot = self.runtime.snapshot(self.clock.as_ref());
        assert!(snapshot.waiting.is_empty());
        assert!(snapshot.active.is_empty());
        assert_eq!(snapshot.target_connecting, 0);
        let permits: Vec<_> = (0..self.limit)
            .map(|_| {
                self.budget
                    .try_acquire(self.limit)
                    .unwrap_or_else(|| panic!("relay place released"))
            })
            .collect();
        assert!(self.budget.try_acquire(self.limit).is_none());
        drop(permits);
    }

    async fn finish(self) {
        self.service.request_shutdown(&self.runtime, Duration::ZERO);
        self.stop.send_replace(true);
        for task in self.tasks {
            bounded(task).await.unwrap_or_else(|error| {
                panic!(concat!("listener or supervisor completes", ": {:?}"), error)
            });
        }
    }
}

trait Wire: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Wire for T {}

struct Client {
    wire: Box<dyn Wire>,
    secure: bool,
}

struct Frame {
    opcode: u8,
    payload: Vec<u8>,
}

impl Client {
    async fn connect(addr: SocketAddr, secure: bool) -> Self {
        let tcp = bounded(TcpStream::connect(addr))
            .await
            .unwrap_or_else(|error| panic!(concat!("viewer connects", ": {:?}"), error));
        let wire: Box<dyn Wire> = if secure {
            let name = ServerName::try_from("localhost")
                .unwrap_or_else(|error| panic!(concat!("TLS hostname", ": {:?}"), error));
            Box::new(
                bounded(connector().connect(name, tcp))
                    .await
                    .unwrap_or_else(|error| panic!(concat!("TLS verified", ": {:?}"), error)),
            )
        } else {
            Box::new(tcp)
        };
        let mut client = Self { wire, secure };
        bounded(client.wire.write_all(
            b"GET /relay/viewer HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n",
        )).await.unwrap_or_else(|error| panic!(concat!("upgrade request sent", ": {:?}"), error));
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            head.push(
                bounded(client.wire.read_u8())
                    .await
                    .unwrap_or_else(|error| panic!(concat!("HTTP response byte", ": {:?}"), error)),
            );
            assert!(head.len() <= 16 * 1024, "bounded HTTP response");
        }
        assert!(head.starts_with(b"HTTP/1.1 101 "), "{head:?}");
        let greeting = client.frame().await;
        assert_eq!(greeting.opcode, 2, "first message is binary");
        assert_eq!(greeting.payload, VIEWER_GREETING);
        client
    }

    async fn send(&mut self, opcode: u8, fin: bool, bytes: &[u8]) {
        let mut frame = masked_header(opcode, fin, bytes.len());
        let mask = [0x13, 0x57, 0x9b, 0xdf];
        frame.extend(bytes.iter().enumerate().map(|(i, byte)| byte ^ mask[i % 4]));
        bounded(self.wire.write_all(&frame))
            .await
            .unwrap_or_else(|error| panic!(concat!("client frame sent", ": {:?}"), error));
    }

    async fn binary(&mut self, bytes: &[u8]) {
        self.send(2, true, bytes).await;
    }

    async fn close(&mut self, code: Option<u16>) {
        let payload = code.map_or_else(Vec::new, |code| code.to_be_bytes().to_vec());
        self.send(8, true, &payload).await;
    }

    async fn frame(&mut self) -> Frame {
        bounded(async {
            let first = self
                .wire
                .read_u8()
                .await
                .unwrap_or_else(|error| panic!(concat!("server frame", ": {:?}"), error));
            assert_eq!(first & 0x70, 0, "no unnegotiated extension");
            assert_ne!(first & 0x80, 0, "server message is not fragmented");
            let second = self
                .wire
                .read_u8()
                .await
                .unwrap_or_else(|error| panic!(concat!("server length", ": {:?}"), error));
            assert_eq!(second & 0x80, 0, "server frame is unmasked");
            let len =
                match second & 0x7f {
                    126 => {
                        usize::from(self.wire.read_u16().await.unwrap_or_else(|error| {
                            panic!(concat!("16-bit length", ": {:?}"), error)
                        }))
                    }
                    127 => {
                        usize::try_from(self.wire.read_u64().await.unwrap_or_else(|error| {
                            panic!(concat!("64-bit length", ": {:?}"), error)
                        }))
                        .unwrap_or_else(|error| panic!("frame length fits usize: {error}"))
                    }
                    len => usize::from(len),
                };
            assert!(len <= M, "server message stays within M");
            let mut payload = vec![0; len];
            self.wire
                .read_exact(&mut payload)
                .await
                .unwrap_or_else(|error| panic!(concat!("server payload", ": {:?}"), error));
            Frame {
                opcode: first & 0x0f,
                payload,
            }
        })
        .await
    }

    async fn data(&mut self, len: usize) -> Vec<u8> {
        let mut bytes = Vec::new();
        while bytes.len() < len {
            let frame = self.frame().await;
            assert_eq!(frame.opcode, 2, "relay sends binary data only");
            bytes.extend_from_slice(&frame.payload);
            assert!(bytes.len() <= len, "no extra relay bytes");
        }
        bytes
    }

    async fn expect_close(&mut self, code: u16, reason: &str) {
        let frame = self.frame().await;
        assert_eq!(frame.opcode, 8, "server sends Close, not data");
        let mut payload = code.to_be_bytes().to_vec();
        payload.extend_from_slice(reason.as_bytes());
        assert_eq!(frame.payload, payload);
    }

    async fn eof(&mut self) {
        let mut bytes = Vec::new();
        let result = bounded(self.wire.read_to_end(&mut bytes)).await;
        match result {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => {}
            // A validated WebSocket Close is authoritative for WS semantics.
            // Normal asymmetric closes may drop TLS without close_notify.
            Err(error) if self.secure && error.kind() == std::io::ErrorKind::UnexpectedEof => {}
            Err(error) => panic!("transport closes after WebSocket Close: {error}"),
        }
        assert!(bytes.is_empty(), "no frames or data after Close: {bytes:?}");
    }

    async fn terminal_eof(&mut self) {
        self.eof().await;
        // Terminal actions have ended server writes. Return the client's
        // transport EOF so the relay can finish bounded raw-input disposal.
        let _ = bounded(self.wire.shutdown()).await;
    }
}

fn masked_header(opcode: u8, fin: bool, len: usize) -> Vec<u8> {
    let mut bytes = vec![opcode | if fin { 0x80 } else { 0 }];
    if len < 126 {
        bytes.push(
            0x80 | u8::try_from(len)
                .unwrap_or_else(|error| panic!(concat!("small length", ": {:?}"), error)),
        );
    } else if let Ok(len) = u16::try_from(len) {
        bytes.push(0x80 | 126);
        bytes.extend_from_slice(&len.to_be_bytes());
    } else {
        bytes.push(0x80 | 127);
        bytes.extend_from_slice(
            &u64::try_from(len)
                .unwrap_or_else(|error| panic!(concat!("64-bit length", ": {:?}"), error))
                .to_be_bytes(),
        );
    }
    bytes.extend_from_slice(&[0x13, 0x57, 0x9b, 0xdf]);
    bytes
}

fn request(value: &str) -> [u8; HANDSHAKE_REQUEST_LEN] {
    assert!(value.len() <= HANDSHAKE_REQUEST_LEN);
    let mut bytes = [0; HANDSHAKE_REQUEST_LEN];
    bytes[..value.len()].copy_from_slice(value.as_bytes());
    bytes
}

async fn target() -> TcpListener {
    TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap_or_else(|error| panic!(concat!("target bind", ": {:?}"), error))
}

async fn read_tcp(peer: &mut TcpStream, len: usize) -> Vec<u8> {
    let mut bytes = vec![0; len];
    bounded(peer.read_exact(&mut bytes))
        .await
        .unwrap_or_else(|error| panic!(concat!("TCP payload", ": {:?}"), error));
    bytes
}

async fn tcp_eof(peer: &mut TcpStream) -> Vec<u8> {
    let mut bytes = Vec::new();
    bounded(peer.read_to_end(&mut bytes))
        .await
        .unwrap_or_else(|error| panic!(concat!("TCP EOF", ": {:?}"), error));
    bytes
}

async fn close_normally(
    running: &Running,
    client: &mut Client,
    peer: &mut TcpStream,
    started: &Value,
) -> Value {
    client.close(Some(1000)).await;
    assert!(tcp_eof(peer).await.is_empty());
    bounded(peer.shutdown())
        .await
        .unwrap_or_else(|error| panic!(concat!("TCP half-close", ": {:?}"), error));
    client.expect_close(1000, "").await;
    client.eof().await;
    running.closed(started, "peer_eof").await
}

/// spec: websocket.md:WS-018
/// spec: websocket.md:WS-003
/// spec: websocket.md:WS-004
/// spec: websocket.md:WS-005
/// spec: websocket.md:WS-006
/// spec: relay.md:RLY-003 partial
#[tokio::test]
async fn ws_and_wss_forward_initial_trailing_data_once() {
    for secure in [false, true] {
        for server_first in [false, true] {
            let running = Running::start(secure, Settings::default()).await;
            let mut client = running.connect().await;
            let mut initial = request("ID:01234567").to_vec();
            initial.extend_from_slice(b"initial trailing bytes\0\xff");
            let mut peer = if server_first {
                let mut peer = bounded(TcpStream::connect(running.server_addr))
                    .await
                    .unwrap_or_else(|error| panic!(concat!("Server connects", ": {:?}"), error));
                bounded(peer.write_all(&request("ID:01234567")))
                    .await
                    .unwrap_or_else(|error| panic!(concat!("Server registers", ": {:?}"), error));
                running
                    .events
                    .wait(|line| line["event"] == "waiting_registered")
                    .await;
                client.binary(&initial).await;
                peer
            } else {
                client.binary(&initial).await;
                running
                    .events
                    .wait(|line| line["event"] == "waiting_registered")
                    .await;
                let mut peer = bounded(TcpStream::connect(running.server_addr))
                    .await
                    .unwrap_or_else(|error| panic!(concat!("Server connects", ": {:?}"), error));
                bounded(peer.write_all(&request("ID:01234567")))
                    .await
                    .unwrap_or_else(|error| panic!(concat!("Server registers", ": {:?}"), error));
                peer
            };
            let started = running
                .events
                .wait(|line| line["event"] == "session_started")
                .await;
            assert_eq!(started["mode"], "mode2");
            client.binary(b"later bytes").await;
            bounded(peer.write_all(b"TCP reply"))
                .await
                .unwrap_or_else(|error| panic!(concat!("Server reply", ": {:?}"), error));
            assert_eq!(client.data(9).await, b"TCP reply");
            client.close(Some(1000)).await;
            let all = tcp_eof(&mut peer).await;
            assert_eq!(
                all, b"initial trailing bytes\0\xfflater bytes",
                "250 request bytes never reach TCP; trailing bytes arrive once"
            );
            bounded(peer.shutdown())
                .await
                .unwrap_or_else(|error| panic!(concat!("Server closes", ": {:?}"), error));
            client.expect_close(1000, "").await;
            client.eof().await;
            let closed = running.closed(&started, "peer_eof").await;
            assert_eq!(closed["bytes_viewer_to_target"], all.len());
            assert_eq!(closed["bytes_target_to_viewer"], 9);
            running.assert_empty();
            running.finish().await;
        }
    }
}

/// spec: websocket.md:WS-001
/// spec: websocket.md:WS-002
/// spec: relay.md:RLY-017 partial
#[tokio::test]
async fn ws_and_wss_mode_i_relay_both_directions() {
    for secure in [false, true] {
        let running = Running::start(secure, Settings::default()).await;
        let target = target().await;
        let (mut client, mut peer, started) = running.mode_i(&target).await;
        assert_eq!(started["mode"], "mode1");
        let from_viewer = b"viewer\0\xff\x80 to target";
        let from_target = b"target\xfe\0\x81 to viewer";
        tokio::join!(client.binary(from_viewer), async {
            bounded(peer.write_all(from_target))
                .await
                .expect("target writes");
        });
        assert_eq!(read_tcp(&mut peer, from_viewer.len()).await, from_viewer);
        assert_eq!(client.data(from_target.len()).await, from_target);
        let closed = close_normally(&running, &mut client, &mut peer, &started).await;
        assert_eq!(closed["bytes_viewer_to_target"], from_viewer.len());
        assert_eq!(closed["bytes_target_to_viewer"], from_target.len());
        running.assert_empty();
        running.finish().await;
    }
}

/// spec: websocket.md:WS-016 partial
#[tokio::test]
async fn client_close_drains_to_tcp_and_echoes_only_the_code() {
    for secure in [false, true] {
        for code in [None, Some(3001)] {
            let running = Running::start(secure, Settings::default()).await;
            let target = target().await;
            let (mut client, mut peer, started) = running.mode_i(&target).await;
            client.binary(b"accepted before Close").await;
            let payload = code.map_or_else(Vec::new, |code: u16| {
                let mut payload = code.to_be_bytes().to_vec();
                payload.extend_from_slice(b"private client reason");
                payload
            });
            client.send(8, true, &payload).await;
            assert_eq!(tcp_eof(&mut peer).await, b"accepted before Close");
            client.expect_close(code.unwrap_or(1000), "").await;
            // The TCP read half is still alive, but these post-Close bytes must
            // be discarded, not framed or counted as relayed payload.
            bounded(peer.write_all(b"discard after client Close"))
                .await
                .unwrap_or_else(|error| {
                    panic!(concat!("TCP remains writable until EOF", ": {:?}"), error)
                });
            bounded(peer.shutdown())
                .await
                .unwrap_or_else(|error| panic!(concat!("target EOF", ": {:?}"), error));
            client.eof().await;
            let closed = running.closed(&started, "peer_eof").await;
            assert_eq!(closed["bytes_viewer_to_target"], 21);
            assert_eq!(closed["bytes_target_to_viewer"], 0);
            assert!(
                !running
                    .events
                    .lock()
                    .iter()
                    .any(|line| line.to_string().contains("private client reason"))
            );
            running.assert_empty();
            running.finish().await;
        }
    }
}

/// spec: websocket.md:WS-016 partial
/// spec: relay.md:RLY-019 partial
#[tokio::test]
async fn tcp_eof_drains_then_forwards_client_data_after_close_1000() {
    for secure in [false, true] {
        let running = Running::start(secure, Settings::default()).await;
        let target = target().await;
        let (mut client, mut peer, started) = running.mode_i(&target).await;
        bounded(peer.write_all(b"accepted before EOF"))
            .await
            .unwrap_or_else(|error| panic!(concat!("target payload", ": {:?}"), error));
        bounded(peer.shutdown())
            .await
            .unwrap_or_else(|error| panic!(concat!("target sends EOF", ": {:?}"), error));
        assert_eq!(client.data(19).await, b"accepted before EOF");
        client.expect_close(1000, "").await;
        // Deliberately bypass a client library's refusal to send after Close.
        client.binary(b"client data after server Close").await;
        assert_eq!(
            read_tcp(&mut peer, 30).await,
            b"client data after server Close"
        );
        client.close(Some(1000)).await;
        assert!(tcp_eof(&mut peer).await.is_empty());
        client.eof().await;
        let closed = running.closed(&started, "peer_eof").await;
        assert_eq!(closed["bytes_viewer_to_target"], 30);
        assert_eq!(closed["bytes_target_to_viewer"], 19);
        running.assert_empty();
        running.finish().await;
    }
}

/// spec: websocket.md:WS-017 partial
/// spec: relay.md:RLY-021 partial
#[tokio::test]
async fn close_timeout_in_either_order_releases_only_its_session() {
    for secure in [false, true] {
        for client_first in [false, true] {
            let running = Running::start(
                secure,
                Settings {
                    close: Duration::from_millis(300),
                    ..Settings::default()
                },
            )
            .await;
            let first_target = target().await;
            let (mut client, mut peer, started) = running.mode_i(&first_target).await;
            let other_target = target().await;
            let (mut other, mut other_peer, other_started) = running.mode_i(&other_target).await;
            if client_first {
                client.close(Some(1000)).await;
                assert!(tcp_eof(&mut peer).await.is_empty());
                client.expect_close(1000, "").await;
                // Do not half-close the target: E must bound this wait too.
            } else {
                bounded(peer.shutdown())
                    .await
                    .unwrap_or_else(|error| panic!(concat!("target EOF", ": {:?}"), error));
                client.expect_close(1000, "").await;
                // Do not answer the server's Close.
            }
            client.eof().await;
            running.closed(&started, "close_timeout").await;
            assert_eq!(
                running
                    .runtime
                    .snapshot(running.clock.as_ref())
                    .active
                    .len(),
                1
            );
            other.binary(b"other session survives").await;
            assert_eq!(
                read_tcp(&mut other_peer, 22).await,
                b"other session survives"
            );
            bounded(other_peer.write_all(b"still duplex"))
                .await
                .unwrap_or_else(|error| panic!(concat!("other target reply", ": {:?}"), error));
            assert_eq!(other.data(12).await, b"still duplex");
            close_normally(&running, &mut other, &mut other_peer, &other_started).await;
            drop(peer);
            running.assert_empty();
            running.finish().await;
        }
    }
}

async fn initial_failure(
    running: &Running,
    event: &str,
    reason: &str,
    kind: Option<&str>,
) -> Value {
    let line = running
        .events
        .wait(|line| {
            line["event"] == event
                && line["stage"] == "initial_exchange"
                && line["reason"] == reason
        })
        .await;
    assert_eq!(line["listener"], running.listener_name());
    assert!(line["conn_id"].is_string());
    match kind {
        Some(kind) => assert_eq!(line["error_kind"], kind),
        None => assert!(line.get("error_kind").is_none()),
    }
    assert_eq!(
        running
            .events
            .lock()
            .iter()
            .filter(|entry| {
                entry["conn_id"] == line["conn_id"]
                    && (entry["event"] == "connection_aborted"
                        || entry["event"] == "connection_rejected")
            })
            .count(),
        1
    );
    running.assert_empty();
    line
}

/// spec: relay.md:RLY-004 partial
#[tokio::test]
async fn invalid_initial_requests_close_4008_and_log_the_rejection() {
    for secure in [false, true] {
        for value in ["", "ID:not_digits", "127.0.0.1:0"] {
            let running = Running::start(secure, Settings::default()).await;
            let mut client = running.connect().await;
            client.binary(&request(value)).await;
            client.expect_close(4008, "handshake_invalid").await;
            client.terminal_eof().await;
            initial_failure(&running, "connection_rejected", "handshake_invalid", None).await;
            running.finish().await;
        }
    }
}

/// spec: relay.md:RLY-005 partial
/// spec: operations.md:OPS-006 partial
#[tokio::test]
async fn incomplete_initial_exchange_closes_4009_and_logs_h_timeout() {
    for secure in [false, true] {
        let running = Running::start(
            secure,
            Settings {
                handshake: Duration::from_millis(300),
                ..Settings::default()
            },
        )
        .await;
        let mut client = running.connect().await;
        client.binary(b"ID:12").await;
        client.expect_close(4009, "handshake_timeout").await;
        client.terminal_eof().await;
        initial_failure(&running, "connection_aborted", "handshake_timeout", None).await;
        running.finish().await;
    }
}

/// spec: relay.md:RLY-022 partial
#[tokio::test]
async fn disabled_modes_close_1008_before_registration_or_target_connect() {
    for secure in [false, true] {
        for mode1 in [false, true] {
            let running = Running::start(
                secure,
                Settings {
                    modes: HandshakePolicy {
                        mode1_enabled: !mode1,
                        mode2_enabled: mode1,
                    },
                    ..Settings::default()
                },
            )
            .await;
            let target = target().await;
            let value = if mode1 {
                target
                    .local_addr()
                    .unwrap_or_else(|error| panic!(concat!("target address", ": {:?}"), error))
                    .to_string()
            } else {
                "ID:123456".to_owned()
            };
            let mut client = running.connect().await;
            client.binary(&request(&value)).await;
            client.expect_close(1008, "mode_disabled").await;
            client.terminal_eof().await;
            let line =
                initial_failure(&running, "connection_rejected", "mode_disabled", None).await;
            assert_eq!(line["mode"], if mode1 { "mode1" } else { "mode2" });
            assert!(
                timeout(Duration::from_millis(30), target.accept())
                    .await
                    .is_err(),
                "disabled mode makes no target connection"
            );
            assert!(
                !running
                    .events
                    .lock()
                    .iter()
                    .any(|line| line["event"] == "waiting_registered"
                        || line["event"] == "session_started")
            );
            running.finish().await;
        }
    }
}

/// spec: websocket.md:WS-008
#[tokio::test]
async fn text_frame_closes_1003_without_forwarding_text_and_logs_typed_error() {
    for secure in [false, true] {
        let running = Running::start(secure, Settings::default()).await;
        let target = target().await;
        let (mut client, mut peer, started) = running.mode_i(&target).await;
        client.send(1, true, b"must not reach TCP").await;
        client.expect_close(1003, "").await;
        client.terminal_eof().await;
        assert!(tcp_eof(&mut peer).await.is_empty());
        let closed = running.closed(&started, "connection_error").await;
        assert_eq!(closed["error_kind"], "ws_text_message");
        assert_eq!(closed["bytes_viewer_to_target"], 0);
        running.assert_empty();
        running.finish().await;
    }
}

/// spec: websocket.md:WS-010
#[tokio::test]
async fn single_and_fragmented_messages_accept_m_and_close_1009_at_m_plus_one() {
    for secure in [false, true] {
        for fragmented in [false, true] {
            let running = Running::start(secure, Settings::default()).await;
            let target = target().await;
            let (mut client, mut peer, started) = running.mode_i(&target).await;
            let payload: Vec<u8> = (0..M).map(|i| (i % 251) as u8).collect();
            let send = async {
                if fragmented {
                    client.send(2, false, &payload[..M / 2]).await;
                    client.send(0, true, &payload[M / 2..]).await;
                } else {
                    client.binary(&payload).await;
                }
            };
            let (_, received) = tokio::join!(send, read_tcp(&mut peer, M));
            assert_eq!(received, payload, "M is accepted, regardless of fragments");
            if fragmented {
                client.send(2, false, &payload).await;
                assert_eq!(read_tcp(&mut peer, M).await, payload);
                // The final continuation's complete header and payload share
                // one write and make the logical message M+1.
                client.send(0, true, b"x").await;
            } else {
                let mut oversized = payload.clone();
                oversized.push(0);
                // Submit the rejected header and all M+1 bytes together.
                client.binary(&oversized).await;
            }
            client.expect_close(1009, "").await;
            client.terminal_eof().await;
            assert!(
                tcp_eof(&mut peer).await.is_empty(),
                "nothing follows the already accepted prefix"
            );
            let closed = running.closed(&started, "connection_error").await;
            assert_eq!(closed["error_kind"], "ws_message_too_large");
            assert_eq!(
                closed["bytes_viewer_to_target"],
                if fragmented { 2 * M } else { M }
            );
            running.assert_empty();
            running.finish().await;
        }
    }
}

/// spec: operations.md:OPS-023 partial
#[tokio::test]
async fn mode_i_target_limit_after_101_closes_4011_and_preserves_existing_session() {
    for secure in [false, true] {
        let running = Running::start(
            secure,
            Settings {
                places: 3,
                ..Settings::default()
            },
        )
        .await;
        let first_target = target().await;
        let (mut first, mut first_peer, started) = running.mode_i(&first_target).await;
        let denied_target = target().await;
        let mut client = running.connect().await;
        // Existing viewer+target use two places. This upgraded viewer owns
        // the third; its Mode I target would exceed the same shared L.
        client
            .binary(&request(
                &denied_target
                    .local_addr()
                    .unwrap_or_else(|error| panic!(concat!("target address", ": {:?}"), error))
                    .to_string(),
            ))
            .await;
        client.expect_close(4011, "relay_connection_limit").await;
        client.terminal_eof().await;
        let rejected = running
            .events
            .wait(|line| {
                line["event"] == "connection_rejected"
                    && line["stage"] == "target_connect"
                    && line["reason"] == "relay_connection_limit"
            })
            .await;
        assert_eq!(rejected["listener"], running.listener_name());
        assert_eq!(rejected["mode"], "mode1");
        assert_eq!(rejected["limit"], 3);
        assert!(
            timeout(Duration::from_millis(30), denied_target.accept())
                .await
                .is_err(),
            "no out-of-budget target socket"
        );
        assert_eq!(
            running
                .runtime
                .snapshot(running.clock.as_ref())
                .active
                .len(),
            1
        );
        first.binary(b"existing remains").await;
        assert_eq!(read_tcp(&mut first_peer, 16).await, b"existing remains");
        bounded(first_peer.write_all(b"reply"))
            .await
            .unwrap_or_else(|error| panic!(concat!("existing target reply", ": {:?}"), error));
        assert_eq!(first.data(5).await, b"reply");
        let available = running
            .budget
            .try_acquire(running.limit)
            .unwrap_or_else(|| panic!("rejected viewer released its L place"));
        assert!(running.budget.try_acquire(running.limit).is_none());
        drop(available);
        close_normally(&running, &mut first, &mut first_peer, &started).await;
        running.assert_empty();
        running.finish().await;
    }
}

/// spec: websocket.md:WS-011 partial
/// spec: relay.md:RLY-018 partial
#[tokio::test]
async fn a_single_frame_larger_than_b_is_relayed_intact() {
    for secure in [false, true] {
        let running = Running::start(secure, Settings::default()).await;
        let target = target().await;
        let (mut client, mut peer, started) = running.mode_i(&target).await;
        let payload: Vec<u8> = (0..3 * B + 37).map(|i| (i % 251) as u8).collect();
        assert!(payload.len() > B && payload.len() < M);
        let (_, received) = tokio::join!(client.binary(&payload), async {
            tokio::time::sleep(Duration::from_millis(30)).await;
            read_tcp(&mut peer, payload.len()).await
        });
        assert_eq!(
            received, payload,
            "one masked frame >B survives chunk boundaries"
        );
        let (_, received) = tokio::join!(
            async {
                bounded(peer.write_all(&payload))
                    .await
                    .expect("TCP large reply");
            },
            async {
                tokio::time::sleep(Duration::from_millis(30)).await;
                client.data(payload.len()).await
            }
        );
        assert_eq!(received, payload, "reverse direction preserves bytes too");
        let closed = close_normally(&running, &mut client, &mut peer, &started).await;
        assert_eq!(closed["bytes_viewer_to_target"], payload.len());
        assert_eq!(closed["bytes_target_to_viewer"], payload.len());
        running.assert_empty();
        running.finish().await;
    }
}
