//! Real-process failure diagnostics and recovery, shared by Unix and ConPTY.
use std::io::ErrorKind;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const LIMIT: Duration = Duration::from_secs(20);
const PREFIX: &str = "warning[log_reopen_failed] key=log.output: ";

pub trait LogProcess: Sized {
    fn start(config: &Path) -> Self;
    fn output(&self) -> String;
    fn request_stop(&mut self);
    fn try_wait(&mut self) -> Option<i32>;
    fn finish_output(&mut self) -> String;
}

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn read(path: &Path) -> String {
    match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => String::new(),
        Err(e) => panic!("read log {}: {e}", path.display()),
    }
}
fn count(path: &Path, event: &str) -> usize {
    read(path)
        .split_inclusive('\n')
        .filter(|line| line.ends_with('\n') && line.contains(&format!("\"event\":\"{event}\"")))
        .count()
}
fn wait<P: LogProcess>(process: &mut P, mut ready: impl FnMut(&P) -> bool) {
    let start = Instant::now();
    while !ready(process) {
        assert!(
            process.try_wait().is_none(),
            "unexpected exit: {}",
            process.output()
        );
        assert!(
            start.elapsed() < LIMIT,
            "condition deadline: {}",
            process.output()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
// ConPTY may encode new rows as cursor positions instead of LF and emits an
// OSC window title. Preserve printable diagnostic text and terminal row breaks.
pub fn plain(text: &str) -> String {
    let mut chars = text.chars();
    let mut out = String::new();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            match chars.next() {
                Some('[') => {
                    for code in chars.by_ref() {
                        if ('@'..='~').contains(&code) {
                            if matches!(code, 'H' | 'f' | 'E') {
                                out.push('\n');
                            }
                            break;
                        }
                    }
                }
                Some(']') => {
                    let mut escaped = false;
                    for code in chars.by_ref() {
                        if code == '\u{7}' || (escaped && code == '\\') {
                            break;
                        }
                        escaped = code == '\u{1b}';
                    }
                }
                _ => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}
fn warning_lines(text: &str) -> Vec<String> {
    plain(text)
        .split(['\r', '\n'])
        .filter(|line| line.starts_with(PREFIX))
        .map(str::to_owned)
        .collect()
}

// If the watcher recreates the file before mkdir, rotate that empty file and
// retry. No failure streak starts in that case. Once mkdir succeeds, leave the
// directory in place until the failure and old-handle writes are observed.
fn block_path<P: LogProcess>(process: &mut P, path: &Path, dir: &Path) -> PathBuf {
    let start = Instant::now();
    let mut attempt = 0_u64;
    loop {
        assert!(
            start.elapsed() < LIMIT,
            "could not replace log path by directory"
        );
        let archived = dir.join(format!("failed-{attempt}.log"));
        std::fs::rename(path, &archived).unwrap_or_else(|e| panic!("rotate for failure: {e}"));
        match std::fs::create_dir(path) {
            Ok(()) => return archived,
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                assert!(path.is_file(), "unexpected existing log path");
                assert!(
                    process.try_wait().is_none(),
                    "process exited during rotation"
                );
                std::thread::sleep(Duration::from_millis(10));
                attempt += 1;
            }
            Err(e) => panic!("block log path: {e}"),
        }
    }
}

pub fn exercise_failure_and_recovery<P: LogProcess>() {
    let holders: Vec<_> = (0..3)
        .map(|_| TcpListener::bind("127.0.0.1:0").unwrap_or_else(|e| panic!("reserve port: {e}")))
        .collect();
    let ports: Vec<_> = holders
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
    let dir = std::env::temp_dir().join(format!("vr-log-failure-{}-{nonce}", std::process::id()));
    std::fs::create_dir(&dir).unwrap_or_else(|e| panic!("fixture: {e}"));
    let fixture = Fixture(dir);
    let path = fixture.0.join("relay.log");
    let config = fixture.0.join("relay.toml");
    std::fs::write(fixture.0.join("accounts.toml"), "version = 1\n[[account]]\nname = 'operator'\npassword_hash = '$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA'\n").unwrap_or_else(|e| panic!("accounts: {e}"));
    std::fs::write(&config, format!("[admin]\naccounts_file = 'accounts.toml'\n[management]\nlisten = '127.0.0.1:{}'\n[status]\nlisten = '127.0.0.1:{}'\n[relay.tcp_viewer]\nenabled = true\nlisten = '127.0.0.1:{}'\nallow = ['127.0.0.1']\n[log]\noutput = 'relay.log'\n[reload]\npoll_secs = 5\n", ports[0], ports[1], ports[2])).unwrap_or_else(|e| panic!("config: {e}"));
    drop(holders);
    let mut process = P::start(&config);
    wait(&mut process, |_| {
        read(&path).contains("\"event\":\"server_ready\"")
    });
    let archived = block_path(&mut process, &path, &fixture.0);
    wait(&mut process, |p| {
        !warning_lines(&p.output()).is_empty() && count(&archived, "log_reopen_failed") == 1
    });
    let failure_start = Instant::now();
    let address = format!("127.0.0.1:{}", ports[2]);
    for n in 1..=5 {
        drop(
            TcpStream::connect(&address)
                .unwrap_or_else(|e| panic!("connect while path blocked: {e}")),
        );
        wait(&mut process, |_| {
            count(&archived, "connection_accepted") == n
        });
    }
    // Observe at least two further Dp attempts while failure remains active.
    wait(&mut process, |p| {
        assert_eq!(warning_lines(&p.output()).len(), 1);
        assert_eq!(count(&archived, "log_reopen_failed"), 1);
        failure_start.elapsed() >= Duration::from_millis(10200)
    });
    std::fs::remove_dir(&path).unwrap_or_else(|e| panic!("restore log path: {e}"));
    wait(&mut process, |_| path.is_file());
    for n in 1..=3 {
        drop(
            TcpStream::connect(&address).unwrap_or_else(|e| panic!("connect after recovery: {e}")),
        );
        wait(&mut process, |_| count(&path, "connection_accepted") == n);
    }
    assert_eq!(count(&archived, "connection_accepted"), 5);
    assert_eq!(count(&path, "log_reopen_failed"), 0);
    process.request_stop();
    let stopped = Instant::now();
    loop {
        if let Some(code) = process.try_wait() {
            assert_eq!(code, 0, "normal shutdown failed: {}", process.output());
            break;
        }
        assert!(
            stopped.elapsed() < LIMIT,
            "shutdown deadline: {}",
            process.output()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = process.finish_output();
    let warnings = warning_lines(&output);
    assert_eq!(warnings.len(), 1, "output: {output}");
    assert!(!warnings[0].contains("relay.log"));
    assert!(!warnings[0].contains(&fixture.0.to_string_lossy().to_string()));
    assert_eq!(count(&archived, "log_reopen_failed"), 1);
    let line = read(&archived)
        .lines()
        .find(|line| line.contains("\"event\":\"log_reopen_failed\""))
        .unwrap_or_else(|| panic!("failure event"))
        .to_owned();
    for field in [
        "\"level\":\"warn\"",
        "\"reason\":\"log_file_unavailable\"",
        "\"config_key\":\"log.output\"",
    ] {
        assert!(line.contains(field), "missing {field}: {line}");
    }
    assert!(!line.contains("relay.log"));
    assert!(read(&path).ends_with("\"event\":\"server_stopped\"}\n"));
}
