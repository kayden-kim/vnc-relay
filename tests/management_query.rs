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
//! - MGT-026 (full on Linux and Windows, partial portable): counts and usage match at L limit with pending connections
//! - OPS-033 (full): login, queries and termination at L; bidirectional relay at shared management/status Lm
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
        Self::start_with_config_edit(
            name,
            extra_config,
            max_entries,
            relay_limit,
            admin_limit,
            false,
        )
    }

    fn start_with_config_edit(
        name: &str,
        extra_config: &str,
        max_entries: u32,
        relay_limit: u32,
        admin_limit: u32,
        config_edit: bool,
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
config_edit = {config_edit}
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
        self.login_as(ALICE)
    }

    fn login_as(&self, account: &str) -> (String, String) {
        let body = format!("{{\"account\":\"{account}\",\"password\":\"{ALICE_PASSWORD}\"}}");
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

#[cfg(windows)]
/// spec: management.md:MGT-026
#[test]
fn counts_and_usage_match_at_limit_with_pending_connections_windows() {
    fn at_limit(summary: &JsonValue) -> bool {
        let counts = summary.obj("counts");
        let relay = summary
            .obj("usage")
            .and_then(|u| u.obj("relay_connections"));
        counts.and_then(|c| c.num("handshaking")) == Some(2)
            && counts.and_then(|c| c.num("target_connecting")) == Some(1)
            && relay.and_then(|r| r.num("current")) == Some(4)
    }

    fn relay_current(summary: &JsonValue) -> Option<i64> {
        summary
            .obj("usage")
            .and_then(|u| u.obj("relay_connections"))
            .and_then(|r| r.num("current"))
    }

    let _lock = lock_ports();
    // Same budget as the Linux test: one Mode I viewer and its target socket,
    // plus two connections in the initial exchange, fill L = 4.
    let server = RunningServer::start("mgt026_windows", "", 5, 4, 5);
    let (cookie, _) = server.login();
    server.poll_summary_until(&cookie, IO_LIMIT, |summary| {
        relay_current(summary) == Some(0)
    });

    let addr = SocketAddr::from(([127, 0, 0, 1], server.tcp_viewer_port));
    let mut h1 = TcpStream::connect(addr).unwrap_or_else(|e| panic!("{e:?}"));
    let mut g1 = [0u8; 12];
    h1.read_exact(&mut g1).unwrap_or_else(|e| panic!("{e:?}"));
    h1.write_all(b"ID:10").unwrap_or_else(|e| panic!("{e:?}"));
    let mut h2 = TcpStream::connect(addr).unwrap_or_else(|e| panic!("{e:?}"));
    let mut g2 = [0u8; 12];
    h2.read_exact(&mut g2).unwrap_or_else(|e| panic!("{e:?}"));
    h2.write_all(b"ID:20").unwrap_or_else(|e| panic!("{e:?}"));
    server.poll_summary_until(&cookie, IO_LIMIT, |summary| {
        relay_current(summary) == Some(2)
    });

    // Windows refuses a connect to a closed loopback port only after about
    // 2 s of SYN retries (2.0 s measured on 2026-11-04), so the relay's target
    // connect stays pending for that window. A window missed under load is
    // retried with a new Mode I connection; each judgement is one response.
    let closed_target = TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .unwrap_or_else(|e| panic!("reserve a closed port: {e:?}"));
    let mut observed = None;
    for _ in 0..5 {
        let mut viewer = server.connect_tcp_viewer();
        viewer
            .write_all(&make_mode1_request(&closed_target.to_string()))
            .unwrap_or_else(|e| panic!("{e:?}"));
        let window_end = Instant::now() + Duration::from_millis(1500);
        while observed.is_none() && Instant::now() < window_end {
            let reply = server.query(&cookie, "/api/v1/summary");
            if reply.status == 200 {
                let summary = reply.json();
                if at_limit(&summary) {
                    observed = Some(summary);
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        drop(viewer);
        if observed.is_some() {
            break;
        }
        server.poll_summary_until(&cookie, IO_LIMIT, |summary| {
            relay_current(summary) == Some(2)
        });
    }
    let summary = some(observed, "summary while the target connect is pending");

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
    drop((h1, h2));
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

/// spec: operations.md:OPS-033
#[test]
fn exhausted_relay_and_admin_budgets_leave_the_other_service_operational() {
    fn wait_event(log: &Path, after: usize, predicate: impl Fn(&JsonValue) -> bool) -> JsonValue {
        let deadline = Instant::now() + IO_LIMIT;
        loop {
            let text =
                fs::read_to_string(log).unwrap_or_else(|e| panic!("read {}: {e:?}", log.display()));
            for line in text[after..].split_inclusive('\n') {
                if line.ends_with('\n') {
                    let event = parse_json(line);
                    if predicate(&event) {
                        return event;
                    }
                }
            }
            assert!(Instant::now() < deadline, "event not observed in {text}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn log_position(log: &Path) -> usize {
        fs::read_to_string(log)
            .unwrap_or_else(|e| panic!("read {}: {e:?}", log.display()))
            .len()
    }

    fn assert_closed(stream: &mut TcpStream) {
        let mut byte = [0];
        match stream.read(&mut byte) {
            Ok(0) => {}
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
                ) => {}
            result => panic!("expected connection refusal without response, got {result:?}"),
        }
    }

    fn idle_connection(port: u16) -> TcpStream {
        // A just-completed HTTP request may still be releasing its Lm permit.
        // Retry only connections actually closed by the server, not a fixed delay.
        let deadline = Instant::now() + IO_LIMIT;
        loop {
            let stream =
                TcpStream::connect_timeout(&SocketAddr::from(([127, 0, 0, 1], port)), IO_LIMIT)
                    .unwrap_or_else(|e| panic!("connect idle socket: {e:?}"));
            stream
                .set_read_timeout(Some(Duration::from_millis(20)))
                .unwrap_or_else(|e| panic!("idle read timeout: {e:?}"));
            let mut byte = [0];
            match stream.peek(&mut byte) {
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    stream
                        .set_read_timeout(Some(IO_LIMIT))
                        .unwrap_or_else(|e| panic!("restore read timeout: {e:?}"));
                    stream
                        .set_write_timeout(Some(IO_LIMIT))
                        .unwrap_or_else(|e| panic!("idle write timeout: {e:?}"));
                    return stream;
                }
                Ok(0) => {}
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
                    ) => {}
                result => panic!("unexpected idle socket response: {result:?}"),
            }
            assert!(Instant::now() < deadline, "Lm permit was not released");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn finish_held_request(stream: &mut TcpStream, request: &str) -> Reply {
        stream
            .write_all(request.as_bytes())
            .unwrap_or_else(|e| panic!("write held request: {e:?}"));
        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .unwrap_or_else(|e| panic!("read held response: {e:?}"));
        parse_http_reply(&response)
    }

    let _lock = lock_ports();
    // L=2 is the contract minimum and fits exactly one active Mode II pair.
    // Lm=2 lets one idle management and one idle status socket fill the shared pool.
    let server = RunningServer::start(
        "ops033",
        "[log]\noutput = \"relay.log\"\n[timeouts]\nclose_secs = 1\ntls_upgrade_secs = 120",
        5,
        2,
        2,
    );
    let log = server.dir.join("relay.log");
    let (initial_cookie, _) = server.login();
    server.poll_summary_until(&initial_cookie, IO_LIMIT, |summary| {
        summary
            .obj("usage")
            .and_then(|u| u.obj("relay_connections"))
            .and_then(|r| r.num("current"))
            == Some(0)
    });

    let mut target = server.connect_tcp_server();
    target
        .write_all(&make_mode2_request("33001"))
        .unwrap_or_else(|e| panic!("first server request: {e:?}"));
    let mut viewer = server.connect_tcp_viewer();
    viewer
        .write_all(&make_mode2_request("33001"))
        .unwrap_or_else(|e| panic!("first viewer request: {e:?}"));
    let started = wait_event(&log, 0, |e| e.str("event") == Some("session_started"));
    let sid = some(started.str("session_id"), "first session_id");

    // Both real relay sockets consume L; the next socket is refused at accept.
    let before_refusal = log_position(&log);
    let mut refused = server.connect_tcp_server();
    assert_closed(&mut refused);
    let rejected = wait_event(&log, before_refusal, |e| {
        e.str("event") == Some("connection_rejected")
            && e.str("listener") == Some("tcp_server")
            && e.str("reason") == Some("relay_connection_limit")
    });
    assert_eq!(rejected.str("stage"), Some("accept"));
    assert_eq!(rejected.num("limit"), Some(2));

    // Login itself must succeed after saturation, not merely reuse an old cookie.
    let (cookie, csrf) = server.login();
    let reply = server.query(&cookie, "/api/v1/summary");
    assert_eq!(reply.status, 200);
    let summary = reply.json();
    let usage = some(summary.obj("usage"), "usage");
    let relay = some(usage.obj("relay_connections"), "relay_connections");
    assert_eq!(relay.num("current"), Some(2));
    assert_eq!(relay.num("limit"), Some(2));
    let admin = some(usage.obj("admin_connections"), "admin_connections");
    assert!(some(admin.num("current"), "admin current") >= 1);
    assert_eq!(admin.num("limit"), Some(2));
    let reply = server.query(&cookie, "/api/v1/sessions");
    assert_eq!(reply.status, 200);
    let sessions = reply.json();
    let items = some(sessions.arr("items"), "sessions items");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].str("session_id"), Some(sid));
    assert_eq!(items[0].str("state"), Some("active"));
    let reply = server.send_http(
        "POST",
        &format!("/api/v1/sessions/{sid}/terminate"),
        &[("Cookie", &cookie), ("X-CSRF-Token", &csrf)],
        b"",
    );
    assert_eq!(reply.status, 200, "terminate: {}", reply.body);
    assert_eq!(reply.json().str("result"), Some("terminated"));
    assert_closed(&mut target);
    assert_closed(&mut viewer);
    drop((target, viewer, refused));
    server.poll_summary_until(&cookie, IO_LIMIT, |summary| {
        summary
            .obj("usage")
            .and_then(|u| u.obj("relay_connections"))
            .and_then(|r| r.num("current"))
            == Some(0)
    });

    let mut status_holder = idle_connection(server.status_port);
    server.poll_summary_until(&cookie, IO_LIMIT, |summary| {
        summary
            .obj("usage")
            .and_then(|u| u.obj("admin_connections"))
            .and_then(|a| a.num("current"))
            == Some(2)
    });
    let mut management_holder = idle_connection(server.management_port);
    let before_admin_refusals = log_position(&log);
    for (port, listener) in [
        (server.management_port, "management"),
        (server.status_port, "status"),
    ] {
        let mut refused =
            TcpStream::connect_timeout(&SocketAddr::from(([127, 0, 0, 1], port)), IO_LIMIT)
                .unwrap_or_else(|e| panic!("connect excess {listener}: {e:?}"));
        refused
            .set_read_timeout(Some(IO_LIMIT))
            .unwrap_or_else(|e| panic!("refusal timeout: {e:?}"));
        assert_closed(&mut refused);
        let rejected = wait_event(&log, before_admin_refusals, |e| {
            e.str("event") == Some("connection_rejected")
                && e.str("listener") == Some(listener)
                && e.str("reason") == Some("admin_connection_limit")
        });
        assert_eq!(rejected.str("stage"), Some("accept"));
        assert_eq!(rejected.num("limit"), Some(2));
    }

    // No management/status permit is released before a whole relay exchange.
    let before_new_pair = log_position(&log);
    let mut target = server.connect_tcp_server();
    target
        .write_all(&make_mode2_request("33002"))
        .unwrap_or_else(|e| panic!("second server request: {e:?}"));
    let mut viewer = server.connect_tcp_viewer();
    viewer
        .write_all(&make_mode2_request("33002"))
        .unwrap_or_else(|e| panic!("second viewer request: {e:?}"));
    for listener in ["tcp_server", "tcp_viewer"] {
        wait_event(&log, before_new_pair, |e| {
            e.str("event") == Some("connection_accepted") && e.str("listener") == Some(listener)
        });
    }
    viewer
        .write_all(b"viewer-to-target")
        .unwrap_or_else(|e| panic!("viewer payload: {e:?}"));
    let mut received = [0; 16];
    target
        .read_exact(&mut received)
        .unwrap_or_else(|e| panic!("target payload: {e:?}"));
    assert_eq!(&received, b"viewer-to-target");
    target
        .write_all(b"target-to-viewer")
        .unwrap_or_else(|e| panic!("target response: {e:?}"));
    viewer
        .read_exact(&mut received)
        .unwrap_or_else(|e| panic!("viewer response: {e:?}"));
    assert_eq!(&received, b"target-to-viewer");

    // Successful requests on the original sockets also prove neither idle
    // holder was itself refused while establishing Lm saturation.
    let reply = finish_held_request(
        &mut status_holder,
        "GET /readyz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    );
    assert_eq!(reply.status, 200);
    drop(status_holder);
    let reply = finish_held_request(
        &mut management_holder,
        &format!(
            "GET /api/v1/summary HTTP/1.1\r\nHost: localhost\r\nCookie: {cookie}\r\nConnection: close\r\n\r\n"
        ),
    );
    assert_eq!(reply.status, 200);
    assert_eq!(
        reply
            .json()
            .obj("usage")
            .and_then(|u| u.obj("relay_connections"))
            .and_then(|r| r.num("current")),
        Some(2)
    );
    drop(management_holder);
    let (recovered_cookie, _) = server.login();
    let reply = server.query(&recovered_cookie, "/api/v1/sessions");
    assert_eq!(reply.status, 200);
    assert_eq!(reply.json().arr("items").map(<[_]>::len), Some(1));
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
    // Note on Si expiration: per config-v3.md, minimum allowed management.session.idle_secs
    // is 300s (5m), which is not cheap to wait in automated tests; timing test is skipped
    // per prompt instructions ("if the timing is cheap; else skip and say so").
}

// File-backed configuration scenarios share the real listener, credentials,
// HTTP transport and lifecycle fixture above.
type ConfigJson = serde_json::Value;

fn config_json(reply: &Reply) -> ConfigJson {
    serde_json::from_str(&reply.body)
        .unwrap_or_else(|error| panic!("configuration JSON: {error}: {}", reply.body))
}

fn config_server(name: &str, enabled: bool) -> RunningServer {
    RunningServer::start_with_config_edit(
        name,
        "[reload]\nsettle_secs = 1\npoll_secs = 5\n[log]\noutput = \"relay.log\"",
        5,
        10,
        5,
        enabled,
    )
}

impl RunningServer {
    fn config_path(&self) -> PathBuf {
        self.dir.join("relay.toml")
    }

    fn config_bytes(&self) -> Vec<u8> {
        fs::read(self.config_path()).unwrap_or_else(|error| panic!("read config: {error}"))
    }

    fn config_write(&self, bytes: impl AsRef<[u8]>) {
        fs::write(self.config_path(), bytes)
            .unwrap_or_else(|error| panic!("write config: {error}"));
    }

    fn config_view(&self, cookie: &str) -> ConfigJson {
        let reply = self.query(cookie, "/api/v1/config");
        assert_eq!(reply.status, 200, "{}", reply.body);
        assert_eq!(reply.header("cache-control"), Some("no-store"));
        let view = config_json(&reply);
        assert_timestamp(some(view["as_of"].as_str(), "config as_of"));
        view
    }

    fn config_patch(
        &self,
        admin: &(String, String),
        revision: &ConfigJson,
        changes: ConfigJson,
    ) -> Reply {
        let body = serde_json::json!({"revision": revision, "changes": changes}).to_string();
        self.send_http(
            "PATCH",
            "/api/v1/config",
            &[
                ("Cookie", &admin.0),
                ("X-CSRF-Token", &admin.1),
                ("Content-Type", "application/json"),
            ],
            body.as_bytes(),
        )
    }

    fn config_events(&self) -> Vec<ConfigJson> {
        let text = fs::read_to_string(self.dir.join("relay.log"))
            .unwrap_or_else(|error| panic!("read config events: {error}"));
        text.split_inclusive('\n')
            .filter(|line| line.ends_with('\n'))
            .map(|line| {
                serde_json::from_str(line)
                    .unwrap_or_else(|error| panic!("config event: {error}: {line}"))
            })
            .collect()
    }

    fn config_wait_event(&self, after: usize, name: &str) -> ConfigJson {
        let deadline = Instant::now() + READY_LIMIT;
        loop {
            let events = self.config_events();
            if let Some(event) = events.iter().skip(after).find(|e| e["event"] == name) {
                return event.clone();
            }
            assert!(Instant::now() < deadline, "missing {name}: {events:?}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn config_restart(&mut self) {
        let before = self.config_events().len();
        if let Some(mut child) = self.child.take() {
            child
                .kill()
                .unwrap_or_else(|error| panic!("stop config child: {error}"));
            child
                .wait()
                .unwrap_or_else(|error| panic!("reap config child: {error}"));
        }
        self.child = Some(
            Command::new(BIN)
                .args(["serve", "--config"])
                .arg(self.config_path())
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap_or_else(|error| panic!("restart config child: {error}")),
        );
        self.config_wait_event(before, "server_ready");
    }
}

fn config_item<'a>(view: &'a ConfigJson, key: &str) -> &'a ConfigJson {
    some(
        some(view["items"].as_array(), "config items")
            .iter()
            .find(|item| item["key"] == key),
        key,
    )
}

fn config_revision(bytes: &[u8]) -> String {
    use vnc_relay_app::management::config::ConfigFileStore;
    vnc_relay_adapters::config_edit::FsConfigFileStore.revision(bytes)
}

fn config_assert_revision(view: &ConfigJson, bytes: &[u8]) {
    assert_eq!(view["revision"], config_revision(bytes));
    assert!(vnc_relay_app::management::config::valid_revision(some(
        view["revision"].as_str(),
        "revision"
    )));
}

fn config_assert_error(reply: &Reply, status: u16, code: &str) -> ConfigJson {
    assert_eq!(reply.status, status, "{}", reply.body);
    let body = config_json(reply);
    assert_eq!(body["error"]["code"], code, "{}", reply.body);
    body
}

fn config_assert_no_success(events: &[ConfigJson]) {
    assert!(
        events.iter().all(|event| {
            event["event"] != "config_edited" && event["event"] != "reload_applied"
        }),
        "{events:?}"
    );
}

fn config_assert_audit(event: &ConfigJson, reason: Option<&str>, keys: ConfigJson) {
    config_assert_audit_account(event, ALICE, reason, keys);
}

fn config_assert_audit_account(
    event: &ConfigJson,
    account: &str,
    reason: Option<&str>,
    keys: ConfigJson,
) {
    assert_eq!(event["admin_account"], account);
    assert_eq!(event["peer_ip"], "127.0.0.1");
    assert_eq!(event["config_keys"], keys);
    if let Some(reason) = reason {
        assert_eq!(event["event"], "config_edit_rejected");
        assert_eq!(event["reason"], reason);
        assert!(event.get("revision").is_none());
    } else {
        assert_eq!(event["event"], "config_edited");
    }
    for field in ["changes", "value", "values", "password", "csrf_token"] {
        assert!(event.get(field).is_none(), "unexpected {field}: {event}");
    }
}

/// spec: management.md:MGT-028
#[test]
fn config_view_reports_effective_values_all_file_states_and_reload_history() {
    let _lock = lock_ports();
    let mut server = config_server("config-view", false);
    let admin = server.login();
    let original = String::from_utf8(server.config_bytes())
        .unwrap_or_else(|error| panic!("config UTF-8: {error}"));
    let initial = server.config_view(&admin.0);
    assert_eq!(initial["edit_enabled"], false);
    assert_eq!(
        initial["file"],
        serde_json::json!({
            "state": "in_effect", "config_keys": [], "diagnostics": []
        })
    );
    assert!(initial["last_reload"].is_null());
    config_assert_revision(&initial, original.as_bytes());
    let keys = [
        "admin.accounts_file",
        "admin.argon2.memory_kib",
        "admin.argon2.iterations",
        "admin.argon2.parallelism",
        "relay.tcp_viewer.enabled",
        "relay.tcp_viewer.listen",
        "relay.tcp_viewer.allow",
        "relay.tcp_server.enabled",
        "relay.tcp_server.listen",
        "relay.tcp_server.allow",
        "relay.ws_viewer.enabled",
        "relay.ws_viewer.listen",
        "relay.ws_viewer.allow",
        "relay.ws_viewer.allowed_origins",
        "relay.wss_viewer.enabled",
        "relay.wss_viewer.listen",
        "relay.wss_viewer.allow",
        "relay.wss_viewer.allowed_origins",
        "relay.wss_viewer.cert_file",
        "relay.wss_viewer.key_file",
        "mode1.enabled",
        "mode1.allow_targets",
        "mode2.enabled",
        "limits.relay_connections",
        "limits.admin_connections",
        "limits.waiting",
        "limits.buffer_bytes",
        "limits.ws_message_bytes",
        "timeouts.handshake_secs",
        "timeouts.tls_upgrade_secs",
        "timeouts.wait_secs",
        "timeouts.connect_secs",
        "timeouts.close_secs",
        "timeouts.shutdown_grace_secs",
        "keepalive.tcp_idle_secs",
        "keepalive.tcp_interval_secs",
        "keepalive.tcp_probes",
        "keepalive.ws_ping_secs",
        "keepalive.ws_pong_timeout_secs",
        "management.listen",
        "management.cert_file",
        "management.key_file",
        "management.config_edit",
        "management.login_limit.per_account",
        "management.login_limit.per_source",
        "management.login_limit.window_secs",
        "management.login_limit.lockout_secs",
        "management.session.idle_secs",
        "management.session.max_lifetime_secs",
        "dashboard.poll_secs",
        "dashboard.query_timeout_secs",
        "recent_closed.max_entries",
        "status.listen",
        "log.output",
        "reload.settle_secs",
        "reload.poll_secs",
    ];
    let items = some(initial["items"].as_array(), "items");
    assert_eq!(
        items
            .iter()
            .map(|item| some(item["key"].as_str(), "item key"))
            .collect::<Vec<_>>(),
        keys
    );
    for item in items {
        assert!(item.get("value").is_some(), "{item}");
        assert!(
            matches!(item["reload"].as_str(), Some("dynamic" | "restart")),
            "{item}"
        );
        assert!(item["editable"].is_boolean(), "{item}");
        if let Some(value) = item["value"].as_i64() {
            let min = some(item["min"].as_i64(), "item min");
            let max = some(item["max"].as_i64(), "item max");
            assert!((min..=max).contains(&value), "{item}");
        } else {
            assert!(
                item.get("min").is_none() && item.get("max").is_none(),
                "{item}"
            );
        }
    }
    for (key, value) in [
        ("admin.argon2.memory_kib", 19456),
        ("admin.argon2.iterations", 2),
        ("admin.argon2.parallelism", 1),
        ("limits.relay_connections", 10),
        ("limits.admin_connections", 5),
        ("limits.waiting", 10),
        ("limits.buffer_bytes", 131072),
        ("limits.ws_message_bytes", 1048576),
        ("timeouts.handshake_secs", 10),
        ("timeouts.tls_upgrade_secs", 10),
        ("timeouts.wait_secs", 3600),
        ("timeouts.connect_secs", 10),
        ("timeouts.close_secs", 10),
        ("timeouts.shutdown_grace_secs", 30),
        ("keepalive.tcp_idle_secs", 60),
        ("keepalive.tcp_interval_secs", 10),
        ("keepalive.tcp_probes", 6),
        ("keepalive.ws_ping_secs", 30),
        ("keepalive.ws_pong_timeout_secs", 30),
        ("management.login_limit.per_account", 5),
        ("management.login_limit.per_source", 20),
        ("management.login_limit.window_secs", 900),
        ("management.login_limit.lockout_secs", 900),
        ("management.session.idle_secs", 1800),
        ("management.session.max_lifetime_secs", 43200),
        ("dashboard.poll_secs", 2),
        ("dashboard.query_timeout_secs", 5),
        ("recent_closed.max_entries", 5),
        ("reload.settle_secs", 1),
        ("reload.poll_secs", 5),
    ] {
        assert_eq!(config_item(&initial, key)["value"], value, "{key}");
    }
    for (prefix, port) in [
        ("relay.tcp_viewer", server.tcp_viewer_port),
        ("relay.tcp_server", server.tcp_server_port),
        ("relay.ws_viewer", server.ws_viewer_port),
        ("relay.wss_viewer", server.wss_viewer_port),
    ] {
        assert_eq!(
            config_item(&initial, &format!("{prefix}.enabled"))["value"],
            true
        );
        assert_eq!(
            config_item(&initial, &format!("{prefix}.listen"))["value"],
            format!("127.0.0.1:{port}")
        );
        assert_eq!(
            config_item(&initial, &format!("{prefix}.allow"))["value"],
            serde_json::json!(["127.0.0.1"])
        );
        assert_eq!(
            config_item(&initial, &format!("{prefix}.allow"))["editable"],
            true
        );
    }
    for key in ["mode1.enabled", "mode2.enabled"] {
        assert_eq!(config_item(&initial, key)["value"], true);
    }
    assert_eq!(
        config_item(&initial, "mode1.allow_targets")["value"],
        serde_json::json!(["127.0.0.1:1-65535"])
    );
    assert_eq!(
        config_item(&initial, "management.config_edit")["value"],
        false
    );
    assert_eq!(
        config_item(&initial, "relay.ws_viewer.allowed_origins")["value"],
        serde_json::json!(["http://127.0.0.1"])
    );
    assert_eq!(
        config_item(&initial, "relay.wss_viewer.allowed_origins")["value"],
        serde_json::json!(["http://127.0.0.1", "https://127.0.0.1"])
    );
    for (key, expected) in [
        (
            "management.listen",
            format!("127.0.0.1:{}", server.management_port),
        ),
        ("status.listen", format!("127.0.0.1:{}", server.status_port)),
        (
            "log.output",
            server.dir.join("relay.log").to_string_lossy().into_owned(),
        ),
        (
            "relay.wss_viewer.cert_file",
            tls_fixture("server-chain.pem"),
        ),
        (
            "relay.wss_viewer.key_file",
            tls_fixture("server-key-p256.pem"),
        ),
    ] {
        let actual = some(config_item(&initial, key)["value"].as_str(), key).replace('\\', "/");
        assert_eq!(actual, expected.replace('\\', "/"), "{key}");
    }
    for key in ["management.cert_file", "management.key_file"] {
        assert!(config_item(&initial, key)["value"].is_null());
        assert_eq!(config_item(&initial, key)["reload"], "dynamic");
        assert_eq!(config_item(&initial, key)["editable"], false);
    }
    assert_eq!(config_item(&initial, "limits.waiting")["max"], 65535);
    assert_eq!(config_item(&initial, "limits.waiting")["editable"], true);
    assert_eq!(
        config_item(&initial, "admin.accounts_file")["value"],
        server
            .dir
            .join("admin-accounts.toml")
            .to_string_lossy()
            .as_ref()
    );
    let rendered = initial.to_string();
    for secret in [
        ALICE_PASSWORD,
        "password_hash",
        "$argon2id$",
        "BEGIN PRIVATE KEY",
        "BEGIN CERTIFICATE",
    ] {
        assert!(!rendered.contains(secret), "{rendered}");
    }

    let before = server.config_events().len();
    let pending = original.replace("admin_connections = 5", "admin_connections = 6");
    server.config_write(&pending);
    let view = server.config_view(&admin.0);
    assert_eq!(view["file"]["state"], "pending");
    assert_eq!(
        view["file"]["config_keys"],
        serde_json::json!(["limits.admin_connections"])
    );
    assert_eq!(view["items"], initial["items"]);
    assert!(view["last_reload"].is_null());
    config_assert_revision(&view, pending.as_bytes());
    server.config_wait_event(before, "reload_applied");
    let applied = server.config_view(&admin.0);
    assert_eq!(applied["file"]["state"], "in_effect");
    assert_eq!(
        config_item(&applied, "limits.admin_connections")["value"],
        6
    );
    assert_eq!(applied["last_reload"]["result"], "applied");
    assert_eq!(applied["last_reload"]["trigger"], "file_watch");
    assert_eq!(
        applied["last_reload"]["config_keys"],
        serde_json::json!(["limits.admin_connections"])
    );
    assert_timestamp(some(applied["last_reload"]["at"].as_str(), "reload at"));

    let before = server.config_events().len();
    server.config_write("[private-marker-invalid\n");
    let invalid = server.config_view(&admin.0);
    assert_eq!(invalid["file"]["state"], "invalid");
    assert_eq!(invalid["file"]["config_keys"], serde_json::json!([]));
    assert!(!some(invalid["file"]["diagnostics"].as_array(), "diagnostics").is_empty());
    assert_eq!(invalid["items"], applied["items"]);
    assert!(
        !invalid["file"]["diagnostics"]
            .to_string()
            .contains("private-marker-invalid")
    );
    server.config_wait_event(before, "reload_rejected");
    let rejected = server.config_view(&admin.0);
    assert_eq!(rejected["last_reload"]["result"], "rejected");
    assert_eq!(rejected["last_reload"]["trigger"], "file_watch");
    assert_eq!(
        rejected["last_reload"]["config_keys"],
        serde_json::json!([])
    );

    let before = server.config_events().len();
    let restart = pending.replace(
        &format!("listen = \"127.0.0.1:{}\"", server.tcp_viewer_port),
        "listen = \"127.0.0.1:1\"",
    );
    server.config_write(&restart);
    let view = server.config_view(&admin.0);
    assert_eq!(view["file"]["state"], "restart_required");
    assert_eq!(
        view["file"]["config_keys"],
        serde_json::json!(["relay.tcp_viewer.listen"])
    );
    assert_eq!(view["items"], applied["items"]);
    server.config_wait_event(before, "reload_restart_required");
    let view = server.config_view(&admin.0);
    assert_eq!(view["last_reload"]["result"], "restart_required");
    assert_eq!(view["last_reload"]["trigger"], "file_watch");
    assert_eq!(
        view["last_reload"]["config_keys"],
        serde_json::json!(["relay.tcp_viewer.listen"])
    );

    fs::remove_file(server.config_path()).unwrap_or_else(|error| panic!("remove config: {error}"));
    let unreadable = server.config_view(&admin.0);
    assert!(unreadable["revision"].is_null());
    assert_eq!(
        unreadable["file"],
        serde_json::json!({
            "state": "unreadable", "config_keys": [], "diagnostics": []
        })
    );
    assert_eq!(unreadable["items"], applied["items"]);
    server.config_write([0xff, 0xfe]);
    let unreadable = server.config_view(&admin.0);
    config_assert_revision(&unreadable, &[0xff, 0xfe]);
    assert_eq!(unreadable["file"]["state"], "unreadable");
    assert_eq!(unreadable["file"]["diagnostics"], serde_json::json!([]));
    assert_eq!(unreadable["items"], applied["items"]);

    // Disabled omitted values are null, while an explicitly empty list stays [].
    let ws_start = some(pending.find("[relay.ws_viewer]"), "WS table");
    let mode_start = some(pending.find("[mode1]"), "mode table");
    let disabled = format!(
        "{}[relay.ws_viewer]\nenabled = false\n[relay.wss_viewer]\nenabled = false\nallowed_origins = []\ncert_file = \"certs/view-only.pem\"\n{}",
        &pending[..ws_start],
        &pending[mode_start..]
    );
    server.config_write(&disabled);
    server.config_restart();
    assert_eq!(server.query(&admin.0, "/api/v1/config").status, 401);
    let fresh = server.login();
    let view = server.config_view(&fresh.0);
    assert!(view["last_reload"].is_null());
    for key in [
        "relay.ws_viewer.listen",
        "relay.ws_viewer.allow",
        "relay.ws_viewer.allowed_origins",
        "relay.wss_viewer.listen",
        "relay.wss_viewer.allow",
        "relay.wss_viewer.key_file",
    ] {
        assert!(config_item(&view, key)["value"].is_null(), "{key}");
        assert_eq!(config_item(&view, key)["editable"], false, "{key}");
    }
    assert_eq!(
        config_item(&view, "relay.wss_viewer.allowed_origins")["value"],
        serde_json::json!([])
    );
    let cert = config_item(&view, "relay.wss_viewer.cert_file");
    assert_eq!(
        cert["value"],
        server
            .dir
            .join("certs/view-only.pem")
            .to_string_lossy()
            .as_ref()
    );
    assert_eq!(cert["editable"], false);
    // A disabled TLS path need not exist. Its display-only presence changes
    // without publishing runtime settings, a reload event, or reload history.
    let before = server.config_events().len();
    server.config_write(disabled.replace("certs/view-only.pem", "certs/updated-view.pem"));
    server.config_wait_event(before, "reload_change_detected");
    std::thread::sleep(Duration::from_secs(6));
    let updated = server.config_view(&fresh.0);
    assert_eq!(updated["file"]["state"], "in_effect");
    assert_eq!(
        config_item(&updated, "relay.wss_viewer.cert_file")["value"],
        server
            .dir
            .join("certs/updated-view.pem")
            .to_string_lossy()
            .as_ref()
    );
    assert!(config_item(&updated, "relay.wss_viewer.key_file")["value"].is_null());
    assert!(updated["last_reload"].is_null());
    config_assert_no_success(&server.config_events()[before..]);
}

/// spec: management.md:MGT-031
/// spec: operations.md:OPS-060
#[test]
fn config_edit_preserves_document_applies_once_and_survives_restart() {
    let _lock = lock_ports();
    for newline in ["\n", "\r\n"] {
        let mut server = config_server("config-preservation", true);
        let text = String::from_utf8(server.config_bytes())
            .unwrap_or_else(|error| panic!("config UTF-8: {error}"));
        let original = format!("# preserve leading comment\n{text}")
            .replace(
                "relay_connections = 10",
                "relay_connections   = 10 # preserve scalar",
            )
            .replace("waiting = 10", "waiting = 10 # untouched waiting")
            .replace('\n', newline);
        server.config_write(&original);
        server.config_restart();
        let admin = server.login();
        let before = server.config_events().len();
        // The edit incorporates a valid external pending field, but audits only
        // requested fields whose file values actually changed.
        let pending = original.replace("admin_connections = 5", "admin_connections = 6");
        server.config_write(&pending);
        let view = server.config_view(&admin.0);
        assert_eq!(view["file"]["state"], "pending");
        let reply = server.config_patch(
            &admin,
            &view["revision"],
            serde_json::json!({
                "limits.relay_connections": 12,
                "limits.buffer_bytes": 262144,
                "timeouts.close_secs": 2,
                "keepalive.tcp_idle_secs": 90
            }),
        );
        assert_eq!(reply.status, 200, "{}", reply.body);
        let response = config_json(&reply);
        assert_eq!(response["result"], "applied");
        assert_timestamp(some(response["as_of"].as_str(), "patch as_of"));
        let applied_keys = serde_json::json!([
            "limits.relay_connections",
            "limits.admin_connections",
            "limits.buffer_bytes",
            "timeouts.close_secs",
            "keepalive.tcp_idle_secs"
        ]);
        assert_eq!(response["applied"], applied_keys);
        let saved = server.config_bytes();
        config_assert_revision(&response, &saved);
        let saved_text =
            std::str::from_utf8(&saved).unwrap_or_else(|error| panic!("saved UTF-8: {error}"));
        assert!(saved_text.contains("# preserve leading comment"));
        assert!(saved_text.contains("relay_connections   = 12 # preserve scalar"));
        assert!(saved_text.contains("waiting = 10 # untouched waiting"));
        assert!(saved_text.contains("admin_connections = 6"));
        let original_limits = some(pending.split_once("[limits]"), "original limits").1;
        let original_after_limits =
            some(original_limits.split_once("[recent_closed]"), "next table").1;
        let saved_after_limits =
            some(saved_text.split_once("[recent_closed]"), "saved next table").1;
        let preserved_tail = original_after_limits.trim_end_matches(['\r', '\n']);
        assert!(
            saved_after_limits.starts_with(preserved_tail),
            "{saved_text}"
        );
        assert!(saved_text.contains(&format!("buffer_bytes = 262144{newline}")));
        assert!(
            some(saved_text.find("buffer_bytes = 262144"), "inserted key")
                < some(saved_text.find("[recent_closed]"), "following table")
        );
        for (table, key) in [
            ("[timeouts]", "close_secs = 2"),
            ("[keepalive]", "tcp_idle_secs = 90"),
        ] {
            let position = some(saved_text.find(table), "inserted table");
            assert!(position > some(saved_text.find("[log]"), "last original table"));
            assert!(saved_text[position..].contains(key), "{saved_text}");
        }
        if newline == "\r\n" {
            assert!(
                !saved_text.replace("\r\n", "").contains('\n'),
                "{saved_text:?}"
            );
        } else {
            assert!(!saved_text.contains('\r'), "{saved_text:?}");
        }
        let effective = server.config_view(&admin.0);
        assert_eq!(effective["file"]["state"], "in_effect");
        assert_eq!(effective["revision"], response["revision"]);
        for (key, value) in [
            ("limits.relay_connections", 12),
            ("limits.admin_connections", 6),
            ("limits.buffer_bytes", 262144),
            ("timeouts.close_secs", 2),
            ("keepalive.tcp_idle_secs", 90),
        ] {
            assert_eq!(config_item(&effective, key)["value"], value, "{key}");
        }
        assert_eq!(effective["last_reload"]["trigger"], "config_edit");
        assert_eq!(effective["last_reload"]["result"], "applied");
        assert_eq!(effective["last_reload"]["config_keys"], applied_keys);
        assert_eq!(effective["last_reload"]["at"], response["as_of"]);
        server.config_wait_event(before, "config_edited");
        // Cross both the native-watch quiet window and a fallback poll cycle.
        std::thread::sleep(Duration::from_secs(6));
        let events = server.config_events();
        let edits: Vec<_> = events[before..]
            .iter()
            .filter(|e| e["event"] == "config_edited")
            .collect();
        let reloads: Vec<_> = events[before..]
            .iter()
            .filter(|e| e["event"] == "reload_applied")
            .collect();
        assert_eq!(edits.len(), 1, "{events:?}");
        assert_eq!(reloads.len(), 1, "{events:?}");
        assert_eq!(reloads[0]["trigger"], "config_edit");
        assert_eq!(reloads[0]["applied"], applied_keys);
        config_assert_audit(
            edits[0],
            None,
            serde_json::json!([
                "keepalive.tcp_idle_secs",
                "limits.buffer_bytes",
                "limits.relay_connections",
                "timeouts.close_secs"
            ]),
        );
        assert_eq!(edits[0]["revision"], response["revision"]);
        let reload_position = some(
            events[before..]
                .iter()
                .position(|e| e["event"] == "reload_applied"),
            "reload position",
        );
        let edit_position = some(
            events[before..]
                .iter()
                .position(|e| e["event"] == "config_edited"),
            "edit position",
        );
        assert!(reload_position < edit_position);
        assert_eq!(
            server.config_view(&admin.0)["last_reload"],
            effective["last_reload"]
        );
        server.config_restart();
        assert_eq!(server.query(&admin.0, "/api/v1/config").status, 401);
        let fresh = server.login();
        let restarted = server.config_view(&fresh.0);
        assert_eq!(restarted["items"], effective["items"]);
        assert_eq!(restarted["revision"], response["revision"]);
        assert!(restarted["last_reload"].is_null());
        assert_eq!(server.config_bytes(), saved);
    }
}

/// spec: management.md:MGT-033
#[test]
fn stale_config_revision_refuses_other_admin_and_external_comment_changes() {
    let _lock = lock_ports();
    let mut server = config_server("config-conflict", true);
    let accounts_path = server.dir.join("admin-accounts.toml");
    let accounts = fs::read_to_string(&accounts_path)
        .unwrap_or_else(|error| panic!("read two-admin fixture: {error}"));
    let bob = accounts
        .replace("version = 1", "")
        .replace("name = \"alice\"", "name = \"bob\"");
    fs::write(&accounts_path, format!("{accounts}\n{bob}"))
        .unwrap_or_else(|error| panic!("write two-admin fixture: {error}"));
    server.config_restart();
    let first = server.login();
    let second = server.login_as("bob");
    assert_ne!(first.0, second.0);
    let first_view = server.config_view(&first.0);
    let second_view = server.config_view(&second.0);
    assert_eq!(first_view["revision"], second_view["revision"]);
    let changed = server.config_patch(
        &first,
        &first_view["revision"],
        serde_json::json!({
            "limits.admin_connections": 6
        }),
    );
    assert_eq!(changed.status, 200, "{}", changed.body);
    let saved = server.config_bytes();
    let effective = server.config_view(&first.0);
    let before = server.config_events().len();
    let stale = server.config_patch(
        &second,
        &second_view["revision"],
        serde_json::json!({
            "limits.admin_connections": 7
        }),
    );
    config_assert_error(&stale, 409, "config_conflict");
    assert_eq!(server.config_bytes(), saved);
    let view = server.config_view(&first.0);
    assert_eq!(view["items"], effective["items"]);
    assert_eq!(view["last_reload"], effective["last_reload"]);
    let rejected = server.config_wait_event(before, "config_edit_rejected");
    config_assert_audit_account(
        &rejected,
        "bob",
        Some("config_conflict"),
        serde_json::json!(["limits.admin_connections"]),
    );
    config_assert_no_success(&server.config_events()[before..]);

    let mut comment_only = saved.clone();
    comment_only.extend_from_slice(b"\n# external comment only\n");
    server.config_write(&comment_only);
    let before = server.config_events().len();
    let stale = server.config_patch(
        &second,
        &effective["revision"],
        serde_json::json!({
            "limits.admin_connections": 8
        }),
    );
    config_assert_error(&stale, 409, "config_conflict");
    assert_eq!(server.config_bytes(), comment_only);
    let current = server.config_view(&second.0);
    assert_ne!(current["revision"], effective["revision"]);
    config_assert_revision(&current, &comment_only);
    assert_eq!(current["items"], effective["items"]);
    assert_eq!(current["last_reload"], effective["last_reload"]);
    config_assert_audit_account(
        &server.config_wait_event(before, "config_edit_rejected"),
        "bob",
        Some("config_conflict"),
        serde_json::json!(["limits.admin_connections"]),
    );
    config_assert_no_success(&server.config_events()[before..]);
}

/// spec: management.md:MGT-043 partial
#[test]
fn config_conflict_retry_with_fresh_revision_applies_and_preserves_both_changes() {
    let _lock = lock_ports();
    let server = config_server("config-conflict-retry", true);
    let admin = server.login();
    let initial_view = server.config_view(&admin.0);
    let initial_items = initial_view["items"].clone();
    let initial_last_reload = initial_view["last_reload"].clone();

    // 1. External change introduces a new comment into the file.
    let mut modified = server.config_bytes();
    modified.extend_from_slice(b"\n# external operator note\n");
    server.config_write(&modified);

    let before = server.config_events().len();

    // 2. PATCH using the stale revision is refused with 409 config_conflict.
    let stale_patch = server.config_patch(
        &admin,
        &initial_view["revision"],
        serde_json::json!({
            "limits.admin_connections": 12
        }),
    );
    config_assert_error(&stale_patch, 409, "config_conflict");

    // File bytes, runtime values, and last_reload remain unchanged by the rejected request.
    assert_eq!(server.config_bytes(), modified);
    let mid_view = server.config_view(&admin.0);
    assert_eq!(mid_view["items"], initial_items);
    assert_eq!(mid_view["last_reload"], initial_last_reload);

    // 3. GET fetches fresh revision, values, and file state.
    let fresh_view = server.config_view(&admin.0);
    assert_ne!(fresh_view["revision"], initial_view["revision"]);
    config_assert_revision(&fresh_view, &modified);
    assert_eq!(fresh_view["file"]["state"], "in_effect");
    assert_eq!(fresh_view["items"], initial_items);

    // 4. PATCH using the fresh revision succeeds with 200 applied.
    let applied_patch = server.config_patch(
        &admin,
        &fresh_view["revision"],
        serde_json::json!({
            "limits.admin_connections": 12
        }),
    );
    assert_eq!(applied_patch.status, 200, "{}", applied_patch.body);
    let response: serde_json::Value = serde_json::from_str(&applied_patch.body)
        .unwrap_or_else(|error| panic!("response json: {error}"));
    assert_eq!(response["result"], "applied");
    assert_eq!(
        response["applied"],
        serde_json::json!(["limits.admin_connections"])
    );

    // 5. File contains both the external comment and the new edit.
    let final_bytes = server.config_bytes();
    let final_text = String::from_utf8_lossy(&final_bytes);
    assert!(
        final_text.contains("# external operator note"),
        "missing external change: {final_text}"
    );
    assert!(
        final_text.contains("admin_connections = 12"),
        "missing new edit: {final_text}"
    );

    // 6. Events: exactly one config_edit_rejected, then reload_applied(trigger=config_edit) -> config_edited.
    server.config_wait_event(before, "config_edited");
    let events = server.config_events();
    let window = &events[before..];

    let rejections: Vec<_> = window
        .iter()
        .filter(|e| e["event"] == "config_edit_rejected")
        .collect();
    let reloads: Vec<_> = window
        .iter()
        .filter(|e| e["event"] == "reload_applied")
        .collect();
    let edits: Vec<_> = window
        .iter()
        .filter(|e| e["event"] == "config_edited")
        .collect();

    assert_eq!(rejections.len(), 1, "{window:?}");
    assert_eq!(rejections[0]["reason"], "config_conflict");
    assert_eq!(
        rejections[0]["config_keys"],
        serde_json::json!(["limits.admin_connections"])
    );

    assert_eq!(reloads.len(), 1, "{window:?}");
    assert_eq!(reloads[0]["trigger"], "config_edit");
    assert_eq!(
        reloads[0]["applied"],
        serde_json::json!(["limits.admin_connections"])
    );

    assert_eq!(edits.len(), 1, "{window:?}");
    assert_eq!(
        edits[0]["config_keys"],
        serde_json::json!(["limits.admin_connections"])
    );
    assert_eq!(edits[0]["revision"], response["revision"]);

    let rejection_pos = some(
        window
            .iter()
            .position(|e| e["event"] == "config_edit_rejected"),
        "rejection pos",
    );
    let reload_pos = some(
        window.iter().position(|e| e["event"] == "reload_applied"),
        "reload pos",
    );
    let edit_pos = some(
        window.iter().position(|e| e["event"] == "config_edited"),
        "edit pos",
    );

    assert!(
        rejection_pos < reload_pos,
        "rejection should precede reload: {window:?}"
    );
    assert!(
        reload_pos < edit_pos,
        "reload should precede config_edited: {window:?}"
    );

    let after_view = server.config_view(&admin.0);
    assert_eq!(after_view["revision"], response["revision"]);
    assert_eq!(
        config_item(&after_view, "limits.admin_connections")["value"],
        12
    );
    assert_eq!(after_view["last_reload"]["trigger"], "config_edit");
    assert_eq!(after_view["last_reload"]["result"], "applied");
    assert_eq!(
        after_view["last_reload"]["config_keys"],
        serde_json::json!(["limits.admin_connections"])
    );
}

/// spec: management.md:MGT-034
/// spec: operations.md:OPS-063
#[test]
fn restart_pending_file_blocks_config_patch_and_reports_only_restart_keys() {
    let _lock = lock_ports();
    let server = config_server("config-restart-pending", true);
    let admin = server.login();
    let initial = server.config_view(&admin.0);
    let holder = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|error| panic!("reserve restart-only port: {error}"));
    let replacement_port = holder
        .local_addr()
        .unwrap_or_else(|error| panic!("replacement address: {error}"))
        .port();
    let original = String::from_utf8(server.config_bytes())
        .unwrap_or_else(|error| panic!("config UTF-8: {error}"));
    let candidate = original
        .replace(
            &format!("listen = \"127.0.0.1:{}\"", server.tcp_viewer_port),
            &format!("listen = \"127.0.0.1:{replacement_port}\""),
        )
        .replace("admin_connections = 5", "admin_connections = 6");
    let before = server.config_events().len();
    server.config_write(&candidate);
    let view = server.config_view(&admin.0);
    assert_eq!(view["file"]["state"], "restart_required");
    assert_eq!(
        view["file"]["config_keys"],
        serde_json::json!(["relay.tcp_viewer.listen"])
    );
    assert_eq!(view["items"], initial["items"]);
    let watch = server.config_wait_event(before, "reload_restart_required");
    assert_eq!(
        watch["config_keys"],
        serde_json::json!(["relay.tcp_viewer.listen"])
    );
    let view = server.config_view(&admin.0);
    let last_before_patch = view["last_reload"].clone();
    let reply = server.config_patch(
        &admin,
        &view["revision"],
        serde_json::json!({
            "limits.relay_connections": 12
        }),
    );
    let error = config_assert_error(&reply, 409, "config_restart_pending");
    assert_eq!(
        error["error"]["config_keys"],
        serde_json::json!(["relay.tcp_viewer.listen"])
    );
    assert_eq!(server.config_bytes(), candidate.as_bytes());
    assert_eq!(
        server.config_view(&admin.0)["last_reload"],
        last_before_patch
    );
    config_assert_audit(
        &server.config_wait_event(before, "config_edit_rejected"),
        Some("config_restart_pending"),
        serde_json::json!(["limits.relay_connections"]),
    );
    let refused = server.config_view(&admin.0);
    assert_eq!(refused["items"], initial["items"]);
    assert_eq!(refused["last_reload"]["result"], "restart_required");
    assert_eq!(refused["last_reload"]["trigger"], "file_watch");
    assert_eq!(
        refused["last_reload"]["config_keys"],
        serde_json::json!(["relay.tcp_viewer.listen"])
    );
    assert_eq!(server.config_bytes(), candidate.as_bytes());
    let summary = server.query(&admin.0, "/api/v1/summary").json();
    assert_eq!(
        some(
            some(summary.obj("usage"), "usage").obj("admin_connections"),
            "admin usage"
        )
        .num("limit"),
        Some(5)
    );
    let _old_listener = server.connect_tcp_viewer();
    config_assert_no_success(&server.config_events()[before..]);
}

/// spec: management.md:MGT-039
#[test]
fn same_file_value_patch_is_noop_even_with_pending_runtime_delta() {
    let _lock = lock_ports();
    let server = config_server("config-noop", true);
    let admin = server.login();
    let initial = server.config_view(&admin.0);
    for pending in [false, true] {
        if pending {
            let text = String::from_utf8(server.config_bytes())
                .unwrap_or_else(|error| panic!("config UTF-8: {error}"));
            server.config_write(text.replace("admin_connections = 5", "admin_connections = 6"));
        }
        let view = server.config_view(&admin.0);
        assert_eq!(
            view["file"]["state"],
            if pending { "pending" } else { "in_effect" }
        );
        let bytes = server.config_bytes();
        let modified = fs::metadata(server.config_path())
            .and_then(|m| m.modified())
            .unwrap_or_else(|error| panic!("config mtime: {error}"));
        let before = server.config_events().len();
        let reply = server.config_patch(
            &admin,
            &view["revision"],
            serde_json::json!({
                "limits.admin_connections": if pending { 6 } else { 5 }
            }),
        );
        assert_eq!(reply.status, 200, "{}", reply.body);
        let response = config_json(&reply);
        assert_eq!(response["result"], "applied");
        assert_eq!(response["applied"], serde_json::json!([]));
        assert_eq!(response["revision"], view["revision"]);
        assert_eq!(server.config_bytes(), bytes);
        assert_eq!(
            fs::metadata(server.config_path())
                .and_then(|m| m.modified())
                .unwrap_or_else(|error| panic!("config mtime after: {error}")),
            modified
        );
        let after = server.config_view(&admin.0);
        assert_eq!(after["items"], initial["items"]);
        assert_eq!(after["last_reload"], view["last_reload"]);
        config_assert_no_success(&server.config_events()[before..]);
        if pending {
            let event = server.config_wait_event(before, "reload_applied");
            assert_eq!(event["trigger"], "file_watch");
            assert_eq!(
                event["applied"],
                serde_json::json!(["limits.admin_connections"])
            );
            let applied = server.config_view(&admin.0);
            assert_eq!(
                config_item(&applied, "limits.admin_connections")["value"],
                6
            );
            assert_eq!(applied["last_reload"]["trigger"], "file_watch");
            assert!(
                server.config_events()[before..]
                    .iter()
                    .all(|e| e["event"] != "config_edited")
            );
        } else {
            std::thread::sleep(Duration::from_secs(6));
            config_assert_no_success(&server.config_events()[before..]);
            let unchanged = server.config_view(&admin.0);
            assert_eq!(unchanged["revision"], view["revision"]);
            assert_eq!(unchanged["last_reload"], view["last_reload"]);
            assert_eq!(unchanged["items"], view["items"]);
        }
    }
}

/// spec: management.md:MGT-044
#[test]
fn config_edit_recovers_only_after_disabled_restart_or_file_repair() {
    let _lock = lock_ports();
    let mut server = config_server("config-recovery", false);
    let disabled_admin = server.login();
    let disabled = server.config_view(&disabled_admin.0);
    let reply = server.config_patch(
        &disabled_admin,
        &disabled["revision"],
        serde_json::json!({
            "limits.admin_connections": 6
        }),
    );
    config_assert_error(&reply, 403, "config_edit_disabled");
    let text = String::from_utf8(server.config_bytes())
        .unwrap_or_else(|error| panic!("config UTF-8: {error}"));
    server.config_write(text.replace("config_edit = false", "config_edit = true"));
    let enabled_file = server.config_view(&disabled_admin.0);
    assert_eq!(enabled_file["edit_enabled"], false);
    assert_eq!(enabled_file["file"]["state"], "restart_required");
    let reply = server.config_patch(
        &disabled_admin,
        &enabled_file["revision"],
        serde_json::json!({
            "limits.admin_connections": 6
        }),
    );
    config_assert_error(&reply, 403, "config_edit_disabled");
    server.config_restart();
    assert_eq!(
        server.query(&disabled_admin.0, "/api/v1/config").status,
        401
    );
    let admin = server.login();
    let enabled = server.config_view(&admin.0);
    assert_eq!(enabled["edit_enabled"], true);
    assert_eq!(enabled["file"]["state"], "in_effect");
    let reply = server.config_patch(
        &admin,
        &enabled["revision"],
        serde_json::json!({
            "limits.admin_connections": 6
        }),
    );
    assert_eq!(reply.status, 200, "{}", reply.body);

    for damage in ["restart", "invalid", "unreadable"] {
        let healthy = server.config_bytes();
        match damage {
            "restart" => {
                let text = std::str::from_utf8(&healthy)
                    .unwrap_or_else(|error| panic!("healthy UTF-8: {error}"));
                server.config_write(text.replace(
                    &format!("listen = \"127.0.0.1:{}\"", server.tcp_viewer_port),
                    "listen = \"127.0.0.1:1\"",
                ));
            }
            "invalid" => server.config_write("[broken\n"),
            "unreadable" => {
                fs::remove_file(server.config_path())
                    .unwrap_or_else(|error| panic!("remove current file: {error}"));
            }
            _ => unreachable!(),
        }
        let damaged = server.config_view(&admin.0);
        let revision = if damaged["revision"].is_null() {
            serde_json::json!(config_revision(&healthy))
        } else {
            damaged["revision"].clone()
        };
        let reply = server.config_patch(
            &admin,
            &revision,
            serde_json::json!({
                "timeouts.close_secs": 3
            }),
        );
        config_assert_error(
            &reply,
            409,
            if damage == "restart" {
                "config_restart_pending"
            } else {
                "config_file_invalid"
            },
        );
        server.config_write(&healthy);
        let repaired = server.config_view(&admin.0);
        assert_eq!(repaired["file"]["state"], "in_effect");
        config_assert_revision(&repaired, &healthy);
        // A new explicit request uses the revision returned by the repaired GET.
        let next = if config_item(&repaired, "limits.admin_connections")["value"] == 6 {
            7
        } else {
            6
        };
        let reply = server.config_patch(
            &admin,
            &repaired["revision"],
            serde_json::json!({
                "limits.admin_connections": next
            }),
        );
        assert_eq!(reply.status, 200, "{damage}: {}", reply.body);
        let after = server.config_view(&admin.0);
        assert_eq!(
            config_item(&after, "limits.admin_connections")["value"],
            next
        );
        assert_eq!(after["file"]["state"], "in_effect");
        assert_eq!(after["last_reload"]["trigger"], "config_edit");
    }
}
