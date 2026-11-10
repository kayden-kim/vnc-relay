//! Real WS/WSS acceptance scenarios. Raw masked RFC 6455 clients exercise the
//! production entry points, relay service, TCP targets, and Mode II registry.

use std::io;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, SystemTime};

use rustls::crypto::aws_lc_rs;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::version::TLS13;
use rustls::{ClientConfig, RootCertStore};
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Notify, watch};
use tokio::task::JoinHandle;
use tokio::time::{Instant, timeout};
use tokio_rustls::TlsConnector;
use vnc_relay_adapters::clock::SystemClock;
use vnc_relay_adapters::crypto::{RelayIdKey, SessionIdKey};
use vnc_relay_adapters::net::{AcceptPolicy, AllowSource, TcpEntryPoint};
use vnc_relay_adapters::ops_log::format_line;
use vnc_relay_adapters::target::TcpTargetConnector;
use vnc_relay_adapters::tls::{self, TlsSource};
use vnc_relay_adapters::websocket::{WsEntryPoint, WsPolicy};
use vnc_relay_adapters::websocket_stream::WsRelayStream;
use vnc_relay_app::events::Event;
use vnc_relay_app::ports::{Clock, EventSink, RelayCloseAction, RelayStream};
use vnc_relay_app::relay::{ConnectionContext, RelayRuntime, RelayRuntimeConfig, RelayService};
use vnc_relay_app::reload::snapshot::{ReloadCell, ReloadSnapshot};
use vnc_relay_app::settings::KeepaliveSettings;
use vnc_relay_domain::allow_list::{AllowList, IpItem};
use vnc_relay_domain::budget::ConnectionBudget;
use vnc_relay_domain::conn_id::ConnId;
use vnc_relay_domain::entry_point::{EntryPoint, Listener};
use vnc_relay_domain::handshake::{HANDSHAKE_REQUEST_LEN, HandshakePolicy, VIEWER_GREETING};
use vnc_relay_domain::peer_ip::PeerIp;
use vnc_relay_domain::session::Transport;
use vnc_relay_domain::target_policy::{TargetItem, TargetPolicy};

const LIMIT: Duration = Duration::from_secs(10);
const E: Duration = Duration::from_secs(1);
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
        .unwrap_or_else(|error| panic!("socket scenario exceeded {LIMIT:?}: {error}"))
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/tls")
        .join(name)
}

fn connector() -> TlsConnector {
    let mut roots = RootCertStore::empty();
    for cert in CertificateDer::pem_file_iter(fixture("root-ca.pem"))
        .unwrap_or_else(|error| panic!("test root opens: {error}"))
    {
        roots
            .add(cert.unwrap_or_else(|error| panic!("root PEM: {error}")))
            .unwrap_or_else(|error| panic!("test CA: {error}"));
    }
    TlsConnector::from(Arc::new(
        ClientConfig::builder_with_provider(Arc::new(aws_lc_rs::default_provider()))
            .with_protocol_versions(&[&TLS13])
            .unwrap_or_else(|error| panic!("TLS 1.3: {error}"))
            .with_root_certificates(roots)
            .with_no_client_auth(),
    ))
}

fn acceptor() -> tokio_rustls::TlsAcceptor {
    tls::load_acceptor(
        &fixture("server-chain.pem"),
        &fixture("server-key-p256.pem"),
    )
    .unwrap_or_else(|error| panic!("test TLS identity: {error}"))
}

#[derive(Default)]
struct Recorder {
    lines: Mutex<Vec<Value>>,
    changed: Notify,
}

impl Recorder {
    async fn wait(&self, matches: impl Fn(&Value) -> bool) -> Value {
        bounded(async {
            loop {
                let changed = self.changed.notified();
                let lines = lock(&self.lines).clone();
                if let Some(line) = lines.iter().find(|line| matches(line)).cloned() {
                    return line;
                }
                changed.await;
            }
        })
        .await
    }
}

impl EventSink for Recorder {
    fn record(&self, event: &Event) {
        let text = format_line(event, SystemTime::UNIX_EPOCH);
        assert_eq!(text.lines().count(), 1);
        lock(&self.lines).push(
            serde_json::from_str(&text)
                .unwrap_or_else(|error| panic!("operation log JSON: {error}")),
        );
        self.changed.notify_one();
    }
}

struct Running {
    addr: SocketAddr,
    server_addr: SocketAddr,
    viewer_addr: SocketAddr,
    ws_addr: SocketAddr,
    wss_addr: SocketAddr,
    cell: Arc<ReloadCell<tokio_rustls::TlsAcceptor>>,
    secure: bool,
    events: Arc<Recorder>,
    service: Arc<RelayService>,
    runtime: Arc<RelayRuntime<TcpTargetConnector>>,
    clock: Arc<dyn Clock>,
    budget: Arc<ConnectionBudget>,
    waiting_budget: Arc<ConnectionBudget>,
    stop: watch::Sender<bool>,
    tasks: Vec<JoinHandle<()>>,
}

impl Running {
    async fn start(secure: bool) -> Self {
        Self::start_with_keepalive(secure, KEEPALIVE, LIMIT).await
    }

    async fn start_with_keepalive(
        secure: bool,
        keepalive: KeepaliveSettings,
        wait_timeout: Duration,
    ) -> Self {
        Self::start_configured(secure, keepalive, wait_timeout, |_| {}).await
    }

    async fn start_configured(
        secure: bool,
        keepalive: KeepaliveSettings,
        wait_timeout: Duration,
        configure: impl FnOnce(&mut vnc_relay_app::settings::Settings),
    ) -> Self {
        let listener = target().await;
        let server = target().await;
        let other_ws = target().await;
        let tcp_viewer = target().await;
        let other_addr = other_ws
            .local_addr()
            .unwrap_or_else(|e| panic!("other WS address: {e}"));
        let viewer_addr = tcp_viewer
            .local_addr()
            .unwrap_or_else(|e| panic!("TCP Viewer address: {e}"));
        let addr = listener
            .local_addr()
            .unwrap_or_else(|error| panic!("WS address: {error}"));
        let server_addr = server
            .local_addr()
            .unwrap_or_else(|error| panic!("Server address: {error}"));
        let allow = AllowList::new(vec![
            IpItem::parse("127.0.0.1").unwrap_or_else(|error| panic!("loopback allow: {error:?}")),
        ]);
        let mut settings = vnc_relay_adapters::config_file::parse(
            "[admin]\naccounts_file='accounts.toml'\n[management]\nlisten='127.0.0.1:8443'\n[status]\nlisten='127.0.0.1:9090'\n",
            Path::new("."),
        ).unwrap_or_else(|error| panic!("settings: {error:?}"));
        settings.relay.tcp_server.allow = Some(allow.clone());
        settings.relay.tcp_viewer.allow = Some(allow.clone());
        settings.relay.ws.viewer.enabled = true;
        settings.relay.wss.viewer.enabled = true;
        settings.relay.ws.viewer.allow = Some(allow.clone());
        settings.relay.wss.viewer.allow = Some(allow);
        settings.relay.ws.server = settings.relay.ws.viewer.clone();
        settings.relay.wss.server = settings.relay.wss.viewer.clone();
        settings.limits.relay_connections = 8;
        settings.limits.waiting = 8;
        settings.limits.buffer_bytes = B as u32;
        settings.limits.ws_message_bytes = M as u32;
        settings.timeouts.handshake = Duration::from_secs(3);
        settings.timeouts.tls_upgrade = Duration::from_secs(3);
        settings.timeouts.wait = wait_timeout;
        settings.timeouts.connect = E;
        settings.timeouts.close = E;
        settings.keepalive = keepalive;
        settings.mode1.allow_targets = Some(TargetPolicy::new(vec![
            TargetItem::parse("127.0.0.1:1-65535")
                .unwrap_or_else(|error| panic!("loopback targets: {error:?}")),
        ]));
        configure(&mut settings);
        let cell = Arc::new(ReloadCell::new(ReloadSnapshot::new(
            vnc_relay_app::reload::ConfigCandidate {
                settings,
                presence: Default::default(),
            },
            Some(acceptor()),
            None,
        )));
        let source: Arc<dyn vnc_relay_app::reload::ports::RuntimeSettingsSource> = cell.clone();
        let clock: Arc<dyn Clock> = Arc::new(SystemClock::new());
        let events = Arc::new(Recorder::default());
        let log: Arc<dyn EventSink> = events.clone();
        let budget = ConnectionBudget::new();
        let waiting_budget = ConnectionBudget::new();
        let runtime = Arc::new(RelayRuntime::new(
            TcpTargetConnector::new(Arc::clone(&source)),
            Arc::new(RelayIdKey::generate().unwrap_or_else(|error| panic!("relay key: {error:?}"))),
            Arc::new(SessionIdKey::from_bytes(&[0x61; 32])),
            Arc::clone(&budget),
            Arc::clone(&waiting_budget),
            RelayRuntimeConfig { recent_closed: 32 },
            Arc::clone(&source),
        ));
        let service = Arc::new(RelayService::new(
            HandshakePolicy {
                mode1_enabled: true,
                mode2_enabled: true,
            },
            source,
            Arc::clone(&clock),
            Arc::clone(&log),
        ));
        let (stop, stopped) = watch::channel(false);
        let accept = |allow| AcceptPolicy {
            allow,
            relay_budget: Arc::clone(&budget),
        };
        let ws = WsEntryPoint::new(
            if secure { Listener::Wss } else { Listener::Ws },
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
            accept(AllowSource::tcp_server(Arc::clone(&cell))),
            Arc::clone(&service),
            Arc::clone(&runtime),
            Arc::clone(&clock),
        );
        let other = WsEntryPoint::new(
            if secure { Listener::Ws } else { Listener::Wss },
            other_ws,
            (!secure).then(|| TlsSource::wss(Arc::clone(&cell))),
            WsPolicy {
                accept: accept(if secure {
                    AllowSource::ws(Arc::clone(&cell))
                } else {
                    AllowSource::wss(Arc::clone(&cell))
                }),
            },
            Arc::clone(&service),
            Arc::clone(&runtime),
            Arc::clone(&clock),
        );
        let viewer = TcpEntryPoint::new(
            EntryPoint::TcpViewer,
            tcp_viewer,
            accept(AllowSource::tcp_viewer(Arc::clone(&cell))),
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
            tokio::spawn(other.run(Arc::clone(&log), stopped.clone())),
            tokio::spawn(viewer.run(Arc::clone(&log), stopped.clone())),
            tokio::spawn(tcp.run(log, stopped)),
            supervisor,
        ];
        Self {
            addr,
            server_addr,
            viewer_addr,
            ws_addr: if secure { other_addr } else { addr },
            wss_addr: if secure { addr } else { other_addr },
            cell,
            secure,
            events,
            service,
            runtime,
            clock,
            budget,
            waiting_budget,
            stop,
            tasks,
        }
    }

    async fn connect(&self) -> Client {
        let mut client = Client::transport(self.addr, self.secure).await;
        let response = client
            .http(&upgrade("GET", "/relay/viewer", "13", true))
            .await;
        assert!(response.starts_with("HTTP/1.1 101 "), "{response}");
        assert!(!response.to_ascii_lowercase().contains("set-cookie:"));
        client.greeting().await;
        client
    }

    async fn pair(&self, mode2: bool, id: &str) -> (Client, TcpStream, Value) {
        let first = lock(&self.events.lines).len();
        let mut client = self.connect().await;
        let (peer, port) = if mode2 {
            client.binary(&request(&format!("ID:{id}"))).await;
            let registered = self
                .events
                .wait(|line| {
                    line["event"] == "waiting_registered"
                        && lock(&self.events.lines)[first..]
                            .iter()
                            .any(|new| new == line)
                })
                .await;
            assert_eq!(registered["role"], "viewer");
            assert_eq!(self.runtime.snapshot(self.clock.as_ref()).waiting.len(), 1);
            let mut peer = bounded(TcpStream::connect(self.server_addr))
                .await
                .unwrap_or_else(|error| panic!("TCP Server connects: {error}"));
            bounded(peer.write_all(&request(&format!("ID:{id}"))))
                .await
                .unwrap_or_else(|error| panic!("TCP Server registers: {error}"));
            (peer, None)
        } else {
            let listener = target().await;
            let address = listener
                .local_addr()
                .unwrap_or_else(|error| panic!("target address: {error}"));
            client.binary(&request(&address.to_string())).await;
            let (peer, _) = bounded(listener.accept())
                .await
                .unwrap_or_else(|error| panic!("target accepted: {error}"));
            (peer, Some(address.port()))
        };
        let started = self
            .events
            .wait(|line| {
                line["event"] == "session_started"
                    && port.map_or(line["mode"] == "mode2", |port| line["target_port"] == port)
                    && lock(&self.events.lines)[first..]
                        .iter()
                        .any(|new| new == line)
            })
            .await;
        assert_eq!(
            started["viewer_transport"],
            if self.secure { "wss" } else { "ws" }
        );
        (client, peer, started)
    }

