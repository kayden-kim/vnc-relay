//! `serve` start sequence and shutdown wait (cli-v1 §1.8).
//! Design: docs/architectures/serve-startup.md, docs/architectures/ops-log.md.

use std::io;
use std::path::Path;
use std::process::ExitCode;
use std::task::Poll;

use vnc_relay_adapters::ops_log::JsonLinesEventSink;
use vnc_relay_adapters::{accounts_file, config_file, net};
use vnc_relay_app::accounts::{self, ACCOUNTS_FILE_KEY, AccountsDiagnostic, AccountsWarning};
use vnc_relay_app::events::{Event, Phase};
use vnc_relay_app::ports::EventSink;
use vnc_relay_app::settings::{LogOutput, Settings};

use crate::cli::{self, EXIT_ACCOUNTS, EXIT_CONFIG, EXIT_LISTENER, EXIT_RUNTIME};

/// Config key of the log output (config-v1 §1.2).
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
    let log = match JsonLinesEventSink::open(&settings.log.output, report_log_write_failure) {
        Ok(log) => log,
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

    // Stage 2: account file. Stage 3 (file watch) comes with task 5.1.
    let loaded = match accounts_file::load(&settings.admin.accounts_file) {
        Ok(loaded) => loaded,
        Err(diagnostics) => return accounts_failure(&log, &diagnostics),
    };
    for warning in &loaded.warnings {
        let message = match warning {
            AccountsWarning::FilePermissive => {
                "그룹이나 다른 사용자에게 계정 파일 권한이 있다. 권한을 0600으로 바꾸기를 권장한다."
            }
        };
        cli::report("warning", warning.code(), ACCOUNTS_FILE_KEY, message);
    }
    if let Err(diagnostic) = accounts::require_any(&loaded.accounts) {
        return accounts_failure(&log, &[diagnostic]);
    }

    // Stage 4: runtime, then listeners.
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => return runtime_failure(&log, "비동기 런타임을 만들 수 없다", &error),
    };
    runtime.block_on(listen_until_shutdown(&settings, &log))
}

async fn listen_until_shutdown(settings: &Settings, log: &dyn EventSink) -> ExitCode {
    // Registered before binding so that a request during startup is not lost.
    let mut shutdown = match ShutdownSignal::register() {
        Ok(shutdown) => shutdown,
        Err(error) => return runtime_failure(log, "종료 신호 처리기를 준비할 수 없다", &error),
    };
    let listeners = match net::bind(&settings.listeners()).await {
        Ok(listeners) => listeners,
        Err(failures) => {
            for failure in &failures {
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
            return ExitCode::from(EXIT_LISTENER);
        }
    };
    shutdown.wait().await;
    drop(listeners);
    log.record(&Event::ServerStopped);
    ExitCode::SUCCESS
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

fn runtime_failure(log: &dyn EventSink, what: &str, error: &io::Error) -> ExitCode {
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
