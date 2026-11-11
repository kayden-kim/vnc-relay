//! Post-101 ticket bindings over real loopback WebSocket and TCP streams.
//! Upgrade signature checks are deliberately outside these partial scenarios.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use vnc_relay_adapters::clock::SystemClock;
use vnc_relay_adapters::crypto::{RelayIdKey, SessionIdKey};
use vnc_relay_adapters::net::TcpRelayStream;
use vnc_relay_adapters::ops_log::format_line;
use vnc_relay_adapters::target::TcpTargetConnector;
use vnc_relay_adapters::websocket_stream::WsRelayStream;
use vnc_relay_app::events::Event;
use vnc_relay_app::ports::{Clock, EventSink};
use vnc_relay_app::relay::{
    ConnectionContext, RelayRuntime, RelayRuntimeConfig, RelayService, TicketBinding,
};
use vnc_relay_app::reload::ConfigCandidate;
use vnc_relay_app::reload::ports::RuntimeSettingsSource;
use vnc_relay_app::reload::snapshot::{ReloadCell, ReloadSnapshot};
use vnc_relay_app::settings::KeepaliveSettings;
use vnc_relay_domain::budget::ConnectionBudget;
use vnc_relay_domain::conn_id::ConnId;
use vnc_relay_domain::entry_point::{EntryPoint, Role};
use vnc_relay_domain::handshake::{HANDSHAKE_REQUEST_LEN, HandshakePolicy, VIEWER_GREETING};
use vnc_relay_domain::peer_ip::PeerIp;
use vnc_relay_domain::relay_id::RelayId;
use vnc_relay_domain::session::{CloseCause, Transport};

const LIMIT: Duration = Duration::from_secs(10);
const B: usize = 4096;
const KEEPALIVE: KeepaliveSettings = KeepaliveSettings {
    tcp_idle: Duration::from_secs(60),
    tcp_interval: Duration::from_secs(10),
    tcp_probes: 6,
    ws_ping: Duration::from_secs(30),
    ws_pong_timeout: Duration::from_secs(30),
};

#[expect(
    clippy::expect_used,
    reason = "test helper: a missed bound fails the test"
)]
async fn bounded<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(LIMIT, future)
        .await
        .expect("bounded relay test")
}

#[derive(Default)]
struct Recorder {
    lines: Mutex<Vec<Value>>,
    changed: Notify,
}

#[expect(
    clippy::expect_used,
    reason = "test helper: a broken log line fails the test"
)]
impl EventSink for Recorder {
    fn record(&self, event: &Event) {
        self.lines.lock().expect("logs").push(
            serde_json::from_str(&format_line(event, SystemTime::UNIX_EPOCH)).expect("JSON log"),
        );
        self.changed.notify_one();
    }
}

#[expect(
    clippy::expect_used,
    reason = "test helper: a poisoned log lock fails the test"
)]
impl Recorder {
    async fn wait(&self, matches: impl Fn(&Value) -> bool) -> Value {
        bounded(async {
            loop {
                let notified = self.changed.notified();
                if let Some(line) = self
                    .lines
                    .lock()
                    .expect("logs")
                    .iter()
                    .find(|line| matches(line))
                    .cloned()
                {
                    return line;
                }
                notified.await;
            }
        })
        .await
    }
}

struct Harness {
    service: Arc<RelayService>,
    runtime: Arc<RelayRuntime<TcpTargetConnector>>,
    clock: Arc<dyn Clock>,
    budget: Arc<ConnectionBudget>,
    events: Arc<Recorder>,
    references: Arc<RelayIdKey>,
}