    async fn closed(&self, started: &Value, cause: &str, side: &str) -> Value {
        let closed = self
            .events
            .wait(|line| {
                line["event"] == "session_closed" && line["session_id"] == started["session_id"]
            })
            .await;
        assert_eq!(closed["close_cause"], cause);
        assert_eq!(closed["side"], side);
        assert_eq!(
            closed["level"],
            if cause == "peer_eof" { "info" } else { "warn" }
        );
        assert_eq!(closed["viewer_transport"], started["viewer_transport"]);
        assert_eq!(
            lock(&self.events.lines)
                .iter()
                .filter(|line| line["event"] == "session_closed"
                    && line["session_id"] == started["session_id"])
                .count(),
            1
        );
        assert!(
            !self
                .runtime
                .snapshot(self.clock.as_ref())
                .active
                .iter()
                .any(|entry| Some(entry.session_id.as_str()) == started["session_id"].as_str())
        );
        closed
    }

    fn assert_empty(&self) {
        let snapshot = self.runtime.snapshot(self.clock.as_ref());
        assert!(snapshot.waiting.is_empty());
        assert!(snapshot.active.is_empty());
        assert_eq!(snapshot.target_connecting, 0);
        for budget in [&self.budget, &self.waiting_budget] {
            let permits: Vec<_> = (0..8)
                .map(|_| {
                    budget
                        .try_acquire(8)
                        .unwrap_or_else(|| panic!("all connection and waiting permits released"))
                })
                .collect();
            assert!(budget.try_acquire(8).is_none());
            drop(permits);
        }
    }

    async fn finish(self) {
        self.service.request_shutdown(&self.runtime, Duration::ZERO);
        self.stop.send_replace(true);
        for task in self.tasks {
            bounded(task)
                .await
                .unwrap_or_else(|error| panic!("listener completes: {error}"));
        }
    }
}

enum Peer {
    Tcp(TcpStream),
    WebSocket(Client),
}

impl Peer {
    async fn connect(running: &Running, transport: Transport, server: bool) -> Self {
        if transport == Transport::Tcp {
            let mut stream = bounded(TcpStream::connect(if server {
                running.server_addr
            } else {
                running.viewer_addr
            }))
            .await
            .unwrap_or_else(|e| panic!("TCP peer connect: {e}"));
            if !server {
                assert_eq!(
                    read_tcp(&mut stream, VIEWER_GREETING.len()).await,
                    VIEWER_GREETING
                );
            }
            return Self::Tcp(stream);
        }
        let secure = transport == Transport::Wss;
        let mut client = Client::transport(
            if secure {
                running.wss_addr
            } else {
                running.ws_addr
            },
            secure,
        )
        .await;
        let response = client
            .http(&upgrade(
                "GET",
                if server {
                    "/relay/server"
                } else {
                    "/relay/viewer"
                },
                "13",
                true,
            ))
            .await;
        assert!(response.starts_with("HTTP/1.1 101 "), "{response}");
        if server {
            let mut byte = [0; 1];
            assert!(
                timeout(Duration::from_millis(30), client.wire.read(&mut byte))
                    .await
                    .is_err(),
                "Server receives no greeting"
            );
        } else {
            client.greeting().await;
        }
        Self::WebSocket(client)
    }

    async fn send(&mut self, bytes: &[u8]) {
        match self {
            Self::Tcp(stream) => bounded(stream.write_all(bytes))
                .await
                .unwrap_or_else(|e| panic!("TCP peer write: {e}")),
            Self::WebSocket(client) => client.binary(bytes).await,
        }
    }

    async fn data(&mut self, len: usize) -> Vec<u8> {
        match self {
            Self::Tcp(stream) => read_tcp(stream, len).await,
            Self::WebSocket(client) => client.data(len).await,
        }
    }

    async fn rejected(&mut self, code: u16, reason: &str) {
        match self {
            Self::Tcp(stream) => tcp_eof(stream).await,
            Self::WebSocket(client) => {
                let frame = client.frame().await;
                assert_eq!(frame.opcode, 8);
                assert_eq!(&frame.payload[..2], &code.to_be_bytes());
                assert_eq!(&frame.payload[2..], reason.as_bytes());
                client.close(Some(code)).await;
                client.eof().await;
            }
        }
    }
}

/// spec: websocket.md:WS-028
#[tokio::test]
async fn websocket_servers_pair_with_every_viewer_transport_in_both_orders() {
    for server_transport in [Transport::Ws, Transport::Wss] {
        for viewer_transport in [Transport::Tcp, Transport::Ws, Transport::Wss] {
            for server_first in [false, true] {
                let running = Running::start(false).await;
                let (first_transport, first_server) = if server_first {
                    (server_transport, true)
                } else {
                    (viewer_transport, false)
                };
                let mut first = Peer::connect(&running, first_transport, first_server).await;
                first.send(&request("ID:28001")).await;
                let registered = running
                    .events
                    .wait(|line| line["event"] == "waiting_registered")
                    .await;
                assert_eq!(
                    registered["role"],
                    if server_first { "server" } else { "viewer" }
                );
                let mut second = Peer::connect(
                    &running,
                    if server_first {
                        viewer_transport
                    } else {
                        server_transport
                    },
                    !server_first,
                )
                .await;
                second.send(&request("ID:28001")).await;
                let started = running
                    .events
                    .wait(|line| line["event"] == "session_started")
                    .await;
                assert_eq!(started["mode"], "mode2");
                assert_eq!(started["viewer_transport"], viewer_transport.as_str());
                assert_eq!(started["server_transport"], server_transport.as_str());
                let query = vnc_relay_app::management::query::ManagementQueryService::new(
                    running.runtime.clone(),
                    running.budget.clone(),
                    ConnectionBudget::new(),
                    running.clock.clone(),
                    running.cell.clone(),
                );
                let sessions = query.sessions();
                assert_eq!(sessions.items.len(), 1);
                assert_eq!(
                    sessions.items[0].target_transport,
                    server_transport.as_str()
                );
                assert_eq!(
                    sessions.items[0].viewer_transport,
                    viewer_transport.as_str()
                );
                let (viewer, server) = if server_first {
                    (&mut second, &mut first)
                } else {
                    (&mut first, &mut second)
                };
                let forward = b"viewer-to-server\0\xff";
                viewer.send(forward).await;
                assert_eq!(server.data(forward.len()).await, forward);
                let reverse = b"server-to-viewer\xff\0";
                server.send(reverse).await;
                assert_eq!(viewer.data(reverse.len()).await, reverse);
                drop(first);
                drop(second);
                running.finish().await;
            }
        }
    }
}

async fn same_role_duplicates_share_one_id_space_across_all_transports() {
    for server in [false, true] {
        for first_transport in [Transport::Tcp, Transport::Ws, Transport::Wss] {
            for next_transport in [Transport::Tcp, Transport::Ws, Transport::Wss] {
                let running = Running::start(false).await;
                let mut first = Peer::connect(&running, first_transport, server).await;
                first.send(&request("ID:28002")).await;
                let registered = running
                    .events
                    .wait(|line| line["event"] == "waiting_registered")
                    .await;
                assert_eq!(registered["role"], if server { "server" } else { "viewer" });
                let mut next = Peer::connect(&running, next_transport, server).await;
                next.send(&request("ID:28002")).await;
                next.rejected(4004, "duplicate_id").await;
                let rejected = running
                    .events
                    .wait(|line| {
                        line["event"] == "connection_rejected" && line["reason"] == "duplicate_id"
                    })
                    .await;
                assert_eq!(rejected["role"], if server { "server" } else { "viewer" });
                let snapshot = running.runtime.snapshot(running.clock.as_ref());
                assert_eq!(snapshot.waiting.len(), 1);
                assert!(snapshot.active.is_empty());
                assert!(
                    !lock(&running.events.lines)
                        .iter()
                        .any(|line| line["event"] == "session_started")
                );
                drop(next);
                drop(first);
                running.finish().await;
            }
        }
    }
}

async fn websocket_server_paths_refuse_mode_i_without_registration_or_target_connect() {
    for transport in [Transport::Ws, Transport::Wss] {
        let running = Running::start(false).await;
        let target = target().await;
        let mut server = Peer::connect(&running, transport, true).await;
        server
            .send(&request(
                &target
                    .local_addr()
                    .unwrap_or_else(|error| panic!("target address: {error}"))
                    .to_string(),
            ))
            .await;
        server.rejected(4008, "handshake_invalid").await;
        assert!(
            timeout(Duration::from_millis(30), target.accept())
                .await
                .is_err()
        );
        let rejected = running
            .events
            .wait(|line| {
                line["event"] == "connection_rejected" && line["reason"] == "handshake_invalid"
            })
            .await;
        assert_eq!(rejected["role"], "server");
        assert_eq!(
            rejected["listener"],
            if transport == Transport::Wss {
                "wss"
            } else {
                "ws"
            }
        );
        running.assert_empty();
        drop(server);
        running.finish().await;
    }
}

/// spec: relay.md:RLY-028
#[tokio::test]
async fn websocket_server_rules_cover_mode_i_and_waiting_and_active_duplicates() {
    websocket_server_paths_refuse_mode_i_without_registration_or_target_connect().await;
    same_role_duplicates_share_one_id_space_across_all_transports().await;
    for server_transport in [Transport::Ws, Transport::Wss] {
        for duplicate_server in [false, true] {
            for duplicate_transport in [Transport::Tcp, Transport::Ws, Transport::Wss] {
                let running = Running::start(false).await;
                let mut server = Peer::connect(&running, server_transport, true).await;
                server.send(&request("ID:28003")).await;
                running
                    .events
                    .wait(|line| line["event"] == "waiting_registered")
                    .await;
                let mut viewer = Peer::connect(&running, Transport::Tcp, false).await;
                viewer.send(&request("ID:28003")).await;
                running
                    .events
                    .wait(|line| line["event"] == "session_started")
                    .await;
                let mut duplicate =
                    Peer::connect(&running, duplicate_transport, duplicate_server).await;
                duplicate.send(&request("ID:28003")).await;
                duplicate.rejected(4004, "duplicate_id").await;
                let snapshot = running.runtime.snapshot(running.clock.as_ref());
                assert_eq!(snapshot.active.len(), 1);
                assert!(snapshot.waiting.is_empty());
                viewer.send(b"existing-viewer").await;
                assert_eq!(server.data(15).await, b"existing-viewer");
                server.send(b"existing-server").await;
                assert_eq!(viewer.data(15).await, b"existing-server");
                drop(duplicate);
                drop(viewer);
                drop(server);
                running.finish().await;
            }
        }
    }
}

async fn role_pair(
    running: &Running,
    viewer_transport: Transport,
    server_transport: Transport,
) -> (Peer, Peer, Value) {
    let mut server = Peer::connect(running, server_transport, true).await;
    server.send(&request("ID:28003")).await;
    running
        .events
        .wait(|line| line["event"] == "waiting_registered")
        .await;
    let mut viewer = Peer::connect(running, viewer_transport, false).await;
    viewer.send(&request("ID:28003")).await;
    let started = running
        .events
        .wait(|line| line["event"] == "session_started")
        .await;
    (viewer, server, started)
}

/// spec: websocket.md:WS-016
/// spec: websocket.md:WS-028 partial
#[tokio::test]
async fn tcp_viewer_and_websocket_server_drain_data_on_server_close_or_viewer_eof() {
    for server_transport in [Transport::Ws, Transport::Wss] {
        for server_closes in [false, true] {
            let running = Running::start(false).await;
            let (viewer, server, started) =
                role_pair(&running, Transport::Tcp, server_transport).await;
            let Peer::Tcp(mut viewer) = viewer else {
                panic!("TCP Viewer");
            };
            let Peer::WebSocket(mut server) = server else {
                panic!("WebSocket Server");
            };
            bounded(viewer.write_all(b"viewer trailing bytes"))
                .await
                .unwrap();
            assert_eq!(server.data(21).await, b"viewer trailing bytes");
            server.binary(b"server trailing bytes").await;
            assert_eq!(read_tcp(&mut viewer, 21).await, b"server trailing bytes");
            if server_closes {
                server.close(Some(1000)).await;
                tcp_eof(&mut viewer).await;
                bounded(viewer.shutdown()).await.unwrap();
                server.expect_close(1000).await;
            } else {
                bounded(viewer.shutdown()).await.unwrap();
                server.expect_close(1000).await;
                server.binary(b"last response").await;
                assert_eq!(read_tcp(&mut viewer, 13).await, b"last response");
                server.close(Some(1000)).await;
                tcp_eof(&mut viewer).await;
            }
            server.eof().await;
            let closed = running
                .closed(
                    &started,
                    "peer_eof",
                    if server_closes { "server" } else { "viewer" },
                )
                .await;
            assert_eq!(closed["bytes_viewer_to_target"], 21);
            assert_eq!(
                closed["bytes_target_to_viewer"],
                if server_closes { 21 } else { 34 }
            );
            running.assert_empty();
            running.finish().await;
        }
    }
}

