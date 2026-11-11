//! Real TCP streaming acceptance scenarios. The bounded-buffer scenario first
//! crosses the unmodified TcpEntryPoint, then instruments real accepted sockets
//! at RelayService's public stream boundary; no payload comes from a mock.

use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, SystemTime};

use serde_json::Value;
use socket2::SockRef;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Barrier, Notify, oneshot, watch};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use vnc_relay_adapters::clock::SystemClock;
use vnc_relay_adapters::config_file::parse;
use vnc_relay_adapters::crypto::{RelayIdKey, SessionIdKey};
use vnc_relay_adapters::net::{AcceptPolicy, AllowSource, TcpEntryPoint, TcpRelayStream};
use vnc_relay_adapters::ops_log::format_line;
use vnc_relay_adapters::target::TcpTargetConnector;
use vnc_relay_app::events::Event;
use vnc_relay_app::ports::{Clock, EventSink, RelayCloseAction, RelayStream};
use vnc_relay_app::relay::{ConnectionContext, RelayRuntime, RelayRuntimeConfig, RelayService};
use vnc_relay_app::reload::ports::RuntimeSettingsSource;
use vnc_relay_app::reload::snapshot::{ReloadCell, ReloadSnapshot};
use vnc_relay_app::reload::{ConfigCandidate, ConfigPresence};
use vnc_relay_app::settings::KeepaliveSettings;
use vnc_relay_domain::allow_list::{AllowList, IpItem};
use vnc_relay_domain::budget::ConnectionBudget;
use vnc_relay_domain::conn_id::ConnId;
use vnc_relay_domain::entry_point::EntryPoint;
use vnc_relay_domain::handshake::{HANDSHAKE_REQUEST_LEN, HandshakePolicy, VIEWER_GREETING};
use vnc_relay_domain::peer_ip::PeerIp;
use vnc_relay_domain::session::Transport;

const B: usize = 16_384;
const LIMIT: Duration = Duration::from_secs(10);
const H_C_E: Duration = Duration::from_secs(1);
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
        .unwrap_or_else(|error| panic!("TCP scenario exceeded {LIMIT:?}: {error}"))
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
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
        let line = format_line(event, SystemTime::UNIX_EPOCH);
        lock(&self.lines).push(
            serde_json::from_str(&line)
                .unwrap_or_else(|error| panic!("operation log is JSON: {error}")),
        );
        self.changed.notify_one();
    }
}

struct Running {
    viewer: SocketAddr,
    server: SocketAddr,
    service: Arc<RelayService>,
    runtime: Arc<RelayRuntime<TcpTargetConnector>>,
    clock: Arc<dyn Clock>,
    events: Arc<Recorder>,
    budget: Arc<ConnectionBudget>,
    places: u32,
    stop: watch::Sender<bool>,
    tasks: Vec<JoinHandle<()>>,
}

