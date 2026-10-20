//! System tests of `vnc-relay serve` start and stop: the start order, exit
//! codes, and diagnostic lines of docs/contracts/cli-v1.md §1.5, §1.8, §2.

use std::io::Read;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

const BIN: &str = env!("CARGO_BIN_EXE_vnc-relay");

/// 16 and 32 zero bytes in unpadded Base64. Test data, not a real hash.
const SALT: &str = "AAAAAAAAAAAAAAAAAAAAAA";
const HASH: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

const EXIT_LIMIT: Duration = Duration::from_secs(20);

/// Tests pick free ports by binding port 0 and releasing it. Two tests that
/// run at the same time can get the same port, so tests that bind take this
/// lock first.
static PORTS_LOCK: Mutex<()> = Mutex::new(());

fn lock_ports() -> MutexGuard<'static, ()> {
    PORTS_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Listener ports of one test config, in bind order.
#[derive(Clone, Copy)]
struct Ports {
    status: u16,
    management: u16,
    tcp_viewer: u16,
    tcp_server: u16,
}

impl Ports {
    fn free() -> Self {
        // Hold all four until every port is chosen so that they differ.
        let holders: Vec<TcpListener> = (0..4)
            .map(|_| TcpListener::bind("127.0.0.1:0").unwrap_or_else(|e| panic!("free port: {e}")))
            .collect();
        let port = |i: usize| {
            holders[i]
                .local_addr()
                .unwrap_or_else(|e| panic!("local addr: {e}"))
                .port()
        };
        Self {
            status: port(0),
            management: port(1),
            tcp_viewer: port(2),
            tcp_server: port(3),
        }
    }

    fn all(self) -> [u16; 4] {
        [
            self.status,
            self.management,
            self.tcp_viewer,
            self.tcp_server,
        ]
    }
}

struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Run {
    /// `(code, key)` of every `error[..]` line; an absent key is empty.
    fn errors(&self) -> Vec<(String, String)> {
        self.stderr
            .lines()
            .filter_map(|line| line.strip_prefix("error["))
            .map(|rest| {
                let (code, rest) = rest.split_once(']').unwrap_or((rest, ""));
                let key = rest
                    .strip_prefix(" key=")
                    .and_then(|rest| rest.split_once(':'))
                    .map_or("", |(key, _)| key);
                (code.to_owned(), key.to_owned())
            })
            .collect()
    }

    fn assert_no_secret(&self) {
        for text in [&self.stdout, &self.stderr] {
            assert!(
                !text.contains(HASH) && !text.contains(SALT),
                "secret in output: {text}"
            );
        }
    }
}

fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
    items
        .iter()
        .map(|(code, key)| ((*code).to_owned(), (*key).to_owned()))
        .collect()
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vnc-relay-serve-{}-{name}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir)
            .unwrap_or_else(|error| panic!("clear {}: {error}", dir.display()));
    }
    std::fs::create_dir_all(&dir)
        .unwrap_or_else(|error| panic!("create {}: {error}", dir.display()));
    dir
}

fn write(path: &Path, text: &str) {
    std::fs::write(path, text).unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .unwrap_or_else(|error| panic!("chmod {}: {error}", path.display()));
    }
}

fn config_text(ports: Ports) -> String {
    format!(
        r#"
[admin]
accounts_file = "admin-accounts.toml"

[management]
listen = "127.0.0.1:{}"

[status]
listen = "127.0.0.1:{}"

[relay.tcp_viewer]
enabled = true
listen = "127.0.0.1:{}"
allow = ["127.0.0.1"]

[relay.tcp_server]
enabled = true
listen = "127.0.0.1:{}"
allow = ["127.0.0.1"]
"#,
        ports.management, ports.status, ports.tcp_viewer, ports.tcp_server
    )
}

fn accounts_text(params: &str) -> String {
    format!(
        "version = 1\n\n[[account]]\nname = \"alice\"\npassword_hash = \"$argon2id$v=19${params}${SALT}${HASH}\"\n"
    )
}

/// Writes a valid config and account file into a new directory and returns
/// the config path.
fn valid_setup(name: &str, ports: Ports) -> PathBuf {
    let dir = temp_dir(name);
    write(
        &dir.join("admin-accounts.toml"),
        &accounts_text("m=19456,t=2,p=1"),
    );
    let config = dir.join("relay.toml");
    write(&config, &config_text(ports));
    config
}

fn spawn(config: &Path) -> Child {
    Command::new(BIN)
        .arg("serve")
        .arg("--config")
        .arg(config)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("spawn {BIN}: {error}"))
}

