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
#[cfg(target_os = "linux")]
use std::sync::Arc;
use std::sync::{Mutex, MutexGuard, PoisonError};
#[cfg(target_os = "linux")]
use std::thread::JoinHandle;
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
    let dir = std::env::temp_dir().join(format!("vnc-relay-session-{}-{name}", std::process::id()));
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
    let typed = Password::parse(password.to_owned()).unwrap_or_else(|e| panic!("password: {e:?}"));
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

[mode1]
enabled = true
allow_targets = ["127.0.0.1:1-65535"]
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
        let mut command = Command::new(BIN);
        command
            .arg("serve")
            .arg("--config")
            .arg(&config)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let child = command
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
            std::thread::sleep(Duration::from_millis(10));
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

    #[cfg(unix)]
    fn stop_normally(mut self) -> String {
        let pid = self.running().id().to_string();
        let status = Command::new("kill")
            .args(["-TERM", &pid])
            .status()
            .unwrap_or_else(|e| panic!("send SIGTERM: {e}"));
        assert!(status.success(), "normal shutdown request failed: {status}");
        let deadline = Instant::now() + READY_LIMIT;
        loop {
            if let Some(status) = self
                .running()
                .try_wait()
                .unwrap_or_else(|e| panic!("wait for normal shutdown: {e}"))
            {
                assert!(status.success(), "normal shutdown failed: {status}");
                break;
            }
            assert!(Instant::now() < deadline, "normal shutdown timed out");
            std::thread::sleep(Duration::from_millis(10));
        }
        let output = self
            .child
            .take()
            .unwrap_or_else(|| panic!("server already stopped"))
            .wait_with_output()
            .unwrap_or_else(|e| panic!("normal shutdown output: {e}"));
        let log = String::from_utf8_lossy(&output.stdout).into_owned();
        assert!(log.contains("\"event\":\"shutdown_started\""), "{log}");
        assert!(log.contains("\"event\":\"server_stopped\""), "{log}");
        log
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
    let mut stream = TcpStream::connect(addr).unwrap_or_else(|e| panic!("connect {addr}: {e}"));
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

fn query(port: u16, cookie: &str, path: &str) -> Reply {
    send(port, "GET", path, &[("Cookie", cookie)], "")
}

fn terminate(port: u16, cookie: &str, csrf: &str, sid: &str) -> Reply {
    send(
        port,
        "POST",
        &format!("/api/v1/sessions/{sid}/terminate"),
        &[("Cookie", cookie), ("X-CSRF-Token", csrf)],
        "",
    )
}

fn assert_unauthenticated(reply: &Reply) {
    assert_eq!(reply.status, 401, "{}", reply.body);
    assert!(
        reply.body.contains("\"code\":\"unauthenticated\""),
        "{}",
        reply.body
    );
    assert_eq!(
        reply.header("www-authenticate"),
        Some("Session realm=\"vnc-relay\"")
    );
}

fn assert_listed(port: u16, cookie: &str, sid: &str) {
    let reply = query(port, cookie, "/api/v1/sessions");
    assert_eq!(reply.status, 200, "{}", reply.body);
    assert!(
        reply.body.contains(&format!("\"session_id\":\"{sid}\"")),
        "{}",
        reply.body
    );
}

struct RelayPair {
    viewer: TcpStream,
    target: TcpStream,
    sid: String,
}

impl RelayPair {
    fn start(ports: [u16; 4], cookie: &str) -> Self {
        let listener =
            TcpListener::bind("127.0.0.1:0").unwrap_or_else(|e| panic!("bind target: {e}"));
        listener
            .set_nonblocking(true)
            .unwrap_or_else(|e| panic!("target mode: {e}"));
        let mut viewer = TcpStream::connect(("127.0.0.1", ports[2]))
            .unwrap_or_else(|e| panic!("viewer connect: {e}"));
        viewer
            .set_read_timeout(Some(READY_LIMIT))
            .unwrap_or_else(|e| panic!("viewer timeout: {e}"));
        viewer
            .set_write_timeout(Some(READY_LIMIT))
            .unwrap_or_else(|e| panic!("viewer timeout: {e}"));
        let mut greeting = [0; 12];
        viewer
            .read_exact(&mut greeting)
            .unwrap_or_else(|e| panic!("viewer greeting: {e}"));
        assert_eq!(&greeting, b"RFB 000.000\n");
        let address = listener
            .local_addr()
            .unwrap_or_else(|e| panic!("target address: {e}"))
            .to_string();
        let mut initial = [0; 250];
        initial[..address.len()].copy_from_slice(address.as_bytes());
        viewer
            .write_all(&initial)
            .unwrap_or_else(|e| panic!("viewer initial: {e}"));
        let deadline = Instant::now() + READY_LIMIT;
        let target = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "target accept timed out");
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => panic!("target accept: {e}"),
            }
        };
        target
            .set_nonblocking(false)
            .unwrap_or_else(|e| panic!("target mode: {e}"));
        target
            .set_read_timeout(Some(READY_LIMIT))
            .unwrap_or_else(|e| panic!("target timeout: {e}"));
        target
            .set_write_timeout(Some(READY_LIMIT))
            .unwrap_or_else(|e| panic!("target timeout: {e}"));
        let sid = loop {
            let reply = query(ports[0], cookie, "/api/v1/sessions");
            assert_eq!(reply.status, 200, "{}", reply.body);
            if reply.body.contains("\"session_id\":") {
                break field(&reply.body, "session_id").to_owned();
            }
            assert!(
                Instant::now() < deadline,
                "relay not listed: {}",
                reply.body
            );
            std::thread::sleep(Duration::from_millis(10));
        };
        Self {
            viewer,
            target,
            sid,
        }
    }

    fn transfer(&mut self) {
        self.viewer
            .write_all(b"viewer bytes")
            .unwrap_or_else(|e| panic!("viewer write: {e}"));
        let mut forward = [0; 12];
        self.target
            .read_exact(&mut forward)
            .unwrap_or_else(|e| panic!("target read: {e}"));
        assert_eq!(&forward, b"viewer bytes");
        self.target
            .write_all(b"target reply")
            .unwrap_or_else(|e| panic!("target write: {e}"));
        let mut reverse = [0; 12];
        self.viewer
            .read_exact(&mut reverse)
            .unwrap_or_else(|e| panic!("viewer read: {e}"));
        assert_eq!(&reverse, b"target reply");
    }

    fn terminate(&mut self, port: u16, cookie: &str, csrf: &str) {
        let reply = terminate(port, cookie, csrf, &self.sid);
        assert_eq!(reply.status, 200, "{}", reply.body);
        assert!(
            reply.body.contains("\"close_cause\":\"admin_terminated\""),
            "{}",
            reply.body
        );
        for stream in [&mut self.viewer, &mut self.target] {
            let mut byte = [0];
            assert_eq!(
                stream
                    .read(&mut byte)
                    .unwrap_or_else(|e| panic!("relay EOF: {e}")),
                0
            );
        }
        let list = query(port, cookie, "/api/v1/sessions");
        assert_eq!(list.status, 200, "{}", list.body);
        assert!(!list.body.contains(&self.sid), "{}", list.body);
    }
}

