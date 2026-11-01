//! `serve` start sequence and shutdown wait (cli-v1 §1.8).
//! Design: docs/architectures/serve-startup.md, docs/architectures/ops-log.md,
//! docs/architectures/tcp-entry-point.md, docs/architectures/status-listener.md.

use std::fmt::Display;
use std::io;
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;

use tokio::sync::watch;
use tokio::task::JoinSet;
use vnc_relay_adapters::accounts_file::{Argon2Hasher, Argon2Verifier, HashFloor};
use vnc_relay_adapters::clock::SystemClock;
use vnc_relay_adapters::crypto::{CryptoError, RelayIdKey, SessionIdKey, SystemTokenSource};
use vnc_relay_adapters::file_watch::{self, FsReloadFileStore, SystemReloadClock};
use vnc_relay_adapters::management_http::{DashboardHints, ManagementListener};
use vnc_relay_adapters::net::{AcceptPolicy, AllowSource, BindFailure, TcpEntryPoint};
use vnc_relay_adapters::ops_log::JsonLinesEventSink;
use vnc_relay_adapters::status_http::StatusListener;
use vnc_relay_adapters::target::TcpTargetConnector;
use vnc_relay_adapters::tls::{self, TlsError, TlsFile, TlsSource};
use vnc_relay_adapters::websocket::{WsEntryPoint, WsPolicy};
use vnc_relay_adapters::{accounts_file, config_file, net};
use vnc_relay_app::accounts::{self, ACCOUNTS_FILE_KEY, AccountsDiagnostic};
use vnc_relay_app::events::{Event, Phase};
use vnc_relay_app::lifecycle::LifecycleService;
use vnc_relay_app::management::login::dummy_hash;
use vnc_relay_app::management::{AdminAuth, AdminPorts};
use vnc_relay_app::ports::{Clock, EventSink};
use vnc_relay_app::relay::{RelayRuntime, RelayRuntimeConfig, RelayService};
use vnc_relay_app::reload::service::ReloadService;
use vnc_relay_app::reload::snapshot::{RelayAllow, ReloadCell, ReloadSnapshot};
use vnc_relay_app::settings::{LogOutput, Settings, TlsFiles};
use vnc_relay_domain::admin_session::AdminSessionPolicy;
use vnc_relay_domain::budget::ConnectionBudget;
use vnc_relay_domain::entry_point::EntryPoint;
use vnc_relay_domain::handshake::HandshakePolicy;
use vnc_relay_domain::login_limit::LoginLimitPolicy;

use crate::cli::{self, EXIT_ACCOUNTS, EXIT_CONFIG, EXIT_LISTENER, EXIT_RUNTIME};

/// Config key of the log output (config-v2 §1.2).
const LOG_OUTPUT_KEY: &str = "log.output";