fn wait_exit(child: &mut Child) -> ExitStatus {
    let deadline = Instant::now() + EXIT_LIMIT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            Ok(None) => {
                let _ = child.kill();
                panic!("vnc-relay did not exit in {EXIT_LIMIT:?}");
            }
            Err(error) => panic!("wait: {error}"),
        }
    }
}

/// Waits for exit and collects the output. stdout and stderr stay small,
/// so the child never blocks on a full pipe.
fn finish(mut child: Child) -> Run {
    let status = wait_exit(&mut child);
    Run {
        code: status.code(),
        stdout: read_all(child.stdout.as_mut()),
        stderr: read_all(child.stderr.as_mut()),
    }
}

fn read_all(stream: Option<&mut impl Read>) -> String {
    let mut text = String::new();
    if let Some(stream) = stream {
        stream
            .read_to_string(&mut text)
            .unwrap_or_else(|error| panic!("read output: {error}"));
    }
    text
}

fn serve(config: &Path) -> Run {
    finish(spawn(config))
}

fn connects(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok()
}

/// Every port can be bound again: the process released its listeners.
fn assert_released(ports: Ports) {
    for port in ports.all() {
        if let Err(error) = TcpListener::bind(("127.0.0.1", port)) {
            panic!("port {port} not released: {error}");
        }
    }
}