/// spec: websocket.md:WS-016
/// spec: websocket.md:WS-028 partial
#[tokio::test]
async fn websocket_pairs_complete_normal_close_from_either_role_without_control_bytes() {
    for server_transport in [Transport::Ws, Transport::Wss] {
        for viewer_transport in [Transport::Ws, Transport::Wss] {
            for server_closes in [false, true] {
                let running = Running::start(false).await;
                let (viewer, server, started) =
                    role_pair(&running, viewer_transport, server_transport).await;
                let Peer::WebSocket(mut viewer) = viewer else {
                    panic!("WebSocket Viewer");
                };
                let Peer::WebSocket(mut server) = server else {
                    panic!("WebSocket Server");
                };
                viewer.binary(b"viewer bytes").await;
                assert_eq!(server.data(12).await, b"viewer bytes");
                server.binary(b"server bytes").await;
                assert_eq!(viewer.data(12).await, b"server bytes");
                let (initiator, responder) = if server_closes {
                    (&mut server, &mut viewer)
                } else {
                    (&mut viewer, &mut server)
                };
                initiator.close(Some(1000)).await;
                responder.expect_close(1000).await;
                responder.close(Some(1000)).await;
                initiator.expect_close(1000).await;
                viewer.eof().await;
                server.eof().await;
                let closed = running
                    .closed(
                        &started,
                        "peer_eof",
                        if server_closes { "server" } else { "viewer" },
                    )
                    .await;
                assert_eq!(closed["bytes_viewer_to_target"], 12);
                assert_eq!(closed["bytes_target_to_viewer"], 12);
                running.assert_empty();
                running.finish().await;
            }
        }
    }
}

async fn control_after_answering_pings(client: &mut Client) -> Frame {
    loop {
        let frame = client.frame().await;
        if frame.opcode != 9 {
            return frame;
        }
        assert!(frame.payload.is_empty());
        client.send(10, true, &frame.payload).await;
    }
}

/// spec: websocket.md:WS-009
/// spec: websocket.md:WS-026 partial
#[tokio::test]
async fn server_role_handles_fragmented_data_control_frames_and_idle_pongs() {
    for server_transport in [Transport::Ws, Transport::Wss] {
        let keepalive = KeepaliveSettings {
            ws_ping: Duration::from_millis(200),
            ws_pong_timeout: Duration::from_millis(300),
            ..KEEPALIVE
        };
        let running = Running::start_with_keepalive(false, keepalive, LIMIT).await;
        let (viewer, server, started) = role_pair(&running, Transport::Tcp, server_transport).await;
        let Peer::Tcp(mut viewer) = viewer else {
            panic!("TCP Viewer");
        };
        let Peer::WebSocket(mut server) = server else {
            panic!("WebSocket Server");
        };
        server.send(2, false, b"fragment-one").await;
        server.send(9, true, b"server-ping\0\xff").await;
        let pong = control_after_answering_pings(&mut server).await;
        assert_eq!(pong.opcode, 10);
        assert_eq!(pong.payload, b"server-ping\0\xff");
        server.send(10, true, b"unsolicited-server-pong").await;
        server.send(0, true, b"-fragment-two").await;
        assert_eq!(
            read_tcp(&mut viewer, 25).await,
            b"fragment-one-fragment-two"
        );
        let until = Instant::now() + Duration::from_millis(800);
        let mut pings = 0;
        while Instant::now() < until {
            let ping = server.frame().await;
            assert_eq!(ping.opcode, 9);
            assert!(ping.payload.is_empty());
            server.send(10, true, &ping.payload).await;
            pings += 1;
        }
        assert!(pings >= 3);
        assert_eq!(
            running
                .runtime
                .snapshot(running.clock.as_ref())
                .active
                .len(),
            1
        );
        let mut byte = [0; 1];
        assert!(
            timeout(Duration::from_millis(30), viewer.read(&mut byte))
                .await
                .is_err(),
            "Server ping/pong bytes never reach TCP Viewer"
        );
        bounded(viewer.write_all(b"still alive")).await.unwrap();
        assert_eq!(server.data(11).await, b"still alive");
        server.close(Some(1000)).await;
        tcp_eof(&mut viewer).await;
        bounded(viewer.shutdown()).await.unwrap();
        let close = control_after_answering_pings(&mut server).await;
        assert_eq!(close.opcode, 8);
        assert_eq!(close.payload, 1000_u16.to_be_bytes());
        server.eof().await;
        let closed = running.closed(&started, "peer_eof", "server").await;
        assert_eq!(closed["bytes_viewer_to_target"], 11);
        assert_eq!(closed["bytes_target_to_viewer"], 25);
        running.assert_empty();
        running.finish().await;
    }
}

/// spec: websocket.md:WS-026 partial
#[tokio::test]
async fn unresponsive_websocket_server_releases_tcp_viewer_and_session() {
    for server_transport in [Transport::Ws, Transport::Wss] {
        let running = Running::start_with_keepalive(
            false,
            KeepaliveSettings {
                ws_ping: Duration::from_millis(100),
                ws_pong_timeout: Duration::from_millis(200),
                ..KEEPALIVE
            },
            LIMIT,
        )
        .await;
        let (viewer, server, started) = role_pair(&running, Transport::Tcp, server_transport).await;
        let Peer::Tcp(mut viewer) = viewer else {
            panic!("TCP Viewer");
        };
        let Peer::WebSocket(mut server) = server else {
            panic!("WebSocket Server");
        };
        assert!(server.kw_eof().await > 0);
        tcp_eof(&mut viewer).await;
        running.closed(&started, "dead_connection", "server").await;
        running.assert_empty();
        running.finish().await;
    }
}

fn update_settings(running: &Running, update: impl FnOnce(&mut vnc_relay_app::settings::Settings)) {
    let current = running.cell.current();
    let mut settings = (*current.settings).clone();
    update(&mut settings);
    running.cell.replace(ReloadSnapshot::new(
        vnc_relay_app::reload::ConfigCandidate {
            settings,
            presence: (*current.presence).clone(),
        },
        current.wss_tls.clone(),
        current.management_tls.clone(),
    ));
}

/// spec: websocket.md:WS-029
#[tokio::test]
async fn exact_enabled_paths_select_roles_and_rejections_never_register() {
    for secure in [false, true] {
        let running = Running::start(secure).await;
        for (path, server) in [
            ("/relay/viewer?ignored=1", false),
            ("/relay/server?ignored=1", true),
        ] {
            let mut client = Client::transport(running.addr, secure).await;
            assert!(
                client
                    .http(&upgrade("GET", path, "13", true))
                    .await
                    .starts_with("HTTP/1.1 101 ")
            );
            if !server {
                client.greeting().await;
            }
            client
                .binary(&request(if server { "ID:29001" } else { "ID:29002" }))
                .await;
            let role = if server { "server" } else { "viewer" };
            running
                .events
                .wait(|line| line["event"] == "waiting_registered" && line["role"] == role)
                .await;
            drop(client);
        }
        running.finish().await;
        for disabled_server in [false, true] {
            let running = Running::start_configured(secure, KEEPALIVE, LIMIT, |settings| {
                let listener = if secure {
                    &mut settings.relay.wss
                } else {
                    &mut settings.relay.ws
                };
                listener.viewer.enabled = disabled_server;
                listener.server.enabled = !disabled_server;
            })
            .await;
            for path in [
                if disabled_server {
                    "/relay/server"
                } else {
                    "/relay/viewer"
                },
                "/",
                "/relay",
                "/relay/",
                "/relay/viewer/",
                "/relay/server/",
                "/relay/Viewer",
                "/relay%2Fviewer",
                "/relay%2Fserver",
                "/api/v1/sessions",
            ] {
                let mut client = Client::transport(running.addr, secure).await;
                let before = lock(&running.events.lines).len();
                assert!(
                    client
                        .http(&upgrade("GET", path, "13", true))
                        .await
                        .starts_with("HTTP/1.1 404 ")
                );
                client.eof().await;
                let lines = lock(&running.events.lines);
                let rejected = lines[before..]
                    .iter()
                    .find(|line| line["event"] == "connection_rejected")
                    .unwrap();
                assert_eq!(rejected["reason"], "invalid_upgrade");
                assert!(rejected.get("role").is_none());
                assert!(
                    !lines[before..]
                        .iter()
                        .any(|line| line["event"] == "waiting_registered"
                            || line["event"] == "session_started")
                );
            }
            running.finish().await;
        }
    }
}

/// spec: websocket.md:WS-030
#[tokio::test]
async fn accept_union_does_not_replace_server_role_ip_check() {
    for secure in [false, true] {
        let running = Running::start(secure).await;
        update_settings(&running, |settings| {
            let listener = if secure {
                &mut settings.relay.wss
            } else {
                &mut settings.relay.ws
            };
            listener.viewer.allow = Some(AllowList::new(vec![IpItem::parse("127.0.0.1").unwrap()]));
            listener.server.allow = Some(AllowList::new(Vec::new()));
        });
        let mut client = Client::transport(running.addr, secure).await;
        assert!(
            client
                .http(&upgrade("GET", "/relay/server", "13", true))
                .await
                .starts_with("HTTP/1.1 403 ")
        );
        client.eof().await;
        let rejected = running
            .events
            .wait(|line| line["event"] == "connection_rejected" && line["stage"] == "upgrade")
            .await;
        assert_eq!(rejected["reason"], "ip_not_allowed");
        assert_eq!(rejected["role"], "server");
        update_settings(&running, |settings| {
            let listener = if secure {
                &mut settings.relay.wss
            } else {
                &mut settings.relay.ws
            };
            listener.viewer.allow = Some(AllowList::new(Vec::new()));
            listener.trusted_proxies = AllowList::new(Vec::new());
        });
        let mut socket = bounded(TcpStream::connect(running.addr)).await.unwrap();
        let mut bytes = Vec::new();
        match bounded(socket.read_to_end(&mut bytes)).await {
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::ConnectionReset => {}
            other => panic!("accept refusal before TLS/HTTP: {other:?}"),
        }
        assert!(bytes.is_empty());
        running
            .events
            .wait(|line| {
                line["event"] == "connection_rejected"
                    && line["stage"] == "accept"
                    && line["reason"] == "ip_not_allowed"
            })
            .await;
        assert!(!lock(&running.events.lines).iter().any(|line| line["event"]
            == "waiting_registered"
            || line["event"] == "session_started"));
        running.assert_empty();
        running.finish().await;
    }
}

/// spec: websocket.md:WS-031
#[tokio::test]
async fn server_path_uses_only_server_origin_policy_on_ws_and_wss() {
    for secure in [false, true] {
        let running = Running::start(secure).await;
        update_settings(&running, |settings| {
            let listener = if secure {
                &mut settings.relay.wss
            } else {
                &mut settings.relay.ws
            };
            listener.viewer.allowed_origins =
                vec![vnc_relay_domain::origin::Origin::parse("https://viewer.example").unwrap()];
            listener.server.allowed_origins =
                vec![vnc_relay_domain::origin::Origin::parse("https://server.example").unwrap()];
        });
        for (origin, status) in [
            ("https://server.example", 101),
            ("https://viewer.example", 403),
            ("https://other.example", 403),
        ] {
            let mut client = Client::transport(running.addr, secure).await;
            let head = String::from_utf8(upgrade("GET", "/relay/server", "13", true)).unwrap();
            let head = head.strip_suffix("\r\n").unwrap().to_owned()
                + &format!("Origin: {origin}\r\n\r\n");
            let before = lock(&running.events.lines).len();
            assert!(
                client
                    .http(head.as_bytes())
                    .await
                    .starts_with(&format!("HTTP/1.1 {status} "))
            );
            if status == 101 {
                client.binary(&request("ID:31001")).await;
                let registered = running
                    .events
                    .wait(|line| line["event"] == "waiting_registered")
                    .await;
                assert_eq!(registered["role"], "server");
            } else {
                client.eof().await;
                let lines = lock(&running.events.lines);
                let rejected = lines[before..]
                    .iter()
                    .find(|line| line["event"] == "connection_rejected")
                    .unwrap();
                assert_eq!(rejected["reason"], "origin_not_allowed");
                assert_eq!(rejected["role"], "server");
                assert_eq!(rejected["origin"], origin);
                assert!(
                    !lines[before..]
                        .iter()
                        .any(|line| line["event"] == "waiting_registered"
                            || line["event"] == "session_started")
                );
            }
        }
        running.finish().await;
    }
}

trait Wire: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Wire for T {}

struct Client {
    wire: Box<dyn Wire>,
    secure: bool,
}
struct FrameHeader {
    fin: bool,
    opcode: u8,
    len: usize,
}
struct Frame {
    fin: bool,
    opcode: u8,
    payload: Vec<u8>,
}

impl Client {
    async fn transport(addr: SocketAddr, secure: bool) -> Self {
        let tcp = bounded(TcpStream::connect(addr))
            .await
            .unwrap_or_else(|error| panic!("viewer connects: {error}"));
        let wire: Box<dyn Wire> = if secure {
            Box::new(
                bounded(
                    connector().connect(
                        ServerName::try_from("localhost")
                            .unwrap_or_else(|error| panic!("TLS hostname: {error}")),
                        tcp,
                    ),
                )
                .await
                .unwrap_or_else(|error| panic!("verified TLS: {error}")),
            )
        } else {
            Box::new(tcp)
        };
        Self { wire, secure }
    }