impl Running {
    async fn start(places: u32) -> Self {
        let viewer = TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|error| panic!("viewer bind: {error}"));
        let server = TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|error| panic!("server bind: {error}"));
        let viewer_addr = viewer
            .local_addr()
            .unwrap_or_else(|error| panic!("viewer address: {error}"));
        let server_addr = server
            .local_addr()
            .unwrap_or_else(|error| panic!("server address: {error}"));
        let allow = AllowList::new(vec![
            IpItem::parse("127.0.0.1").unwrap_or_else(|error| panic!("loopback allow: {error:?}")),
        ]);
        let mut settings = parse(
            r#"[admin]
accounts_file = "accounts.toml"
[management]
listen = "127.0.0.1:8443"
[status]
listen = "127.0.0.1:9090"
[mode1]
enabled = true
allow_targets = []
[mode2]
enabled = true
"#,
            std::path::Path::new("."),
        )
        .unwrap_or_else(|error| panic!("stream settings: {error:?}"));
        settings.relay.tcp_viewer.enabled = true;
        settings.relay.tcp_viewer.listen = Some(viewer_addr);
        settings.relay.tcp_viewer.allow = Some(allow.clone());
        settings.relay.tcp_server.enabled = true;
        settings.relay.tcp_server.listen = Some(server_addr);
        settings.relay.tcp_server.allow = Some(allow);
        settings.limits.relay_connections = places;
        settings.limits.waiting = places;
        settings.limits.buffer_bytes = B as u32;
        settings.timeouts.handshake = H_C_E;
        settings.timeouts.wait = LIMIT;
        settings.timeouts.connect = H_C_E;
        settings.timeouts.close = H_C_E;
        settings.keepalive = KEEPALIVE;
        settings.recent_closed.max_entries = 16;
        let cell = Arc::new(ReloadCell::new(ReloadSnapshot::new(
            ConfigCandidate {
                settings,
                presence: ConfigPresence::default(),
            },
            None,
            None,
        )));
        let settings: Arc<dyn RuntimeSettingsSource> = cell.clone();
        let clock: Arc<dyn Clock> = Arc::new(SystemClock::new());
        let events = Arc::new(Recorder::default());
        let log: Arc<dyn EventSink> = events.clone();
        let budget = ConnectionBudget::new();
        let runtime = Arc::new(RelayRuntime::new(
            TcpTargetConnector::new(Arc::clone(&settings)),
            Arc::new(RelayIdKey::generate().unwrap_or_else(|error| panic!("relay key: {error:?}"))),
            Arc::new(SessionIdKey::from_bytes(&[0x59; 32])),
            Arc::clone(&budget),
            ConnectionBudget::new(),
            RelayRuntimeConfig { recent_closed: 16 },
            Arc::clone(&settings),
        ));
        let service = Arc::new(RelayService::new(
            HandshakePolicy {
                mode1_enabled: true,
                mode2_enabled: true,
            },
            settings,
            Arc::clone(&clock),
            Arc::clone(&log),
        ));
        let (stop, stopped) = watch::channel(false);
        let mut tasks = Vec::new();
        for (kind, listener, allow) in [
            (
                EntryPoint::TcpViewer,
                viewer,
                AllowSource::tcp_viewer(Arc::clone(&cell)),
            ),
            (EntryPoint::TcpServer, server, AllowSource::tcp_server(cell)),
        ] {
            let entry = TcpEntryPoint::new(
                kind,
                listener,
                AcceptPolicy {
                    allow,
                    relay_budget: Arc::clone(&budget),
                },
                Arc::clone(&service),
                Arc::clone(&runtime),
                Arc::clone(&clock),
            );
            tasks.push(tokio::spawn(entry.run(Arc::clone(&log), stopped.clone())));
        }
        let shutdown_service = Arc::clone(&service);
        let shutdown_runtime = Arc::clone(&runtime);
        tasks.push(tokio::spawn(async move {
            shutdown_service.drive_shutdown(&shutdown_runtime).await;
        }));
        Self {
            viewer: viewer_addr,
            server: server_addr,
            service,
            runtime,
            clock,
            events,
            budget,
            places,
            stop,
            tasks,
        }
    }

    async fn pair(&self, id: &str) -> (TcpStream, TcpStream, Value) {
        let mut viewer = bounded(TcpStream::connect(self.viewer))
            .await
            .unwrap_or_else(|error| panic!("viewer connect: {error}"));
        greeting(&mut viewer).await;
        bounded(viewer.write_all(&request(id)))
            .await
            .unwrap_or_else(|error| panic!("viewer request: {error}"));
        let waiting = self
            .events
            .wait(|line| {
                line["event"] == "waiting_registered"
                    && line["role"] == "viewer"
                    && line["conn_id"].as_str().is_some()
                    && !lock(&self.events.lines).iter().any(|other| {
                        other["event"] == "session_started"
                            && other["viewer_conn_id"] == line["conn_id"]
                    })
            })
            .await;
        let mut server = bounded(TcpStream::connect(self.server))
            .await
            .unwrap_or_else(|error| panic!("server connect: {error}"));
        bounded(server.write_all(&request(id)))
            .await
            .unwrap_or_else(|error| panic!("server request: {error}"));
        let started = self
            .events
            .wait(|line| {
                line["event"] == "session_started" && line["viewer_conn_id"] == waiting["conn_id"]
            })
            .await;
        (viewer, server, started)
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
            lock(&self.events.lines)
                .iter()
                .filter(|line| {
                    line["event"] == "session_closed" && line["session_id"] == started["session_id"]
                })
                .count(),
            1
        );
        assert!(
            !self
                .runtime
                .snapshot(self.clock.as_ref())
                .active
                .iter()
                .any(|active| {
                    Some(active.session_id.as_str()) == started["session_id"].as_str()
                })
        );
        closed
    }

    fn assert_empty(&self) {
        let snapshot = self.runtime.snapshot(self.clock.as_ref());
        assert!(snapshot.active.is_empty());
        assert!(snapshot.waiting.is_empty());
        assert_eq!(snapshot.target_connecting, 0);
        let permits: Vec<_> = (0..self.places)
            .map(|_| {
                self.budget
                    .try_acquire(self.places)
                    .unwrap_or_else(|| panic!("all relay places returned"))
            })
            .collect();
        assert!(self.budget.try_acquire(self.places).is_none());
        drop(permits);
    }

    async fn finish(mut self) {
        self.service.request_shutdown(&self.runtime, Duration::ZERO);
        self.stop.send_replace(true);
        for task in &mut self.tasks {
            bounded(task)
                .await
                .unwrap_or_else(|error| panic!("listener completed: {error}"));
        }
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        self.stop.send_replace(true);
        for task in &self.tasks {
            task.abort();
        }
    }
}

