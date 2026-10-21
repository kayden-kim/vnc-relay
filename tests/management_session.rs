//! Admin login, session view, and restart through the real `serve` binary
//! (management-api-v1 §1.4–§1.5, ops-info-v1 §1.5). The adapter tests in
//! crates/adapters/tests/management.rs cover the HTTP rules in process. These
//! tests cover the wiring in `serve`: the Argon2id check, the system clock, the
//! token source, and the ops log.

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use vnc_relay_adapters::accounts_file::Argon2Hasher;
use vnc_relay_app::ports::PasswordHasher;
use vnc_relay_domain::account::{Argon2Params, Password};

const BIN: &str = env!("CARGO_BIN_EXE_vnc-relay");

/// Time limit for the listeners to start.
const READY_LIMIT: Duration = Duration::from_secs(20);

const ALICE: &str = "alice";
const ALICE_PASSWORD: &str = "correct-horse-battery";
const BOB: &str = "bob";
const BOB_PASSWORD: &str = "another-long-password";

/// The lowest approved cost, as in tests/admin.rs. Keeps the hashing fast.
const PARAMS: Argon2Params = Argon2Params {
    memory_kib: 19_456,
    iterations: 2,
    parallelism: 1,
};

/// Tests pick free ports by binding port 0 and releasing it. Two tests that
/// run at the same time can get the same port, so each test takes this lock
/// first.
static PORTS_LOCK: Mutex<()> = Mutex::new(());

fn lock_ports() -> MutexGuard<'static, ()> {
    PORTS_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Four free loopback ports: management, status, tcp_viewer, tcp_server.
fn free_ports() -> [u16; 4] {
    // Hold all four until every port is chosen, so that they differ.
    let holders: Vec<TcpListener> = (0..4)
        .map(|_| TcpListener::bind("127.0.0.1:0").unwrap_or_else(|e| panic!("free port: {e}")))
        .collect();
    let mut ports = [0; 4];
    for (slot, holder) in ports.iter_mut().zip(&holders) {
        *slot = holder
            .local_addr()
            .unwrap_or_else(|e| panic!("local addr: {e}"))
            .port();
    }
    ports
}

fn setup_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "vnc-relay-session-{}-{name}",
        std::process::id()
    ));
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap_or_else(|e| panic!("clear {}: {e}", dir.display()));
    }
    fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("create {}: {e}", dir.display()));
    dir
}

