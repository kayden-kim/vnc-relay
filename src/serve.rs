//! `serve` start sequence and shutdown wait (cli-v1 §1.8), shared with the
//! Windows service run (cli-v1 §1.9).
//! Design: docs/architectures/serve-startup.md, docs/architectures/ops-log.md,
//! docs/architectures/tcp-entry-point.md, docs/architectures/status-listener.md,
//! docs/architectures/windows-service.md.

use std::fmt::Display;
use std::io;
use std::path::Path;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::time::{Duration, SystemTime};

use tokio::sync::watch;
use tokio::task::JoinSet;
use vnc_relay_adapters::accounts_file::{
    Argon2PasswordHashGateway, Argon2PasswordVerifyGateway, HashFloor,
};
use vnc_relay_adapters::clock::SystemClock;
use vnc_relay_adapters::config_edit::{FsConfigFileStore, TomlConfigDocumentGateway};
use vnc_relay_adapters::crypto::{CryptoError, RelayIdKey, SessionIdKey, SystemTokenSource};
use vnc_relay_adapters::file_watch::{self, FsReloadFileStore, SystemReloadClock};
use vnc_relay_adapters::ip_db::MmdbFileGateway;
use vnc_relay_adapters::listener::{ListenerFailure, SupervisedListener};
use vnc_relay_adapters::management_http::{DashboardHints, ManagementListener, ManagementServices};
use vnc_relay_adapters::net::{AcceptPolicy, AllowSource, BindFailure, TcpEntryPoint};
use vnc_relay_adapters::ops_log::{JsonLinesEventSink, RecentLogs};
use vnc_relay_adapters::process_stats::OsProcessStats;
use vnc_relay_adapters::status_http::StatusListener;
use vnc_relay_adapters::target::TcpTargetGateway;
use vnc_relay_adapters::tls::{self, TlsError, TlsFile, TlsSource};
use vnc_relay_adapters::websocket::{WsEntryPoint, WsPolicy};
use vnc_relay_adapters::{accounts_file, config_file, net};
use vnc_relay_app::accounts::{self, ACCOUNTS_FILE_KEY, AccountsDiagnostic};
use vnc_relay_app::events::{Event, Phase};
use vnc_relay_app::ip_info::{IpInfoService, IpLookup};
use vnc_relay_app::lifecycle::LifecycleService;
use vnc_relay_app::management::config::{ConfigEditService, ConfigService};
use vnc_relay_app::management::login::dummy_hash;
use vnc_relay_app::management::query::ManagementQueryService;
use vnc_relay_app::management::terminate::ManagementTerminateService;
use vnc_relay_app::management::{AdminAuth, AdminPorts};
use vnc_relay_app::metrics::history::HistoryWindow;
use vnc_relay_app::metrics::{MetricSources, Metrics, MetricsService, MetricsSink};
use vnc_relay_app::ports::{
    AdminSessionSource, Clock, EventSink, RelaySessionControl, RelayStateSource,
};
use vnc_relay_app::relay::{RelayRuntime, RelayRuntimeConfig, RelayService};
use vnc_relay_app::reload::service::ReloadService;
use vnc_relay_app::reload::snapshot::{ReloadCell, ReloadSnapshot};
use vnc_relay_app::reload::{ConfigCandidate, SharedReload};
use vnc_relay_app::settings::{LogOutput, Settings, TlsFiles};
use vnc_relay_app::sources::{SourceStats, SourcesService, SourcesSink};
use vnc_relay_domain::admin_session::AdminSessionPolicy;
use vnc_relay_domain::budget::ConnectionBudget;
use vnc_relay_domain::entry_point::{EntryPoint, Listener};
use vnc_relay_domain::handshake::HandshakePolicy;
use vnc_relay_domain::login_limit::LoginLimitPolicy;
use vnc_relay_domain::ticket::TicketUseTable;

#[cfg(windows)]
use vnc_relay_adapters::windows_service::{ServiceSession, StatusReporter, StopControl};

use crate::cli::{self, EXIT_ACCOUNTS, EXIT_CONFIG, EXIT_LISTENER, EXIT_RUNTIME, EXIT_SUCCESS};

/// Config key of the log output (config-v3 §1.2).
const LOG_OUTPUT_KEY: &str = "log.output";

/// Where shutdown requests come from and who hears about the server state
/// (cli-v1 §1.8, §1.9).
pub enum Host {
    /// `serve` in a terminal: console signals. Start failures before the log
    /// file opens also go to stdout as operation log lines.
    Terminal,
    /// `service run` under the SCM: stop and system shutdown controls, and
    /// status reports. The process writes nothing to stdout or stderr.
    #[cfg(windows)]
    Service(ServiceSession),
}