fn request(id: &str) -> [u8; HANDSHAKE_REQUEST_LEN] {
    let mut bytes = [0; HANDSHAKE_REQUEST_LEN];
    let text = format!("ID:{id}");
    bytes[..text.len()].copy_from_slice(text.as_bytes());
    bytes
}

async fn greeting(viewer: &mut TcpStream) {
    let mut actual = [0; VIEWER_GREETING.len()];
    bounded(viewer.read_exact(&mut actual))
        .await
        .unwrap_or_else(|error| panic!("viewer greeting: {error}"));
    assert_eq!(&actual, VIEWER_GREETING);
}

fn payload(seed: u8, length: usize) -> Vec<u8> {
    (0..length)
        .map(|index| {
            seed.wrapping_add((index % 251) as u8)
                .wrapping_add((index / 251 % 251) as u8)
        })
        .collect()
}

async fn write_chunks(mut stream: tokio::net::tcp::WriteHalf<'_>, bytes: &[u8], barrier: &Barrier) {
    for chunk in bytes.chunks(1024) {
        // Both directions rendezvous for every chunk, not just at startup.
        barrier.wait().await;
        stream
            .write_all(chunk)
            .await
            .unwrap_or_else(|error| panic!("concurrent write: {error}"));
    }
    stream
        .shutdown()
        .await
        .unwrap_or_else(|error| panic!("concurrent half-close: {error}"));
}

async fn simultaneous(
    viewer: &mut TcpStream,
    server: &mut TcpStream,
    forward: &[u8],
    reverse: &[u8],
) {
    simultaneous_after_prefix(viewer, server, forward, reverse, 0).await;
}

async fn simultaneous_after_prefix(
    viewer: &mut TcpStream,
    server: &mut TcpStream,
    forward: &[u8],
    reverse: &[u8],
    prefix: usize,
) {
    assert_eq!(forward.len(), reverse.len());
    let barrier = Barrier::new(2);
    let (mut viewer_read, viewer_write) = viewer.split();
    let (mut server_read, server_write) = server.split();
    let mut got_forward = Vec::new();
    let mut got_reverse = Vec::new();
    bounded(async {
        tokio::join!(
            write_chunks(viewer_write, &forward[prefix..], &barrier),
            write_chunks(server_write, &reverse[prefix..], &barrier),
            async {
                server_read
                    .read_to_end(&mut got_forward)
                    .await
                    .unwrap_or_else(|error| panic!("forward drain: {error}"));
            },
            async {
                viewer_read
                    .read_to_end(&mut got_reverse)
                    .await
                    .unwrap_or_else(|error| panic!("reverse drain: {error}"));
            },
        );
    })
    .await;
    assert_eq!(got_forward, forward);
    assert_eq!(got_reverse, reverse);
}