/// Runs the server until the first shutdown request. Each failing stage
/// writes all its diagnostics to stderr, one `startup_failed` line per
/// diagnostic to the operation log, and returns its exit code.
pub fn run(config: &Path) -> ExitCode {
    // Stage 1: config, then the log output. Until the log is open, events
    // go to stdout (ops-info-v1 §1.1).
    let settings = match config_file::load(config) {
        Ok(settings) => settings,
        Err(diagnostics) => {
            let log = JsonLinesEventSink::stdout(report_log_write_failure);
            for d in &diagnostics {
                cli::report("error", d.error.code(), &d.key, &d.message);
                let config_key = (!d.key.is_empty()).then(|| d.key.clone());
                record_failure(&log, d.error.code(), config_key, None, None);
            }
            return ExitCode::from(EXIT_CONFIG);
        }
    };
    let log: Arc<dyn EventSink> =
        match JsonLinesEventSink::open(&settings.log.output, report_log_write_failure) {
            Ok(log) => Arc::new(log),
            Err(error) => {
                let path = match &settings.log.output {
                    LogOutput::File(path) => path.display().to_string(),
                    LogOutput::Stdout => String::new(),
                };
                cli::report(
                    "error",
                    error.code(),
                    LOG_OUTPUT_KEY,
                    &format!("로그 파일을 열 수 없다({path}). {}", error.cause()),
                );
                let stdout = JsonLinesEventSink::stdout(report_log_write_failure);
                record_failure(
                    &stdout,
                    error.code(),
                    Some(LOG_OUTPUT_KEY.to_owned()),
                    None,
                    Some(error.cause().to_owned()),
                );
                return ExitCode::from(EXIT_CONFIG);
            }
        };
    log.record(&Event::ServerStarting);
    for warning in settings.warnings() {
        log.record(&Event::ConfigWarning {
            warning,
            phase: Phase::Startup,
        });
    }

    // Stage 2: account file.
    let loaded = match accounts_file::load(&settings.admin.accounts_file, HashFloor::Reject) {
        Ok(loaded) => loaded,
        Err(diagnostics) => return accounts_failure(log.as_ref(), &diagnostics),
    };
    for warning in &loaded.warnings {
        cli::report(
            "warning",
            warning.code(),
            ACCOUNTS_FILE_KEY,
            &warning.message(),
        );
    }
    if let Err(diagnostic) = accounts::require_any(&loaded.accounts) {
        return accounts_failure(log.as_ref(), &[diagnostic]);
    }

    // Admin sessions (management-api-v1 §1.4). An unknown account is checked
    // against the dummy hash, so it costs one password check like a known one
    // (MGT-022).
    let dummy = match dummy_hash(&Argon2Hasher, settings.admin.argon2.params()) {
        Ok(dummy) => dummy,
        Err(error) => {
            return runtime_failure(log.as_ref(), "관리 세션용 더미 해시를 만들 수 없다", &error);
        }
    };
    let session = settings.management.session;
    let limit = settings.management.login_limit;
    let clock: Arc<dyn Clock> = Arc::new(SystemClock::new());
    let auth = Arc::new(AdminAuth::new(
        loaded.accounts,
        dummy,
        AdminPorts {
            verifier: Arc::new(Argon2Verifier),
            tokens: Arc::new(SystemTokenSource),
            clock: Arc::clone(&clock),
            events: Arc::clone(&log),
        },
        AdminSessionPolicy::new(session.idle, session.max_lifetime),
        LoginLimitPolicy::new(
            limit.per_account,
            limit.per_source,
            limit.window,
            limit.lockout,
        ),
    ));

    // Stage 4: runtime, then listeners.
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            return runtime_failure(log.as_ref(), "비동기 런타임을 만들 수 없다", &error);
        }
    };
    runtime.block_on(listen_until_shutdown(&settings, log, auth, config, clock))
}

