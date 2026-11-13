//! Real-process fixtures used only by ticket_system.rs.
use std::fmt::Debug;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use serde_json::{Value, json};
use vnc_relay_adapters::accounts_file::Argon2Hasher;
use vnc_relay_app::ports::PasswordHasher;
use vnc_relay_domain::account::{Argon2Params, Password};

pub const LIMIT: Duration = Duration::from_secs(20);
static SERIAL: AtomicU64 = AtomicU64::new(0);
static PORTS: Mutex<()> = Mutex::new(());
const PASSWORD: &str = "correct-horse-battery";

pub fn must<T, E: Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("ticket system fixture: {error:?}"))
}
pub fn some<T>(value: Option<T>) -> T {
    value.unwrap_or_else(|| panic!("missing ticket system fixture value"))
}
pub fn lock() -> MutexGuard<'static, ()> {
    PORTS.lock().unwrap_or_else(PoisonError::into_inner)
}
pub fn now() -> u64 {
    must(SystemTime::now().duration_since(UNIX_EPOCH)).as_secs()
}
/// Monotonic instant at which the relay ends a connection whose ticket has `sx`.
/// Call it right after the 101: the relay turns `sx` into a monotonic deadline
/// from its wall clock at that moment. A later wall-clock step (WSL2 corrects
/// its clock forward by seconds) moves neither that deadline nor this one.
pub fn sx_deadline(sx: u64) -> Instant {
    let start = Instant::now();
    let remaining = (UNIX_EPOCH + Duration::from_secs(sx))
        .duration_since(SystemTime::now())
        .unwrap_or_default();
    start + remaining
}
/// Lateness the former wall-clock check `now() <= sx + 2` accepted: `now()`
/// rounds down to whole seconds, so any close before `sx + 3` passed.
pub const SX_LATE: Duration = Duration::from_secs(3);
/// Asserts that a `session_expired` close arrived in `[earliest, latest)`,
/// both built from [`sx_deadline`]. The close may come up to 50 ms before
/// `earliest` because the relay and this test read their clocks a moment apart.
pub fn assert_closed_between(earliest: Instant, latest: Instant, context: &str) {
    let closed = Instant::now();
    assert!(
        closed + Duration::from_millis(50) >= earliest,
        "{context}: closed {:?} before the deadline",
        earliest.saturating_duration_since(closed)
    );
    assert!(
        closed < latest,
        "{context}: closed {:?} after the deadline",
        closed.saturating_duration_since(earliest)
    );
}
pub fn wait(mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + LIMIT;
    while !ready() {
        assert!(Instant::now() < deadline, "ticket system deadline exceeded");
        std::thread::sleep(Duration::from_millis(20));
    }
}
pub fn request(value: &str) -> [u8; 250] {
    let mut bytes = [0; 250];
    bytes[..value.len()].copy_from_slice(value.as_bytes());
    bytes
}
pub fn claims(role: &str, jti: &str, rid: &str, sx: Option<u64>) -> Value {
    let iat = now() + 5;
    let mut value = json!({"iss":format!("issuer/{role}"), "aud":format!("vnc-relay/{role}"),
        "rh":"relay.example", "rid":rid, "jti":jti, "iat":iat, "exp":iat+60,
        "sub":"system-test-subject"});
    if let Some(sx) = sx {
        value["sx"] = json!(sx);
    }
    value
}
fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("crates/adapters/tests/fixtures/tls")
        .join(name)
}
pub fn tcp(port: u16) -> TcpStream {
    let stream = must(TcpStream::connect_timeout(
        &SocketAddr::from(([127, 0, 0, 1], port)),
        LIMIT,
    ));
    must(stream.set_read_timeout(Some(Duration::from_secs(8))));
    must(stream.set_write_timeout(Some(Duration::from_secs(8))));
    stream
}