/// spec: relay.md:RLY-017
#[tokio::test]
async fn simultaneously_relays_distinct_ordered_streams_in_both_directions() {
    let running = Running::start(2).await;
    let (mut viewer, mut server, started) = running.pair("17001").await;
    let forward = payload(0x17, B * 8);
    let reverse = payload(0xa3, B * 8);
    assert_ne!(forward, reverse);
    simultaneous(&mut viewer, &mut server, &forward, &reverse).await;
    let closed = running.closed(&started, "peer_eof").await;
    assert_eq!(closed["bytes_viewer_to_target"], forward.len());
    assert_eq!(closed["bytes_target_to_viewer"], reverse.len());
    running.assert_empty();
    running.finish().await;
}

/// spec: relay.md:RLY-019
#[tokio::test]
async fn drains_request_before_response_after_half_close_and_reuses_all_places() {
    let running = Running::start(2).await;
    let (mut viewer, mut server, started) = running.pair("19001").await;
    let request = payload(0x19, B * 3 + 37);
    let response = payload(0xc1, B * 2 + 59);
    let mut got_request = Vec::new();
    let mut got_response = Vec::new();
    bounded(async {
        tokio::join!(
            async {
                viewer
                    .write_all(&request)
                    .await
                    .unwrap_or_else(|error| panic!("request: {error}"));
                viewer
                    .shutdown()
                    .await
                    .unwrap_or_else(|error| panic!("request half-close: {error}"));
                viewer
                    .read_to_end(&mut got_response)
                    .await
                    .unwrap_or_else(|error| panic!("response drain: {error}"));
            },
            async {
                server
                    .read_to_end(&mut got_request)
                    .await
                    .unwrap_or_else(|error| panic!("request drain: {error}"));
                assert_eq!(got_request, request);
                // A response is not even written until the request EOF reaches the peer.
                assert!(!lock(&running.events.lines).iter().any(|line| {
                    line["event"] == "session_closed" && line["session_id"] == started["session_id"]
                }));
                server
                    .write_all(&response)
                    .await
                    .unwrap_or_else(|error| panic!("response: {error}"));
                server
                    .shutdown()
                    .await
                    .unwrap_or_else(|error| panic!("response half-close: {error}"));
            },
        );
    })
    .await;
    assert_eq!(got_response, response);
    let closed = running.closed(&started, "peer_eof").await;
    assert_eq!(closed["side"], "viewer");
    assert_eq!(closed["bytes_viewer_to_target"], request.len());
    assert_eq!(closed["bytes_target_to_viewer"], response.len());
    running.assert_empty();
    let (mut next_viewer, mut next_server, next) = running.pair("19001").await;
    assert_ne!(next["session_id"], started["session_id"]);
    simultaneous(
        &mut next_viewer,
        &mut next_server,
        b"reuse-new-viewer",
        b"reuse-new-server",
    )
    .await;
    running.closed(&next, "peer_eof").await;
    running.assert_empty();
    running.finish().await;
}