    async fn http(&mut self, request: &[u8]) -> String {
        bounded(self.wire.write_all(request))
            .await
            .unwrap_or_else(|error| panic!("HTTP request: {error}"));
        bounded(async {
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                head.push(
                    self.wire
                        .read_u8()
                        .await
                        .unwrap_or_else(|error| panic!("HTTP response: {error}")),
                );
                assert!(head.len() <= 16 * 1024);
            }
            String::from_utf8(head).unwrap_or_else(|error| panic!("HTTP text: {error}"))
        })
        .await
    }

    async fn greeting(&mut self) {
        bounded(async {
            let mut frame = self.frame().await;
            assert_eq!(frame.opcode, 2, "greeting is one binary message");
            while !frame.fin {
                let continuation = self.frame().await;
                assert_eq!(continuation.opcode, 0, "greeting message continuation");
                frame.fin = continuation.fin;
                frame.payload.extend_from_slice(&continuation.payload);
                assert!(frame.payload.len() <= VIEWER_GREETING.len());
            }
            assert_eq!(frame.payload, VIEWER_GREETING);
        })
        .await;
    }

    async fn send(&mut self, opcode: u8, fin: bool, bytes: &[u8]) {
        let mut frame = masked_header(opcode, fin, bytes.len());
        frame.extend(
            bytes
                .iter()
                .enumerate()
                .map(|(i, byte)| byte ^ [0x13, 0x57, 0x9b, 0xdf][i % 4]),
        );
        bounded(self.wire.write_all(&frame))
            .await
            .unwrap_or_else(|error| panic!("masked frame: {error}"));
    }

    async fn binary(&mut self, bytes: &[u8]) {
        self.send(2, true, bytes).await;
    }
    async fn close(&mut self, code: Option<u16>) {
        self.send(
            8,
            true,
            &code.map_or_else(Vec::new, |code| code.to_be_bytes().to_vec()),
        )
        .await;
    }

    async fn header(&mut self) -> FrameHeader {
        bounded(async {
            let first = self
                .wire
                .read_u8()
                .await
                .unwrap_or_else(|error| panic!("frame header: {error}"));
            assert_eq!(first & 0x70, 0);
            let second = self
                .wire
                .read_u8()
                .await
                .unwrap_or_else(|error| panic!("frame length: {error}"));
            assert_eq!(second & 0x80, 0);
            let len = match second & 0x7f {
                126 => usize::from(
                    self.wire
                        .read_u16()
                        .await
                        .unwrap_or_else(|error| panic!("16-bit length: {error}")),
                ),
                127 => usize::try_from(
                    self.wire
                        .read_u64()
                        .await
                        .unwrap_or_else(|error| panic!("64-bit length: {error}")),
                )
                .unwrap_or_else(|error| panic!("length fits usize: {error}")),
                len => usize::from(len),
            };
            let opcode = first & 15;
            let fin = first & 0x80 != 0;
            assert!(len <= M);
            if opcode >= 8 {
                assert!(fin, "control frames cannot be fragmented");
                assert!(len <= 125, "control payload limit");
            }
            FrameHeader { fin, opcode, len }
        })
        .await
    }

    async fn frame(&mut self) -> Frame {
        let header = self.header().await;
        let mut payload = vec![0; header.len];
        bounded(self.wire.read_exact(&mut payload))
            .await
            .unwrap_or_else(|error| panic!("frame payload: {error}"));
        Frame {
            fin: header.fin,
            opcode: header.opcode,
            payload,
        }
    }

    async fn data(&mut self, len: usize) -> Vec<u8> {
        bounded(async {
            let mut bytes = Vec::new();
            let mut fragmented = false;
            let mut message_len = 0;
            while bytes.len() < len || fragmented {
                let frame = self.frame().await;
                match frame.opcode {
                    2 => assert!(!fragmented, "binary cannot interrupt an unfinished message"),
                    0 => assert!(fragmented, "continuation requires a binary message"),
                    9 => {
                        self.send(10, true, &frame.payload).await;
                        continue;
                    }
                    10 => continue,
                    opcode => panic!("expected binary data or continuation, got {opcode}"),
                }
                message_len += frame.payload.len();
                assert!(message_len <= M, "logical server message limit");
                fragmented = !frame.fin;
                if frame.fin {
                    message_len = 0;
                }
                bytes.extend_from_slice(&frame.payload);
                assert!(bytes.len() <= len, "no extra bytes");
            }
            bytes
        })
        .await
    }

    async fn expect_close(&mut self, code: u16) {
        let frame = self.frame().await;
        assert_eq!(frame.opcode, 8);
        assert_eq!(frame.payload, code.to_be_bytes());
    }

    async fn eof(&mut self) {
        let mut byte = [0; 1];
        match bounded(self.wire.read(&mut byte)).await {
            Ok(0) => {}
            Err(error) if error.kind() == io::ErrorKind::ConnectionReset => {}
            Err(error) if self.secure && error.kind() == io::ErrorKind::UnexpectedEof => {}
            other => panic!("no data or frames after Close, transport EOF: {other:?}"),
        }
    }

    /// Reads raw frames without the automatic Pong of a WebSocket library.
    async fn kw_eof(&mut self) -> usize {
        timeout(Duration::from_secs(20), async {
            let mut pings = 0;
            loop {
                let mut byte = [0; 1];
                match self.wire.read(&mut byte).await {
                    Ok(0) => return pings,
                    Err(error)
                        if error.kind() == io::ErrorKind::ConnectionReset
                            || (self.secure && error.kind() == io::ErrorKind::UnexpectedEof) =>
                    {
                        return pings;
                    }
                    Err(error) => panic!("Kw transport: {error}"),
                    Ok(_) => {
                        assert_eq!(
                            byte[0], 0x89,
                            "only empty Ping; no Close on own Pong expiry"
                        );
                        assert_eq!(
                            self.wire
                                .read_u8()
                                .await
                                .unwrap_or_else(|error| panic!("empty Ping length: {error}")),
                            0
                        );
                        pings += 1;
                    }
                }
            }
        })
        .await
        .unwrap_or_else(|error| panic!("Kw did not reclaim transport: {error}"))
    }
}

