//! Q114 exercises actual accept loops, HTTP, relay owners and serve supervision.
//! Only accept results are injected at the Listener port; payloads use OS TCP.

use super::*;
use std::net::SocketAddr;
use std::sync::{Mutex, PoisonError};
use std::time::{Instant, SystemTime};

use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::task::{AbortHandle, JoinHandle};
use vnc_relay_adapters::listener::{Listener, is_fatal_accept_error};
use vnc_relay_adapters::ops_log::format_line;
use vnc_relay_app::lifecycle::Readiness;
use vnc_relay_app::reload::ports::RuntimeSettingsSource;
use vnc_relay_app::settings::KeepaliveSettings;
use vnc_relay_domain::allow_list::{AllowList, IpItem};
use vnc_relay_domain::handshake::{HANDSHAKE_REQUEST_LEN, VIEWER_GREETING};

const LIMIT: Duration = Duration::from_secs(10);
const KEEPALIVE: KeepaliveSettings = KeepaliveSettings {
    tcp_idle: Duration::from_secs(60),
    tcp_interval: Duration::from_secs(10),
    tcp_probes: 3,
    ws_ping: Duration::from_secs(30),
    ws_pong_timeout: Duration::from_secs(30),
};

async fn bounded<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(LIMIT, future)
        .await
        .unwrap_or_else(|error| panic!("listener scenario deadline: {error}"))
}

#[derive(Default)]
struct Recorder(Mutex<Vec<Value>>);

