//! Real-process log rotation on Windows and Linux. Deterministic notification
//! suppression and Dp/failure-streak coverage live in the adapter tests.
use std::collections::BTreeSet;
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[cfg(unix)]
#[path = "common/log_reopen.rs"]
mod log_reopen;

struct Server {
    child: Child,
    stderr: Arc<Mutex<String>>,
    reader: Option<thread::JoinHandle<()>>,
    dir: Option<PathBuf>,
}
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        if let Some(dir) = &self.dir {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

impl Server {
    fn start(config: &Path, dir: Option<PathBuf>) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_vnc-relay"))
            .args(["serve", "--config"])
            .arg(config)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("spawn: {e}"));
        let mut pipe = child.stderr.take().unwrap_or_else(|| panic!("stderr pipe"));
        let stderr = Arc::new(Mutex::new(String::new()));
        let copy = stderr.clone();
        let reader = thread::spawn(move || {
            let mut bytes = [0_u8; 2048];
            loop {
                match pipe.read(&mut bytes) {
                    Ok(0) | Err(_) => return,
                    Ok(n) => copy
                        .lock()
                        .unwrap_or_else(|e| panic!("stderr lock: {e}"))
                        .push_str(&String::from_utf8_lossy(&bytes[..n])),
                }
            }
        });
        Self {
            child,
            stderr,
            reader: Some(reader),
            dir,
        }
    }
}

#[cfg(unix)]
impl log_reopen::LogProcess for Server {
    fn start(config: &Path) -> Self {
        Self::start(config, None)
    }
    fn output(&self) -> String {
        self.stderr
            .lock()
            .unwrap_or_else(|e| panic!("stderr lock: {e}"))
            .clone()
    }
    fn request_stop(&mut self) {
        let status = Command::new("kill")
            .arg("-TERM")
            .arg(self.child.id().to_string())
            .status()
            .unwrap_or_else(|e| panic!("SIGTERM: {e}"));
        assert!(status.success(), "SIGTERM failed: {status}");
    }
    fn try_wait(&mut self) -> Option<i32> {
        self.child
            .try_wait()
            .unwrap_or_else(|e| panic!("poll child: {e}"))
            .map(|status| {
                status
                    .code()
                    .unwrap_or_else(|| panic!("child terminated by signal"))
            })
    }
    fn finish_output(&mut self) -> String {
        if let Some(reader) = self.reader.take() {
            reader
                .join()
                .unwrap_or_else(|_| panic!("stderr reader panicked"));
        }
        self.output()
    }
}

/// Checks actual stderr, old-handle writes, recovery and normal shutdown.
/// Notification suppression and subsequent failure streaks are adapter coverage.
/// spec: operations.md:OPS-045 partial
#[cfg(unix)]
#[test]
fn log_reopen_failure_warns_once_keeps_old_file_and_recovers() {
    log_reopen::exercise_failure_and_recovery::<Server>();
}
fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}
fn wait_for(server: &mut Server, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !condition() {
        assert!(
            server
                .child
                .try_wait()
                .unwrap_or_else(|e| panic!("poll process: {e}"))
                .is_none(),
            "process exited: {}",
            server
                .stderr
                .lock()
                .unwrap_or_else(|e| panic!("stderr: {e}"))
        );
        assert!(
            Instant::now() < deadline,
            "condition deadline: {}",
            server
                .stderr
                .lock()
                .unwrap_or_else(|e| panic!("stderr: {e}"))
        );
        thread::sleep(Duration::from_millis(10));
    }
}
fn accepted(text: &str) -> Vec<String> {
    text.split_inclusive('\n')
        .filter(|line| line.ends_with('\n'))
        .filter(|line| line.contains("\"event\":\"connection_accepted\""))
        .map(|line| {
            line.split_once("\"conn_id\":\"")
                .and_then(|(_, rest)| rest.split_once('"'))
                .map(|(id, _)| id.to_owned())
                .unwrap_or_else(|| panic!("missing conn_id: {line}"))
        })
        .collect()
}

/// spec: operations.md:OPS-045 partial
#[test]
fn renamed_log_is_recreated_without_lost_or_duplicate_lines() {
    let holders: Vec<TcpListener> = (0..3)
        .map(|_| TcpListener::bind("127.0.0.1:0").unwrap_or_else(|e| panic!("reserve port: {e}")))
        .collect();
    let ports: Vec<u16> = holders
        .iter()
        .map(|listener| {
            listener
                .local_addr()
                .unwrap_or_else(|e| panic!("port: {e}"))
                .port()
        })
        .collect();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|e| panic!("time: {e}"))
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("vr-log-process-{}-{nonce}", std::process::id()));
    std::fs::create_dir(&dir).unwrap_or_else(|e| panic!("temp dir: {e}"));
    // The log is outside the config directory, so it needs its own directory registration.
    let log_dir = dir.join("logs");
    std::fs::create_dir(&log_dir).unwrap_or_else(|e| panic!("log dir: {e}"));
    let path = log_dir.join("relay.log");
    let rotated = log_dir.join("rotated.log");
    let config = dir.join("relay.toml");
    std::fs::write(dir.join("accounts.toml"), "version = 1\n[[account]]\nname = 'operator'\npassword_hash = '$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA'\n").unwrap_or_else(|e| panic!("accounts: {e}"));
    std::fs::write(&config, format!("[admin]\naccounts_file = 'accounts.toml'\n[management]\nlisten = '127.0.0.1:{}'\n[status]\nlisten = '127.0.0.1:{}'\n[relay.tcp_viewer]\nenabled = true\nlisten = '127.0.0.1:{}'\nallow = ['127.0.0.1']\n[log]\noutput = 'logs/relay.log'\n[reload]\npoll_secs = 30\n", ports[0], ports[1], ports[2])).unwrap_or_else(|e| panic!("config: {e}"));
    drop(holders);
    let mut server = Server::start(&config, Some(dir));
    wait_for(&mut server, || {
        read(&path).contains("\"event\":\"server_ready\"")
    });
    let address = format!("127.0.0.1:{}", ports[2]);
    for n in 1..=10 {
        drop(
            TcpStream::connect(&address).unwrap_or_else(|e| panic!("connect before rotation: {e}")),
        );
        wait_for(&mut server, || accepted(&read(&path)).len() == n);
    }
    let start = Instant::now();
    std::fs::rename(&path, &rotated).unwrap_or_else(|e| panic!("rotate: {e}"));
    wait_for(&mut server, || path.is_file());
    assert!(
        start.elapsed() < Duration::from_secs(10),
        "notification must beat Dp=30s"
    );
    for n in 1..=10 {
        drop(
            TcpStream::connect(&address).unwrap_or_else(|e| panic!("connect after rotation: {e}")),
        );
        wait_for(&mut server, || accepted(&read(&path)).len() == n);
    }
    let before = accepted(&read(&rotated));
    let after = accepted(&read(&path));
    assert_eq!(before.len(), 10);
    assert_eq!(after.len(), 10);
    let all: BTreeSet<_> = before.iter().chain(after.iter()).collect();
    assert_eq!(all.len(), 20, "each caused connection is logged once");
    let log = read(&rotated) + &read(&path);
    assert!(!log.contains("\"event\":\"reload_change_detected\""));
    assert!(!log.contains("\"event\":\"log_reopen_failed\""));
}