async fn listen_until_shutdown(
    settings: &Settings,
    log: Arc<dyn EventSink>,
    auth: Arc<AdminAuth>,
    config: &Path,
    clock: Arc<dyn Clock>,
) -> ExitCode {
    // Registered before binding so that a request during startup is not lost.
    let mut shutdown = match ShutdownSignal::register() {
        Ok(shutdown) => shutdown,
        Err(error) => {
            return runtime_failure(log.as_ref(), "종료 신호 처리기를 준비할 수 없다", &error);
        }
    };

    let (relay_key, session_key) = match generate_startup_keys() {
        Ok(keys) => keys,
        Err(error) => {
            return startup_keys_failure(log.as_ref(), &error);
        }
    };

    // TLS identity files of the enabled WSS entry point and of the management
    // listener are checked before any listener is bound (cli-v1 §1.8). Each
    // acceptor goes into the reload snapshot below.
    let mut management_tls = None;
    let mut wss_tls = None;
    let mut tls_failed = false;
    for check in tls_checks(settings) {
        match tls::load_acceptor(&check.files.cert_file, &check.files.key_file) {
            Ok(acceptor) if check.listener == "management" => management_tls = Some(acceptor),
            Ok(acceptor) => wss_tls = Some(acceptor),
            Err(error) => {
                tls_failed = true;
                tls_failure(log.as_ref(), &check, &error);
            }
        }
    }
    if tls_failed {
        return ExitCode::from(EXIT_CONFIG);
    }

    // Reload state (file-watch.md §5): the running values are the snapshot of
    // this start. A failed watch start exits 1 before any listener is bound.
    let wss_present = wss_tls.is_some();
    let management_tls_on = management_tls.is_some();
    let allow = |entry_point: EntryPoint| {
        settings
            .relay
            .get(entry_point)
            .allow
            .clone()
            .unwrap_or_default()
    };
    let cell = Arc::new(ReloadCell::new(ReloadSnapshot::new(
        RelayAllow {
            tcp_viewer: allow(EntryPoint::TcpViewer),
            tcp_server: allow(EntryPoint::TcpServer),
            ws_viewer: allow(EntryPoint::WsViewer),
            wss_viewer: allow(EntryPoint::WssViewer),
        },
        wss_tls,
        management_tls,
    )));
    let service = ReloadService::new(
        settings.clone(),
        Arc::clone(&cell),
        Arc::new(FsReloadFileStore),
        Arc::new(tls::RustlsTlsStore),
        Arc::clone(&log),
    );
    let (stop, stopped) = watch::channel(false);
    let watcher = match file_watch::start(
        config,
        service,
        settings.reload.poll,
        SystemReloadClock,
        Arc::new(FsReloadFileStore),
        Arc::clone(&log),
    ) {
        Ok(watcher) => watcher,
        Err(error) => return watch_unavailable(log.as_ref(), error.detail()),
    };
    let watch_task = tokio::spawn(watcher.run(stopped.clone()));
    let lifecycle = Arc::new(LifecycleService::new());
    // Lm, shared by the status listener and the management listener (4.2).
    let admin_budget = ConnectionBudget::new(settings.limits.admin_connections);
    let status_plan = settings.status_listener();
    let management_plan = settings.management_listener();

    // The status listener answers before the others are bound, so a query
    // during the start sees `starting` (ops-info-v1 §1.12).
    let mut failures = Vec::new();
    let status_task = match net::bind_one(status_plan).await {
        Ok(bound) => {
            let status = StatusListener::new(
                bound.listener,
                Arc::clone(&admin_budget),
                Arc::clone(&lifecycle),
            );
            Some(tokio::spawn(status.run(Arc::clone(&log))))
        }
        Err(failure) => {
            failures.push(failure);
            None
        }
    };
    let management = match net::bind_one(management_plan).await {
        Ok(bound) => Some(bound),
        Err(failure) => {
            failures.push(failure);
            None
        }
    };
    let listeners = match net::bind(&settings.other_listeners()).await {
        Ok(listeners) => listeners,
        Err(others) => {
            failures.extend(others);
            Vec::new()
        }
    };
    if !failures.is_empty() {
        drop(listeners);
        drop(management);
        stop_task(status_task).await;
        // The file watch stops with the other tasks.
        stop.send_replace(true);
        let _ = watch_task.await;
        report_bind_failures(log.as_ref(), &failures);
        return ExitCode::from(EXIT_LISTENER);
    }

    lifecycle.mark_ready();
    let ready = std::iter::once(&status_plan)
        .chain(management.iter().map(|bound| &bound.plan))
        .chain(listeners.iter().map(|bound| &bound.plan))
        .map(|plan| (plan.name, plan.addr))
        .collect();
    log.record(&Event::ServerReady { listeners: ready });

    // The management listener accepts from here on.
    let management_task = management.map(|bound| {
        let dashboard = DashboardHints::new(
            settings.dashboard.poll,
            settings.dashboard.query_timeout,
            settings.timeouts.close,
        );
        let listener = ManagementListener::new(
            bound.listener,
            management_tls_on.then(|| TlsSource::management(Arc::clone(&cell))),
            Arc::clone(&admin_budget),
            settings.timeouts.tls_upgrade,
            auth,
            dashboard,
        );
        tokio::spawn(listener.run(Arc::clone(&log)))
    });

    // The TCP and WebSocket entry points accept from here on. Every relay entry
    // point takes places from one L.
    let relay_budget = ConnectionBudget::new(settings.limits.relay_connections);
    let waiting_budget = ConnectionBudget::new(settings.limits.waiting);
    let connector = TcpTargetConnector::new(settings.keepalive);
    let runtime = Arc::new(RelayRuntime::new(
        connector,
        Arc::new(relay_key),
        Arc::new(session_key),
        Arc::clone(&relay_budget),
        waiting_budget,
        RelayRuntimeConfig {
            wait_timeout: settings.timeouts.wait,
            connect_timeout: settings.timeouts.connect,
            close_timeout: settings.timeouts.close,
            recent_closed: settings.recent_closed.max_entries as usize,
            target_policy: settings.mode1.allow_targets.clone().unwrap_or_default(),
        },
    ));
    let relay_service = Arc::new(RelayService::new(
        HandshakePolicy {
            mode1_enabled: settings.mode1.enabled,
            mode2_enabled: settings.mode2.enabled,
        },
        settings.timeouts.handshake,
        settings.limits.buffer_bytes as usize,
        settings.timeouts.close,
        Arc::clone(&clock),
        Arc::clone(&log),
    ));

    let supervisor = {
        let service = Arc::clone(&relay_service);
        let runtime = Arc::clone(&runtime);
        tokio::spawn(async move {
            service.drive_shutdown(&runtime).await;
        })
    };

    let mut entry_points = JoinSet::new();
    let mut idle = Vec::new();
    for bound in listeners {
        match bound.plan.entry_point {
            Some(entry_point @ (EntryPoint::TcpViewer | EntryPoint::TcpServer)) => {
                let policy = AcceptPolicy {
                    allow: match entry_point {
                        EntryPoint::TcpViewer => AllowSource::tcp_viewer(Arc::clone(&cell)),
                        _ => AllowSource::tcp_server(Arc::clone(&cell)),
                    },
                    relay_budget: Arc::clone(&relay_budget),
                    handshake: settings.timeouts.handshake,
                    keepalive: settings.keepalive,
                };
                let entry = TcpEntryPoint::new(
                    entry_point,
                    bound.listener,
                    policy,
                    Arc::clone(&relay_service),
                    Arc::clone(&runtime),
                    Arc::clone(&clock),
                );
                entry_points.spawn(entry.run(Arc::clone(&log), stopped.clone()));
            }
            Some(entry_point @ (EntryPoint::WsViewer | EntryPoint::WssViewer)) => {
                let entry_settings = settings.relay.get(entry_point);
                let tls = match entry_point {
                    EntryPoint::WssViewer if wss_present => Some(TlsSource::wss(Arc::clone(&cell))),
                    _ => None,
                };
                if matches!(entry_point, EntryPoint::WssViewer) && tls.is_none() {
                    idle.push(bound);
                    continue;
                }
                let policy = WsPolicy {
                    accept: AcceptPolicy {
                        allow: match entry_point {
                            EntryPoint::WssViewer => AllowSource::wss_viewer(Arc::clone(&cell)),
                            _ => AllowSource::ws_viewer(Arc::clone(&cell)),
                        },
                        relay_budget: Arc::clone(&relay_budget),
                        handshake: settings.timeouts.handshake,
                        keepalive: settings.keepalive,
                    },
                    upgrade: settings.timeouts.tls_upgrade,
                    origins: entry_settings.allowed_origins.clone(),
                    message_bytes: settings.limits.ws_message_bytes as usize,
                    buffer_bytes: settings.limits.buffer_bytes as usize,
                };
                let entry = WsEntryPoint::new(
                    entry_point,
                    bound.listener,
                    tls,
                    policy,
                    Arc::clone(&relay_service),
                    Arc::clone(&runtime),
                    Arc::clone(&clock),
                );
                entry_points.spawn(entry.run(Arc::clone(&log), stopped.clone()));
            }
            _ => idle.push(bound),
        }
    }

    shutdown.wait().await;
    lifecycle.begin_shutdown();
    relay_service.request_shutdown(&runtime, settings.timeouts.shutdown_grace);
    stop.send_replace(true);

    finish_relay_shutdown(&mut entry_points, supervisor, shutdown.wait(), || {
        relay_service.request_shutdown(&runtime, Duration::ZERO)
    })
    .await;
    drop(idle);
    stop_task(management_task).await;
    stop_task(status_task).await;
    let _ = watch_task.await;
    log.record(&Event::ServerStopped);
    ExitCode::SUCCESS
}

