//! `serve` start sequence and shutdown wait (cli-v1 §1.8).
//! Design: docs/architectures/serve-startup.md.

use std::io;
use std::path::Path;
use std::process::ExitCode;
use std::task::Poll;

use vnc_relay_adapters::{accounts_file, config_file, net};
use vnc_relay_app::accounts::{self, ACCOUNTS_FILE_KEY, AccountsDiagnostic, AccountsWarning};
use vnc_relay_app::settings::Settings;

use crate::cli::{self, EXIT_ACCOUNTS, EXIT_CONFIG, EXIT_LISTENER, EXIT_RUNTIME};

/// Runs the server until the first shutdown request. Each failing stage
/// writes all its diagnostics to stderr and returns its exit code.
pub fn run(config: &Path) -> ExitCode {
    // Stage 1: config.
    let settings = match config_file::load(config) {
        Ok(settings) => settings,
        Err(diagnostics) => {
            for d in &diagnostics {
                cli::report("error", d.error.code(), &d.key, &d.message);
            }
            return ExitCode::from(EXIT_CONFIG);
        }
    };

    // Stage 2: account file. Stage 3 (file watch) comes with task 5.1.
    let loaded = match accounts_file::load(&settings.admin.accounts_file) {
        Ok(loaded) => loaded,
        Err(diagnostics) => return accounts_failure(&diagnostics),
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
        return accounts_failure(&[diagnostic]);
    }

    // Stage 4: runtime, then listeners.
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            return runtime_failure(&format!("비동기 런타임을 만들 수 없다({error})."));
        }
    };
    runtime.block_on(listen_until_shutdown(&settings))
}

async fn listen_until_shutdown(settings: &Settings) -> ExitCode {
    // Registered before binding so that a request during startup is not lost.
    let mut shutdown = match ShutdownSignal::register() {
        Ok(shutdown) => shutdown,
        Err(error) => {
            return runtime_failure(&format!("종료 신호 처리기를 준비할 수 없다({error})."));
        }
    };
    let listeners = match net::bind(&settings.listeners()).await {
        Ok(listeners) => listeners,
        Err(failures) => {
            for failure in &failures {
                cli::report(
                    "error",
                    "listener_bind_failed",
                    failure.plan.key,
                    &failure.message,
                );
            }
            return ExitCode::from(EXIT_LISTENER);
        }
    };
    shutdown.wait().await;
    drop(listeners);
    ExitCode::SUCCESS
}

fn accounts_failure(diagnostics: &[AccountsDiagnostic]) -> ExitCode {
    for d in diagnostics {
        cli::report("error", d.error.code(), &d.key, &d.message);
    }
    ExitCode::from(EXIT_ACCOUNTS)
}

fn runtime_failure(message: &str) -> ExitCode {
    cli::report("error", "runtime_unavailable", "", message);
    ExitCode::from(EXIT_RUNTIME)
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