fn masked_header(opcode: u8, fin: bool, len: usize) -> Vec<u8> {
    let mut bytes = vec![opcode | if fin { 0x80 } else { 0 }];
    if len < 126 {
        bytes
            .push(0x80 | u8::try_from(len).unwrap_or_else(|error| panic!("small length: {error}")));
    } else if let Ok(len) = u16::try_from(len) {
        bytes.push(0x80 | 126);
        bytes.extend_from_slice(&len.to_be_bytes());
    } else {
        bytes.push(0x80 | 127);
        bytes.extend_from_slice(
            &u64::try_from(len)
                .unwrap_or_else(|error| panic!("large length: {error}"))
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

fn upgrade(method: &str, path: &str, version: &str, key: bool) -> Vec<u8> {
    format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n{}Sec-WebSocket-Version: {version}\r\n\r\n",
        if key { "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n" } else { "" }).into_bytes()
}

async fn target() -> TcpListener {
    TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap_or_else(|error| panic!("ephemeral bind: {error}"))
}

async fn read_tcp(peer: &mut TcpStream, len: usize) -> Vec<u8> {
    let mut bytes = vec![0; len];
    bounded(peer.read_exact(&mut bytes))
        .await
        .unwrap_or_else(|error| panic!("TCP bytes: {error}"));
    bytes
}

async fn tcp_eof(peer: &mut TcpStream) {
    let mut byte = [0; 1];
    assert_eq!(
        bounded(peer.read(&mut byte))
            .await
            .unwrap_or_else(|error| panic!("TCP EOF: {error}")),
        0,
        "no control, handshake, or post-Close bytes reach TCP"
    );
}

async fn close_normally(
    running: &Running,
    client: &mut Client,
    peer: &mut TcpStream,
    started: &Value,
) -> Value {
    client.close(Some(1000)).await;
    tcp_eof(peer).await;
    bounded(peer.shutdown())
        .await
        .unwrap_or_else(|error| panic!("target EOF: {error}"));
    client.expect_close(1000).await;
    client.eof().await;
    running.closed(started, "peer_eof", "viewer").await
}

fn payload(seed: u8, len: usize) -> Vec<u8> {
    (0..len).map(|index| (index % 251) as u8 ^ seed).collect()
}

/// spec: websocket.md:WS-007
#[tokio::test]
async fn message_and_fragment_boundaries_preserve_the_same_stream_on_ws_and_wss() {
    for secure in [false, true] {
        let running = Running::start(secure).await;
        let bytes = payload(0x71, M);
        for layout in 0..3 {
            let (mut client, mut peer, started) = running.pair(layout == 2, "7001").await;
            match layout {
                0 => client.binary(&bytes).await,
                1 => {
                    client.binary(&[]).await;
                    for part in bytes.chunks(997) {
                        client.binary(part).await;
                    }
                }
                _ => {
                    client.send(2, false, &bytes[..13]).await;
                    client.send(0, false, &[]).await;
                    client.send(0, false, &bytes[13..B + 27]).await;
                    client.send(0, true, &bytes[B + 27..]).await;
                    client.binary(&[]).await;
                }
            }
            assert_eq!(read_tcp(&mut peer, bytes.len()).await, bytes);
            let reply = payload(0xa7, M + B + 19);
            for part in reply.chunks(if layout == 0 { B } else { 419 }) {
                bounded(peer.write_all(part))
                    .await
                    .unwrap_or_else(|error| panic!("TCP chunk: {error}"));
            }
            assert_eq!(client.data(reply.len()).await, reply);
            let closed = close_normally(&running, &mut client, &mut peer, &started).await;
            assert_eq!(closed["bytes_viewer_to_target"], bytes.len());
            assert_eq!(closed["bytes_target_to_viewer"], reply.len());
            running.assert_empty();
        }
        running.finish().await;
    }
}

/// spec: websocket.md:WS-009
#[tokio::test]
async fn fragmented_binary_with_ping_pong_and_close_never_leaks_control_bytes_to_tcp() {
    for secure in [false, true] {
        for mode2 in [false, true] {
            let running = Running::start(secure).await;
            let (mut client, mut peer, started) = running.pair(mode2, "9001").await;
            client.send(2, false, b"fragment-one").await;
            client.send(9, true, b"ping-control\0\xff").await;
            let pong = client.frame().await;
            assert_eq!(pong.opcode, 10);
            assert_eq!(pong.payload, b"ping-control\0\xff");
            client.send(10, true, b"unsolicited-pong-control").await;
            client.send(0, false, b"-fragment-two").await;
            client.send(9, true, &[]).await;
            let pong = client.frame().await;
            assert_eq!(pong.opcode, 10);
            assert!(pong.payload.is_empty());
            client.send(0, true, b"-fragment-three").await;
            let expected = b"fragment-one-fragment-two-fragment-three";
            assert_eq!(read_tcp(&mut peer, expected.len()).await, expected);
            bounded(peer.write_all(b"reverse binary"))
                .await
                .unwrap_or_else(|error| panic!("reverse data: {error}"));
            assert_eq!(client.data(14).await, b"reverse binary");
            let closed = close_normally(&running, &mut client, &mut peer, &started).await;
            assert_eq!(closed["bytes_viewer_to_target"], expected.len());
            assert_eq!(closed["bytes_target_to_viewer"], 14);
            running.assert_empty();
            running.finish().await;
        }
    }
}

/// spec: websocket.md:WS-012
#[tokio::test]
async fn management_paths_and_invalid_upgrades_cannot_register_or_connect_a_target() {
    for secure in [false, true] {
        let running = Running::start(secure).await;
        let target = target().await;
        let addr = target
            .local_addr()
            .unwrap_or_else(|error| panic!("target address: {error}"));
        let cases = [
            (upgrade("GET", "/api/v1/session", "13", true), 404),
            (upgrade("GET", "/api/v1/sessions", "13", true), 404),
            (upgrade("GET", "/relay", "13", true), 404),
            (upgrade("GET", "/relay/viewer/", "13", true), 404),
            (upgrade("GET", "/relay/viewer", "13", false), 400),
            (upgrade("POST", "/relay/viewer", "13", true), 405),
            (upgrade("GET", "/relay/viewer", "8", true), 426),
            (
                b"GET /relay/viewer HTTP/1.1\r\nHost: localhost\r\nBroken Header\r\n\r\n".to_vec(),
                400,
            ),
        ];
        for (index, (mut head, status)) in cases.into_iter().enumerate() {
            // Pipeline syntactically valid Mode I AND Mode II requests after a
            // refused upgrade. Neither suffix may be treated as relay input.
            for value in [addr.to_string(), "ID:12001".to_owned()] {
                let mut frame = masked_header(2, true, HANDSHAKE_REQUEST_LEN);
                frame.extend(
                    request(&value)
                        .iter()
                        .enumerate()
                        .map(|(i, byte)| byte ^ [0x13, 0x57, 0x9b, 0xdf][i % 4]),
                );
                head.extend_from_slice(&frame);
            }
            let mut client = Client::transport(running.addr, secure).await;
            let response = client.http(&head).await;
            assert!(
                response.starts_with(&format!("HTTP/1.1 {status} ")),
                "{response}"
            );
            assert!(!response.contains("101 Switching"));
            if status == 405 {
                assert!(response.contains("Allow: GET\r\n"));
            }
            if status == 426 {
                assert!(response.contains("Sec-WebSocket-Version: 13\r\n"));
            }
            client.eof().await;
            let rejected = running
                .events
                .wait(|line| {
                    line["event"] == "connection_rejected"
                        && line["stage"] == "upgrade"
                        && line["reason"] == "invalid_upgrade"
                        && lock(&running.events.lines)
                            .iter()
                            .filter(|entry| entry["event"] == "connection_rejected")
                            .count()
                            > index
                })
                .await;
            assert_eq!(rejected["listener"], if secure { "wss" } else { "ws" });
            running.assert_empty();
            assert!(
                !lock(&running.events.lines)
                    .iter()
                    .any(|line| line["event"] == "waiting_registered"
                        || line["event"] == "session_started"
                        || line["event"] == "target_connect_failed")
            );
            assert!(
                timeout(Duration::from_millis(30), target.accept())
                    .await
                    .is_err(),
                "rejected request did not open the real target"
            );
        }
        // Admission and the formerly pipelined ID remain usable.
        let (mut client, mut peer, started) = running.pair(true, "12001").await;
        client.binary(b"valid after refused upgrade").await;
        assert_eq!(
            read_tcp(&mut peer, 27).await,
            b"valid after refused upgrade"
        );
        close_normally(&running, &mut client, &mut peer, &started).await;
        running.assert_empty();
        running.finish().await;
    }
}

/// spec: websocket.md:WS-021
#[tokio::test]
async fn allowed_non_browser_client_relays_without_origin_cookie_authorization_or_login() {
    for secure in [false, true] {
        for mode2 in [false, true] {
            let running = Running::start(secure).await;
            // The only request headers are visible in upgrade(): no Origin,
            // Cookie, Authorization, or management session exists in this harness.
            let (mut client, mut peer, started) = running.pair(mode2, "21001").await;
            let accepted = running
                .events
                .wait(|line| {
                    line["event"] == "connection_accepted"
                        && line["conn_id"] == started["viewer_conn_id"]
                })
                .await;
            assert_eq!(accepted["peer_ip"], "127.0.0.1");
            assert!(accepted.get("role").is_none());
            assert_eq!(accepted["listener"], if secure { "wss" } else { "ws" });
            client.binary(b"no administrator login").await;
            assert_eq!(read_tcp(&mut peer, 22).await, b"no administrator login");
            bounded(peer.write_all(b"anonymous relay reply"))
                .await
                .unwrap_or_else(|error| panic!("TCP reply: {error}"));
            assert_eq!(client.data(21).await, b"anonymous relay reply");
            let closed = close_normally(&running, &mut client, &mut peer, &started).await;
            assert_eq!(closed["bytes_viewer_to_target"], 22);
            assert_eq!(closed["bytes_target_to_viewer"], 21);
            running.assert_empty();
            running.finish().await;
        }
    }
}

/// spec: websocket.md:WS-016
#[tokio::test]
async fn close_from_either_end_drains_accepted_data_and_records_the_first_cause() {
    for secure in [false, true] {
        for mode2 in [false, true] {
            for client_first in [false, true] {
                for code in [None, Some(3001)] {
                    let running = Running::start(secure).await;
                    let (mut client, mut peer, started) = running.pair(mode2, "16001").await;
                    client.binary(b"accepted viewer bytes").await;
                    if client_first {
                        let mut close =
                            code.map_or_else(Vec::new, |code: u16| code.to_be_bytes().to_vec());
                        if code.is_some() {
                            close.extend_from_slice(b"private client reason");
                        }
                        client.send(8, true, &close).await;
                        assert_eq!(read_tcp(&mut peer, 21).await, b"accepted viewer bytes");
                        tcp_eof(&mut peer).await;
                        client.expect_close(code.unwrap_or(1000)).await;
                        // Close was already parsed (echo observed): no new TCP
                        // data frame may be started, even while TCP is writable.
                        bounded(peer.write_all(b"discard after close"))
                            .await
                            .unwrap_or_else(|error| panic!("post-Close TCP write: {error}"));
                        bounded(peer.shutdown())
                            .await
                            .unwrap_or_else(|error| panic!("target EOF: {error}"));
                        client.eof().await;
                    } else {
                        bounded(peer.write_all(b"accepted target bytes"))
                            .await
                            .unwrap_or_else(|error| panic!("target data: {error}"));
                        bounded(peer.shutdown())
                            .await
                            .unwrap_or_else(|error| panic!("target EOF: {error}"));
                        assert_eq!(client.data(21).await, b"accepted target bytes");
                        client.expect_close(1000).await;
                        // A server Close after EOF must not reject client data
                        // that arrives before the client's closing response.
                        client.binary(b"after server close").await;
                        assert_eq!(
                            read_tcp(&mut peer, 39).await,
                            b"accepted viewer bytesafter server close"
                        );
                        client.close(code).await;
                        tcp_eof(&mut peer).await;
                        client.eof().await;
                    }
                    let closed = running
                        .closed(
                            &started,
                            "peer_eof",
                            if client_first {
                                "viewer"
                            } else if mode2 {
                                "server"
                            } else {
                                "target"
                            },
                        )
                        .await;
                    assert_eq!(
                        closed["bytes_viewer_to_target"],
                        if client_first { 21 } else { 39 }
                    );
                    assert_eq!(
                        closed["bytes_target_to_viewer"],
                        if client_first { 0 } else { 21 }
                    );
                    assert!(
                        !lock(&running.events.lines)
                            .iter()
                            .any(|line| line.to_string().contains("private client reason")
                                || line.to_string().contains("discard after close"))
                    );
                    running.assert_empty();
                    let (mut next, mut next_peer, next_started) =
                        running.pair(mode2, "16001").await;
                    assert_ne!(next_started["session_id"], started["session_id"]);
                    next.binary(b"same ID new lifetime").await;
                    assert_eq!(read_tcp(&mut next_peer, 20).await, b"same ID new lifetime");
                    close_normally(&running, &mut next, &mut next_peer, &next_started).await;
                    running.assert_empty();
                    running.finish().await;
                }
            }
        }
        finishes_started_reverse_frame_before_echoing_close(secure).await;
    }
}

async fn survivor_exchange(client: &mut Client, peer: &mut TcpStream) {
    client.binary(b"unaffected forward").await;
    assert_eq!(read_tcp(peer, 18).await, b"unaffected forward");
    bounded(peer.write_all(b"unaffected reverse"))
        .await
        .unwrap_or_else(|error| panic!("survivor reverse: {error}"));
    assert_eq!(client.data(18).await, b"unaffected reverse");
}

/// spec: websocket.md:WS-017
#[tokio::test]
async fn disconnect_during_transfer_and_missing_close_responses_release_only_the_failed_session() {
    for secure in [false, true] {
        for failure in 0..3 {
            let running = Running::start(secure).await;
            let (mut client, mut peer, started) = running.pair(true, "17001").await;
            let (mut other, mut other_peer, survivor) = running.pair(true, "17002").await;
            survivor_exchange(&mut other, &mut other_peer).await;
            let (done, finished) = watch::channel(false);
            let cleanup = async {
                if failure == 0 {
                    // Disconnect in the middle of a real data frame: the header
                    // promises M bytes but only an observed prefix is delivered.
                    let mut frame = masked_header(2, true, M);
                    frame.extend((0..B).map(|_| 0x41 ^ 0x13));
                    // Use a repeated mask to make the known prefix exact.
                    let header = frame.len() - B;
                    for index in 0..B {
                        frame[header + index] = 0x41 ^ [0x13, 0x57, 0x9b, 0xdf][index % 4];
                    }
                    bounded(client.wire.write_all(&frame))
                        .await
                        .unwrap_or_else(|error| panic!("in-flight prefix: {error}"));
                    assert_eq!(read_tcp(&mut peer, B).await, vec![0x41; B]);
                    drop(client);
                    let closed = running.closed(&started, "connection_error", "viewer").await;
                    assert_eq!(closed["error_kind"], "io_error");
                    assert_eq!(closed["bytes_viewer_to_target"], B);
                    let mut byte = [0; 1];
                    match bounded(peer.read(&mut byte)).await {
                        Ok(0) => {}
                        Err(error) if error.kind() == io::ErrorKind::ConnectionReset => {}
                        other => panic!("failed peer closes: {other:?}"),
                    }
                } else {
                    client.binary(b"before timeout").await;
                    assert_eq!(read_tcp(&mut peer, 14).await, b"before timeout");
                    let began;
                    if failure == 1 {
                        began = Instant::now();
                        bounded(peer.shutdown())
                            .await
                            .unwrap_or_else(|error| panic!("target EOF: {error}"));
                        client.expect_close(1000).await;
                        // No client Close response.
                    } else {
                        began = Instant::now();
                        client.close(Some(1000)).await;
                        tcp_eof(&mut peer).await;
                        client.expect_close(1000).await;
                        // No TCP EOF response to the client's Close.
                    }
                    client.eof().await;
                    let ended = Instant::now();
                    let closed = running
                        .closed(
                            &started,
                            "close_timeout",
                            if failure == 1 { "server" } else { "viewer" },
                        )
                        .await;
                    assert_eq!(closed["bytes_viewer_to_target"], 14);
                    let elapsed = ended.duration_since(began);
                    assert!(elapsed >= E, "E did not expire early: {elapsed:?}");
                    assert!(
                        elapsed < E + Duration::from_secs(3),
                        "bounded E cleanup: {elapsed:?}"
                    );
                }
                done.send_replace(true);
            };
            let flowing = async {
                // Keep the independent session active during the entire cleanup,
                // not merely after the failure has already been removed.
                let mut rounds = 0;
                while rounds < 32 || !*finished.borrow() {
                    survivor_exchange(&mut other, &mut other_peer).await;
                    rounds += 1;
                    tokio::task::yield_now().await;
                }
            };
            bounded(async { tokio::join!(cleanup, flowing) }).await;
            let snapshot = running.runtime.snapshot(running.clock.as_ref());
            assert!(snapshot.waiting.is_empty());
            assert_eq!(snapshot.active.len(), 1);
            assert_eq!(
                Some(snapshot.active[0].session_id.as_str()),
                survivor["session_id"].as_str()
            );
            assert!(
                !lock(&running.events.lines)
                    .iter()
                    .any(|line| line["event"] == "session_closed"
                        && line["session_id"] == survivor["session_id"])
            );
            survivor_exchange(&mut other, &mut other_peer).await;
            close_normally(&running, &mut other, &mut other_peer, &survivor).await;
            running.assert_empty();
            let (mut next, mut next_peer, next_started) = running.pair(true, "17001").await;
            survivor_exchange(&mut next, &mut next_peer).await;
            close_normally(&running, &mut next, &mut next_peer, &next_started).await;
            running.assert_empty();
            running.finish().await;
        }
    }
}

/// spec: websocket.md:WS-019
#[tokio::test]
async fn invalid_frame_sequences_and_controls_close_1002_and_release_registrations() {
    for secure in [false, true] {
        // Continuation without a message; a new data message before FIN;
        // fragmented/oversized control; invalid Close bodies and reserved code.
        for invalid in 0..7 {
            let running = Running::start(secure).await;
            let (mut client, mut peer, started) = running.pair(true, "19001").await;
            let prefix = if invalid == 1 {
                client.send(2, false, b"valid prefix").await;
                assert_eq!(read_tcp(&mut peer, 12).await, b"valid prefix");
                12
            } else {
                0
            };
            match invalid {
                0 => client.send(0, true, b"orphan").await,
                1 => client.send(2, true, b"new message before FIN").await,
                2 => client.send(9, false, b"fragmented ping").await,
                3 => client.send(10, true, &[0x41; 126]).await,
                4 => client.send(8, true, &[1]).await,
                5 => client.send(8, true, &1005_u16.to_be_bytes()).await,
                _ => client.send(8, true, &[3, 232, 0xff]).await,
            }
            client.expect_close(1002).await;
            // Finish raw-input disposal; only the WebSocket error determines
            // the cause, never a cleanup EOF.
            let _ = bounded(client.wire.shutdown()).await;
            client.eof().await;
            let mut byte = [0; 1];
            match bounded(peer.read(&mut byte)).await {
                Ok(0) => {}
                Err(error) if error.kind() == io::ErrorKind::ConnectionReset => {}
                other => panic!("protocol-error peer is closed without control bytes: {other:?}"),
            }
            let closed = running.closed(&started, "connection_error", "viewer").await;
            assert_eq!(closed["error_kind"], "ws_protocol_error");
            assert_eq!(closed["bytes_viewer_to_target"], prefix);
            assert_eq!(closed["bytes_target_to_viewer"], 0);
            running.assert_empty();
            let (mut next, mut next_peer, next_started) = running.pair(true, "19001").await;
            survivor_exchange(&mut next, &mut next_peer).await;
            close_normally(&running, &mut next, &mut next_peer, &next_started).await;
            running.assert_empty();
            running.finish().await;
        }
    }
}

// Queue measurements use the same real service/runtime after the public
// entrypoint exchange below. WsEntryPoint owns its concrete transport type,
// so a second real accepted WS/WSS socket is wrapped at the public RelayStream
// boundary, just as RLY-018 instruments accepted TCP sockets. No payload source
// is mocked. Gating the recipient write makes OS socket-buffer sizes irrelevant.
#[derive(Default)]
struct Measurements {
    reads: [usize; 2],
    writes: [usize; 2],
    peaks: [usize; 2],
    armed: bool,
    gated: bool,
    writers: [Option<Waker>; 2],
}

#[derive(Default)]
struct Meter {
    state: Mutex<Measurements>,
    changed: Notify,
}

impl Meter {
    async fn saturated(&self) {
        bounded(async {
            loop {
                let changed = self.changed.notified();
                {
                    let state = lock(&self.state);
                    assert_eq!(state.writes, [0, 0], "both recipients are delayed");
                    if state.writers.iter().all(Option::is_some) {
                        assert!(state.reads.iter().all(|read| *read > 0 && *read <= B));
                        assert_eq!(state.peaks, state.reads);
                        return;
                    }
                }
                changed.await;
            }
        })
        .await;
    }

    fn release(&self) {
        let writers = {
            let mut state = lock(&self.state);
            state.gated = false;
            [state.writers[0].take(), state.writers[1].take()]
        };
        for writer in writers.into_iter().flatten() {
            writer.wake();
        }
    }
}

struct MeasuredStream<S> {
    inner: S,
    side: usize,
    meter: Arc<Meter>,
    frame_gate: Option<Arc<FrameGate>>,
}

impl<S: AsyncRead + Unpin> AsyncRead for MeasuredStream<S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let before = buffer.filled().len();
        let result = Pin::new(&mut this.inner).poll_read(cx, buffer);
        let read = buffer.filled().len() - before;
        if read == 0
            && matches!(result, Poll::Ready(Ok(())))
            && let Some(gate) = &this.frame_gate
        {
            lock(&gate.state)
                .close_seen
                .get_or_insert_with(Instant::now);
            gate.changed.notify_one();
        }
        let mut state = lock(&this.meter.state);
        if state.armed && read > 0 {
            state.reads[this.side] += read;
            let queued = state.reads[this.side]
                .checked_sub(state.writes[this.side])
                .unwrap_or_else(|| panic!("payload cannot be written before being read"));
            state.peaks[this.side] = state.peaks[this.side].max(queued);
            assert!(queued <= B, "unwritten payload {queued} exceeds B={B}");
            this.meter.changed.notify_one();
        }
        result
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for MeasuredStream<S> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        {
            let mut state = lock(&this.meter.state);
            if state.armed && state.gated {
                state.writers[this.side] = Some(cx.waker().clone());
                this.meter.changed.notify_one();
                return Poll::Pending;
            }
        }
        let result = Pin::new(&mut this.inner).poll_write(cx, bytes);
        if let Poll::Ready(Ok(written)) = result {
            let mut state = lock(&this.meter.state);
            if state.armed {
                state.writes[1 - this.side] += written;
                assert!(state.writes[1 - this.side] <= state.reads[1 - this.side]);
            }
        }
        result
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}

impl<S: RelayStream> RelayStream for MeasuredStream<S> {
    fn transport(&self) -> Transport {
        self.inner.transport()
    }
    fn pending_write_len(&self) -> usize {
        self.inner.pending_write_len()
    }
    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        action: RelayCloseAction,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_close(cx, action)
    }
}