/// `serve`: runs the server for the terminal.
pub fn run(config: &Path) -> ExitCode {
    ExitCode::from(start(config, Host::Terminal))
}

/// Runs the server until the first shutdown request and returns the exit
/// code of cli-v1 §2.1. Each failing stage writes all its diagnostics to
/// stderr, one `startup_failed` line per diagnostic to the operation log, and
/// returns its exit code.
pub fn start(config: &Path, host: Host) -> u8 {
    // Process start time of the ticket check (websocket-relay-v2 §1.9.3 step 11).
    let started_at = SystemTime::now();
    // Stage 1: config, then the log output. Until the log is open, events
    // go to stdout (ops-info-v1 §1.1); a service has no stdout.
    let running = match config_file::load_candidate(config) {
        Ok(running) => running,
        Err(diagnostics) => {
            let early = early_log(&host);
            for d in &diagnostics {
                cli::report("error", d.error.code(), &d.key, &d.message);
                if let Some(log) = &early {
                    let config_key = (!d.key.is_empty()).then(|| d.key.clone());
                    record_failure(log, d.error.code(), config_key, None, None);
                }
            }
            return EXIT_CONFIG;
        }
    };
    let settings = &running.settings;
    #[cfg(windows)]
    if matches!(host, Host::Service(_)) && matches!(settings.log.output, LogOutput::Stdout) {
        report_service_log_stdout();
        return EXIT_CONFIG;
    }
    let recent_logs = Arc::new(RecentLogs::new(settings.recent_logs.max_entries as usize));
    let output = match JsonLinesEventSink::open(
        &settings.log.output,
        report_log_write_failure,
        report_log_reopen_failure,
    ) {
        Ok(log) => Arc::new(log.with_recent_logs(Arc::clone(&recent_logs))),
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
            if let Some(stdout) = early_log(&host) {
                record_failure(
                    &stdout,
                    error.code(),
                    Some(LOG_OUTPUT_KEY.to_owned()),
                    None,
                    Some(error.cause().to_owned()),
                );
            }
            return EXIT_CONFIG;
        }
    };
    // Metrics and source statistics count the events of this process from
    // here on (metrics-v1 §1.1, source-stats.md §4.1). The sink chain is
    // MetricsSink → SourcesSink → the log output.
    let clock: Arc<dyn Clock> = Arc::new(SystemClock::new());
    let metrics = Arc::new(Metrics::new(clock.as_ref()));
    let ip_lookup = Arc::new(IpLookup::new());
    let sources = Arc::new(SourceStats::new(Arc::clone(&ip_lookup), Arc::clone(&clock)));
    let log: Arc<dyn EventSink> = Arc::new(MetricsSink::new(
        Arc::clone(&metrics),
        Arc::new(SourcesSink::new(Arc::clone(&sources), output.clone())),
    ));
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
    report_accounts_warnings(&loaded.warnings);
    if let Err(diagnostic) = accounts::require_any(&loaded.accounts) {
        return accounts_failure(log.as_ref(), &[diagnostic]);
    }

    // Admin sessions (management-api-v1 §1.4). An unknown account is checked
    // against the dummy hash, so it costs one password check like a known one
    // (MGT-022).
    let dummy = match dummy_hash(&Argon2PasswordHashGateway, settings.admin.argon2.params()) {
        Ok(dummy) => dummy,
        Err(error) => {
            return runtime_failure(log.as_ref(), "관리 세션용 더미 해시를 만들 수 없다", &error);
        }
    };
    let session = settings.management.session;
    let limit = settings.management.login_limit;
    let auth = Arc::new(AdminAuth::new(
        loaded.accounts,
        dummy,
        AdminPorts {
            verifier: Arc::new(Argon2PasswordVerifyGateway),
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
    runtime.block_on(listen_until_shutdown(
        &running,
        Logging {
            output,
            log,
            recent_logs,
            metrics,
            ip_lookup,
            sources,
        },
        auth,
        config,
        clock,
        started_at,
        host,
    ))
}

/// The log sinks and the event counters that `start` opened before the
/// runtime. `log` counts each event in `metrics` and `sources`, then writes
/// it to `output`. `ip_lookup` holds the IP information databases that
/// `sources` reads; `listen_until_shutdown` fills it.
struct Logging {
    output: Arc<JsonLinesEventSink>,
    log: Arc<dyn EventSink>,
    recent_logs: Arc<RecentLogs>,
    metrics: Arc<Metrics>,
    ip_lookup: Arc<IpLookup>,
    sources: Arc<SourceStats>,
}

/// The file checks of `service install` (cli-v1 §1.9 install step 4): stages
/// 1 and 2 and the TLS and ticket key files of stage 4, with the same codes.
/// No log file opens, so the diagnostics go to stderr only.
#[cfg(windows)]
pub fn check_service_files(config: &Path) -> Result<(), u8> {
    let mut running = config_file::load_candidate(config).map_err(|diagnostics| {
        for d in &diagnostics {
            cli::report("error", d.error.code(), &d.key, &d.message);
        }
        EXIT_CONFIG
    })?;
    if matches!(running.settings.log.output, LogOutput::Stdout) {
        report_service_log_stdout();
        return Err(EXIT_CONFIG);
    }
    let accounts_file = &running.settings.admin.accounts_file;
    let loaded = accounts_file::load(accounts_file, HashFloor::Reject).map_err(|diagnostics| {
        report_accounts(&diagnostics);
        EXIT_ACCOUNTS
    })?;
    report_accounts_warnings(&loaded.warnings);
    accounts::require_any(&loaded.accounts).map_err(|diagnostic| {
        report_accounts(&[diagnostic]);
        EXIT_ACCOUNTS
    })?;
    let mut failed = false;
    for check in tls_checks(&running.settings) {
        if let Err(error) = tls::load_acceptor(&check.files.cert_file, &check.files.key_file) {
            report_tls(&check, &error);
            failed = true;
        }
    }
    if let Err(diagnostics) = vnc_relay_app::reload::ticket_keys::load(
        &mut running.settings,
        &FsReloadFileStore,
        &vnc_relay_adapters::ticket::TicketJwksStore,
    ) {
        failed = true;
        for diagnostic in diagnostics {
            cli::report(
                "error",
                diagnostic.error.code(),
                &diagnostic.key,
                &diagnostic.message,
            );
        }
    }
    if failed { Err(EXIT_CONFIG) } else { Ok(()) }
}

/// The stdout sink for start failures before the log file opens. A service
/// has no stdout, so its failures stay in the exit code (cli-v1 §1.9).
fn early_log(host: &Host) -> Option<JsonLinesEventSink> {
    match host {
        Host::Terminal => Some(JsonLinesEventSink::stdout(report_log_write_failure)),
        #[cfg(windows)]
        Host::Service(_) => None,
    }
}

/// A service needs a log file (Q247).
#[cfg(windows)]
fn report_service_log_stdout() {
    cli::report(
        "error",
        "service_log_stdout",
        LOG_OUTPUT_KEY,
        "서비스로 실행하려면 로그 파일이 필요하다. `log.output`에 파일 경로를 쓴다.",
    );
}

async fn listen_until_shutdown(
    running: &ConfigCandidate,
    logging: Logging,
    auth: Arc<AdminAuth>,
    config: &Path,
    clock: Arc<dyn Clock>,
    started_at: SystemTime,
    host: Host,
) -> u8 {
    let mut running = running.clone();
    let settings = &running.settings;
    let Logging {
        output,
        log,
        recent_logs,
        metrics,
        ip_lookup,
        sources,
    } = logging;
    let (mut requests, status) = match host {
        Host::Terminal => match ShutdownSignal::register() {
            Ok(signal) => (Requests::Terminal(signal), HostStatus::default()),
            Err(error) => {
                return runtime_failure(log.as_ref(), "종료 신호 처리기를 준비할 수 없다", &error);
            }
        },
        #[cfg(windows)]
        Host::Service(session) => (
            Requests::Service(session.controls),
            HostStatus {
                service: Some(session.status),
            },
        ),
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
    let mut files_failed = false;
    for check in tls_checks(settings) {
        match tls::load_acceptor(&check.files.cert_file, &check.files.key_file) {
            Ok(acceptor) if check.listener == "management" => management_tls = Some(acceptor),
            Ok(acceptor) => wss_tls = Some(acceptor),
            Err(error) => {
                files_failed = true;
                tls_failure(log.as_ref(), &check, &error);
            }
        }
    }
    // Ticket public key files of enabled roles are checked at the same point
    // (config-v4 §1.11): any failure exits 3 before binding.
    if let Err(diagnostics) = vnc_relay_app::reload::ticket_keys::load(
        &mut running.settings,
        &FsReloadFileStore,
        &vnc_relay_adapters::ticket::TicketJwksStore,
    ) {
        files_failed = true;
        for diagnostic in diagnostics {
            cli::report(
                "error",
                diagnostic.error.code(),
                &diagnostic.key,
                &diagnostic.message,
            );
            record_failure(
                log.as_ref(),
                diagnostic.error.code(),
                Some(diagnostic.key),
                None,
                None,
            );
        }
    }
    if files_failed {
        return EXIT_CONFIG;
    }
    let settings = &running.settings;

    // Reload state (file-watch.md §5): the running values are the snapshot of
    // this start. A failed watch start exits 1 before any listener is bound.
    let wss_present = wss_tls.is_some();
    let management_tls_on = management_tls.is_some();
    let cell = Arc::new(ReloadCell::new(ReloadSnapshot::new(
        running.clone(),
        wss_tls,
        management_tls,
    )));
    let reload = Arc::new(std::sync::Mutex::new(ReloadService::new(
        running.clone(),
        Arc::clone(&cell),
        Arc::new(FsReloadFileStore),
        Arc::new(tls::RustlsTlsStore),
        Arc::new(vnc_relay_adapters::ticket::TicketJwksStore),
        Arc::clone(&log),
    )));
    let config_service = match config_management(
        config,
        Arc::clone(&reload),
        Arc::clone(&clock),
        Arc::clone(&log),
    ) {
        Ok(service) => service,
        Err(error) => {
            return runtime_failure(log.as_ref(), "설정 편집 경로를 준비할 수 없다", &error);
        }
    };
    let (stop, stopped) = watch::channel(false);
    // IP information databases (config-v4 §1.12): the watcher hashes them as
    // its baseline, then the first read runs before any listener is bound.
    // A failed read is a warning; the server starts without that database.
    let ip_info = Arc::new(IpInfoService::new(
        settings.ip_info.country_db.clone(),
        settings.ip_info.asn_db.clone(),
        Arc::new(MmdbFileGateway),
        ip_lookup,
        Arc::clone(&clock),
        Arc::clone(&log),
    ));
    let mut watcher = match file_watch::start(
        config,
        Arc::clone(&reload),
        settings.reload.poll,
        SystemReloadClock,
        Arc::new(FsReloadFileStore),
        Arc::clone(&log),
        output,
    ) {
        Ok(watcher) => watcher,
        Err(error) => return watch_unavailable(log.as_ref(), error.detail()),
    };
    watcher.attach_ip_info(Arc::clone(&ip_info));
    // Reading and checking a file of up to 256 MiB blocks this thread.
    tokio::task::block_in_place(|| ip_info.load_startup());
    let sources_service = Arc::new(SourcesService::new(
        sources,
        ip_info,
        Arc::clone(&cell) as Arc<dyn vnc_relay_app::reload::ports::RuntimeSettingsSource>,
    ));
    let watch_task = tokio::spawn(watcher.run(stopped.clone()));
    let lifecycle = Arc::new(LifecycleService::new());
    let (listener_failures, mut failed_listener) = watch::channel(None);
    // Lm, shared by the status listener and the management listener (4.2).
    let admin_budget = ConnectionBudget::new();
    // Every relay entry point takes places from one L.
    let relay_budget = ConnectionBudget::new();
    let waiting_budget = ConnectionBudget::new();
    let connector = TcpTargetGateway::new(
        Arc::clone(&cell) as Arc<dyn vnc_relay_app::reload::ports::RuntimeSettingsSource>
    );
    let runtime = Arc::new(RelayRuntime::new(
        connector,
        Arc::new(relay_key),
        Arc::new(session_key),
        Arc::clone(&relay_budget),
        waiting_budget,
        RelayRuntimeConfig {
            recent_closed: settings.recent_closed.max_entries as usize,
        },
        Arc::clone(&cell) as Arc<dyn vnc_relay_app::reload::ports::RuntimeSettingsSource>,
    ));
    // A published deny list change closes the open relay connections it
    // denies (config-v4 §1.8). No relay connection exists before this point.
    reload
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .attach_deny_control(
            Arc::clone(&runtime) as Arc<dyn vnc_relay_app::ports::RelayDenyControl>,
            Arc::clone(&clock),
        );
    // The management API always reads metrics; the status listener serves
    // them only when `metrics.enabled` is on (metrics-v1 §1.1).
    let metrics_service = Arc::new(MetricsService::new(
        Arc::clone(&metrics),
        MetricSources {
            relay: Arc::clone(&runtime) as Arc<dyn RelayStateSource>,
            relay_budget: Arc::clone(&relay_budget),
            admin_budget: Arc::clone(&admin_budget),
            settings: Arc::clone(&cell)
                as Arc<dyn vnc_relay_app::reload::ports::RuntimeSettingsSource>,
            admin: Arc::clone(&auth) as Arc<dyn AdminSessionSource>,
            process: Arc::new(OsProcessStats::new()),
        },
        Arc::clone(&clock),
        env!("CARGO_PKG_VERSION"),
    ));
    let status_plan = settings.status_listener();
    let management_plan = settings.management_listener();

    // The status listener answers before the others are bound, so a query
    // during the start sees `starting` (ops-info-v1 §1.12).
    let mut failures = Vec::new();
    let status_task = match net::bind_one(status_plan).await {
        Ok(bound) => {
            let status = StatusListener::new(
                SupervisedListener::new(bound.listener, bound.plan.name, listener_failures.clone()),
                Arc::clone(&admin_budget),
                Arc::clone(&lifecycle),
                Arc::clone(&cell) as Arc<dyn vnc_relay_app::reload::ports::RuntimeSettingsSource>,
                settings
                    .metrics
                    .enabled
                    .then(|| Arc::clone(&metrics_service)),
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
        return EXIT_LISTENER;
    }

    // The first history point is taken before ready (MGT-R32), so every
    // request that sees the server ready also sees that point.
    metrics_service.sample_history();
    lifecycle.mark_ready();
    let ready = std::iter::once(&status_plan)
        .chain(management.iter().map(|bound| &bound.plan))
        .chain(listeners.iter().map(|bound| &bound.plan))
        .map(|plan| (plan.name, plan.addr))
        .collect();
    log.record(&Event::ServerReady { listeners: ready });
    status.ready();
    // Points go on through the shutdown grace, so the task stops with the
    // listeners.
    let sampler_task = Some(tokio::spawn(sample_history(Arc::clone(&metrics_service))));

    // The TCP and WebSocket entry points accept from here on. One ticket use
    // table for WS and WSS (websocket-relay-v2 §1.9.4).
    let ticket_uses = Arc::new(Mutex::new(TicketUseTable::new()));
    let query_service = Arc::new(ManagementQueryService::new(
        Arc::clone(&runtime) as Arc<dyn RelayStateSource>,
        Arc::clone(&relay_budget),
        Arc::clone(&admin_budget),
        Arc::clone(&clock),
        Arc::clone(&cell) as Arc<dyn vnc_relay_app::reload::ports::RuntimeSettingsSource>,
    ));
    let terminate_service = Arc::new(ManagementTerminateService::new(
        Arc::clone(&runtime) as Arc<dyn RelaySessionControl>,
        Arc::clone(&clock),
        Arc::clone(&log),
    ));

    // The management listener accepts from here on.
    let management_task = management.map(|bound| {
        let dashboard = DashboardHints::new(
            settings.dashboard.poll,
            settings.dashboard.query_timeout,
            Arc::clone(&cell) as Arc<dyn vnc_relay_app::reload::ports::RuntimeSettingsSource>,
        );
        let listener = ManagementListener::new(
            SupervisedListener::new(bound.listener, bound.plan.name, listener_failures.clone()),
            management_tls_on.then(|| TlsSource::management(Arc::clone(&cell))),
            Arc::clone(&admin_budget),
            Arc::clone(&cell) as Arc<dyn vnc_relay_app::reload::ports::RuntimeSettingsSource>,
            dashboard,
            ManagementServices {
                auth,
                query: query_service,
                terminate: terminate_service,
                config: config_service,
                logs: Arc::clone(&recent_logs),
                metrics: metrics_service,
                sources: sources_service,
            },
        );
        tokio::spawn(listener.run(Arc::clone(&log)))
    });
    let relay_service = Arc::new(RelayService::new(
        HandshakePolicy {
            mode1_enabled: settings.mode1.enabled,
            mode2_enabled: settings.mode2.enabled,
        },
        Arc::clone(&cell) as Arc<dyn vnc_relay_app::reload::ports::RuntimeSettingsSource>,
        Arc::clone(&clock),
        Arc::clone(&log),
        metrics,
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
        match bound.plan.listener {
            Some(listener @ (Listener::TcpViewer | Listener::TcpServer)) => {
                let entry_point = match listener {
                    Listener::TcpViewer => EntryPoint::TcpViewer,
                    _ => EntryPoint::TcpServer,
                };
                let policy = AcceptPolicy {
                    allow: match listener {
                        Listener::TcpViewer => AllowSource::tcp_viewer(Arc::clone(&cell)),
                        _ => AllowSource::tcp_server(Arc::clone(&cell)),
                    },
                    relay_budget: Arc::clone(&relay_budget),
                };
                let entry = TcpEntryPoint::new(
                    entry_point,
                    SupervisedListener::new(
                        bound.listener,
                        bound.plan.name,
                        listener_failures.clone(),
                    ),
                    policy,
                    Arc::clone(&relay_service),
                    Arc::clone(&runtime),
                    Arc::clone(&clock),
                );
                entry_points.spawn(entry.run(Arc::clone(&log), stopped.clone()));
            }
            Some(listener @ (Listener::Ws | Listener::Wss)) => {
                let tls = match listener {
                    Listener::Wss if wss_present => Some(TlsSource::wss(Arc::clone(&cell))),
                    _ => None,
                };
                if matches!(listener, Listener::Wss) && tls.is_none() {
                    idle.push(bound);
                    continue;
                }
                let policy = WsPolicy {
                    accept: AcceptPolicy {
                        allow: match listener {
                            Listener::Wss => AllowSource::wss(Arc::clone(&cell)),
                            _ => AllowSource::ws(Arc::clone(&cell)),
                        },
                        relay_budget: Arc::clone(&relay_budget),
                    },
                    ticket_uses: Arc::clone(&ticket_uses),
                    started_at,
                };
                let entry = WsEntryPoint::new(
                    listener,
                    SupervisedListener::new(
                        bound.listener,
                        bound.plan.name,
                        listener_failures.clone(),
                    ),
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

    let first_request = wait_for_shutdown(requests.wait(), &mut failed_listener).await;
    if let ShutdownRequest::ListenerFailed(failure) = first_request {
        let key = if matches!(failure.listener, "status" | "management") {
            format!("{}.listen", failure.listener)
        } else {
            format!("relay.{}.listen", failure.listener)
        };
        cli::report("error", "listener_failed", &key, &failure.detail());
    }
    status.stopping();
    let exit = begin_process_shutdown(
        first_request,
        &lifecycle,
        &relay_service,
        &runtime,
        cell.current().settings.timeouts.shutdown_grace,
    );
    stop.send_replace(true);

    finish_relay_shutdown(
        &mut entry_points,
        supervisor,
        async {
            requests.wait().await;
        },
        || relay_service.request_shutdown(&runtime, Duration::ZERO),
    )
    .await;
    drop(idle);
    stop_task(management_task).await;
    stop_task(status_task).await;
    stop_task(sampler_task).await;
    let _ = watch_task.await;
    log.record(&Event::ServerStopped);
    exit
}

/// Keeps path resolution identical to the loader and shares the watcher's
/// transaction guard. No file is opened until a management request needs it.
fn config_management<A: Clone + Send + Sync + 'static>(
    config: &Path,
    reload: SharedReload<A>,
    clock: Arc<dyn Clock>,
    log: Arc<dyn EventSink>,
) -> io::Result<Arc<dyn ConfigService>> {
    let config = std::path::absolute(config)?;
    Ok(Arc::new(ConfigEditService::new(
        reload,
        Arc::new(FsConfigFileStore),
        Arc::new(TomlConfigDocumentGateway),
        config.to_path_buf(),
        config.parent().unwrap_or(Path::new("")).to_path_buf(),
        clock,
        log,
    )))
}

/// A closed Windows console (Q102) and a system shutdown of the service
/// (Q248) request Grace and Force in the same turn.
#[derive(Clone, Copy)]
enum ShutdownRequest {
    Requested,
    ListenerFailed(ListenerFailure),
    #[cfg(any(windows, test))]
    CloseNow,
}

async fn wait_for_shutdown(
    requested: impl Future<Output = ShutdownRequest>,
    failed: &mut watch::Receiver<Option<ListenerFailure>>,
) -> ShutdownRequest {
    tokio::select! {
        biased;
        request = requested => request,
        failure = async {
            let failure = {
                match failed.wait_for(Option::is_some).await {
                    Ok(value) => *value,
                    Err(_) => None,
                }
            };
            match failure {
                Some(failure) => Some(failure),
                None => std::future::pending().await,
            }
        } => match failure {
            Some(failure) => ShutdownRequest::ListenerFailed(failure),
            None => std::future::pending().await,
        },
    }
}

fn begin_process_shutdown<C>(
    request: ShutdownRequest,
    lifecycle: &LifecycleService,
    service: &RelayService,
    runtime: &RelayRuntime<C>,
    grace: Duration,
) -> u8 {
    match request {
        ShutdownRequest::ListenerFailed(failure) => {
            lifecycle.listener_failed();
            service.request_fatal_shutdown(runtime, grace, failure.detail());
            EXIT_RUNTIME
        }
        requested => {
            lifecycle.begin_shutdown();
            begin_relay_shutdown(requested, grace, |grace| {
                service.request_shutdown(runtime, grace)
            });
            EXIT_SUCCESS
        }
    }
}
fn begin_relay_shutdown(
    request: ShutdownRequest,
    grace: Duration,
    mut request_shutdown: impl FnMut(Duration),
) {
    request_shutdown(grace);
    match request {
        ShutdownRequest::Requested | ShutdownRequest::ListenerFailed(_) => {}
        #[cfg(any(windows, test))]
        ShutdownRequest::CloseNow => request_shutdown(Duration::ZERO),
    }
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

fn startup_keys_failure(log: &dyn EventSink, error: &dyn Display) -> u8 {
    runtime_failure(log, "운영체제 난수원으로 시작 키를 만들 수 없다", error)
}

/// Takes one history point every one-hour interval (10 seconds) after the
/// ready point that `listen_until_shutdown` takes itself (management-api-v1
/// §1.16). A late tick moves the next one back, so two points are never
/// taken at once.
async fn sample_history(metrics: Arc<MetricsService>) {
    let mut ticks = tokio::time::interval(HistoryWindow::Hour.interval());
    ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    // The first tick completes at once. The ready point is already taken.
    ticks.tick().await;
    loop {
        ticks.tick().await;
        metrics.sample_history();
    }
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
    /// Config key prefix of the two files, such as `relay.wss`.
    prefix: &'static str,
    /// `listener` value of the operation log (ops-info-v1 §1.5).
    listener: &'static str,
    files: &'a TlsFiles,
}

/// The enabled WSS listener, then management, when they set TLS files.
fn tls_checks(settings: &Settings) -> Vec<TlsCheck<'_>> {
    let mut checks = Vec::new();
    let wss = &settings.relay.wss;
    if let (true, Some(files)) = (wss.enabled(), wss.tls.as_ref()) {
        checks.push(TlsCheck {
            prefix: "relay.wss",
            listener: Listener::Wss.name(),
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
    let key = report_tls(check, error);
    record_failure(log, error.code(), Some(key), Some(check.listener), None);
}

/// Writes the diagnostic of one TLS file error and returns its config key.
fn report_tls(check: &TlsCheck<'_>, error: &TlsError) -> String {
    let file = match error.file() {
        TlsFile::Cert => "cert_file",
        TlsFile::Key => "key_file",
    };
    let key = format!("{}.{file}", check.prefix);
    cli::report("error", error.code(), &key, &error.to_string());
    key
}

/// Account file diagnostics may quote file paths inside the file, so the log
/// line carries the config key of the file and no detail (ops-info-v1 §1.11).
fn accounts_failure(log: &dyn EventSink, diagnostics: &[AccountsDiagnostic]) -> u8 {
    report_accounts(diagnostics);
    for d in diagnostics {
        record_failure(
            log,
            d.error.code(),
            Some(ACCOUNTS_FILE_KEY.to_owned()),
            None,
            None,
        );
    }
    EXIT_ACCOUNTS
}

fn report_accounts(diagnostics: &[AccountsDiagnostic]) {
    for d in diagnostics {
        cli::report("error", d.error.code(), &d.key, &d.message);
    }
}

fn report_accounts_warnings(warnings: &[accounts::AccountsWarning]) {
    for warning in warnings {
        cli::report(
            "warning",
            warning.code(),
            ACCOUNTS_FILE_KEY,
            &warning.message(),
        );
    }
}

fn runtime_failure(log: &dyn EventSink, what: &str, error: &dyn Display) -> u8 {
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
    EXIT_RUNTIME
}

/// A watch that cannot register at start refuses the start (file-watch.md §6, OPS-028): one
/// `watch_unavailable` diagnostic, one `startup_failed` line, exit code 1.
fn watch_unavailable(log: &dyn EventSink, detail: &str) -> u8 {
    cli::report(
        "error",
        "watch_unavailable",
        "",
        &format!("파일 변경 감지를 준비할 수 없다({detail})."),
    );
    record_failure(log, "watch_unavailable", None, None, None);
    EXIT_RUNTIME
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

fn report_log_reopen_failure(error: &vnc_relay_adapters::ops_log::LogFileError) {
    cli::report(
        "warning",
        "log_reopen_failed",
        LOG_OUTPUT_KEY,
        &error.to_string(),
    );
}

/// Shutdown requests of the host.
enum Requests {
    Terminal(ShutdownSignal),
    /// Stop and system shutdown controls of the SCM (cli-v1 §1.9). Console
    /// signals are not shutdown requests of a service.
    #[cfg(windows)]
    Service(tokio::sync::mpsc::UnboundedReceiver<StopControl>),
}

impl Requests {
    async fn wait(&mut self) -> ShutdownRequest {
        match self {
            Self::Terminal(signal) => signal.wait().await,
            #[cfg(windows)]
            Self::Service(controls) => match controls.recv().await {
                Some(StopControl::Stop) => ShutdownRequest::Requested,
                Some(StopControl::SystemShutdown) => ShutdownRequest::CloseNow,
                // The control handler keeps the sender until the process ends.
                None => std::future::pending().await,
            },
        }
    }
}

/// State changes that the host hears about: the SCM status of a service run.
/// A terminal hears nothing.
#[derive(Default)]
struct HostStatus {
    #[cfg(windows)]
    service: Option<StatusReporter>,
}

impl HostStatus {
    /// The server entered ready (cli-v1 §1.9 "상태 보고").
    fn ready(&self) {
        #[cfg(windows)]
        if let Some(status) = &self.service {
            status.running();
        }
    }

    /// The shutdown began, by a request or by a listener failure.
    fn stopping(&self) {
        #[cfg(windows)]
        if let Some(status) = &self.service {
            status.stopping();
        }
    }
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

    async fn wait(&mut self) -> ShutdownRequest {
        std::future::poll_fn(|cx| {
            if self.interrupt.poll_recv(cx).is_ready() || self.terminate.poll_recv(cx).is_ready() {
                Poll::Ready(ShutdownRequest::Requested)
            } else {
                Poll::Pending
            }
        })
        .await
    }
}

/// Shutdown requests of cli-v1 §1.8: Ctrl+C, Ctrl+Break, and console close.
#[cfg(windows)]
struct ShutdownSignal {
    ctrl_c: tokio::signal::windows::CtrlC,
    ctrl_break: tokio::signal::windows::CtrlBreak,
    ctrl_close: tokio::signal::windows::CtrlClose,
}

#[cfg(windows)]
impl ShutdownSignal {
    fn register() -> io::Result<Self> {
        Ok(Self {
            ctrl_c: tokio::signal::windows::ctrl_c()?,
            ctrl_break: tokio::signal::windows::ctrl_break()?,
            ctrl_close: tokio::signal::windows::ctrl_close()?,
        })
    }

    async fn wait(&mut self) -> ShutdownRequest {
        std::future::poll_fn(|cx| {
            // Close takes priority if an ordinary request is also pending.
            if self.ctrl_close.poll_recv(cx).is_ready() {
                Poll::Ready(ShutdownRequest::CloseNow)
            } else if self.ctrl_c.poll_recv(cx).is_ready()
                || self.ctrl_break.poll_recv(cx).is_ready()
            {
                Poll::Ready(ShutdownRequest::Requested)
            } else {
                Poll::Pending
            }
        })
        .await
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

    #[test]
    fn ordinary_shutdown_request_starts_the_configured_grace_once() {
        let mut requests = Vec::new();
        begin_relay_shutdown(
            ShutdownRequest::Requested,
            Duration::from_secs(90),
            |grace| {
                requests.push(grace);
            },
        );
        assert_eq!(requests, [Duration::from_secs(90)]);
    }

    #[test]
    fn console_close_starts_grace_then_immediately_requests_force() {
        let mut requests = Vec::new();
        begin_relay_shutdown(
            ShutdownRequest::CloseNow,
            Duration::from_secs(90),
            |grace| requests.push(grace),
        );
        assert_eq!(requests, [Duration::from_secs(90), Duration::ZERO]);
    }

    #[test]
    fn console_close_with_zero_grace_still_preserves_both_requests() {
        let mut requests = Vec::new();
        begin_relay_shutdown(ShutdownRequest::CloseNow, Duration::ZERO, |grace| {
            requests.push(grace);
        });
        assert_eq!(requests, [Duration::ZERO, Duration::ZERO]);
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
        assert_eq!(code, EXIT_RUNTIME);
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
        assert_eq!(code, EXIT_RUNTIME);
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

#[cfg(test)]
#[path = "serve/listener_tests.rs"]
mod listener_tests;
