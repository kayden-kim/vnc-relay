//! System tests of `vnc-relay service` on Windows 11: install, uninstall,
//! and the server under the SCM (docs/contracts/cli-v1.md §1.9,
//! docs/specs/operations.md §3.7).
//!
//! Only `service_run_from_a_terminal_exits_2_without_opening_anything` runs by
//! default. The other tests register the real service `vnc-relay` on this PC,
//! so they are ignored and need an elevated terminal
//! (docs/how-to/set-up-and-verify.md "Windows 서비스 시험"). Each one refuses
//! to start when a service `vnc-relay` already exists, and deletes the service
//! it made at the end, also when it fails.
#![cfg(windows)]

use std::collections::BTreeSet;
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

const BIN: &str = env!("CARGO_BIN_EXE_vnc-relay");
const LIMIT: Duration = Duration::from_secs(30);
const SERVICE: &str = "vnc-relay";

/// 16 and 32 zero bytes in unpadded Base64. Test data, not a real hash.
const ACCOUNTS: &str = "version = 1\n\n[[account]]\nname = \"alice\"\npassword_hash = \"$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\"\n";

/// Win32 error numbers that `sc.exe` returns as its exit code.
const ERROR_SERVICE_CANNOT_ACCEPT_CTRL: i32 = 1061;
const ERROR_SERVICE_SPECIFIC_ERROR: u32 = 1066;

/// The tests share one service name, so they never overlap.
static SERVICE_LOCK: Mutex<()> = Mutex::new(());

