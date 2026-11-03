//! Black-box system tests for management query API (TODO 4.4).
//! Covers:
//! - MGT-001 (full): counts match lists and details
//! - MGT-002 (full): inclusion before/after initial exchange, target connect, closing
//! - MGT-003 (full): detail fields, UTC ISO 8601 ms format, integer bytes
//! - MGT-004 (full): empty success vs real query-path 400/404 errors (Q95)
//! - MGT-005 (full): per-direction bytes and partial writes
//! - MGT-006 (full): same stream over TCP, WS, WSS gives identical counters
//! - MGT-007 (partial): reconnect gives new session id and resets counters; restart gives fresh state (clock correction is unit tested)
//! - MGT-013 (partial): query/error body privacy; full HTTP failure coverage lives in adapters/tests/management.rs
//! - MGT-015 (full): forwarded headers ignored and real socket address used
//! - MGT-016 (partial): counts equal list lengths under churn as far as black-box allows
//! - MGT-021 (full): unauthenticated queries rejected on all routes with 401
//! - MGT-023 (partial): EOF and connection error causes in closed-sessions, detail of closed gives 404 (admin-terminate requires 4.5)
//! - MGT-024 (full): R overflow drops oldest in FIFO, restart clears in-memory list
//! - MGT-026 (full on Linux, partial portable): counts and usage match at L limit with pending connections
//! - OPS-033 (partial): query-visible L and Lm usage and mutual isolation under relay saturation
//! - Contract rules: error order, 405 Allow GET, X-Relay-Poll 1 idle preservation, Cache-Control: no-store, session_id grammar

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::version::{TLS12, TLS13};
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use vnc_relay_adapters::accounts_file::Argon2Hasher;
use vnc_relay_app::ports::PasswordHasher;
use vnc_relay_domain::account::{Argon2Params, Password};

const BIN: &str = env!("CARGO_BIN_EXE_vnc-relay");
const READY_LIMIT: Duration = Duration::from_secs(20);
const IO_LIMIT: Duration = Duration::from_secs(10);
static SERIAL: AtomicU64 = AtomicU64::new(0);
static PORTS_LOCK: Mutex<()> = Mutex::new(());

const ALICE: &str = "alice";
const ALICE_PASSWORD: &str = "correct-horse-battery";
const PARAMS: Argon2Params = Argon2Params {
    memory_kib: 19_456,
    iterations: 2,
    parallelism: 1,
};

fn lock_ports() -> MutexGuard<'static, ()> {
    PORTS_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

fn some<T>(option: Option<T>, context: &str) -> T {
    match option {
        Some(value) => value,
        None => panic!("missing {context}"),
    }
}