impl Recorder {
    fn lines(&self) -> Vec<Value> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    async fn event(&self, name: &str) -> Value {
        let start = Instant::now();
        loop {
            if let Some(line) = self.lines().into_iter().find(|line| line["event"] == name) {
                return line;
            }
            assert!(start.elapsed() < LIMIT, "missing {name}");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}

impl EventSink for Recorder {
    fn record(&self, event: &Event) {
        let line = format_line(event, SystemTime::UNIX_EPOCH);
        let value = serde_json::from_str(&line)
            .unwrap_or_else(|error| panic!("actual event JSON: {error}"));
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(value);
    }
}

struct FaultListener {
    inner: TcpListener,
    faults: mpsc::UnboundedReceiver<i32>,
}

impl Listener for FaultListener {
    async fn accept(&mut self) -> io::Result<(TcpStream, SocketAddr)> {
        tokio::select! {
            biased;
            Some(code) = self.faults.recv() => Err(io::Error::from_raw_os_error(code)),
            accepted = self.inner.accept() => accepted,
        }
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.inner.local_addr()
    }
}

async fn socket() -> (TcpListener, SocketAddr) {
    let listener = bounded(TcpListener::bind("127.0.0.1:0"))
        .await
        .unwrap_or_else(|error| panic!("bind: {error}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|error| panic!("address: {error}"));
    (listener, addr)
}

struct Running {
    viewer: SocketAddr,
    server: SocketAddr,
    status: SocketAddr,
    relay_fault: mpsc::UnboundedSender<i32>,
    status_fault: mpsc::UnboundedSender<i32>,
    requests: mpsc::UnboundedSender<ShutdownRequest>,
    service: Arc<RelayService>,
    runtime: Arc<RelayRuntime<TcpTargetConnector>>,
    lifecycle: Arc<LifecycleService>,
    budget: Arc<ConnectionBudget>,
    admin_budget: Arc<ConnectionBudget>,
    clock: Arc<dyn Clock>,
    events: Arc<Recorder>,
    done: JoinHandle<ExitCode>,
    abort: Vec<AbortHandle>,
}

impl Running {
    async fn start() -> Self {
        let (viewer, viewer_addr) = socket().await;
        let (server, server_addr) = socket().await;
        let (status, status_addr) = socket().await;
        let (relay_fault, relay_faults) = mpsc::unbounded_channel();
        let (status_fault, status_faults) = mpsc::unbounded_channel();
        let (failures, mut failed) = watch::channel(None);
        let (requests, mut requested) = mpsc::unbounded_channel();
        let (stop, stopped) = watch::channel(false);
        let lifecycle = Arc::new(LifecycleService::new());
        let events = Arc::new(Recorder::default());
        let log: Arc<dyn EventSink> = events.clone();
        log.record(&Event::ServerStarting);
        let budget = ConnectionBudget::new();
        let admin_budget = ConnectionBudget::new();
        let clock: Arc<dyn Clock> = Arc::new(SystemClock::new());
        let allow = AllowList::new(vec![
            IpItem::parse("127.0.0.1").unwrap_or_else(|error| panic!("allow: {error:?}")),
        ]);
        let mut settings = config_file::parse(
            r#"[admin]
accounts_file = "accounts.toml"
[management]
listen = "127.0.0.1:8443"
[status]
listen = "127.0.0.1:9090"
[mode1]
allow_targets = []
[mode2]
enabled = true
"#,
            std::path::Path::new("."),
        )
        .unwrap_or_else(|error| panic!("listener settings: {error:?}"));
        settings.relay.tcp_viewer.enabled = true;
        settings.relay.tcp_viewer.listen = Some(viewer_addr);
        settings.relay.tcp_viewer.allow = Some(allow.clone());
        settings.relay.tcp_server.enabled = true;
        settings.relay.tcp_server.listen = Some(server_addr);
        settings.relay.tcp_server.allow = Some(allow);
        settings.status.listen = status_addr;
        settings.limits.relay_connections = 8;
        settings.limits.admin_connections = 8;
        settings.limits.waiting = 8;
        settings.limits.buffer_bytes = 16_384;
        settings.timeouts.handshake = LIMIT;
        settings.timeouts.wait = LIMIT;
        settings.timeouts.connect = Duration::from_secs(1);
        settings.timeouts.close = Duration::from_secs(1);
        settings.keepalive = KEEPALIVE;
        settings.recent_closed.max_entries = 8;
        let cell = Arc::new(ReloadCell::new(ReloadSnapshot::new(
            ConfigCandidate {
                settings,
                presence: Default::default(),
            },
            None,
            None,
        )));
        let settings: Arc<dyn RuntimeSettingsSource> = cell.clone();
        let runtime = Arc::new(RelayRuntime::new(
            TcpTargetConnector::new(Arc::clone(&settings)),
            Arc::new(RelayIdKey::generate().unwrap_or_else(|error| panic!("relay key: {error:?}"))),
            Arc::new(SessionIdKey::from_bytes(&[0x51; 32])),
            Arc::clone(&budget),
            ConnectionBudget::new(),
            RelayRuntimeConfig { recent_closed: 8 },
            Arc::clone(&settings),
        ));
        let service = Arc::new(RelayService::new(
            HandshakePolicy {
                mode1_enabled: false,
                mode2_enabled: true,
            },
            Arc::clone(&settings),
            Arc::clone(&clock),
            Arc::clone(&log),
        ));
        let mut owners = JoinSet::new();
        let mut abort = Vec::new();
        for (kind, inner, faults, allow) in [
            (
                EntryPoint::TcpViewer,
                viewer,
                relay_faults,
                AllowSource::tcp_viewer(Arc::clone(&cell)),
            ),
            (
                EntryPoint::TcpServer,
                server,
                mpsc::unbounded_channel().1,
                AllowSource::tcp_server(cell),
            ),
        ] {
            let listener = SupervisedListener::new(
                FaultListener { inner, faults },
                kind.name(),
                failures.clone(),
            );
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
            abort.push(owners.spawn(entry.run(Arc::clone(&log), stopped.clone())));
        }
        let status = StatusListener::new(
            SupervisedListener::new(
                FaultListener {
                    inner: status,
                    faults: status_faults,
                },
                "status",
                failures,
            ),
            Arc::clone(&admin_budget),
            Arc::clone(&lifecycle),
            settings,
        );
        let status_task = tokio::spawn(status.run(Arc::clone(&log)));
        abort.push(status_task.abort_handle());
        let driver = {
            let service = Arc::clone(&service);
            let runtime = Arc::clone(&runtime);
            tokio::spawn(async move { service.drive_shutdown(&runtime).await })
        };
        abort.push(driver.abort_handle());
        lifecycle.mark_ready();
        log.record(&Event::ServerReady {
            listeners: vec![
                ("status", status_addr),
                ("tcp_viewer", viewer_addr),
                ("tcp_server", server_addr),
            ],
        });
        let done = {
            let lifecycle = Arc::clone(&lifecycle);
            let service = Arc::clone(&service);
            let runtime = Arc::clone(&runtime);
            tokio::spawn(async move {
                let first = wait_for_shutdown(
                    async {
                        match requested.recv().await {
                            Some(request) => request,
                            None => std::future::pending().await,
                        }
                    },
                    &mut failed,
                )
                .await;
                let exit = begin_process_shutdown(
                    first,
                    &lifecycle,
                    &service,
                    &runtime,
                    Duration::from_secs(2),
                );
                stop.send_replace(true);
                finish_relay_shutdown(
                    &mut owners,
                    driver,
                    async {
                        requested.recv().await;
                    },
                    || service.request_shutdown(&runtime, Duration::ZERO),
                )
                .await;
                stop_task(Some(status_task)).await;
                log.record(&Event::ServerStopped);
                exit
            })
        };
        Self {
            viewer: viewer_addr,
            server: server_addr,
            status: status_addr,
            relay_fault,
            status_fault,
            requests,
            service,
            runtime,
            lifecycle,
            budget,
            admin_budget,
            clock,
            events,
            done,
            abort,
        }
    }

    async fn pair(&self) -> (TcpStream, TcpStream) {
        let mut viewer = bounded(TcpStream::connect(self.viewer))
            .await
            .unwrap_or_else(|error| panic!("viewer: {error}"));
        let mut greeting = vec![0; VIEWER_GREETING.len()];
        bounded(viewer.read_exact(&mut greeting))
            .await
            .unwrap_or_else(|error| panic!("greeting: {error}"));
        assert_eq!(greeting, VIEWER_GREETING);
        let mut request = [0; HANDSHAKE_REQUEST_LEN];
        request[..6].copy_from_slice(b"ID:114");
        bounded(viewer.write_all(&request))
            .await
            .unwrap_or_else(|error| panic!("viewer request: {error}"));
        self.events.event("waiting_registered").await;
        let mut server = bounded(TcpStream::connect(self.server))
            .await
            .unwrap_or_else(|error| panic!("server: {error}"));
        bounded(server.write_all(&request))
            .await
            .unwrap_or_else(|error| panic!("server request: {error}"));
        self.events.event("session_started").await;
        (viewer, server)
    }

    async fn http(&self, path: &str) -> String {
        let mut stream = bounded(TcpStream::connect(self.status))
            .await
            .unwrap_or_else(|error| panic!("status connect: {error}"));
        bounded(
            stream.write_all(
                format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                    .as_bytes(),
            ),
        )
        .await
        .unwrap_or_else(|error| panic!("status request: {error}"));
        let mut bytes = Vec::new();
        bounded(stream.read_to_end(&mut bytes))
            .await
            .unwrap_or_else(|error| panic!("status answer: {error}"));
        String::from_utf8(bytes).unwrap_or_else(|error| panic!("HTTP utf8: {error}"))
    }

    async fn finished(&mut self, exit: ExitCode) {
        assert_eq!(
            bounded(&mut self.done)
                .await
                .unwrap_or_else(|error| panic!("supervisor: {error}")),
            exit
        );
        assert_eq!(self.budget.current(), 0);
        let start = Instant::now();
        while self.admin_budget.current() != 0 {
            assert!(start.elapsed() < LIMIT, "HTTP permits released");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let snapshot = self.runtime.snapshot(self.clock.as_ref());
        assert!(snapshot.active.is_empty());
        assert!(snapshot.waiting.is_empty());
        assert_eq!(snapshot.target_connecting, 0);
        for addr in [self.viewer, self.server, self.status] {
            let listener = bounded(TcpListener::bind(addr))
                .await
                .unwrap_or_else(|error| panic!("listener reclaimed {addr}: {error}"));
            drop(listener);
        }
        let lines = self.events.lines();
        assert_eq!(
            lines.last().map(|line| line["event"].as_str()),
            Some(Some("server_stopped"))
        );
        assert_eq!(
            lines
                .iter()
                .filter(|line| line["event"] == "shutdown_started")
                .count(),
            1
        );
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        self.service.request_shutdown(&self.runtime, Duration::ZERO);
        self.done.abort();
        for task in &self.abort {
            task.abort();
        }
    }
}

fn fatal_code() -> i32 {
    if cfg!(windows) { 10038 } else { 22 }
}

/// spec: operations.md:OPS-046
#[tokio::test]
async fn listener_failures_retry_then_drive_fatal_grace_cleanup_and_preserve_requested_races() {
    let transient = if cfg!(windows) {
        vec![10024, 10055, 10050, 995]
    } else {
        vec![23, 24, 12, 105, 95]
    };
    for code in transient {
        assert!(!is_fatal_accept_error(&io::Error::from_raw_os_error(code)));
    }
    // Both the relay and HTTP implementations retry descriptor exhaustion.
    let mut running = Running::start().await;
    let resource = if cfg!(windows) { 10024 } else { 24 };
    running
        .relay_fault
        .send(resource)
        .unwrap_or_else(|error| panic!("inject resource: {error}"));
    running
        .status_fault
        .send(resource)
        .unwrap_or_else(|error| panic!("inject HTTP resource: {error}"));
    assert!(running.http("/readyz").await.starts_with("HTTP/1.1 200"));
    let (mut viewer, mut server) = running.pair().await;
    assert!(!running.done.is_finished());
    running
        .relay_fault
        .send(fatal_code())
        .unwrap_or_else(|error| panic!("inject fatal: {error}"));
    let started = running.events.event("shutdown_started").await;
    assert_eq!(started["trigger"], "fatal_error");
    assert_eq!(started["level"], "error");
    assert_eq!(started["active_sessions"], 1);
    assert_eq!(started["grace_secs"], 2);
    assert_eq!(
        started["detail"],
        ListenerFailure {
            listener: "tcp_viewer",
            os_error: fatal_code()
        }
        .detail()
    );
    let ready = running.http("/readyz").await;
    assert!(ready.starts_with("HTTP/1.1 503"));
    assert!(ready.contains("\"reason\":\"listener_failed\""));
    assert!(running.http("/livez").await.starts_with("HTTP/1.1 200"));
    bounded(viewer.write_all(b"real grace bytes"))
        .await
        .unwrap_or_else(|error| panic!("relay grace write: {error}"));
    let mut bytes = [0; 16];
    bounded(server.read_exact(&mut bytes))
        .await
        .unwrap_or_else(|error| panic!("relay grace read: {error}"));
    assert_eq!(&bytes, b"real grace bytes");
    assert!(
        bounded(TcpStream::connect(running.viewer)).await.is_err(),
        "new relay listener closed"
    );
    let mut byte = [0];
    assert_eq!(
        bounded(viewer.read(&mut byte))
            .await
            .unwrap_or_else(|error| panic!("forced viewer EOF: {error}")),
        0
    );
    assert_eq!(
        bounded(server.read(&mut byte))
            .await
            .unwrap_or_else(|error| panic!("forced server EOF: {error}")),
        0
    );
    drop((viewer, server));
    running.finished(ExitCode::from(EXIT_RUNTIME)).await;
    let lines = running.events.lines();
    let at = |name: &str| {
        lines
            .iter()
            .position(|line| line["event"] == name)
            .unwrap_or_else(|| panic!("missing {name}"))
    };
    assert!(at("server_ready") < at("shutdown_started"));
    assert!(at("shutdown_started") < at("shutdown_forced"));
    assert!(at("shutdown_forced") < at("session_closed"));
    assert!(at("session_closed") < at("server_stopped"));
    assert_eq!(
        lines[at("session_closed")]["close_cause"],
        "server_shutdown"
    );

    // A real OS request after a fatal first cause forces G but keeps the cause.
    let mut running = Running::start().await;
    let (viewer, server) = running.pair().await;
    running
        .status_fault
        .send(fatal_code())
        .unwrap_or_else(|error| panic!("status fatal before OS request: {error}"));
    let event = running.events.event("shutdown_started").await;
    assert_eq!(
        event["detail"],
        ListenerFailure {
            listener: "status",
            os_error: fatal_code()
        }
        .detail()
    );
    running.lifecycle.begin_shutdown();
    running.lifecycle.mark_ready();
    assert_eq!(running.lifecycle.readiness(), Readiness::ListenerFailed);
    let forced_at = Instant::now();
    running
        .requests
        .send(ShutdownRequest::Requested)
        .unwrap_or_else(|_| panic!("OS request during fatal G"));
    running.finished(ExitCode::from(EXIT_RUNTIME)).await;
    assert!(forced_at.elapsed() < Duration::from_secs(2));
    assert_eq!(
        running.events.event("shutdown_started").await["trigger"],
        "fatal_error"
    );
    drop((viewer, server));

    // No active owners means no artificial G wait, including a dead status socket.
    for status_failure in [false, true] {
        let mut running = Running::start().await;
        let started = Instant::now();
        let faults = if status_failure {
            &running.status_fault
        } else {
            &running.relay_fault
        };
        faults
            .send(fatal_code())
            .unwrap_or_else(|error| panic!("inject idle fatal: {error}"));
        running.finished(ExitCode::from(EXIT_RUNTIME)).await;
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "idle shutdown did not dwell for G"
        );
        let event = running.events.event("shutdown_started").await;
        assert_eq!(event["trigger"], "fatal_error");
        assert_eq!(event["active_sessions"], 0);
        assert!(
            !running
                .events
                .lines()
                .iter()
                .any(|line| line["event"] == "shutdown_forced")
        );
    }

    // An already-started requested shutdown ignores a later fatal report.
    let mut running = Running::start().await;
    let (viewer, server) = running.pair().await;
    running
        .requests
        .send(ShutdownRequest::Requested)
        .unwrap_or_else(|_| panic!("request shutdown"));
    let first = running.events.event("shutdown_started").await;
    assert_eq!(first["trigger"], "requested");
    let before = *running.runtime.lifecycle().borrow();
    running.lifecycle.listener_failed();
    running.service.request_fatal_shutdown(
        &running.runtime,
        Duration::ZERO,
        "late listener report".to_owned(),
    );
    assert_eq!(running.lifecycle.readiness(), Readiness::ShuttingDown);
    assert_eq!(*running.runtime.lifecycle().borrow(), before);
    assert!(
        running
            .http("/readyz")
            .await
            .contains("\"reason\":\"shutting_down\"")
    );
    running
        .requests
        .send(ShutdownRequest::Requested)
        .unwrap_or_else(|_| panic!("second shutdown"));
    drop((viewer, server));
    running.finished(ExitCode::SUCCESS).await;

    // A ready console request wins simultaneous reports; the supervisor does not
    // reinterpret a listener error as ConsoleClosed or as a second OS signal.
    let (sender, mut failed) = watch::channel(Some(ListenerFailure {
        listener: "status",
        os_error: fatal_code(),
    }));
    let request = wait_for_shutdown(async { ShutdownRequest::Requested }, &mut failed).await;
    assert!(matches!(request, ShutdownRequest::Requested));
    drop(sender);
}