/// spec: relay.md:RLY-021
#[tokio::test]
async fn resetting_one_session_preserves_another_sessions_bytes_and_lifetime() {
    let running = Running::start(4).await;
    let (bad_viewer, mut bad_server, bad) = running.pair("21001").await;
    let (mut viewer, mut server, survivor) = running.pair("21002").await;
    let (flowing, flow_started) = oneshot::channel();
    let (error_finished, mut error_observed) = oneshot::channel();
    let traffic = async {
        let mut flowing = Some(flowing);
        let mut count = 0_u64;
        let mut transferred = 0_u64;
        let mut saw_error = false;
        while count < 32 || !saw_error {
            let forward = payload((count % 251) as u8, 1024);
            let reverse = payload((count % 251) as u8 ^ 0xc7, 1024);
            let mut got_forward = vec![0; forward.len()];
            let mut got_reverse = vec![0; reverse.len()];
            let (viewer_read, viewer_write) = viewer.split();
            let (server_read, server_write) = server.split();
            tokio::join!(
                async {
                    let mut write = viewer_write;
                    write
                        .write_all(&forward)
                        .await
                        .unwrap_or_else(|error| panic!("survivor forward: {error}"));
                },
                async {
                    let mut write = server_write;
                    write
                        .write_all(&reverse)
                        .await
                        .unwrap_or_else(|error| panic!("survivor reverse: {error}"));
                },
                async {
                    let mut read = server_read;
                    read.read_exact(&mut got_forward)
                        .await
                        .unwrap_or_else(|error| panic!("survivor forward bytes: {error}"));
                },
                async {
                    let mut read = viewer_read;
                    read.read_exact(&mut got_reverse)
                        .await
                        .unwrap_or_else(|error| panic!("survivor reverse bytes: {error}"));
                },
            );
            assert_eq!(got_forward, forward);
            assert_eq!(got_reverse, reverse);
            count += 1;
            transferred += 1024;
            if count == 8 && flowing.is_some() {
                flowing
                    .take()
                    .unwrap_or_else(|| panic!("one flowing signal"))
                    .send(())
                    .unwrap_or_else(|_| panic!("error injector alive"));
            }
            if !saw_error && error_observed.try_recv().is_ok() {
                saw_error = true;
                // Require another 32 exchanges after the error has been observed.
                count = 0;
            }
        }
        (count, transferred)
    };
    let inject = async {
        flow_started
            .await
            .unwrap_or_else(|error| panic!("other session transfers first: {error}"));
        SockRef::from(&bad_viewer)
            .set_linger(Some(Duration::ZERO))
            .unwrap_or_else(|error| panic!("arm TCP RST: {error}"));
        drop(bad_viewer);
        let closed = running.closed(&bad, "connection_error").await;
        assert_eq!(closed["error_kind"], "io_error");
        assert_eq!(closed["side"], "viewer");
        let active = running.runtime.snapshot(running.clock.as_ref()).active;
        assert_eq!(active.len(), 1);
        assert_eq!(
            Some(active[0].session_id.as_str()),
            survivor["session_id"].as_str()
        );
        assert!(!lock(&running.events.lines).iter().any(|line| {
            line["event"] == "session_closed" && line["session_id"] == survivor["session_id"]
        }));
        error_finished
            .send(())
            .unwrap_or_else(|_| panic!("continuous transfer still alive"));
        let mut discarded = Vec::new();
        match bad_server.read_to_end(&mut discarded).await {
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::ConnectionReset => {}
            Err(error) => panic!("failed session peer closes: {error}"),
        }
        assert!(discarded.is_empty());
    };
    let ((after_error, transferred), ()) = bounded(async { tokio::join!(traffic, inject) }).await;
    assert_eq!(after_error, 32);
    assert_eq!(
        running
            .runtime
            .snapshot(running.clock.as_ref())
            .active
            .len(),
        1
    );
    simultaneous(
        &mut viewer,
        &mut server,
        b"still-open-left",
        b"still-open-peer",
    )
    .await;
    let closed = running.closed(&survivor, "peer_eof").await;
    assert_eq!(closed["bytes_viewer_to_target"], transferred + 15);
    assert_eq!(closed["bytes_target_to_viewer"], transferred + 15);
    running.assert_empty();
    running.finish().await;
}

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
                    assert_eq!(state.writes, [0, 0], "recipient writes are gated");
                    if state.reads == [B, B] {
                        assert_eq!(state.peaks, [B, B]);
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

struct MeasuredStream {
    inner: TcpStream,
    side: usize,
    meter: Arc<Meter>,
}

impl AsyncRead for MeasuredStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let before = buffer.filled().len();
        let result = Pin::new(&mut this.inner).poll_read(cx, buffer);
        let read = buffer.filled().len() - before;
        let mut state = lock(&this.meter.state);
        if state.armed && read > 0 {
            state.reads[this.side] += read;
            let queued = state.reads[this.side]
                .checked_sub(state.writes[this.side])
                .unwrap_or_else(|| panic!("cannot write payload before reading it"));
            state.peaks[this.side] = state.peaks[this.side].max(queued);
            assert!(
                queued <= B,
                "actual unread-to-unwritten payload {queued} exceeds B={B}"
            );
            this.meter.changed.notify_one();
        }
        result
    }
}