/// spec: management.md:MGT-020
#[test]
fn logout_refuses_the_old_cookie_but_keeps_other_admin_and_relay_sessions() {
    let _guard = lock_ports();
    let dir = setup_dir("logout");
    write_accounts(&dir);
    let ports = free_ports();
    let server = Server::start(&dir, ports);
    let alice = login(ports[0], ALICE, ALICE_PASSWORD);
    let bob = login(ports[0], BOB, BOB_PASSWORD);
    assert_eq!(alice.status, 200, "{}", alice.body);
    assert_eq!(bob.status, 200, "{}", bob.body);
    let alice_cookie = cookie_pair(&alice);
    let bob_cookie = cookie_pair(&bob);
    let alice_csrf = field(&alice.body, "csrf_token");
    let mut relay = RelayPair::start(ports, &bob_cookie);
    relay.transfer();

    let logout = send(
        ports[0],
        "DELETE",
        "/api/v1/session",
        &[("Cookie", &alice_cookie), ("X-CSRF-Token", alice_csrf)],
        "",
    );
    assert_eq!(logout.status, 204, "{}", logout.body);
    assert!(logout.body.is_empty(), "{}", logout.body);
    assert!(
        logout
            .header("set-cookie")
            .is_some_and(|v| v.starts_with("relay_admin_session=")
                && v.contains("Max-Age=0")
                && v.contains("Path=/"))
    );
    assert_unauthenticated(&query(ports[0], &alice_cookie, "/api/v1/summary"));
    assert_unauthenticated(&terminate(ports[0], &alice_cookie, alice_csrf, &relay.sid));
    let bob_view = query(ports[0], &bob_cookie, "/api/v1/session");
    assert_eq!(bob_view.status, 200, "{}", bob_view.body);
    assert_eq!(field(&bob_view.body, "account"), BOB);
    assert_listed(ports[0], &bob_cookie, &relay.sid);
    relay.transfer();
    relay.terminate(ports[0], &bob_cookie, field(&bob.body, "csrf_token"));
    server.stop();
    let _ = fs::remove_dir_all(&dir);
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
    for secret in [ALICE_PASSWORD, token.as_str(), csrf.as_str(), "$argon2id$"] {
        assert!(!log.contains(secret), "ops log has a secret: {log}");
    }
    let _ = fs::remove_dir_all(&dir);
}