fn tls_fixture(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("crates/adapters/tests/fixtures/tls")
        .join(name)
        .display()
        .to_string()
        .replace('\\', "/")
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

fn is_valid_session_id(sid: &str) -> bool {
    sid.starts_with("ses_")
        && sid.len() <= 64
        && sid
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

// -----------------------------------------------------------------------------
// Standalone JSON Parser & Value
// -----------------------------------------------------------------------------

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
            Self::Object(entries) => {
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
            Self::Object(entries) => {
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

    fn obj(&self, key: &str) -> Option<&JsonValue> {
        match self {
            Self::Object(entries) => {
                for (k, v) in entries {
                    if k == key {
                        match v {
                            JsonValue::Object(_) => return Some(v),
                            _ => return None,
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn arr(&self, key: &str) -> Option<&[JsonValue]> {
        match self {
            Self::Object(entries) => {
                for (k, v) in entries {
                    if k == key {
                        match v {
                            JsonValue::Array(items) => return Some(items.as_slice()),
                            _ => return None,
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn is_null(&self, key: &str) -> bool {
        match self {
            Self::Object(entries) => {
                for (k, v) in entries {
                    if k == key {
                        return matches!(v, JsonValue::Null);
                    }
                }
                false
            }
            _ => false,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(n) => Some(*n),
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
        while self.pos < self.input.len()
            && (self.input[self.pos] == b' '
                || self.input[self.pos] == b'\t'
                || self.input[self.pos] == b'\r'
                || self.input[self.pos] == b'\n')
        {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        if self.pos < self.input.len() {
            Some(self.input[self.pos])
        } else {
            None
        }
    }

    fn consume(&mut self, expected: u8) {
        self.skip_ws();
        if self.pos >= self.input.len() || self.input[self.pos] != expected {
            panic!(
                "expected '{}', found {:?}",
                expected as char,
                self.input.get(self.pos).map(|&b| b as char)
            );
        }
        self.pos += 1;
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

fn parse_json(s: &str) -> JsonValue {
    let mut parser = JsonParser::new(s);
    let val = parser.parse_value();
    parser.skip_ws();
    val
}

// -----------------------------------------------------------------------------
// HTTP & TLS Client Primitives
// -----------------------------------------------------------------------------

struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

impl Reply {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    fn json(&self) -> JsonValue {
        parse_json(&self.body)
    }
}

fn parse_http_reply(raw: &str) -> Reply {
    let (head, body) = raw
        .split_once("\r\n\r\n")
        .unwrap_or_else(|| panic!("no end of HTTP head: {raw}"));
    let mut lines = head.lines();
    let status_line = lines.next().unwrap_or_else(|| panic!("no status line"));
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status code: {status_line}"));
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_owned()))
        .collect();
    Reply {
        status,
        headers,
        body: body.to_owned(),
    }
}

fn tls_client_config() -> ClientConfig {
    let mut roots = RootCertStore::empty();
    let root_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("crates/adapters/tests/fixtures/tls/root-ca.pem");
    let certs = match CertificateDer::pem_file_iter(&root_path) {
        Ok(iter) => iter,
        Err(e) => panic!("open root-ca.pem: {e:?}"),
    };
    for cert in certs {
        let cert = match cert {
            Ok(c) => c,
            Err(e) => panic!("parse root cert: {e:?}"),
        };
        if let Err(e) = roots.add(cert) {
            panic!("add root cert: {e:?}");
        }
    }
    ClientConfig::builder_with_provider(std::sync::Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_protocol_versions(&[&TLS13, &TLS12])
    .unwrap_or_else(|e| panic!("tls versions: {e:?}"))
    .with_root_certificates(roots)
    .with_no_client_auth()
}

enum Wire {
    Plain(TcpStream),
    Tls(Box<StreamOwned<ClientConnection, TcpStream>>),
}

impl Read for Wire {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(s) => s.read(buf),
            Self::Tls(s) => s.read(buf),
        }
    }
}

impl Write for Wire {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(s) => s.write(buf),
            Self::Tls(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Plain(s) => s.flush(),
            Self::Tls(s) => s.flush(),
        }
    }
}

struct WebSocket {
    wire: Wire,
}

impl WebSocket {
    fn upgrade(mut wire: Wire, path: &str, host: &str, origin: &str) -> Self {
        let req = format!(
            "GET {path} HTTP/1.1\r\n\
             Host: {host}\r\n\
             Upgrade: websocket\r\n\
             Connection: Upgrade\r\n\
             Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
             Sec-WebSocket-Version: 13\r\n\
             Origin: {origin}\r\n\
             \r\n"
        );
        wire.write_all(req.as_bytes())
            .unwrap_or_else(|e| panic!("write ws upgrade: {e:?}"));
        wire.flush()
            .unwrap_or_else(|e| panic!("flush ws upgrade: {e:?}"));

        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            let mut byte = [0u8; 1];
            wire.read_exact(&mut byte)
                .unwrap_or_else(|e| panic!("read ws upgrade response: {e:?}"));
            head.push(byte[0]);
            assert!(head.len() < 16384, "ws upgrade response head bounded");
        }
        assert!(
            head.starts_with(b"HTTP/1.1 101 "),
            "expected 101: {:?}",
            String::from_utf8_lossy(&head)
        );
        Self { wire }
    }

    fn send_binary(&mut self, payload: &[u8]) {
        assert!(payload.len() <= u16::MAX as usize);
        let mut bytes = vec![0x82]; // final binary frame
        if payload.len() < 126 {
            bytes.push(0x80 | u8::try_from(payload.len()).unwrap_or_else(|e| panic!("len: {e:?}")));
        } else {
            bytes.push(0x80 | 126);
            bytes.extend_from_slice(
                &u16::try_from(payload.len())
                    .unwrap_or_else(|e| panic!("len: {e:?}"))
                    .to_be_bytes(),
            );
        }
        let mask = [0x13, 0x57, 0x9b, 0xdf];
        bytes.extend_from_slice(&mask);
        bytes.extend(payload.iter().enumerate().map(|(i, b)| b ^ mask[i % 4]));
        self.wire
            .write_all(&bytes)
            .unwrap_or_else(|e| panic!("send ws frame: {e:?}"));
        self.wire
            .flush()
            .unwrap_or_else(|e| panic!("flush ws frame: {e:?}"));
    }

    fn read_binary(&mut self) -> Vec<u8> {
        loop {
            let mut head = [0u8; 2];
            self.wire
                .read_exact(&mut head)
                .unwrap_or_else(|e| panic!("read ws frame head: {e:?}"));
            let is_final = head[0] & 0x80 != 0;
            let opcode = head[0] & 0x0f;
            let is_masked = head[1] & 0x80 != 0;
            let initial_len = head[1] & 0x7f;
            assert!(!is_masked, "server frame must be unmasked");

            let len = match initial_len {
                126 => {
                    let mut b = [0u8; 2];
                    self.wire
                        .read_exact(&mut b)
                        .unwrap_or_else(|e| panic!("read len16: {e:?}"));
                    usize::from(u16::from_be_bytes(b))
                }
                127 => {
                    let mut b = [0u8; 8];
                    self.wire
                        .read_exact(&mut b)
                        .unwrap_or_else(|e| panic!("read len64: {e:?}"));
                    usize::try_from(u64::from_be_bytes(b))
                        .unwrap_or_else(|e| panic!("len overflow: {e:?}"))
                }
                l => usize::from(l),
            };

            let mut payload = vec![0u8; len];
            self.wire
                .read_exact(&mut payload)
                .unwrap_or_else(|e| panic!("read ws payload: {e:?}"));

            match opcode {
                9 => {
                    // Ping -> reply Pong
                    let mut pong = vec![0x8a];
                    pong.push(
                        0x80 | u8::try_from(payload.len()).unwrap_or_else(|e| panic!("{e:?}")),
                    );
                    let mask = [0x11, 0x22, 0x33, 0x44];
                    pong.extend_from_slice(&mask);
                    pong.extend(payload.iter().enumerate().map(|(i, b)| b ^ mask[i % 4]));
                    self.wire
                        .write_all(&pong)
                        .unwrap_or_else(|e| panic!("send pong: {e:?}"));
                    self.wire
                        .flush()
                        .unwrap_or_else(|e| panic!("flush pong: {e:?}"));
                }
                10 => {} // Pong
                2 => {
                    assert!(is_final, "expected final binary frame");
                    return payload;
                }
                other => panic!("unexpected ws opcode: {other}"),
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Running Relay Server Harness
// -----------------------------------------------------------------------------

struct RunningServer {
    dir: PathBuf,
    child: Option<Child>,
    management_port: u16,
    status_port: u16,
    tcp_viewer_port: u16,
    tcp_server_port: u16,
    ws_viewer_port: u16,
    wss_viewer_port: u16,
}

impl RunningServer {
    fn start(
        name: &str,
        extra_config: &str,
        max_entries: u32,
        relay_limit: u32,
        admin_limit: u32,
    ) -> Self {
        let n = SERIAL.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "vnc-relay-query-{}-{}-{name}",
            std::process::id(),
            n
        ));
        if dir.exists() {
            let _ = fs::remove_dir_all(&dir);
        }
        fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("create dir {}: {e:?}", dir.display()));

        let accounts_text = format!(
            "version = 1\n\n[[account]]\nname = \"{ALICE}\"\npassword_hash = \"{}\"\n",
            {
                let typed = Password::parse(ALICE_PASSWORD.to_owned())
                    .unwrap_or_else(|e| panic!("password: {e:?}"));
                let hashed = Argon2Hasher
                    .hash(&typed, PARAMS)
                    .unwrap_or_else(|e| panic!("hash: {e:?}"));
                hashed.as_str().to_owned()
            }
        );
        fs::write(dir.join("admin-accounts.toml"), accounts_text)
            .unwrap_or_else(|e| panic!("write accounts: {e:?}"));

        let holders: Vec<TcpListener> = (0..6)
            .map(|_| {
                TcpListener::bind("127.0.0.1:0")
                    .unwrap_or_else(|e| panic!("reserve free port: {e:?}"))
            })
            .collect();
        let mut ports = [0u16; 6];
        for (i, h) in holders.iter().enumerate() {
            ports[i] = h
                .local_addr()
                .unwrap_or_else(|e| panic!("local addr: {e:?}"))
                .port();
        }
        drop(holders);

        let [
            mgt_port,
            status_port,
            tcp_vwr_port,
            tcp_srv_port,
            ws_vwr_port,
            wss_vwr_port,
        ] = ports;

        let cert = tls_fixture("server-chain.pem");
        let key = tls_fixture("server-key-p256.pem");
        let waiting_limit = relay_limit.clamp(1, 10);

        let config_text = format!(
            r#"
[admin]
accounts_file = "admin-accounts.toml"

[management]
listen = "127.0.0.1:{mgt_port}"
[status]
listen = "127.0.0.1:{status_port}"

[limits]
relay_connections = {relay_limit}
admin_connections = {admin_limit}
waiting = {waiting_limit}

[recent_closed]
max_entries = {max_entries}

[relay.tcp_viewer]
enabled = true
listen = "127.0.0.1:{tcp_vwr_port}"
allow = ["127.0.0.1"]

[relay.tcp_server]
enabled = true
listen = "127.0.0.1:{tcp_srv_port}"
allow = ["127.0.0.1"]

[relay.ws_viewer]
enabled = true
listen = "127.0.0.1:{ws_vwr_port}"
allow = ["127.0.0.1"]
allowed_origins = ["http://127.0.0.1"]

[relay.wss_viewer]
enabled = true
listen = "127.0.0.1:{wss_vwr_port}"
allow = ["127.0.0.1"]
allowed_origins = ["http://127.0.0.1", "https://127.0.0.1"]
cert_file = "{cert}"
key_file = "{key}"

[mode1]
enabled = true
allow_targets = ["127.0.0.1:1-65535"]

[mode2]
enabled = true

{extra_config}
"#
        );

        let config_path = dir.join("relay.toml");
        fs::write(&config_path, config_text).unwrap_or_else(|e| panic!("write config: {e:?}"));

        let child = Command::new(BIN)
            .arg("serve")
            .arg("--config")
            .arg(&config_path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("spawn {BIN}: {e:?}"));

        let mut server = Self {
            dir,
            child: Some(child),
            management_port: mgt_port,
            status_port,
            tcp_viewer_port: tcp_vwr_port,
            tcp_server_port: tcp_srv_port,
            ws_viewer_port: ws_vwr_port,
            wss_viewer_port: wss_vwr_port,
        };

        server.wait_ready();
        server
    }

    fn wait_ready(&mut self) {
        let deadline = Instant::now() + READY_LIMIT;
        let ports = [
            self.management_port,
            self.status_port,
            self.tcp_viewer_port,
            self.tcp_server_port,
            self.ws_viewer_port,
            self.wss_viewer_port,
        ];
        while !ports.into_iter().all(|p| {
            let addr = SocketAddr::from(([127, 0, 0, 1], p));
            TcpStream::connect_timeout(&addr, Duration::from_millis(150)).is_ok()
        }) {
            if let Some(child) = self.child.as_mut()
                && let Ok(Some(status)) = child.try_wait()
            {
                let mut stderr = String::new();
                if let Some(mut err_pipe) = child.stderr.take() {
                    let _ = err_pipe.read_to_string(&mut stderr);
                }
                panic!("server exited early before ready: {status}, stderr:\n{stderr}");
            }
            assert!(Instant::now() < deadline, "server not ready in time");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn login(&self) -> (String, String) {
        let body = format!("{{\"account\":\"{ALICE}\",\"password\":\"{ALICE_PASSWORD}\"}}");
        let reply = self.send_http(
            "POST",
            "/api/v1/session",
            &[("Content-Type", "application/json")],
            body.as_bytes(),
        );
        assert_eq!(reply.status, 200, "login failed: {}", reply.body);
        let set_cookie = some(reply.header("set-cookie"), "Set-Cookie");
        let cookie_pair = set_cookie
            .split(';')
            .next()
            .unwrap_or(set_cookie)
            .to_owned();
        let json = reply.json();
        let csrf_token = some(json.str("csrf_token"), "csrf_token").to_owned();
        (cookie_pair, csrf_token)
    }

    fn send_http(&self, method: &str, path: &str, headers: &[(&str, &str)], body: &[u8]) -> Reply {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.management_port));
        let mut stream = TcpStream::connect_timeout(&addr, IO_LIMIT)
            .unwrap_or_else(|e| panic!("connect management {addr}: {e:?}"));
        stream
            .set_read_timeout(Some(IO_LIMIT))
            .unwrap_or_else(|e| panic!("read timeout: {e:?}"));
        stream
            .set_write_timeout(Some(IO_LIMIT))
            .unwrap_or_else(|e| panic!("write timeout: {e:?}"));

        let mut req = format!(
            "{method} {path} HTTP/1.1\r\n\
             Host: 127.0.0.1\r\n\
             Connection: close\r\n\
             Content-Length: {}\r\n",
            body.len()
        );
        for (k, v) in headers {
            req.push_str(&format!("{k}: {v}\r\n"));
        }
        req.push_str("\r\n");

        stream
            .write_all(req.as_bytes())
            .unwrap_or_else(|e| panic!("write http request: {e:?}"));
        if !body.is_empty() {
            stream
                .write_all(body)
                .unwrap_or_else(|e| panic!("write http body: {e:?}"));
        }
        stream
            .flush()
            .unwrap_or_else(|e| panic!("flush request: {e:?}"));

        let mut resp = Vec::new();
        stream
            .read_to_end(&mut resp)
            .unwrap_or_else(|e| panic!("read http response: {e:?}"));
        parse_http_reply(&String::from_utf8_lossy(&resp))
    }

    fn query(&self, cookie: &str, path: &str) -> Reply {
        self.send_http("GET", path, &[("Cookie", cookie)], b"")
    }

    fn query_poll(&self, cookie: &str, path: &str) -> Reply {
        self.send_http(
            "GET",
            path,
            &[("Cookie", cookie), ("X-Relay-Poll", "1")],
            b"",
        )
    }
    fn poll_json_until<F>(
        &self,
        cookie: &str,
        path: &str,
        timeout: Duration,
        mut predicate: F,
    ) -> JsonValue
    where
        F: FnMut(&JsonValue) -> bool,
    {
        let deadline = Instant::now() + timeout;
        let mut last_val = None;
        while Instant::now() < deadline {
            let res = self.query(cookie, path);
            if res.status == 200 {
                let json = res.json();
                if predicate(&json) {
                    return json;
                }
                last_val = Some(json);
            }
            // Bound query frequency while waiting for the requested state.
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("poll_json_until({path}) timed out after {timeout:?}; last response: {last_val:?}");
    }

    fn poll_summary_until<F>(&self, cookie: &str, timeout: Duration, predicate: F) -> JsonValue
    where
        F: FnMut(&JsonValue) -> bool,
    {
        self.poll_json_until(cookie, "/api/v1/summary", timeout, predicate)
    }

    fn connect_tcp_viewer(&self) -> TcpStream {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.tcp_viewer_port));
        let mut s = TcpStream::connect_timeout(&addr, IO_LIMIT)
            .unwrap_or_else(|e| panic!("connect tcp viewer: {e:?}"));
        s.set_read_timeout(Some(IO_LIMIT))
            .unwrap_or_else(|e| panic!("set read timeout: {e:?}"));
        s.set_write_timeout(Some(IO_LIMIT))
            .unwrap_or_else(|e| panic!("set write timeout: {e:?}"));
        let mut greeting = [0u8; 12];
        s.read_exact(&mut greeting)
            .unwrap_or_else(|e| panic!("read viewer greeting: {e:?}"));
        assert_eq!(&greeting, b"RFB 000.000\n");
        s
    }

    fn connect_tcp_server(&self) -> TcpStream {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.tcp_server_port));
        let s = TcpStream::connect_timeout(&addr, IO_LIMIT)
            .unwrap_or_else(|e| panic!("connect tcp server: {e:?}"));
        s.set_read_timeout(Some(IO_LIMIT))
            .unwrap_or_else(|e| panic!("set read timeout: {e:?}"));
        s.set_write_timeout(Some(IO_LIMIT))
            .unwrap_or_else(|e| panic!("set write timeout: {e:?}"));
        s
    }

    fn connect_ws_viewer(&self) -> WebSocket {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.ws_viewer_port));
        let tcp = TcpStream::connect_timeout(&addr, IO_LIMIT)
            .unwrap_or_else(|e| panic!("connect ws viewer: {e:?}"));
        tcp.set_read_timeout(Some(IO_LIMIT))
            .unwrap_or_else(|e| panic!("set read timeout: {e:?}"));
        tcp.set_write_timeout(Some(IO_LIMIT))
            .unwrap_or_else(|e| panic!("set write timeout: {e:?}"));
        let mut ws =
            WebSocket::upgrade(Wire::Plain(tcp), "/relay", "127.0.0.1", "http://127.0.0.1");
        let greeting = ws.read_binary();
        assert_eq!(&greeting, b"RFB 000.000\n");
        ws
    }

    fn connect_wss_viewer(&self) -> WebSocket {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.wss_viewer_port));
        let tcp = TcpStream::connect_timeout(&addr, IO_LIMIT)
            .unwrap_or_else(|e| panic!("connect wss viewer: {e:?}"));
        tcp.set_read_timeout(Some(IO_LIMIT))
            .unwrap_or_else(|e| panic!("set read timeout: {e:?}"));
        tcp.set_write_timeout(Some(IO_LIMIT))
            .unwrap_or_else(|e| panic!("set write timeout: {e:?}"));

        let config = std::sync::Arc::new(tls_client_config());
        let name = ServerName::try_from("localhost")
            .unwrap_or_else(|e| panic!("server name localhost: {e:?}"));
        let conn =
            ClientConnection::new(config, name).unwrap_or_else(|e| panic!("client conn: {e:?}"));
        let tls = StreamOwned::new(conn, tcp);

        let mut ws = WebSocket::upgrade(
            Wire::Tls(Box::new(tls)),
            "/relay",
            "localhost",
            "https://127.0.0.1",
        );
        let greeting = ws.read_binary();
        assert_eq!(&greeting, b"RFB 000.000\n");
        ws
    }

    fn stop(mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for RunningServer {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn make_mode2_request(id: &str) -> [u8; 250] {
    let text = format!("ID:{id}");
    assert!(text.len() <= 250);
    let mut b = [0u8; 250];
    b[..text.len()].copy_from_slice(text.as_bytes());
    b
}

fn make_mode1_request(target: &str) -> [u8; 250] {
    assert!(target.len() <= 250);
    let mut b = [0u8; 250];
    b[..target.len()].copy_from_slice(target.as_bytes());
    b
}

// -----------------------------------------------------------------------------
// Linux Backlog Fixture (Pending Connect for MGT-026)
// -----------------------------------------------------------------------------

#[cfg(target_os = "linux")]
struct Backlog {
    listener: TcpListener,
    _fillers: Vec<TcpStream>,
}

#[cfg(target_os = "linux")]
impl Backlog {
    fn saturated() -> Self {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap_or_else(|e| panic!("runtime: {e:?}"));
        let listener = runtime.block_on(async {
            let socket =
                tokio::net::TcpSocket::new_v4().unwrap_or_else(|e| panic!("socket: {e:?}"));
            socket
                .bind(SocketAddr::from(([127, 0, 0, 1], 0)))
                .unwrap_or_else(|e| panic!("bind: {e:?}"));
            socket
                .listen(1)
                .unwrap_or_else(|e| panic!("listen: {e:?}"))
                .into_std()
                .unwrap_or_else(|e| panic!("into_std: {e:?}"))
        });
        let addr = listener
            .local_addr()
            .unwrap_or_else(|e| panic!("local_addr: {e:?}"));

        let mut fillers = Vec::new();
        for _ in 0..64 {
            match TcpStream::connect_timeout(&addr, Duration::from_millis(100)) {
                Ok(s) => fillers.push(s),
                Err(err)
                    if err.kind() == std::io::ErrorKind::TimedOut
                        || err.kind() == std::io::ErrorKind::WouldBlock =>
                {
                    assert!(!fillers.is_empty(), "backlog has filled streams");
                    return Self {
                        listener,
                        _fillers: fillers,
                    };
                }
                Err(err) => panic!("unexpected backlog connect err: {err:?}"),
            }
        }
        panic!("OS listen backlog did not saturate");
    }

    fn address(&self) -> SocketAddr {
        self.listener
            .local_addr()
            .unwrap_or_else(|e| panic!("address: {e:?}"))
    }
}

// =============================================================================
// Scenario Acceptance Tests
// =============================================================================

/// spec: management.md:MGT-001
#[test]
fn management_query_counts_match_lists_and_details() {
    let _lock = lock_ports();
    let server = RunningServer::start("mgt001", "", 5, 10, 5);
    let (cookie, _) = server.login();

    // Initially: empty waiting and sessions
    let summary = server.query(&cookie, "/api/v1/summary").json();
    assert_eq!(summary.str("as_of").map(|s| s.len()), Some(24));
    assert_eq!(
        summary.obj("counts").and_then(|c| c.num("waiting")),
        Some(0)
    );
    assert_eq!(summary.obj("counts").and_then(|c| c.num("active")), Some(0));

    let waiting = server.query(&cookie, "/api/v1/waiting").json();
    assert_eq!(waiting.arr("items").map(|a| a.len()), Some(0));

    let sessions = server.query(&cookie, "/api/v1/sessions").json();
    assert_eq!(sessions.arr("items").map(|a| a.len()), Some(0));

    // Connect Server in Mode II (ID 1001), waiting for viewer
    let mut srv = server.connect_tcp_server();
    srv.write_all(&make_mode2_request("1001"))
        .unwrap_or_else(|e| panic!("write srv req: {e:?}"));
    srv.flush()
        .unwrap_or_else(|e| panic!("flush srv req: {e:?}"));

    let summary = server.poll_summary_until(&cookie, IO_LIMIT, |s| {
        s.obj("counts").and_then(|c| c.num("waiting")) == Some(1)
    });
    assert_eq!(summary.obj("counts").and_then(|c| c.num("active")), Some(0));

    let waiting = server.query(&cookie, "/api/v1/waiting").json();
    let items = some(waiting.arr("items"), "waiting items");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].str("role"), Some("server"));
    assert_eq!(items[0].str("relay_id"), Some("1001"));
    assert_timestamp(some(items[0].str("connected_at"), "connected_at"));
    assert!(some(items[0].num("waited_ms"), "waited_ms") >= 0);

    // Connect Viewer in Mode II (ID 1001) -> pair matches and activates
    let mut vwr = server.connect_tcp_viewer();
    vwr.write_all(&make_mode2_request("1001"))
        .unwrap_or_else(|e| panic!("write vwr req: {e:?}"));
    vwr.flush()
        .unwrap_or_else(|e| panic!("flush vwr req: {e:?}"));

    let summary = server.poll_summary_until(&cookie, IO_LIMIT, |s| {
        s.obj("counts").and_then(|c| c.num("active")) == Some(1)
    });
    assert_eq!(
        summary.obj("counts").and_then(|c| c.num("waiting")),
        Some(0)
    );

    let waiting = server.query(&cookie, "/api/v1/waiting").json();
    assert_eq!(waiting.arr("items").map(|a| a.len()), Some(0));

    let sessions = server.query(&cookie, "/api/v1/sessions").json();
    let session_items = some(sessions.arr("items"), "sessions items");
    assert_eq!(session_items.len(), 1);
    let sid = some(session_items[0].str("session_id"), "session_id");
    assert!(is_valid_session_id(sid));
    assert_eq!(session_items[0].str("state"), Some("active"));
    assert_eq!(session_items[0].str("mode"), Some("mode2"));
    assert_eq!(session_items[0].str("viewer_transport"), Some("tcp"));

    // Detail query by session_id matches list item
    let detail = server
        .query(&cookie, &format!("/api/v1/sessions/{sid}"))
        .json();
    let session_obj = some(detail.obj("session"), "session detail");
    assert_eq!(session_obj.str("session_id"), Some(sid));
    assert_eq!(session_obj.str("state"), Some("active"));
    assert_eq!(session_obj.str("mode"), Some("mode2"));
    assert_eq!(session_obj.str("viewer_transport"), Some("tcp"));
    assert_timestamp(some(session_obj.str("started_at"), "started_at"));
    assert_eq!(session_obj.num("bytes_viewer_to_target"), Some(0));
    assert_eq!(session_obj.num("bytes_target_to_viewer"), Some(0));
    assert_eq!(
        session_obj.obj("viewer_address").and_then(|a| a.str("ip")),
        Some("127.0.0.1")
    );
    assert_eq!(
        session_obj.obj("target_address").and_then(|a| a.str("ip")),
        Some("127.0.0.1")
    );
}

/// spec: management.md:MGT-002
#[test]
fn inclusion_before_and_after_initial_exchange_mode1_connect_and_closing() {
    let _lock = lock_ports();
    let server = RunningServer::start("mgt002", "[timeouts]\nclose_secs = 5", 5, 10, 5);
    let (cookie, _) = server.login();

    // 1. Raw socket connected, sending partial initial exchange (5 bytes)
    let addr = SocketAddr::from(([127, 0, 0, 1], server.tcp_viewer_port));
    let mut partial_raw =
        TcpStream::connect(addr).unwrap_or_else(|e| panic!("connect raw viewer: {e:?}"));
    let mut greeting = [0u8; 12];
    partial_raw
        .read_exact(&mut greeting)
        .unwrap_or_else(|e| panic!("read raw greeting: {e:?}"));
    partial_raw
        .write_all(b"ID:10")
        .unwrap_or_else(|e| panic!("write partial: {e:?}"));
    partial_raw
        .flush()
        .unwrap_or_else(|e| panic!("flush partial: {e:?}"));

    let summary = server.poll_summary_until(&cookie, IO_LIMIT, |s| {
        s.obj("counts").and_then(|c| c.num("handshaking")) == Some(1)
    });
    assert_eq!(
        summary.obj("counts").and_then(|c| c.num("waiting")),
        Some(0)
    );
    assert_eq!(summary.obj("counts").and_then(|c| c.num("active")), Some(0));
    assert_eq!(
        server
            .query(&cookie, "/api/v1/sessions")
            .json()
            .arr("items")
            .map(|items| items.len()),
        Some(0),
        "initial exchange must not create an active session"
    );
    // 2. Mode I target connect: target listener accepts
    let target_listener =
        TcpListener::bind("127.0.0.1:0").unwrap_or_else(|e| panic!("bind target: {e:?}"));
    let target_port = target_listener
        .local_addr()
        .unwrap_or_else(|e| panic!("{e:?}"))
        .port();

    let mut m1_vwr = server.connect_tcp_viewer();
    m1_vwr
        .write_all(&make_mode1_request(&format!("127.0.0.1:{target_port}")))
        .unwrap_or_else(|e| panic!("write m1 req: {e:?}"));
    m1_vwr
        .flush()
        .unwrap_or_else(|e| panic!("flush m1 req: {e:?}"));

    let (target_stream, _) = target_listener
        .accept()
        .unwrap_or_else(|e| panic!("accept target: {e:?}"));

    let summary = server.poll_summary_until(&cookie, IO_LIMIT, |s| {
        s.obj("counts").and_then(|c| c.num("active")) == Some(1)
    });
    assert_eq!(
        summary.obj("counts").and_then(|c| c.num("waiting")),
        Some(0)
    );
    let sessions = server.query(&cookie, "/api/v1/sessions").json();
    let items = some(sessions.arr("items"), "m1 sessions items");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].str("mode"), Some("mode1"));
    let m1_sid = some(items[0].str("session_id"), "m1 session_id").to_owned();
    assert_eq!(
        summary
            .obj("counts")
            .and_then(|counts| counts.num("active")),
        Some(items.len() as i64)
    );

    // One EOF starts E, but the viewer keeps its write half open until explicitly closed.
    target_stream
        .shutdown(std::net::Shutdown::Write)
        .unwrap_or_else(|error| panic!("half-close target: {error:?}"));
    let detail_path = format!("/api/v1/sessions/{m1_sid}");
    let closing_detail = server.poll_json_until(&cookie, &detail_path, IO_LIMIT, |detail| {
        detail
            .obj("session")
            .and_then(|session| session.str("state"))
            == Some("closing")
    });
    assert_eq!(
        closing_detail
            .obj("session")
            .and_then(|session| session.str("session_id")),
        Some(m1_sid.as_str())
    );
    let closing_sessions = server.poll_json_until(&cookie, "/api/v1/sessions", IO_LIMIT, |list| {
        list.arr("items").is_some_and(|items| {
            items.len() == 1
                && items[0].str("session_id") == Some(m1_sid.as_str())
                && items[0].str("state") == Some("closing")
        })
    });
    let closing_items = some(closing_sessions.arr("items"), "closing sessions");
    // Summary and list are separate API responses; held sockets keep the state stable.
    let summary = server.poll_summary_until(&cookie, IO_LIMIT, |summary| {
        summary
            .obj("counts")
            .and_then(|counts| counts.num("active"))
            == Some(1)
    });
    assert_eq!(
        summary
            .obj("counts")
            .and_then(|counts| counts.num("active")),
        Some(closing_items.len() as i64),
        "closing session remains counted until cleanup completes"
    );
    assert_eq!(
        summary
            .obj("counts")
            .and_then(|counts| counts.num("waiting")),
        Some(0)
    );
    assert_eq!(
        server
            .query(&cookie, &detail_path)
            .json()
            .obj("session")
            .and_then(|session| session.str("state")),
        Some("closing"),
        "closing window must span the list, summary, and detail observations"
    );

    m1_vwr
        .shutdown(std::net::Shutdown::Write)
        .unwrap_or_else(|error| panic!("finish viewer half-close: {error:?}"));
    drop(m1_vwr);
    drop(target_stream);
    let deadline = Instant::now() + IO_LIMIT;
    loop {
        let summary = server.query(&cookie, "/api/v1/summary").json();
        let sessions = server.query(&cookie, "/api/v1/sessions").json();
        let detail = server.query(&cookie, &detail_path);
        let closed = server.query(&cookie, "/api/v1/closed-sessions").json();
        if summary
            .obj("counts")
            .and_then(|counts| counts.num("active"))
            == Some(0)
            && sessions.arr("items").is_some_and(|items| items.is_empty())
            && detail.status == 404
            && closed.arr("items").is_some_and(|items| {
                items
                    .iter()
                    .any(|session| session.str("session_id") == Some(m1_sid.as_str()))
            })
        {
            assert_eq!(
                detail
                    .json()
                    .obj("error")
                    .and_then(|error| error.str("code")),
                Some("not_found")
            );
            break;
        }
        assert!(
            Instant::now() < deadline,
            "session {m1_sid} did not finish cleanup: summary={summary:?}, sessions={sessions:?}, detail_status={}, closed={closed:?}",
            detail.status
        );
        // Bound query frequency while waiting for cleanup across the query endpoints.
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// spec: management.md:MGT-003
#[test]
fn session_detail_fields_timestamp_and_integer_bytes() {
    let _lock = lock_ports();
    let server = RunningServer::start("mgt003", "", 5, 10, 5);
    let (cookie, _) = server.login();

    let mut srv = server.connect_tcp_server();
    srv.write_all(&make_mode2_request("3001"))
        .unwrap_or_else(|e| panic!("write srv: {e:?}"));
    srv.flush().unwrap_or_else(|e| panic!("flush: {e:?}"));

    let mut vwr = server.connect_tcp_viewer();
    vwr.write_all(&make_mode2_request("3001"))
        .unwrap_or_else(|e| panic!("write vwr: {e:?}"));
    vwr.flush().unwrap_or_else(|e| panic!("flush: {e:?}"));

    let sessions = server.poll_json_until(&cookie, "/api/v1/sessions", IO_LIMIT, |s| {
        s.arr("items").and_then(|items| items.first()).is_some()
    });
    let items = some(sessions.arr("items"), "items");
    let sid = some(items[0].str("session_id"), "sid").to_owned();
    // Transfer payload
    let v2t_data = b"VIEWER_TO_TARGET_PAYLOAD_50_BYTES_1234567890123456";
    vwr.write_all(v2t_data)
        .unwrap_or_else(|e| panic!("write: {e:?}"));
    vwr.flush().unwrap_or_else(|e| panic!("flush: {e:?}"));
    let mut recv_target = vec![0u8; v2t_data.len()];
    srv.read_exact(&mut recv_target)
        .unwrap_or_else(|e| panic!("read target: {e:?}"));

    let t2v_data = b"TARGET_TO_VIEWER_PAYLOAD_30_BYTES!";
    srv.write_all(t2v_data)
        .unwrap_or_else(|e| panic!("write: {e:?}"));
    srv.flush().unwrap_or_else(|e| panic!("flush: {e:?}"));
    let mut recv_vwr = vec![0u8; t2v_data.len()];
    vwr.read_exact(&mut recv_vwr)
        .unwrap_or_else(|e| panic!("read vwr: {e:?}"));

    let detail =
        server.poll_json_until(&cookie, &format!("/api/v1/sessions/{sid}"), IO_LIMIT, |d| {
            d.obj("session")
                .and_then(|s| s.num("bytes_viewer_to_target"))
                == Some(v2t_data.len() as i64)
                && d.obj("session")
                    .and_then(|s| s.num("bytes_target_to_viewer"))
                    == Some(t2v_data.len() as i64)
        });
    let s = some(detail.obj("session"), "session detail");
    assert_eq!(s.str("session_id"), Some(sid.as_str()));
    assert_eq!(s.str("state"), Some("active"));
    assert_eq!(s.str("mode"), Some("mode2"));
    assert_eq!(s.str("viewer_transport"), Some("tcp"));
    assert_timestamp(some(s.str("started_at"), "started_at"));
    assert_eq!(s.num("bytes_viewer_to_target"), Some(v2t_data.len() as i64));
    assert_eq!(s.num("bytes_target_to_viewer"), Some(t2v_data.len() as i64));

    // Bad session_id formats -> 400 invalid_request
    for bad in [
        "bad!id",
        "toolong________________________________________________________65",
        "",
    ] {
        let path = format!("/api/v1/sessions/{bad}");
        let rep = server.query(&cookie, &path);
        assert_eq!(rep.status, 400, "bad format {bad} must be 400");
        let err = rep.json();
        assert_eq!(
            err.obj("error").and_then(|e| e.str("code")),
            Some("invalid_request")
        );
    }

    // Well-formed unknown session_id -> 404 not_found
    let rep404 = server.query(&cookie, "/api/v1/sessions/ses_00000000000000000000000000");
    assert_eq!(rep404.status, 404);
    assert_eq!(
        rep404.json().obj("error").and_then(|e| e.str("code")),
        Some("not_found")
    );
}

/// spec: management.md:MGT-004
#[test]
fn empty_success_vs_failure_distinction() {
    let _lock = lock_ports();
    let server = RunningServer::start("mgt004", "", 5, 10, 5);
    let (cookie, csrf) = server.login();

    // Empty success responses: 200 with empty items, never 404 or 500
    let w_rep = server.query(&cookie, "/api/v1/waiting");
    assert_eq!(w_rep.status, 200);
    assert_eq!(w_rep.json().arr("items").map(|a| a.len()), Some(0));

    let s_rep = server.query(&cookie, "/api/v1/sessions");
    assert_eq!(s_rep.status, 200);
    assert_eq!(s_rep.json().arr("items").map(|a| a.len()), Some(0));

    let c_rep = server.query(&cookie, "/api/v1/closed-sessions");
    assert_eq!(c_rep.status, 200);
    assert_eq!(c_rep.json().arr("items").map(|a| a.len()), Some(0));
    assert_eq!(c_rep.json().num("max_entries"), Some(5));

    // Failure cases: distinguished from empty success
    let rep_404 = server.query(&cookie, "/api/v1/not_a_real_path");
    assert_eq!(rep_404.status, 404);
    assert_eq!(
        rep_404.json().obj("error").and_then(|e| e.str("code")),
        Some("not_found")
    );

    let rep_405 = server.send_http(
        "POST",
        "/api/v1/summary",
        &[("Cookie", &cookie), ("X-CSRF-Token", &csrf)],
        b"",
    );
    assert_eq!(rep_405.status, 405);
    assert_eq!(rep_405.header("allow"), Some("GET"));
    assert_eq!(
        rep_405.json().obj("error").and_then(|e| e.str("code")),
        Some("invalid_request")
    );

    let rep_400 = server.send_http(
        "GET",
        "/api/v1/summary",
        &[("Cookie", &cookie), ("X-Relay-Poll", "true")],
        b"",
    );
    assert_eq!(rep_400.status, 400);
    assert_eq!(
        rep_400.json().obj("error").and_then(|e| e.str("code")),
        Some("invalid_request")
    );

    let rep_415 = server.send_http(
        "GET",
        "/api/v1/summary",
        &[("Cookie", &cookie), ("Content-Type", "text/plain")],
        b"non-empty plain text body",
    );
    assert_eq!(rep_415.status, 415);
    assert_eq!(
        rep_415.json().obj("error").and_then(|e| e.str("code")),
        Some("invalid_request")
    );
}

// Partial socket writes and chunk fragmentation are not exercised.
/// spec: management.md:MGT-005 partial
#[test]
fn directional_bytes_and_partial_writes() {
    let _lock = lock_ports();
    let server = RunningServer::start("mgt005", "", 5, 10, 5);
    let (cookie, _) = server.login();

    let mut srv = server.connect_tcp_server();
    srv.write_all(&make_mode2_request("5001"))
        .unwrap_or_else(|e| panic!("write srv: {e:?}"));
    srv.flush().unwrap_or_else(|e| panic!("flush: {e:?}"));

    let mut vwr = server.connect_tcp_viewer();
    vwr.write_all(&make_mode2_request("5001"))
        .unwrap_or_else(|e| panic!("write vwr: {e:?}"));
    vwr.flush().unwrap_or_else(|e| panic!("flush: {e:?}"));

    let sessions = server.poll_json_until(&cookie, "/api/v1/sessions", IO_LIMIT, |s| {
        s.arr("items").and_then(|items| items.first()).is_some()
    });
    let sid = some(
        some(sessions.arr("items"), "items")[0].str("session_id"),
        "sid",
    )
    .to_owned();

    // First write: 100 bytes viewer -> server
    let chunk1 = vec![0x41u8; 100];
    vwr.write_all(&chunk1)
        .unwrap_or_else(|e| panic!("write: {e:?}"));
    vwr.flush().unwrap_or_else(|e| panic!("flush: {e:?}"));
    let mut recv1 = vec![0u8; 100];
    srv.read_exact(&mut recv1)
        .unwrap_or_else(|e| panic!("read: {e:?}"));

    let detail1 =
        server.poll_json_until(&cookie, &format!("/api/v1/sessions/{sid}"), IO_LIMIT, |d| {
            d.obj("session")
                .and_then(|s| s.num("bytes_viewer_to_target"))
                == Some(100)
                && d.obj("session")
                    .and_then(|s| s.num("bytes_target_to_viewer"))
                    == Some(0)
        });
    let s1 = some(detail1.obj("session"), "session 1");
    assert_eq!(s1.num("bytes_viewer_to_target"), Some(100));
    assert_eq!(s1.num("bytes_target_to_viewer"), Some(0));
    // Second write: 250 bytes server -> viewer
    let chunk2 = vec![0x42u8; 250];
    srv.write_all(&chunk2)
        .unwrap_or_else(|e| panic!("write: {e:?}"));
    srv.flush().unwrap_or_else(|e| panic!("flush: {e:?}"));
    let mut recv2 = vec![0u8; 250];
    vwr.read_exact(&mut recv2)
        .unwrap_or_else(|e| panic!("read: {e:?}"));

    let detail2 =
        server.poll_json_until(&cookie, &format!("/api/v1/sessions/{sid}"), IO_LIMIT, |d| {
            d.obj("session")
                .and_then(|s| s.num("bytes_viewer_to_target"))
                == Some(100)
                && d.obj("session")
                    .and_then(|s| s.num("bytes_target_to_viewer"))
                    == Some(250)
        });
    let s2 = some(detail2.obj("session"), "session 2");
    assert_eq!(s2.num("bytes_viewer_to_target"), Some(100));
    assert_eq!(s2.num("bytes_target_to_viewer"), Some(250));
    // Additional write viewer -> server: 50 bytes (accumulates monotonically to 150)
    let chunk3 = vec![0x43u8; 50];
    vwr.write_all(&chunk3)
        .unwrap_or_else(|e| panic!("write: {e:?}"));
    vwr.flush().unwrap_or_else(|e| panic!("flush: {e:?}"));
    let mut recv3 = vec![0u8; 50];
    srv.read_exact(&mut recv3)
        .unwrap_or_else(|e| panic!("read: {e:?}"));

    let detail3 =
        server.poll_json_until(&cookie, &format!("/api/v1/sessions/{sid}"), IO_LIMIT, |d| {
            d.obj("session")
                .and_then(|s| s.num("bytes_viewer_to_target"))
                == Some(150)
                && d.obj("session")
                    .and_then(|s| s.num("bytes_target_to_viewer"))
                    == Some(250)
        });
    let s3 = some(detail3.obj("session"), "session 3");
    assert_eq!(s3.num("bytes_viewer_to_target"), Some(150));
    assert_eq!(s3.num("bytes_target_to_viewer"), Some(250));
}

/// spec: management.md:MGT-006
#[test]
fn same_stream_over_tcp_ws_and_wss_gives_same_counters() {
    let _lock = lock_ports();
    let server = RunningServer::start("mgt006", "", 5, 10, 5);
    let (cookie, _) = server.login();

    let v2t_payload = vec![0x55u8; 120];
    let t2v_payload = vec![0xaau8; 240];

    // 1. TCP Viewer + TCP Server
    {
        let mut srv = server.connect_tcp_server();
        srv.write_all(&make_mode2_request("6001"))
            .unwrap_or_else(|e| panic!("{e:?}"));
        let mut vwr = server.connect_tcp_viewer();
        vwr.write_all(&make_mode2_request("6001"))
            .unwrap_or_else(|e| panic!("{e:?}"));

        vwr.write_all(&v2t_payload)
            .unwrap_or_else(|e| panic!("{e:?}"));
        let mut buf = vec![0u8; 120];
        srv.read_exact(&mut buf).unwrap_or_else(|e| panic!("{e:?}"));

        srv.write_all(&t2v_payload)
            .unwrap_or_else(|e| panic!("{e:?}"));
        let mut buf = vec![0u8; 240];
        vwr.read_exact(&mut buf).unwrap_or_else(|e| panic!("{e:?}"));

        let list = server.poll_json_until(&cookie, "/api/v1/sessions", IO_LIMIT, |l| {
            l.arr("items")
                .and_then(|items| {
                    items.iter().find(|i| {
                        i.str("viewer_transport") == Some("tcp")
                            && i.num("bytes_viewer_to_target") == Some(120)
                            && i.num("bytes_target_to_viewer") == Some(240)
                    })
                })
                .is_some()
        });
        let items = some(list.arr("items"), "items");
        let item = some(
            items
                .iter()
                .find(|i| i.str("viewer_transport") == Some("tcp")),
            "tcp session",
        );
        assert_eq!(item.num("bytes_viewer_to_target"), Some(120));
        assert_eq!(item.num("bytes_target_to_viewer"), Some(240));
    }

    // 2. WS Viewer + TCP Server
    {
        let mut srv = server.connect_tcp_server();
        srv.write_all(&make_mode2_request("6002"))
            .unwrap_or_else(|e| panic!("{e:?}"));
        let mut ws_vwr = server.connect_ws_viewer();
        ws_vwr.send_binary(&make_mode2_request("6002"));

        ws_vwr.send_binary(&v2t_payload);
        let mut buf = vec![0u8; 120];
        srv.read_exact(&mut buf).unwrap_or_else(|e| panic!("{e:?}"));

        srv.write_all(&t2v_payload)
            .unwrap_or_else(|e| panic!("{e:?}"));
        let recv = ws_vwr.read_binary();
        assert_eq!(recv, t2v_payload);

        let list = server.poll_json_until(&cookie, "/api/v1/sessions", IO_LIMIT, |l| {
            l.arr("items")
                .and_then(|items| {
                    items.iter().find(|i| {
                        i.str("viewer_transport") == Some("ws")
                            && i.num("bytes_viewer_to_target") == Some(120)
                            && i.num("bytes_target_to_viewer") == Some(240)
                    })
                })
                .is_some()
        });
        let items = some(list.arr("items"), "items");
        let item = some(
            items
                .iter()
                .find(|i| i.str("viewer_transport") == Some("ws")),
            "ws session",
        );
        assert_eq!(item.num("bytes_viewer_to_target"), Some(120));
        assert_eq!(item.num("bytes_target_to_viewer"), Some(240));
    }

    // 3. WSS Viewer + TCP Server
    {
        let mut srv = server.connect_tcp_server();
        srv.write_all(&make_mode2_request("6003"))
            .unwrap_or_else(|e| panic!("{e:?}"));
        let mut wss_vwr = server.connect_wss_viewer();
        wss_vwr.send_binary(&make_mode2_request("6003"));

        wss_vwr.send_binary(&v2t_payload);
        let mut buf = vec![0u8; 120];
        srv.read_exact(&mut buf).unwrap_or_else(|e| panic!("{e:?}"));

        srv.write_all(&t2v_payload)
            .unwrap_or_else(|e| panic!("{e:?}"));
        let recv = wss_vwr.read_binary();
        assert_eq!(recv, t2v_payload);

        let list = server.poll_json_until(&cookie, "/api/v1/sessions", IO_LIMIT, |l| {
            l.arr("items")
                .and_then(|items| {
                    items.iter().find(|i| {
                        i.str("viewer_transport") == Some("wss")
                            && i.num("bytes_viewer_to_target") == Some(120)
                            && i.num("bytes_target_to_viewer") == Some(240)
                    })
                })
                .is_some()
        });
        let items = some(list.arr("items"), "items");
        let item = some(
            items
                .iter()
                .find(|i| i.str("viewer_transport") == Some("wss")),
            "wss session",
        );
        assert_eq!(item.num("bytes_viewer_to_target"), Some(120));
        assert_eq!(item.num("bytes_target_to_viewer"), Some(240));
    }
}

/// spec: management.md:MGT-007 partial
#[test]
fn reconnect_gives_new_session_id_and_restarts_counters() {
    let _lock = lock_ports();
    let server = RunningServer::start("mgt007", "", 5, 10, 5);
    let (cookie, _) = server.login();

    // Session 1 on ID 7001
    let mut srv1 = server.connect_tcp_server();
    srv1.write_all(&make_mode2_request("7001"))
        .unwrap_or_else(|e| panic!("{e:?}"));
    let mut vwr1 = server.connect_tcp_viewer();
    vwr1.write_all(&make_mode2_request("7001"))
        .unwrap_or_else(|e| panic!("{e:?}"));

    vwr1.write_all(
        b"SOME_DATA_80_BYTES_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    )
    .unwrap_or_else(|e| panic!("{e:?}"));
    let mut buf = vec![0u8; 80];
    srv1.read_exact(&mut buf)
        .unwrap_or_else(|e| panic!("{e:?}"));

    let list1 = server.poll_json_until(&cookie, "/api/v1/sessions", IO_LIMIT, |l| {
        l.arr("items").and_then(|items| items.first()).is_some()
    });
    let sid1 = some(
        some(list1.arr("items"), "items")[0].str("session_id"),
        "sid1",
    )
    .to_owned();

    // Close session 1
    drop(srv1);
    drop(vwr1);
    server.poll_json_until(&cookie, "/api/v1/sessions", IO_LIMIT, |l| {
        l.arr("items").map(|items| items.is_empty()) == Some(true)
    });

    // Reconnect on same ID 7001
    let mut srv2 = server.connect_tcp_server();
    srv2.write_all(&make_mode2_request("7001"))
        .unwrap_or_else(|e| panic!("{e:?}"));
    let mut vwr2 = server.connect_tcp_viewer();
    vwr2.write_all(&make_mode2_request("7001"))
        .unwrap_or_else(|e| panic!("{e:?}"));

    let list2 = server.poll_json_until(&cookie, "/api/v1/sessions", IO_LIMIT, |l| {
        l.arr("items").map(|items| items.len() == 1) == Some(true)
    });
    let items2 = some(list2.arr("items"), "items2");
    assert_eq!(items2.len(), 1);
    let sid2 = some(items2[0].str("session_id"), "sid2");

    assert_ne!(sid1, sid2, "reconnect must issue fresh session_id");
    assert_eq!(
        items2[0].num("bytes_viewer_to_target"),
        Some(0),
        "counters restart from 0"
    );
    assert_eq!(items2[0].num("bytes_target_to_viewer"), Some(0));

    // Server uptime is non-negative
    let summary = server.query(&cookie, "/api/v1/summary").json();
    let uptime = some(
        summary.obj("server").and_then(|s| s.num("uptime_ms")),
        "uptime",
    );
    assert!(uptime >= 0);
}

// Query-body and relay_id omission coverage; the adapters test also covers auth
// failures, termination failure, response headers and the actual credential hash.
/// spec: management.md:MGT-013 partial
#[test]
fn no_secrets_or_raw_payload_in_query_or_error_bodies() {
    let _lock = lock_ports();
    let server = RunningServer::start("mgt013", "", 5, 10, 5);
    let (cookie, _) = server.login();

    let raw_secret_payload = "SUPER_SECRET_RAW_RFB_STREAM_MARKER_99887766";
    let mut srv = server.connect_tcp_server();
    srv.write_all(&make_mode2_request("1313"))
        .unwrap_or_else(|e| panic!("{e:?}"));
    let mut vwr = server.connect_tcp_viewer();
    vwr.write_all(&make_mode2_request("1313"))
        .unwrap_or_else(|e| panic!("{e:?}"));

    vwr.write_all(raw_secret_payload.as_bytes())
        .unwrap_or_else(|e| panic!("{e:?}"));
    let mut buf = vec![0u8; raw_secret_payload.len()];
    srv.read_exact(&mut buf).unwrap_or_else(|e| panic!("{e:?}"));

    let list = server.poll_json_until(&cookie, "/api/v1/sessions", IO_LIMIT, |l| {
        l.arr("items").and_then(|items| items.first()).is_some()
    });
    let sid = some(some(list.arr("items"), "items")[0].str("session_id"), "sid");

    let responses = [
        server.query(&cookie, "/api/v1/summary"),
        server.query(&cookie, "/api/v1/waiting"),
        server.query(&cookie, "/api/v1/sessions"),
        server.query(&cookie, &format!("/api/v1/sessions/{sid}")),
        server.query(&cookie, "/api/v1/closed-sessions"),
        server.query(&cookie, "/api/v1/not_found"),
        server.send_http("POST", "/api/v1/summary", &[("Cookie", &cookie)], b""),
        server.send_http(
            "GET",
            "/api/v1/summary",
            &[("Cookie", &cookie), ("X-Relay-Poll", "invalid")],
            b"",
        ),
        server.send_http("GET", "/api/v1/summary", &[], b""),
    ];

    for r in &responses {
        assert!(
            !r.body.contains(ALICE_PASSWORD),
            "leaked password in {}",
            r.body
        );
        assert!(
            !r.body.contains("argon2id"),
            "leaked hash string in {}",
            r.body
        );
        assert!(
            !r.body.contains(raw_secret_payload),
            "leaked raw payload in {}",
            r.body
        );
    }

    // relay_id must NOT appear in sessions or closed-sessions
    let sess_rep = server.query(&cookie, "/api/v1/sessions");
    assert!(
        !sess_rep.body.contains("\"relay_id\""),
        "relay_id leaked in sessions: {}",
        sess_rep.body
    );
}

/// spec: management.md:MGT-015
#[test]
fn forwarded_headers_are_ignored_and_real_socket_address_used() {
    let _lock = lock_ports();
    let server = RunningServer::start("mgt015", "", 5, 10, 5);
    let (cookie, _) = server.login();

    let mut srv = server.connect_tcp_server();
    srv.write_all(&make_mode2_request("1515"))
        .unwrap_or_else(|e| panic!("{e:?}"));
    let mut vwr = server.connect_tcp_viewer();
    vwr.write_all(&make_mode2_request("1515"))
        .unwrap_or_else(|e| panic!("{e:?}"));

    server.poll_json_until(&cookie, "/api/v1/sessions", IO_LIMIT, |l| {
        l.arr("items").and_then(|items| items.first()).is_some()
    });

    let list_rep = server.send_http(
        "GET",
        "/api/v1/sessions",
        &[
            ("Cookie", &cookie),
            ("X-Forwarded-For", "198.51.100.1, 203.0.113.1"),
            ("Forwarded", "for=192.0.2.1;proto=https"),
        ],
        b"",
    );
    assert_eq!(list_rep.status, 200);
    let json = list_rep.json();
    let item = &some(json.arr("items"), "items")[0];
    let v_ip = some(
        item.obj("viewer_address").and_then(|a| a.str("ip")),
        "viewer ip",
    );
    let t_ip = some(
        item.obj("target_address").and_then(|a| a.str("ip")),
        "target ip",
    );

    assert_eq!(v_ip, "127.0.0.1", "must ignore forwarded headers");
    assert_eq!(t_ip, "127.0.0.1", "must ignore forwarded headers");
}

/// spec: management.md:MGT-016 partial
#[test]
fn single_response_consistency_under_churn() {
    let _lock = lock_ports();
    let server = RunningServer::start("mgt016", "", 5, 10, 5);
    let (cookie, _) = server.login();

    // In each individual query response under steady or active state,
    // as_of is valid and internal structures are mutually consistent.
    for i in 0..5 {
        let mut srv = server.connect_tcp_server();
        let id = format!("16{i}");
        srv.write_all(&make_mode2_request(&id))
            .unwrap_or_else(|e| panic!("{e:?}"));

        let sum = server.query(&cookie, "/api/v1/summary").json();
        assert_timestamp(some(sum.str("as_of"), "as_of"));
        let w_cnt = some(sum.obj("counts").and_then(|c| c.num("waiting")), "w_cnt");
        let a_cnt = some(sum.obj("counts").and_then(|c| c.num("active")), "a_cnt");
        assert!(w_cnt >= 0);
        assert!(a_cnt >= 0);

        let w = server.query(&cookie, "/api/v1/waiting").json();
        assert_timestamp(some(w.str("as_of"), "w as_of"));
        let items = some(w.arr("items"), "items");
        for it in items {
            assert_timestamp(some(it.str("connected_at"), "connected_at"));
            assert!(some(it.num("waited_ms"), "waited_ms") >= 0);
        }
    }
}

/// spec: management.md:MGT-021
#[test]
fn unauthenticated_queries_are_rejected_on_all_routes() {
    let _lock = lock_ports();
    let server = RunningServer::start("mgt021", "", 5, 10, 5);

    let protected_routes = [
        "/api/v1/summary",
        "/api/v1/waiting",
        "/api/v1/sessions",
        "/api/v1/sessions/ses_00000000000000000000000000",
        "/api/v1/closed-sessions",
        "/api/v1/session",
    ];

    for path in &protected_routes {
        // Without cookie
        let rep = server.send_http("GET", path, &[], b"");
        assert_eq!(rep.status, 401, "unauthenticated on {path}");
        assert_eq!(
            rep.header("www-authenticate"),
            Some("Session realm=\"vnc-relay\""),
            "WWW-Authenticate missing on {path}"
        );
        let err = rep.json();
        assert_eq!(
            err.obj("error").and_then(|e| e.str("code")),
            Some("unauthenticated"),
            "expected unauthenticated code on {path}"
        );

        // With bogus cookie
        let rep_bogus = server.send_http(
            "GET",
            path,
            &[("Cookie", "relay_admin_session=bogus123")],
            b"",
        );
        assert_eq!(rep_bogus.status, 401, "bogus cookie on {path}");
        assert_eq!(
            rep_bogus.header("www-authenticate"),
            Some("Session realm=\"vnc-relay\"")
        );
    }
}

/// spec: management.md:MGT-023 partial
#[test]
fn closed_sessions_records_eof_and_error_causes_and_detail_of_closed_is_404() {
    let _lock = lock_ports();
    let server = RunningServer::start("mgt023", "", 5, 10, 5);
    let (cookie, _) = server.login();

    // 1. Session 1: normal peer_eof close
    let mut srv1 = server.connect_tcp_server();
    srv1.write_all(&make_mode2_request("2301"))
        .unwrap_or_else(|e| panic!("{e:?}"));
    let mut vwr1 = server.connect_tcp_viewer();
    vwr1.write_all(&make_mode2_request("2301"))
        .unwrap_or_else(|e| panic!("{e:?}"));
    server.poll_json_until(&cookie, "/api/v1/sessions", IO_LIMIT, |l| {
        l.arr("items").and_then(|items| items.first()).is_some()
    });
    drop(srv1);
    drop(vwr1);

    // 2. Session 2: connection error / abrupt close
    let mut srv2 = server.connect_tcp_server();
    srv2.write_all(&make_mode2_request("2302"))
        .unwrap_or_else(|e| panic!("{e:?}"));
    let mut vwr2 = server.connect_tcp_viewer();
    vwr2.write_all(&make_mode2_request("2302"))
        .unwrap_or_else(|e| panic!("{e:?}"));
    server.poll_json_until(&cookie, "/api/v1/sessions", IO_LIMIT, |l| {
        l.arr("items").and_then(|items| items.first()).is_some()
    });
    drop(vwr2);
    let _ = srv2.write_all(b"DATA_AFTER_PEER_CLOSED");
    let _ = srv2.flush();
    drop(srv2);

    let closed = server.poll_json_until(&cookie, "/api/v1/closed-sessions", IO_LIMIT, |c| {
        c.arr("items").map(|items| items.len() >= 2) == Some(true)
    });
    let items = some(closed.arr("items"), "closed items");
    assert!(!items.is_empty(), "must have closed items");

    let mut found_eof = false;
    let mut any_sid = String::new();
    for it in items {
        assert_timestamp(some(it.str("started_at"), "started_at"));
        assert_timestamp(some(it.str("ended_at"), "ended_at"));
        assert!(some(it.num("bytes_viewer_to_target"), "bytes_v2t") >= 0);
        assert!(some(it.num("bytes_target_to_viewer"), "bytes_t2v") >= 0);
        let cause = some(it.str("close_cause"), "close_cause");
        if cause == "peer_eof" {
            found_eof = true;
        }
        any_sid = some(it.str("session_id"), "sid").to_owned();
    }
    assert!(found_eof, "peer_eof must be present in closed_sessions");

    // Detail query with a closed session_id returns 404 not_found
    let detail = server.query(&cookie, &format!("/api/v1/sessions/{any_sid}"));
    assert_eq!(detail.status, 404, "closed session detail must return 404");
    assert_eq!(
        detail.json().obj("error").and_then(|e| e.str("code")),
        Some("not_found")
    );
}

/// spec: management.md:MGT-024
#[test]
fn recent_closed_fifo_overflow_and_restart_clears_list() {
    let _lock = lock_ports();
    // Configure R = 2
    let server = RunningServer::start("mgt024", "", 2, 10, 5);
    let (cookie, _) = server.login();

    // Session A
    let mut srv_a = server.connect_tcp_server();
    srv_a
        .write_all(&make_mode2_request("2401"))
        .unwrap_or_else(|e| panic!("{e:?}"));
    let mut vwr_a = server.connect_tcp_viewer();
    vwr_a
        .write_all(&make_mode2_request("2401"))
        .unwrap_or_else(|e| panic!("{e:?}"));
    server.poll_json_until(&cookie, "/api/v1/sessions", IO_LIMIT, |l| {
        l.arr("items").and_then(|items| items.first()).is_some()
    });
    drop(srv_a);
    drop(vwr_a);
    let list_a = server.poll_json_until(&cookie, "/api/v1/closed-sessions", IO_LIMIT, |c| {
        c.arr("items").map(|items| items.len() == 1) == Some(true)
    });
    let sid_a = some(
        some(list_a.arr("items"), "items")[0].str("session_id"),
        "sid_a",
    )
    .to_owned();

    // Session B
    let mut srv_b = server.connect_tcp_server();
    srv_b
        .write_all(&make_mode2_request("2402"))
        .unwrap_or_else(|e| panic!("{e:?}"));
    let mut vwr_b = server.connect_tcp_viewer();
    vwr_b
        .write_all(&make_mode2_request("2402"))
        .unwrap_or_else(|e| panic!("{e:?}"));
    server.poll_json_until(&cookie, "/api/v1/sessions", IO_LIMIT, |l| {
        l.arr("items").and_then(|items| items.first()).is_some()
    });
    drop(srv_b);
    drop(vwr_b);
    let list_b = server.poll_json_until(&cookie, "/api/v1/closed-sessions", IO_LIMIT, |c| {
        c.arr("items").map(|items| items.len() == 2) == Some(true)
    });
    let items_b = some(list_b.arr("items"), "items_b");
    assert_eq!(items_b.len(), 2);
    let sid_b = some(items_b[0].str("session_id"), "sid_b").to_owned();

    // Session C: overflows capacity R=2 -> oldest session A is dropped
    let mut srv_c = server.connect_tcp_server();
    srv_c
        .write_all(&make_mode2_request("2403"))
        .unwrap_or_else(|e| panic!("{e:?}"));
    let mut vwr_c = server.connect_tcp_viewer();
    vwr_c
        .write_all(&make_mode2_request("2403"))
        .unwrap_or_else(|e| panic!("{e:?}"));
    server.poll_json_until(&cookie, "/api/v1/sessions", IO_LIMIT, |l| {
        l.arr("items").and_then(|items| items.first()).is_some()
    });
    drop(srv_c);
    drop(vwr_c);
    let list_c = server.poll_json_until(&cookie, "/api/v1/closed-sessions", IO_LIMIT, |c| {
        c.arr("items").map(|items| {
            items.len() == 2
                && items
                    .iter()
                    .all(|it| it.str("session_id") != Some(sid_a.as_str()))
        }) == Some(true)
    });
    let items_c = some(list_c.arr("items"), "items_c");
    assert_eq!(items_c.len(), 2, "max_entries bounds list length");
    let c_ids: Vec<&str> = items_c
        .iter()
        .filter_map(|it| it.str("session_id"))
        .collect();
    assert!(
        !c_ids.contains(&sid_a.as_str()),
        "oldest session A must be dropped"
    );
    assert!(c_ids.contains(&sid_b.as_str()), "session B retained");

    // Restart process: stops server and starts fresh server
    server.stop();
    let fresh_server = RunningServer::start("mgt024_fresh", "", 2, 10, 5);
    let (fresh_cookie, _) = fresh_server.login();
    let fresh_closed = fresh_server
        .query(&fresh_cookie, "/api/v1/closed-sessions")
        .json();
    assert_eq!(
        fresh_closed.arr("items").map(|a| a.len()),
        Some(0),
        "restart clears in-memory recent closed list"
    );
}

#[cfg(target_os = "linux")]
/// spec: management.md:MGT-026
#[test]
fn counts_and_usage_match_at_limit_with_pending_connections_linux() {
    let _lock = lock_ports();
    // Mode I target connections count toward L:
    // 1. docs/specs/operations.md §3.1 table row L:
    //    "L | 중계 측 연결 소켓 상한 | 중계 리스너가 수락한 연결(TLS·업그레이드·초기 교환·대기·활성)과
    //     Mode I의 연결 중·연결된 대상 소켓 포함."
    // 2. docs/specs/operations.md §3.1 (line 64):
    //    "Mode I의 Viewer 연결과 생성한 대상 연결은 L에서 각각 한 개다. Mode II의 Viewer·Server 연결도 각각 한 개다.
    //     관리·상태 연결은 L이 아니라 Lm에서 센다. 수신·송신 소켓을 구분해 상한을 우회해서는 안 되며,
    //     대상 연결 생성 전에도 예산을 확보한다."
    // 3. docs/architectures/connection-limits.md §2.1 (line 28):
    //    "Mode I 대상 소켓의 L 집계. 작업 2.8이 대상 연결 전에 같은 L 예산에서 자리를 잡는다(OPS-023)."
    //
    // Budget calculation:
    // - 1 Mode I connection dialing target = 2 slots of L (1 viewer socket + 1 target socket)
    // - 2 handshaking connections = 2 slots of L
    // Sizing relay_connections limit L = 2 + 2 = 4, admin_limit Lm = 5.
    let server = RunningServer::start("mgt026_linux", "", 5, 4, 5);
    let (cookie, _) = server.login();

    // Ensure initial readiness probe sockets are completely released
    server.poll_summary_until(&cookie, IO_LIMIT, |summary| {
        summary
            .obj("usage")
            .and_then(|u| u.obj("relay_connections"))
            .and_then(|r| r.num("current"))
            == Some(0)
    });

    // Backlog target causes Mode I target connect to hang pending
    let backlog = Backlog::saturated();
    let target_addr = backlog.address();

    let mut pending_target_vwr = server.connect_tcp_viewer();
    pending_target_vwr
        .write_all(&make_mode1_request(&target_addr.to_string()))
        .unwrap_or_else(|e| panic!("{e:?}"));
    pending_target_vwr
        .flush()
        .unwrap_or_else(|e| panic!("{e:?}"));

    server.poll_summary_until(&cookie, IO_LIMIT, |summary| {
        summary
            .obj("counts")
            .and_then(|c| c.num("target_connecting"))
            == Some(1)
    });

    // Hold two connections in initial exchange (handshaking)
    let addr = SocketAddr::from(([127, 0, 0, 1], server.tcp_viewer_port));
    let mut h1 = TcpStream::connect(addr).unwrap_or_else(|e| panic!("{e:?}"));
    let mut g1 = [0u8; 12];
    h1.read_exact(&mut g1).unwrap_or_else(|e| panic!("{e:?}"));
    h1.write_all(b"ID:10").unwrap_or_else(|e| panic!("{e:?}"));

    let mut h2 = TcpStream::connect(addr).unwrap_or_else(|e| panic!("{e:?}"));
    let mut g2 = [0u8; 12];
    h2.read_exact(&mut g2).unwrap_or_else(|e| panic!("{e:?}"));
    h2.write_all(b"ID:20").unwrap_or_else(|e| panic!("{e:?}"));

    let summary = server.poll_summary_until(&cookie, IO_LIMIT, |summary| {
        let counts = summary.obj("counts");
        let relay = summary
            .obj("usage")
            .and_then(|u| u.obj("relay_connections"));
        counts.and_then(|c| c.num("handshaking")) == Some(2)
            && counts.and_then(|c| c.num("target_connecting")) == Some(1)
            && relay.and_then(|r| r.num("current")) == Some(4)
    });

    let counts = some(summary.obj("counts"), "counts");
    let usage = some(summary.obj("usage"), "usage");
    let relay_usage = some(usage.obj("relay_connections"), "relay_connections");
    let admin_usage = some(usage.obj("admin_connections"), "admin_connections");

    assert_eq!(counts.num("handshaking"), Some(2));
    assert_eq!(counts.num("target_connecting"), Some(1));
    assert_eq!(counts.num("waiting"), Some(0));
    assert_eq!(counts.num("active"), Some(0));

    assert_eq!(relay_usage.num("current"), Some(4));
    assert_eq!(relay_usage.num("limit"), Some(4));
    assert!(relay_usage.is_null("current_unavailable_reason"));

    assert_eq!(admin_usage.num("current"), Some(1));
    assert_eq!(admin_usage.num("limit"), Some(5));
    assert!(admin_usage.is_null("current_unavailable_reason"));
}

/// spec: management.md:MGT-026 partial
#[test]
fn counts_and_usage_match_at_limit_portable() {
    let _lock = lock_ports();
    let server = RunningServer::start("mgt026_portable", "", 5, 2, 5);
    let (cookie, _) = server.login();

    // Ensure initial readiness probe sockets are completely released
    server.poll_summary_until(&cookie, IO_LIMIT, |summary| {
        summary
            .obj("usage")
            .and_then(|u| u.obj("relay_connections"))
            .and_then(|r| r.num("current"))
            == Some(0)
    });

    // Saturate L=2 with handshaking connections
    let addr = SocketAddr::from(([127, 0, 0, 1], server.tcp_viewer_port));
    let mut h1 = TcpStream::connect(addr).unwrap_or_else(|e| panic!("{e:?}"));
    let mut g1 = [0u8; 12];
    h1.read_exact(&mut g1).unwrap_or_else(|e| panic!("{e:?}"));
    h1.write_all(b"ID:1").unwrap_or_else(|e| panic!("{e:?}"));

    let mut h2 = TcpStream::connect(addr).unwrap_or_else(|e| panic!("{e:?}"));
    let mut g2 = [0u8; 12];
    h2.read_exact(&mut g2).unwrap_or_else(|e| panic!("{e:?}"));
    h2.write_all(b"ID:2").unwrap_or_else(|e| panic!("{e:?}"));

    let summary = server.poll_summary_until(&cookie, IO_LIMIT, |summary| {
        let counts = summary.obj("counts");
        let relay = summary
            .obj("usage")
            .and_then(|u| u.obj("relay_connections"));
        counts.and_then(|c| c.num("handshaking")) == Some(2)
            && relay.and_then(|r| r.num("current")) == Some(2)
    });

    let counts = some(summary.obj("counts"), "counts");
    let usage = some(summary.obj("usage"), "usage");
    let relay_usage = some(usage.obj("relay_connections"), "relay_connections");

    assert_eq!(counts.num("handshaking"), Some(2));
    assert_eq!(relay_usage.num("current"), Some(2));
    assert_eq!(relay_usage.num("limit"), Some(2));
    assert!(relay_usage.is_null("current_unavailable_reason"));
}

/// spec: operations.md:OPS-033 partial
#[test]
fn relay_exhaustion_does_not_block_management_and_admin_connections_are_visible() {
    let _lock = lock_ports();
    let server = RunningServer::start("ops033", "", 5, 2, 5);
    let (cookie, _) = server.login();

    // After wait_ready, poll the management summary until usage.relay_connections.current == 0
    // before saturating L. Do not sleep.
    server.poll_summary_until(&cookie, IO_LIMIT, |summary| {
        summary
            .obj("usage")
            .and_then(|u| u.obj("relay_connections"))
            .and_then(|r| r.num("current"))
            == Some(0)
    });

    // 1. Saturate relay limit L=2 with connections
    let addr = SocketAddr::from(([127, 0, 0, 1], server.tcp_viewer_port));
    let mut h1 = TcpStream::connect(addr).unwrap_or_else(|e| panic!("{e:?}"));
    let mut g1 = [0u8; 12];
    h1.read_exact(&mut g1).unwrap_or_else(|e| panic!("{e:?}"));
    h1.write_all(b"ID:91").unwrap_or_else(|e| panic!("{e:?}"));

    let mut h2 = TcpStream::connect(addr).unwrap_or_else(|e| panic!("{e:?}"));
    let mut g2 = [0u8; 12];
    h2.read_exact(&mut g2).unwrap_or_else(|e| panic!("{e:?}"));
    h2.write_all(b"ID:92").unwrap_or_else(|e| panic!("{e:?}"));

    server.poll_summary_until(&cookie, IO_LIMIT, |summary| {
        summary
            .obj("usage")
            .and_then(|u| u.obj("relay_connections"))
            .and_then(|r| r.num("current"))
            == Some(2)
    });

    // 2. While relay budget L is completely exhausted, management login and query MUST succeed!
    let (cookie2, _) = server.login();
    let summary = server.query(&cookie2, "/api/v1/summary").json();
    let usage = some(summary.obj("usage"), "usage");
    let relay_u = some(usage.obj("relay_connections"), "relay_connections");
    let admin_u = some(usage.obj("admin_connections"), "admin_connections");

    assert_eq!(relay_u.num("current"), Some(2));
    assert_eq!(relay_u.num("limit"), Some(2));
    let admin_curr = some(admin_u.num("current"), "admin curr");
    assert!(admin_curr >= 1, "management connection is counted in Lm");
    assert_eq!(admin_u.num("limit"), Some(5));
}

#[test]
fn contract_rules_error_order_methods_polling_and_headers() {
    let _lock = lock_ports();
    let server = RunningServer::start("contract_rules", "", 5, 10, 5);
    let (cookie, _) = server.login();

    // 1. Error order: 401 unauthenticated takes precedence over 404, 405, 400
    let r1 = server.send_http("GET", "/api/v1/nonexistent", &[], b"");
    assert_eq!(r1.status, 401);
    assert_eq!(
        r1.json().obj("error").and_then(|e| e.str("code")),
        Some("unauthenticated")
    );

    let r2 = server.send_http("POST", "/api/v1/summary", &[], b"");
    assert_eq!(r2.status, 401);

    let r3 = server.send_http("GET", "/api/v1/sessions/bad!format", &[], b"");
    assert_eq!(r3.status, 401);

    // 2. Cache-Control: no-store header on API responses
    let sum_rep = server.query(&cookie, "/api/v1/summary");
    assert_eq!(sum_rep.status, 200);
    assert_eq!(sum_rep.header("cache-control"), Some("no-store"));

    let wait_rep = server.query(&cookie, "/api/v1/waiting");
    assert_eq!(wait_rep.header("cache-control"), Some("no-store"));

    // 3. Polling with X-Relay-Poll: 1 succeeds and is accepted
    let poll_rep = server.query_poll(&cookie, "/api/v1/summary");
    assert_eq!(
        poll_rep.status, 200,
        "poll query with X-Relay-Poll: 1 succeeds"
    );
    assert_eq!(poll_rep.header("cache-control"), Some("no-store"));

    // 4. Bad X-Relay-Poll values rejected with 400 invalid_request
    for bad in ["2", "true", "0", ""] {
        let rep = server.send_http(
            "GET",
            "/api/v1/summary",
            &[("Cookie", &cookie), ("X-Relay-Poll", bad)],
            b"",
        );
        assert_eq!(rep.status, 400, "bad X-Relay-Poll {bad} must be 400");
        assert_eq!(
            rep.json().obj("error").and_then(|e| e.str("code")),
            Some("invalid_request")
        );
    }
    // Note on Si expiration: per config-v2.md, minimum allowed management.session.idle_secs
    // is 300s (5m), which is not cheap to wait in automated tests; timing test is skipped
    // per prompt instructions ("if the timing is cheap; else skip and say so").
}
