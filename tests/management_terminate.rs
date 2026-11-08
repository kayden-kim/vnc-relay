//! Real-process terminate coverage. No failure injection or test-only product paths.
//! Binary HTTP, transport and audit paths supplement the in-process failure
//! coverage in adapters/tests/management.rs. The history test covers all three
//! RLY-020 close causes together with list/count consistency and permit recovery.

use std::fs;
use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use vnc_relay_adapters::accounts_file::Argon2Hasher;
use vnc_relay_app::ports::PasswordHasher;
use vnc_relay_domain::account::{Argon2Params, Password};

const BIN: &str = env!("CARGO_BIN_EXE_vnc-relay");
const LIMIT: Duration = Duration::from_secs(15);
const POLL: Duration = Duration::from_millis(10);
const PASSWORD: &str = "terminate-test-password";
const PAYLOAD: &[u8] = b"PRIVATE_RFB_PAYLOAD_terminator_8675309";
static SERIAL: AtomicU64 = AtomicU64::new(0);
static PORTS: Mutex<()> = Mutex::new(());

fn must<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("operation failed: {error:?}"))
}

// The root package has no JSON dev dependency. Keep this response-only parser
// local, as management_query.rs does, rather than adding a product dependency.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Json {
    Object(Vec<(String, Json)>),
    Array(Vec<Json>),
    String(String),
    Number(u64),
    Bool(bool),
    Null,
}

impl Json {
    fn get(&self, key: &str) -> &Self {
        match self {
            Self::Object(fields) => fields
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value)
                .unwrap_or(&Self::Null),
            _ => &Self::Null,
        }
    }

    fn text(&self) -> &str {
        match self {
            Self::String(value) => value,
            _ => panic!("expected JSON string: {self:?}"),
        }
    }

    fn number(&self) -> u64 {
        match self {
            Self::Number(value) => *value,
            _ => panic!("expected JSON integer: {self:?}"),
        }
    }

    fn items(&self) -> &[Self] {
        match self {
            Self::Array(values) => values,
            _ => panic!("expected JSON array: {self:?}"),
        }
    }
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn whitespace(&mut self) {
        while self
            .bytes
            .get(self.pos)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.pos += 1;
        }
    }

    fn take(&mut self, byte: u8) {
        self.whitespace();
        assert_eq!(self.bytes.get(self.pos), Some(&byte));
        self.pos += 1;
    }

    fn string(&mut self) -> String {
        self.take(b'"');
        let mut result = String::new();
        loop {
            let byte = self.bytes[self.pos];
            self.pos += 1;
            match byte {
                b'"' => return result,
                b'\\' => {
                    let escaped = self.bytes[self.pos];
                    self.pos += 1;
                    match escaped {
                        b'"' | b'\\' | b'/' => result.push(char::from(escaped)),
                        b'n' => result.push('\n'),
                        b'r' => result.push('\r'),
                        b't' => result.push('\t'),
                        b'b' => result.push('\x08'),
                        b'f' => result.push('\x0c'),
                        b'u' => {
                            let digits =
                                must(std::str::from_utf8(&self.bytes[self.pos..self.pos + 4]));
                            let code = must(u32::from_str_radix(digits, 16));
                            result.push(
                                char::from_u32(code)
                                    .unwrap_or_else(|| panic!("bad unicode escape")),
                            );
                            self.pos += 4;
                        }
                        _ => panic!("invalid JSON escape"),
                    }
                }
                0..=31 => panic!("invalid JSON string"),
                32..=127 => result.push(char::from(byte)),
                _ => {
                    let rest = must(std::str::from_utf8(&self.bytes[self.pos - 1..]));
                    let ch = rest
                        .chars()
                        .next()
                        .unwrap_or_else(|| panic!("missing UTF-8 char"));
                    self.pos += ch.len_utf8() - 1;
                    result.push(ch);
                }
            }
        }
    }

    fn value(&mut self) -> Json {
        self.whitespace();
        match self.bytes[self.pos] {
            b'{' => {
                self.pos += 1;
                let mut fields = Vec::new();
                self.whitespace();
                while self.bytes[self.pos] != b'}' {
                    if !fields.is_empty() {
                        self.take(b',');
                    }
                    let name = self.string();
                    self.take(b':');
                    fields.push((name, self.value()));
                    self.whitespace();
                }
                self.pos += 1;
                Json::Object(fields)
            }
            b'[' => {
                self.pos += 1;
                let mut values = Vec::new();
                self.whitespace();
                while self.bytes[self.pos] != b']' {
                    if !values.is_empty() {
                        self.take(b',');
                    }
                    values.push(self.value());
                    self.whitespace();
                }
                self.pos += 1;
                Json::Array(values)
            }
            b'"' => Json::String(self.string()),
            b'0'..=b'9' => {
                let start = self.pos;
                while self.bytes.get(self.pos).is_some_and(u8::is_ascii_digit) {
                    self.pos += 1;
                }
                Json::Number(must(
                    must(std::str::from_utf8(&self.bytes[start..self.pos])).parse(),
                ))
            }
            _ => {
                for (literal, value) in [
                    ("null", Json::Null),
                    ("true", Json::Bool(true)),
                    ("false", Json::Bool(false)),
                ] {
                    if self.bytes[self.pos..].starts_with(literal.as_bytes()) {
                        self.pos += literal.len();
                        return value;
                    }
                }
                panic!("invalid JSON value at {}", self.pos);
            }
        }
    }
}

fn json(text: &str) -> Json {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        pos: 0,
    };
    let result = parser.value();
    parser.whitespace();
    assert_eq!(parser.pos, parser.bytes.len(), "trailing JSON input");
    result
}

struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

impl Reply {
    fn read(mut stream: TcpStream) -> Self {
        let mut bytes = String::new();
        must(stream.read_to_string(&mut bytes));
        let (head, body) = bytes
            .split_once("\r\n\r\n")
            .unwrap_or_else(|| panic!("missing HTTP response: {bytes:?}"));
        let mut lines = head.lines();
        let status = must(
            lines
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap_or_else(|| panic!("missing HTTP status"))
                .parse(),
        );
        let headers = lines
            .filter_map(|line| line.split_once(':'))
            .map(|(key, value)| (key.to_ascii_lowercase(), value.trim().to_owned()))
            .collect();
        Self {
            status,
            headers,
            body: body.to_owned(),
        }
    }

