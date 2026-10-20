//! System tests of `vnc-relay serve` start and stop: the start order, exit
//! codes, diagnostic lines, and operation log lines of
//! docs/contracts/cli-v1.md §1.5, §1.8, §2 and docs/contracts/ops-info-v1.md
//! §1.1, §1.5, §1.6.

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

/// The string value of `"name":"value"` in one log line. Values in these
/// tests hold no quote, so a plain search is enough. The ops-info contract
/// tests parse lines as JSON.
fn field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let start = line.find(&format!("\"{name}\":\""))? + name.len() + 4;
    let rest = &line[start..];
    rest.find('"').map(|end| &rest[..end])
}

/// The `event` of each log line, in order.
fn events(log: &str) -> Vec<&str> {
    log.lines()
        .map(|line| field(line, "event").unwrap_or_else(|| panic!("no event: {line}")))
        .collect()
}

/// `(reason, config_key)` of each `startup_failed` line; an absent key is
/// empty.
fn startup_failures(log: &str) -> Vec<(String, String)> {
    log.lines()
        .filter(|line| field(line, "event") == Some("startup_failed"))
        .map(|line| {
            let reason = field(line, "reason").unwrap_or_else(|| panic!("no reason: {line}"));
            let key = field(line, "config_key").unwrap_or("");
            (reason.to_owned(), key.to_owned())
        })
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
/// spec: operations.md:OPS-012 partial
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

[relay.tcp_viewer]
enabled = true
listen = "127.0.0.1:18082"
allow = ["198.51.100.300"]

[limits]
admin_connections = 0

[timeouts]
handshake_secs = 121
"#,
    );
    let run = serve(&config);
    assert_eq!(run.code, Some(3), "{}", run.stderr);
    let mut errors = run.errors();
    errors.sort();
    assert_eq!(
        errors,
        pairs(&[
            ("config_ip_item_invalid", "relay.tcp_viewer.allow[0]"),
            ("config_listen_invalid", "management.listen"),
            ("config_missing_key", "admin.accounts_file"),
            ("config_value_out_of_range", "limits.admin_connections"),
            ("config_value_out_of_range", "timeouts.handshake_secs"),
        ])
    );
    // The log output is not known yet, so stdout gets one line per
    // diagnostic. The lines name keys, never the rejected values.
    assert!(
        events(&run.stdout)
            .iter()
            .all(|event| *event == "startup_failed"),
        "{}",
        run.stdout
    );
    let mut logged = startup_failures(&run.stdout);
    logged.sort();
    assert_eq!(logged, errors);
    for value in ["198.51.100.300", "99999", "18082"] {
        assert!(!run.stdout.contains(value), "{value} in {}", run.stdout);
    }
}

/// spec: operations.md:OPS-001
#[test]
fn rejects_toml_syntax_error_and_missing_config_file_with_exit_code_3() {
    let dir = temp_dir("syntax");
    let config = dir.join("relay.toml");
    write(&config, "[admin\naccounts_file = \"a.toml\"\n");
    let run = serve(&config);
    assert_eq!(run.code, Some(3), "{}", run.stderr);
    assert_eq!(events(&run.stdout), ["startup_failed"]);
    assert_eq!(
        startup_failures(&run.stdout),
        pairs(&[("config_syntax_error", "")])
    );
    assert_eq!(run.errors(), pairs(&[("config_syntax_error", "")]));
    assert!(
        run.stderr.starts_with("error[config_syntax_error]: "),
        "{}",
        run.stderr
    );

    let run = serve(&dir.join("missing.toml"));
    assert_eq!(run.code, Some(3), "{}", run.stderr);
    assert_eq!(run.errors(), pairs(&[("config_file_unreadable", "")]));
    assert_eq!(
        startup_failures(&run.stdout),
        pairs(&[("config_file_unreadable", "")])
    );
}