impl AsyncWrite for MeasuredStream {
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
                return Poll::Pending;
            }
        }
        let result = Pin::new(&mut this.inner).poll_write(cx, bytes);
        if let Poll::Ready(Ok(written)) = result {
            let mut state = lock(&this.meter.state);
            if state.armed {
                // Writes on one socket discharge the opposite input direction.
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

impl RelayStream for MeasuredStream {
    fn transport(&self) -> Transport {
        Transport::Tcp
    }
    fn pending_write_len(&self) -> usize {
        0
    }
    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        _action: RelayCloseAction,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}

async fn measured_pair(
    running: &Running,
    meter: Arc<Meter>,
    forward: &[u8],
    reverse: &[u8],
) -> (TcpStream, TcpStream, Value, Vec<JoinHandle<()>>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap_or_else(|error| panic!("measured bind: {error}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|error| panic!("measured address: {error}"));
    let mut prepared = Vec::new();
    let mut peers = Vec::new();
    for (side, entry_point) in [EntryPoint::TcpViewer, EntryPoint::TcpServer]
        .into_iter()
        .enumerate()
    {
        let peer = bounded(TcpStream::connect(addr))
            .await
            .unwrap_or_else(|error| panic!("measured connect: {error}"));
        let (stream, peer_addr) = bounded(listener.accept())
            .await
            .unwrap_or_else(|error| panic!("measured accept: {error}"));
        let now = running.clock.monotonic();
        let context = ConnectionContext {
            conn_id: ConnId::next(),
            entry_point,
            peer_ip: PeerIp::from(peer_addr),
            forwarded_ip: None,
            peer_addr,
            accepted_wall: running.clock.wall(),
            accepted_at: now,
            exchange_at: now,
            buffer_bytes: B,
            handshake: H_C_E,
        };
        let permit = running
            .budget
            .try_acquire(running.places)
            .unwrap_or_else(|| panic!("measured admission"));
        let input = MeasuredStream {
            inner: TcpRelayStream::with_keepalive(stream, &KEEPALIVE)
                .unwrap_or_else(|error| panic!("measured keepalive: {error}"))
                .into_inner(),
            side,
            meter: Arc::clone(&meter),
        };
        let mut peer = peer;
        let prepare = running.service.handle_connection(
            input,
            context,
            None,
            permit,
            running.runtime.lifecycle(),
        );
        let send = async {
            if side == 0 {
                greeting(&mut peer).await;
            }
            peer.write_all(&request("18002"))
                .await
                .unwrap_or_else(|error| panic!("measured request: {error}"));
        };
        let (connection, ()) = bounded(async { tokio::join!(prepare, send) }).await;
        let connection = connection.unwrap_or_else(|error| panic!("measured prepared: {error:?}"));
        assert_eq!(
            connection.buffered_bytes(),
            0,
            "payload starts only after arming"
        );
        prepared.push(connection);
        peers.push(peer);
    }
    {
        let mut state = lock(&meter.state);
        state.armed = true;
        state.gated = true;
    }
    // Make exactly B real bytes available before either dispatch can read. Peeking
    // avoids relying on TCP packet boundaries or a scheduler delay to reach B.
    for ((connection, peer), bytes) in prepared
        .iter_mut()
        .zip(peers.iter_mut())
        .zip([forward, reverse])
    {
        bounded(peer.write_all(&bytes[..B]))
            .await
            .unwrap_or_else(|error| panic!("measured prefix: {error}"));
        let mut peek = vec![0; B];
        bounded(async {
            loop {
                let available = connection
                    .stream_mut()
                    .inner
                    .peek(&mut peek)
                    .await
                    .unwrap_or_else(|error| panic!("measured availability: {error}"));
                if available == B {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await;
    }
    let mut tasks = Vec::new();
    for connection in prepared {
        let service = Arc::clone(&running.service);
        let runtime = Arc::clone(&running.runtime);
        tasks.push(tokio::spawn(async move {
            service
                .dispatch(connection, &runtime)
                .await
                .unwrap_or_else(|error| panic!("measured dispatch: {error:?}"));
        }));
    }
    let started = running
        .events
        .wait(|line| {
            line["event"] == "session_started"
                && line["relay_id_ref"].as_str().is_some()
                && !lock(&running.events.lines).iter().any(|other| {
                    other["event"] == "session_closed" && other["session_id"] == line["session_id"]
                })
        })
        .await;
    let server = peers.pop().unwrap_or_else(|| panic!("server peer"));
    let viewer = peers.pop().unwrap_or_else(|| panic!("viewer peer"));
    (viewer, server, started, tasks)
}

/// spec: relay.md:RLY-018
#[tokio::test]
async fn bounds_each_real_tcp_direction_at_b_and_preserves_order_after_recipient_resumes() {
    let running = Running::start(2).await;
    // First exercise the actual public entrypoints with a deliberately unread peer.
    let (mut viewer, mut server, started) = running.pair("18001").await;
    let forward = payload(0x18, B * 16);
    let reverse = payload(0xb8, B * 16);
    bounded(viewer.write_all(&forward[..B]))
        .await
        .unwrap_or_else(|error| panic!("delayed recipient prefix: {error}"));
    bounded(server.write_all(&reverse[..B]))
        .await
        .unwrap_or_else(|error| panic!("delayed reverse prefix: {error}"));
    // Neither peer has read payload yet. Resume and drain exact ordered bytes.
    let mut got_forward = vec![0; B];
    let mut got_reverse = vec![0; B];
    let (forward_result, reverse_result) = bounded(async {
        tokio::join!(
            server.read_exact(&mut got_forward),
            viewer.read_exact(&mut got_reverse)
        )
    })
    .await;
    forward_result.unwrap_or_else(|error| panic!("resumed forward prefix: {error}"));
    reverse_result.unwrap_or_else(|error| panic!("resumed reverse prefix: {error}"));
    assert_eq!(got_forward, forward[..B]);
    assert_eq!(got_reverse, reverse[..B]);
    simultaneous(&mut viewer, &mut server, &forward[B..], &reverse[B..]).await;
    running.closed(&started, "peer_eof").await;
    running.assert_empty();

    // TcpEntryPoint fixes the concrete stream type, so use public preparation and
    // dispatch on the SAME service/runtime to wrap two other real accepted TCP
    // sockets. Only writes are gated; reads always come from the real OS socket.
    // Each read and successful opposite write updates a direct queue measurement.
    let meter = Arc::new(Meter::default());
    let forward = payload(0x38, B * 16 + 173);
    let reverse = payload(0xd8, B * 16 + 173);
    let (mut viewer, mut server, started, tasks) =
        measured_pair(&running, Arc::clone(&meter), &forward, &reverse).await;
    let exchange = simultaneous_after_prefix(&mut viewer, &mut server, &forward, &reverse, B);
    let resume = async {
        meter.saturated().await;
        // Both real input directions reached exactly B with zero recipient writes.
        meter.release();
    };
    bounded(async { tokio::join!(exchange, resume) }).await;
    let closed = running.closed(&started, "peer_eof").await;
    assert_eq!(closed["bytes_viewer_to_target"], forward.len());
    assert_eq!(closed["bytes_target_to_viewer"], reverse.len());
    {
        let state = lock(&meter.state);
        assert_eq!(state.reads, [forward.len(), reverse.len()]);
        assert_eq!(state.writes, state.reads);
        assert_eq!(state.peaks, [B, B]);
    }
    for task in tasks {
        bounded(task)
            .await
            .unwrap_or_else(|error| panic!("measured owner finishes: {error}"));
    }
    running.assert_empty();
    running.finish().await;
}