/// spec: management.md:MGT-027
#[cfg(unix)]
#[test]
fn restart_ends_the_admin_session_and_login_works_again() {
    restart_session_scenario(Server::stop_normally);
}

#[cfg(unix)]
fn restart_session_scenario(stop: fn(Server) -> String) {
    let _guard = lock_ports();
    let dir = setup_dir("restart");
    write_accounts(&dir);
    let ports = free_ports();
    let first = Server::start(&dir, ports);
    let reply = login(ports[0], ALICE, ALICE_PASSWORD);
    assert_eq!(reply.status, 200, "{}", reply.body);
    let old_pair = cookie_pair(&reply);
    let old_csrf = field(&reply.body, "csrf_token").to_owned();
    assert_eq!(query(ports[0], &old_pair, "/api/v1/summary").status, 200);
    stop(first);

    // Keep exactly the same config, including listeners and account file.
    let second = Server::start(&dir, ports);
    let observer = login(ports[0], BOB, BOB_PASSWORD);
    assert_eq!(observer.status, 200, "{}", observer.body);
    let observer_pair = cookie_pair(&observer);
    let mut relay = RelayPair::start(ports, &observer_pair);
    relay.transfer();
    assert_unauthenticated(&query(ports[0], &old_pair, "/api/v1/sessions"));
    assert_unauthenticated(&terminate(ports[0], &old_pair, &old_csrf, &relay.sid));
    assert_listed(ports[0], &observer_pair, &relay.sid);
    relay.transfer();

    let again = login(ports[0], ALICE, ALICE_PASSWORD);
    assert_eq!(again.status, 200, "{}", again.body);
    let new_pair = cookie_pair(&again);
    assert_ne!(new_pair, old_pair);
    assert_eq!(query(ports[0], &new_pair, "/api/v1/summary").status, 200);
    assert_listed(ports[0], &new_pair, &relay.sid);
    relay.terminate(ports[0], &new_pair, field(&again.body, "csrf_token"));
    stop(second);
    let _ = fs::remove_dir_all(&dir);
}

// Windows retains the login-only evidence: Linux below also exercises local
// CLI hidden input and restricted Unix file permissions end to end.
/// spec: operations.md:OPS-029 partial
#[test]
#[cfg(not(target_os = "linux"))]
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

// As in tests/admin.rs, script supplies a real PTY and a reader collects its
// transcript. Unlike a fixed settle delay, observing -echo on that PTY waits
// for the password reader to enter hidden mode before typing.
#[cfg(target_os = "linux")]
struct PtySession {
    child: Child,
    transcript: Arc<Mutex<Vec<u8>>>,
    reader: Option<JoinHandle<()>>,
}

