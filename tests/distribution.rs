//! Distribution example contract and Linux real-binary operations evidence.

use std::fs;
use std::path::Path;

use vnc_relay_adapters::config_file::load;
use vnc_relay_app::settings::LogOutput;

#[cfg(target_os = "linux")]
#[path = "common/distribution.rs"]
mod distribution;

/// spec: operations.md:OPS-031 partial
#[test]
fn repository_example_loads_without_secrets() {
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/relay.toml");
    let text = fs::read_to_string(&example).unwrap_or_else(|e| panic!("read exact example: {e}"));
    let settings = load(&example).unwrap_or_else(|e| panic!("load exact example: {e:?}"));
    let base = example
        .parent()
        .unwrap_or_else(|| panic!("example has no parent"));
    assert_eq!(
        settings.admin.accounts_file,
        base.join("admin-accounts.toml")
    );
    assert_eq!(settings.log.output, LogOutput::File(base.join("relay.log")));
    assert!(settings.management.listen.ip().is_loopback());
    assert!(
        settings.management.tls.is_none(),
        "example management uses loopback HTTP"
    );
    assert!(settings.status.listen.ip().is_loopback());
    assert!(!settings.relay.ws.enabled());
    assert!(!settings.relay.wss.enabled());
    for entry in [&settings.relay.tcp_viewer, &settings.relay.tcp_server] {
        assert!(entry.enabled);
        assert!(
            entry
                .listen
                .is_some_and(|address| address.ip().is_loopback())
        );
        assert!(
            entry.allow.is_some(),
            "enabled entry has an explicit allow list"
        );
    }
    assert!(settings.mode2.enabled);
    assert!(!settings.mode1.enabled);
    for marker in ["$argon2", "-----BEGIN", "-----END"] {
        assert!(!text.contains(marker), "secret material in example");
    }
    for line in text
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
    {
        if let Some((key, _)) = line.split_once('=') {
            let key = key.trim().to_ascii_lowercase();
            assert!(!key.contains("password"), "credential key in example");
            assert!(!key.contains("hash"), "hash key in example");
        }
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::io::{Read, Write};
    use std::path::Path;
    use std::process::{Child, Command, Stdio};
    use std::sync::{Arc, Mutex, PoisonError};
    use std::thread::JoinHandle;
    use std::time::{Duration, Instant};

    use super::distribution::DistributionProcess;

    const BIN: &str = env!("CARGO_BIN_EXE_vnc-relay");
    const LIMIT: Duration = Duration::from_secs(20);

    // Same script + stty handshake as management_session.rs: a prompt alone
    // does not prove that the product has already disabled terminal echo.
    struct PtySession {
        child: Child,
        transcript: Arc<Mutex<Vec<u8>>>,
        reader: Option<JoinHandle<()>>,
    }

    impl PtySession {
        fn start(config: &Path) -> Self {
            let quote = |text: &str| format!("'{}'", text.replace('\'', "'\\''"));
            let command = format!(
                "printf '__TTY='; tty; {} admin add opsadmin --config {}; echo __RC=$?",
                quote(BIN),
                quote(&config.to_string_lossy())
            );
            let mut child = Command::new("script")
                .args(["-q", "-E", "always", "-c", &command, "/dev/null"])
                .env("SHELL", "/bin/sh")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .unwrap_or_else(|e| panic!("start distribution PTY: {e}"));
            let mut stdout = child
                .stdout
                .take()
                .unwrap_or_else(|| panic!("PTY stdout missing"));
            let transcript = Arc::new(Mutex::new(Vec::new()));
            let sink = Arc::clone(&transcript);
            let reader = std::thread::spawn(move || {
                let mut chunk = [0; 4096];
                loop {
                    let n = stdout
                        .read(&mut chunk)
                        .unwrap_or_else(|e| panic!("read PTY: {e}"));
                    if n == 0 {
                        break;
                    }
                    sink.lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .extend_from_slice(&chunk[..n]);
                }
            });
            Self {
                child,
                transcript,
                reader: Some(reader),
            }
        }

        fn text(&self) -> String {
            String::from_utf8_lossy(
                &self
                    .transcript
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner),
            )
            .replace("\r\n", "\n")
        }

        fn wait_for(&self, marker: &str) {
            let deadline = Instant::now() + LIMIT;
            while !self.text().contains(marker) {
                assert!(
                    Instant::now() < deadline,
                    "missing PTY {marker:?}: {}",
                    self.text()
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        }

        fn password(&mut self, prompt: &str, password: &str) {
            self.wait_for(prompt);
            let text = self.text();
            let terminal = text
                .lines()
                .find_map(|line| line.strip_prefix("__TTY="))
                .unwrap_or_else(|| panic!("PTY path missing: {text}"));
            let deadline = Instant::now() + LIMIT;
            loop {
                let mode = Command::new("stty")
                    .args(["-F", terminal, "-a"])
                    .output()
                    .unwrap_or_else(|e| panic!("read PTY mode: {e}"));
                assert!(mode.status.success(), "stty failed: {:?}", mode.status);
                if String::from_utf8_lossy(&mode.stdout)
                    .split_whitespace()
                    .any(|word| word == "-echo")
                {
                    break;
                }
                assert!(Instant::now() < deadline, "PTY echo stayed enabled");
                std::thread::sleep(Duration::from_millis(10));
            }
            let stdin = self
                .child
                .stdin
                .as_mut()
                .unwrap_or_else(|| panic!("PTY stdin missing"));
            stdin
                .write_all(format!("{password}\n").as_bytes())
                .unwrap_or_else(|e| panic!("type password: {e}"));
            stdin.flush().unwrap_or_else(|e| panic!("flush PTY: {e}"));
        }

        fn finish(mut self) -> String {
            self.wait_for("__RC=");
            self.child.stdin.take();
            let deadline = Instant::now() + LIMIT;
            loop {
                if let Some(status) = self
                    .child
                    .try_wait()
                    .unwrap_or_else(|e| panic!("wait PTY: {e}"))
                {
                    assert!(status.success(), "PTY failed: {status}");
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "PTY did not finish: {}",
                    self.text()
                );
                std::thread::sleep(Duration::from_millis(10));
            }
            if let Some(reader) = self.reader.take() {
                reader
                    .join()
                    .unwrap_or_else(|_| panic!("PTY reader panicked"));
            }
            self.text()
        }
    }

    impl Drop for PtySession {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }

    pub struct UnixProcess {
        child: Child,
    }

    impl DistributionProcess for UnixProcess {
        fn start(config: &Path) -> Self {
            let errors = std::fs::File::create(config.with_file_name("stderr.log"))
                .unwrap_or_else(|e| panic!("create distribution stderr: {e}"));
            let child = Command::new(BIN)
                .args(["serve", "--config"])
                .arg(config)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::from(errors))
                .spawn()
                .unwrap_or_else(|e| panic!("start distribution server: {e}"));
            Self { child }
        }

        fn request_stop(&mut self) {
            let status = Command::new("kill")
                .arg("-TERM")
                .arg(self.child.id().to_string())
                .status()
                .unwrap_or_else(|e| panic!("distribution SIGTERM: {e}"));
            assert!(status.success(), "SIGTERM failed: {status}");
        }

        fn try_wait(&mut self) -> Option<i32> {
            self.child
                .try_wait()
                .unwrap_or_else(|e| panic!("wait distribution server: {e}"))
                .map(|status| {
                    status
                        .code()
                        .unwrap_or_else(|| panic!("server killed by signal: {status}"))
                })
        }
    }

    impl Drop for UnixProcess {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }

    pub fn prepare_account(config: &Path, password: &str) {
        let mut terminal = PtySession::start(config);
        terminal.password("비밀번호: ", password);
        terminal.password("비밀번호 확인: ", password);
        let transcript = terminal.finish();
        assert!(
            transcript.contains("__RC=0"),
            "admin add failed: {transcript}"
        );
        assert!(
            !transcript.contains(password),
            "typed password in PTY transcript"
        );
        assert!(!transcript.contains("$argon2"), "hash in PTY transcript");
    }
}

/// spec: operations.md:OPS-019
/// spec: operations.md:OPS-031 partial
#[cfg(target_os = "linux")]
#[test]
fn example_cli_account_relay_reload_and_graceful_stop() {
    distribution::exercise_example::<linux::UnixProcess>(linux::prepare_account);
}