#[expect(
    clippy::expect_used,
    reason = "test fixture: a failed setup step fails the test"
)]
impl Harness {
    fn new(mode1_enabled: bool) -> Self {
        let mut settings = vnc_relay_adapters::config_file::parse(
            "[admin]\naccounts_file='accounts.toml'\n[management]\nlisten='127.0.0.1:8443'\n[status]\nlisten='127.0.0.1:9090'\n",
            Path::new("."),
        ).expect("settings");
        settings.timeouts.handshake = LIMIT;
        settings.timeouts.wait = LIMIT;
        settings.timeouts.close = Duration::from_millis(100);
        settings.limits.buffer_bytes = B as u32;
        settings.keepalive = KEEPALIVE;
        let source: Arc<dyn RuntimeSettingsSource> =
            Arc::new(ReloadCell::new(ReloadSnapshot::<()>::new(
                ConfigCandidate {
                    settings,
                    presence: Default::default(),
                },
                None,
                None,
            )));
        let clock: Arc<dyn Clock> = Arc::new(SystemClock::new());
        let events = Arc::new(Recorder::default());
        let budget = ConnectionBudget::new();
        let references = Arc::new(RelayIdKey::generate().expect("relay reference key"));
        let runtime = Arc::new(RelayRuntime::new(
            TcpTargetConnector::new(source.clone()),
            references.clone(),
            Arc::new(SessionIdKey::from_bytes(&[0x65; 32])),
            budget.clone(),
            ConnectionBudget::new(),
            RelayRuntimeConfig { recent_closed: 8 },
            source.clone(),
        ));
        let service = Arc::new(RelayService::new(
            HandshakePolicy {
                mode1_enabled,
                mode2_enabled: true,
            },
            source,
            clock.clone(),
            events.clone(),
        ));
        Self {
            service,
            runtime,
            clock,
            budget,
            events,
            references,
        }
    }

    async fn connect(
        &self,
        entry: EntryPoint,
        rid: Option<&str>,
        sx: Option<Duration>,
    ) -> (Peer, ConnId, JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let client = TcpStream::connect(listener.local_addr().expect("address"))
            .await
            .expect("connect");
        let (socket, peer_addr) = listener.accept().await.expect("accept");
        socket.set_nodelay(true).expect("nodelay");
        let now = self.clock.monotonic();
        let context = ConnectionContext {
            conn_id: ConnId::next(),
            entry_point: entry,
            peer_ip: PeerIp::new(peer_addr.ip()),
            forwarded_ip: None,
            peer_addr,
            accepted_wall: self.clock.wall(),
            accepted_at: now,
            exchange_at: now,
            buffer_bytes: B,
            handshake: LIMIT,
        };
        let ticket = rid.map(|rid| TicketBinding {
            jti: "integration-ticket".to_owned(),
            relay_id: RelayId::parse(rid).expect("rid"),
            session_deadline: sx.map(|duration| now + duration),
        });
        let service = self.service.clone();
        let runtime = self.runtime.clone();
        let permit = self.budget.try_acquire(8).expect("L");
        let task = tokio::spawn(async move {
            if entry.transport().is_websocket() {
                let stream =
                    WsRelayStream::new(socket, Vec::new(), B * 2, B, Transport::Ws, KEEPALIVE);
                if let Ok(prepared) = service
                    .handle_connection(stream, context, ticket, permit, runtime.lifecycle())
                    .await
                {
                    let _ = service.dispatch(prepared, &runtime).await;
                }
            } else {
                let stream = TcpRelayStream::new(socket);
                if let Ok(prepared) = service
                    .handle_connection(stream, context, ticket, permit, runtime.lifecycle())
                    .await
                {
                    let _ = service.dispatch(prepared, &runtime).await;
                }
            }
        });
        let mut peer = Peer {
            socket: client,
            ws: entry.transport().is_websocket(),
        };
        if entry.role() == Role::Viewer {
            assert_eq!(peer.payload().await, VIEWER_GREETING);
        }
        (peer, context.conn_id, task)
    }

    async fn registered(&self, conn: ConnId) {
        self.events
            .wait(|line| {
                line["event"] == "waiting_registered" && line["conn_id"] == conn.to_string()
            })
            .await;
    }
}

struct Peer {
    socket: TcpStream,
    ws: bool,
}

#[expect(
    clippy::expect_used,
    reason = "test peer: a failed socket step fails the test"
)]
impl Peer {
    async fn frame(&mut self, opcode: u8, payload: &[u8]) {
        assert!(self.ws);
        let mut bytes = vec![0x80 | opcode];
        if payload.len() < 126 {
            bytes.push(0x80 | payload.len() as u8);
        } else {
            bytes.push(0x80 | 126);
            bytes.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        }
        let mask = [0x12, 0x34, 0x56, 0x78];
        bytes.extend_from_slice(&mask);
        bytes.extend(
            payload
                .iter()
                .enumerate()
                .map(|(index, byte)| byte ^ mask[index % 4]),
        );
        bounded(self.socket.write_all(&bytes))
            .await
            .expect("write frame");
    }