async fn measured_pair(
    running: &Running,
    meter: Arc<Meter>,
    frame_gate: Option<Arc<FrameGate>>,
) -> (Client, TcpStream, Value, Vec<JoinHandle<()>>) {
    let listener = target().await;
    let address = listener
        .local_addr()
        .unwrap_or_else(|error| panic!("measured address: {error}"));
    let viewer = async {
        let (tcp, addr) = listener
            .accept()
            .await
            .unwrap_or_else(|error| panic!("measured viewer accepted: {error}"));
        let wire: Box<dyn Wire> = if running.secure {
            Box::new(
                acceptor()
                    .accept(tcp)
                    .await
                    .unwrap_or_else(|error| panic!("measured TLS: {error}")),
            )
        } else {
            Box::new(tcp)
        };
        (wire, addr)
    };
    let (client, (wire, addr)) =
        bounded(async { tokio::join!(Client::transport(address, running.secure), viewer) }).await;
    let wire: Box<dyn Wire> = if let Some(gate) = &frame_gate {
        Box::new(FrameGatedWire {
            inner: wire,
            gate: Arc::clone(gate),
        })
    } else {
        wire
    };
    let now = running.clock.monotonic();
    let viewer_context = ConnectionContext {
        conn_id: ConnId::next(),
        entry_point: if running.secure {
            EntryPoint::WssViewer
        } else {
            EntryPoint::WsViewer
        },
        peer_ip: PeerIp::from(addr),
        forwarded_ip: None,
        peer_addr: addr,
        accepted_wall: running.clock.wall(),
        accepted_at: now,
        exchange_at: now,
        buffer_bytes: B,
        handshake: Duration::from_secs(3),
    };
    let viewer_stream = MeasuredStream {
        inner: WsRelayStream::new(
            wire,
            Vec::new(),
            M,
            B,
            if running.secure {
                Transport::Wss
            } else {
                Transport::Ws
            },
            KEEPALIVE,
        ),
        side: 0,
        meter: Arc::clone(&meter),
        frame_gate,
    };
    let viewer_permit = running
        .budget
        .try_acquire(8)
        .unwrap_or_else(|| panic!("measured viewer admission"));
    let prepare = running.service.handle_connection(
        viewer_stream,
        viewer_context,
        viewer_permit,
        running.runtime.lifecycle(),
    );
    let send = async {
        let mut client = client;
        client.greeting().await;
        client.binary(&request("ID:11001")).await;
        client
    };
    let (viewer, client) = bounded(async { tokio::join!(prepare, send) }).await;
    let viewer = viewer.unwrap_or_else(|error| panic!("measured viewer prepared: {error:?}"));
    assert_eq!(viewer.buffered_bytes(), 0);

    let mut peer = bounded(TcpStream::connect(address))
        .await
        .unwrap_or_else(|error| panic!("measured Server connects: {error}"));
    let (tcp, addr) = bounded(listener.accept())
        .await
        .unwrap_or_else(|error| panic!("measured Server accepted: {error}"));
    let now = running.clock.monotonic();
    let server_context = ConnectionContext {
        conn_id: ConnId::next(),
        entry_point: EntryPoint::TcpServer,
        peer_ip: PeerIp::from(addr),
        forwarded_ip: None,
        peer_addr: addr,
        accepted_wall: running.clock.wall(),
        accepted_at: now,
        exchange_at: now,
        buffer_bytes: B,
        handshake: Duration::from_secs(3),
    };
    let server_stream = MeasuredStream {
        inner: vnc_relay_adapters::net::TcpRelayStream::with_keepalive(tcp, &KEEPALIVE)
            .unwrap_or_else(|error| panic!("measured Server keepalive: {error}")),
        side: 1,
        meter: Arc::clone(&meter),
        frame_gate: None,
    };
    let server_permit = running
        .budget
        .try_acquire(8)
        .unwrap_or_else(|| panic!("measured Server admission"));
    let prepare = running.service.handle_connection(
        server_stream,
        server_context,
        server_permit,
        running.runtime.lifecycle(),
    );
    let send = async {
        bounded(peer.write_all(&request("ID:11001")))
            .await
            .unwrap_or_else(|error| panic!("measured Server request: {error}"));
    };
    let (server, ()) = bounded(async { tokio::join!(prepare, send) }).await;
    let server = server.unwrap_or_else(|error| panic!("measured Server prepared: {error:?}"));
    assert_eq!(server.buffered_bytes(), 0);
    {
        let mut state = lock(&meter.state);
        state.armed = true;
        state.gated = true;
    }
    let first = lock(&running.events.lines).len();
    let service = Arc::clone(&running.service);
    let runtime = Arc::clone(&running.runtime);
    let viewer_task = tokio::spawn(async move {
        service
            .dispatch(viewer, &runtime)
            .await
            .unwrap_or_else(|error| panic!("measured viewer dispatch: {error:?}"));
    });
    let service = Arc::clone(&running.service);
    let runtime = Arc::clone(&running.runtime);
    let server_task = tokio::spawn(async move {
        service
            .dispatch(server, &runtime)
            .await
            .unwrap_or_else(|error| panic!("measured Server dispatch: {error:?}"));
    });
    let started = running
        .events
        .wait(|line| {
            line["event"] == "session_started"
                && lock(&running.events.lines)[first..]
                    .iter()
                    .any(|new| new == line)
        })
        .await;
    (client, peer, started, vec![viewer_task, server_task])
}

/// spec: websocket.md:WS-011
#[tokio::test]
async fn delayed_tcp_and_ws_receivers_bound_both_queues_at_b_and_resume_without_loss() {
    for secure in [false, true] {
        let running = Running::start(secure).await;
        let (mut client, mut peer, started) = running.pair(true, "11000").await;
        let forward = payload(0x11, B * 2);
        let reverse = payload(0x91, B * 2);
        // Both actual entrypoint recipients are deliberately unread until their
        // sender has completed. Socket buffers are not mistaken for application B.
        client.binary(&forward).await;
        bounded(peer.write_all(&reverse))
            .await
            .unwrap_or_else(|error| panic!("delayed WS prefix: {error}"));
        assert_eq!(read_tcp(&mut peer, forward.len()).await, forward);
        assert_eq!(client.data(reverse.len()).await, reverse);
        close_normally(&running, &mut client, &mut peer, &started).await;
        running.assert_empty();

        let meter = Arc::new(Meter::default());
        let (mut client, mut peer, started, tasks) =
            measured_pair(&running, Arc::clone(&meter), None).await;
        let forward = payload(0x31, M);
        let reverse = payload(0xb1, M + 173);
        // The recipient gate is active before any payload exists. Supply more
        // than B to each real socket so a hidden unbounded queue cannot pass.
        client.binary(&forward).await;
        bounded(peer.write_all(&reverse))
            .await
            .unwrap_or_else(|error| panic!("measured reverse: {error}"));
        meter.saturated().await;
        meter.release();
        let (got_forward, got_reverse) = bounded(async {
            tokio::join!(
                read_tcp(&mut peer, forward.len()),
                client.data(reverse.len())
            )
        })
        .await;
        assert_eq!(got_forward, forward);
        assert_eq!(got_reverse, reverse);
        let closed = close_normally(&running, &mut client, &mut peer, &started).await;
        assert_eq!(closed["bytes_viewer_to_target"], forward.len());
        assert_eq!(closed["bytes_target_to_viewer"], reverse.len());
        {
            let state = lock(&meter.state);
            assert_eq!(state.reads, [forward.len(), reverse.len()]);
            assert_eq!(state.writes, state.reads);
            assert!(state.peaks.iter().all(|peak| *peak > 0 && *peak <= B));
        }
        for task in tasks {
            bounded(task)
                .await
                .unwrap_or_else(|error| panic!("measured owner completes: {error}"));
        }
        running.assert_empty();
        running.finish().await;
    }
}

#[derive(Default)]
struct FrameGateState {
    armed: bool,
    header_remaining: Option<usize>,
    blocked: bool,
    released: bool,
    writer: Option<Waker>,
    close_seen: Option<Instant>,
}

#[derive(Default)]
struct FrameGate {
    state: Mutex<FrameGateState>,
    changed: Notify,
}

impl FrameGate {
    async fn wait(&self, condition: impl Fn(&FrameGateState) -> bool) {
        bounded(async {
            loop {
                let changed = self.changed.notified();
                if condition(&lock(&self.state)) {
                    return;
                }
                changed.await;
            }
        })
        .await;
    }

    fn release(&self) {
        let writer = {
            let mut state = lock(&self.state);
            state.released = true;
            state.writer.take()
        };
        if let Some(writer) = writer {
            writer.wake();
        }
    }
}

// This gate is above the real TCP/TLS wire, below WsRelayStream. It passes
// exactly one complete header, flushes it to the client, and holds every
// payload byte until the real WS decoder has observed the client's Close.
struct FrameGatedWire {
    inner: Box<dyn Wire>,
    gate: Arc<FrameGate>,
}

impl AsyncRead for FrameGatedWire {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_read(cx, buffer)
    }
}