/// spec: operations.md:OPS-081 partial
#[test]
fn service_run_from_a_terminal_exits_2_without_opening_anything() {
    let dir = std::env::temp_dir().join(format!("vnc-relay-service-run-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap_or_else(|error| panic!("create {}: {error}", dir.display()));
    fs::write(
        dir.join("relay.toml"),
        config_text(free_ports(), 30, "relay.log"),
    )
    .unwrap_or_else(|error| panic!("write config: {error}"));
    fs::write(dir.join("admin-accounts.toml"), ACCOUNTS)
        .unwrap_or_else(|error| panic!("write accounts: {error}"));

    let output = run_limited(
        Command::new(BIN)
            .args(["service", "run", "--config"])
            .arg(dir.join("relay.toml")),
    );
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(text(&output.stdout), "");
    let stderr = text(&output.stderr);
    assert!(
        stderr.starts_with("error[service_controller_unavailable]: ")
            && stderr.lines().count() == 1,
        "{stderr}"
    );
    // The log file opens before any listener, so no log file means no listener.
    assert!(!dir.join("relay.log").exists());
    let _ = fs::remove_dir_all(&dir);
}

/// spec: operations.md:OPS-074
/// spec: operations.md:OPS-080 partial
#[test]
#[ignore = "registers the real service in an elevated terminal; docs/how-to/set-up-and-verify.md"]
fn install_registers_a_stopped_service_and_uninstall_keeps_the_files() {
    let fixture = Fixture::new("install");
    let (user, user_sid) = current_user();
    let users = users_group();

    // Windows's own access entries, from a registration with no operator.
    fixture.install(&[]);
    let default_sddl = sc_ok(&["sdshow", SERVICE]);
    record(&format!("default service SDDL: {}", default_sddl.trim()));
    let defaults = aces(&default_sddl);
    fixture.uninstall();
    fixture.wait_gone();

    let output = fixture.vnc_relay(&[
        "service",
        "install",
        "--config",
        "relay.toml",
        "--operator",
        &user,
        "--operator",
        &users,
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), "서비스 'vnc-relay'를 설치했다.\n");
    assert_eq!(text(&output.stderr), "");

    let config = sc_ok(&["qc", SERVICE]);
    assert!(config.contains("WIN32_OWN_PROCESS"), "{config}");
    let start_type = field(&config, "START_TYPE").unwrap_or_default();
    assert!(
        start_type.contains("AUTO_START") && start_type.contains("(DELAYED)"),
        "{config}"
    );
    assert_eq!(
        field(&config, "DISPLAY_NAME"),
        Some("VNC Relay"),
        "{config}"
    );
    assert_eq!(
        field(&config, "SERVICE_START_NAME"),
        Some(r"NT SERVICE\vnc-relay"),
        "{config}"
    );
    // The relative `--config` became absolute. These paths have no space, so
    // the command has no quotes.
    assert_eq!(
        field(&config, "BINARY_PATH_NAME"),
        Some(
            format!(
                "{} service run --config {}",
                fixture.exe.display(),
                fixture.dir.join("relay.toml").display()
            )
            .as_str()
        ),
        "{config}"
    );
    let recovery = sc_ok(&["qfailure", SERVICE]);
    assert_eq!(
        field(&recovery, "FAILURE_ACTIONS").unwrap_or_default(),
        "",
        "{recovery}"
    );

    let sddl = sc_ok(&["sdshow", SERVICE]);
    let added: BTreeSet<_> = aces(&sddl).difference(&defaults).cloned().collect();
    let removed: BTreeSet<_> = defaults.difference(&aces(&sddl)).cloned().collect();
    assert_eq!(
        added,
        BTreeSet::from([
            format!("(A;;CCLCRPWP;;;{user_sid})"),
            "(A;;CCLCRPWP;;;BU)".to_owned(),
        ]),
        "{sddl}"
    );
    assert!(removed.is_empty(), "{sddl}");
    assert_eq!(status().map(|status| status.state), Some(State::Stopped));

    let output = fixture.vnc_relay(&["service", "uninstall"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), "서비스 'vnc-relay'를 제거했다.\n");
    fixture.wait_gone();
    assert!(fixture.dir.join("relay.toml").exists());
    assert!(fixture.dir.join("admin-accounts.toml").exists());

    let output = fixture.vnc_relay(&["service", "uninstall"]);
    assert_eq!(output.status.code(), Some(1));
    assert_error(&output, "service_not_found");
}

/// spec: operations.md:OPS-075 partial
///
/// Access denied in a terminal that is not elevated needs a real UAC session,
/// so it is a manual check (docs/how-to/set-up-and-verify.md).
#[test]
#[ignore = "registers the real service in an elevated terminal; docs/how-to/set-up-and-verify.md"]
fn install_refuses_bad_input_and_leaves_no_registration() {
    let fixture = Fixture::new("refuse");
    let config = fixture.dir.join("relay.toml");

    let unknown = format!("vnc-relay-no-such-user-{}", std::process::id());
    let output = fixture.vnc_relay(&[
        "service",
        "install",
        "--config",
        "relay.toml",
        "--operator",
        &unknown,
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert_error(&output, "service_operator_unknown");
    assert!(text(&output.stderr).contains(&format!("`{unknown}`")));
    assert!(status().is_none());

    let valid = fs::read_to_string(&config).unwrap_or_else(|error| panic!("read config: {error}"));
    for (text_now, code, exit) in [
        (
            valid.replace("output = \"relay.log\"", "output = \"stdout\""),
            "service_log_stdout",
            3,
        ),
        ("[admin\n".to_owned(), "config_", 3),
    ] {
        fs::write(&config, text_now).unwrap_or_else(|error| panic!("write config: {error}"));
        let output = fixture.vnc_relay(&["service", "install", "--config", "relay.toml"]);
        assert_eq!(output.status.code(), Some(exit), "{code}");
        assert_error(&output, code);
        assert!(status().is_none(), "{code}");
    }
    fs::write(&config, &valid).unwrap_or_else(|error| panic!("write config: {error}"));
    let accounts = fixture.dir.join("admin-accounts.toml");
    fs::remove_file(&accounts).unwrap_or_else(|error| panic!("remove accounts: {error}"));
    let output = fixture.vnc_relay(&["service", "install", "--config", "relay.toml"]);
    assert_eq!(output.status.code(), Some(5));
    assert_error(&output, "accounts_file_missing");
    assert!(status().is_none());
    fs::write(&accounts, ACCOUNTS).unwrap_or_else(|error| panic!("write accounts: {error}"));

    // An existing service stays as it was.
    fixture.install(&[]);
    let before = sc_ok(&["qc", SERVICE]);
    fs::write(fixture.dir.join("other.toml"), &valid)
        .unwrap_or_else(|error| panic!("write config: {error}"));
    let output = fixture.vnc_relay(&["service", "install", "--config", "other.toml"]);
    assert_eq!(output.status.code(), Some(1));
    assert_error(&output, "service_exists");
    assert_eq!(sc_ok(&["qc", SERVICE]), before);
    assert_eq!(status().map(|status| status.state), Some(State::Stopped));
}

/// spec: operations.md:OPS-076
/// spec: operations.md:OPS-080 partial
#[test]
#[ignore = "registers the real service in an elevated terminal; docs/how-to/set-up-and-verify.md"]
fn the_scm_runs_the_server_and_a_stop_request_applies_the_grace() {
    let fixture = Fixture::new("run");
    fixture.install(&[]);
    fixture.grant_service_account();

    sc_ok(&["start", SERVICE]);
    let states = fixture.wait_running();
    record(&format!("states before running: {states:?}"));

    let mut pair = Pair::start(fixture.ports, "4321");
    let output = fixture.vnc_relay(&["service", "uninstall"]);
    assert_eq!(output.status.code(), Some(1));
    assert_error(&output, "service_running");
    assert_eq!(status().map(|status| status.state), Some(State::Running));

    let stopped_at = Instant::now();
    sc_ok(&["stop", SERVICE]);
    let started = fixture.wait_event("shutdown_started");
    assert_eq!(
        string_field(&started, "trigger"),
        Some("requested"),
        "{started}"
    );
    assert_eq!(number_field(&started, "grace_secs"), Some(3), "{started}");
    assert_eq!(
        number_field(&started, "active_sessions"),
        Some(1),
        "{started}"
    );
    // The session keeps relaying in the grace while the SCM shows stop pending.
    while stopped_at.elapsed() < Duration::from_secs(1) {
        pair.transfer();
        assert_eq!(
            status().map(|status| status.state),
            Some(State::StopPending)
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    // OPS-TBD-013: the SCM refuses a second stop while the service stops.
    let second = sc(&["stop", SERVICE]);
    record(&format!("second stop while stopping: exit {:?}", second.0));
    assert_eq!(
        second.0,
        Some(ERROR_SERVICE_CANNOT_ACCEPT_CTRL),
        "{}",
        second.1
    );

    let status = fixture.wait_stopped();
    assert!(stopped_at.elapsed() >= Duration::from_secs(3));
    assert_eq!((status.win32_exit, status.service_exit), (0, 0));
    pair.assert_closed();
    let log = fixture.log();
    let events = events(&log);
    let forced = events
        .iter()
        .position(|event| *event == "shutdown_forced")
        .unwrap_or_else(|| panic!("{log}"));
    assert!(
        events.iter().position(|event| *event == "shutdown_started") < Some(forced),
        "{log}"
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| **event == "shutdown_started")
            .count(),
        1
    );
    assert_eq!(
        events.last().map(String::as_str),
        Some("server_stopped"),
        "{log}"
    );
    assert!(
        log.lines()
            .any(|line| string_field(line, "event") == Some("session_closed")
                && string_field(line, "close_cause") == Some("server_shutdown")),
        "{log}"
    );
    assert_ports_released(fixture.ports);

    fixture.uninstall();
    fixture.wait_gone();
    for name in ["relay.toml", "admin-accounts.toml", "relay.log"] {
        assert!(fixture.dir.join(name).exists(), "{name}");
    }
}

/// spec: operations.md:OPS-077
#[test]
#[ignore = "registers the real service in an elevated terminal; docs/how-to/set-up-and-verify.md"]
fn a_start_failure_stops_the_service_with_the_serve_exit_code() {
    let fixture = Fixture::new("fail");
    fixture.install(&[]);
    fixture.grant_service_account();
    let config = fixture.dir.join("relay.toml");
    let valid = fs::read_to_string(&config).unwrap_or_else(|error| panic!("read config: {error}"));

    // Failures before the log file opens leave only the exit code.
    for (broken, exit) in [
        ("[admin\n".to_owned(), 3),
        (
            valid.replace("output = \"relay.log\"", "output = \"stdout\""),
            3,
        ),
    ] {
        fs::write(&config, broken).unwrap_or_else(|error| panic!("write config: {error}"));
        let status = fixture.start_and_fail();
        assert_eq!(
            (status.win32_exit, status.service_exit),
            (ERROR_SERVICE_SPECIFIC_ERROR, exit)
        );
        assert!(!fixture.dir.join("relay.log").exists());
    }
    fs::write(&config, &valid).unwrap_or_else(|error| panic!("write config: {error}"));

    let taken = TcpListener::bind(("127.0.0.1", fixture.ports[0]))
        .unwrap_or_else(|error| panic!("take the management port: {error}"));
    let status = fixture.start_and_fail();
    assert_eq!(
        (status.win32_exit, status.service_exit),
        (ERROR_SERVICE_SPECIFIC_ERROR, 4)
    );
    fixture.assert_startup_failed("listener_bind_failed");
    drop(taken);

    let accounts = fixture.dir.join("admin-accounts.toml");
    icacls(&accounts, &["/deny", r"NT SERVICE\vnc-relay:(R)"]);
    let status = fixture.start_and_fail();
    icacls(&accounts, &["/remove:d", r"NT SERVICE\vnc-relay"]);
    assert_eq!(
        (status.win32_exit, status.service_exit),
        (ERROR_SERVICE_SPECIFIC_ERROR, 5)
    );
    fixture.assert_startup_failed("accounts_file_unreadable");
    assert_ports_released(fixture.ports);
}

/// A directory with the service files and a copy of `vnc-relay.exe` under
/// `%ProgramData%`. The service runs that copy. Dropping the fixture deletes
/// the service and the directory.
struct Fixture {
    _lock: MutexGuard<'static, ()>,
    dir: PathBuf,
    exe: PathBuf,
    /// Management, status, TCP Viewer, TCP Server.
    ports: [u16; 4],
}

impl Fixture {
    fn new(name: &str) -> Self {
        let lock = SERVICE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        assert!(
            elevated(),
            "run the ignored service tests in an elevated terminal"
        );
        assert!(
            status().is_none(),
            "a service named {SERVICE} already exists; these tests never touch it"
        );
        let root = std::env::var_os("ProgramData")
            .map(PathBuf::from)
            .unwrap_or_else(|| panic!("ProgramData is not set"));
        let parent = root.join("vnc-relay-tests");
        let dir = parent.join(format!("{}-{name}", std::process::id()));
        fs::create_dir_all(&parent)
            .unwrap_or_else(|error| panic!("create {}: {error}", parent.display()));
        // A directory that exists is not this run's, so it is never cleared.
        fs::create_dir(&dir).unwrap_or_else(|error| panic!("create {}: {error}", dir.display()));
        let exe = dir.join("vnc-relay.exe");
        fs::copy(BIN, &exe).unwrap_or_else(|error| panic!("copy {BIN}: {error}"));
        let ports = free_ports();
        fs::write(dir.join("relay.toml"), config_text(ports, 3, "relay.log"))
            .unwrap_or_else(|error| panic!("write config: {error}"));
        fs::write(dir.join("admin-accounts.toml"), ACCOUNTS)
            .unwrap_or_else(|error| panic!("write accounts: {error}"));
        Self {
            _lock: lock,
            dir,
            exe,
            ports,
        }
    }

    /// Runs the copied `vnc-relay.exe` in the fixture directory.
    fn vnc_relay(&self, args: &[&str]) -> Output {
        run_limited(Command::new(&self.exe).args(args).current_dir(&self.dir))
    }

    fn install(&self, operators: &[&str]) {
        let mut args = vec!["service", "install", "--config", "relay.toml"];
        for operator in operators {
            args.extend(["--operator", operator]);
        }
        let output = self.vnc_relay(&args);
        assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    }

    fn uninstall(&self) {
        let output = self.vnc_relay(&["service", "uninstall"]);
        assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    }

    fn wait_gone(&self) {
        wait_for("service deleted", || status().is_none());
    }

    /// Lets the service account read the files and write the log. An
    /// operator does the same for a real install.
    fn grant_service_account(&self) {
        icacls(&self.dir, &["/grant", r"NT SERVICE\vnc-relay:(OI)(CI)M"]);
    }

    /// Waits for running and returns the states seen before it. The SCM shows
    /// running only after the server is ready, so `/readyz` is 200 by then.
    fn wait_running(&self) -> Vec<State> {
        let mut seen = Vec::new();
        let deadline = Instant::now() + LIMIT;
        loop {
            let state = status().map(|status| status.state);
            if state == Some(State::Running) {
                assert_eq!(readyz(self.ports[1]), Some(200));
                return seen;
            }
            assert_eq!(state, Some(State::StartPending), "{seen:?} {}", self.log());
            if seen.last() != Some(&State::StartPending) {
                seen.push(State::StartPending);
            }
            assert!(
                Instant::now() < deadline,
                "service not running: {}",
                self.log()
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn wait_stopped(&self) -> Status {
        let deadline = Instant::now() + LIMIT;
        loop {
            if let Some(status) = status().filter(|status| status.state == State::Stopped) {
                return status;
            }
            assert!(
                Instant::now() < deadline,
                "service not stopped: {}",
                self.log()
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Starts the service and waits for it to stop without ever running.
    fn start_and_fail(&self) -> Status {
        // `sc.exe start` may report the start or the early stop; the state
        // that follows is what counts.
        let _ = sc(&["start", SERVICE]);
        let deadline = Instant::now() + LIMIT;
        loop {
            let status = status().unwrap_or_else(|| panic!("service vanished"));
            assert_ne!(status.state, State::Running, "{}", self.log());
            if status.state == State::Stopped {
                return status;
            }
            assert!(
                Instant::now() < deadline,
                "service did not stop: {}",
                self.log()
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn log(&self) -> String {
        fs::read_to_string(self.dir.join("relay.log")).unwrap_or_default()
    }

    fn wait_event(&self, event: &str) -> String {
        let mut found = None;
        wait_for(event, || {
            found = self
                .log()
                .lines()
                .find(|line| string_field(line, "event") == Some(event))
                .map(str::to_owned);
            found.is_some()
        });
        found.unwrap_or_default()
    }

    fn assert_startup_failed(&self, reason: &str) {
        let log = self.log();
        assert!(
            log.lines()
                .any(|line| string_field(line, "event") == Some("startup_failed")
                    && string_field(line, "reason") == Some(reason)),
            "{reason}: {log}"
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Only a service whose executable is this fixture's copy is stopped
        // and deleted. Errors are ignored: a panic here would abort the run.
        let exe = self.exe.display().to_string().to_ascii_lowercase();
        let (_, config) = sc(&["qc", SERVICE]);
        if field(&config, "BINARY_PATH_NAME").is_some_and(|command| {
            let command = command.to_ascii_lowercase();
            let program = match command.strip_prefix('"') {
                Some(quoted) => quoted.split('"').next(),
                None => command.split(' ').next(),
            };
            program == Some(exe.as_str())
        }) {
            let _ = sc(&["stop", SERVICE]);
            let deadline = Instant::now() + LIMIT;
            while status().is_some_and(|status| status.state != State::Stopped)
                && Instant::now() < deadline
            {
                std::thread::sleep(Duration::from_millis(50));
            }
            let _ = sc(&["delete", SERVICE]);
        }
        // The service process may still be exiting and holding its copy.
        let deadline = Instant::now() + LIMIT;
        while fs::remove_dir_all(&self.dir).is_err() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Stopped,
    StartPending,
    StopPending,
    Running,
    Other,
}

#[derive(Debug, Clone, Copy)]
struct Status {
    state: State,
    win32_exit: u32,
    service_exit: u32,
}

/// The status from `sc.exe query`, or `None` when no service exists.
/// Some labels are translated, so the state comes from its English name.
fn status() -> Option<Status> {
    let (code, output) = sc(&["query", SERVICE]);
    if code == Some(1060) {
        return None;
    }
    assert_eq!(code, Some(0), "{output}");
    let state = output
        .split_whitespace()
        .find_map(|word| match word {
            "STOPPED" => Some(State::Stopped),
            "START_PENDING" => Some(State::StartPending),
            "STOP_PENDING" => Some(State::StopPending),
            "RUNNING" => Some(State::Running),
            "CONTINUE_PENDING" | "PAUSE_PENDING" | "PAUSED" => Some(State::Other),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no state: {output}"));
    let exit = |label| {
        field(&output, label)
            .and_then(|value| value.split_whitespace().next())
            .and_then(|value| value.parse().ok())
            .unwrap_or_else(|| panic!("no {label}: {output}"))
    };
    Some(Status {
        state,
        win32_exit: exit("WIN32_EXIT_CODE"),
        service_exit: exit("SERVICE_EXIT_CODE"),
    })
}

/// Runs `sc.exe` and returns its exit code, a Win32 error number, and stdout.
fn sc(args: &[&str]) -> (Option<i32>, String) {
    let output = run_limited(Command::new("sc.exe").args(args));
    (output.status.code(), text(&output.stdout))
}

fn sc_ok(args: &[&str]) -> String {
    let (code, output) = sc(args);
    assert_eq!(code, Some(0), "sc.exe {args:?}: {output}");
    output
}

fn icacls(path: &Path, args: &[&str]) {
    let output = run_limited(Command::new("icacls.exe").arg(path).args(args));
    assert!(
        output.status.success(),
        "icacls {args:?}: {}",
        text(&output.stdout)
    );
}

/// The value after `label :` on the line that starts with `label`.
fn field<'a>(output: &'a str, label: &str) -> Option<&'a str> {
    output.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        (name.trim() == label).then(|| value.trim())
    })
}

/// The access entries of an SDDL string, such as `(A;;CCLCRPWP;;;BU)`.
fn aces(sddl: &str) -> BTreeSet<String> {
    sddl.split_inclusive(')')
        .filter_map(|part| part.find('(').map(|start| part[start..].to_owned()))
        .collect()
}

/// `whoami /user`: the account name and its SID.
fn current_user() -> (String, String) {
    let output = run_limited(Command::new("whoami.exe").args(["/user", "/fo", "csv", "/nh"]));
    let line = text(&output.stdout);
    let mut fields = line.trim().split(',').map(|field| field.trim_matches('"'));
    match (fields.next(), fields.next()) {
        (Some(name), Some(sid)) => (name.to_owned(), sid.to_owned()),
        _ => panic!("whoami /user: {line}"),
    }
}

/// The name of the built-in Users group (S-1-5-32-545) on this PC.
fn users_group() -> String {
    let output = run_limited(Command::new("whoami.exe").args(["/groups", "/fo", "csv", "/nh"]));
    text(&output.stdout)
        .lines()
        .find(|line| line.contains("\"S-1-5-32-545\""))
        .and_then(|line| line.split(',').next())
        .map(|name| name.trim_matches('"').to_owned())
        .unwrap_or_else(|| panic!("no Users group in whoami /groups"))
}

/// True when this process has the high integrity level of an elevated token.
fn elevated() -> bool {
    let output = run_limited(Command::new("whoami.exe").args(["/groups", "/fo", "csv", "/nh"]));
    text(&output.stdout).contains("S-1-16-12288")
}

/// Writes one line for the verification record. Run with `--nocapture`.
fn record(line: &str) {
    let _ = writeln!(std::io::stderr(), "record: {line}");
}

fn config_text(ports: [u16; 4], grace: u64, log: &str) -> String {
    format!(
        r#"[admin]
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

[mode2]
enabled = true

[timeouts]
shutdown_grace_secs = {grace}

[log]
output = "{log}"
"#,
        ports[0], ports[1], ports[2], ports[3]
    )
}

/// Four loopback ports that were free a moment ago.
fn free_ports() -> [u16; 4] {
    let holders: Vec<_> = (0..4)
        .map(|_| {
            TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| panic!("reserve port: {error}"))
        })
        .collect();
    std::array::from_fn(|index| {
        holders[index]
            .local_addr()
            .unwrap_or_else(|error| panic!("reserved address: {error}"))
            .port()
    })
}

fn assert_ports_released(ports: [u16; 4]) {
    for port in ports {
        let rebound = TcpListener::bind(("127.0.0.1", port));
        assert!(rebound.is_ok(), "port {port} was not released: {rebound:?}");
    }
}

/// The HTTP status of `GET /readyz`, or `None` when nothing answers.
fn readyz(port: u16) -> Option<u16> {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(1)).ok()?;
    stream.set_read_timeout(Some(LIMIT)).ok()?;
    stream
        .write_all(b"GET /readyz HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .ok()?;
    let mut reply = String::new();
    stream.read_to_string(&mut reply).ok()?;
    reply.split_whitespace().nth(1)?.parse().ok()
}

/// One TCP Mode II relay pair.
struct Pair {
    viewer: TcpStream,
    server: TcpStream,
}

impl Pair {
    fn start(ports: [u16; 4], id: &str) -> Self {
        let mut server = connect(ports[3]);
        server
            .write_all(&frame(&format!("ID:{id}")))
            .unwrap_or_else(|error| panic!("server request: {error}"));
        let mut viewer = connect(ports[2]);
        let mut greeting = [0; 12];
        viewer
            .read_exact(&mut greeting)
            .unwrap_or_else(|error| panic!("viewer greeting: {error}"));
        assert_eq!(&greeting, b"RFB 000.000\n");
        viewer
            .write_all(&frame(&format!("ID:{id}")))
            .unwrap_or_else(|error| panic!("viewer request: {error}"));
        let mut pair = Self { viewer, server };
        pair.transfer();
        pair
    }

    fn transfer(&mut self) {
        let mut received = [0; 11];
        self.viewer
            .write_all(b"viewer-data")
            .unwrap_or_else(|error| panic!("viewer write: {error}"));
        self.server
            .read_exact(&mut received)
            .unwrap_or_else(|error| panic!("server read: {error}"));
        assert_eq!(&received, b"viewer-data");
        self.server
            .write_all(b"server-data")
            .unwrap_or_else(|error| panic!("server write: {error}"));
        self.viewer
            .read_exact(&mut received)
            .unwrap_or_else(|error| panic!("viewer read: {error}"));
        assert_eq!(&received, b"server-data");
    }

    fn assert_closed(&mut self) {
        for stream in [&mut self.viewer, &mut self.server] {
            let mut byte = [0];
            match stream.read(&mut byte) {
                Ok(0) => {}
                Err(error)
                    if matches!(
                        error.kind(),
                        ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted
                    ) => {}
                other => panic!("relay connection was not closed: {other:?}"),
            }
        }
    }
}

fn connect(port: u16) -> TcpStream {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let stream = TcpStream::connect_timeout(&address, LIMIT)
        .unwrap_or_else(|error| panic!("connect {address}: {error}"));
    stream
        .set_read_timeout(Some(LIMIT))
        .unwrap_or_else(|error| panic!("read timeout: {error}"));
    stream
}

fn frame(value: &str) -> [u8; 250] {
    let mut frame = [0; 250];
    frame[..value.len()].copy_from_slice(value.as_bytes());
    frame
}

fn events(log: &str) -> Vec<String> {
    log.lines()
        .filter_map(|line| string_field(line, "event").map(str::to_owned))
        .collect()
}

/// A string field of one JSON log line.
fn string_field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let start = line.find(&format!("\"{name}\":\""))? + name.len() + 4;
    line[start..].split('"').next()
}

/// A number field of one JSON log line.
fn number_field(line: &str, name: &str) -> Option<u64> {
    let start = line.find(&format!("\"{name}\":"))? + name.len() + 3;
    let digits: String = line[start..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

fn wait_for(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + LIMIT;
    while !done() {
        assert!(Instant::now() < deadline, "timed out: {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Runs a command to its end, killing it after `LIMIT`.
fn run_limited(command: &mut Command) -> Output {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("spawn {command:?}: {error}"));
    let deadline = Instant::now() + LIMIT;
    while child
        .try_wait()
        .unwrap_or_else(|error| panic!("wait {command:?}: {error}"))
        .is_none()
    {
        if Instant::now() >= deadline {
            let _ = child.kill();
            panic!("{command:?} did not end in {LIMIT:?}");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    child
        .wait_with_output()
        .unwrap_or_else(|error| panic!("output of {command:?}: {error}"))
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn assert_error(output: &Output, code: &str) {
    let stderr = text(&output.stderr);
    assert!(
        stderr.starts_with(&format!("error[{code}")),
        "{code}: {stderr}"
    );
    assert_eq!(text(&output.stdout), "", "{code}");
}