    async fn request(&mut self, request: &str) {
        let mut bytes = [0; HANDSHAKE_REQUEST_LEN];
        bytes[..request.len()].copy_from_slice(request.as_bytes());
        if self.ws {
            self.frame(2, &bytes).await;
        } else {
            bounded(self.socket.write_all(&bytes))
                .await
                .expect("write request");
        }
    }

    async fn read_frame(&mut self) -> (u8, Vec<u8>) {
        let mut header = [0; 2];
        bounded(self.socket.read_exact(&mut header))
            .await
            .expect("frame header");
        assert_eq!(header[1] & 0x80, 0, "server frames are unmasked");
        let mut len = usize::from(header[1] & 127);
        if len == 126 {
            len = usize::from(bounded(self.socket.read_u16()).await.expect("frame length"));
        }
        assert!(len < 65536);
        let mut payload = vec![0; len];
        bounded(self.socket.read_exact(&mut payload))
            .await
            .expect("frame payload");
        (header[0] & 15, payload)
    }

    async fn payload(&mut self) -> Vec<u8> {
        if self.ws {
            let (opcode, payload) = self.read_frame().await;
            assert_eq!(opcode, 2);
            payload
        } else {
            let mut greeting = vec![0; VIEWER_GREETING.len()];
            bounded(self.socket.read_exact(&mut greeting))
                .await
                .expect("greeting");
            greeting
        }
    }

    async fn close(&mut self, code: u16, reason: &str) {
        let (opcode, payload) = self.read_frame().await;
        assert_eq!(opcode, 8);
        let mut expected = code.to_be_bytes().to_vec();
        expected.extend_from_slice(reason.as_bytes());
        assert_eq!(payload, expected);
        self.frame(8, &payload).await;
    }
}

/// spec: relay.md:RLY-029 partial
/// spec: relay.md:RLY-032 partial
#[tokio::test]
async fn ticket_mismatch_closes_4012_logs_exact_fields_and_preserves_waiting_peer() {
    for role in [Role::Viewer, Role::Server] {
        let harness = Harness::new(true);
        let opposite = if role == Role::Viewer {
            EntryPoint::TcpServer
        } else {
            EntryPoint::TcpViewer
        };
        let entry = if role == Role::Viewer {
            EntryPoint::WsViewer
        } else {
            EntryPoint::WsServer
        };
        let (mut waiting_peer, waiting_id, waiting) = harness.connect(opposite, None, None).await;
        waiting_peer.request("ID:123").await;
        harness.registered(waiting_id).await;
        let (mut wrong, conn, task) = harness.connect(entry, Some("0123"), None).await;
        wrong.request("ID:123").await;
        wrong.close(4012, "ticket_relay_mismatch").await;
        bounded(task).await.expect("mismatch task");
        let log = harness
            .events
            .wait(|line| {
                line["event"] == "connection_rejected" && line["conn_id"] == conn.to_string()
            })
            .await;
        assert_eq!(
            log,
            json!({
                "ts": "1970-01-01T00:00:00.000Z", "level": "warn", "event": "connection_rejected",
                "conn_id": conn.to_string(), "listener": "ws", "role": role.as_str(),
                "stage": "initial_exchange", "reason": "ticket_relay_mismatch", "peer_ip": "127.0.0.1",
                "mode": "mode2", "relay_id_ref": harness.references.reference(b"123").as_str(),
                "ticket_jti": "integration-ticket"
            })
        );
        assert_eq!(
            harness
                .events
                .lines
                .lock()
                .expect("logs")
                .iter()
                .filter(|line| line["conn_id"] == conn.to_string())
                .count(),
            1
        );
        let snapshot = harness.runtime.snapshot(harness.clock.as_ref());
        assert_eq!(snapshot.waiting.len(), 1);
        assert_eq!(snapshot.waiting[0].context.conn_id, waiting_id);
        let (mut correct, _, matched) = harness.connect(entry, Some("123"), None).await;
        correct.request("ID:123").await;
        harness
            .events
            .wait(|line| line["event"] == "session_started")
            .await;
        assert_eq!(
            harness
                .runtime
                .snapshot(harness.clock.as_ref())
                .active
                .len(),
            1
        );
        harness
            .service
            .request_shutdown(&harness.runtime, Duration::ZERO);
        bounded(waiting).await.expect("waiter task");
        bounded(matched).await.expect("matched task");
    }
}

