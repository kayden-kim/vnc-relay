//! Real-binary shutdown scenarios shared by the OS signal wrappers.

use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use vnc_relay_adapters::accounts_file::Argon2Hasher;
use vnc_relay_app::ports::PasswordHasher;
use vnc_relay_domain::account::{Argon2Params, Password};

const LIMIT: Duration = Duration::from_secs(20);
const PASSWORD: &str = "correct-horse-battery";
static NEXT_DIR: AtomicU64 = AtomicU64::new(0);
static PORTS: Mutex<()> = Mutex::new(());

pub trait ServerProcess: Sized {
    fn start(config: &Path) -> Self;
    fn request_stop(&mut self);
    fn try_wait(&mut self) -> Option<i32>;
}

pub struct Fixture {
    pub config: PathBuf,
    /// Management, status, TCP Viewer, TCP Server.
    pub ports: [u16; 4],
}

impl Fixture {
    pub fn new(name: &str, grace_secs: u64) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "vnc-relay-shutdown-{}-{}-{name}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap_or_else(|error| panic!("create fixture: {error}"));
        let holders: Vec<_> = (0..4)
            .map(|_| {
                TcpListener::bind("127.0.0.1:0")
                    .unwrap_or_else(|error| panic!("reserve port: {error}"))
            })
            .collect();
        let mut ports = [0; 4];
        for (port, listener) in ports.iter_mut().zip(&holders) {
            *port = listener
                .local_addr()
                .unwrap_or_else(|error| panic!("reserved address: {error}"))
                .port();
        }
        let password = Password::parse(PASSWORD.to_owned())
            .unwrap_or_else(|error| panic!("test password: {error:?}"));
        let hash = Argon2Hasher
            .hash(
                &password,
                Argon2Params {
                    memory_kib: 19_456,
                    iterations: 2,
                    parallelism: 1,
                },
            )
            .unwrap_or_else(|error| panic!("test password hash: {error}"));
        let accounts = dir.join("admin-accounts.toml");
        fs::write(
            &accounts,
            format!(
                "version = 1\n[[account]]\nname = \"alice\"\npassword_hash = \"{}\"\n",
                hash.as_str()
            ),
        )
        .unwrap_or_else(|error| panic!("write accounts: {error}"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&accounts, fs::Permissions::from_mode(0o600))
                .unwrap_or_else(|error| panic!("protect accounts: {error}"));
        }
        let config = dir.join("relay.toml");
        fs::write(
            &config,
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
[mode1]
enabled = true
allow_targets = ["127.0.0.1:1-65535"]
[mode2]
enabled = true
[timeouts]
handshake_secs = 120
wait_secs = 120
connect_secs = 120
close_secs = 60
shutdown_grace_secs = {grace_secs}
"#,
                ports[0], ports[1], ports[2], ports[3]
            ),
        )
        .unwrap_or_else(|error| panic!("write config: {error}"));
        #[cfg(windows)]
        {
            let log_path = dir.join("stdout.log").to_string_lossy().replace('\\', "/");
            let mut file = fs::OpenOptions::new()
                .append(true)
                .open(&config)
                .unwrap_or_else(|error| panic!("open Windows log config: {error}"));
            file.write_all(format!("\n[log]\noutput = \"{log_path}\"\n").as_bytes())
                .unwrap_or_else(|error| panic!("write Windows log config: {error}"));
        }
        Self { config, ports }
    }

    pub fn wait_ready<P: ServerProcess>(&self, process: &mut P) {
        let deadline = Instant::now() + LIMIT;
        loop {
            if self.log().contains("\"event\":\"server_ready\"") {
                break;
            }
            assert_eq!(process.try_wait(), None, "server exited before ready");
            assert!(
                Instant::now() < deadline,
                "server not ready: {}",
                self.log()
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_status(self.ports[1], "/livez", 200, r#"{"status":"alive"}"#);
        assert_status(self.ports[1], "/readyz", 200, r#"{"status":"ready"}"#);
    }

    pub fn login(&self) -> Admin {
        login(self.ports[0], "alice", PASSWORD)
    }

    pub fn query(&self, admin: &Admin, path: &str) -> Reply {
        send(self.ports[0], "GET", path, &[("Cookie", &admin.cookie)], "")
    }

    pub fn terminate(&self, admin: &Admin, sid: &str) -> Reply {
        send(
            self.ports[0],
            "POST",
            &format!("/api/v1/sessions/{sid}/terminate"),
            &[("Cookie", &admin.cookie), ("X-CSRF-Token", &admin.csrf)],
            "",
        )
    }

    fn log(&self) -> String {
        fs::read_to_string(self.config.with_file_name("stdout.log"))
            .unwrap_or_else(|error| panic!("read child log: {error}"))
    }

    fn wait_line(&self, wanted: impl Fn(&str) -> bool) -> String {
        let deadline = Instant::now() + LIMIT;
        loop {
            let log = self.log();
            if let Some(line) = log.lines().find(|line| wanted(line)) {
                return line.to_owned();
            }
            assert!(Instant::now() < deadline, "missing log event: {log}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn event(&self, event: &str) -> String {
        self.wait_line(|line| string_field_opt(line, "event") == Some(event))
    }

    fn wait_summary(&self, admin: &Admin, wanted: impl Fn(&str) -> bool) -> String {
        let deadline = Instant::now() + LIMIT;
        loop {
            let reply = self.query(admin, "/api/v1/summary");
            assert_eq!(reply.status, 200, "{}", reply.body);
            if wanted(&reply.body) {
                return reply.body;
            }
            assert!(
                Instant::now() < deadline,
                "unexpected summary: {}",
                reply.body
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn closed(&self, sid: &str) -> String {
        self.wait_line(|line| {
            string_field_opt(line, "event") == Some("session_closed")
                && string_field_opt(line, "session_id") == Some(sid)
        })
    }

    fn assert_final_log(&self) {
        let log = self.log();
        let events: Vec<_> = log
            .lines()
            .map(|line| string_field(line, "event"))
            .collect();
        assert_eq!(events.first(), Some(&"server_starting"), "{log}");
        assert_eq!(events.last(), Some(&"server_stopped"), "{log}");
        let started = position(&events, "shutdown_started");
        if let Some(forced) = events.iter().position(|event| *event == "shutdown_forced") {
            assert!(started < forced, "{log}");
            for (index, line) in log.lines().enumerate() {
                if string_field_opt(line, "close_cause") == Some("server_shutdown") {
                    assert!(
                        forced < index,
                        "forced event must precede session closure: {log}"
                    );
                }
            }
        }
        for port in self.ports {
            let address = SocketAddr::from(([127, 0, 0, 1], port));
            #[cfg(unix)]
            match TcpStream::connect_timeout(&address, Duration::from_secs(1)) {
                Err(error) if error.kind() == ErrorKind::ConnectionRefused => {}
                other => panic!("listener {port} was not released: {other:?}"),
            }
            #[cfg(windows)]
            {
                let rebound = TcpListener::bind(address);
                assert!(
                    rebound.is_ok(),
                    "listener {port} was not released: {rebound:?}"
                );
            }
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(dir) = self.config.parent() {
            let _ = fs::remove_dir_all(dir);
        }
    }
}

pub struct Admin {
    pub cookie: String,
    pub csrf: String,
}

pub struct Reply {
    pub status: u16,
    pub body: String,
    headers: Vec<(String, String)>,
}

pub fn send(port: u16, method: &str, path: &str, headers: &[(&str, &str)], body: &str) -> Reply {
    let mut stream = connect(port);
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
        .unwrap_or_else(|error| panic!("HTTP write: {error}"));
    let mut raw = String::new();
    stream
        .read_to_string(&mut raw)
        .unwrap_or_else(|error| panic!("HTTP read: {error}"));
    let (head, body) = raw
        .split_once("\r\n\r\n")
        .unwrap_or_else(|| panic!("HTTP head: {raw}"));
    let mut lines = head.lines();
    let status = lines
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("HTTP status: {head}"));
    Reply {
        status,
        body: body.to_owned(),
        headers: lines
            .filter_map(|line| line.split_once(':'))
            .map(|(key, value)| (key.to_ascii_lowercase(), value.trim().to_owned()))
            .collect(),
    }
}

pub fn login(port: u16, account: &str, password: &str) -> Admin {
    let reply = send(
        port,
        "POST",
        "/api/v1/session",
        &[("Content-Type", "application/json")],
        &format!(r#"{{"account":"{account}","password":"{password}"}}"#),
    );
    assert_eq!(reply.status, 200, "login failed");
    assert_eq!(string_field(&reply.body, "account"), account);
    let cookie = reply
        .headers
        .iter()
        .find(|(key, _)| key == "set-cookie")
        .map(|(_, value)| value.split(';').next().unwrap_or(value).to_owned())
        .unwrap_or_else(|| panic!("missing session cookie"));
    Admin {
        cookie,
        csrf: string_field(&reply.body, "csrf_token").to_owned(),
    }
}

// These fields contain only contract-defined ASCII tokens; no JSON escapes.
pub fn string_field<'a>(text: &'a str, name: &str) -> &'a str {
    string_field_opt(text, name).unwrap_or_else(|| panic!("missing string field {name}"))
}

fn string_field_opt<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let marker = format!("\"{name}\":\"");
    let rest = text.get(text.find(&marker)? + marker.len()..)?;
    rest.get(..rest.find('"')?)
}

fn number(text: &str, name: &str) -> u64 {
    let marker = format!("\"{name}\":");
    let rest = text
        .split_once(&marker)
        .map(|(_, rest)| rest.trim_start())
        .unwrap_or_else(|| panic!("missing number {name}: {text}"));
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits
        .parse()
        .unwrap_or_else(|error| panic!("number {name}: {error}: {text}"))
}

fn position(events: &[&str], wanted: &str) -> usize {
    events
        .iter()
        .position(|event| *event == wanted)
        .unwrap_or_else(|| panic!("missing {wanted}: {events:?}"))
}

fn connect(port: u16) -> TcpStream {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let stream = TcpStream::connect_timeout(&address, LIMIT)
        .unwrap_or_else(|error| panic!("connect {address}: {error}"));
    stream
        .set_read_timeout(Some(LIMIT))
        .unwrap_or_else(|error| panic!("read timeout: {error}"));
    stream
        .set_write_timeout(Some(LIMIT))
        .unwrap_or_else(|error| panic!("write timeout: {error}"));
    stream
}

fn viewer(fixture: &Fixture) -> TcpStream {
    let mut stream = connect(fixture.ports[2]);
    let mut greeting = [0; 12];
    stream
        .read_exact(&mut greeting)
        .unwrap_or_else(|error| panic!("viewer greeting: {error}"));
    assert_eq!(&greeting, b"RFB 000.000\n");
    stream
}

fn request(stream: &mut TcpStream, value: &str) {
    let mut frame = [0; 250];
    frame[..value.len()].copy_from_slice(value.as_bytes());
    stream
        .write_all(&frame)
        .unwrap_or_else(|error| panic!("initial request: {error}"));
}

fn assert_closed(stream: &mut TcpStream) {
    let mut byte = [0];
    match stream.read(&mut byte) {
        Ok(0) => {}
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted | ErrorKind::BrokenPipe
            ) => {}
        other => panic!("connection was not closed: {other:?}"),
    }
}

fn assert_status(port: u16, path: &str, status: u16, body: &str) {
    let reply = send(port, "GET", path, &[], "");
    assert_eq!(reply.status, status);
    // Status objects are flat string fields. Compare all fields without
    // depending on serde_json's map ordering.
    let fields = |text: &str| {
        let mut fields: Vec<_> = text
            .trim()
            .trim_start_matches('{')
            .trim_end_matches('}')
            .split(',')
            .map(str::trim)
            .map(str::to_owned)
            .collect();
        fields.sort_unstable();
        fields
    };
    assert_eq!(fields(&reply.body), fields(body));
}

fn wait_exit<P: ServerProcess>(process: &mut P, limit: Duration) {
    let deadline = Instant::now() + limit;
    loop {
        if let Some(code) = process.try_wait() {
            assert_eq!(code, 0, "normal shutdown exit code");
            return;
        }
        assert!(
            Instant::now() < deadline,
            "normal shutdown exceeded {limit:?}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

pub struct RelayPair {
    pub viewer: TcpStream,
    pub server: TcpStream,
    pub sid: String,
}

impl RelayPair {
    pub fn start(fixture: &Fixture, id: &str) -> Self {
        let before = fixture.log();
        let previous_waiting: Vec<_> = before
            .lines()
            .filter(|line| string_field_opt(line, "event") == Some("waiting_registered"))
            .map(|line| string_field(line, "conn_id"))
            .collect();
        let mut server = connect(fixture.ports[3]);
        request(&mut server, &format!("ID:{id}"));
        let registered = fixture.wait_line(|line| {
            string_field_opt(line, "event") == Some("waiting_registered")
                && string_field_opt(line, "role") == Some("server")
                && !previous_waiting.contains(&string_field(line, "conn_id"))
        });
        let server_conn = string_field(&registered, "conn_id");
        let mut viewer = viewer(fixture);
        request(&mut viewer, &format!("ID:{id}"));
        let mut pair = Self {
            viewer,
            server,
            sid: String::new(),
        };
        pair.transfer();
        let started = fixture.wait_line(|line| {
            string_field_opt(line, "event") == Some("session_started")
                && string_field_opt(line, "server_conn_id") == Some(server_conn)
        });
        pair.sid = string_field(&started, "session_id").to_owned();
        pair
    }

    pub fn transfer(&mut self) {
        self.viewer
            .write_all(b"viewer-data")
            .unwrap_or_else(|error| panic!("viewer write: {error}"));
        let mut received = [0; 11];
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

    pub fn finish(&mut self) {
        self.viewer
            .shutdown(Shutdown::Write)
            .unwrap_or_else(|error| panic!("viewer EOF: {error}"));
        assert_closed(&mut self.server);
        self.server
            .shutdown(Shutdown::Write)
            .unwrap_or_else(|error| panic!("server EOF: {error}"));
        assert_closed(&mut self.viewer);
    }

    fn assert_closed(&mut self) {
        assert_closed(&mut self.viewer);
        assert_closed(&mut self.server);
    }
}

/// A bounded local listen queue, never accepted. Verify saturation before use.
struct Backlog {
    listener: TcpListener,
    _fillers: Vec<TcpStream>,
}

impl Backlog {
    fn saturated() -> Self {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .unwrap_or_else(|error| panic!("backlog runtime: {error}"));
        let listener = runtime.block_on(async {
            let socket = tokio::net::TcpSocket::new_v4()
                .unwrap_or_else(|error| panic!("backlog socket: {error}"));
            socket
                .bind(SocketAddr::from(([127, 0, 0, 1], 0)))
                .unwrap_or_else(|error| panic!("backlog bind: {error}"));
            socket
                .listen(1)
                .unwrap_or_else(|error| panic!("backlog listen: {error}"))
                .into_std()
                .unwrap_or_else(|error| panic!("backlog std listener: {error}"))
        });
        let address = listener
            .local_addr()
            .unwrap_or_else(|error| panic!("backlog address: {error}"));
        let mut fillers = Vec::new();
        for _ in 0..64 {
            match TcpStream::connect_timeout(&address, Duration::from_millis(100)) {
                Ok(stream) => fillers.push(stream),
                Err(error)
                    if matches!(error.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock) =>
                {
                    assert!(!fillers.is_empty(), "backlog has established fillers");
                    return Self {
                        listener,
                        _fillers: fillers,
                    };
                }
                Err(error) => panic!("OS cannot supply a pending local target: {error}"),
            }
        }
        panic!("OS did not enforce a bounded local listen queue");
    }

    fn address(&self) -> SocketAddr {
        self.listener
            .local_addr()
            .unwrap_or_else(|error| panic!("target address: {error}"))
    }
}

pub fn stop_unfinished_work<P: ServerProcess>() {
    let _lock = PORTS.lock().unwrap_or_else(PoisonError::into_inner);
    let fixture = Fixture::new("unfinished", 60);
    let mut process = P::start(&fixture.config);
    fixture.wait_ready(&mut process);
    let admin = fixture.login();
    let mut active = RelayPair::start(&fixture, "7000");
    let mut initial = viewer(&fixture);
    initial
        .write_all(b"ID:7")
        .unwrap_or_else(|error| panic!("partial initial request: {error}"));
    let mut waiting_server = connect(fixture.ports[3]);
    request(&mut waiting_server, "ID:7001");
    let mut waiting_viewer = viewer(&fixture);
    request(&mut waiting_viewer, "ID:7002");
    let backlog = Backlog::saturated();
    let mut target = viewer(&fixture);
    request(&mut target, &backlog.address().to_string());
    fixture.wait_summary(&admin, |body| {
        number(body, "handshaking") == 1
            && number(body, "waiting") == 2
            && number(body, "target_connecting") == 1
            && number(body, "active") == 1
    });
    let before = fixture.log();
    let accepted: Vec<_> = before
        .lines()
        .filter(|line| string_field_opt(line, "event") == Some("connection_accepted"))
        .map(|line| string_field(line, "conn_id").to_owned())
        .collect();
    process.request_stop();
    fixture.event("shutdown_started");
    assert_closed(&mut initial);
    assert_closed(&mut waiting_server);
    assert_closed(&mut waiting_viewer);
    assert_closed(&mut target);
    fixture.wait_summary(&admin, |body| {
        number(body, "waiting") == 0
            && number(body, "handshaking") == 0
            && number(body, "target_connecting") == 0
    });
    // New accepts may fail at TCP connect or close without a greeting; neither
    // may create a session, including an attempted match for the removed IDs.
    for (port, id) in [(fixture.ports[2], "7001"), (fixture.ports[3], "7002")] {
        let address = SocketAddr::from(([127, 0, 0, 1], port));
        if let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_secs(1)) {
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap_or_else(|error| panic!("new connection timeout: {error}"));
            let _ = stream.write_all(&make_frame(&format!("ID:{id}")));
            assert_closed(&mut stream);
        }
    }
    let log = fixture.log();
    for conn in accepted {
        if log.lines().any(|line| {
            string_field_opt(line, "event") == Some("session_started")
                && (string_field_opt(line, "viewer_conn_id") == Some(&conn)
                    || string_field_opt(line, "server_conn_id") == Some(&conn))
        }) {
            continue;
        }
        assert!(
            log.lines()
                .any(|line| string_field_opt(line, "conn_id") == Some(&conn)
                    && matches!(
                        string_field_opt(line, "event"),
                        Some("connection_aborted" | "waiting_removed")
                    )
                    && string_field_opt(line, "reason") == Some("server_shutdown")),
            "missing cleanup for {conn}: {log}"
        );
    }
    assert!(
        log.lines().any(
            |line| string_field_opt(line, "event") == Some("connection_aborted")
                && string_field_opt(line, "stage") == Some("target_connect")
                && string_field_opt(line, "reason") == Some("server_shutdown")
        ),
        "{log}"
    );
    active.transfer();
    active.finish();
    wait_exit(&mut process, LIMIT);
    let log = fixture.log();
    let after_shutdown = log
        .split_once("\"event\":\"shutdown_started\"")
        .map(|(_, rest)| rest)
        .unwrap_or_else(|| panic!("missing shutdown event"));
    assert!(
        !after_shutdown.contains("\"event\":\"session_started\""),
        "{log}"
    );
    assert_eq!(
        log.lines()
            .filter(|line| string_field_opt(line, "event") == Some("waiting_registered"))
            .count(),
        3,
        "{log}"
    );
    fixture.assert_final_log();
}

fn make_frame(value: &str) -> [u8; 250] {
    let mut frame = [0; 250];
    frame[..value.len()].copy_from_slice(value.as_bytes());
    frame
}

pub fn active_session_finishes_in_grace<P: ServerProcess>() {
    let _lock = PORTS.lock().unwrap_or_else(PoisonError::into_inner);
    let fixture = Fixture::new("voluntary", 60);
    let mut process = P::start(&fixture.config);
    fixture.wait_ready(&mut process);
    let mut pair = RelayPair::start(&fixture, "8000");
    let began = Instant::now();
    process.request_stop();
    let started = fixture.event("shutdown_started");
    assert_eq!(number(&started, "active_sessions"), 1);
    let relay_until = Instant::now() + Duration::from_millis(300);
    loop {
        pair.transfer();
        assert_eq!(process.try_wait(), None);
        if Instant::now() >= relay_until {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    pair.finish();
    wait_exit(&mut process, LIMIT);
    assert!(began.elapsed() < Duration::from_secs(60));
    assert_eq!(
        string_field(&fixture.closed(&pair.sid), "close_cause"),
        "peer_eof"
    );
    assert!(!fixture.log().contains("\"event\":\"shutdown_forced\""));
    fixture.assert_final_log();
}

pub fn grace_expiry_cleans_session<P: ServerProcess>() {
    let _lock = PORTS.lock().unwrap_or_else(PoisonError::into_inner);
    for grace in [3, 0] {
        let fixture = Fixture::new("expiry", grace);
        let mut process = P::start(&fixture.config);
        fixture.wait_ready(&mut process);
        let mut pair = RelayPair::start(&fixture, "9000");
        let began = Instant::now();
        process.request_stop();
        let started = fixture.event("shutdown_started");
        assert_eq!(number(&started, "grace_secs"), grace);
        if grace != 0 {
            while began.elapsed() < Duration::from_secs(1) {
                pair.transfer();
                assert_eq!(process.try_wait(), None);
                assert!(!fixture.log().contains("\"event\":\"shutdown_forced\""));
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        let forced = fixture.event("shutdown_forced");
        assert!(
            began.elapsed() >= Duration::from_secs(grace),
            "shutdown_forced appeared before monotonic G elapsed"
        );
        assert_eq!(number(&forced, "remaining_sessions"), 1);
        wait_exit(&mut process, LIMIT);
        pair.assert_closed();
        assert_eq!(
            string_field(&fixture.closed(&pair.sid), "close_cause"),
            "server_shutdown"
        );
        fixture.assert_final_log();
    }
}

pub fn restart_has_fresh_state<P: ServerProcess>() {
    let _lock = PORTS.lock().unwrap_or_else(PoisonError::into_inner);
    let fixture = Fixture::new("restart", 0);
    let mut first = P::start(&fixture.config);
    fixture.wait_ready(&mut first);
    let admin = fixture.login();
    let mut old = RelayPair::start(&fixture, "10000");
    old.transfer();
    let mut waiting = connect(fixture.ports[3]);
    request(&mut waiting, "ID:10001");
    fixture.wait_summary(&admin, |body| {
        number(body, "waiting") == 1 && number(body, "active") == 1
    });
    first.request_stop();
    wait_exit(&mut first, LIMIT);
    old.assert_closed();
    assert_closed(&mut waiting);
    fixture.assert_final_log();
    drop(first);

    let mut second = P::start(&fixture.config);
    fixture.wait_ready(&mut second);
    let rejected = fixture.query(&admin, "/api/v1/summary");
    assert_eq!(rejected.status, 401);
    assert_eq!(string_field(&rejected.body, "code"), "unauthenticated");
    let fresh_admin = fixture.login();
    let summary = fixture.query(&fresh_admin, "/api/v1/summary");
    assert_eq!(summary.status, 200);
    for key in ["waiting", "active", "handshaking", "target_connecting"] {
        assert_eq!(number(&summary.body, key), 0, "{}", summary.body);
    }
    let relay_usage = summary
        .body
        .split_once("\"relay_connections\":")
        .map(|(_, rest)| rest)
        .unwrap_or_else(|| panic!("missing relay usage"));
    assert_eq!(number(relay_usage, "current"), 0);
    for path in [
        "/api/v1/waiting",
        "/api/v1/sessions",
        "/api/v1/closed-sessions",
    ] {
        let reply = fixture.query(&fresh_admin, path);
        assert_eq!(reply.status, 200);
        assert!(reply.body.contains("\"items\":[]"), "{}", reply.body);
    }
    let old_detail = fixture.query(&fresh_admin, &format!("/api/v1/sessions/{}", old.sid));
    assert_eq!(old_detail.status, 404);
    let mut unmatched = viewer(&fixture);
    request(&mut unmatched, "ID:10001");
    fixture.wait_summary(&fresh_admin, |body| {
        number(body, "waiting") == 1 && number(body, "active") == 0
    });
    let mut fresh = RelayPair::start(&fixture, "10000");
    assert_ne!(fresh.sid, old.sid);
    let deadline = Instant::now() + LIMIT;
    loop {
        let detail = fixture.query(&fresh_admin, &format!("/api/v1/sessions/{}", fresh.sid));
        assert_eq!(detail.status, 200);
        let forward = number(&detail.body, "bytes_viewer_to_target");
        let reverse = number(&detail.body, "bytes_target_to_viewer");
        assert!(
            forward <= 11 && reverse <= 11,
            "old counters restored: {}",
            detail.body
        );
        if forward == 11 && reverse == 11 {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "new counters not published: {}",
            detail.body
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    fresh.transfer();
    fresh.finish();
    second.request_stop();
    wait_exit(&mut second, LIMIT);
    assert_closed(&mut unmatched);
    fixture.assert_final_log();
}

pub fn second_request_forces_cleanup<P: ServerProcess>() {
    let _lock = PORTS.lock().unwrap_or_else(PoisonError::into_inner);
    let fixture = Fixture::new("second", 60);
    let mut process = P::start(&fixture.config);
    fixture.wait_ready(&mut process);
    let admin = fixture.login();
    let mut untouched = RelayPair::start(&fixture, "42000");
    let mut closing = RelayPair::start(&fixture, "42001");
    closing
        .viewer
        .shutdown(Shutdown::Write)
        .unwrap_or_else(|error| panic!("start natural cleanup: {error}"));
    assert_closed(&mut closing.server);
    let detail = fixture.query(&admin, &format!("/api/v1/sessions/{}", closing.sid));
    assert_eq!(detail.status, 200);
    assert_eq!(string_field(&detail.body, "state"), "closing");
    process.request_stop();
    fixture.event("shutdown_started");
    untouched.transfer();
    assert_eq!(process.try_wait(), None);
    let began = Instant::now();
    process.request_stop();
    wait_exit(&mut process, Duration::from_secs(5));
    assert!(began.elapsed() < Duration::from_secs(5));
    untouched.assert_closed();
    closing.assert_closed();
    assert_eq!(
        string_field(&fixture.closed(&untouched.sid), "close_cause"),
        "server_shutdown"
    );
    let closed = fixture.closed(&closing.sid);
    assert_eq!(string_field(&closed, "close_cause"), "peer_eof");
    assert_eq!(string_field(&closed, "side"), "viewer");
    assert_eq!(
        number(&fixture.event("shutdown_forced"), "remaining_sessions"),
        2
    );
    fixture.assert_final_log();
}

pub fn management_works_during_grace<P: ServerProcess>() {
    let _lock = PORTS.lock().unwrap_or_else(PoisonError::into_inner);
    let fixture = Fixture::new("management", 60);
    let mut process = P::start(&fixture.config);
    fixture.wait_ready(&mut process);
    let before = fixture.login();
    let mut keeper = RelayPair::start(&fixture, "43000");
    let mut terminated = RelayPair::start(&fixture, "43001");
    process.request_stop();
    fixture.event("shutdown_started");
    let admin = fixture.login();
    for account in [&before, &admin] {
        let summary = fixture.query(account, "/api/v1/summary");
        assert_eq!(summary.status, 200);
        assert_eq!(string_field(&summary.body, "state"), "shutdown_grace");
        assert_eq!(number(&summary.body, "active"), 2);
    }
    assert_status(fixture.ports[1], "/livez", 200, r#"{"status":"alive"}"#);
    assert_status(
        fixture.ports[1],
        "/readyz",
        503,
        r#"{"status":"not_ready","reason":"shutting_down"}"#,
    );
    // The real peer acknowledges EOF, allowing terminate to complete inside E.
    let server = terminated
        .server
        .try_clone()
        .unwrap_or_else(|error| panic!("clone server peer: {error}"));
    let viewer = terminated
        .viewer
        .try_clone()
        .unwrap_or_else(|error| panic!("clone viewer peer: {error}"));
    let peers = std::thread::spawn(move || {
        let mut server = server;
        let mut viewer = viewer;
        assert_closed(&mut server);
        server
            .shutdown(Shutdown::Write)
            .unwrap_or_else(|error| panic!("server close acknowledgement: {error}"));
        assert_closed(&mut viewer);
        viewer
            .shutdown(Shutdown::Write)
            .unwrap_or_else(|error| panic!("viewer close acknowledgement: {error}"));
    });
    let reply = fixture.terminate(&admin, &terminated.sid);
    peers
        .join()
        .unwrap_or_else(|_| panic!("close acknowledgement thread failed"));
    assert_eq!(reply.status, 200, "{}", reply.body);
    assert_eq!(string_field(&reply.body, "result"), "terminated");
    assert_eq!(string_field(&reply.body, "close_cause"), "admin_terminated");
    terminated.assert_closed();
    let audit = fixture.wait_line(|line| {
        string_field_opt(line, "event") == Some("admin_terminate")
            && string_field_opt(line, "session_id") == Some(terminated.sid.as_str())
    });
    assert_eq!(string_field(&audit, "admin_account"), "alice");
    fixture.wait_summary(&admin, |body| {
        number(body, "active") == 1 && string_field(body, "state") == "shutdown_grace"
    });
    assert_status(fixture.ports[1], "/livez", 200, r#"{"status":"alive"}"#);
    assert_status(
        fixture.ports[1],
        "/readyz",
        503,
        r#"{"status":"not_ready","reason":"shutting_down"}"#,
    );
    keeper.transfer();
    keeper.finish();
    wait_exit(&mut process, LIMIT);
    assert!(!fixture.log().contains("\"event\":\"shutdown_forced\""));
    fixture.assert_final_log();
}

#[cfg(windows)]
pub fn console_close_forces_cleanup<P: ServerProcess>(close: impl FnOnce(&mut P)) {
    let _lock = PORTS.lock().unwrap_or_else(PoisonError::into_inner);
    let fixture = Fixture::new("console-close", 60);
    let mut process = P::start(&fixture.config);
    fixture.wait_ready(&mut process);
    let mut pair = RelayPair::start(&fixture, "44000");
    let began = Instant::now();
    close(&mut process);
    wait_exit(&mut process, Duration::from_secs(5));
    assert!(
        began.elapsed() < Duration::from_secs(5),
        "console close must beat the OS timeout"
    );
    pair.assert_closed();
    let started = fixture.event("shutdown_started");
    assert_eq!(string_field(&started, "trigger"), "requested");
    assert_eq!(number(&started, "grace_secs"), 60);
    assert_eq!(
        number(&fixture.event("shutdown_forced"), "remaining_sessions"),
        1
    );
    assert_eq!(
        string_field(&fixture.closed(&pair.sid), "close_cause"),
        "server_shutdown"
    );
    fixture.assert_final_log();
}