/// Keeps the G driver alive until all entry-point owners release their work.
/// With no owners left, neither G nor another signal should delay process exit.
async fn finish_relay_shutdown(
    entry_points: &mut JoinSet<()>,
    supervisor: tokio::task::JoinHandle<()>,
    second_request: impl Future<Output = ()>,
    force: impl FnOnce(),
) {
    tokio::select! {
        _ = async {
            while entry_points.join_next().await.is_some() {}
        } => {}
        _ = second_request => {
            force();
            while entry_points.join_next().await.is_some() {}
        }
    }
    // Entry points join their connection owners before returning. Their last
    // completion makes the independent G deadline unnecessary.
    stop_task(Some(supervisor)).await;
}

fn generate_startup_keys() -> Result<(RelayIdKey, SessionIdKey), CryptoError> {
    generate_startup_keys_with(RelayIdKey::generate, SessionIdKey::generate)
}

fn generate_startup_keys_with<F, G>(
    relay_gen: F,
    session_gen: G,
) -> Result<(RelayIdKey, SessionIdKey), CryptoError>
where
    F: FnOnce() -> Result<RelayIdKey, CryptoError>,
    G: FnOnce() -> Result<SessionIdKey, CryptoError>,
{
    let relay = relay_gen()?;
    let session = session_gen()?;
    Ok((relay, session))
}