impl AsyncWrite for FrameGatedWire {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        let remaining = {
            let mut state = lock(&this.gate.state);
            if !state.armed || state.released {
                None
            } else {
                Some(*state.header_remaining.get_or_insert_with(|| {
                    assert!(bytes.len() >= 2, "first output contains the frame header");
                    match bytes[1] & 0x7f {
                        126 => 4,
                        127 => 10,
                        _ => 2,
                    }
                }))
            }
        };
        if remaining == Some(0) {
            // In WSS the header may still be in the TLS writer. Flush it while
            // preserving the pending application-payload write.
            if let Poll::Ready(Err(error)) = Pin::new(&mut this.inner).poll_flush(cx) {
                return Poll::Ready(Err(error));
            }
            let mut state = lock(&this.gate.state);
            state.blocked = true;
            state.writer = Some(cx.waker().clone());
            this.gate.changed.notify_one();
            return Poll::Pending;
        }
        let count = remaining.map_or(bytes.len(), |remaining| remaining.min(bytes.len()));
        let result = Pin::new(&mut this.inner).poll_write(cx, &bytes[..count]);
        if let (Some(remaining), Poll::Ready(Ok(written))) = (remaining, &result) {
            lock(&this.gate.state).header_remaining = Some(remaining - written);
        }
        result
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}

async fn finishes_started_reverse_frame_before_echoing_close(secure: bool) {
    for code in [None, Some(3001)] {
        let running = Running::start(secure).await;
        let meter = Arc::new(Meter::default());
        let gate = Arc::new(FrameGate::default());
        let (mut client, mut peer, started, tasks) =
            measured_pair(&running, Arc::clone(&meter), Some(Arc::clone(&gate))).await;
        meter.release();
        client.binary(b"accepted before gated Close").await;
        assert_eq!(
            read_tcp(&mut peer, 27).await,
            b"accepted before gated Close"
        );
        lock(&gate.state).armed = true;
        let offered = payload(0x83, B);
        bounded(peer.write_all(&offered))
            .await
            .unwrap_or_else(|error| panic!("start reverse frame: {error}"));
        let header = client.header().await;
        assert_eq!(header.opcode, 2);
        assert!(header.len > 0 && header.len <= offered.len());
        gate.wait(|state| state.blocked).await;
        client.close(code).await;
        gate.wait(|state| state.close_seen.is_some()).await;
        let close_seen = lock(&gate.state)
            .close_seen
            .unwrap_or_else(|| panic!("real WS decoder observed Close"));
        let discarded = b"TCP bytes written after decoded Close";
        bounded(peer.write_all(discarded))
            .await
            .unwrap_or_else(|error| panic!("post-Close TCP input: {error}"));
        gate.release();
        let mut received = vec![0; header.len];
        bounded(client.wire.read_exact(&mut received))
            .await
            .unwrap_or_else(|error| panic!("started frame completes: {error}"));
        assert_eq!(received, offered[..header.len]);
        // The very next frame must be the matching Close: neither the unread
        // original TCP suffix nor newly received TCP bytes may start a frame.
        client.expect_close(code.unwrap_or(1000)).await;
        assert!(
            close_seen.elapsed() < E,
            "started frame and echo finish within E"
        );
        tcp_eof(&mut peer).await;
        bounded(peer.shutdown())
            .await
            .unwrap_or_else(|error| panic!("post-Close TCP EOF: {error}"));
        client.eof().await;
        let closed = running.closed(&started, "peer_eof", "viewer").await;
        assert_eq!(closed["bytes_viewer_to_target"], 27);
        assert_eq!(closed["bytes_target_to_viewer"], header.len);
        {
            let state = lock(&meter.state);
            assert_eq!(
                state.reads[1],
                offered.len() + discarded.len(),
                "post-Close TCP bytes were actually read and discarded"
            );
            assert_eq!(
                state.writes[1], header.len,
                "only the started frame was delivered"
            );
        }
        for task in tasks {
            bounded(task)
                .await
                .unwrap_or_else(|error| panic!("gated stream owner completes: {error}"));
        }
        running.assert_empty();
        let (mut next, mut next_peer, next_started) = running.pair(true, "11001").await;
        survivor_exchange(&mut next, &mut next_peer).await;
        close_normally(&running, &mut next, &mut next_peer, &next_started).await;
        running.assert_empty();
        running.finish().await;
    }
}

#[tokio::test]
async fn raw_reader_accepts_fragmented_binary_messages_with_interleaved_control_frames() {
    let listener = target().await;
    let address = listener
        .local_addr()
        .unwrap_or_else(|error| panic!("reader test address: {error}"));
    let mut client = Client::transport(address, false).await;
    let (mut peer, _) = bounded(listener.accept())
        .await
        .unwrap_or_else(|error| panic!("reader test accept: {error}"));
    let mut frames = vec![0x02, 4];
    frames.extend_from_slice(&VIEWER_GREETING[..4]);
    frames.extend_from_slice(&[0x80, 8]);
    frames.extend_from_slice(&VIEWER_GREETING[4..]);
    frames.extend_from_slice(b"\x02\x03abc\x89\x01p\x00\x02de\x80\x01f\x82\x02gh");
    bounded(peer.write_all(&frames))
        .await
        .unwrap_or_else(|error| panic!("reader fixture frames: {error}"));
    client.greeting().await;
    assert_eq!(client.data(8).await, b"abcdefgh");
    let mut pong = [0; 7];
    bounded(peer.read_exact(&mut pong))
        .await
        .unwrap_or_else(|error| panic!("reader answered interleaved Ping: {error}"));
    assert_eq!(pong[..2], [0x8a, 0x81], "FIN Pong with one masked byte");
    assert_eq!(pong[6] ^ pong[2], b'p');
}

/// spec: websocket.md:WS-026
#[tokio::test]
async fn ws_and_wss_kw_keeps_idle_peers_and_drops_unresponsive_peers() {
    for secure in [false, true] {
        let keepalive = KeepaliveSettings {
            ws_ping: Duration::from_secs(5),
            ws_pong_timeout: Duration::from_secs(5),
            ..KEEPALIVE
        };
        let running =
            Running::start_with_keepalive(secure, keepalive, Duration::from_secs(60)).await;
        // Exact timing is covered by paused-time unit tests. These socket
        // windows allow two seconds early and ten seconds late; the latter
        // exceeds the 3.3-second scheduling overruns observed in the WSL gate.
        let silent_at = Instant::now();
        let (mut silent, mut silent_tcp, silent_started) = running.pair(true, "8261").await;
        let silent_task = tokio::spawn(async move {
            let pings = silent.kw_eof().await;
            let elapsed = silent_at.elapsed();
            assert!(
                (Duration::from_secs(8)..=Duration::from_secs(20)).contains(&elapsed),
                "silent client Kw window: {elapsed:?}"
            );
            pings
        });
        let live_at = Instant::now();
        let (mut live, mut live_tcp, live_started) = running.pair(true, "8262").await;
        // Begin answering immediately, before setting up any later connection.
        let live_task = tokio::spawn(async move {
            for _ in 0..3 {
                let frame = live.frame().await;
                assert_eq!(frame.opcode, 9);
                assert!(frame.payload.is_empty());
                live.send(10, true, &frame.payload).await;
            }
            let elapsed = live_at.elapsed();
            assert!(
                (Duration::from_secs(13)..=Duration::from_secs(25)).contains(&elapsed),
                "idle client's three-Ping window: {elapsed:?}"
            );
            live
        });
        let waiting_at = Instant::now();
        let mut waiting = running.connect().await;
        waiting.binary(&request("ID:8263")).await;
        let waiting_task = tokio::spawn(async move {
            let pings = waiting.kw_eof().await;
            let elapsed = waiting_at.elapsed();
            assert!(
                (Duration::from_secs(8)..=Duration::from_secs(20)).contains(&elapsed),
                "waiting client Kw window: {elapsed:?}"
            );
            pings
        });
        let registered = running
            .events
            .wait(|line| {
                line["event"] == "waiting_registered"
                    && line["conn_id"] != silent_started["viewer_conn_id"]
                    && line["conn_id"] != live_started["viewer_conn_id"]
            })
            .await;
        // No Ping header/payload may reach TCP, even on the dead session.
        let mut byte = [0; 1];
        assert_eq!(
            timeout(Duration::from_secs(20), silent_tcp.read(&mut byte))
                .await
                .unwrap_or_else(|error| panic!("dead TCP reclaimed: {error}"))
                .unwrap_or_else(|error| panic!("dead TCP EOF: {error}")),
            0
        );
        let closed = running
            .closed(&silent_started, "dead_connection", "viewer")
            .await;
        assert_eq!(closed["bytes_viewer_to_target"], 0);
        assert_eq!(closed["bytes_target_to_viewer"], 0);
        assert!(
            silent_task
                .await
                .unwrap_or_else(|error| panic!("silent client: {error}"))
                >= 1
        );
        assert!(
            waiting_task
                .await
                .unwrap_or_else(|error| panic!("waiting client: {error}"))
                >= 1
        );
        let removed = running
            .events
            .wait(|line| {
                line["event"] == "waiting_removed" && line["conn_id"] == registered["conn_id"]
            })
            .await;
        assert_eq!(removed["reason"], "dead_connection");
        assert_eq!(removed["level"], "warn");
        let mut live = timeout(Duration::from_secs(20), live_task)
            .await
            .unwrap_or_else(|error| panic!("idle Pong client: {error}"))
            .unwrap_or_else(|error| panic!("idle Pong task: {error}"));
        assert_eq!(
            running
                .runtime
                .snapshot(running.clock.as_ref())
                .active
                .len(),
            1
        );
        let mut byte = [0; 1];
        assert!(
            timeout(Duration::from_millis(10), live_tcp.read(&mut byte))
                .await
                .is_err(),
            "server Ping bytes never appear in TCP"
        );
        live.binary(b"still alive").await;
        assert_eq!(read_tcp(&mut live_tcp, 11).await, b"still alive");
        bounded(live_tcp.write_all(b"idle survived"))
            .await
            .unwrap_or_else(|error| panic!("live reverse probe: {error}"));
        assert_eq!(live.data(13).await, b"idle survived");
        close_normally(&running, &mut live, &mut live_tcp, &live_started).await;
        running.assert_empty();
        running.finish().await;
    }
}

fn ip_list(items: &[&str]) -> AllowList {
    AllowList::new(
        items
            .iter()
            .map(|item| {
                IpItem::parse(item).unwrap_or_else(|error| panic!("valid ip item: {error:?}"))
            })
            .collect(),
    )
}

fn forwarded_upgrade(server: bool, headers: &str) -> Vec<u8> {
    let head = String::from_utf8(upgrade(
        "GET",
        if server {
            "/relay/server"
        } else {
            "/relay/viewer"
        },
        "13",
        true,
    ))
    .unwrap_or_else(|error| panic!("valid utf-8 upgrade head: {error}"));
    let stripped = head
        .strip_suffix("\r\n")
        .unwrap_or_else(|| panic!("crlf ending"));
    (stripped.to_owned() + headers + "\r\n").into_bytes()
}

async fn proxy_running(secure: bool, client: &str) -> Running {
    Running::start_configured(secure, KEEPALIVE, LIMIT, |settings| {
        for listener in [&mut settings.relay.ws, &mut settings.relay.wss] {
            listener.trusted_proxies = ip_list(&["127.0.0.1", "192.0.2.254", "2001:db8:ffff::/48"]);
            listener.viewer.allow = Some(ip_list(&[client]));
            listener.server.allow = Some(ip_list(&[client]));
        }
    })
    .await
}

/// spec: websocket.md:WS-032
#[tokio::test]
async fn trusted_proxy_uses_rightmost_untrusted_canonical_address_on_both_paths() {
    for secure in [false, true] {
        for (client_ip, headers) in [
            (
                "198.51.100.7",
                "X-Forwarded-For: invalid-left, 203.0.113.99\r\nX-Forwarded-For: \t198.51.100.7 \t, 192.0.2.254\r\n",
            ),
            (
                "2001:db8::7",
                "X-Forwarded-For: unknown, [2001:db8::7], [2001:db8:ffff::1]\r\n",
            ),
            (
                "2001:db8::7",
                "X-Forwarded-For: unknown, 2001:db8::7, 2001:db8:ffff::1\r\n",
            ),
            (
                "198.51.100.7",
                "X-Forwarded-For: unknown, [::ffff:198.51.100.7], ::ffff:192.0.2.254\r\n",
            ),
        ] {
            let running = proxy_running(secure, client_ip).await;
            let mut viewer = Client::transport(running.addr, secure).await;
            assert!(
                viewer
                    .http(&forwarded_upgrade(false, headers))
                    .await
                    .starts_with("HTTP/1.1 101 ")
            );
            viewer.greeting().await;
            viewer.binary(&request("ID:32001")).await;
            let waiting = running
                .events
                .wait(|line| line["event"] == "waiting_registered")
                .await;
            assert_eq!(waiting["peer_ip"], "127.0.0.1");
            assert_eq!(waiting["forwarded_ip"], client_ip);
            let mut server = Client::transport(running.addr, secure).await;
            assert!(
                server
                    .http(&forwarded_upgrade(true, headers))
                    .await
                    .starts_with("HTTP/1.1 101 ")
            );
            server.binary(&request("ID:32001")).await;
            let started = running
                .events
                .wait(|line| line["event"] == "session_started")
                .await;
            for side in ["viewer", "server"] {
                assert_eq!(started[format!("{side}_peer_ip")], "127.0.0.1");
                assert_eq!(started[format!("{side}_forwarded_ip")], client_ip);
            }
            let query = vnc_relay_app::management::query::ManagementQueryService::new(
                running.runtime.clone(),
                running.budget.clone(),
                ConnectionBudget::new(),
                running.clock.clone(),
                running.cell.clone(),
            );
            let sessions = query.sessions();
            assert_eq!(sessions.items.len(), 1);
            let item = &sessions.items[0];
            assert_eq!(item.viewer_address.ip().to_string(), "127.0.0.1");
            assert_eq!(item.target_address.ip().to_string(), "127.0.0.1");
            assert_eq!(item.viewer_forwarded_ip.unwrap().to_string(), client_ip);
            assert_eq!(item.target_forwarded_ip.unwrap().to_string(), client_ip);
            viewer.binary(b"forward\0\xff").await;
            assert_eq!(server.data(9).await, b"forward\0\xff");
            server.binary(b"reverse\xff\0").await;
            assert_eq!(viewer.data(9).await, b"reverse\xff\0");
            {
                let lines = lock(&running.events.lines);
                for accepted in lines
                    .iter()
                    .filter(|line| line["event"] == "connection_accepted")
                {
                    assert_eq!(accepted["peer_ip"], "127.0.0.1");
                    assert!(accepted.get("forwarded_ip").is_none());
                    assert!(accepted.get("role").is_none());
                }
            }
            drop(viewer);
            drop(server);
            running.finish().await;
        }
    }
}