/// spec: relay.md:RLY-030 partial
#[tokio::test]
async fn ticket_mode_one_is_4012_but_disabled_mode_and_invalid_syntax_stay_first() {
    for (enabled, request, code, reason) in [
        (true, "192.0.2.1:5900", 4012, "ticket_relay_mismatch"),
        (false, "192.0.2.1:5900", 1008, "mode_disabled"),
        (true, "invalid target", 4008, "handshake_invalid"),
    ] {
        let harness = Harness::new(enabled);
        let (mut peer, _, task) = harness
            .connect(EntryPoint::WsViewer, Some("123"), None)
            .await;
        peer.request(request).await;
        peer.close(code, reason).await;
        bounded(task).await.expect("rejection task");
        let snapshot = harness.runtime.snapshot(harness.clock.as_ref());
        assert!(snapshot.waiting.is_empty());
        assert!(snapshot.active.is_empty());
        assert_eq!(snapshot.target_connecting, 0);
        let log = harness
            .events
            .wait(|line| line["event"] == "connection_rejected")
            .await;
        assert_eq!(log["reason"], reason);
        assert!(log.get("relay_id_ref").is_none());
    }
    let harness = Harness::new(true);
    let (mut peer, _, task) = harness
        .connect(EntryPoint::WsServer, Some("123"), None)
        .await;
    peer.request("192.0.2.1:5900").await;
    peer.close(4008, "handshake_invalid").await;
    bounded(task).await.expect("server rejects mode one");
}

/// spec: relay.md:RLY-031 partial
#[tokio::test]
async fn ticket_sx_in_initial_exchange_and_waiting_closes_4013_with_info_events() {
    for waiting in [false, true] {
        let harness = Harness::new(true);
        let (mut peer, conn, task) = harness
            .connect(
                EntryPoint::WsServer,
                Some("123"),
                Some(Duration::from_millis(200)),
            )
            .await;
        if waiting {
            peer.request("ID:123").await;
            harness.registered(conn).await;
        }
        peer.close(4013, "session_expired").await;
        bounded(task).await.expect("expired task");
        let log = harness
            .events
            .wait(|line| {
                line["event"]
                    == if waiting {
                        "waiting_removed"
                    } else {
                        "connection_aborted"
                    }
            })
            .await;
        assert_eq!(log["reason"], "session_expired");
        assert_eq!(log["level"], "info");
        if !waiting {
            assert_eq!(log["stage"], "initial_exchange");
        }
        assert!(
            harness
                .runtime
                .snapshot(harness.clock.as_ref())
                .waiting
                .is_empty()
        );
        assert_eq!(harness.budget.current(), 0);
    }
}

/// spec: relay.md:RLY-031 partial
/// spec: management.md:MGT-052 partial
#[tokio::test]
async fn ticket_active_sx_closes_ws_and_tcp_and_keeps_recent_cause_distinct_from_admin() {
    let harness = Harness::new(true);
    let (mut server, server_id, waiting) = harness.connect(EntryPoint::TcpServer, None, None).await;
    server.request("ID:123").await;
    harness.registered(server_id).await;
    let (mut viewer, _, active) = harness
        .connect(
            EntryPoint::WsViewer,
            Some("123"),
            Some(Duration::from_millis(200)),
        )
        .await;
    viewer.request("ID:123").await;
    harness
        .events
        .wait(|line| line["event"] == "session_started")
        .await;
    viewer.close(4013, "session_expired").await;
    let mut byte = [0];
    assert_eq!(
        bounded(server.socket.read(&mut byte))
            .await
            .expect("TCP EOF"),
        0
    );
    bounded(waiting).await.expect("waiter");
    bounded(active).await.expect("active");
    let log = harness
        .events
        .wait(|line| line["event"] == "session_closed")
        .await;
    assert_eq!(log["close_cause"], "session_expired");
    assert_eq!(log["level"], "info");
    assert!(log.get("side").is_none());
    assert!(log.get("admin_account").is_none());
    let snapshot = harness.runtime.snapshot(harness.clock.as_ref());
    assert!(snapshot.active.is_empty());
    assert_eq!(snapshot.recent_closed.len(), 1);
    assert_eq!(
        snapshot.recent_closed[0].close_cause,
        CloseCause::SessionExpired
    );
    assert_eq!(snapshot.recent_closed[0].viewer_transport, Transport::Ws);
    assert_eq!(snapshot.recent_closed[0].target_transport, Transport::Tcp);
    assert_eq!(harness.budget.current(), 0);
}