fn startup_keys_failure(log: &dyn EventSink, error: &dyn Display) -> ExitCode {
    runtime_failure(log, "운영체제 난수원으로 시작 키를 만들 수 없다", error)
}
/// Cancels a background task and joins it before releasing its resources.
async fn stop_task(task: Option<tokio::task::JoinHandle<()>>) {
    if let Some(task) = task {
        task.abort();
        // The result is the cancellation that `abort` asked for.
        let _ = task.await;
    }
}

fn report_bind_failures(log: &dyn EventSink, failures: &[BindFailure]) {
    for failure in failures {
        let plan = failure.plan;
        cli::report(
            "error",
            "listener_bind_failed",
            plan.key,
            &format!("{}에 바인드할 수 없다. {}", plan.addr, failure.cause),
        );
        record_failure(
            log,
            "listener_bind_failed",
            Some(plan.key.to_owned()),
            Some(plan.name),
            Some(failure.cause.clone()),
        );
    }
}

fn record_failure(
    log: &dyn EventSink,
    reason: &'static str,
    config_key: Option<String>,
    listener: Option<&'static str>,
    detail: Option<String>,
) {
    log.record(&Event::StartupFailed {
        reason,
        config_key,
        listener,
        detail,
    });
}

/// One TLS identity that startup checks before any listener is bound.
struct TlsCheck<'a> {
    /// Config key prefix of the two files, such as `relay.wss_viewer`.
    prefix: &'static str,
    /// `listener` value of the operation log (ops-info-v1 §1.5).
    listener: &'static str,
    files: &'a TlsFiles,
}

/// The enabled WSS entry point, then management, when they set TLS files.
fn tls_checks(settings: &Settings) -> Vec<TlsCheck<'_>> {
    let mut checks = Vec::new();
    let wss = settings.relay.get(EntryPoint::WssViewer);
    if let (true, Some(files)) = (wss.enabled, wss.tls.as_ref()) {
        checks.push(TlsCheck {
            prefix: "relay.wss_viewer",
            listener: EntryPoint::WssViewer.name(),
            files,
        });
    }
    if let Some(files) = settings.management.tls.as_ref() {
        checks.push(TlsCheck {
            prefix: "management",
            listener: "management",
            files,
        });
    }
    checks
}

/// Reports one TLS file error on stderr and in the operation log. The config
/// key names the file that failed. The detail stays empty, so the log never
/// carries file paths or key material.
fn tls_failure(log: &dyn EventSink, check: &TlsCheck<'_>, error: &TlsError) {
    let file = match error.file() {
        TlsFile::Cert => "cert_file",
        TlsFile::Key => "key_file",
    };
    let key = format!("{}.{file}", check.prefix);
    cli::report("error", error.code(), &key, &error.to_string());
    record_failure(log, error.code(), Some(key), Some(check.listener), None);
}

/// Account file diagnostics may quote file paths inside the file, so the log
/// line carries the config key of the file and no detail (ops-info-v1 §1.11).
fn accounts_failure(log: &dyn EventSink, diagnostics: &[AccountsDiagnostic]) -> ExitCode {
    for d in diagnostics {
        cli::report("error", d.error.code(), &d.key, &d.message);
        record_failure(
            log,
            d.error.code(),
            Some(ACCOUNTS_FILE_KEY.to_owned()),
            None,
            None,
        );
    }
    ExitCode::from(EXIT_ACCOUNTS)
}