pub trait Wire: Read + Write {}
impl<T: Read + Write> Wire for T {}
pub struct Ws {
    wire: Box<dyn Wire>,
}
impl Ws {
    fn send_frame(&mut self, opcode: u8, payload: &[u8]) {
        let mut bytes = vec![0x80 | opcode];
        if payload.len() < 126 {
            bytes.push(0x80 | must(u8::try_from(payload.len())));
        } else {
            bytes.extend_from_slice(&[0x80 | 126]);
            bytes.extend_from_slice(&must(u16::try_from(payload.len())).to_be_bytes());
        }
        let mask = [1, 3, 5, 7];
        bytes.extend_from_slice(&mask);
        bytes.extend(payload.iter().enumerate().map(|(i, b)| b ^ mask[i % 4]));
        must(self.wire.write_all(&bytes));
        must(self.wire.flush());
    }
    pub fn send(&mut self, payload: &[u8]) {
        self.send_frame(2, payload);
    }
    fn frame(&mut self) -> (u8, Vec<u8>) {
        loop {
            let mut head = [0; 2];
            must(self.wire.read_exact(&mut head));
            assert_eq!(head[1] & 0x80, 0, "server frames are unmasked");
            let len = match head[1] & 127 {
                126 => {
                    let mut b = [0; 2];
                    must(self.wire.read_exact(&mut b));
                    usize::from(u16::from_be_bytes(b))
                }
                127 => {
                    let mut b = [0; 8];
                    must(self.wire.read_exact(&mut b));
                    must(usize::try_from(u64::from_be_bytes(b)))
                }
                n => usize::from(n),
            };
            assert!(len <= 1_048_576);
            let mut payload = vec![0; len];
            must(self.wire.read_exact(&mut payload));
            match head[0] & 15 {
                9 => self.send_frame(10, &payload),
                10 => {}
                opcode => return (opcode, payload),
            }
        }
    }
    pub fn data(&mut self, expected: &[u8]) {
        let mut bytes = Vec::new();
        while bytes.len() < expected.len() {
            let (opcode, payload) = self.frame();
            assert_eq!(opcode, 2, "expected binary: {payload:?}");
            bytes.extend(payload);
        }
        assert_eq!(bytes, expected);
    }
    pub fn close(&mut self, code: u16, reason: &str) {
        let (opcode, payload) = self.frame();
        assert_eq!(opcode, 8, "expected close frame: {payload:?}");
        assert_eq!(u16::from_be_bytes([payload[0], payload[1]]), code);
        assert_eq!(&payload[2..], reason.as_bytes());
        self.send_frame(8, &payload);
    }
}

pub struct Reply {
    pub status: u16,
    pub raw: String,
    pub body: String,
}
impl Reply {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.raw
            .split("\r\n\r\n")
            .next()?
            .lines()
            .skip(1)
            .filter_map(|line| line.split_once(':'))
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.trim())
    }
    pub fn json(&self) -> Value {
        must(serde_json::from_str(&self.body))
    }
}
fn reply(raw: String) -> Reply {
    let status = must(some(raw.split_whitespace().nth(1)).parse());
    let body = some(raw.split_once("\r\n\r\n")).1.to_owned();
    Reply { status, raw, body }
}
pub fn http(port: u16, method: &str, path: &str, headers: &[(&str, &str)], body: &str) -> Reply {
    let mut wire = tcp(port);
    let mut request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    );
    for (key, value) in headers {
        request.push_str(&format!("{key}: {value}\r\n"));
    }
    request.push_str("\r\n");
    request.push_str(body);
    must(wire.write_all(request.as_bytes()));
    let mut raw = String::new();
    must(wire.read_to_string(&mut raw));
    reply(raw)
}