#[cfg(target_os = "linux")]
impl PtySession {
    fn start(account: &str, config: &Path) -> Self {
        let command = format!(
            "printf '__TTY='; tty; '{BIN}' admin add {account} --config '{}'; echo __RC=$?",
            config.display()
        );
        let mut child = Command::new("script")
            .args(["-q", "-E", "always", "-c", &command, "/dev/null"])
            .env("SHELL", "/bin/sh")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|e| panic!("start script: {e}"));
        let mut stdout = child
            .stdout
            .take()
            .unwrap_or_else(|| panic!("script stdout missing"));
        let transcript = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&transcript);
        let reader = std::thread::spawn(move || {
            let mut chunk = [0; 4096];
            loop {
                let n = stdout
                    .read(&mut chunk)
                    .unwrap_or_else(|e| panic!("PTY transcript: {e}"));
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
        let deadline = Instant::now() + READY_LIMIT;
        while !self.text().contains(marker) {
            assert!(
                Instant::now() < deadline,
                "PTY did not print {marker:?}: {}",
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
        let deadline = Instant::now() + READY_LIMIT;
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
            assert!(
                Instant::now() < deadline,
                "password input never disabled echo"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        let stdin = self
            .child
            .stdin
            .as_mut()
            .unwrap_or_else(|| panic!("script stdin missing"));
        stdin
            .write_all(format!("{password}\n").as_bytes())
            .unwrap_or_else(|e| panic!("type password: {e}"));
        stdin
            .flush()
            .unwrap_or_else(|e| panic!("flush password: {e}"));
    }

    fn finish(mut self) -> String {
        self.wait_for("__RC=");
        self.child.stdin.take();
        let deadline = Instant::now() + READY_LIMIT;
        loop {
            if let Some(status) = self
                .child
                .try_wait()
                .unwrap_or_else(|e| panic!("wait script: {e}"))
            {
                assert!(status.success(), "script failed: {status}");
                break;
            }
            assert!(
                Instant::now() < deadline,
                "script did not exit: {}",
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

#[cfg(target_os = "linux")]
impl Drop for PtySession {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// Full end-to-end evidence on Linux only. Windows retains the explicitly
// partial login test above; this does not claim Windows hidden-input coverage.
/// spec: operations.md:OPS-029
#[cfg(target_os = "linux")]
#[test]
fn cli_prepared_admins_have_hidden_salted_credentials_and_equal_relay_rights() {
    use std::os::unix::fs::PermissionsExt;
    use vnc_relay_adapters::accounts_file::{self, HashFloor};

    let _guard = lock_ports();
    let dir = setup_dir("cli-two-accounts");
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))
        .unwrap_or_else(|e| panic!("restrict account directory: {e}"));
    let ports = free_ports();
    let config = write_config(&dir, ports);
    let accounts_path = dir.join("admin-accounts.toml");
    for (account, password) in [(ALICE, ALICE_PASSWORD), (BOB, BOB_PASSWORD)] {
        let mut terminal = PtySession::start(account, &config);
        terminal.password("비밀번호: ", password);
        terminal.password("비밀번호 확인: ", password);
        let transcript = terminal.finish();
        assert!(transcript.contains("__RC=0"), "{transcript}");
        assert!(
            transcript.contains(&format!("계정 '{account}'을(를) 추가했다.")),
            "{transcript}"
        );
        for secret in [ALICE_PASSWORD, BOB_PASSWORD] {
            assert!(
                !transcript.contains(secret),
                "typed password in PTY: {transcript}"
            );
        }
    }
    let text = fs::read_to_string(&accounts_path).unwrap_or_else(|e| panic!("account file: {e}"));
    for password in [ALICE_PASSWORD, BOB_PASSWORD] {
        assert!(!text.contains(password), "plaintext password stored");
    }
    assert_eq!(
        fs::metadata(&accounts_path)
            .unwrap_or_else(|e| panic!("account metadata: {e}"))
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(&dir)
            .unwrap_or_else(|e| panic!("directory metadata: {e}"))
            .permissions()
            .mode()
            & 0o077,
        0
    );
    let accounts = accounts_file::load(&accounts_path, HashFloor::Reject)
        .unwrap_or_else(|e| panic!("load CLI accounts: {e:?}"))
        .accounts;
    assert_eq!(accounts.len(), 2);
    let mut salts = Vec::new();
    for account in &accounts {
        let hash = account.password_hash.as_str();
        assert!(
            hash.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"),
            "{hash}"
        );
        let parts: Vec<_> = hash.split('$').collect();
        assert_eq!(parts.len(), 6, "{hash}");
        assert_eq!(parts[4].len(), 22, "16-byte unpadded salt");
        assert_eq!(parts[5].len(), 43, "32-byte unpadded digest");
        salts.push(parts[4]);
    }
    assert_ne!(salts[0], salts[1], "separate accounts need different salts");

    let server = Server::start(&dir, ports);
    for (account, password) in [(ALICE, ALICE_PASSWORD), (BOB, BOB_PASSWORD)] {
        let reply = login(ports[0], account, password);
        assert_eq!(reply.status, 200, "{account}: {}", reply.body);
        assert_eq!(field(&reply.body, "account"), account);
        let cookie = cookie_pair(&reply);
        assert_eq!(query(ports[0], &cookie, "/api/v1/summary").status, 200);
        let mut relay = RelayPair::start(ports, &cookie);
        relay.transfer();
        assert_listed(ports[0], &cookie, &relay.sid);
        relay.terminate(ports[0], &cookie, field(&reply.body, "csrf_token"));
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