fn runtime_failure(log: &dyn EventSink, what: &str, error: &dyn Display) -> ExitCode {
    cli::report(
        "error",
        "runtime_unavailable",
        "",
        &format!("{what}({error})."),
    );
    record_failure(
        log,
        "runtime_unavailable",
        None,
        None,
        Some(error.to_string()),
    );
    ExitCode::from(EXIT_RUNTIME)
}

/// A watch that cannot register at start refuses the start (file-watch.md §6, OPS-028): one
/// `watch_unavailable` diagnostic, one `startup_failed` line, exit code 1.
fn watch_unavailable(log: &dyn EventSink, detail: &str) -> ExitCode {
    cli::report(
        "error",
        "watch_unavailable",
        "",
        &format!("파일 변경 감지를 준비할 수 없다({detail})."),
    );
    record_failure(log, "watch_unavailable", None, None, None);
    ExitCode::from(EXIT_RUNTIME)
}

/// First failed log write (ops-info-v1 §1.1). Later failures stay silent.
fn report_log_write_failure(error: &io::Error) {
    cli::report(
        "warning",
        "log_write_failed",
        LOG_OUTPUT_KEY,
        &format!("운영 로그를 쓸 수 없다({error}). 이후 실패는 알리지 않는다."),
    );
}

/// Shutdown requests of cli-v1 §1.8: SIGINT (Ctrl+C) and SIGTERM.
#[cfg(unix)]
struct ShutdownSignal {
    interrupt: tokio::signal::unix::Signal,
    terminate: tokio::signal::unix::Signal,
}

#[cfg(unix)]
impl ShutdownSignal {
    fn register() -> io::Result<Self> {
        use tokio::signal::unix::{SignalKind, signal};
        Ok(Self {
            interrupt: signal(SignalKind::interrupt())?,
            terminate: signal(SignalKind::terminate())?,
        })
    }