/// spec: websocket.md:WS-033
/// spec: websocket.md:WS-036
#[tokio::test]
async fn invalid_or_all_trusted_forwarded_chains_fail_before_origin_and_registration() {
    for secure in [false, true] {
        for server in [false, true] {
            for value in [
                None,
                Some(""),
                Some(" \t"),
                Some("198.51.100.7:5900"),
                Some("[2001:db8::7]:5900"),
                Some("fe80::1%eth0"),
                Some("[fe80::1%1]"),
                Some("unknown"),
                Some("_obfuscated"),
                Some("198.51.100.7,"),
                Some("198.51.100.7,,192.0.2.254"),
                Some("999.1.1.1"),
                Some("127.0.0.1"),
                Some("192.0.2.254, 127.0.0.1"),
                Some("[::ffff:127.0.0.1], [2001:db8:ffff::1]"),
            ] {
                let running = proxy_running(secure, "127.0.0.1").await;
                update_settings(&running, |settings| {
                    let listener = if secure {
                        &mut settings.relay.wss
                    } else {
                        &mut settings.relay.ws
                    };
                    listener.viewer.allowed_origins = vec![
                        vnc_relay_domain::origin::Origin::parse("https://allowed.example").unwrap(),
                    ];
                    listener.server.allowed_origins = listener.viewer.allowed_origins.clone();
                });
                let headers = value
                    .map_or_else(String::new, |value| format!("X-Forwarded-For: {value}\r\n"))
                    + "Origin: https://denied.example\r\n";
                let mut client = Client::transport(running.addr, secure).await;
                let response = client.http(&forwarded_upgrade(server, &headers)).await;
                assert!(
                    response.starts_with("HTTP/1.1 400 "),
                    "{value:?}: {response}"
                );
                let rejected = running
                    .events
                    .wait(|line| line["event"] == "connection_rejected")
                    .await;
                assert_eq!(rejected["reason"], "forwarded_for_invalid");
                assert_eq!(rejected["peer_ip"], "127.0.0.1");
                assert_eq!(rejected["role"], if server { "server" } else { "viewer" });
                assert!(rejected.get("forwarded_ip").is_none());
                assert!(!lock(&running.events.lines).iter().any(|line| matches!(
                    line["event"].as_str(),
                    Some("waiting_registered" | "session_started" | "target_connect_failed")
                )));
                assert!(
                    running
                        .runtime
                        .snapshot(running.clock.as_ref())
                        .waiting
                        .is_empty()
                );
                drop(client);
                running.finish().await;
            }
        }
    }
}

/// spec: websocket.md:WS-034
#[tokio::test]
async fn untrusted_peer_headers_neither_grant_access_nor_supply_forwarded_metadata() {
    for secure in [false, true] {
        for server in [false, true] {
            let running = Running::start(secure).await;
            update_settings(&running, |settings| {
                let listener = if secure {
                    &mut settings.relay.wss
                } else {
                    &mut settings.relay.ws
                };
                listener.trusted_proxies = ip_list(&["192.0.2.254"]);
                if server {
                    listener.server.allow = Some(ip_list(&["198.51.100.7"]));
                } else {
                    listener.viewer.allow = Some(ip_list(&["198.51.100.7"]));
                }
            });
            let headers = "X-Forwarded-For: 198.51.100.7\r\nForwarded: for=198.51.100.7\r\nX-Real-IP: 198.51.100.7\r\nCookie: address=198.51.100.7\r\nAuthorization: Bearer 198.51.100.7\r\n";
            let mut client = Client::transport(running.addr, secure).await;
            assert!(
                client
                    .http(&forwarded_upgrade(server, headers))
                    .await
                    .starts_with("HTTP/1.1 403 ")
            );
            let rejected = running
                .events
                .wait(|line| line["event"] == "connection_rejected")
                .await;
            assert_eq!(rejected["reason"], "ip_not_allowed");
            assert_eq!(rejected["peer_ip"], "127.0.0.1");
            assert!(rejected.get("forwarded_ip").is_none());
            drop(client);
            update_settings(&running, |settings| {
                let listener = if secure {
                    &mut settings.relay.wss
                } else {
                    &mut settings.relay.ws
                };
                listener.viewer.allow = Some(ip_list(&["127.0.0.1"]));
                listener.server.allow = Some(ip_list(&["127.0.0.1"]));
            });
            let mut client = Client::transport(running.addr, secure).await;
            assert!(
                client
                    .http(&forwarded_upgrade(
                        server,
                        "X-Forwarded-For: unknown,,bad:port\r\n"
                    ))
                    .await
                    .starts_with("HTTP/1.1 101 ")
            );
            if !server {
                client.greeting().await;
            }
            client.binary(&request("ID:34001")).await;
            let waiting = running
                .events
                .wait(|line| line["event"] == "waiting_registered")
                .await;
            assert_eq!(waiting["peer_ip"], "127.0.0.1");
            assert!(waiting.get("forwarded_ip").is_none());
            drop(client);
            running.finish().await;
        }
    }
}

/// spec: websocket.md:WS-032
#[tokio::test]
async fn derived_address_survives_origin_rejection_abort_and_target_connect_failure() {
    for secure in [false, true] {
        for outcome in ["origin", "ip", "abort", "target"] {
            let running = proxy_running(secure, "198.51.100.7").await;
            update_settings(&running, |settings| {
                let listener = if secure {
                    &mut settings.relay.wss
                } else {
                    &mut settings.relay.ws
                };
                if outcome == "origin" {
                    listener.viewer.allowed_origins = vec![
                        vnc_relay_domain::origin::Origin::parse("https://allowed.example").unwrap(),
                    ];
                }
                if outcome == "ip" {
                    listener.viewer.allow = Some(ip_list(&["203.0.113.8"]));
                }
            });
            let mut client = Some(Client::transport(running.addr, secure).await);
            let headers = if outcome == "origin" {
                "X-Forwarded-For: 198.51.100.7\r\nOrigin: https://denied.example\r\n"
            } else {
                "X-Forwarded-For: 198.51.100.7\r\n"
            };
            let response = client
                .as_mut()
                .unwrap()
                .http(&forwarded_upgrade(false, headers))
                .await;
            if outcome == "origin" || outcome == "ip" {
                assert!(response.starts_with("HTTP/1.1 403 "));
            } else {
                assert!(response.starts_with("HTTP/1.1 101 "));
                client.as_mut().unwrap().greeting().await;
                if outcome == "target" {
                    let listener = target().await;
                    let addr = listener.local_addr().unwrap();
                    drop(listener);
                    client
                        .as_mut()
                        .unwrap()
                        .binary(&request(&addr.to_string()))
                        .await;
                }
            }
            if outcome == "abort" {
                drop(client.take());
            }
            let event = running
                .events
                .wait(|line| match outcome {
                    "origin" | "ip" => line["event"] == "connection_rejected",
                    "target" => line["event"] == "target_connect_failed",
                    _ => {
                        line["event"] == "connection_aborted" && line["stage"] == "initial_exchange"
                    }
                })
                .await;
            assert_eq!(event["peer_ip"], "127.0.0.1");
            assert_eq!(event["forwarded_ip"], "198.51.100.7");
            if outcome == "origin" {
                assert_eq!(event["reason"], "origin_not_allowed");
            }
            if outcome == "ip" {
                assert_eq!(event["reason"], "ip_not_allowed");
            }
            drop(client);
            running.finish().await;
        }
    }
}

/// spec: websocket.md:WS-035
#[tokio::test]
async fn tls_terminating_proxy_adds_each_client_address_and_pipes_mode_two_both_ways() {
    let running = Running::start_configured(false, KEEPALIVE, LIMIT, |settings| {
        settings.relay.ws.trusted_proxies = ip_list(&["127.0.0.1"]);
        settings.relay.ws.viewer.allow = Some(ip_list(&["198.51.100.7"]));
        settings.relay.ws.server.allow = Some(ip_list(&["203.0.113.8"]));
    })
    .await;
    let listener = target().await;
    let address = listener.local_addr().unwrap();
    let upstream = running.addr;
    let proxy = tokio::spawn(async move {
        let mut pipes = Vec::new();
        for _ in 0..2 {
            let (socket, _) = bounded(listener.accept()).await.unwrap();
            pipes.push(tokio::spawn(async move {
                let mut external = bounded(acceptor().accept(socket)).await.unwrap();
                let mut head = Vec::new();
                while !head.ends_with(b"\r\n\r\n") {
                    head.push(bounded(external.read_u8()).await.unwrap());
                    assert!(head.len() <= 16 * 1024);
                }
                let head = String::from_utf8(head).unwrap();
                assert!(!head.to_ascii_lowercase().contains("x-forwarded-for"));
                let client = if head.starts_with("GET /relay/server ") {
                    "203.0.113.8"
                } else {
                    assert!(head.starts_with("GET /relay/viewer "));
                    "198.51.100.7"
                };
                let head = head.strip_suffix("\r\n").unwrap().to_owned()
                    + &format!("X-Forwarded-For: {client}\r\n\r\n");
                let mut internal = bounded(TcpStream::connect(upstream)).await.unwrap();
                bounded(internal.write_all(head.as_bytes())).await.unwrap();
                // This leg is deliberately plain WS. TLS exists only between client and proxy.
                let _ = bounded(tokio::io::copy_bidirectional(&mut external, &mut internal)).await;
            }));
        }
        for pipe in pipes {
            bounded(pipe).await.unwrap();
        }
    });
    let mut viewer = Client::transport(address, true).await;
    assert!(
        viewer
            .http(&upgrade("GET", "/relay/viewer", "13", true))
            .await
            .starts_with("HTTP/1.1 101 ")
    );
    viewer.greeting().await;
    viewer.binary(&request("ID:35001")).await;
    let registered = running
        .events
        .wait(|line| line["event"] == "waiting_registered")
        .await;
    assert_eq!(registered["listener"], "ws");
    assert_eq!(registered["forwarded_ip"], "198.51.100.7");
    let mut server = Client::transport(address, true).await;
    assert!(
        server
            .http(&upgrade("GET", "/relay/server", "13", true))
            .await
            .starts_with("HTTP/1.1 101 ")
    );
    server.binary(&request("ID:35001")).await;
    let started = running
        .events
        .wait(|line| line["event"] == "session_started")
        .await;
    assert_eq!(started["viewer_transport"], "ws");
    assert_eq!(started["server_transport"], "ws");
    assert_eq!(started["viewer_peer_ip"], "127.0.0.1");
    assert_eq!(started["server_peer_ip"], "127.0.0.1");
    assert_eq!(started["viewer_forwarded_ip"], "198.51.100.7");
    assert_eq!(started["server_forwarded_ip"], "203.0.113.8");
    viewer.binary(b"TLS viewer\0\xff").await;
    assert_eq!(server.data(12).await, b"TLS viewer\0\xff");
    server.binary(b"TLS server\xff\0").await;
    assert_eq!(viewer.data(12).await, b"TLS server\xff\0");
    let query = vnc_relay_app::management::query::ManagementQueryService::new(
        running.runtime.clone(),
        running.budget.clone(),
        ConnectionBudget::new(),
        running.clock.clone(),
        running.cell.clone(),
    );
    let sessions = query.sessions();
    assert_eq!(
        sessions.items[0].viewer_forwarded_ip.unwrap().to_string(),
        "198.51.100.7"
    );
    assert_eq!(
        sessions.items[0].target_forwarded_ip.unwrap().to_string(),
        "203.0.113.8"
    );
    assert_eq!(sessions.items[0].viewer_transport, "ws");
    assert_eq!(sessions.items[0].target_transport, "ws");
    drop(viewer);
    drop(server);
    running.finish().await;
    bounded(proxy).await.unwrap();
}
