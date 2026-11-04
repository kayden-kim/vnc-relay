//! Acceptance tests for relay logging, secret exclusion, and keepalive duration.
//! Covers:
//! - OPS-011 (full): JSON Lines, lifecycle identifier correlation and administrator termination audit
//! - OPS-012 (partial): Operations log and status endpoint secret and raw ID non-exposure
//! - RLY-027 (full): Active idle session preservation over K and post-idle transfer

use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use vnc_relay_adapters::accounts_file::Argon2Hasher;
use vnc_relay_app::ports::PasswordHasher;
use vnc_relay_domain::account::{Argon2Params, Password};

const BIN: &str = env!("CARGO_BIN_EXE_vnc-relay");
const LIMIT: Duration = Duration::from_secs(20);
const ADMIN_PASSWORD: &str = "operations-log-admin-password";
static SERIAL: AtomicU64 = AtomicU64::new(0);
static PORTS: Mutex<()> = Mutex::new(());

fn lock_ports() -> std::sync::MutexGuard<'static, ()> {
    PORTS.lock().unwrap_or_else(PoisonError::into_inner)
}

fn some<T>(option: Option<T>, context: &str) -> T {
    match option {
        Some(value) => value,
        None => panic!("missing {context}"),
    }
}

fn make_mode2_request(id: &str) -> [u8; 250] {
    let text = format!("ID:{id}");
    assert!(text.len() <= 250, "ID too long: {id}");
    let mut bytes = [0u8; 250];
    bytes[..text.len()].copy_from_slice(text.as_bytes());
    bytes
}

fn make_mode1_request(target: &str) -> [u8; 250] {
    assert!(target.len() <= 250, "target too long: {target}");
    let mut bytes = [0u8; 250];
    bytes[..target.len()].copy_from_slice(target.as_bytes());
    bytes
}

fn make_raw_request(value: &[u8]) -> [u8; 250] {
    let mut bytes = [0u8; 250];
    let len = value.len().min(250);
    bytes[..len].copy_from_slice(&value[..len]);
    bytes
}

fn assert_timestamp(ts: &str) {
    let bytes = ts.as_bytes();
    assert_eq!(bytes.len(), 24, "timestamp must be 24 bytes: {ts}");
    for (i, byte) in bytes.iter().enumerate() {
        let expected = match i {
            4 | 7 => Some(b'-'),
            10 => Some(b'T'),
            13 | 16 => Some(b':'),
            19 => Some(b'.'),
            23 => Some(b'Z'),
            _ => None,
        };
        match expected {
            Some(exp) => assert_eq!(*byte, exp, "bad char at index {i} in {ts}"),
            None => assert!(byte.is_ascii_digit(), "char at {i} must be digit in {ts}"),
        }
    }
}