pub struct Server {
    pub dir: PathBuf,
    child: Child,
    pub management: u16,
    pub status: u16,
    pub viewer: u16,
    pub server: u16,
    ws: u16,
    wss: u16,
    pub admin: (String, String),
}
impl Server {
    pub fn start(name: &str, viewer_keys: &str, server_keys: &str, mode1: bool) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "vnc-relay-ticket-{}-{}-{name}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        must(fs::create_dir(&dir));
        must(fs::write(dir.join("viewer.json"), viewer_keys));
        must(fs::write(dir.join("server.json"), server_keys));
        let password = must(Password::parse(PASSWORD.to_owned()));
        let hash = must(Argon2Hasher.hash(
            &password,
            Argon2Params {
                memory_kib: 19456,
                iterations: 2,
                parallelism: 1,
            },
        ));
        must(fs::write(
            dir.join("accounts.toml"),
            format!(
                "version = 1\n[[account]]\nname = \"alice\"\npassword_hash = \"{}\"\n",
                hash.as_str()
            ),
        ));
        let holders: Vec<_> = (0..6)
            .map(|_| must(TcpListener::bind("127.0.0.1:0")))
            .collect();
        let ports: Vec<_> = holders
            .iter()
            .map(|h| must(h.local_addr()).port())
            .collect();
        let (management, status, viewer, server, ws, wss) =
            (ports[0], ports[1], ports[2], ports[3], ports[4], ports[5]);
        let cert = fixture("server-chain.pem")
            .display()
            .to_string()
            .replace('\\', "/");
        let key = fixture("server-key-p256.pem")
            .display()
            .to_string()
            .replace('\\', "/");
        let config = format!(
            r#"[admin]
accounts_file = "accounts.toml"
[management]
listen = "127.0.0.1:{management}"
config_edit = true
[status]
listen = "127.0.0.1:{status}"
[log]
output = "relay.log"
[reload]
settle_secs = 1
poll_secs = 5
[timeouts]
handshake_secs = 20
wait_secs = 60
close_secs = 1
[recent_closed]
max_entries = 100
[recent_logs]
max_entries = 1000
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
[relay.ws.server]
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
[relay.wss.server]
enabled = true
allow = ["127.0.0.1"]
allowed_origins = ["http://127.0.0.1"]
[mode1]
enabled = {mode1}
allow_targets = ["127.0.0.1:1-65535"]
[mode2]
enabled = true
[relay.ticket.viewer]
enabled = true
issuer = "issuer/viewer"
relay_host = "relay.example"
public_keys_file = "viewer.json"
max_lifetime_secs = 60
[relay.ticket.server]
enabled = true
issuer = "issuer/server"
relay_host = "relay.example"
public_keys_file = "server.json"
max_lifetime_secs = 60
"#
        );
        must(fs::write(dir.join("relay.toml"), config));
        let stderr = must(fs::File::create(dir.join("stderr.txt")));
        drop(holders);
        let child = must(
            Command::new(env!("CARGO_BIN_EXE_vnc-relay"))
                .args(["serve", "--config"])
                .arg(dir.join("relay.toml"))
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(stderr)
                .spawn(),
        );
        let mut result = Self {
            dir,
            child,
            management,
            status,
            viewer,
            server,
            ws,
            wss,
            admin: (String::new(), String::new()),
        };
        let deadline = Instant::now() + LIMIT;
        loop {
            if result.events().iter().any(|e| e["event"] == "server_ready") {
                break;
            }
            assert!(
                must(result.child.try_wait()).is_none(),
                "server exited: {}",
                result.stderr()
            );
            assert!(Instant::now() < deadline, "not ready: {}", result.stderr());
            std::thread::sleep(Duration::from_millis(20));
        }
        let login = http(
            management,
            "POST",
            "/api/v1/session",
            &[("Content-Type", "application/json")],
            &json!({"account":"alice","password":PASSWORD}).to_string(),
        );
        assert_eq!(login.status, 200, "{}", login.raw);
        result.admin = (
            some(some(login.header("set-cookie")).split(';').next()).to_owned(),
            some(login.json()["csrf_token"].as_str()).to_owned(),
        );
        // Guarantees iat=now+5 is strictly later than process_start+5.
        wait(|| {
            result.query("/api/v1/summary")["server"]["uptime_ms"]
                .as_u64()
                .is_some_and(|n| n >= 1500)
        });
        result
    }
    pub fn stderr(&self) -> String {
        must(fs::read_to_string(self.dir.join("stderr.txt")))
    }
    pub fn log(&self) -> String {
        fs::read_to_string(self.dir.join("relay.log")).unwrap_or_default()
    }
    pub fn events(&self) -> Vec<Value> {
        self.log()
            .split_inclusive('\n')
            .filter(|line| line.ends_with('\n'))
            .map(|line| must(serde_json::from_str(line)))
            .collect()
    }
    pub fn event(&self, after: usize, predicate: impl Fn(&Value) -> bool) -> Value {
        let mut found = None;
        wait(|| {
            found = self.events().into_iter().skip(after).find(&predicate);
            found.is_some()
        });
        some(found)
    }
    pub fn get(&self, path: &str) -> Reply {
        http(
            self.management,
            "GET",
            path,
            &[("Cookie", &self.admin.0)],
            "",
        )
    }
    pub fn query(&self, path: &str) -> Value {
        let reply = self.get(path);
        assert_eq!(reply.status, 200, "{}", reply.raw);
        reply.json()
    }
    pub fn poll(&self, path: &str, predicate: impl Fn(&Value) -> bool) -> Value {
        let mut value = Value::Null;
        wait(|| {
            value = self.query(path);
            predicate(&value)
        });
        value
    }
    pub fn patch(&self, changes: Value) {
        let view = self.query("/api/v1/config");
        let reply = http(
            self.management,
            "PATCH",
            "/api/v1/config",
            &[
                ("Cookie", &self.admin.0),
                ("X-CSRF-Token", &self.admin.1),
                ("Content-Type", "application/json"),
            ],
            &json!({"revision":view["revision"],"changes":changes}).to_string(),
        );
        assert_eq!(reply.status, 200, "{}", reply.raw);
    }
    pub fn rotate(&self, role: &str, keys: &str) {
        let before = self.events().len();
        must(fs::write(self.dir.join(format!("{role}.json")), keys));
        self.event(before, |e| e["event"] == "reload_applied");
    }
    pub fn upgrade(&self, secure: bool, role: &str, ticket: Option<&str>) -> (Reply, Option<Ws>) {
        let stream = tcp(if secure { self.wss } else { self.ws });
        let mut wire: Box<dyn Wire> = if secure {
            let mut roots = RootCertStore::empty();
            for cert in must(CertificateDer::pem_file_iter(fixture("root-ca.pem"))) {
                must(roots.add(must(cert)));
            }
            let config = must(
                ClientConfig::builder_with_provider(std::sync::Arc::new(
                    rustls::crypto::aws_lc_rs::default_provider(),
                ))
                .with_protocol_versions(&[&rustls::version::TLS13, &rustls::version::TLS12]),
            )
            .with_root_certificates(roots)
            .with_no_client_auth();
            Box::new(StreamOwned::new(
                must(ClientConnection::new(
                    std::sync::Arc::new(config),
                    must(ServerName::try_from("localhost")),
                )),
                stream,
            ))
        } else {
            Box::new(stream)
        };
        let protocols = ticket.map_or_else(
            || "binary".to_owned(),
            |ticket| format!("binary, ticket.{ticket}"),
        );
        let request = format!(
            "GET /relay/{role} HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Protocol: {protocols}\r\nOrigin: http://127.0.0.1\r\n\r\n"
        );
        must(wire.write_all(request.as_bytes()));
        must(wire.flush());
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            must(wire.read_exact(&mut byte));
            head.push(byte[0]);
            assert!(head.len() < 16384);
        }
        let mut result = reply(must(String::from_utf8(head)));
        if result.status == 101 {
            assert_eq!(result.header("Sec-WebSocket-Protocol"), Some("binary"));
            (result, Some(Ws { wire }))
        } else {
            let length = result
                .header("Content-Length")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(0);
            let mut body = vec![0; length];
            must(wire.read_exact(&mut body));
            result.body = must(String::from_utf8(body));
            result.raw.push_str(&result.body);
            (result, None)
        }
    }
    pub fn ws(&self, secure: bool, role: &str, ticket: &str) -> Ws {
        let (reply, ws) = self.upgrade(secure, role, Some(ticket));
        assert_eq!(reply.status, 101, "{}", reply.raw);
        let mut ws = some(ws);
        if role == "viewer" {
            ws.data(b"RFB 000.000\n");
        }
        ws
    }
    pub fn tcp_peer(&self, role: &str, rid: &str) -> TcpStream {
        let mut peer = tcp(if role == "viewer" {
            self.viewer
        } else {
            self.server
        });
        if role == "viewer" {
            let mut greeting = [0; 12];
            must(peer.read_exact(&mut greeting));
            assert_eq!(&greeting, b"RFB 000.000\n");
        }
        must(peer.write_all(&request(&format!("ID:{rid}"))));
        peer
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_dir_all(&self.dir);
    }
}
pub fn exchange(ws: &mut Ws, peer: &mut TcpStream) {
    ws.send(b"viewer-data");
    let mut bytes = [0; 11];
    must(peer.read_exact(&mut bytes));
    assert_eq!(&bytes, b"viewer-data");
    must(peer.write_all(b"server-data"));
    ws.data(b"server-data");
}
pub fn tcp_closed(peer: &mut TcpStream) {
    let mut byte = [0];
    match peer.read(&mut byte) {
        Ok(0) => {}
        Err(e)
            if matches!(
                e.kind(),
                std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::BrokenPipe
            ) => {}
        other => panic!("expected closed TCP peer: {other:?}"),
    }
}
pub fn timestamp(value: &Value) {
    let value = some(value.as_str());
    assert_eq!(value.len(), 24, "{value}");
    assert_eq!(&value[10..11], "T");
    assert!(value.ends_with(".000Z"), "{value}");
}