    async fn wait(&mut self) {
        std::future::poll_fn(|cx| {
            if self.interrupt.poll_recv(cx).is_ready() || self.terminate.poll_recv(cx).is_ready() {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await;
    }
}

/// Shutdown requests of cli-v1 §1.8: Ctrl+C and Ctrl+Break.
#[cfg(windows)]
struct ShutdownSignal {
    ctrl_c: tokio::signal::windows::CtrlC,
    ctrl_break: tokio::signal::windows::CtrlBreak,
}

#[cfg(windows)]
impl ShutdownSignal {
    fn register() -> io::Result<Self> {
        Ok(Self {
            ctrl_c: tokio::signal::windows::ctrl_c()?,
            ctrl_break: tokio::signal::windows::ctrl_break()?,
        })
    }

    async fn wait(&mut self) {
        std::future::poll_fn(|cx| {
            if self.ctrl_c.poll_recv(cx).is_ready() || self.ctrl_break.poll_recv(cx).is_ready() {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};
    use vnc_relay_app::events::Event;

    #[derive(Default)]
    struct RecordingSink {
        events: Mutex<Vec<Event>>,
    }

    impl EventSink for RecordingSink {
        fn record(&self, event: &Event) {
            self.events.lock().expect("sink lock").push(event.clone());
        }
    }

    struct SupervisorDrop(Arc<AtomicBool>);

    impl Drop for SupervisorDrop {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    fn shutdown_driver(
        grace: Duration,
        force: watch::Sender<bool>,
    ) -> (tokio::task::JoinHandle<()>, Arc<AtomicBool>) {
        let stopped = Arc::new(AtomicBool::new(false));
        let guard = SupervisorDrop(Arc::clone(&stopped));
        let task = tokio::spawn(async move {
            let _guard = guard;
            tokio::time::sleep(grace).await;
            force.send_replace(true);
        });
        (task, stopped)
    }

    /// spec: operations.md:OPS-008 partial
    #[tokio::test(start_paused = true)]
    async fn no_entry_point_owners_cancel_and_join_supervisor_without_waiting_for_g() {
        let start = tokio::time::Instant::now();
        let (force, forced) = watch::channel(false);
        let (supervisor, stopped) = shutdown_driver(Duration::from_secs(90), force);
        let mut owners = JoinSet::new();
        finish_relay_shutdown(&mut owners, supervisor, std::future::pending(), || {
            panic!("no second request")
        })
        .await;
        assert_eq!(start.elapsed(), Duration::ZERO);
        assert!(stopped.load(Ordering::SeqCst), "supervisor was joined");
        assert!(!*forced.borrow(), "G was not needed");
    }

    /// spec: operations.md:OPS-008 partial
    #[tokio::test(start_paused = true)]
    async fn last_entry_point_owner_finishing_early_cancels_and_joins_supervisor() {
        let start = tokio::time::Instant::now();
        let (force, forced) = watch::channel(false);
        let (supervisor, stopped) = shutdown_driver(Duration::from_secs(90), force);
        let mut owners = JoinSet::new();
        owners.spawn(async {});
        owners.spawn(async {
            tokio::time::sleep(Duration::from_secs(1)).await;
        });
        finish_relay_shutdown(&mut owners, supervisor, std::future::pending(), || {
            panic!("no second request")
        })
        .await;
        assert_eq!(start.elapsed(), Duration::from_secs(1));
        assert!(stopped.load(Ordering::SeqCst), "supervisor was joined");
        assert!(!*forced.borrow(), "last owner finished before G");
    }

    /// spec: operations.md:OPS-009 partial
    #[tokio::test(start_paused = true)]
    async fn g_driver_remains_alive_until_entry_point_owners_finish_cleanup() {
        let start = tokio::time::Instant::now();
        let (force, mut forced) = watch::channel(false);
        let (supervisor, stopped) = shutdown_driver(Duration::from_secs(3), force);
        let mut owners = JoinSet::new();
        owners.spawn(async {});
        owners.spawn(async move {
            forced.wait_for(|forced| *forced).await.expect("G driver");
            tokio::time::sleep(Duration::from_secs(1)).await;
        });
        finish_relay_shutdown(&mut owners, supervisor, std::future::pending(), || {
            panic!("no second request")
        })
        .await;
        assert_eq!(start.elapsed(), Duration::from_secs(4));
        assert!(stopped.load(Ordering::SeqCst), "supervisor was joined");
        assert!(owners.is_empty(), "forced owners completed cleanup");
    }

    /// spec: operations.md:OPS-042 partial
    #[tokio::test(start_paused = true)]
    async fn second_request_forces_owners_then_cancels_and_joins_supervisor() {
        let start = tokio::time::Instant::now();
        let (force, mut forced) = watch::channel(false);
        let (supervisor, stopped) = shutdown_driver(Duration::from_secs(90), force.clone());
        let mut owners = JoinSet::new();
        owners.spawn(async move {
            forced
                .wait_for(|forced| *forced)
                .await
                .expect("second request");
            tokio::time::sleep(Duration::from_secs(1)).await;
        });
        finish_relay_shutdown(
            &mut owners,
            supervisor,
            tokio::time::sleep(Duration::from_secs(1)),
            || {
                force.send_replace(true);
            },
        )
        .await;
        assert_eq!(start.elapsed(), Duration::from_secs(2));
        assert!(stopped.load(Ordering::SeqCst), "supervisor was joined");
        assert!(owners.is_empty(), "forced owners completed cleanup");
    }

    /// spec: operations.md:OPS-028 partial
    #[test]
    fn watch_registration_refused_at_start_is_a_startup_failure() {
        let sink = RecordingSink::default();
        let code = watch_unavailable(&sink, "registration refused");
        assert_eq!(code, ExitCode::from(EXIT_RUNTIME));
        let events = sink.events.lock().expect("sink lock");
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            Event::StartupFailed { reason, .. } if *reason == "watch_unavailable"
        ));
    }

    #[test]
    fn startup_key_generation_failure_is_a_startup_failure() {
        let sink = RecordingSink::default();
        let error = CryptoError;
        let code = startup_keys_failure(&sink, &error);
        assert_eq!(code, ExitCode::from(EXIT_RUNTIME));
        let events = sink.events.lock().expect("sink lock");
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            Event::StartupFailed { reason, detail, .. }
                if *reason == "runtime_unavailable" && detail.as_deref() == Some("운영체제 난수를 읽을 수 없다.")
        ));

        let err = generate_startup_keys_with(|| Err(CryptoError), SessionIdKey::generate);
        assert!(matches!(err, Err(CryptoError)));
    }
}