fn is_16_hex(s: &str) -> bool {
    s.len() == 16
        && s.chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum JsonValue {
    String(String),
    Number(i64),
    Bool(bool),
    Object(Vec<(String, JsonValue)>),
    Array(Vec<JsonValue>),
    Null,
}

impl JsonValue {
    fn str(&self, key: &str) -> Option<&str> {
        match self {
            JsonValue::Object(entries) => {
                for (k, v) in entries {
                    if k == key {
                        return v.as_str();
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn num(&self, key: &str) -> Option<i64> {
        match self {
            JsonValue::Object(entries) => {
                for (k, v) in entries {
                    if k == key {
                        return v.as_i64();
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn has_key(&self, key: &str) -> bool {
        match self {
            JsonValue::Object(entries) => entries.iter().any(|(k, _)| k == key),
            _ => false,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    fn as_i64(&self) -> Option<i64> {
        match self {
            JsonValue::Number(n) => Some(*n),
            _ => None,
        }
    }
}

struct JsonParser<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            input: s.as_bytes(),
            pos: 0,
        }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.input.len() && self.input[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.input.get(self.pos).copied()
    }

    fn consume(&mut self, expected: u8) {
        self.skip_ws();
        if self.pos < self.input.len() && self.input[self.pos] == expected {
            self.pos += 1;
        } else {
            panic!("expected byte {}", expected as char);
        }
    }

    fn parse_string(&mut self) -> String {
        self.consume(b'"');
        let mut s = String::new();
        while self.pos < self.input.len() {
            let b = self.input[self.pos];
            self.pos += 1;
            if b == b'"' {
                return s;
            }
            if b == b'\\' {
                if self.pos >= self.input.len() {
                    panic!("unterminated escape in string");
                }
                let esc = self.input[self.pos];
                self.pos += 1;
                match esc {
                    b'"' => s.push('"'),
                    b'\\' => s.push('\\'),
                    b'/' => s.push('/'),
                    b'b' => s.push('\x08'),
                    b'f' => s.push('\x0C'),
                    b'n' => s.push('\n'),
                    b'r' => s.push('\r'),
                    b't' => s.push('\t'),
                    _ => s.push(esc as char),
                }
            } else {
                s.push(b as char);
            }
        }
        panic!("unterminated string");
    }

    fn parse_number(&mut self) -> i64 {
        self.skip_ws();
        let start = self.pos;
        if self.pos < self.input.len()
            && (self.input[self.pos] == b'-' || self.input[self.pos] == b'+')
        {
            self.pos += 1;
        }
        while self.pos < self.input.len() && self.input[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        let s = match std::str::from_utf8(&self.input[start..self.pos]) {
            Ok(v) => v,
            Err(e) => panic!("invalid utf8 number: {e:?}"),
        };
        match s.parse::<i64>() {
            Ok(n) => n,
            Err(e) => panic!("invalid i64 '{s}': {e:?}"),
        }
    }

    fn parse_value(&mut self) -> JsonValue {
        self.skip_ws();
        match self.peek() {
            Some(b'"') => JsonValue::String(self.parse_string()),
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b't') => {
                self.pos += 4;
                JsonValue::Bool(true)
            }
            Some(b'f') => {
                self.pos += 5;
                JsonValue::Bool(false)
            }
            Some(b'n') => {
                self.pos += 4;
                JsonValue::Null
            }
            Some(b'0'..=b'9' | b'-') => JsonValue::Number(self.parse_number()),
            other => panic!("unexpected json token: {other:?}"),
        }
    }

    fn parse_array(&mut self) -> JsonValue {
        self.consume(b'[');
        let mut items = Vec::new();
        if self.peek() == Some(b']') {
            self.consume(b']');
            return JsonValue::Array(items);
        }
        loop {
            items.push(self.parse_value());
            match self.peek() {
                Some(b',') => self.consume(b','),
                Some(b']') => {
                    self.consume(b']');
                    break;
                }
                other => panic!("expected ',' or ']', found {other:?}"),
            }
        }
        JsonValue::Array(items)
    }

    fn parse_object(&mut self) -> JsonValue {
        self.consume(b'{');
        let mut entries = Vec::new();
        if self.peek() == Some(b'}') {
            self.consume(b'}');
            return JsonValue::Object(entries);
        }
        loop {
            let key = self.parse_string();
            self.consume(b':');
            let val = self.parse_value();
            entries.push((key, val));
            match self.peek() {
                Some(b',') => self.consume(b','),
                Some(b'}') => {
                    self.consume(b'}');
                    break;
                }
                other => panic!("expected ',' or '}}', found {other:?}"),
            }
        }
        JsonValue::Object(entries)
    }
}

fn parse_json_object_line(line: &str) -> JsonValue {
    let mut parser = JsonParser::new(line);
    let val = parser.parse_value();
    parser.skip_ws();
    assert_eq!(
        parser.pos,
        parser.input.len(),
        "trailing garbage on JSON line: {line}"
    );
    match val {
        JsonValue::Object(_) => val,
        other => panic!("expected object, got {other:?} for line: {line}"),
    }
}

fn parse_log_objects(dir: &Path) -> Vec<JsonValue> {
    let log_path = dir.join("events.jsonl");
    let content = match std::fs::read_to_string(&log_path) {
        Ok(v) => v,
        Err(e) => panic!("read {}: {e:?}", log_path.display()),
    };
    assert!(!content.is_empty(), "log file is empty");
    assert!(
        content.ends_with('\n'),
        "log file does not end with LF newline"
    );
    let mut objects = Vec::new();
    let mut line_count = 0;
    for line in content.lines() {
        line_count += 1;
        assert!(!line.is_empty(), "empty line in log file at {line_count}");
        objects.push(parse_json_object_line(line));
    }
    assert_eq!(
        line_count,
        objects.len(),
        "line count mismatch with parsed objects"
    );
    objects
}

fn extract_field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("\"{name}\":\"");
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

fn field_matches(line: &str, name: &str, expected: &str) -> bool {
    extract_field(line, name) == Some(expected)
}

fn connect_from(source: [u8; 4], port: u16) -> TcpStream {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => panic!("runtime: {e:?}"),
    };
    runtime.block_on(async {
        let socket = match tokio::net::TcpSocket::new_v4() {
            Ok(s) => s,
            Err(e) => panic!("socket: {e:?}"),
        };
        if let Err(e) = socket.bind(SocketAddr::from((source, 0))) {
            panic!("bind {source:?}: {e:?}");
        }
        let stream = match socket
            .connect(SocketAddr::from(([127, 0, 0, 1], port)))
            .await
        {
            Ok(s) => s,
            Err(e) => panic!("connect {port}: {e:?}"),
        };
        let std_stream = match stream.into_std() {
            Ok(s) => s,
            Err(e) => panic!("into std: {e:?}"),
        };
        if let Err(e) = std_stream.set_nonblocking(false) {
            panic!("blocking: {e:?}");
        }
        std_stream
    })
}

fn http_get(port: u16, path: &str) -> String {
    let mut stream = match TcpStream::connect(("127.0.0.1", port)) {
        Ok(s) => s,
        Err(e) => panic!("connect status {port}: {e:?}"),
    };
    if let Err(e) = stream.set_read_timeout(Some(LIMIT)) {
        panic!("read timeout: {e:?}");
    }
    if let Err(e) = stream.set_write_timeout(Some(LIMIT)) {
        panic!("write timeout: {e:?}");
    }
    let req = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
    if let Err(e) = stream.write_all(req.as_bytes()) {
        panic!("write GET {path}: {e:?}");
    }
    let mut resp = Vec::new();
    if let Err(e) = stream.read_to_end(&mut resp) {
        panic!("read GET {path}: {e:?}");
    }
    match String::from_utf8(resp) {
        Ok(s) => s,
        Err(e) => panic!("utf8 {path}: {e:?}"),
    }
}

fn read_management_reply(mut stream: TcpStream) -> (u16, String, JsonValue) {
    let mut raw = String::new();
    stream
        .read_to_string(&mut raw)
        .unwrap_or_else(|error| panic!("read management response: {error}"));
    let (head, body) = raw
        .split_once("\r\n\r\n")
        .unwrap_or_else(|| panic!("management response has headers: {raw}"));
    let status = some(head.split_whitespace().nth(1), "HTTP status")
        .parse::<u16>()
        .unwrap_or_else(|error| panic!("management HTTP status: {error}"));
    (status, head.to_owned(), parse_json_object_line(body))
}

struct TargetServer {
    listener: TcpListener,
    port: u16,
}

impl TargetServer {
    fn bind() -> Self {
        let listener = match TcpListener::bind("127.0.0.1:0") {
            Ok(l) => l,
            Err(e) => panic!("bind target server: {e:?}"),
        };
        let port = match listener.local_addr() {
            Ok(addr) => addr.port(),
            Err(e) => panic!("target local addr: {e:?}"),
        };
        Self { listener, port }
    }

    /// Waits at most `LIMIT`; a rejected target request fails the test instead of hanging the run.
    fn accept(&self) -> TcpStream {
        if let Err(e) = self.listener.set_nonblocking(true) {
            panic!("target nonblocking: {e:?}");
        }
        let deadline = Instant::now() + LIMIT;
        let stream = loop {
            match self.listener.accept() {
                Ok((stream, _)) => break stream,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        Instant::now() < deadline,
                        "target port {} received no connection",
                        self.port
                    );
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => panic!("accept target stream: {e:?}"),
            }
        };
        // Windows accepted sockets inherit the listener's non-blocking mode.
        if let Err(e) = stream.set_nonblocking(false) {
            panic!("target stream blocking: {e:?}");
        }
        if let Err(e) = stream.set_read_timeout(Some(LIMIT)) {
            panic!("target read timeout: {e:?}");
        }
        if let Err(e) = stream.set_write_timeout(Some(LIMIT)) {
            panic!("target write timeout: {e:?}");
        }
        if let Err(e) = stream.set_nodelay(true) {
            panic!("target nodelay: {e:?}");
        }
        stream
    }
}

struct RunningRelay {
    child: Child,
    dir: PathBuf,
    status: u16,
    management: u16,
    viewer: u16,
    server: u16,
    ws_viewer: Option<u16>,
}

impl RunningRelay {
    fn start(name: &str, extra_config: &str, wait_secs: u64, places: u32, with_ws: bool) -> Self {
        let port_count = if with_ws { 5 } else { 4 };
        let holders: Vec<_> = (0..port_count)
            .map(|_| match TcpListener::bind("127.0.0.1:0") {
                Ok(l) => l,
                Err(e) => panic!("reserve ephemeral port: {e:?}"),
            })
            .collect();
        let ports: Vec<_> = holders
            .iter()
            .map(|l| match l.local_addr() {
                Ok(addr) => addr.port(),
                Err(e) => panic!("local addr: {e:?}"),
            })
            .collect();
        let dir = std::env::temp_dir().join(format!(
            "vnc-relay-log-{}-{}-{name}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        if let Err(e) = std::fs::create_dir_all(&dir) {
            panic!("create dir {}: {e:?}", dir.display());
        }
        let accounts = dir.join("accounts.toml");
        let password = Password::parse(ADMIN_PASSWORD.to_owned())
            .unwrap_or_else(|error| panic!("admin password: {error:?}"));
        let hash = Argon2Hasher
            .hash(
                &password,
                Argon2Params {
                    memory_kib: 19_456,
                    iterations: 2,
                    parallelism: 1,
                },
            )
            .unwrap_or_else(|error| panic!("admin password hash: {error}"));
        std::fs::write(
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
            if let Err(e) =
                std::fs::set_permissions(&accounts, std::fs::Permissions::from_mode(0o600))
            {
                panic!("set accounts perm: {e:?}");
            }
        }
        let ws_table = if with_ws {
            format!(
                r#"
[relay.ws_viewer]
enabled = true
listen = "127.0.0.1:{}"
allow = ["127.0.0.1"]
allowed_origins = ["http://127.0.0.1"]
"#,
                ports[4]
            )
        } else {
            String::new()
        };
        let config = format!(
            r#"[admin]
accounts_file = "accounts.toml"
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
{ws_table}
[mode1]
enabled = true
allow_targets = ["127.0.0.1:1024-65535"]
[mode2]
enabled = true
[limits]
relay_connections = {places}
waiting = {places}
[timeouts]
handshake_secs = 2
wait_secs = {wait_secs}
connect_secs = 2
close_secs = 2
[log]
output = "events.jsonl"
{extra_config}
"#,
            ports[0], ports[1], ports[2], ports[3]
        );
        let config_path = dir.join("relay.toml");
        if let Err(e) = std::fs::write(&config_path, config) {
            panic!("write config: {e:?}");
        }
        let stderr_file = match std::fs::File::create(dir.join("stderr")) {
            Ok(f) => f,
            Err(e) => panic!("create stderr file: {e:?}"),
        };
        drop(holders);
        let child = match Command::new(BIN)
            .args(["serve", "--config"])
            .arg(&config_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(stderr_file)
            .spawn()
        {
            Ok(c) => c,
            Err(e) => panic!("spawn {BIN}: {e:?}"),
        };
        let mut running = Self {
            child,
            dir,
            status: ports[1],
            management: ports[0],
            viewer: ports[2],
            server: ports[3],
            ws_viewer: if with_ws { Some(ports[4]) } else { None },
        };
        let deadline = Instant::now() + LIMIT;
        loop {
            if running
                .lines()
                .iter()
                .any(|line| field_matches(line, "event", "server_ready"))
            {
                return running;
            }
            let status = match running.child.try_wait() {
                Ok(s) => s,
                Err(e) => panic!("try_wait: {e:?}"),
            };
            assert!(
                status.is_none() && Instant::now() < deadline,
                "server did not become ready: {status:?}; {}",
                std::fs::read_to_string(running.dir.join("stderr")).unwrap_or_default()
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn lines(&self) -> Vec<String> {
        std::fs::read_to_string(self.dir.join("events.jsonl"))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    fn wait_for_line_after(&self, after: usize, predicate: impl Fn(&str) -> bool) -> String {
        let deadline = Instant::now() + LIMIT;
        loop {
            let lines = self.lines();
            if let Some(line) = lines.iter().skip(after).find(|l| predicate(l)) {
                return (*line).clone();
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for event; lines: {lines:#?}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn management_begin(
        &self,
        method: &str,
        path: &str,
        headers: &[&str],
        body: &str,
    ) -> TcpStream {
        let mut stream = TcpStream::connect(("127.0.0.1", self.management))
            .unwrap_or_else(|error| panic!("connect management: {error}"));
        stream
            .set_read_timeout(Some(LIMIT))
            .unwrap_or_else(|error| panic!("management read timeout: {error}"));
        stream
            .set_write_timeout(Some(LIMIT))
            .unwrap_or_else(|error| panic!("management write timeout: {error}"));
        let mut request = format!(
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: {}\r\n",
            body.len()
        );
        for header in headers {
            request.push_str(header);
            request.push_str("\r\n");
        }
        request.push_str("\r\n");
        request.push_str(body);
        stream
            .write_all(request.as_bytes())
            .unwrap_or_else(|error| panic!("send management request: {error}"));
        stream
    }

    fn connect_viewer(&self) -> (TcpStream, String) {
        let after = self.lines().len();
        let mut stream = match TcpStream::connect(("127.0.0.1", self.viewer)) {
            Ok(s) => s,
            Err(e) => panic!("connect viewer: {e:?}"),
        };
        if let Err(e) = stream.set_read_timeout(Some(LIMIT)) {
            panic!("read timeout: {e:?}");
        }
        if let Err(e) = stream.set_write_timeout(Some(LIMIT)) {
            panic!("write timeout: {e:?}");
        }
        if let Err(e) = stream.set_nodelay(true) {
            panic!("nodelay: {e:?}");
        }
        let mut greeting = [0u8; 12];
        if let Err(e) = stream.read_exact(&mut greeting) {
            panic!("read greeting: {e:?}");
        }
        assert_eq!(&greeting, b"RFB 000.000\n");
        let accepted = self.wait_for_line_after(after, |l| {
            field_matches(l, "event", "connection_accepted") && field_matches(l, "role", "viewer")
        });
        let conn_id = some(
            extract_field(&accepted, "conn_id"),
            "accepted viewer conn_id",
        )
        .to_owned();
        (stream, conn_id)
    }

    fn connect_server(&self) -> (TcpStream, String) {
        let after = self.lines().len();
        let stream = match TcpStream::connect(("127.0.0.1", self.server)) {
            Ok(s) => s,
            Err(e) => panic!("connect server: {e:?}"),
        };
        if let Err(e) = stream.set_read_timeout(Some(LIMIT)) {
            panic!("read timeout: {e:?}");
        }
        if let Err(e) = stream.set_write_timeout(Some(LIMIT)) {
            panic!("write timeout: {e:?}");
        }
        if let Err(e) = stream.set_nodelay(true) {
            panic!("nodelay: {e:?}");
        }
        let accepted = self.wait_for_line_after(after, |l| {
            field_matches(l, "event", "connection_accepted") && field_matches(l, "role", "server")
        });
        let conn_id = some(
            extract_field(&accepted, "conn_id"),
            "accepted server conn_id",
        )
        .to_owned();
        (stream, conn_id)
    }
}

impl Drop for RunningRelay {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// spec: operations.md:OPS-011
#[test]
fn operations_log_records_contract_events_and_correlates_identifiers() {
    let _lock = lock_ports();
    let relay = RunningRelay::start("ops011", "", 10, 10, false);

    // 1. connection_rejected at accept (from forbidden source address 127.0.0.3)
    let after_reject_accept = relay.lines().len();
    {
        let mut stranger = connect_from([127, 0, 0, 3], relay.viewer);
        let _ = stranger.write_all(b"UNAUTHORIZED_SOURCE_DATA");
    }
    relay.wait_for_line_after(after_reject_accept, |l| {
        field_matches(l, "event", "connection_rejected")
            && field_matches(l, "stage", "accept")
            && field_matches(l, "reason", "ip_not_allowed")
    });

    // 2. connection_aborted during initial exchange (peer disconnects before sending handshake)
    let after_abort = relay.lines().len();
    {
        let (mut abort_viewer, _conn_id) = relay.connect_viewer();
        let _ = abort_viewer.write_all(b"");
        drop(abort_viewer);
    }
    relay.wait_for_line_after(after_abort, |l| {
        field_matches(l, "event", "connection_aborted")
            && field_matches(l, "stage", "initial_exchange")
            && field_matches(l, "reason", "peer_eof")
    });

    // 3. connection_rejected at initial_exchange (malformed initial request syntax in 250 bytes)
    let after_bad_handshake = relay.lines().len();
    let (mut bad_viewer, bad_conn_id) = relay.connect_viewer();
    if let Err(e) = bad_viewer.write_all(&make_raw_request(b"MALFORMED_INITIAL_EXCHANGE_DATA")) {
        panic!("write malformed: {e:?}");
    }
    relay.wait_for_line_after(after_bad_handshake, |l| {
        field_matches(l, "event", "connection_rejected")
            && field_matches(l, "stage", "initial_exchange")
            && field_matches(l, "reason", "handshake_invalid")
            && field_matches(l, "conn_id", &bad_conn_id)
    });
    drop(bad_viewer);

    // 4. target_connect_failed (Mode I to allowed target port where no listener exists)
    let unused_target = TargetServer::bind();
    let unused_port = unused_target.port;
    drop(unused_target); // Port is now unused
    let after_target_failed = relay.lines().len();
    let (mut target_viewer, target_viewer_conn_id) = relay.connect_viewer();
    let req = format!("127.0.0.1:{unused_port}");
    if let Err(e) = target_viewer.write_all(&make_mode1_request(&req)) {
        panic!("write mode1 target: {e:?}");
    }
    relay.wait_for_line_after(after_target_failed, |l| {
        field_matches(l, "event", "target_connect_failed")
            && field_matches(l, "conn_id", &target_viewer_conn_id)
    });
    drop(target_viewer);

    // 5. connection_rejected at target_connect (Mode I to target port 22 not in allow_targets)
    let after_target_rejected = relay.lines().len();
    let (mut forbidden_viewer, forbidden_conn_id) = relay.connect_viewer();
    if let Err(e) = forbidden_viewer.write_all(&make_mode1_request("127.0.0.1:22")) {
        panic!("write forbidden target: {e:?}");
    }
    relay.wait_for_line_after(after_target_rejected, |l| {
        field_matches(l, "event", "connection_rejected")
            && field_matches(l, "stage", "target_connect")
            && field_matches(l, "reason", "target_not_allowed")
            && field_matches(l, "conn_id", &forbidden_conn_id)
    });
    drop(forbidden_viewer);

    // 6. waiting_registered and waiting_expired (wait_secs = 10, wait until expiration)
    let after_expired = relay.lines().len();
    let (mut expiring_server, expiring_conn_id) = relay.connect_server();
    if let Err(e) = expiring_server.write_all(&make_mode2_request("1001")) {
        panic!("write mode2: {e:?}");
    }
    relay.wait_for_line_after(after_expired, |l| {
        field_matches(l, "event", "waiting_registered")
            && field_matches(l, "conn_id", &expiring_conn_id)
    });
    relay.wait_for_line_after(after_expired, |l| {
        field_matches(l, "event", "waiting_expired")
            && field_matches(l, "conn_id", &expiring_conn_id)
    });
    drop(expiring_server);

    // 7. waiting_registered and waiting_removed (peer disconnects while waiting)
    let after_removed = relay.lines().len();
    let (mut removed_server, removed_conn_id) = relay.connect_server();
    if let Err(e) = removed_server.write_all(&make_mode2_request("1002")) {
        panic!("write mode2: {e:?}");
    }
    relay.wait_for_line_after(after_removed, |l| {
        field_matches(l, "event", "waiting_registered")
            && field_matches(l, "conn_id", &removed_conn_id)
    });
    drop(removed_server);
    relay.wait_for_line_after(after_removed, |l| {
        field_matches(l, "event", "waiting_removed")
            && field_matches(l, "conn_id", &removed_conn_id)
            && field_matches(l, "reason", "peer_eof")
    });

    // 8. connection_rejected at register (duplicate ID registered for same role)
    let after_dup = relay.lines().len();
    let (mut server_waiter, server_waiter_conn_id) = relay.connect_server();
    if let Err(e) = server_waiter.write_all(&make_mode2_request("1003")) {
        panic!("write mode2 waiter: {e:?}");
    }
    relay.wait_for_line_after(after_dup, |l| {
        field_matches(l, "event", "waiting_registered")
            && field_matches(l, "conn_id", &server_waiter_conn_id)
    });

    let (mut duplicate_server, dup_server_conn_id) = relay.connect_server();
    if let Err(e) = duplicate_server.write_all(&make_mode2_request("1003")) {
        panic!("write duplicate mode2: {e:?}");
    }
    relay.wait_for_line_after(after_dup, |l| {
        field_matches(l, "event", "connection_rejected")
            && field_matches(l, "stage", "register")
            && field_matches(l, "reason", "duplicate_id")
            && field_matches(l, "conn_id", &dup_server_conn_id)
    });
    drop(duplicate_server);

    // 9. match (session_started) and session_closed for session 1
    let (mut viewer_matcher, viewer_matcher_conn_id) = relay.connect_viewer();
    if let Err(e) = viewer_matcher.write_all(&make_mode2_request("1003")) {
        panic!("write viewer match: {e:?}");
    }
    let started_s1 = relay.wait_for_line_after(after_dup, |l| {
        field_matches(l, "event", "session_started")
            && field_matches(l, "server_conn_id", &server_waiter_conn_id)
            && field_matches(l, "viewer_conn_id", &viewer_matcher_conn_id)
    });
    let s1_session_id = some(extract_field(&started_s1, "session_id"), "s1 session_id").to_owned();
    let s1_ref = some(
        extract_field(&started_s1, "relay_id_ref"),
        "s1 relay_id_ref",
    )
    .to_owned();

    // Exchange 4 bytes viewer -> server
    if let Err(e) = viewer_matcher.write_all(b"PONG") {
        panic!("write payload: {e:?}");
    }
    let mut srv_buf = [0u8; 4];
    if let Err(e) = server_waiter.read_exact(&mut srv_buf) {
        panic!("read payload: {e:?}");
    }
    assert_eq!(&srv_buf, b"PONG");
    drop(viewer_matcher);
    drop(server_waiter);

    let closed_s1 = relay.wait_for_line_after(after_dup, |l| {
        field_matches(l, "event", "session_closed")
            && field_matches(l, "session_id", &s1_session_id)
    });
    assert_eq!(extract_field(&closed_s1, "close_cause"), Some("peer_eof"));

    // 10. RLY-R11 check: reuse same ID "1003" to verify fresh session_id with same relay_id_ref
    let after_s2 = relay.lines().len();
    let (mut server_s2, server_s2_conn_id) = relay.connect_server();
    if let Err(e) = server_s2.write_all(&make_mode2_request("1003")) {
        panic!("write mode2 s2 server: {e:?}");
    }
    relay.wait_for_line_after(after_s2, |l| {
        field_matches(l, "event", "waiting_registered")
            && field_matches(l, "conn_id", &server_s2_conn_id)
    });

    let (mut viewer_s2, viewer_s2_conn_id) = relay.connect_viewer();
    if let Err(e) = viewer_s2.write_all(&make_mode2_request("1003")) {
        panic!("write mode2 s2 viewer: {e:?}");
    }
    let started_s2 = relay.wait_for_line_after(after_s2, |l| {
        field_matches(l, "event", "session_started")
            && field_matches(l, "server_conn_id", &server_s2_conn_id)
            && field_matches(l, "viewer_conn_id", &viewer_s2_conn_id)
    });
    let s2_session_id = some(extract_field(&started_s2, "session_id"), "s2 session_id").to_owned();
    let s2_ref = some(
        extract_field(&started_s2, "relay_id_ref"),
        "s2 relay_id_ref",
    )
    .to_owned();

    assert_ne!(
        s1_session_id, s2_session_id,
        "RLY-R11: session_id must be fresh on reused relay ID"
    );
    assert_eq!(
        s1_ref, s2_ref,
        "relay_id_ref must be identical for same relay ID within process"
    );

    // Close the reused-ID session through real authenticated management HTTP.
    let login_body = format!(r#"{{"account":"alice","password":"{ADMIN_PASSWORD}"}}"#);
    let (status, headers, login) = read_management_reply(relay.management_begin(
        "POST",
        "/api/v1/session",
        &["Content-Type: application/json"],
        &login_body,
    ));
    assert_eq!(status, 200, "administrator login: {login:?}");
    assert_eq!(login.str("account"), Some("alice"));
    let set_cookie = some(
        headers.lines().find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("set-cookie")
                .then_some(value.trim())
        }),
        "login Set-Cookie",
    );
    let cookie = format!(
        "Cookie: {}",
        some(set_cookie.split(';').next(), "cookie pair")
    );
    let csrf = format!(
        "X-CSRF-Token: {}",
        some(login.str("csrf_token"), "login CSRF")
    );
    let pending = relay.management_begin(
        "POST",
        &format!("/api/v1/sessions/{s2_session_id}/terminate"),
        &[&cookie, &csrf],
        "",
    );
    for endpoint in [&mut viewer_s2, &mut server_s2] {
        let mut unexpected = [0; 1];
        assert_eq!(
            endpoint
                .read(&mut unexpected)
                .unwrap_or_else(|error| panic!("administrator closes relay endpoint: {error}")),
            0
        );
        endpoint
            .shutdown(Shutdown::Write)
            .unwrap_or_else(|error| panic!("acknowledge administrator close: {error}"));
    }
    let (status, _, terminated) = read_management_reply(pending);
    assert_eq!(status, 200, "administrator termination: {terminated:?}");
    assert_eq!(terminated.str("result"), Some("terminated"));
    relay.wait_for_line_after(after_s2, |line| {
        field_matches(line, "event", "session_closed")
            && field_matches(line, "session_id", &s2_session_id)
            && field_matches(line, "close_cause", "admin_terminated")
            && field_matches(line, "admin_account", "alice")
    });
    relay.wait_for_line_after(after_s2, |line| {
        field_matches(line, "event", "admin_terminate")
            && field_matches(line, "session_id", &s2_session_id)
            && field_matches(line, "admin_account", "alice")
            && field_matches(line, "result", "terminated")
    });

    // 11. Assert all lines in log file:
    // - exactly one JSON object per line
    // - line count == event count (no blank lines, no split lines, no interleaving)
    // - contract-required fields per event condition
    let objects = parse_log_objects(&relay.dir);

    for (index, obj) in objects.iter().enumerate() {
        let ts = some(obj.str("ts"), "ts");
        assert_timestamp(ts);
        let level = some(obj.str("level"), "level");
        assert!(
            level == "info" || level == "warn" || level == "error",
            "invalid level: {level}"
        );
        let event = some(obj.str("event"), "event");

        if let Some(conn_id) = obj.str("conn_id")
            && event != "connection_accepted"
        {
            // Line order proves the acceptance came first. `ts` is the wall
            // clock at record time; a clock correction may step it back.
            let accepted = some(
                objects[..index].iter().find(|prior| {
                    prior.str("event") == Some("connection_accepted")
                        && prior.str("conn_id") == Some(conn_id)
                }),
                "earlier acceptance of lifecycle conn_id",
            );
            for field in ["listener", "role"] {
                if let Some(value) = obj.str(field) {
                    assert_eq!(accepted.str(field), Some(value), "{event}: {field}");
                }
            }
        }

        match event {
            "connection_accepted" => {
                assert_eq!(level, "info");
                assert!(some(obj.str("conn_id"), "accepted conn_id").starts_with("c"));
                let listener = some(obj.str("listener"), "accepted listener");
                assert!(listener == "tcp_viewer" || listener == "tcp_server");
                let role = some(obj.str("role"), "accepted role");
                assert!(role == "viewer" || role == "server");
                assert_eq!(obj.str("peer_ip"), Some("127.0.0.1"));
            }
            "connection_rejected" => {
                assert_eq!(level, "warn");
                let stage = some(obj.str("stage"), "rejected stage");
                let reason = some(obj.str("reason"), "rejected reason");
                assert!(!reason.is_empty());
                assert!(obj.str("peer_ip").is_some());
                if stage == "accept" {
                    assert!(
                        !obj.has_key("conn_id"),
                        "stage=accept must not have conn_id"
                    );
                } else {
                    assert!(some(obj.str("conn_id"), "rejected conn_id").starts_with("c"));
                }
                if stage == "register" && reason == "duplicate_id" {
                    let r_ref = some(obj.str("relay_id_ref"), "duplicate_id relay_id_ref");
                    assert!(is_16_hex(r_ref));
                }
                if stage == "target_connect" && reason == "target_not_allowed" {
                    assert_eq!(obj.str("target_host"), Some("127.0.0.1"));
                    assert_eq!(obj.num("target_port"), Some(22));
                }
            }
            "connection_aborted" => {
                assert_eq!(level, "info"); // peer_eof is info
                assert!(some(obj.str("conn_id"), "aborted conn_id").starts_with("c"));
                assert!(obj.str("listener").is_some());
                assert_eq!(obj.str("stage"), Some("initial_exchange"));
                assert_eq!(obj.str("reason"), Some("peer_eof"));
                assert_eq!(obj.str("peer_ip"), Some("127.0.0.1"));
                assert!(some(obj.num("elapsed_ms"), "aborted elapsed_ms") >= 0);
            }
            "target_connect_failed" => {
                assert_eq!(level, "warn");
                assert!(some(obj.str("conn_id"), "target_failed conn_id").starts_with("c"));
                assert_eq!(obj.str("listener"), Some("tcp_viewer"));
                assert_eq!(obj.str("viewer_transport"), Some("tcp"));
                let r = some(obj.str("reason"), "target_failed reason");
                assert!(
                    r == "connect_refused"
                        || r == "connect_timeout"
                        || r == "resolve_failed"
                        || r == "connect_error",
                    "invalid reason: {r}"
                );
                assert_eq!(obj.str("peer_ip"), Some("127.0.0.1"));
                assert_eq!(obj.str("target_host"), Some("127.0.0.1"));
                assert_eq!(obj.num("target_port"), Some(unused_port as i64));
                assert!(some(obj.num("elapsed_ms"), "target_failed elapsed_ms") >= 0);
            }
            "waiting_registered" => {
                assert_eq!(level, "info");
                assert!(some(obj.str("conn_id"), "waiting_registered conn_id").starts_with("c"));
                assert_eq!(obj.str("listener"), Some("tcp_server"));
                assert_eq!(obj.str("role"), Some("server"));
                assert_eq!(obj.str("mode"), Some("mode2"));
                assert_eq!(obj.str("peer_ip"), Some("127.0.0.1"));
                let r_ref = some(obj.str("relay_id_ref"), "waiting_registered relay_id_ref");
                assert!(is_16_hex(r_ref));
            }
            "waiting_expired" => {
                assert_eq!(level, "info");
                assert!(some(obj.str("conn_id"), "waiting_expired conn_id").starts_with("c"));
                assert_eq!(obj.str("role"), Some("server"));
                let r_ref = some(obj.str("relay_id_ref"), "waiting_expired relay_id_ref");
                assert!(is_16_hex(r_ref));
                assert!(some(obj.num("waited_ms"), "waited_ms") >= 10000);
            }
            "waiting_removed" => {
                assert_eq!(level, "info"); // peer_eof is info
                assert!(some(obj.str("conn_id"), "waiting_removed conn_id").starts_with("c"));
                assert_eq!(obj.str("role"), Some("server"));
                let r_ref = some(obj.str("relay_id_ref"), "waiting_removed relay_id_ref");
                assert!(is_16_hex(r_ref));
                assert_eq!(obj.str("reason"), Some("peer_eof"));
                assert!(some(obj.num("waited_ms"), "waited_ms") >= 0);
            }
            "session_started" => {
                assert_eq!(level, "info");
                let sid = some(obj.str("session_id"), "started session_id");
                assert!(!sid.is_empty());
                assert_eq!(obj.str("mode"), Some("mode2"));
                assert_eq!(obj.str("viewer_transport"), Some("tcp"));
                assert!(some(obj.str("viewer_conn_id"), "viewer_conn_id").starts_with("c"));
                assert_eq!(obj.str("viewer_peer_ip"), Some("127.0.0.1"));
                assert!(some(obj.str("server_conn_id"), "server_conn_id").starts_with("c"));
                assert_eq!(obj.str("server_peer_ip"), Some("127.0.0.1"));
                let r_ref = some(obj.str("relay_id_ref"), "started relay_id_ref");
                assert!(is_16_hex(r_ref));
                assert!(some(obj.num("waited_ms"), "waited_ms") >= 0);
                for (field, role) in [("viewer_conn_id", "viewer"), ("server_conn_id", "server")] {
                    let conn_id = some(obj.str(field), field);
                    let accepted = some(
                        objects[..index].iter().find(|prior| {
                            prior.str("event") == Some("connection_accepted")
                                && prior.str("conn_id") == Some(conn_id)
                        }),
                        "session endpoint acceptance",
                    );
                    assert_eq!(accepted.str("role"), Some(role));
                }
                let waiter = some(
                    objects[..index].iter().find(|prior| {
                        prior.str("event") == Some("waiting_registered")
                            && prior.str("conn_id") == obj.str("server_conn_id")
                    }),
                    "matched server's waiting registration",
                );
                assert_eq!(waiter.str("relay_id_ref"), obj.str("relay_id_ref"));
            }
            "session_closed" => {
                assert_eq!(level, "info"); // peer_eof is info
                let sid = some(obj.str("session_id"), "closed session_id");
                assert!(!sid.is_empty());
                assert_eq!(obj.str("mode"), Some("mode2"));
                assert_eq!(obj.str("viewer_transport"), Some("tcp"));
                assert_timestamp(some(obj.str("started_at"), "started_at"));
                assert!(some(obj.num("duration_ms"), "duration_ms") >= 0);
                assert!(some(obj.num("bytes_viewer_to_target"), "bytes_v2t") >= 0);
                assert!(some(obj.num("bytes_target_to_viewer"), "bytes_t2v") >= 0);
                let started = some(
                    objects[..index].iter().find(|prior| {
                        prior.str("event") == Some("session_started")
                            && prior.str("session_id") == Some(sid)
                    }),
                    "closed session's start event",
                );
                for field in ["mode", "viewer_transport", "relay_id_ref"] {
                    assert_eq!(obj.str(field), started.str(field), "{sid}: {field}");
                }
                if sid == s2_session_id {
                    assert_eq!(obj.str("close_cause"), Some("admin_terminated"));
                    assert_eq!(obj.str("admin_account"), Some("alice"));
                    assert!(!obj.has_key("side"));
                } else {
                    assert_eq!(sid, s1_session_id);
                    assert_eq!(obj.str("close_cause"), Some("peer_eof"));
                    let side = some(obj.str("side"), "session_closed side");
                    assert!(side == "viewer" || side == "server", "invalid side: {side}");
                    assert!(!obj.has_key("admin_account"));
                }
                let r_ref = some(obj.str("relay_id_ref"), "closed relay_id_ref");
                assert!(is_16_hex(r_ref));
            }
            "admin_login_succeeded" => {
                assert_eq!(level, "info");
                assert_eq!(obj.str("admin_account"), Some("alice"));
                assert_eq!(obj.str("peer_ip"), Some("127.0.0.1"));
            }
            "admin_terminate" => {
                assert_eq!(level, "info");
                assert_eq!(obj.str("admin_account"), Some("alice"));
                assert_eq!(obj.str("peer_ip"), Some("127.0.0.1"));
                assert_eq!(obj.str("session_id"), Some(s2_session_id.as_str()));
                assert_eq!(obj.str("result"), Some("terminated"));
                let closed = some(
                    objects[..index].iter().find(|prior| {
                        prior.str("event") == Some("session_closed")
                            && prior.str("session_id") == obj.str("session_id")
                    }),
                    "cleanup before administrator audit",
                );
                assert_eq!(closed.str("admin_account"), obj.str("admin_account"));
            }
            _ => {}
        }
    }
}

/// spec: operations.md:OPS-012 partial
#[test]
fn operations_log_and_status_endpoints_never_expose_secrets_or_raw_identifiers() {
    let _lock = lock_ports();
    let relay = RunningRelay::start("ops012", "", 10, 10, true);

    let raw_relay_id = "99887766554433221100998877665544";
    let vnc_challenge_marker = "SECRET_VNC_CHALLENGE_KEY_ABCDEF1234567890";
    let vnc_response_marker = "SECRET_VNC_RESPONSE_VAL_0987654321FEDCBA";
    let mode1_payload_marker = "SECRET_MODE1_TARGET_STREAM_DATA_XYZ123";
    let mode1_reply_marker = "SECRET_MODE1_TARGET_REPLY_DATA_UVW456";
    let invalid_req_marker = "SECRET_INVALID_REQ_MARKER_9999_BOGUS";
    let secret_cookie = "SECRET_ADMIN_COOKIE_VALUE_A1B2C3D4";
    let secret_auth = "SECRET_AUTH_BEARER_TOKEN_E5F6G7H8";
    let secret_forwarded_for = "SECRET_X_FORWARDED_FOR_99.88.77.66";

    // 1. Mode II session with long raw relay ID and VNC auth challenge/response patterns
    let after_mode2 = relay.lines().len();
    let (mut srv, _srv_id) = relay.connect_server();
    if let Err(e) = srv.write_all(&make_mode2_request(raw_relay_id)) {
        panic!("write mode2 raw id server: {e:?}");
    }
    let (mut vwr, _vwr_id) = relay.connect_viewer();
    if let Err(e) = vwr.write_all(&make_mode2_request(raw_relay_id)) {
        panic!("write mode2 raw id viewer: {e:?}");
    }
    relay.wait_for_line_after(after_mode2, |l| {
        field_matches(l, "event", "session_started")
    });

    if let Err(e) = vwr.write_all(vnc_challenge_marker.as_bytes()) {
        panic!("write challenge: {e:?}");
    }
    let mut srv_challenge_buf = vec![0u8; vnc_challenge_marker.len()];
    if let Err(e) = srv.read_exact(&mut srv_challenge_buf) {
        panic!("read challenge: {e:?}");
    }
    assert_eq!(&srv_challenge_buf, vnc_challenge_marker.as_bytes());

    if let Err(e) = srv.write_all(vnc_response_marker.as_bytes()) {
        panic!("write response: {e:?}");
    }
    let mut vwr_resp_buf = vec![0u8; vnc_response_marker.len()];
    if let Err(e) = vwr.read_exact(&mut vwr_resp_buf) {
        panic!("read response: {e:?}");
    }
    assert_eq!(&vwr_resp_buf, vnc_response_marker.as_bytes());
    drop(vwr);
    drop(srv);

    // 2. Mode I target request with unique stream markers
    let target_server = TargetServer::bind();
    let target_port = target_server.port;
    let after_mode1 = relay.lines().len();
    let (mut mode1_viewer, _m1_id) = relay.connect_viewer();
    let req = format!("127.0.0.1:{target_port}");
    if let Err(e) = mode1_viewer.write_all(&make_mode1_request(&req)) {
        panic!("write mode1 req: {e:?}");
    }
    let mut target_stream = target_server.accept();
    relay.wait_for_line_after(after_mode1, |l| {
        field_matches(l, "event", "session_started")
    });

    if let Err(e) = mode1_viewer.write_all(mode1_payload_marker.as_bytes()) {
        panic!("write mode1 payload: {e:?}");
    }
    let mut target_buf = vec![0u8; mode1_payload_marker.len()];
    if let Err(e) = target_stream.read_exact(&mut target_buf) {
        panic!("read mode1 payload: {e:?}");
    }
    assert_eq!(&target_buf, mode1_payload_marker.as_bytes());

    if let Err(e) = target_stream.write_all(mode1_reply_marker.as_bytes()) {
        panic!("write mode1 reply: {e:?}");
    }
    let mut viewer_reply_buf = vec![0u8; mode1_reply_marker.len()];
    if let Err(e) = mode1_viewer.read_exact(&mut viewer_reply_buf) {
        panic!("read mode1 reply: {e:?}");
    }
    assert_eq!(&viewer_reply_buf, mode1_reply_marker.as_bytes());
    drop(mode1_viewer);
    drop(target_stream);

    // 3. Invalid initial request containing secret marker
    let after_invalid = relay.lines().len();
    let (mut bad_client, _bad_id) = relay.connect_viewer();
    if let Err(e) = bad_client.write_all(&make_raw_request(invalid_req_marker.as_bytes())) {
        panic!("write invalid marker: {e:?}");
    }
    relay.wait_for_line_after(after_invalid, |l| {
        field_matches(l, "event", "connection_rejected")
            && field_matches(l, "reason", "handshake_invalid")
    });
    drop(bad_client);

    // 4. WebSocket upgrade request containing Cookie, Authorization, X-Forwarded-For secret values
    let ws_port = some(relay.ws_viewer, "ws_viewer port");
    let after_ws = relay.lines().len();
    {
        let mut ws_stream = match TcpStream::connect(("127.0.0.1", ws_port)) {
            Ok(s) => s,
            Err(e) => panic!("connect ws {ws_port}: {e:?}"),
        };
        if let Err(e) = ws_stream.set_read_timeout(Some(LIMIT)) {
            panic!("read timeout: {e:?}");
        }
        if let Err(e) = ws_stream.set_write_timeout(Some(LIMIT)) {
            panic!("write timeout: {e:?}");
        }
        let ws_req = format!(
            "GET /relay HTTP/1.1\r\n\
             Host: 127.0.0.1\r\n\
             Upgrade: websocket\r\n\
             Connection: Upgrade\r\n\
             Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
             Sec-WebSocket-Version: 13\r\n\
             Origin: http://127.0.0.1\r\n\
             Cookie: relay_admin_session={secret_cookie}\r\n\
             Authorization: Basic {secret_auth}\r\n\
             X-Forwarded-For: {secret_forwarded_for}\r\n\
             \r\n"
        );
        if let Err(e) = ws_stream.write_all(ws_req.as_bytes()) {
            panic!("write ws req: {e:?}");
        }
        let mut ws_resp_header = [0u8; 128];
        let n = match ws_stream.read(&mut ws_resp_header) {
            Ok(count) => count,
            Err(e) => panic!("read ws response: {e:?}"),
        };
        assert!(
            &ws_resp_header[..n].starts_with(b"HTTP/1.1 101"),
            "expected 101 switching protocols, got {:?}",
            String::from_utf8_lossy(&ws_resp_header[..n])
        );
        drop(ws_stream);
    }
    relay.wait_for_line_after(after_ws, |l| {
        field_matches(l, "event", "connection_aborted")
            || field_matches(l, "event", "connection_rejected")
    });

    // 5. Query status endpoints (/livez, /readyz)
    let livez_resp = http_get(relay.status, "/livez");
    assert!(
        livez_resp.contains("\"status\":\"alive\""),
        "livez: {livez_resp}"
    );
    let readyz_resp = http_get(relay.status, "/readyz");
    assert!(
        readyz_resp.contains("\"status\":\"ready\""),
        "readyz: {readyz_resp}"
    );

    // 6. Assert non-exposure of all secret tokens and raw relay ID
    let log_content = match std::fs::read_to_string(relay.dir.join("events.jsonl")) {
        Ok(c) => c,
        Err(e) => panic!("read events.jsonl: {e:?}"),
    };

    let forbidden_strings = [
        raw_relay_id,
        vnc_challenge_marker,
        vnc_response_marker,
        mode1_payload_marker,
        mode1_reply_marker,
        invalid_req_marker,
        secret_cookie,
        secret_auth,
        secret_forwarded_for,
    ];

    for forbidden in forbidden_strings {
        assert!(
            !log_content.contains(forbidden),
            "log leaked secret or raw identifier: '{forbidden}'"
        );
        assert!(
            !livez_resp.contains(forbidden),
            "/livez leaked secret or raw identifier: '{forbidden}'"
        );
        assert!(
            !readyz_resp.contains(forbidden),
            "/readyz leaked secret or raw identifier: '{forbidden}'"
        );
    }

    // 7. Verify all relay_id_ref fields in log are 16 lowercase hex characters
    let objects = parse_log_objects(&relay.dir);
    let mut ref_count = 0;
    for obj in &objects {
        if let Some(r_ref) = obj.str("relay_id_ref") {
            ref_count += 1;
            assert!(
                is_16_hex(r_ref),
                "relay_id_ref must be 16 lowercase hex characters: '{r_ref}'"
            );
        }
    }
    assert!(ref_count > 0, "expected at least one relay_id_ref in log");
}

/// spec: relay.md:RLY-027
#[test]
fn idle_active_sessions_survive_longer_than_k_and_transfer_data() {
    let _lock = lock_ports();
    // Minimum keepalive values allowed by config-v2.md:
    // tcp_idle_secs = 10 (range 10..=3600)
    // tcp_interval_secs = 1 (range 1..=300)
    // tcp_probes = 1 (range 1..=20)
    // Effective K = 10 + 1 * 1 = 11 seconds.
    let keepalive_config = r#"
[keepalive]
tcp_idle_secs = 10
tcp_interval_secs = 1
tcp_probes = 1
"#;
    let relay = RunningRelay::start("rly027", keepalive_config, 30, 10, false);

    // 1. Establish Mode I active session
    let target_server = TargetServer::bind();
    let target_port = target_server.port;
    let after_m1_start = relay.lines().len();
    let (mut m1_viewer, _m1_vid) = relay.connect_viewer();
    let m1_req = format!("127.0.0.1:{target_port}");
    if let Err(e) = m1_viewer.write_all(&make_mode1_request(&m1_req)) {
        panic!("write mode1 request: {e:?}");
    }
    let mut m1_target_stream = target_server.accept();
    let m1_started = relay.wait_for_line_after(after_m1_start, |l| {
        field_matches(l, "event", "session_started") && field_matches(l, "mode", "mode1")
    });
    let m1_session_id = some(extract_field(&m1_started, "session_id"), "m1 session_id").to_owned();

    // 2. Establish Mode II active session
    let after_m2_start = relay.lines().len();
    let (mut m2_server, m2_sid) = relay.connect_server();
    if let Err(e) = m2_server.write_all(&make_mode2_request("9007")) {
        panic!("write mode2 server request: {e:?}");
    }
    relay.wait_for_line_after(after_m2_start, |l| {
        field_matches(l, "event", "waiting_registered") && field_matches(l, "conn_id", &m2_sid)
    });
    let (mut m2_viewer, m2_vid) = relay.connect_viewer();
    if let Err(e) = m2_viewer.write_all(&make_mode2_request("9007")) {
        panic!("write mode2 viewer request: {e:?}");
    }
    let m2_started = relay.wait_for_line_after(after_m2_start, |l| {
        field_matches(l, "event", "session_started")
            && field_matches(l, "viewer_conn_id", &m2_vid)
            && field_matches(l, "server_conn_id", &m2_sid)
    });
    let m2_session_id = some(extract_field(&m2_started, "session_id"), "m2 session_id").to_owned();

    // 3. Keep both sessions active and idle without data for 13 seconds (> effective K = 11s)
    let idle_target = Duration::from_millis(13000);
    let idle_start = Instant::now();
    while Instant::now().duration_since(idle_start) < idle_target {
        std::thread::sleep(Duration::from_millis(50));
    }

    // 4. Assert neither session was closed or labeled dead_connection
    let current_lines = relay.lines();
    for line in &current_lines {
        if field_matches(line, "event", "session_closed") {
            let sid = extract_field(line, "session_id");
            assert!(
                sid != Some(&m1_session_id) && sid != Some(&m2_session_id),
                "session closed during idle keepalive period: {line}"
            );
        }
        assert!(
            !line.contains("dead_connection"),
            "unexpected dead_connection event during idle keepalive period: {line}"
        );
    }

    // 5. Transfer exact bytes in both directions on Mode II
    let m2_v2s_payload = b"POST_KEEPALIVE_MODE2_VIEWER_TO_SERVER_BYTE_STREAM_DATA_12345678";
    if let Err(e) = m2_viewer.write_all(m2_v2s_payload) {
        panic!("write m2 v2s: {e:?}");
    }
    let mut m2_srv_buf = vec![0u8; m2_v2s_payload.len()];
    if let Err(e) = m2_server.read_exact(&mut m2_srv_buf) {
        panic!("read m2 v2s: {e:?}");
    }
    assert_eq!(&m2_srv_buf, m2_v2s_payload);

    let m2_s2v_payload = b"POST_KEEPALIVE_MODE2_SERVER_TO_VIEWER_BYTE_STREAM_DATA_87654321";
    if let Err(e) = m2_server.write_all(m2_s2v_payload) {
        panic!("write m2 s2v: {e:?}");
    }
    let mut m2_vwr_buf = vec![0u8; m2_s2v_payload.len()];
    if let Err(e) = m2_viewer.read_exact(&mut m2_vwr_buf) {
        panic!("read m2 s2v: {e:?}");
    }
    assert_eq!(&m2_vwr_buf, m2_s2v_payload);

    // 6. Transfer exact bytes in both directions on Mode I
    let m1_v2t_payload = b"POST_KEEPALIVE_MODE1_VIEWER_TO_TARGET_BYTE_STREAM_DATA_ABCDEFGH";
    if let Err(e) = m1_viewer.write_all(m1_v2t_payload) {
        panic!("write m1 v2t: {e:?}");
    }
    let mut m1_target_buf = vec![0u8; m1_v2t_payload.len()];
    if let Err(e) = m1_target_stream.read_exact(&mut m1_target_buf) {
        panic!("read m1 v2t: {e:?}");
    }
    assert_eq!(&m1_target_buf, m1_v2t_payload);

    let m1_t2v_payload = b"POST_KEEPALIVE_MODE1_TARGET_TO_VIEWER_BYTE_STREAM_DATA_HGFEDCBA";
    if let Err(e) = m1_target_stream.write_all(m1_t2v_payload) {
        panic!("write m1 t2v: {e:?}");
    }
    let mut m1_vwr_buf = vec![0u8; m1_t2v_payload.len()];
    if let Err(e) = m1_viewer.read_exact(&mut m1_vwr_buf) {
        panic!("read m1 t2v: {e:?}");
    }
    assert_eq!(&m1_vwr_buf, m1_t2v_payload);

    // 7. Cleanly close connections
    drop(m1_viewer);
    drop(m1_target_stream);
    drop(m2_viewer);
    drop(m2_server);

    relay.wait_for_line_after(0, |l| {
        field_matches(l, "event", "session_closed")
            && field_matches(l, "session_id", &m1_session_id)
            && field_matches(l, "close_cause", "peer_eof")
    });
    relay.wait_for_line_after(0, |l| {
        field_matches(l, "event", "session_closed")
            && field_matches(l, "session_id", &m2_session_id)
            && field_matches(l, "close_cause", "peer_eof")
    });
}