    fn header(&self, key: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    }

    fn json(&self) -> Json {
        json(&self.body)
    }

    fn error(&self, status: u16, code: &str) {
        assert_eq!(self.status, status, "{}", self.body);
        assert_eq!(self.json().get("error").get("code").text(), code);
        assert_eq!(self.header("cache-control"), Some("no-store"));
        assert_eq!(self.json().get("closed_session"), &Json::Null);
    }
}

#[derive(Clone)]
struct Admin {
    cookie: String,
    csrf: String,
}

fn connect(port: u16) -> TcpStream {
    let stream = must(TcpStream::connect_timeout(
        &SocketAddr::from(([127, 0, 0, 1], port)),
        LIMIT,
    ));
    must(stream.set_read_timeout(Some(LIMIT)));
    must(stream.set_write_timeout(Some(LIMIT)));
    stream
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("crates/adapters/tests/fixtures/tls")
        .join(name)
}

fn tls_config() -> ClientConfig {
    let mut roots = RootCertStore::empty();
    for cert in must(CertificateDer::pem_file_iter(fixture("root-ca.pem"))) {
        must(roots.add(must(cert)));
    }
    must(
        ClientConfig::builder_with_provider(
            Arc::new(rustls::crypto::aws_lc_rs::default_provider()),
        )
        .with_protocol_versions(&[&rustls::version::TLS13, &rustls::version::TLS12]),
    )
    .with_root_certificates(roots)
    .with_no_client_auth()
}

enum Wire {
    Tcp(TcpStream),
    Tls(Box<StreamOwned<ClientConnection, TcpStream>>),
}

impl Read for Wire {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Tcp(s) => s.read(bytes),
            Self::Tls(s) => s.read(bytes),
        }
    }
}

impl Write for Wire {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        match self {
            Self::Tcp(s) => s.write(bytes),
            Self::Tls(s) => s.write(bytes),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Tcp(s) => s.flush(),
            Self::Tls(s) => s.flush(),
        }
    }
}

struct WebSocket {
    wire: Wire,
}

impl WebSocket {
    fn open(port: u16, tls: bool) -> Self {
        let tcp = connect(port);
        let mut wire = if tls {
            let connection = must(ClientConnection::new(
                Arc::new(tls_config()),
                must(ServerName::try_from("localhost")),
            ));
            Wire::Tls(Box::new(StreamOwned::new(connection, tcp)))
        } else {
            Wire::Tcp(tcp)
        };
        must(wire.write_all(b"GET /relay HTTP/1.1\r\nHost: localhost\r\nOrigin: http://127.0.0.1\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"));
        must(wire.flush());
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            must(wire.read_exact(&mut byte));
            head.push(byte[0]);
            assert!(head.len() < 16384);
        }
        assert!(
            head.starts_with(b"HTTP/1.1 101 "),
            "{}",
            String::from_utf8_lossy(&head)
        );
        let mut ws = Self { wire };
        assert_eq!(ws.binary(), b"RFB 000.000\n");
        ws
    }

    fn send(&mut self, opcode: u8, bytes: &[u8]) {
        assert!(bytes.len() <= usize::from(u16::MAX));
        let mut frame = vec![0x80 | opcode];
        if bytes.len() < 126 {
            frame.push(0x80 | must(u8::try_from(bytes.len())));
        } else {
            frame.push(0xfe);
            frame.extend_from_slice(&must(u16::try_from(bytes.len())).to_be_bytes());
        }
        let mask = [0x13, 0x57, 0x9b, 0xdf];
        frame.extend_from_slice(&mask);
        frame.extend(
            bytes
                .iter()
                .enumerate()
                .map(|(index, byte)| byte ^ mask[index % 4]),
        );
        must(self.wire.write_all(&frame));
        must(self.wire.flush());
    }

    fn frame(&mut self) -> (u8, Vec<u8>) {
        loop {
            let mut head = [0; 2];
            must(self.wire.read_exact(&mut head));
            assert_eq!(head[1] & 0x80, 0, "server frames are unmasked");
            let len = match head[1] & 0x7f {
                126 => {
                    let mut n = [0; 2];
                    must(self.wire.read_exact(&mut n));
                    usize::from(u16::from_be_bytes(n))
                }
                127 => {
                    let mut n = [0; 8];
                    must(self.wire.read_exact(&mut n));
                    must(usize::try_from(u64::from_be_bytes(n)))
                }
                n => usize::from(n),
            };
            assert!(len <= 1024 * 1024, "bounded test frame");
            let mut payload = vec![0; len];
            must(self.wire.read_exact(&mut payload));
            match head[0] & 15 {
                9 => self.send(10, &payload),
                10 => {}
                opcode => return (opcode, payload),
            }
        }
    }

    fn binary(&mut self) -> Vec<u8> {
        let (opcode, payload) = self.frame();
        assert_eq!(opcode, 2);
        payload
    }

    fn close(&mut self, code: u16, echo: bool) {
        let (opcode, payload) = self.frame();
        assert_eq!(opcode, 8);
        assert!(payload.len() >= 2);
        assert_eq!(u16::from_be_bytes([payload[0], payload[1]]), code);
        if echo {
            self.send(8, &payload);
        }
    }
}

enum Viewer {
    Tcp(TcpStream),
    Ws(WebSocket),
}

impl Viewer {
    fn send(&mut self, bytes: &[u8]) {
        match self {
            Self::Tcp(s) => must(s.write_all(bytes)),
            Self::Ws(s) => s.send(2, bytes),
        }
    }
    fn receive(&mut self, expected: &[u8]) {
        match self {
            Self::Tcp(s) => {
                let mut bytes = vec![0; expected.len()];
                must(s.read_exact(&mut bytes));
                assert_eq!(bytes, expected);
            }
            Self::Ws(s) => {
                let mut bytes = Vec::new();
                while bytes.len() < expected.len() {
                    bytes.extend(s.binary());
                }
                assert_eq!(bytes, expected);
            }
        }
    }
    fn websocket(&mut self) -> &mut WebSocket {
        match self {
            Self::Ws(ws) => ws,
            Self::Tcp(_) => panic!("expected websocket"),
        }
    }
    fn acknowledge_admin_close(&mut self) {
        match self {
            Self::Ws(ws) => ws.close(4005, true),
            Self::Tcp(tcp) => {
                let mut byte = [0];
                assert_eq!(must(tcp.read(&mut byte)), 0);
            }
        }
    }
}