fn write(path: &Path, text: &str) {
    fs::write(path, text).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

/// A real Argon2id hash of `password` with the test cost.
fn hash(password: &str) -> String {
    let typed = Password::parse(password.to_owned())
        .unwrap_or_else(|e| panic!("password: {e:?}"));
    let hashed = Argon2Hasher
        .hash(&typed, PARAMS)
        .unwrap_or_else(|e| panic!("hash: {e}"));
    hashed.as_str().to_owned()
}

/// The account file with two admin accounts, alice and bob.
fn write_accounts(dir: &Path) {
    let text = format!(
        "version = 1\n\n[[account]]\nname = \"{ALICE}\"\npassword_hash = \"{}\"\n\n[[account]]\nname = \"{BOB}\"\npassword_hash = \"{}\"\n",
        hash(ALICE_PASSWORD),
        hash(BOB_PASSWORD),
    );
    write(&dir.join("admin-accounts.toml"), &text);
}

/// Writes the config for `ports` into `dir` and returns its path.
fn write_config(dir: &Path, ports: [u16; 4]) -> PathBuf {
    let text = format!(
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
        ports[0], ports[1], ports[2], ports[3]
    );
    let path = dir.join("relay.toml");
    write(&path, &text);
    path
}

fn connects(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok()
}

/// A running `serve` child. Dropping it kills the child.
struct Server {
    child: Option<Child>,
}

impl Server {
    fn start(dir: &Path, ports: [u16; 4]) -> Self {
        let config = write_config(dir, ports);
        let child = Command::new(BIN)
            .arg("serve")
            .arg("--config")
            .arg(&config)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("spawn {BIN}: {e}"));
        let mut server = Self { child: Some(child) };
        server.wait_ready(ports);
        server
    }

    fn running(&mut self) -> &mut Child {
        self.child
            .as_mut()
            .unwrap_or_else(|| panic!("server already stopped"))
    }

    fn wait_ready(&mut self, ports: [u16; 4]) {
        let deadline = Instant::now() + READY_LIMIT;
        while !ports.into_iter().all(connects) {
            if let Ok(Some(status)) = self.running().try_wait() {
                panic!("vnc-relay exited before it was ready: {status}");
            }
            assert!(
                Instant::now() < deadline,
                "listeners not ready in {READY_LIMIT:?}"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Kills the child and returns its stdout, the ops log.
    fn stop(mut self) -> String {
        let mut child = self
            .child
            .take()
            .unwrap_or_else(|| panic!("server already stopped"));
        let _ = child.kill();
        let output = child
            .wait_with_output()
            .unwrap_or_else(|e| panic!("wait: {e}"));
        String::from_utf8_lossy(&output.stdout).into_owned()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// A parsed HTTP/1.1 response. Header names are lowercase.
struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

impl Reply {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

fn send(port: u16, method: &str, path: &str, headers: &[(&str, &str)], body: &str) -> Reply {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream =
        TcpStream::connect(addr).unwrap_or_else(|e| panic!("connect {addr}: {e}"));
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap_or_else(|e| panic!("read timeout: {e}"));
    let mut request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    );
    for (name, value) in headers {
        request.push_str(&format!("{name}: {value}\r\n"));
    }
    request.push_str("\r\n");
    request.push_str(body);
    stream
        .write_all(request.as_bytes())
        .unwrap_or_else(|e| panic!("write request: {e}"));
    let mut raw = Vec::new();
    stream
        .read_to_end(&mut raw)
        .unwrap_or_else(|e| panic!("read response: {e}"));
    parse(&String::from_utf8_lossy(&raw))
}

fn parse(raw: &str) -> Reply {
    let (head, body) = raw
        .split_once("\r\n\r\n")
        .unwrap_or_else(|| panic!("no end of head: {raw}"));
    let mut lines = head.lines();
    let status_line = lines.next().unwrap_or_else(|| panic!("no status line"));
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status code: {status_line}"));
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_owned()))
        .collect();
    Reply {
        status,
        headers,
        body: body.to_owned(),
    }
}

fn login(port: u16, account: &str, password: &str) -> Reply {
    let body = format!("{{\"account\":\"{account}\",\"password\":\"{password}\"}}");
    send(
        port,
        "POST",
        "/api/v1/session",
        &[("Content-Type", "application/json")],
        &body,
    )
}

/// The `relay_admin_session=<token>` pair of a login reply.
fn cookie_pair(reply: &Reply) -> String {
    let line = reply
        .header("set-cookie")
        .unwrap_or_else(|| panic!("no Set-Cookie: {}", reply.body));
    line.split(';').next().unwrap_or(line).to_owned()
}

/// The text of a string field in a JSON body. The bodies here have no
/// escapes, as in tests/serve.rs.
fn field<'a>(text: &'a str, name: &str) -> &'a str {
    let marker = format!("\"{name}\":\"");
    let start = text
        .find(&marker)
        .unwrap_or_else(|| panic!("no {name}: {text}"))
        + marker.len();
    let rest = &text[start..];
    let end = rest
        .find('"')
        .unwrap_or_else(|| panic!("unterminated {name}"));
    &rest[..end]
}

// Covers the plain loopback path through the real binary. The cookie has no
// `Secure` attribute here. The HTTPS path is covered in
// crates/adapters/tests/management.rs.
/// spec: management.md:MGT-017 partial
#[test]
fn plain_loopback_login_opens_the_session_and_logs_success() {
    let _guard = lock_ports();
    let dir = setup_dir("plain-login");
    write_accounts(&dir);
    let ports = free_ports();
    let server = Server::start(&dir, ports);

    let reply = login(ports[0], ALICE, ALICE_PASSWORD);
    assert_eq!(reply.status, 200, "{}", reply.body);
    let set_cookie = reply
        .header("set-cookie")
        .unwrap_or_else(|| panic!("no Set-Cookie"))
        .to_owned();
    assert!(set_cookie.contains("HttpOnly"), "{set_cookie}");
    assert!(set_cookie.contains("SameSite=Strict"), "{set_cookie}");
    assert!(!set_cookie.contains("Secure"), "{set_cookie}");
    let pair = cookie_pair(&reply);
    let token = pair.trim_start_matches("relay_admin_session=").to_owned();
    let csrf = field(&reply.body, "csrf_token").to_owned();

    let view = send(
        ports[0],
        "GET",
        "/api/v1/session",
        &[("Cookie", pair.as_str())],
        "",
    );
    assert_eq!(view.status, 200, "{}", view.body);
    assert!(view.body.contains("\"account\":\"alice\""), "{}", view.body);

    let log = server.stop();
    assert!(log.contains("\"event\":\"admin_login_succeeded\""), "{log}");
    assert!(log.contains("\"admin_account\":\"alice\""), "{log}");
    for secret in [
        ALICE_PASSWORD,
        token.as_str(),
        csrf.as_str(),
        "$argon2id$",
    ] {
        assert!(!log.contains(secret), "ops log has a secret: {log}");
    }
    let _ = fs::remove_dir_all(&dir);
}

// Covers the restart with the real binary: the new process refuses the old
// cookie, and a new login works. The terminate path (4.5) is not in 4.3.
/// spec: management.md:MGT-027 partial
#[test]
fn restart_ends_the_admin_session_and_login_works_again() {
    let _guard = lock_ports();
    let dir = setup_dir("restart");
    write_accounts(&dir);

    let first_ports = free_ports();
    let first = Server::start(&dir, first_ports);
    let reply = login(first_ports[0], ALICE, ALICE_PASSWORD);
    assert_eq!(reply.status, 200, "{}", reply.body);
    let old_pair = cookie_pair(&reply);
    first.stop();

    let second_ports = free_ports();
    let second = Server::start(&dir, second_ports);
    let stale = send(
        second_ports[0],
        "GET",
        "/api/v1/session",
        &[("Cookie", old_pair.as_str())],
        "",
    );
    assert_eq!(stale.status, 401, "{}", stale.body);
    assert!(
        stale.body.contains("\"code\":\"unauthenticated\""),
        "{}",
        stale.body
    );

    let again = login(second_ports[0], ALICE, ALICE_PASSWORD);
    assert_eq!(again.status, 200, "{}", again.body);
    let new_pair = cookie_pair(&again);
    let view = send(
        second_ports[0],
        "GET",
        "/api/v1/session",
        &[("Cookie", new_pair.as_str())],
        "",
    );
    assert_eq!(view.status, 200, "{}", view.body);
    second.stop();
    let _ = fs::remove_dir_all(&dir);
}

// Covers the login of each of two admin accounts through the binary, and a
// wrong password for the other account. Account setup by the local CLI is in
// tests/admin.rs. The query and terminate rights (4.4, 4.5) are not covered.
/// spec: operations.md:OPS-029 partial
#[test]
fn each_admin_account_logs_in_and_gets_its_own_session_view() {
    let _guard = lock_ports();
    let dir = setup_dir("two-accounts");
    write_accounts(&dir);
    let ports = free_ports();
    let server = Server::start(&dir, ports);

    for (account, password) in [(ALICE, ALICE_PASSWORD), (BOB, BOB_PASSWORD)] {
        let reply = login(ports[0], account, password);
        assert_eq!(reply.status, 200, "{account}: {}", reply.body);
        assert!(
            reply.body.contains(&format!("\"account\":\"{account}\"")),
            "{account}: {}",
            reply.body
        );
    }

    let wrong = login(ports[0], BOB, ALICE_PASSWORD);
    assert_eq!(wrong.status, 401, "{}", wrong.body);
    assert!(
        wrong.body.contains("\"code\":\"login_failed\""),
        "{}",
        wrong.body
    );
    server.stop();
    let _ = fs::remove_dir_all(&dir);
}