/// spec: operations.md:OPS-001
/// spec: operations.md:OPS-012 partial
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
        // The log names the account file key; keys inside the file stay on
        // stderr.
        assert_eq!(events(&run.stdout), ["server_starting", "startup_failed"]);
        assert_eq!(
            startup_failures(&run.stdout),
            pairs(&[(code, "admin.accounts_file")])
        );
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
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(events(&run.stdout), ["server_starting", "startup_failed"]);
    assert_eq!(field(lines[1], "reason"), Some("listener_bind_failed"));
    assert_eq!(field(lines[1], "listener"), Some("tcp_viewer"));
    assert_eq!(
        field(lines[1], "config_key"),
        Some("relay.tcp_viewer.listen")
    );
    assert_eq!(field(lines[1], "detail"), Some("이미 사용 중이다."));
    assert!(!run.stdout.contains("127.0.0.1"), "{}", run.stdout);
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
    assert_eq!(
        events(&second.stdout),
        [
            "server_starting",
            "startup_failed",
            "startup_failed",
            "startup_failed",
            "startup_failed"
        ]
    );
    let listeners: Vec<&str> = second
        .stdout
        .lines()
        .filter_map(|line| field(line, "listener"))
        .collect();
    assert_eq!(
        listeners,
        ["status", "management", "tcp_viewer", "tcp_server"]
    );
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
        assert_eq!(events(&run.stdout), ["server_starting", "server_stopped"]);
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
        assert_eq!(events(&run.stdout), ["server_starting", "server_stopped"]);
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

#[cfg(unix)]
#[test]
fn writes_the_log_to_the_configured_file_from_start_to_stop() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = valid_setup("log-file", ports);
    let text = config_text(ports).replacen("allow = [\"127.0.0.1\"]", "allow = [\"0.0.0.0/0\"]", 1)
        + "\n[log]\noutput = \"relay.log\"\n";
    write(&config, &text);
    let log_path = config.with_file_name("relay.log");
    std::fs::write(&log_path, "earlier line\n")
        .unwrap_or_else(|error| panic!("write log: {error}"));

    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);
    let child = guard.take();
    send_signal(&child, "TERM");
    let run = finish(child);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(run.stdout, "");
    assert_eq!(run.stderr, "");

    let log =
        std::fs::read_to_string(&log_path).unwrap_or_else(|error| panic!("read log: {error}"));
    let log = log
        .strip_prefix("earlier line\n")
        .unwrap_or_else(|| panic!("log was not appended: {log}"));
    assert_eq!(
        events(log),
        ["server_starting", "allow_all_source", "server_stopped"]
    );
    let lines: Vec<&str> = log.lines().collect();
    assert_eq!(field(lines[0], "level"), Some("info"));
    assert_eq!(field(lines[0], "log_contract"), Some("ops-info/1.2.0"));
    assert_eq!(field(lines[1], "level"), Some("warn"));
    assert_eq!(field(lines[1], "listener"), Some("tcp_viewer"));
    assert_eq!(field(lines[1], "item"), Some("0.0.0.0/0"));
    assert_eq!(field(lines[1], "phase"), Some("startup"));
}

#[test]
fn fails_with_exit_code_3_when_the_log_file_cannot_be_opened() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = valid_setup("log-unavailable", ports);
    let text = config_text(ports) + "\n[log]\noutput = \"no-such-dir/relay.log\"\n";
    write(&config, &text);
    let run = serve(&config);
    assert_eq!(run.code, Some(3), "{}", run.stderr);
    assert_eq!(
        run.errors(),
        pairs(&[("log_file_unavailable", "log.output")])
    );
    // The log file is not open, so the line goes to stdout without the path.
    assert_eq!(events(&run.stdout), ["startup_failed"]);
    assert_eq!(
        startup_failures(&run.stdout),
        pairs(&[("log_file_unavailable", "log.output")])
    );
    assert_eq!(field(&run.stdout, "detail"), Some("상위 디렉터리가 없다."));
    assert!(!run.stdout.contains("no-such-dir"), "{}", run.stdout);
    assert_released(ports);
}

#[cfg(target_os = "linux")]
#[test]
fn warns_once_when_log_writes_fail_and_keeps_running() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = valid_setup("log-full", ports);
    write(
        &config,
        &(config_text(ports) + "\n[log]\noutput = \"/dev/full\"\n"),
    );
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);
    let child = guard.take();
    send_signal(&child, "TERM");
    let run = finish(child);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(run.stdout, "");
    assert!(
        run.stderr
            .starts_with("warning[log_write_failed] key=log.output: "),
        "{}",
        run.stderr
    );
    // server_starting and server_stopped both fail; only the first is told.
    assert_eq!(run.stderr.lines().count(), 1, "{}", run.stderr);
}