struct Pair {
    viewer: Viewer,
    target: TcpStream,
    sid: String,
}

impl Pair {
    fn transfer(&mut self) {
        self.viewer.send(PAYLOAD);
        let mut bytes = vec![0; PAYLOAD.len()];
        must(self.target.read_exact(&mut bytes));
        assert_eq!(bytes, PAYLOAD);
        must(self.target.write_all(b"target-reply"));
        self.viewer.receive(b"target-reply");
    }
}

struct Server {
    dir: PathBuf,
    child: Child,
    ports: [u16; 6],
}

impl Server {
    fn start(entries: u32, close_secs: u32, upgrade_secs: u32) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "vnc-relay-terminate-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        must(fs::create_dir_all(&dir));
        let password = must(Password::parse(PASSWORD.to_owned()));
        let hash = must(Argon2Hasher.hash(
            &password,
            Argon2Params {
                memory_kib: 19_456,
                iterations: 2,
                parallelism: 1,
            },
        ));
        must(fs::write(
            dir.join("accounts.toml"),
            format!(
                "version = 1\n[[account]]\nname = \"alice\"\npassword_hash = \"{}\"\n[[account]]\nname = \"bob\"\npassword_hash = \"{}\"\n",
                hash.as_str(),
                hash.as_str()
            ),
        ));
        let holders: Vec<_> = (0..6)
            .map(|_| must(TcpListener::bind("127.0.0.1:0")))
            .collect();
        let mut ports = [0; 6];
        for (slot, listener) in ports.iter_mut().zip(&holders) {
            *slot = must(listener.local_addr()).port();
        }
        let [management, status, viewer, server, ws, wss] = ports;
        let cert = fixture("server-chain.pem")
            .display()
            .to_string()
            .replace('\\', "/");
        let key = fixture("server-key-p256.pem")
            .display()
            .to_string()
            .replace('\\', "/");
        let config = format!(
            r#"
[admin]
accounts_file = "accounts.toml"
[management]
listen = "127.0.0.1:{management}"
[status]
listen = "127.0.0.1:{status}"
[log]
output = "events.jsonl"
[limits]
relay_connections = 8
admin_connections = 8
waiting = 8
[recent_closed]
max_entries = {entries}
[timeouts]
close_secs = {close_secs}
tls_upgrade_secs = {upgrade_secs}
[relay.tcp_viewer]
enabled = true
listen = "127.0.0.1:{viewer}"
allow = ["127.0.0.1"]
[relay.tcp_server]
enabled = true
listen = "127.0.0.1:{server}"
allow = ["127.0.0.1"]
[relay.ws]
listen = "127.0.0.1:{ws}"
[relay.ws.viewer]
enabled = true
allow = ["127.0.0.1"]
allowed_origins = ["http://127.0.0.1"]
[relay.wss]
listen = "127.0.0.1:{wss}"
cert_file = "{cert}"
key_file = "{key}"
[relay.wss.viewer]
enabled = true
allow = ["127.0.0.1"]
allowed_origins = ["http://127.0.0.1"]
[mode1]
enabled = true
allow_targets = ["127.0.0.1:1-65535"]
[mode2]
enabled = true
"#
        );
        must(fs::write(dir.join("relay.toml"), config));
        drop(holders);
        let child = must(
            Command::new(BIN)
                .args(["serve", "--config"])
                .arg(dir.join("relay.toml"))
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn(),
        );
        let mut result = Self { dir, child, ports };
        let deadline = Instant::now() + LIMIT;
        loop {
            if let Some(status) = must(result.child.try_wait()) {
                let mut errors = String::new();
                if let Some(mut stderr) = result.child.stderr.take() {
                    must(stderr.read_to_string(&mut errors));
                }
                panic!("serve exited {status}: {errors}");
            }
            let ready = TcpStream::connect_timeout(
                &SocketAddr::from(([127, 0, 0, 1], status)),
                Duration::from_millis(100),
            );
            if let Ok(mut stream) = ready {
                must(stream.set_read_timeout(Some(LIMIT)));
                must(stream.write_all(
                    b"GET /readyz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                ));
                let mut reply = String::new();
                must(stream.read_to_string(&mut reply));
                if reply.starts_with("HTTP/1.1 200 ") {
                    break;
                }
            }
            assert!(Instant::now() < deadline, "serve readiness timeout");
            std::thread::sleep(POLL);
        }
        result
    }

    fn begin(
        &self,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        framing: &str,
        body: &[u8],
    ) -> TcpStream {
        let mut stream = connect(self.ports[0]);
        let mut request = format!(
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n{framing}\r\n"
        );
        for (name, value) in headers {
            request.push_str(&format!("{name}: {value}\r\n"));
        }
        request.push_str("\r\n");
        must(stream.write_all(request.as_bytes()));
        must(stream.write_all(body));
        must(stream.flush());
        stream
    }

    fn request(&self, method: &str, path: &str, headers: &[(&str, &str)], body: &[u8]) -> Reply {
        Reply::read(self.begin(
            method,
            path,
            headers,
            &format!("Content-Length: {}", body.len()),
            body,
        ))
    }

    fn login(&self, account: &str) -> Admin {
        let reply = self.request(
            "POST",
            "/api/v1/session",
            &[("Content-Type", "application/json")],
            format!("{{\"account\":\"{account}\",\"password\":\"{PASSWORD}\"}}").as_bytes(),
        );
        assert_eq!(reply.status, 200, "{}", reply.body);
        Admin {
            cookie: reply
                .header("set-cookie")
                .and_then(|v| v.split(';').next())
                .unwrap_or_else(|| panic!("missing cookie"))
                .to_owned(),
            csrf: reply.json().get("csrf_token").text().to_owned(),
        }
    }

    fn query(&self, admin: &Admin, path: &str) -> Reply {
        self.request("GET", path, &[("Cookie", &admin.cookie)], b"")
    }

    fn terminate_begin(&self, admin: &Admin, sid: &str) -> TcpStream {
        self.begin(
            "POST",
            &terminate_path(sid),
            &[("Cookie", &admin.cookie), ("X-CSRF-Token", &admin.csrf)],
            "Content-Length: 0",
            b"",
        )
    }

    fn terminate(&self, admin: &Admin, sid: &str) -> Reply {
        Reply::read(self.terminate_begin(admin, sid))
    }

    fn poll(&self, admin: &Admin, path: &str, predicate: impl Fn(&Json) -> bool) -> Json {
        let deadline = Instant::now() + LIMIT;
        loop {
            let reply = self.query(admin, path);
            assert_eq!(reply.status, 200, "poll {path}: {}", reply.body);
            let value = reply.json();
            if predicate(&value) {
                return value;
            }
            assert!(
                Instant::now() < deadline,
                "poll {path} timed out: {value:?}"
            );
            std::thread::sleep(POLL);
        }
    }

    fn counts(&self, admin: &Admin, active: u64, used: u64) {
        self.poll(admin, "/api/v1/summary", |v| {
            v.get("counts").get("active").number() == active
                && v.get("usage")
                    .get("relay_connections")
                    .get("current")
                    .number()
                    == used
        });
    }

    fn pair(&self, admin: &Admin, transport: &str, mode: &str, id: &str) -> Pair {
        let existing = self.query(admin, "/api/v1/sessions").json();
        let old: Vec<_> = existing
            .get("items")
            .items()
            .iter()
            .map(|v| v.get("session_id").text().to_owned())
            .collect();
        let mut viewer = match transport {
            "tcp" => {
                let mut tcp = connect(self.ports[2]);
                let mut greeting = [0; 12];
                must(tcp.read_exact(&mut greeting));
                assert_eq!(&greeting, b"RFB 000.000\n");
                Viewer::Tcp(tcp)
            }
            "ws" => Viewer::Ws(WebSocket::open(self.ports[4], false)),
            "wss" => Viewer::Ws(WebSocket::open(self.ports[5], true)),
            _ => panic!("unknown transport"),
        };
        let target = if mode == "mode2" {
            let mut tcp = connect(self.ports[3]);
            must(tcp.write_all(&initial(&format!("ID:{id}"))));
            self.poll(admin, "/api/v1/waiting", |v| {
                v.get("items")
                    .items()
                    .iter()
                    .any(|v| v.get("relay_id").text() == id)
            });
            viewer.send(&initial(&format!("ID:{id}")));
            tcp
        } else {
            let listener = must(TcpListener::bind("127.0.0.1:0"));
            must(listener.set_nonblocking(true));
            viewer.send(&initial(&must(listener.local_addr()).to_string()));
            let deadline = Instant::now() + LIMIT;
            loop {
                match listener.accept() {
                    Ok((tcp, _)) => break tcp,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "target accept timeout");
                        std::thread::sleep(POLL);
                    }
                    Err(error) => panic!("target accept: {error:?}"),
                }
            }
        };
        // Windows accepts inherit a nonblocking listener's mode.
        must(target.set_nonblocking(false));
        must(target.set_read_timeout(Some(LIMIT)));
        must(target.set_write_timeout(Some(LIMIT)));
        let list = self.poll(admin, "/api/v1/sessions", |v| {
            v.get("items")
                .items()
                .iter()
                .any(|v| !old.iter().any(|sid| sid == v.get("session_id").text()))
        });
        let item = list
            .get("items")
            .items()
            .iter()
            .find(|v| !old.iter().any(|sid| sid == v.get("session_id").text()))
            .unwrap_or_else(|| panic!("new session missing"));
        assert_eq!(item.get("mode").text(), mode);
        assert_eq!(item.get("viewer_transport").text(), transport);
        Pair {
            viewer,
            target,
            sid: item.get("session_id").text().to_owned(),
        }
    }

    fn logs(&self, predicate: impl Fn(&[Json]) -> bool) -> Vec<Json> {
        let deadline = Instant::now() + LIMIT;
        loop {
            let text = must(fs::read_to_string(self.dir.join("events.jsonl")));
            // A concurrent write may expose an incomplete last line. Only complete
            // JSON Lines records are observations; malformed complete lines fail.
            let end = text.rfind('\n').map_or(0, |index| index + 1);
            let records: Vec<_> = text[..end].lines().map(json).collect();
            if predicate(&records) {
                return records;
            }
            assert!(Instant::now() < deadline, "audit timeout: {records:?}");
            std::thread::sleep(POLL);
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn initial(text: &str) -> [u8; 250] {
    let mut bytes = [0; 250];
    assert!(text.len() <= bytes.len());
    bytes[..text.len()].copy_from_slice(text.as_bytes());
    bytes
}

fn terminate_path(sid: &str) -> String {
    format!("/api/v1/sessions/{sid}/terminate")
}
fn detail_path(sid: &str) -> String {
    format!("/api/v1/sessions/{sid}")
}

fn timestamp(value: &Json) {
    let text = value.text().as_bytes();
    assert_eq!(text.len(), 24);
    for (index, byte) in text.iter().enumerate() {
        match index {
            4 | 7 => assert_eq!(*byte, b'-'),
            10 => assert_eq!(*byte, b'T'),
            13 | 16 => assert_eq!(*byte, b':'),
            19 => assert_eq!(*byte, b'.'),
            23 => assert_eq!(*byte, b'Z'),
            _ => assert!(byte.is_ascii_digit()),
        }
    }
}

fn summary(
    reply: &Reply,
    sid: &str,
    result: &str,
    cause: &str,
    mode: &str,
    transport: &str,
    transfers: u64,
) -> Json {
    assert_eq!(reply.status, 200, "{}", reply.body);
    assert_eq!(reply.header("cache-control"), Some("no-store"));
    let value = reply.json();
    timestamp(value.get("as_of"));
    assert_eq!(value.get("result").text(), result);
    let closed = value.get("closed_session");
    assert_eq!(closed.get("session_id").text(), sid);
    assert_eq!(closed.get("close_cause").text(), cause);
    assert_eq!(closed.get("mode").text(), mode);
    assert_eq!(closed.get("viewer_transport").text(), transport);
    timestamp(closed.get("started_at"));
    timestamp(closed.get("ended_at"));
    assert!(closed.get("ended_at").text() >= closed.get("started_at").text());
    assert_eq!(
        closed.get("bytes_viewer_to_target").number(),
        must(u64::try_from(PAYLOAD.len())) * transfers
    );
    assert_eq!(
        closed.get("bytes_target_to_viewer").number(),
        12 * transfers
    );
    for key in [
        "admin_account",
        "relay_id",
        "viewer_address",
        "target_address",
        "csrf_token",
    ] {
        assert_eq!(closed.get(key), &Json::Null, "unexpected {key}");
    }
    closed.clone()
}

fn audit(server: &Server, sid: &str, cause: &str, outcomes: &[(&str, &str)]) {
    let records = server.logs(|records| {
        outcomes.iter().all(|(account, result)| {
            records.iter().any(|r| {
                r.get("event") == &Json::String("admin_terminate".to_owned())
                    && r.get("session_id") == &Json::String(sid.to_owned())
                    && r.get("admin_account") == &Json::String((*account).to_owned())
                    && r.get("result") == &Json::String((*result).to_owned())
            })
        })
    });
    let closed: Vec<_> = records
        .iter()
        .enumerate()
        .filter(|(_, r)| {
            r.get("event") == &Json::String("session_closed".to_owned())
                && r.get("session_id") == &Json::String(sid.to_owned())
        })
        .collect();
    assert_eq!(closed.len(), 1, "exactly one close event per session");
    let (closed_index, closed) = closed[0];
    assert_eq!(closed.get("close_cause").text(), cause);
    timestamp(closed.get("ts"));
    for (account, outcome) in outcomes {
        let (index, record) = records
            .iter()
            .enumerate()
            .find(|(_, r)| {
                r.get("event") == &Json::String("admin_terminate".to_owned())
                    && r.get("session_id") == &Json::String(sid.to_owned())
                    && r.get("admin_account") == &Json::String((*account).to_owned())
                    && r.get("result") == &Json::String((*outcome).to_owned())
            })
            .unwrap_or_else(|| panic!("missing audit"));
        assert!(index > closed_index, "session_closed precedes audit");
        assert_eq!(record.get("level").text(), "info");
        assert_eq!(record.get("peer_ip").text(), "127.0.0.1");
        timestamp(record.get("ts"));
    }
    if cause == "admin_terminated" {
        assert_eq!(closed.get("admin_account").text(), "alice");
    } else {
        assert_eq!(closed.get("admin_account"), &Json::Null);
    }
    if cause == "close_timeout" && outcomes.iter().any(|(_, result)| *result == "terminated") {
        assert_eq!(closed.get("side"), &Json::Null, "no EOF was observed");
    }
}

/// spec: management.md:MGT-008
#[test]
fn completion_mgt008_unauthorized_termination_preserves_both_sessions() {
    let _ports = PORTS.lock().unwrap_or_else(PoisonError::into_inner);
    let server = Server::start(4, 3, 2);
    let admin = server.login("alice");
    let mut first = server.pair(&admin, "tcp", "mode2", "8001");
    let mut other = server.pair(&admin, "tcp", "mode2", "8002");
    first.transfer();
    other.transfer();
    for cookie in [None, Some("relay_admin_session=not-a-session")] {
        let headers: Vec<_> = cookie.into_iter().map(|value| ("Cookie", value)).collect();
        let reply = server.request("POST", &terminate_path(&first.sid), &headers, b"");
        reply.error(401, "unauthenticated");
        assert!(reply.header("www-authenticate").is_some());
        if cookie.is_some() {
            assert!(reply.header("set-cookie").is_some());
        }
        first.transfer();
        other.transfer();
        server.counts(&admin, 2, 4);
    }
    let revoked = server.login("bob");
    assert_eq!(
        server
            .request(
                "DELETE",
                "/api/v1/session",
                &[("Cookie", &revoked.cookie), ("X-CSRF-Token", &revoked.csrf)],
                b""
            )
            .status,
        204
    );
    server
        .terminate(&revoked, &first.sid)
        .error(401, "unauthenticated");
    first.transfer();
    other.transfer();
    server.counts(&admin, 2, 4);
    for sid in [&first.sid, &other.sid] {
        let detail = server.query(&admin, &detail_path(sid)).json();
        assert_eq!(detail.get("session").get("state").text(), "active");
        assert_eq!(
            detail.get("session").get("bytes_viewer_to_target").number(),
            must(u64::try_from(PAYLOAD.len())) * 4
        );
        assert_eq!(
            detail.get("session").get("bytes_target_to_viewer").number(),
            48
        );
    }
    let records = server.logs(|r| {
        r.iter()
            .filter(|v| v.get("event") == &Json::String("session_started".to_owned()))
            .count()
            == 2
    });
    assert!(
        !records
            .iter()
            .any(|r| r.get("event") == &Json::String("admin_terminate".to_owned()))
    );
}

/// spec: management.md:MGT-009
/// spec: relay.md:RLY-020 partial
/// spec: operations.md:OPS-011 partial
#[test]
fn completion_mgt009_selective_tcp_ws_wss_mode1_mode2_cleanup_and_final_summary() {
    let _ports = PORTS.lock().unwrap_or_else(PoisonError::into_inner);
    for transport in ["tcp", "ws", "wss"] {
        for mode in ["mode1", "mode2"] {
            // R=0 exercises completion ownership independently of retained history.
            let retained = if mode == "mode1" { 0 } else { 4 };
            let server = Server::start(retained, 5, 2);
            let admin = server.login("alice");
            let mut other = server.pair(&admin, "tcp", "mode2", "9001");
            let mut target = server.pair(&admin, transport, mode, "9002");
            other.transfer();
            target.transfer();
            server.counts(&admin, 2, 4);
            let response = server.terminate_begin(&admin, &target.sid);
            if transport != "tcp" {
                // Receiving Close is an observable acceptance barrier. Until the
                // echo, the owner must keep its registration and both permits.
                target.viewer.websocket().close(4005, false);
                let detail = server.query(&admin, &detail_path(&target.sid)).json();
                assert_eq!(detail.get("session").get("state").text(), "closing");
                let sessions = server.query(&admin, "/api/v1/sessions").json();
                assert!(
                    sessions
                        .get("items")
                        .items()
                        .iter()
                        .any(|v| v.get("session_id").text() == target.sid
                            && v.get("state").text() == "closing")
                );
                server.counts(&admin, 2, 4);
                other.transfer();
                let mut payload = 4005_u16.to_be_bytes().to_vec();
                payload.extend_from_slice(b"admin_terminated");
                target.viewer.websocket().send(8, &payload);
            } else {
                target.viewer.acknowledge_admin_close();
                other.transfer();
            }
            let reply = Reply::read(response);
            let closed = summary(
                &reply,
                &target.sid,
                "terminated",
                "admin_terminated",
                mode,
                transport,
                1,
            );
            // A success response must follow cleanup, not merely command receipt.
            let state = server.query(&admin, "/api/v1/summary").json();
            assert_eq!(state.get("counts").get("active").number(), 1);
            assert_eq!(
                state
                    .get("usage")
                    .get("relay_connections")
                    .get("current")
                    .number(),
                2
            );
            let mut byte = [0];
            assert_eq!(must(target.target.read(&mut byte)), 0);
            server
                .query(&admin, &detail_path(&target.sid))
                .error(404, "not_found");
            server
                .terminate(&admin, &target.sid)
                .error(404, "not_found");
            let history = server.query(&admin, "/api/v1/closed-sessions").json();
            if retained == 0 {
                assert!(history.get("items").items().is_empty());
            } else {
                assert_eq!(history.get("items").items(), &[closed]);
            }
            other.transfer();
            audit(
                &server,
                &target.sid,
                "admin_terminated",
                &[("alice", "terminated")],
            );
            let records = server.logs(|r| {
                r.iter().any(|v| {
                    v.get("event") == &Json::String("admin_terminate".to_owned())
                        && v.get("result") == &Json::String("not_found".to_owned())
                })
            });
            let missing = records
                .iter()
                .find(|v| {
                    v.get("event") == &Json::String("admin_terminate".to_owned())
                        && v.get("result") == &Json::String("not_found".to_owned())
                })
                .unwrap_or_else(|| panic!("missing not_found audit"));
            assert_eq!(missing.get("session_id"), &Json::Null);
            assert_eq!(missing.get("admin_account").text(), "alice");
            assert_eq!(missing.get("peer_ip").text(), "127.0.0.1");
            assert_eq!(missing.get("level").text(), "info");
        }
    }
}

/// spec: management.md:MGT-010
#[test]
fn completion_mgt010_stale_session_id_cannot_terminate_reused_mode2_id() {
    let _ports = PORTS.lock().unwrap_or_else(PoisonError::into_inner);
    let server = Server::start(4, 3, 2);
    let admin = server.login("alice");
    let old = server.pair(&admin, "tcp", "mode2", "10001");
    let response = server.terminate_begin(&admin, &old.sid);
    let reply = Reply::read(response);
    summary(
        &reply,
        &old.sid,
        "terminated",
        "admin_terminated",
        "mode2",
        "tcp",
        0,
    );
    // Successful reuse also proves the Mode II occupied-ID entry was released.
    let mut fresh = server.pair(&admin, "tcp", "mode2", "10001");
    assert_ne!(fresh.sid, old.sid);
    server.terminate(&admin, &old.sid).error(404, "not_found");
    fresh.transfer();
    server.counts(&admin, 1, 2);
    assert_eq!(
        server
            .query(&admin, &detail_path(&fresh.sid))
            .json()
            .get("session")
            .get("state")
            .text(),
        "active"
    );
}

/// spec: management.md:MGT-011
/// spec: operations.md:OPS-011 partial
/// spec: relay.md:RLY-020 partial
#[test]
fn completion_mgt011_natural_close_and_concurrent_join_keep_real_cause_and_absolute_e() {
    let _ports = PORTS.lock().unwrap_or_else(PoisonError::into_inner);
    // Q92: E > U. Neither accepted waiter may be dropped at U=1.
    let server = Server::start(8, 3, 1);
    let alice = server.login("alice");
    let bob = server.login("bob");
    let mut other = server.pair(&alice, "tcp", "mode2", "11000");
    let mut target = server.pair(&alice, "ws", "mode2", "11001");
    target.transfer();
    let started = Instant::now();
    let first = server.terminate_begin(&alice, &target.sid);
    target.viewer.websocket().close(4005, false);
    let joined = server.terminate_begin(&bob, &target.sid);
    let state = server.query(&alice, &detail_path(&target.sid)).json();
    assert_eq!(state.get("session").get("state").text(), "closing");
    server.counts(&alice, 2, 4);
    other.transfer();
    // Withhold the Close echo, so E -- not a test sleep -- ends cleanup.
    let first = Reply::read(first);
    let joined = Reply::read(joined);
    assert!(
        started.elapsed() >= Duration::from_secs(2),
        "response must outlive U"
    );
    assert!(
        started.elapsed() < Duration::from_secs(6),
        "joining must not restart E"
    );
    let closed = summary(
        &first,
        &target.sid,
        "terminated",
        "close_timeout",
        "mode2",
        "ws",
        1,
    );
    assert_eq!(
        summary(
            &joined,
            &target.sid,
            "already_closed",
            "close_timeout",
            "mode2",
            "ws",
            1
        ),
        closed
    );
    audit(
        &server,
        &target.sid,
        "close_timeout",
        &[("alice", "terminated"), ("bob", "already_closed")],
    );
    server
        .terminate(&alice, &target.sid)
        .error(404, "not_found");
    server.counts(&alice, 1, 2);
    other.transfer();

    // Natural EOF is first. Holding the opposite TCP write-half open retains a
    // real closing session until its original E; a terminate only joins it.
    let mut natural = server.pair(&alice, "tcp", "mode2", "11002");
    natural.transfer();
    must(natural.target.shutdown(Shutdown::Write));
    server.poll(&alice, &detail_path(&natural.sid), |v| {
        v.get("session").get("state") == &Json::String("closing".to_owned())
    });
    let joined = server.terminate_begin(&bob, &natural.sid);
    let reply = Reply::read(joined);
    summary(
        &reply,
        &natural.sid,
        "already_closed",
        "close_timeout",
        "mode2",
        "tcp",
        1,
    );
    audit(
        &server,
        &natural.sid,
        "close_timeout",
        &[("bob", "already_closed")],
    );
    // Unlike the administrator E timeout, natural EOF has a real initiating side.
    let logs = server.logs(|r| {
        r.iter().any(|v| {
            v.get("event") == &Json::String("session_closed".to_owned())
                && v.get("session_id") == &Json::String(natural.sid.clone())
        })
    });
    let event = logs
        .iter()
        .find(|v| {
            v.get("event") == &Json::String("session_closed".to_owned())
                && v.get("session_id") == &Json::String(natural.sid.clone())
        })
        .unwrap_or_else(|| panic!("natural close missing"));
    assert_ne!(event.get("side"), &Json::Null);
    server
        .terminate(&alice, &natural.sid)
        .error(404, "not_found");
    other.transfer();

    // The detail was observed while active, but natural completion wins before
    // the command reaches the registry: this is 404, never already_closed.
    let completed = server.pair(&alice, "tcp", "mode2", "11003");
    let sid = completed.sid.clone();
    must(completed.target.shutdown(Shutdown::Write));
    if let Viewer::Tcp(viewer) = &completed.viewer {
        must(viewer.shutdown(Shutdown::Write));
    }
    server.poll(&alice, "/api/v1/closed-sessions", |v| {
        v.get("items")
            .items()
            .iter()
            .any(|v| v.get("session_id").text() == sid)
    });
    server.terminate(&alice, &sid).error(404, "not_found");
    server.counts(&alice, 1, 2);
    other.transfer();
}

/// spec: management.md:MGT-012 partial
/// spec: operations.md:OPS-011 partial
#[test]
fn completion_mgt012_lost_http_response_does_not_cancel_cleanup_or_initiator_audit() {
    let _ports = PORTS.lock().unwrap_or_else(PoisonError::into_inner);
    let server = Server::start(4, 3, 1);
    let admin = server.login("alice");
    let mut target = server.pair(&admin, "wss", "mode1", "12001");
    target.transfer();
    let response = server.terminate_begin(&admin, &target.sid);
    target.viewer.websocket().close(4005, false);
    // The Close proves acceptance before the requester disappears.
    must(response.shutdown(Shutdown::Both));
    drop(response);
    server.poll(&admin, "/api/v1/closed-sessions", |v| {
        v.get("items")
            .items()
            .iter()
            .any(|v| v.get("session_id").text() == target.sid)
    });
    server
        .query(&admin, &detail_path(&target.sid))
        .error(404, "not_found");
    server.counts(&admin, 0, 0);
    audit(
        &server,
        &target.sid,
        "close_timeout",
        &[("alice", "terminated")],
    );
    // Do not retransmit. Requery is the only recovery action in this test.
}

/// spec: management.md:MGT-013 partial
/// spec: management.md:MGT-014
#[test]
fn completion_q89_auth_csrf_validation_order_and_chunked_body_are_state_preserving() {
    let _ports = PORTS.lock().unwrap_or_else(PoisonError::into_inner);
    let server = Server::start(4, 3, 1);
    let admin = server.login("alice");
    let mut target = server.pair(&admin, "tcp", "mode2", "13001");
    target.transfer();
    let path = terminate_path(&target.sid);
    let valid = [
        ("Cookie", admin.cookie.as_str()),
        ("X-CSRF-Token", admin.csrf.as_str()),
    ];
    server
        .request("POST", "/api/v1/not-a-route", &[], b"x")
        .error(401, "unauthenticated");
    for route in [
        "/api/v1/not-a-route",
        path.as_str(),
        "/api/v1/sessions/bad%21/terminate",
    ] {
        for method in ["POST", "DELETE"] {
            server
                .request(method, route, &[("Cookie", &admin.cookie)], b"x")
                .error(403, "csrf_invalid");
        }
    }
    for headers in [
        vec![("Cookie", admin.cookie.as_str()), ("X-CSRF-Token", "wrong")],
        vec![
            ("Cookie", admin.cookie.as_str()),
            ("X-CSRF-Token", admin.csrf.as_str()),
            ("X-CSRF-Token", admin.csrf.as_str()),
        ],
    ] {
        server
            .request("POST", &path, &headers, b"")
            .error(403, "csrf_invalid");
    }
    server
        .request(
            "DELETE",
            "/api/v1/session",
            &[("Cookie", &admin.cookie)],
            b"",
        )
        .error(403, "csrf_invalid");
    assert_eq!(server.query(&admin, "/api/v1/session").status, 200);
    server
        .request("POST", "/api/v1/not-a-route", &valid, b"")
        .error(404, "not_found");
    for method in ["GET", "PUT", "DELETE"] {
        let reply = server.request(method, &path, &valid, b"");
        reply.error(405, "invalid_request");
        assert_eq!(reply.header("allow"), Some("POST"));
    }
    // GET has no CSRF requirement; POST/DELETE checked it before the method.
    server
        .request("GET", &path, &[("Cookie", &admin.cookie)], b"")
        .error(405, "invalid_request");
    for sid in ["bad%21", &"a".repeat(65)] {
        server
            .request("POST", &terminate_path(sid), &valid, b"")
            .error(400, "invalid_request");
    }
    for value in ["1", "0", "invalid", ""] {
        server
            .request(
                "POST",
                &path,
                &[valid[0], valid[1], ("X-Relay-Poll", value)],
                b"",
            )
            .error(400, "invalid_request");
    }
    for content_type in [
        None,
        Some("application/json"),
        Some("text/plain"),
        Some("application/octet-stream"),
    ] {
        let mut headers = valid.to_vec();
        if let Some(value) = content_type {
            headers.push(("Content-Type", value));
        }
        let reply = server.request("POST", &path, &headers, b"x");
        reply.error(400, "invalid_request");
        assert_safe(&reply, &admin);
        let chunked = Reply::read(server.begin(
            "POST",
            &path,
            &headers,
            "Transfer-Encoding: chunked",
            b"1\r\nx\r\n0\r\n\r\n",
        ));
        chunked.error(400, "invalid_request");
    }
    // Incomplete chunked framing must never issue a command. Its read remains
    // bounded by U; only an accepted completion wait is exempt (Q92).
    let incomplete = server.begin("POST", &path, &valid, "Transfer-Encoding: chunked", b"");
    let mut bytes = Vec::new();
    let mut incomplete = incomplete;
    let read = incomplete.read_to_end(&mut bytes);
    assert!(
        read.is_ok()
            || read.as_ref().is_err_and(|error| matches!(
                error.kind(),
                io::ErrorKind::ConnectionReset | io::ErrorKind::UnexpectedEof
            )),
        "incomplete body was not closed at U: {read:?}"
    );
    target.transfer();
    server.counts(&admin, 1, 2);
    let state = server.query(&admin, &detail_path(&target.sid));
    assert_eq!(state.json().get("session").get("state").text(), "active");
    assert_safe(&state, &admin);
    server
        .terminate(&admin, "not_a_canonical_session")
        .error(404, "not_found");
    let pending = server.terminate_begin(&admin, &target.sid);
    let reply = Reply::read(pending);
    summary(
        &reply,
        &target.sid,
        "terminated",
        "admin_terminated",
        "mode2",
        "tcp",
        2,
    );
    assert_safe(&reply, &admin);
    assert_safe(&server.terminate(&admin, &target.sid), &admin);
}

fn assert_safe(reply: &Reply, admin: &Admin) {
    for secret in [
        PASSWORD,
        "argon2id",
        must(std::str::from_utf8(PAYLOAD)),
        admin.cookie.as_str(),
        admin.csrf.as_str(),
    ] {
        assert!(
            !reply.body.contains(secret),
            "response exposed secret: {}",
            reply.body
        );
    }
}

/// spec: management.md:MGT-023
/// spec: relay.md:RLY-020
#[test]
fn completion_mgt023_eof_connection_error_and_admin_history_have_final_fields_and_old_ids_404() {
    let _ports = PORTS.lock().unwrap_or_else(PoisonError::into_inner);
    let server = Server::start(4, 3, 2);
    let admin = server.login("alice");
    let assert_reclaimed = |sid: &str, cause: &str| {
        server.counts(&admin, 0, 0);
        let state = server.query(&admin, "/api/v1/summary").json();
        for count in ["waiting", "active", "handshaking", "target_connecting"] {
            assert_eq!(
                state.get("counts").get(count).number(),
                0,
                "{cause}: {count}"
            );
        }
        assert_eq!(
            state
                .get("usage")
                .get("relay_connections")
                .get("current")
                .number(),
            0,
            "{cause}: session sockets release all L permits"
        );
        for path in ["/api/v1/waiting", "/api/v1/sessions"] {
            let reply = server.query(&admin, path);
            assert_eq!(reply.status, 200);
            assert!(
                reply.json().get("items").items().is_empty(),
                "{cause}: {path}"
            );
        }
        let records = server.logs(|records| {
            records.iter().any(|record| {
                record.get("event").text() == "session_closed"
                    && record.get("session_id").text() == sid
            })
        });
        let closures: Vec<_> = records
            .iter()
            .filter(|record| {
                record.get("event").text() == "session_closed"
                    && record.get("session_id").text() == sid
            })
            .collect();
        assert_eq!(
            closures.len(),
            1,
            "{cause}: one closure per reclaimed session"
        );
        assert_eq!(closures[0].get("close_cause").text(), cause);
    };
    let mut eof = server.pair(&admin, "tcp", "mode2", "23001");
    eof.transfer();
    server.counts(&admin, 1, 2);
    must(eof.target.shutdown(Shutdown::Write));
    if let Viewer::Tcp(viewer) = &eof.viewer {
        must(viewer.shutdown(Shutdown::Write));
    }
    server.poll(&admin, "/api/v1/closed-sessions", |v| {
        v.get("items").items().len() == 1
    });
    assert_reclaimed(&eof.sid, "peer_eof");
    let mut error = server.pair(&admin, "ws", "mode2", "23002");
    error.transfer();
    server.counts(&admin, 1, 2);
    // A text data frame deterministically triggers a real protocol I/O error;
    // unlike an OS-dependent TCP reset, it cannot silently become normal EOF.
    error.viewer.websocket().send(1, b"forbidden-text");
    error.viewer.websocket().close(1003, false);
    let error_sid = error.sid.clone();
    drop(error);
    server.poll(&admin, "/api/v1/closed-sessions", |v| {
        v.get("items").items().len() == 2
    });
    assert_reclaimed(&error_sid, "connection_error");
    let mut terminated = server.pair(&admin, "wss", "mode1", "23003");
    terminated.transfer();
    server.counts(&admin, 1, 2);
    let pending = server.terminate_begin(&admin, &terminated.sid);
    terminated.viewer.acknowledge_admin_close();
    summary(
        &Reply::read(pending),
        &terminated.sid,
        "terminated",
        "admin_terminated",
        "mode1",
        "wss",
        1,
    );
    assert_reclaimed(&terminated.sid, "admin_terminated");
    let history = server.query(&admin, "/api/v1/closed-sessions").json();
    assert_eq!(history.get("items").items().len(), 3);
    for (sid, cause, mode, transport) in [
        (&eof.sid, "peer_eof", "mode2", "tcp"),
        (&error_sid, "connection_error", "mode2", "ws"),
        (&terminated.sid, "admin_terminated", "mode1", "wss"),
    ] {
        let closed = history
            .get("items")
            .items()
            .iter()
            .find(|v| v.get("session_id").text() == sid)
            .unwrap_or_else(|| panic!("missing closed session {sid}"));
        assert_eq!(closed.get("close_cause").text(), cause);
        assert_eq!(closed.get("mode").text(), mode);
        assert_eq!(closed.get("viewer_transport").text(), transport);
        timestamp(closed.get("started_at"));
        timestamp(closed.get("ended_at"));
        assert!(closed.get("ended_at").text() >= closed.get("started_at").text());
        assert_eq!(
            closed.get("bytes_viewer_to_target").number(),
            must(u64::try_from(PAYLOAD.len()))
        );
        assert_eq!(closed.get("bytes_target_to_viewer").number(), 12);
        server
            .query(&admin, &detail_path(sid))
            .error(404, "not_found");
        server.terminate(&admin, sid).error(404, "not_found");
    }
    server.counts(&admin, 0, 0);
}