/// Waits until every port accepts a TCP connection. Fails early with the
/// output when the child exits.
fn wait_listening(child: &mut Child, ports: Ports) {
    let deadline = Instant::now() + EXIT_LIMIT;
    while !ports.all().into_iter().all(connects) {
        if let Ok(Some(status)) = child.try_wait() {
            let mut stderr = String::new();
            if let Some(stream) = child.stderr.as_mut() {
                let _ = stream.read_to_string(&mut stderr);
            }
            panic!("vnc-relay exited early ({status}): {stderr}");
        }
        assert!(
            Instant::now() < deadline,
            "listeners not ready in {EXIT_LIMIT:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Kills the child if a test fails before it stops the child itself.
struct Guard(Option<Child>);

impl Guard {
    fn child(&mut self) -> &mut Child {
        self.0
            .as_mut()
            .unwrap_or_else(|| panic!("child already taken"))
    }

    /// Hands the child over for a signal test (Unix only).
    #[cfg(unix)]
    fn take(&mut self) -> Child {
        self.0
            .take()
            .unwrap_or_else(|| panic!("child already taken"))
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[cfg(unix)]
fn send_signal(child: &Child, name: &str) {
    let status = Command::new("kill")
        .arg(format!("-{name}"))
        .arg(child.id().to_string())
        .status()
        .unwrap_or_else(|error| panic!("kill -{name}: {error}"));
    assert!(status.success(), "kill -{name} failed: {status}");
}

/// spec: operations.md:OPS-001
#[test]
fn rejects_invalid_config_with_exit_code_3_and_every_cause() {
    let dir = temp_dir("bad-config");
    let config = dir.join("relay.toml");
    write(
        &config,
        r#"
[admin]

[management]
listen = "127.0.0.1:99999"

[status]
listen = "127.0.0.1:18081"

[limits]
admin_connections = 0

[timeouts]
handshake_secs = 121
"#,
    );
    let run = serve(&config);
    assert_eq!(run.code, Some(3), "{}", run.stderr);
    assert_eq!(run.stdout, "");
    let mut errors = run.errors();
    errors.sort();
    assert_eq!(
        errors,
        pairs(&[
            ("config_listen_invalid", "management.listen"),
            ("config_missing_key", "admin.accounts_file"),
            ("config_value_out_of_range", "limits.admin_connections"),
            ("config_value_out_of_range", "timeouts.handshake_secs"),
        ])
    );
}

/// spec: operations.md:OPS-001
#[test]
fn rejects_toml_syntax_error_and_missing_config_file_with_exit_code_3() {
    let dir = temp_dir("syntax");
    let config = dir.join("relay.toml");
    write(&config, "[admin\naccounts_file = \"a.toml\"\n");
    let run = serve(&config);
    assert_eq!(run.code, Some(3), "{}", run.stderr);
    assert_eq!(run.stdout, "");
    assert_eq!(run.errors(), pairs(&[("config_syntax_error", "")]));
    assert!(
        run.stderr.starts_with("error[config_syntax_error]: "),
        "{}",
        run.stderr
    );

    let run = serve(&dir.join("missing.toml"));
    assert_eq!(run.code, Some(3), "{}", run.stderr);
    assert_eq!(run.errors(), pairs(&[("config_file_unreadable", "")]));
}

/// spec: operations.md:OPS-001
#[test]
fn refuses_to_start_without_a_usable_account_with_exit_code_5() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = valid_setup("accounts", ports);
    let accounts = config.with_file_name("admin-accounts.toml");
    // (account file text or none, expected code, expected key)
    let cases = [
        (None, "accounts_file_missing", "admin.accounts_file"),
        (
            Some("version = 1\n".to_owned()),
            "accounts_none",
            "admin.accounts_file",
        ),
        (
            Some(accounts_text("m=19456,t=2,p=1").replace("argon2id", "argon2i")),
            "accounts_file_invalid",
            "account[0].password_hash",
        ),
        (
            Some(accounts_text("m=4096,t=2,p=1")),
            "accounts_hash_below_minimum",
            "account[0].password_hash",
        ),
    ];
    for (text, code, key) in cases {
        match &text {
            Some(text) => write(&accounts, text),
            None => {
                let _ = std::fs::remove_file(&accounts);
            }
        }
        let run = serve(&config);
        assert_eq!(run.code, Some(5), "{}", run.stderr);
        assert_eq!(run.stdout, "");
        assert_eq!(run.errors(), pairs(&[(code, key)]));
        run.assert_no_secret();
        // The account stage comes before any listener.
        assert_released(ports);
    }
}

/// spec: operations.md:OPS-002 partial
#[test]
fn fails_with_exit_code_4_and_releases_listeners_when_a_port_is_taken() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = valid_setup("port-taken", ports);
    let taken = TcpListener::bind(("127.0.0.1", ports.tcp_viewer))
        .unwrap_or_else(|error| panic!("take port: {error}"));
    let run = serve(&config);
    assert_eq!(run.code, Some(4), "{}", run.stderr);
    assert_eq!(run.stdout, "");
    assert_eq!(
        run.stderr,
        format!(
            "error[listener_bind_failed] key=relay.tcp_viewer.listen: 127.0.0.1:{}에 바인드할 수 없다. 이미 사용 중이다.\n",
            ports.tcp_viewer
        )
    );
    drop(taken);
    // Status and management were bound before the failure and are free again.
    assert_released(ports);
}

/// spec: operations.md:OPS-039 partial
#[test]
fn second_instance_fails_with_exit_code_4_and_leaves_the_first_running() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = valid_setup("second", ports);
    let mut first = Guard(Some(spawn(&config)));
    wait_listening(first.child(), ports);

    let second = serve(&config);
    assert_eq!(second.code, Some(4), "{}", second.stderr);
    assert_eq!(second.stdout, "");
    assert_eq!(
        second.errors(),
        pairs(&[
            ("listener_bind_failed", "status.listen"),
            ("listener_bind_failed", "management.listen"),
            ("listener_bind_failed", "relay.tcp_viewer.listen"),
            ("listener_bind_failed", "relay.tcp_server.listen"),
        ])
    );

    assert!(
        matches!(first.child().try_wait(), Ok(None)),
        "first instance stopped"
    );
    assert!(ports.all().into_iter().all(connects));

    #[cfg(unix)]
    {
        let child = first.take();
        send_signal(&child, "TERM");
        let run = finish(child);
        assert_eq!(run.code, Some(0), "{}", run.stderr);
        assert_eq!(run.stdout, "");
        assert_released(ports);
    }
}

#[cfg(unix)]
#[test]
fn stops_with_exit_code_0_on_interrupt_and_terminate() {
    let _lock = lock_ports();
    for signal in ["INT", "TERM"] {
        let ports = Ports::free();
        let config = valid_setup(&format!("signal-{signal}"), ports);
        let mut guard = Guard(Some(spawn(&config)));
        wait_listening(guard.child(), ports);
        let child = guard.take();
        send_signal(&child, signal);
        let run = finish(child);
        assert_eq!(run.code, Some(0), "SIG{signal}: {}", run.stderr);
        assert_eq!(run.stdout, "");
        assert_eq!(run.stderr, "");
        assert_released(ports);
    }
}

#[cfg(unix)]
#[test]
fn warns_about_a_permissive_account_file_and_still_starts() {
    use std::os::unix::fs::PermissionsExt;

    let _lock = lock_ports();
    let ports = Ports::free();
    let config = valid_setup("permissive", ports);
    std::fs::set_permissions(
        config.with_file_name("admin-accounts.toml"),
        std::fs::Permissions::from_mode(0o644),
    )
    .unwrap_or_else(|error| panic!("chmod: {error}"));
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);
    let child = guard.take();
    send_signal(&child, "TERM");
    let run = finish(child);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(
        run.stderr
            .starts_with("warning[accounts_file_permissive] key=admin.accounts_file: "),
        "{}",
        run.stderr
    );
    assert_eq!(run.stderr.lines().count(), 1, "{}", run.stderr);
}
