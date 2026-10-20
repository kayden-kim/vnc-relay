//! Operator input of the `admin` commands (cli-v1 §1.5): hidden password
//! entry, one visible confirmation line, and Ctrl+C detection.
//! Design: docs/architectures/admin-accounts.md §5.

use std::io::{self, ErrorKind, IsTerminal as _, Write as _};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use vnc_relay_app::ports::{AccountInput, InputError};

/// How often a read on a helper thread checks for Ctrl+C.
const CANCEL_POLL: Duration = Duration::from_millis(50);

/// How long a read that is still pending when Ctrl+C is seen may finish before
/// the caller gives up. A reader that returns in time has finished with the
/// terminal. A read still blocked after this (on Windows it stays blocked) is
/// ended by the process exit.
const CANCEL_GRACE: Duration = Duration::from_millis(500);

/// The console of the operator. Prompts go to stderr.
#[derive(Debug)]
pub struct Terminal {
    canceled: Arc<AtomicBool>,
}

impl Terminal {
    /// Starts the Ctrl+C detector. It lives until the process ends. From now
    /// on Ctrl+C sets a flag instead of ending the process, so rpassword can
    /// give the terminal settings back first on Unix. On Windows a blocked read
    /// keeps some of them (docs/architectures/admin-accounts.md §9). The error text is Korean.
    pub fn open() -> Result<Self, String> {
        let canceled = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&canceled);
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
        thread::Builder::new()
            .name("ctrl-c".to_owned())
            .spawn(move || watch_interrupt(&flag, &ready_tx))
            .map_err(|e| format!("입력 취소(Ctrl+C) 감지 스레드를 만들 수 없다: {e}."))?;
        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self { canceled }),
            Ok(Err(message)) => Err(message),
            Err(_) => Err("입력 취소(Ctrl+C) 감지 스레드가 준비 전에 끝났다.".to_owned()),
        }
    }

    /// Standard input is a terminal and no Ctrl+C came yet.
    fn ready(&self) -> Result<(), InputError> {
        if !io::stdin().is_terminal() {
            Err(InputError::NotInteractive)
        } else if self.canceled() {
            Err(InputError::Canceled)
        } else {
            Ok(())
        }
    }

    /// Runs `read` on a helper thread and waits for it while checking Ctrl+C.
    /// The read must not run on the caller's thread: a Ctrl+C does not end a
    /// blocked console read there, so the flag would never be seen.
    fn read_on_thread<T: Send + 'static>(
        &self,
        name: &str,
        read: impl FnOnce() -> io::Result<T> + Send + 'static,
    ) -> Result<io::Result<T>, InputError> {
        let (tx, rx) = mpsc::channel();
        thread::Builder::new()
            .name(name.to_owned())
            .spawn(move || {
                let _ = tx.send(read());
            })
            .map_err(|_| InputError::Canceled)?;
        loop {
            if self.canceled() {
                // The read is abandoned. A read that is still pending gets
                // `CANCEL_GRACE` to finish first.
                let _ = rx.recv_timeout(CANCEL_GRACE);
                return Err(InputError::Canceled);
            }
            match rx.recv_timeout(CANCEL_POLL) {
                Ok(_) if self.canceled() => return Err(InputError::Canceled),
                Ok(result) => return Ok(result),
                Err(RecvTimeoutError::Timeout) => {}
                // The reader ended without a result, which only a panic does.
                Err(RecvTimeoutError::Disconnected) => return Err(InputError::Canceled),
            }
        }
    }
}

/// Runs on the detector thread. Sends the setup result once, then sets
/// `flag` on every Ctrl+C.
fn watch_interrupt(flag: &AtomicBool, ready: &mpsc::Sender<Result<(), String>>) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => {
            let _ = ready.send(Err(format!(
                "입력 취소(Ctrl+C) 감지기를 준비할 수 없다: {e}."
            )));
            return;
        }
    };
    runtime.block_on(async {
        #[cfg(unix)]
        let listener = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt());
        #[cfg(windows)]
        let listener = tokio::signal::windows::ctrl_c();
        let mut listener = match listener {
            Ok(listener) => listener,
            Err(e) => {
                let _ = ready.send(Err(format!(
                    "입력 취소(Ctrl+C) 감지기를 등록할 수 없다: {e}."
                )));
                return;
            }
        };
        let _ = ready.send(Ok(()));
        while listener.recv().await.is_some() {
            flag.store(true, Ordering::SeqCst);
        }
    });
}

fn write_prompt(prompt: &str) {
    let mut err = io::stderr().lock();
    let _ = err.write_all(prompt.as_bytes());
    let _ = err.flush();
}

/// Ends the prompt line after a cancel, so the next message starts on a new line.
fn end_canceled_prompt<T>(outcome: Result<T, InputError>) -> Result<T, InputError> {
    if matches!(outcome, Err(InputError::Canceled)) {
        write_prompt("\n");
    }
    outcome
}

impl AccountInput for Terminal {
    fn hidden(&self, prompt: &str) -> Result<String, InputError> {
        self.ready()?;
        write_prompt(prompt);
        let outcome = self
            .read_on_thread("password", rpassword::read_password)
            .and_then(|result| {
                result.map_err(|error| match error.kind() {
                    // Ctrl+C, or Ctrl+D before any text.
                    ErrorKind::Interrupted | ErrorKind::UnexpectedEof => InputError::Canceled,
                    ErrorKind::InvalidData => InputError::InvalidText,
                    _ => InputError::NotInteractive,
                })
            });
        end_canceled_prompt(outcome)
    }

    /// Reads one visible line on a helper thread.
    fn line(&self, prompt: &str) -> Result<Option<String>, InputError> {
        self.ready()?;
        write_prompt(prompt);
        let read = || {
            let mut line = String::new();
            io::stdin()
                .read_line(&mut line)
                .map(|n| (n > 0).then_some(line))
        };
        let outcome = self.read_on_thread("stdin", read).and_then(|result| {
            result.map_err(|error| match error.kind() {
                ErrorKind::InvalidData => InputError::InvalidText,
                _ => InputError::Canceled,
            })
        });
        end_canceled_prompt(outcome)
    }

    fn canceled(&self) -> bool {
        self.canceled.load(Ordering::SeqCst)
    }
}
