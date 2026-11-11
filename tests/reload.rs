//! Automatic reload against the real binary, file watcher, relay transports,
//! management API, and JSON operation log. Runtime groups are exercised at their
//! documented boundaries; socket-option and shutdown boundary coverage also
//! lives in the adapter and platform-specific suites.

use std::io::{ErrorKind, Read, Write};
use std::net::{Ipv6Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::version::{TLS12, TLS13};
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};

const BIN: &str = env!("CARGO_BIN_EXE_vnc-relay");

/// 16 and 32 zero bytes in unpadded Base64. Test data, not a real hash.
const SALT: &str = "AAAAAAAAAAAAAAAAAAAAAA";
const HASH: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

/// Upper bound for one wait. A hang fails the test instead of blocking it.
const EXIT_LIMIT: Duration = Duration::from_secs(20);

/// How long a connection must stay open to count as still open.
const SETTLE: Duration = Duration::from_millis(500);

/// D of the test configs (`settle_secs = 1` in `config_text`).
const RELOAD_SETTLE: Duration = Duration::from_secs(1);

const VIEWER_ONLY: &str = r#"["127.0.0.1"]"#;
const SECOND_ONLY: &str = r#"["127.0.0.2"]"#;
const TWO_SOURCES: &str = r#"["127.0.0.1", "127.0.0.2"]"#;
const RANGE: &str = r#"["127.0.0.2-127.0.0.3"]"#;
const ANY: &str = r#"["0.0.0.0/0"]"#;
const NONE: &str = "[]";

const CERT: &str = "server-chain.pem";
const KEY: &str = "server-key-p256.pem";

/// Config key of the WSS key file, named by rejected TLS candidates.
const KEY_KEY: &str = "relay.wss.key_file";

static PORTS_LOCK: Mutex<()> = Mutex::new(());

fn lock_ports() -> MutexGuard<'static, ()> {
    PORTS_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Listener ports of one test config.
#[derive(Clone, Copy)]
struct Ports {
    status: u16,
    management: u16,
    tcp_viewer: u16,
    tcp_server: u16,
    ws: u16,
    wss: u16,
}

impl Ports {
    fn free() -> Self {
        // Hold all six until every port is chosen so that they differ.
        let holders: Vec<TcpListener> = (0..6)
            .map(|_| {
                TcpListener::bind("127.0.0.1:0")
                    .unwrap_or_else(|error| panic!("free port: {error}"))
            })
            .collect();
        let port = |i: usize| {
            holders[i]
                .local_addr()
                .unwrap_or_else(|error| panic!("local addr: {error}"))
                .port()
        };
        Self {
            status: port(0),
            management: port(1),
            tcp_viewer: port(2),
            tcp_server: port(3),
            ws: port(4),
            wss: port(5),
        }
    }

    fn all(self) -> [u16; 6] {
        [
            self.status,
            self.management,
            self.tcp_viewer,
            self.tcp_server,
            self.ws,
            self.wss,
        ]
    }
}

/// A free loopback port for a listener that a test moves to.
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .unwrap_or_else(|error| panic!("free port: {error}"))
        .port()
}

/// Path of a fixture in `crates/adapters/tests/fixtures/tls`, for a TOML
/// literal string (no escapes, so Windows separators are safe).
fn tls_file(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("crates/adapters/tests/fixtures/tls")
        .join(name)
        .display()
        .to_string()
}

/// The path a config names for `name`: a fixture name, or a copy made by
/// `copy_tls`, given as an absolute path.
fn tls_path(name: &str) -> String {
    if Path::new(name).is_absolute() {
        name.to_owned()
    } else {
        tls_file(name)
    }
}

/// The config of every entry point. `allow` is a TOML array used by every
/// entry point, `cert` and `key` name the WSS pair (fixture names, or paths of
/// scratch copies), and the timeouts are long enough that held connections stay
/// open for the test.
fn config_text(ports: Ports, allow: &str, cert: &str, key: &str) -> String {
    format!(
        r#"
[admin]
accounts_file = "admin-accounts.toml"

[management]
listen = "127.0.0.1:{management}"

[status]
listen = "127.0.0.1:{status}"

[relay.tcp_viewer]
enabled = true
listen = "127.0.0.1:{tcp_viewer}"
allow = {allow}

[relay.tcp_server]
enabled = true
listen = "127.0.0.1:{tcp_server}"
allow = {allow}

[relay.ws]
listen = "127.0.0.1:{ws}"

[relay.ws.viewer]
enabled = true
allow = {allow}

[relay.wss]
listen = "127.0.0.1:{wss}"
cert_file = '{cert_path}'
key_file = '{key_path}'

[relay.wss.viewer]
enabled = true
allow = {allow}

[mode2]
enabled = true

[timeouts]
handshake_secs = 120
tls_upgrade_secs = 120

[reload]
settle_secs = 1
poll_secs = 5

[log]
output = "relay.log"
"#,
        management = ports.management,
        status = ports.status,
        tcp_viewer = ports.tcp_viewer,
        tcp_server = ports.tcp_server,
        ws = ports.ws,
        wss = ports.wss,
        cert_path = tls_path(cert),
        key_path = tls_path(key),
    )
}

fn accounts_text() -> String {
    format!(
        "version = 1\n\n[[account]]\nname = \"alice\"\npassword_hash = \"$argon2id$v=19$m=19456,t=2,p=1${SALT}${HASH}\"\n"
    )
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vnc-relay-reload-{}-{name}", std::process::id()));
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

/// A new directory with the account file. The caller writes the config.
fn scratch(name: &str) -> PathBuf {
    let dir = temp_dir(name);
    write(&dir.join("admin-accounts.toml"), &accounts_text());
    dir
}

/// Writes `config` and the account file into a new directory and returns the
/// config path. The operation log is `relay.log` next to it.
fn setup(name: &str, config: &str) -> PathBuf {
    let dir = scratch(name);
    let config_path = dir.join("relay.toml");
    write(&config_path, config);
    config_path
}

/// Copies a fixture into `dir` as `file` and returns the copy's path, so a
/// test can replace or remove it.
fn copy_tls(dir: &Path, fixture: &str, file: &str) -> String {
    let path = dir.join(file);
    std::fs::copy(tls_file(fixture), &path)
        .unwrap_or_else(|error| panic!("copy {fixture}: {error}"));
    path.display().to_string()
}

fn log_of(config: &Path) -> PathBuf {
    config.with_file_name("relay.log")
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

/// Kills the child and waits for it to exit, so its listeners are released.
fn stop(child: &mut Child) {
    child.kill().unwrap_or_else(|error| panic!("kill: {error}"));
    child.wait().unwrap_or_else(|error| panic!("wait: {error}"));
}

/// Kills the child if a test fails before it stops the child itself.
struct Guard(Option<Child>);

impl Guard {
    fn child(&mut self) -> &mut Child {
        self.0
            .as_mut()
            .unwrap_or_else(|| panic!("child already stopped"))
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

fn connects(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok()
}

/// Waits until every listener accepts a TCP connection. Fails early with the
/// stderr of the child when it exits.
fn wait_listening(child: &mut Child, ports: Ports) {
    wait_until(child, || ports.all().into_iter().all(connects));
}

/// Waits until `ready` holds. Fails early with the stderr of the child when
/// it exits.
fn wait_until(child: &mut Child, ready: impl Fn() -> bool) {
    let deadline = Instant::now() + EXIT_LIMIT;
    while !ready() {
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

/// The value of a string field of a JSON log line.
fn field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let start = line.find(&format!("\"{name}\":\""))? + name.len() + 4;
    let rest = &line[start..];
    rest.find('"').map(|end| &rest[..end])
}

fn event_is(line: &str, event: &str) -> bool {
    field(line, "event") == Some(event)
}

/// Whether a log line is `event` with `reason`.
fn reason_is(line: &str, event: &str, reason: &str) -> bool {
    event_is(line, event) && field(line, "reason") == Some(reason)
}

/// Whether a log line is `event` on `listener` from `peer_ip`.
fn is(line: &str, event: &str, listener: &str, peer_ip: &str) -> bool {
    event_is(line, event)
        && field(line, "listener") == Some(listener)
        && field(line, "peer_ip") == Some(peer_ip)
}

/// Number of log lines that `wanted` accepts. A missing log has none.
fn logged(log: &Path, wanted: impl Fn(&str) -> bool) -> usize {
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .filter(|line| wanted(line))
        .count()
}

/// Waits until at least `count` log lines are accepted by `wanted`.
fn wait_for_count(log: &Path, count: usize, wanted: impl Fn(&str) -> bool) {
    let deadline = Instant::now() + EXIT_LIMIT;
    loop {
        if logged(log, &wanted) >= count {
            return;
        }
        let text = std::fs::read_to_string(log).unwrap_or_default();
        assert!(
            Instant::now() < deadline,
            "{count} lines not logged in {EXIT_LIMIT:?}: {text}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Waits until the log has a line that `wanted` accepts and returns it.
fn wait_for_line(log: &Path, wanted: impl Fn(&str) -> bool) -> String {
    let deadline = Instant::now() + EXIT_LIMIT;
    loop {
        let text = std::fs::read_to_string(log).unwrap_or_default();
        if let Some(line) = text.lines().find(|line| wanted(line)) {
            return line.to_owned();
        }
        assert!(
            Instant::now() < deadline,
            "line not logged in {EXIT_LIMIT:?}: {text}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Waits until the log has a line that `wanted` accepts and returns an instant
/// before which the line was not written: the start of the last read that did
/// not have it, or `since` if the first read had it. The log writes each line
/// with one unbuffered write, so a read that starts after the write sees it.
fn line_written_after(log: &Path, since: Instant, wanted: impl Fn(&str) -> bool) -> Instant {
    let deadline = Instant::now() + EXIT_LIMIT;
    let mut written_after = since;
    loop {
        let read_at = Instant::now();
        let text = std::fs::read_to_string(log).unwrap_or_default();
        if text.lines().any(&wanted) {
            return written_after;
        }
        written_after = read_at;
        assert!(
            Instant::now() < deadline,
            "line not logged in {EXIT_LIMIT:?}: {text}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Connects to the loopback `port` from `source`, any address of
/// 127.0.0.0/8. std cannot choose the source address, so tokio opens the
/// socket and hands it back as a blocking std stream.
fn connect_from(source: [u8; 4], port: u16) -> TcpStream {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()
        .unwrap_or_else(|error| panic!("runtime: {error}"));
    let stream = runtime.block_on(async {
        let socket =
            tokio::net::TcpSocket::new_v4().unwrap_or_else(|error| panic!("socket: {error}"));
        socket
            .bind(SocketAddr::from((source, 0)))
            .unwrap_or_else(|error| panic!("bind {source:?}: {error}"));
        let stream = socket
            .connect(SocketAddr::from(([127, 0, 0, 1], port)))
            .await
            .unwrap_or_else(|error| panic!("connect {port}: {error}"));
        let stream = stream
            .into_std()
            .unwrap_or_else(|error| panic!("into std: {error}"));
        stream
            .set_nonblocking(false)
            .unwrap_or_else(|error| panic!("blocking: {error}"));
        stream
    });
    stream
        .set_read_timeout(Some(EXIT_LIMIT))
        .unwrap_or_else(|error| panic!("read timeout: {error}"));
    stream
}

const GREETING: &[u8] = b"RFB 000.000\n";
const HANDSHAKE_REQUEST_LEN: usize = 250;

fn request(id: &str) -> [u8; HANDSHAKE_REQUEST_LEN] {
    let text = format!("ID:{id}");
    assert!(text.len() <= HANDSHAKE_REQUEST_LEN);
    let mut bytes = [0; HANDSHAKE_REQUEST_LEN];
    bytes[..text.len()].copy_from_slice(text.as_bytes());
    bytes
}

trait Wire: Read + Write + Send {}
impl<T: Read + Write + Send> Wire for T {}

struct WebSocket {
    wire: Box<dyn Wire>,
}

impl WebSocket {
    fn upgrade(mut wire: Box<dyn Wire>) -> Self {
        wire.write_all(&upgrade_request())
            .unwrap_or_else(|error| panic!("send upgrade: {error}"));
        wire.flush()
            .unwrap_or_else(|error| panic!("flush upgrade: {error}"));
        let head = read_head(&mut wire);
        assert_eq!(status_of(&head), 101, "upgrade response: {head}");
        Self { wire }
    }

    fn send_binary(&mut self, payload: &[u8]) {
        let mut bytes = vec![0x82];
        if payload.len() < 126 {
            bytes.push(
                0x80 | u8::try_from(payload.len())
                    .unwrap_or_else(|error| panic!("short frame: {error}")),
            );
        } else if let Ok(length) = u16::try_from(payload.len()) {
            bytes.push(0x80 | 126);
            bytes.extend_from_slice(&length.to_be_bytes());
        } else {
            bytes.push(0x80 | 127);
            bytes.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        }
        let mask = [0x13, 0x57, 0x9b, 0xdf];
        bytes.extend_from_slice(&mask);
        bytes.extend(
            payload
                .iter()
                .enumerate()
                .map(|(i, byte)| byte ^ mask[i % 4]),
        );
        self.wire
            .write_all(&bytes)
            .unwrap_or_else(|error| panic!("send masked client frame: {error}"));
        self.wire
            .flush()
            .unwrap_or_else(|error| panic!("flush client frame: {error}"));
    }

    fn frame(&mut self) -> (bool, u8, Vec<u8>) {
        loop {
            let mut head = [0; 2];
            self.wire
                .read_exact(&mut head)
                .unwrap_or_else(|error| panic!("read server frame header: {error}"));
            assert_eq!(head[0] & 0x70, 0, "no unnegotiated extensions");
            assert_eq!(head[1] & 0x80, 0, "server frames are unmasked");
            let fin = head[0] & 0x80 != 0;
            let opcode = head[0] & 0x0f;
            let len = match head[1] & 0x7f {
                126 => {
                    let mut bytes = [0; 2];
                    self.wire
                        .read_exact(&mut bytes)
                        .unwrap_or_else(|error| panic!("frame length: {error}"));
                    usize::from(u16::from_be_bytes(bytes))
                }
                127 => {
                    let mut bytes = [0; 8];
                    self.wire
                        .read_exact(&mut bytes)
                        .unwrap_or_else(|error| panic!("frame length: {error}"));
                    usize::try_from(u64::from_be_bytes(bytes))
                        .unwrap_or_else(|error| panic!("length fits usize: {error}"))
                }
                len => usize::from(len),
            };
            assert!(len <= 65536, "server frame remains within configured M");
            let mut payload = vec![0; len];
            self.wire
                .read_exact(&mut payload)
                .unwrap_or_else(|error| panic!("server frame payload: {error}"));
            match opcode {
                9 => {
                    let mut bytes = vec![0x8a];
                    bytes.push(
                        0x80 | u8::try_from(payload.len())
                            .unwrap_or_else(|error| panic!("pong len: {error}")),
                    );
                    let mask = [0x13, 0x57, 0x9b, 0xdf];
                    bytes.extend_from_slice(&mask);
                    bytes.extend(
                        payload
                            .iter()
                            .enumerate()
                            .map(|(i, byte)| byte ^ mask[i % 4]),
                    );
                    self.wire
                        .write_all(&bytes)
                        .unwrap_or_else(|error| panic!("send pong: {error}"));
                    self.wire
                        .flush()
                        .unwrap_or_else(|error| panic!("flush pong: {error}"));
                }
                10 => {}
                _ => return (fin, opcode, payload),
            }
        }
    }

    fn read_data(&mut self, len: usize) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(len);
        let mut fragmented = false;
        while bytes.len() < len || fragmented {
            let (fin, opcode, payload) = self.frame();
            match opcode {
                2 => assert!(
                    !fragmented,
                    "binary frame cannot interrupt unfinished message"
                ),
                0 => assert!(
                    fragmented,
                    "continuation frame requires in-progress message"
                ),
                other => panic!("expected binary or continuation frame, got opcode {other}"),
            }
            fragmented = !fin;
            bytes.extend_from_slice(&payload);
            assert!(bytes.len() <= len, "no unexpected extra relayed data");
        }
        assert_eq!(bytes.len(), len, "exact relayed data length");
        bytes
    }
}
fn ws_upgrade_with_headers(
    mut wire: Box<dyn Wire>,
    path: &str,
    headers: &[(&str, &str)],
) -> (u16, Option<WebSocket>) {
    let mut req = format!(
        "GET {path} HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n"
    );
    for (name, value) in headers {
        req.push_str(&format!("{name}: {value}\r\n"));
    }
    req.push_str("\r\n");
    wire.write_all(req.as_bytes())
        .unwrap_or_else(|error| panic!("send upgrade: {error}"));
    wire.flush()
        .unwrap_or_else(|error| panic!("flush upgrade: {error}"));
    let head = read_head(&mut wire);
    let status = status_of(&head);
    if status == 101 {
        (status, Some(WebSocket { wire }))
    } else {
        (status, None)
    }
}

/// True when the server closes the connection (EOF or reset) within `wait`,
/// false when the connection is still open after it.
fn closed_within(stream: &mut TcpStream, wait: Duration) -> bool {
    let deadline = Instant::now() + wait;
    loop {
        let now = Instant::now();
        if now >= deadline {
            return false;
        }
        let remaining = deadline - now;
        stream
            .set_read_timeout(Some(remaining.min(Duration::from_millis(100))))
            .unwrap_or_else(|error| panic!("read timeout: {error}"));
        let mut buf = [0; 64];
        match stream.read(&mut buf) {
            Ok(0) => return true,
            Ok(_) => continue,
            Err(error) => match error.kind() {
                ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted => return true,
                ErrorKind::WouldBlock | ErrorKind::TimedOut => continue,
                _ => panic!("read: {error}"),
            },
        }
    }
}

type Tls = StreamOwned<ClientConnection, TcpStream>;

/// A client that trusts only the test root CA, over `tcp`. The handshake runs
/// on the first read or write, or in `handshake`.
fn tls_client(tcp: TcpStream) -> Tls {
    let mut roots = RootCertStore::empty();
    let certs = CertificateDer::pem_file_iter(tls_file("root-ca.pem"))
        .unwrap_or_else(|error| panic!("root CA opens: {error}"));
    for cert in certs {
        let cert = cert.unwrap_or_else(|error| panic!("root CA is PEM: {error}"));
        roots
            .add(cert)
            .unwrap_or_else(|error| panic!("root CA is a certificate: {error}"));
    }
    let config = ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_protocol_versions(&[&TLS13, &TLS12])
    .unwrap_or_else(|error| panic!("protocol versions: {error}"))
    .with_root_certificates(roots)
    .with_no_client_auth();
    let name =
        ServerName::try_from("localhost").unwrap_or_else(|error| panic!("server name: {error}"));
    let connection = ClientConnection::new(Arc::new(config), name)
        .unwrap_or_else(|error| panic!("client connection: {error}"));
    StreamOwned::new(connection, tcp)
}

/// Completes the TLS handshake and returns the leaf certificate the server
/// presented, DER.
fn handshake(stream: &mut Tls) -> Vec<u8> {
    while stream.conn.is_handshaking() {
        stream
            .conn
            .complete_io(&mut stream.sock)
            .unwrap_or_else(|error| panic!("handshake: {error}"));
    }
    stream
        .conn
        .peer_certificates()
        .and_then(|chain| chain.first())
        .map(|leaf| leaf.as_ref().to_vec())
        .unwrap_or_else(|| panic!("no server certificate"))
}

/// The leaf certificate that a new handshake to the WSS port presents.
fn wss_leaf(port: u16) -> Vec<u8> {
    handshake(&mut tls_client(connect_from([127, 0, 0, 1], port)))
}

/// The first certificate of a fixture chain, DER.
fn leaf_of(chain: &str) -> Vec<u8> {
    CertificateDer::pem_file_iter(tls_file(chain))
        .unwrap_or_else(|error| panic!("open {chain}: {error}"))
        .next()
        .unwrap_or_else(|| panic!("no certificate in {chain}"))
        .unwrap_or_else(|error| panic!("parse {chain}: {error}"))
        .as_ref()
        .to_vec()
}

/// `closed_within` for a TLS connection: a close or reset is seen as a close,
/// and no data within `wait` is still open.
fn tls_closed_within(stream: &mut Tls, wait: Duration) -> bool {
    stream
        .sock
        .set_read_timeout(Some(wait))
        .unwrap_or_else(|error| panic!("read timeout: {error}"));
    let mut buf = [0; 64];
    match stream.read(&mut buf) {
        Ok(0) => true,
        Ok(n) => panic!("the server sent {n} bytes"),
        Err(error) => match error.kind() {
            ErrorKind::ConnectionReset
            | ErrorKind::ConnectionAborted
            | ErrorKind::UnexpectedEof => true,
            ErrorKind::WouldBlock | ErrorKind::TimedOut => false,
            _ => panic!("read: {error}"),
        },
    }
}

/// A WebSocket upgrade to `/relay/viewer` with the RFC 6455 sample key.
fn upgrade_request() -> Vec<u8> {
    b"GET /relay/viewer HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"
        .to_vec()
}

/// Reads a reply head up to the blank line, or up to the close.
fn read_head(stream: &mut impl Read) -> String {
    let mut head = Vec::new();
    let mut byte = [0u8];
    while !head.ends_with(b"\r\n\r\n") {
        match stream.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => head.push(byte[0]),
            Err(error) => panic!("read head: {error}"),
        }
    }
    String::from_utf8_lossy(&head).into_owned()
}

/// The status code of a reply head.
fn status_of(head: &str) -> u16 {
    head.split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status in {head:?}"))
}

/// spec: operations.md:OPS-025
/// spec: operations.md:OPS-040 partial
#[test]
fn allow_change_keeps_waiting_connections_and_applies_to_new_decisions() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = setup("allow-change", &config_text(ports, TWO_SOURCES, CERT, KEY));
    let log = log_of(&config);
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);

    let mut greeting = [0u8; 12];

    // State 3: Active relays over TCP, WS, and WSS before the reload.
    let mut tcp_v = connect_from([127, 0, 0, 1], ports.tcp_viewer);
    tcp_v
        .read_exact(&mut greeting)
        .unwrap_or_else(|error| panic!("tcp greeting: {error}"));
    assert_eq!(&greeting, GREETING);
    tcp_v
        .write_all(&request("2503"))
        .unwrap_or_else(|error| panic!("tcp v req: {error}"));
    tcp_v
        .flush()
        .unwrap_or_else(|error| panic!("tcp v flush: {error}"));

    let mut tcp_s = connect_from([127, 0, 0, 2], ports.tcp_server);
    tcp_s
        .write_all(&request("2503"))
        .unwrap_or_else(|error| panic!("tcp s req: {error}"));
    tcp_s
        .flush()
        .unwrap_or_else(|error| panic!("tcp s flush: {error}"));
    wait_for_count(&log, 1, |line| event_is(line, "session_started"));

    tcp_v
        .write_all(b"tcp-pre-v2s")
        .unwrap_or_else(|error| panic!("tcp write: {error}"));
    tcp_v
        .flush()
        .unwrap_or_else(|error| panic!("tcp flush: {error}"));
    let mut buf = [0u8; 11];
    tcp_s
        .read_exact(&mut buf)
        .unwrap_or_else(|error| panic!("tcp read: {error}"));
    assert_eq!(&buf, b"tcp-pre-v2s");
    tcp_s
        .write_all(b"tcp-pre-s2v")
        .unwrap_or_else(|error| panic!("tcp write: {error}"));
    tcp_s
        .flush()
        .unwrap_or_else(|error| panic!("tcp flush: {error}"));
    tcp_v
        .read_exact(&mut buf)
        .unwrap_or_else(|error| panic!("tcp read: {error}"));
    assert_eq!(&buf, b"tcp-pre-s2v");

    let mut ws_v = WebSocket::upgrade(Box::new(connect_from([127, 0, 0, 1], ports.ws)));
    assert_eq!(ws_v.read_data(12), GREETING);
    ws_v.send_binary(&request("2504"));

    let mut ws_s = connect_from([127, 0, 0, 2], ports.tcp_server);
    ws_s.write_all(&request("2504"))
        .unwrap_or_else(|error| panic!("ws s req: {error}"));
    ws_s.flush()
        .unwrap_or_else(|error| panic!("ws s flush: {error}"));
    wait_for_count(&log, 2, |line| event_is(line, "session_started"));

    ws_v.send_binary(b"ws-pre-v2s");
    let mut buf_ws = [0u8; 10];
    ws_s.read_exact(&mut buf_ws)
        .unwrap_or_else(|error| panic!("ws read: {error}"));
    assert_eq!(&buf_ws, b"ws-pre-v2s");
    ws_s.write_all(b"ws-pre-s2v")
        .unwrap_or_else(|error| panic!("ws write: {error}"));
    ws_s.flush()
        .unwrap_or_else(|error| panic!("ws flush: {error}"));
    assert_eq!(ws_v.read_data(10), b"ws-pre-s2v");

    let mut wss_v = WebSocket::upgrade(Box::new(tls_client(connect_from(
        [127, 0, 0, 1],
        ports.wss,
    ))));
    assert_eq!(wss_v.read_data(12), GREETING);
    wss_v.send_binary(&request("2505"));

    let mut wss_s = connect_from([127, 0, 0, 2], ports.tcp_server);
    wss_s
        .write_all(&request("2505"))
        .unwrap_or_else(|error| panic!("wss s req: {error}"));
    wss_s
        .flush()
        .unwrap_or_else(|error| panic!("wss s flush: {error}"));
    wait_for_count(&log, 3, |line| event_is(line, "session_started"));

    wss_v.send_binary(b"wss-pre-v2s");
    let mut buf_wss = [0u8; 11];
    wss_s
        .read_exact(&mut buf_wss)
        .unwrap_or_else(|error| panic!("wss read: {error}"));
    assert_eq!(&buf_wss, b"wss-pre-v2s");
    wss_s
        .write_all(b"wss-pre-s2v")
        .unwrap_or_else(|error| panic!("wss write: {error}"));
    wss_s
        .flush()
        .unwrap_or_else(|error| panic!("wss flush: {error}"));
    assert_eq!(wss_v.read_data(11), b"wss-pre-s2v");

    // State 2: Mode II waiting registration before reload.
    let mut waiting_s = connect_from([127, 0, 0, 1], ports.tcp_server);
    waiting_s
        .write_all(&request("2502"))
        .unwrap_or_else(|error| panic!("waiting write: {error}"));
    waiting_s
        .flush()
        .unwrap_or_else(|error| panic!("waiting flush: {error}"));
    wait_for_count(&log, 1, |line| event_is(line, "waiting_registered"));

    // State 1: Connection mid initial exchange before reload.
    let mut mid_v = connect_from([127, 0, 0, 1], ports.tcp_viewer);
    mid_v
        .read_exact(&mut greeting)
        .unwrap_or_else(|error| panic!("mid greeting: {error}"));
    assert_eq!(&greeting, GREETING);
    let req2501 = request("2501");
    mid_v
        .write_all(&req2501[..50])
        .unwrap_or_else(|error| panic!("mid partial write: {error}"));
    mid_v
        .flush()
        .unwrap_or_else(|error| panic!("mid flush: {error}"));

    // Reload: removes 127.0.0.1, keeps 127.0.0.2, adds 127.0.0.3.
    write(&config, &config_text(ports, RANGE, CERT, KEY));
    let applied = wait_for_line(&log, |line| event_is(line, "reload_applied"));
    assert!(applied.contains("relay.tcp_viewer.allow"), "{applied}");
    assert!(applied.contains("relay.tcp_server.allow"), "{applied}");
    assert!(applied.contains("relay.ws.viewer.allow"), "{applied}");
    assert!(applied.contains("relay.wss.viewer.allow"), "{applied}");

    // State 3: Active relays keep forwarding exact bytes both ways after reload.
    tcp_v
        .write_all(b"tcp-post-v2s")
        .unwrap_or_else(|error| panic!("tcp post write: {error}"));
    tcp_v
        .flush()
        .unwrap_or_else(|error| panic!("tcp post flush: {error}"));
    let mut buf_post_tcp = [0u8; 12];
    tcp_s
        .read_exact(&mut buf_post_tcp)
        .unwrap_or_else(|error| panic!("tcp post read: {error}"));
    assert_eq!(&buf_post_tcp, b"tcp-post-v2s");
    tcp_s
        .write_all(b"tcp-post-s2v")
        .unwrap_or_else(|error| panic!("tcp post write: {error}"));
    tcp_s
        .flush()
        .unwrap_or_else(|error| panic!("tcp post flush: {error}"));
    tcp_v
        .read_exact(&mut buf_post_tcp)
        .unwrap_or_else(|error| panic!("tcp post read: {error}"));
    assert_eq!(&buf_post_tcp, b"tcp-post-s2v");

    ws_v.send_binary(b"ws-post-v2s");
    let mut buf_post_ws = [0u8; 11];
    ws_s.read_exact(&mut buf_post_ws)
        .unwrap_or_else(|error| panic!("ws post read: {error}"));
    assert_eq!(&buf_post_ws, b"ws-post-v2s");
    ws_s.write_all(b"ws-post-s2v")
        .unwrap_or_else(|error| panic!("ws post write: {error}"));
    ws_s.flush()
        .unwrap_or_else(|error| panic!("ws post flush: {error}"));
    assert_eq!(ws_v.read_data(11), b"ws-post-s2v");

    wss_v.send_binary(b"wss-post-v2s");
    let mut buf_post_wss = [0u8; 12];
    wss_s
        .read_exact(&mut buf_post_wss)
        .unwrap_or_else(|error| panic!("wss post read: {error}"));
    assert_eq!(&buf_post_wss, b"wss-post-v2s");
    wss_s
        .write_all(b"wss-post-s2v")
        .unwrap_or_else(|error| panic!("wss post write: {error}"));
    wss_s
        .flush()
        .unwrap_or_else(|error| panic!("wss post flush: {error}"));
    assert_eq!(wss_v.read_data(12), b"wss-post-s2v");

    // State 2: Waiting registration matches counterpart arriving after reload.
    let mut counterpart_s2 = connect_from([127, 0, 0, 2], ports.tcp_viewer);
    counterpart_s2
        .read_exact(&mut greeting)
        .unwrap_or_else(|error| panic!("greeting: {error}"));
    assert_eq!(&greeting, GREETING);
    counterpart_s2
        .write_all(&request("2502"))
        .unwrap_or_else(|error| panic!("counterpart s2 write: {error}"));
    counterpart_s2
        .flush()
        .unwrap_or_else(|error| panic!("counterpart s2 flush: {error}"));
    wait_for_count(&log, 4, |line| event_is(line, "session_started"));

    waiting_s
        .write_all(b"s2-post-s2v")
        .unwrap_or_else(|error| panic!("s2 write: {error}"));
    waiting_s
        .flush()
        .unwrap_or_else(|error| panic!("s2 flush: {error}"));
    let mut buf_s2 = [0u8; 11];
    counterpart_s2
        .read_exact(&mut buf_s2)
        .unwrap_or_else(|error| panic!("s2 read: {error}"));
    assert_eq!(&buf_s2, b"s2-post-s2v");
    counterpart_s2
        .write_all(b"s2-post-v2s")
        .unwrap_or_else(|error| panic!("s2 write: {error}"));
    counterpart_s2
        .flush()
        .unwrap_or_else(|error| panic!("s2 flush: {error}"));
    waiting_s
        .read_exact(&mut buf_s2)
        .unwrap_or_else(|error| panic!("s2 read: {error}"));
    assert_eq!(&buf_s2, b"s2-post-v2s");

    // State 1: Connection mid initial exchange completes, registers, and matches.
    mid_v
        .write_all(&req2501[50..])
        .unwrap_or_else(|error| panic!("mid complete write: {error}"));
    mid_v
        .flush()
        .unwrap_or_else(|error| panic!("mid complete flush: {error}"));
    wait_for_count(&log, 2, |line| event_is(line, "waiting_registered"));

    let mut counterpart_s1 = connect_from([127, 0, 0, 2], ports.tcp_server);
    counterpart_s1
        .write_all(&request("2501"))
        .unwrap_or_else(|error| panic!("counterpart s1 write: {error}"));
    counterpart_s1
        .flush()
        .unwrap_or_else(|error| panic!("counterpart s1 flush: {error}"));
    wait_for_count(&log, 5, |line| event_is(line, "session_started"));

    mid_v
        .write_all(b"s1-post-v2s")
        .unwrap_or_else(|error| panic!("s1 write: {error}"));
    mid_v
        .flush()
        .unwrap_or_else(|error| panic!("s1 flush: {error}"));
    let mut buf_s1 = [0u8; 11];
    counterpart_s1
        .read_exact(&mut buf_s1)
        .unwrap_or_else(|error| panic!("s1 read: {error}"));
    assert_eq!(&buf_s1, b"s1-post-v2s");
    counterpart_s1
        .write_all(b"s1-post-s2v")
        .unwrap_or_else(|error| panic!("s1 write: {error}"));
    counterpart_s1
        .flush()
        .unwrap_or_else(|error| panic!("s1 flush: {error}"));
    mid_v
        .read_exact(&mut buf_s1)
        .unwrap_or_else(|error| panic!("s1 read: {error}"));
    assert_eq!(&buf_s1, b"s1-post-s2v");

    // New decisions use the new list: 127.0.0.1 is refused on all entry points.
    assert!(closed_within(
        &mut connect_from([127, 0, 0, 1], ports.tcp_viewer),
        EXIT_LIMIT
    ));
    assert!(closed_within(
        &mut connect_from([127, 0, 0, 1], ports.tcp_server),
        EXIT_LIMIT
    ));
    assert!(closed_within(
        &mut connect_from([127, 0, 0, 1], ports.ws),
        EXIT_LIMIT
    ));
    assert!(closed_within(
        &mut connect_from([127, 0, 0, 1], ports.wss),
        EXIT_LIMIT
    ));

    // Outside range 127.0.0.4 is refused.
    assert!(closed_within(
        &mut connect_from([127, 0, 0, 4], ports.tcp_viewer),
        EXIT_LIMIT
    ));
    wait_for_line(&log, |line| {
        is(line, "connection_rejected", "tcp_viewer", "127.0.0.4")
    });

    // Still-allowed source 127.0.0.2 is accepted.
    let mut second = connect_from([127, 0, 0, 2], ports.tcp_viewer);
    assert!(!closed_within(&mut second, SETTLE), "127.0.0.2 refused");

    // Newly added source 127.0.0.3 is accepted on TCP, WS, and WSS.
    let mut new_tcp_v = connect_from([127, 0, 0, 3], ports.tcp_viewer);
    new_tcp_v
        .read_exact(&mut greeting)
        .unwrap_or_else(|error| panic!("new greeting: {error}"));
    assert_eq!(&greeting, GREETING);

    let mut new_tcp_s = connect_from([127, 0, 0, 3], ports.tcp_server);
    assert!(!closed_within(&mut new_tcp_s, SETTLE));

    new_tcp_v
        .write_all(&request("2506"))
        .unwrap_or_else(|error| panic!("new v write: {error}"));
    new_tcp_v
        .flush()
        .unwrap_or_else(|error| panic!("new v flush: {error}"));
    new_tcp_s
        .write_all(&request("2506"))
        .unwrap_or_else(|error| panic!("new s write: {error}"));
    new_tcp_s
        .flush()
        .unwrap_or_else(|error| panic!("new s flush: {error}"));
    wait_for_count(&log, 6, |line| event_is(line, "session_started"));

    new_tcp_v
        .write_all(b"new-v2s")
        .unwrap_or_else(|error| panic!("new write: {error}"));
    new_tcp_v
        .flush()
        .unwrap_or_else(|error| panic!("new flush: {error}"));
    let mut buf_new = [0u8; 7];
    new_tcp_s
        .read_exact(&mut buf_new)
        .unwrap_or_else(|error| panic!("new read: {error}"));
    assert_eq!(&buf_new, b"new-v2s");
    new_tcp_s
        .write_all(b"new-s2v")
        .unwrap_or_else(|error| panic!("new write: {error}"));
    new_tcp_s
        .flush()
        .unwrap_or_else(|error| panic!("new flush: {error}"));
    new_tcp_v
        .read_exact(&mut buf_new)
        .unwrap_or_else(|error| panic!("new read: {error}"));
    assert_eq!(&buf_new, b"new-s2v");

    let mut new_ws = WebSocket::upgrade(Box::new(connect_from([127, 0, 0, 3], ports.ws)));
    assert_eq!(new_ws.read_data(12), GREETING);

    let mut new_wss = WebSocket::upgrade(Box::new(tls_client(connect_from(
        [127, 0, 0, 3],
        ports.wss,
    ))));
    assert_eq!(new_wss.read_data(12), GREETING);

    wait_for_line(&log, |line| {
        is(line, "connection_accepted", "tcp_viewer", "127.0.0.3")
    });
    wait_for_line(&log, |line| {
        is(line, "connection_accepted", "tcp_server", "127.0.0.3")
    });
    wait_for_line(&log, |line| {
        is(line, "connection_accepted", "ws", "127.0.0.3")
    });
    wait_for_line(&log, |line| {
        is(line, "connection_accepted", "wss", "127.0.0.3")
    });
}

/// spec: operations.md:OPS-026
#[test]
fn rejected_candidates_keep_the_old_settings_certificate_and_connections() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let dir = scratch("candidates-rejected");
    let cert = copy_tls(&dir, CERT, "cert.pem");
    let key = copy_tls(&dir, KEY, "key.pem");
    let config = dir.join("relay.toml");
    write(&config, &config_text(ports, VIEWER_ONLY, &cert, &key));
    let log = log_of(&config);
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);
    let old = leaf_of(CERT);
    // Held under the running list and certificate before any candidate.
    let mut waiting = connect_from([127, 0, 0, 1], ports.tcp_viewer);
    let mut tls = tls_client(connect_from([127, 0, 0, 1], ports.wss));
    handshake(&mut tls);

    // Syntax error: the array is not closed.
    write(&config, &config_text(ports, r#"["127.0.0.2""#, &cert, &key));
    wait_for_count(&log, 1, |line| {
        reason_is(line, "reload_rejected", "config_syntax_error")
    });
    // Bad item: `not-an-ip` is neither an address nor a range.
    write(
        &config,
        &config_text(ports, r#"["127.0.0.2", "not-an-ip"]"#, &cert, &key),
    );
    wait_for_count(&log, 1, |line| {
        reason_is(line, "reload_rejected", "config_ip_item_invalid")
    });
    // The valid list is back, so the next candidate fails on its TLS files only.
    write(&config, &config_text(ports, VIEWER_ONLY, &cert, &key));

    // One file of the pair replaced: the certificate of another key pair.
    copy_tls(&dir, "ecdsa-p384-chain.pem", "cert.pem");
    wait_for_count(&log, 1, |line| tls_rejected(line, KEY_KEY, "pair mismatch"));
    // The old certificate is back; the key is replaced by another key.
    copy_tls(&dir, CERT, "cert.pem");
    copy_tls(&dir, "other-key-p256.pem", "key.pem");
    wait_for_count(&log, 2, |line| tls_rejected(line, KEY_KEY, "pair mismatch"));

    // The config file is gone for a while.
    std::fs::remove_file(&config).unwrap_or_else(|error| panic!("remove config: {error}"));
    wait_for_count(&log, 1, |line| {
        reason_is(line, "reload_rejected", "config_file_unreadable")
    });
    // The config is back with a valid list, but the key pair still mismatches.
    write(&config, &config_text(ports, RANGE, &cert, &key));
    wait_for_count(&log, 3, |line| tls_rejected(line, KEY_KEY, "pair mismatch"));
    // The key file is gone for a while.
    std::fs::remove_file(dir.join("key.pem")).unwrap_or_else(|error| panic!("remove key: {error}"));
    wait_for_count(&log, 1, |line| {
        tls_rejected(line, KEY_KEY, "file unreadable")
    });

    // Nothing was applied: the old list, certificate and connections stay.
    assert_eq!(logged(&log, |line| event_is(line, "reload_applied")), 0);
    assert_eq!(wss_leaf(ports.wss), old, "old certificate replaced");
    assert!(
        !closed_within(&mut waiting, SETTLE),
        "rejected candidates closed the waiting connection"
    );
    assert!(
        !tls_closed_within(&mut tls, SETTLE),
        "rejected candidates closed the open TLS connection"
    );
    assert!(closed_within(
        &mut connect_from([127, 0, 0, 2], ports.tcp_viewer),
        EXIT_LIMIT
    ));
    assert!(!closed_within(
        &mut connect_from([127, 0, 0, 1], ports.tcp_viewer),
        SETTLE
    ));
}

/// spec: operations.md:OPS-040 partial
#[test]
fn ip_item_candidates_with_reversed_mixed_or_bad_prefix_are_rejected() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = setup(
        "ip-items-rejected",
        &config_text(ports, VIEWER_ONLY, CERT, KEY),
    );
    let log = log_of(&config);
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);
    // Held under the running list before any candidate is written.
    let mut kept = connect_from([127, 0, 0, 1], ports.tcp_viewer);

    // Reversed range, then a range of two families, then a prefix beyond 32.
    write(
        &config,
        &config_text(ports, r#"["127.0.0.3-127.0.0.2"]"#, CERT, KEY),
    );
    wait_for_count(&log, 1, |line| {
        reason_is(line, "reload_rejected", "config_ip_range_reversed")
    });
    write(
        &config,
        &config_text(ports, r#"["127.0.0.2-::1"]"#, CERT, KEY),
    );
    wait_for_count(&log, 1, |line| {
        reason_is(line, "reload_rejected", "config_ip_range_family_mixed")
    });
    write(
        &config,
        &config_text(ports, r#"["127.0.0.1/33"]"#, CERT, KEY),
    );
    wait_for_count(&log, 1, |line| {
        reason_is(line, "reload_rejected", "config_ip_item_invalid")
    });

    // Nothing was applied, and the connection held under the old list stays open.
    assert_eq!(logged(&log, |line| event_is(line, "reload_applied")), 0);
    assert!(
        !closed_within(&mut kept, SETTLE),
        "rejected candidates closed the connection held under the old list"
    );
    assert!(closed_within(
        &mut connect_from([127, 0, 0, 2], ports.tcp_viewer),
        EXIT_LIMIT
    ));
}

/// spec: operations.md:OPS-040 partial
#[test]
fn ipv6_items_decide_the_loopback_source_at_their_boundaries() {
    let _lock = lock_ports();
    let ports = Ports::free();
    // tcp_viewer listens on `[::1]` only, so every IPv6 decision is about `::1`.
    let v6 = |allow: &str| {
        config_text(ports, allow, CERT, KEY).replace(
            &format!("\"127.0.0.1:{}\"", ports.tcp_viewer),
            &format!("\"[::1]:{}\"", ports.tcp_viewer),
        )
    };
    let config = setup("ipv6-items", &v6(r#"["::2"]"#));
    let log = log_of(&config);
    let mut guard = Guard(Some(spawn(&config)));
    wait_until(guard.child(), || {
        connects_v6(ports.tcp_viewer)
            && [
                ports.status,
                ports.management,
                ports.tcp_server,
                ports.ws,
                ports.wss,
            ]
            .into_iter()
            .all(connects)
    });

    // Only `::2` is listed, so `::1` is refused: it is just below the item.
    assert!(closed_within(&mut connect_v6(ports.tcp_viewer), EXIT_LIMIT));
    // Inside: the single address, the last address of `::/127`, and the first
    // address of a range that starts at `::1`.
    let mut applied = 0;
    for allow in [r#"["::1"]"#, r#"["::/127"]"#, r#"["::1-::ff"]"#] {
        applied += 1;
        write(&config, &v6(allow));
        wait_for_count(&log, applied, |line| event_is(line, "reload_applied"));
        assert!(
            !closed_within(&mut connect_v6(ports.tcp_viewer), SETTLE),
            "{allow} refused ::1"
        );
    }
    // Outside: a range and a prefix that start just after `::1`, and a single
    // address that ends just before it.
    for allow in [r#"["::2-::ff"]"#, r#"["::2/127"]"#, r#"["::/128"]"#] {
        applied += 1;
        write(&config, &v6(allow));
        wait_for_count(&log, applied, |line| event_is(line, "reload_applied"));
        assert!(
            closed_within(&mut connect_v6(ports.tcp_viewer), EXIT_LIMIT),
            "{allow} admitted ::1"
        );
    }
}

/// Whether a log line rejects a candidate for a TLS file at `config_key`
/// with `detail`.
fn tls_rejected(line: &str, config_key: &str, detail: &str) -> bool {
    reason_is(line, "reload_rejected", "tls_file_invalid")
        && field(line, "config_key") == Some(config_key)
        && field(line, "detail") == Some(detail)
}

/// Connects to `[::1]:port`. The source address is `::1` too.
fn connect_v6(port: u16) -> TcpStream {
    TcpStream::connect(SocketAddr::from((Ipv6Addr::LOCALHOST, port)))
        .unwrap_or_else(|error| panic!("connect [::1]:{port}: {error}"))
}

/// Whether `[::1]:port` accepts a TCP connection.
fn connects_v6(port: u16) -> bool {
    let addr = SocketAddr::from((Ipv6Addr::LOCALHOST, port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok()
}

/// spec: operations.md:OPS-024
#[test]
fn new_tls_pair_is_used_by_new_handshakes_and_keeps_open_connections() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = setup("tls-pair", &config_text(ports, VIEWER_ONLY, CERT, KEY));
    let log = log_of(&config);
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);
    let old = leaf_of(CERT);
    let new = leaf_of("ecdsa-p384-chain.pem");
    assert_ne!(old, new);

    // A TLS connection that is open before the reload.
    let mut open = tls_client(connect_from([127, 0, 0, 1], ports.wss));
    assert_eq!(handshake(&mut open), old);

    let before_write = Instant::now();
    write(
        &config,
        &config_text(
            ports,
            VIEWER_ONLY,
            "ecdsa-p384-chain.pem",
            "ecdsa-p384-key.pem",
        ),
    );
    let detected_after = line_written_after(&log, before_write, |line| {
        event_is(line, "reload_change_detected")
    });
    let applied = wait_for_line(&log, |line| event_is(line, "reload_applied"));
    let applied_by = Instant::now();
    assert!(applied.contains("relay.wss.cert_file"), "{applied}");
    // D is measured from the detection to the result. The log `ts` is wall-clock
    // time, and WSL2 steps that clock by seconds during a run, so the interval
    // uses the monotonic clock. It is never shorter than the logged interval:
    // the detection line was written after `detected_after`, and the result
    // line before `applied_by`.
    let elapsed = applied_by.duration_since(detected_after);
    assert!(
        elapsed <= RELOAD_SETTLE,
        "result within {elapsed:?} of the detection, D is {RELOAD_SETTLE:?}: {applied}"
    );

    assert_eq!(
        wss_leaf(ports.wss),
        new,
        "a new handshake did not get the new certificate"
    );
    assert!(
        !tls_closed_within(&mut open, SETTLE),
        "open TLS connection was closed"
    );
}

/// spec: operations.md:OPS-035
#[test]
fn restart_only_change_holds_back_the_ip_change_until_the_port_is_reverted() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = setup(
        "restart-then-ip",
        &config_text(ports, VIEWER_ONLY, CERT, KEY),
    );
    let log = log_of(&config);
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);

    // Candidate 1: a new tcp_viewer port together with a new allow list.
    let moved = free_port();
    write(
        &config,
        &config_text(
            Ports {
                tcp_viewer: moved,
                ..ports
            },
            SECOND_ONLY,
            CERT,
            KEY,
        ),
    );
    let line = wait_for_line(&log, |line| event_is(line, "reload_restart_required"));
    assert!(line.contains("relay.tcp_viewer.listen"), "{line}");
    // The old port still applies the old list, and the new port is not open.
    assert!(closed_within(
        &mut connect_from([127, 0, 0, 2], ports.tcp_viewer),
        EXIT_LIMIT
    ));
    assert!(!closed_within(
        &mut connect_from([127, 0, 0, 1], ports.tcp_viewer),
        SETTLE
    ));
    assert!(!connects(moved));

    // Candidate 2: the port is reverted and only the allow list changes.
    write(&config, &config_text(ports, SECOND_ONLY, CERT, KEY));
    wait_for_count(&log, 1, |line| event_is(line, "reload_applied"));
    let mut allowed = connect_from([127, 0, 0, 2], ports.tcp_viewer);
    assert!(
        !closed_within(&mut allowed, SETTLE),
        "127.0.0.2 refused after the IP change"
    );
    assert!(closed_within(
        &mut connect_from([127, 0, 0, 1], ports.tcp_viewer),
        EXIT_LIMIT
    ));
}

/// spec: operations.md:OPS-035
#[test]
fn restart_only_change_holds_back_a_certificate_change_until_the_port_is_reverted() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = setup(
        "restart-then-cert",
        &config_text(ports, VIEWER_ONLY, CERT, KEY),
    );
    let log = log_of(&config);
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);
    let old = leaf_of(CERT);
    assert_ne!(old, leaf_of("ecdsa-p384-chain.pem"));

    // Candidate 1: a new tcp_viewer port together with a new certificate pair.
    let moved = free_port();
    write(
        &config,
        &config_text(
            Ports {
                tcp_viewer: moved,
                ..ports
            },
            VIEWER_ONLY,
            "ecdsa-p384-chain.pem",
            "ecdsa-p384-key.pem",
        ),
    );
    let line = wait_for_line(&log, |line| event_is(line, "reload_restart_required"));
    assert!(line.contains("relay.tcp_viewer.listen"), "{line}");
    // The whole candidate is refused, so the old certificate and port stay.
    assert_eq!(wss_leaf(ports.wss), old, "rejected certificate applied");
    assert!(!connects(moved));

    // Candidate 2: the port is reverted and only the allow list changes.
    write(&config, &config_text(ports, SECOND_ONLY, CERT, KEY));
    wait_for_count(&log, 1, |line| event_is(line, "reload_applied"));
    assert!(
        !closed_within(&mut connect_from([127, 0, 0, 2], ports.tcp_viewer), SETTLE),
        "127.0.0.2 refused after the IP change"
    );
    // The wss allow list is now 127.0.0.2 only, so the check connects from it.
    let leaf = handshake(&mut tls_client(connect_from([127, 0, 0, 2], ports.wss)));
    assert_eq!(leaf, old, "rejected certificate came back");
}

/// spec: operations.md:OPS-003
#[test]
fn restart_only_change_applies_at_the_next_start() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = setup("restart-only", &config_text(ports, VIEWER_ONLY, CERT, KEY));
    let log = log_of(&config);
    let mut first = Guard(Some(spawn(&config)));
    wait_listening(first.child(), ports);

    // Listen/accounts require restart, so they also hold back the dynamic limit and allow list.
    let moved = Ports {
        tcp_viewer: free_port(),
        ..ports
    };
    write(
        &config.with_file_name("admin-accounts-next.toml"),
        &accounts_text(),
    );
    let candidate = config_text(moved, SECOND_ONLY, CERT, KEY)
        .replace(
            "accounts_file = \"admin-accounts.toml\"",
            "accounts_file = \"admin-accounts-next.toml\"",
        )
        .replace(
            "[timeouts]",
            "[limits]\nadmin_connections = 1\n\n[timeouts]",
        );
    write(&config, &candidate);
    let line = wait_for_line(&log, |line| event_is(line, "reload_restart_required"));
    for key in ["relay.tcp_viewer.listen", "admin.accounts_file"] {
        assert!(line.contains(key), "{key} missing from {line}");
    }
    assert!(!line.contains("limits.admin_connections"), "{line}");
    // The whole candidate waits: nothing is applied, and the running listener and allow list stay.
    assert_eq!(logged(&log, |line| event_is(line, "reload_applied")), 0);
    assert!(connects(ports.tcp_viewer), "old port closed while running");
    assert!(!connects(moved.tcp_viewer), "new port opened while running");
    assert!(
        !closed_within(&mut connect_from([127, 0, 0, 1], ports.tcp_viewer), SETTLE),
        "running allow list refused 127.0.0.1"
    );
    assert!(closed_within(
        &mut connect_from([127, 0, 0, 2], ports.tcp_viewer),
        EXIT_LIMIT
    ));

    stop(first.child());
    // The old accounts file is removed, so the second start succeeds only with the candidate path.
    std::fs::remove_file(config.with_file_name("admin-accounts.toml"))
        .unwrap_or_else(|error| panic!("remove old accounts file: {error}"));
    let mut second = Guard(Some(spawn(&config)));
    wait_listening(second.child(), moved);
    assert!(
        !connects(ports.tcp_viewer),
        "old port still listens after the restart"
    );
    // The next start applies the whole candidate.
    assert!(
        !closed_within(&mut connect_from([127, 0, 0, 2], moved.tcp_viewer), SETTLE),
        "next start refused 127.0.0.2"
    );
    assert!(closed_within(
        &mut connect_from([127, 0, 0, 1], moved.tcp_viewer),
        EXIT_LIMIT
    ));
    // The new admin limit is 1: a second management connection is refused at accept.
    // connect returns once the kernel queues the socket. The sleep lets the relay accept _held,
    // so the probe is counted after it; the connection_rejected wait below is the real check.
    std::thread::sleep(SETTLE);
    let admin = SocketAddr::from(([127, 0, 0, 1], moved.management));
    let _held = TcpStream::connect(admin).unwrap_or_else(|error| panic!("admin connect: {error}"));
    let _probe = TcpStream::connect(admin).unwrap_or_else(|error| panic!("admin probe: {error}"));
    let refused = wait_for_line(&log, |line| {
        reason_is(line, "connection_rejected", "admin_connection_limit")
            && field(line, "listener") == Some("management")
    });
    assert!(refused.contains("\"limit\":1"), "{refused}");
}

/// spec: operations.md:OPS-038
/// spec: operations.md:OPS-025 partial
#[test]
fn allow_list_modes_hold_at_start_and_at_reload() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = setup("allow-modes", &config_text(ports, ANY, CERT, KEY));
    let log = log_of(&config);
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);
    // All-source is accepted at start, with a startup warning.
    let startup = wait_for_line(&log, |line| {
        event_is(line, "allow_all_source") && field(line, "listener") == Some("tcp_viewer")
    });
    assert_eq!(field(&startup, "phase"), Some("startup"), "{startup}");
    assert!(
        !closed_within(&mut connect_from([127, 0, 0, 2], ports.tcp_viewer), SETTLE),
        "0.0.0.0/0 refused 127.0.0.2 at start"
    );

    // Reload to a specific list, then back to all-source: the warning is recorded again at reload.
    write(&config, &config_text(ports, VIEWER_ONLY, CERT, KEY));
    wait_for_count(&log, 1, |line| event_is(line, "reload_applied"));
    write(&config, &config_text(ports, ANY, CERT, KEY));
    wait_for_line(&log, |line| {
        event_is(line, "allow_all_source")
            && field(line, "listener") == Some("tcp_viewer")
            && field(line, "phase") == Some("reload")
    });
    wait_for_count(&log, 2, |line| event_is(line, "reload_applied"));
    let mut open = connect_from([127, 0, 0, 2], ports.tcp_viewer);
    assert!(
        !closed_within(&mut open, SETTLE),
        "0.0.0.0/0 refused 127.0.0.2 after the reload"
    );

    // An empty list at reload refuses every source; the connection accepted under the old list stays.
    write(&config, &config_text(ports, NONE, CERT, KEY));
    wait_for_count(&log, 3, |line| event_is(line, "reload_applied"));
    assert!(
        !closed_within(&mut open, SETTLE),
        "existing connection was closed"
    );
    assert!(closed_within(
        &mut connect_from([127, 0, 0, 2], ports.tcp_viewer),
        EXIT_LIMIT
    ));
    assert!(closed_within(
        &mut connect_from([127, 0, 0, 1], ports.tcp_viewer),
        EXIT_LIMIT
    ));

    // An empty list at start refuses every source.
    let empty_ports = Ports::free();
    let empty = setup(
        "allow-empty-start",
        &config_text(empty_ports, NONE, CERT, KEY),
    );
    let mut empty_guard = Guard(Some(spawn(&empty)));
    wait_listening(empty_guard.child(), empty_ports);
    assert!(closed_within(
        &mut connect_from([127, 0, 0, 1], empty_ports.tcp_viewer),
        EXIT_LIMIT
    ));
    assert!(closed_within(
        &mut connect_from([127, 0, 0, 2], empty_ports.tcp_viewer),
        EXIT_LIMIT
    ));

    // A missing list fails the start.
    let text =
        config_text(ports, VIEWER_ONLY, CERT, KEY).replace(&format!("allow = {VIEWER_ONLY}\n"), "");
    let missing = setup("allow-missing", &text);
    let output = Command::new(BIN)
        .arg("serve")
        .arg("--config")
        .arg(&missing)
        .stdin(Stdio::null())
        .output()
        .unwrap_or_else(|error| panic!("run {BIN}: {error}"));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "start without an allow list succeeded"
    );
    assert!(stderr.contains("config_missing_key"), "{stderr}");
    assert!(stderr.contains("relay.tcp_viewer.allow"), "{stderr}");
}

/// spec: websocket.md:WS-022
#[test]
fn removed_source_is_refused_at_upgrade_on_connections_opened_before_the_removal() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = setup("ws-removal", &config_text(ports, SECOND_ONLY, CERT, KEY));
    let log = log_of(&config);
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);

    // An already-relaying WS session from the source that will be removed (127.0.0.2).
    let mut relaying_ws = WebSocket::upgrade(Box::new(connect_from([127, 0, 0, 2], ports.ws)));
    assert_eq!(relaying_ws.read_data(12), GREETING);
    relaying_ws.send_binary(&request("2201"));

    let mut relaying_server = connect_from([127, 0, 0, 2], ports.tcp_server);
    relaying_server
        .write_all(&request("2201"))
        .unwrap_or_else(|error| panic!("server req: {error}"));
    relaying_server
        .flush()
        .unwrap_or_else(|error| panic!("server flush: {error}"));
    wait_for_line(&log, |line| event_is(line, "session_started"));

    // Pre-removal data transfer in both directions.
    relaying_ws.send_binary(b"pre-ws2server");
    let mut buf = [0u8; 13];
    relaying_server
        .read_exact(&mut buf)
        .unwrap_or_else(|error| panic!("server read: {error}"));
    assert_eq!(&buf, b"pre-ws2server");
    relaying_server
        .write_all(b"pre-server2ws")
        .unwrap_or_else(|error| panic!("server write: {error}"));
    relaying_server
        .flush()
        .unwrap_or_else(|error| panic!("server flush: {error}"));
    assert_eq!(relaying_ws.read_data(13), b"pre-server2ws");

    // A plain HTTP connection and a TLS connection that send nothing yet.
    let mut held_http = connect_from([127, 0, 0, 2], ports.ws);
    let mut held_tls = tls_client(connect_from([127, 0, 0, 2], ports.wss));
    handshake(&mut held_tls);
    wait_for_count(&log, 2, |line| {
        is(line, "connection_accepted", "ws", "127.0.0.2")
    });
    wait_for_count(&log, 1, |line| {
        is(line, "connection_accepted", "wss", "127.0.0.2")
    });

    write(&config, &config_text(ports, VIEWER_ONLY, CERT, KEY));
    let applied = wait_for_line(&log, |line| event_is(line, "reload_applied"));
    assert!(applied.contains("relay.ws.viewer.allow"), "{applied}");
    assert!(applied.contains("relay.wss.viewer.allow"), "{applied}");

    // The already-relaying WS session keeps forwarding data in both directions after removal.
    relaying_ws.send_binary(b"post-ws2server");
    let mut buf2 = [0u8; 14];
    relaying_server
        .read_exact(&mut buf2)
        .unwrap_or_else(|error| panic!("server read: {error}"));
    assert_eq!(&buf2, b"post-ws2server");
    relaying_server
        .write_all(b"post-server2ws")
        .unwrap_or_else(|error| panic!("server write: {error}"));
    relaying_server
        .flush()
        .unwrap_or_else(|error| panic!("server flush: {error}"));
    assert_eq!(relaying_ws.read_data(14), b"post-server2ws");
    // Upgrades on the held connections are refused at the upgrade re-check.
    held_http
        .write_all(&upgrade_request())
        .unwrap_or_else(|error| panic!("request: {error}"));
    let head = read_head(&mut held_http);
    assert_eq!(status_of(&head), 403, "{head}");
    wait_for_line(&log, |line| {
        is(line, "connection_rejected", "ws", "127.0.0.2")
            && field(line, "stage") == Some("upgrade")
            && field(line, "reason") == Some("ip_not_allowed")
    });

    held_tls
        .write_all(&upgrade_request())
        .unwrap_or_else(|error| panic!("request: {error}"));
    let head = read_head(&mut held_tls);
    assert_eq!(status_of(&head), 403, "{head}");
    wait_for_line(&log, |line| {
        is(line, "connection_rejected", "wss", "127.0.0.2")
            && field(line, "stage") == Some("upgrade")
            && field(line, "reason") == Some("ip_not_allowed")
    });

    // A new connection from the removed source is refused at accept.
    assert!(closed_within(
        &mut connect_from([127, 0, 0, 2], ports.ws),
        EXIT_LIMIT
    ));
    wait_for_line(&log, |line| {
        is(line, "connection_rejected", "ws", "127.0.0.2") && field(line, "stage") == Some("accept")
    });
}

/// Sets a leaf in the fixture TOML without changing unrelated tables.
fn setting(mut text: String, key: &str, value: &str) -> String {
    let (table, leaf) = key
        .rsplit_once('.')
        .unwrap_or_else(|| panic!("leaf key: {key}"));
    let heading = format!("[{table}]\n");
    let assignment = format!("{leaf} = {value}\n");
    if let Some(start) = text.find(&heading) {
        let body = start + heading.len();
        let end = text[body..]
            .find("\n[")
            .map_or(text.len(), |n| body + n + 1);
        let mut offset = body;
        for line in text[body..end].split_inclusive('\n') {
            if line.starts_with(&format!("{leaf} =")) {
                text.replace_range(offset..offset + line.len(), &assignment);
                return text;
            }
            offset += line.len();
        }
        text.insert_str(body, &assignment);
    } else {
        text.push_str(&format!("\n{heading}{assignment}"));
    }
    text
}

const PASSWORD: &str = "correct-horse-battery";

fn authenticated_setup(name: &str, text: &str) -> PathBuf {
    use vnc_relay_adapters::accounts_file::Argon2Hasher;
    use vnc_relay_app::ports::PasswordHasher;
    use vnc_relay_domain::account::{Argon2Params, Password};

    let config = setup(name, text);
    let password =
        Password::parse(PASSWORD.to_owned()).unwrap_or_else(|error| panic!("password: {error:?}"));
    let hash = Argon2Hasher
        .hash(
            &password,
            Argon2Params {
                memory_kib: 19_456,
                iterations: 2,
                parallelism: 1,
            },
        )
        .unwrap_or_else(|error| panic!("hash: {error:?}"));
    write(
        &config.with_file_name("admin-accounts.toml"),
        &format!(
            "version = 1\n\n[[account]]\nname = \"alice\"\npassword_hash = \"{}\"\n",
            hash.as_str()
        ),
    );
    config
}

struct HttpReply {
    status: u16,
    head: String,
    body: serde_json::Value,
}

fn finish_http(
    mut stream: TcpStream,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> HttpReply {
    stream
        .set_read_timeout(Some(EXIT_LIMIT))
        .unwrap_or_else(|error| panic!("HTTP timeout: {error}"));
    let mut request = format!(
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    );
    for (key, value) in headers {
        request.push_str(&format!("{key}: {value}\r\n"));
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
        .unwrap_or_else(|| panic!("HTTP response: {raw}"));
    HttpReply {
        status: status_of(head),
        head: head.to_owned(),
        body: serde_json::from_str(body).unwrap_or_else(|error| panic!("JSON {body}: {error}")),
    }
}

fn http(port: u16, method: &str, path: &str, headers: &[(&str, &str)], body: &str) -> HttpReply {
    finish_http(
        connect_from([127, 0, 0, 1], port),
        method,
        path,
        headers,
        body,
    )
}

fn login(port: u16) -> (String, String, serde_json::Value) {
    let reply = http(
        port,
        "POST",
        "/api/v1/session",
        &[("Content-Type", "application/json")],
        &serde_json::json!({"account": "alice", "password": PASSWORD}).to_string(),
    );
    assert_eq!(reply.status, 200, "{}", reply.body);
    let cookie = reply
        .head
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case("set-cookie")
                .then(|| value.trim().split(';').next().unwrap_or("").to_owned())
        })
        .unwrap_or_else(|| panic!("no cookie: {}", reply.head));
    let csrf = reply.body["csrf_token"]
        .as_str()
        .unwrap_or_else(|| panic!("no CSRF"))
        .to_owned();
    (cookie, csrf, reply.body)
}

fn query(port: u16, cookie: &str, path: &str) -> serde_json::Value {
    let reply = http(port, "GET", path, &[("Cookie", cookie)], "");
    assert_eq!(reply.status, 200, "{}", reply.body);
    reply.body
}

fn poll_query(
    port: u16,
    cookie: &str,
    path: &str,
    ready: impl Fn(&serde_json::Value) -> bool,
) -> serde_json::Value {
    let deadline = Instant::now() + EXIT_LIMIT;
    loop {
        let value = query(port, cookie, path);
        if ready(&value) {
            return value;
        }
        assert!(Instant::now() < deadline, "{path} did not settle: {value}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn effective<'a>(view: &'a serde_json::Value, key: &str) -> &'a serde_json::Value {
    &view["items"]
        .as_array()
        .unwrap_or_else(|| panic!("config items: {view}"))
        .iter()
        .find(|item| item["key"] == key)
        .unwrap_or_else(|| panic!("missing {key}"))["value"]
}

fn reload(config: &Path, text: &str) -> serde_json::Value {
    let log = log_of(config);
    let count = logged(&log, |line| event_is(line, "reload_applied")) + 1;
    write(config, text);
    wait_for_count(&log, count, |line| event_is(line, "reload_applied"));
    let lines = std::fs::read_to_string(&log).unwrap_or_else(|error| panic!("log: {error}"));
    let line = lines
        .lines()
        .filter(|line| event_is(line, "reload_applied"))
        .nth(count - 1)
        .unwrap_or_else(|| panic!("missing reload"));
    let event: serde_json::Value =
        serde_json::from_str(line).unwrap_or_else(|error| panic!("event: {error}"));
    assert_eq!(event["trigger"], "file_watch");
    event
}

fn tcp_viewer(port: u16) -> TcpStream {
    let mut stream = connect_from([127, 0, 0, 1], port);
    let mut greeting = [0; 12];
    stream
        .read_exact(&mut greeting)
        .unwrap_or_else(|error| panic!("greeting: {error}"));
    assert_eq!(&greeting, GREETING);
    stream
}

fn waiting_server(ports: Ports, id: &str) -> TcpStream {
    let mut server = connect_from([127, 0, 0, 1], ports.tcp_server);
    server
        .write_all(&request(id))
        .unwrap_or_else(|error| panic!("register: {error}"));
    server
}

fn pair(ports: Ports, id: &str) -> (TcpStream, TcpStream) {
    let server = waiting_server(ports, id);
    let mut viewer = tcp_viewer(ports.tcp_viewer);
    viewer
        .write_all(&request(id))
        .unwrap_or_else(|error| panic!("match: {error}"));
    (viewer, server)
}

fn exchange(viewer: &mut impl Wire, server: &mut TcpStream) {
    viewer
        .write_all(b"forward")
        .unwrap_or_else(|error| panic!("forward: {error}"));
    viewer
        .flush()
        .unwrap_or_else(|error| panic!("flush: {error}"));
    let mut bytes = [0; 7];
    server
        .read_exact(&mut bytes)
        .unwrap_or_else(|error| panic!("receive: {error}"));
    assert_eq!(&bytes, b"forward");
    server
        .write_all(b"reverse")
        .unwrap_or_else(|error| panic!("reverse: {error}"));
    viewer
        .read_exact(&mut bytes)
        .unwrap_or_else(|error| panic!("receive reverse: {error}"));
    assert_eq!(&bytes, b"reverse");
}

/// spec: operations.md:OPS-064
#[test]
fn reverting_listen_allows_the_entire_dynamic_candidate_without_restart() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let mut original = config_text(ports, VIEWER_ONLY, CERT, KEY);
    original = setting(original, "mode1.enabled", "true");
    original = setting(original, "mode1.allow_targets", "[]");
    original = setting(
        original,
        "relay.ws.viewer.allowed_origins",
        r#"["https://old.example"]"#,
    );
    original = setting(
        original,
        "relay.wss.viewer.allowed_origins",
        r#"["https://old.example"]"#,
    );
    let changes = [
        (
            "relay.ws.viewer.allowed_origins",
            serde_json::json!(["https://new.example"]),
        ),
        (
            "relay.wss.viewer.allowed_origins",
            serde_json::json!(["https://new.example"]),
        ),
        ("mode1.allow_targets", serde_json::json!(["127.0.0.1:5900"])),
        ("limits.relay_connections", serde_json::json!(32)),
        ("limits.admin_connections", serde_json::json!(12)),
        ("limits.waiting", serde_json::json!(12)),
        ("limits.buffer_bytes", serde_json::json!(65536)),
        ("limits.ws_message_bytes", serde_json::json!(131072)),
        ("timeouts.handshake_secs", serde_json::json!(90)),
        ("timeouts.tls_upgrade_secs", serde_json::json!(90)),
        ("timeouts.wait_secs", serde_json::json!(120)),
        ("timeouts.connect_secs", serde_json::json!(20)),
        ("timeouts.close_secs", serde_json::json!(2)),
        ("timeouts.shutdown_grace_secs", serde_json::json!(15)),
        ("keepalive.tcp_idle_secs", serde_json::json!(70)),
        ("keepalive.tcp_interval_secs", serde_json::json!(12)),
        ("keepalive.tcp_probes", serde_json::json!(7)),
        ("keepalive.ws_ping_secs", serde_json::json!(15)),
        ("keepalive.ws_pong_timeout_secs", serde_json::json!(16)),
    ];
    let config = authenticated_setup("all-runtime-listen-revert", &original);
    let log = log_of(&config);
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);
    let (cookie, _, _) = login(ports.management);
    let before = query(ports.management, &cookie, "/api/v1/config");
    let (mut viewer, mut server) = pair(ports, "6401");
    exchange(&mut viewer, &mut server);
    let mut candidate = original;
    for (key, value) in &changes {
        assert_ne!(effective(&before, key), value, "{key} must change");
        candidate = setting(candidate, key, &value.to_string());
    }
    let moved = free_port();
    write(
        &config,
        &setting(
            candidate.clone(),
            "relay.tcp_viewer.listen",
            &format!("\"127.0.0.1:{moved}\""),
        ),
    );
    let rejected = wait_for_line(&log, |line| event_is(line, "reload_restart_required"));
    let rejected: serde_json::Value =
        serde_json::from_str(&rejected).unwrap_or_else(|error| panic!("event: {error}"));
    assert_eq!(
        rejected["config_keys"],
        serde_json::json!(["relay.tcp_viewer.listen"])
    );
    let held = query(ports.management, &cookie, "/api/v1/config");
    for (key, _) in &changes {
        assert_eq!(
            effective(&held, key),
            effective(&before, key),
            "{key} leaked from rejected candidate"
        );
    }
    assert!(!connects(moved));
    exchange(&mut viewer, &mut server);
    let applied = reload(&config, &candidate);
    assert_eq!(
        applied["applied"].as_array().map(Vec::len),
        Some(changes.len())
    );
    let after = query(ports.management, &cookie, "/api/v1/config");
    for (key, value) in &changes {
        assert_eq!(effective(&after, key), value, "{key}");
        assert!(
            applied["applied"]
                .as_array()
                .is_some_and(|keys| keys.iter().any(|item| item.as_str() == Some(*key))),
            "{key}: {applied}"
        );
    }
    assert_eq!(after["file"]["state"], "in_effect");
    assert_eq!(after["last_reload"]["trigger"], "file_watch");
    exchange(&mut viewer, &mut server);
}

fn upgrade_origin(mut wire: Box<dyn Wire>, origin: &str, expected: u16) -> Option<WebSocket> {
    let request = String::from_utf8(upgrade_request())
        .unwrap_or_else(|error| panic!("request: {error}"))
        .replace("\r\n\r\n", &format!("\r\nOrigin: {origin}\r\n\r\n"));
    wire.write_all(request.as_bytes())
        .unwrap_or_else(|error| panic!("upgrade: {error}"));
    wire.flush()
        .unwrap_or_else(|error| panic!("upgrade flush: {error}"));
    let head = read_head(&mut wire);
    assert_eq!(status_of(&head), expected, "{head}");
    (expected == 101).then_some(WebSocket { wire })
}

fn origin_viewer(ports: Ports, secure: bool, origin: &str, expected: u16) -> Option<WebSocket> {
    let wire: Box<dyn Wire> = if secure {
        Box::new(tls_client(connect_from([127, 0, 0, 1], ports.wss)))
    } else {
        Box::new(connect_from([127, 0, 0, 1], ports.ws))
    };
    upgrade_origin(wire, origin, expected)
}

fn ws_exchange(viewer: &mut WebSocket, server: &mut TcpStream) {
    viewer.send_binary(b"forward");
    let mut bytes = [0; 7];
    server
        .read_exact(&mut bytes)
        .unwrap_or_else(|error| panic!("WS receive: {error}"));
    assert_eq!(&bytes, b"forward");
    server
        .write_all(b"reverse")
        .unwrap_or_else(|error| panic!("WS reverse: {error}"));
    assert_eq!(viewer.read_data(7), b"reverse");
}

fn accept_target(listener: &TcpListener) -> TcpStream {
    listener
        .set_nonblocking(true)
        .unwrap_or_else(|error| panic!("target nonblocking: {error}"));
    let deadline = Instant::now() + EXIT_LIMIT;
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream
                    .set_nonblocking(false)
                    .unwrap_or_else(|error| panic!("target blocking: {error}"));
                stream
                    .set_read_timeout(Some(EXIT_LIMIT))
                    .unwrap_or_else(|error| panic!("target timeout: {error}"));
                return stream;
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {}
            Err(error) => panic!("target accept: {error}"),
        }
        assert!(Instant::now() < deadline, "target not connected");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn mode1_viewer(ports: Ports, target: &TcpListener) -> TcpStream {
    let mut viewer = tcp_viewer(ports.tcp_viewer);
    let address = target
        .local_addr()
        .unwrap_or_else(|error| panic!("target address: {error}"))
        .to_string();
    let mut bytes = [0; HANDSHAKE_REQUEST_LEN];
    bytes[..address.len()].copy_from_slice(address.as_bytes());
    viewer
        .write_all(&bytes)
        .unwrap_or_else(|error| panic!("mode1 request: {error}"));
    viewer
}

/// spec: operations.md:OPS-047
/// spec: operations.md:OPS-048
/// spec: operations.md:OPS-049
#[test]
fn origin_and_target_reload_affects_new_checks_and_keeps_established_sessions() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let target = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| panic!("target: {error}"));
    let added =
        TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| panic!("added target: {error}"));
    let old_address = target
        .local_addr()
        .unwrap_or_else(|error| panic!("address: {error}"))
        .to_string();
    let new_address = added
        .local_addr()
        .unwrap_or_else(|error| panic!("address: {error}"))
        .to_string();
    let policy = |origins: serde_json::Value, targets: serde_json::Value| {
        let mut text = setting(
            config_text(ports, VIEWER_ONLY, CERT, KEY),
            "mode1.enabled",
            "true",
        );
        text = setting(text, "mode1.allow_targets", &targets.to_string());
        text = setting(
            text,
            "relay.ws.viewer.allowed_origins",
            &origins.to_string(),
        );
        setting(
            text,
            "relay.wss.viewer.allowed_origins",
            &origins.to_string(),
        )
    };
    let config = setup(
        "origin-target-policy",
        &policy(
            serde_json::json!(["https://old.example"]),
            serde_json::json!([old_address]),
        ),
    );
    let log = log_of(&config);
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);
    let mut established = Vec::new();
    for (secure, id) in [(false, "4701"), (true, "4702")] {
        let mut viewer = origin_viewer(ports, secure, "https://old.example", 101)
            .unwrap_or_else(|| panic!("accepted upgrade"));
        assert_eq!(viewer.read_data(12), GREETING);
        viewer.send_binary(&request(id));
        let server = waiting_server(ports, id);
        established.push((viewer, server));
    }
    let mut mode1 = mode1_viewer(ports, &target);
    let mut target_peer = accept_target(&target);
    exchange(&mut mode1, &mut target_peer);
    wait_for_count(&log, 3, |line| event_is(line, "session_started"));

    // A transport accepted before the addition still uses the current Origin
    // policy when its HTTP upgrade is finally checked.
    let held_add = connect_from([127, 0, 0, 1], ports.ws);
    let both_origins = serde_json::json!(["https://old.example", "https://new.example"]);
    let both_targets = serde_json::json!([old_address, new_address]);
    reload(&config, &policy(both_origins.clone(), both_targets.clone()));
    let _added_upgrade = upgrade_origin(Box::new(held_add), "https://new.example", 101);
    for secure in [false, true] {
        let _new = origin_viewer(ports, secure, "https://new.example", 101);
    }
    let mut new_viewer = mode1_viewer(ports, &added);
    let mut new_target = accept_target(&added);
    exchange(&mut new_viewer, &mut new_target);

    let held_remove = connect_from([127, 0, 0, 1], ports.ws);
    let mut held_tls = tls_client(connect_from([127, 0, 0, 1], ports.wss));
    handshake(&mut held_tls);
    reload(
        &config,
        &policy(
            serde_json::json!(["https://new.example"]),
            serde_json::json!([new_address]),
        ),
    );
    assert!(upgrade_origin(Box::new(held_remove), "https://old.example", 403).is_none());
    assert!(upgrade_origin(Box::new(held_tls), "https://old.example", 403).is_none());
    for secure in [false, true] {
        assert!(origin_viewer(ports, secure, "https://old.example", 403).is_none());
    }
    let mut refused_target = mode1_viewer(ports, &target);
    assert!(closed_within(&mut refused_target, EXIT_LIMIT));
    assert!(
        matches!(target.accept(), Err(error) if error.kind() == ErrorKind::WouldBlock),
        "removed target must not receive a new connection"
    );
    for (viewer, server) in &mut established {
        ws_exchange(viewer, server);
    }
    exchange(&mut mode1, &mut target_peer);
    exchange(&mut new_viewer, &mut new_target);

    reload(&config, &policy(both_origins, both_targets));
    for secure in [false, true] {
        let mut restored = origin_viewer(ports, secure, "https://old.example", 101)
            .unwrap_or_else(|| panic!("restored upgrade"));
        assert_eq!(restored.read_data(12), GREETING);
    }
    let mut restored = mode1_viewer(ports, &target);
    let mut restored_target = accept_target(&target);
    exchange(&mut restored, &mut restored_target);
    for (viewer, server) in &mut established {
        ws_exchange(viewer, server);
    }
    exchange(&mut mode1, &mut target_peer);
    assert_eq!(
        logged(&log, |line| event_is(line, "reload_restart_required")),
        0
    );
}

/// spec: operations.md:OPS-058
#[test]
fn reloaded_all_target_policy_emits_reload_phase_warning() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let mut text = setting(
        config_text(ports, VIEWER_ONLY, CERT, KEY),
        "mode1.enabled",
        "true",
    );
    text = setting(text, "mode1.allow_targets", "[]");
    let config = setup("all-target-warning", &text);
    let log = log_of(&config);
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);
    assert_eq!(logged(&log, |line| event_is(line, "allow_all_target")), 0);
    for targets in [r#"["0.0.0.0/0:5900"]"#, r#"["[::/0]:5900"]"#] {
        text = setting(text, "mode1.allow_targets", targets);
        reload(&config, &text);
    }
    wait_for_count(&log, 2, |line| {
        event_is(line, "allow_all_target") && field(line, "phase") == Some("reload")
    });
    let contents = std::fs::read_to_string(&log).unwrap_or_else(|error| panic!("log: {error}"));
    for line in contents
        .lines()
        .filter(|line| event_is(line, "allow_all_target"))
    {
        let event: serde_json::Value =
            serde_json::from_str(line).unwrap_or_else(|error| panic!("event: {error}"));
        assert_eq!(event["phase"], "reload");
        assert_eq!(event["level"], "warn");
    }
}

fn limited_config(ports: Ports, relay: u32, admin: u32, waiting: u32) -> String {
    let mut text = config_text(ports, VIEWER_ONLY, CERT, KEY);
    for (key, value) in [
        ("limits.relay_connections", relay),
        ("limits.admin_connections", admin),
        ("limits.waiting", waiting),
    ] {
        text = setting(text, key, &value.to_string());
    }
    setting(text, "timeouts.close_secs", "1")
}

fn assert_budget_refusal(log: &Path, port: u16, listener: &str, reason: &str, limit: u32) {
    let count = logged(log, |line| {
        reason_is(line, "connection_rejected", reason) && field(line, "listener") == Some(listener)
    }) + 1;
    assert!(closed_within(
        &mut connect_from([127, 0, 0, 1], port),
        EXIT_LIMIT
    ));
    wait_for_count(log, count, |line| {
        reason_is(line, "connection_rejected", reason)
            && field(line, "listener") == Some(listener)
            && line.contains(&format!("\"limit\":{limit}"))
    });
}

/// spec: operations.md:OPS-051
/// spec: operations.md:OPS-052
/// spec: operations.md:OPS-053
#[test]
fn lowered_relay_and_admin_limits_keep_connections_and_recover_after_release_or_raise() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = authenticated_setup("lower-budgets", &limited_config(ports, 8, 8, 2));
    let log = log_of(&config);
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);
    let (cookie, _, _) = login(ports.management);
    poll_query(ports.management, &cookie, "/api/v1/summary", |value| {
        value["usage"]["relay_connections"]["current"] == 0
    });
    let (mut viewer, mut server) = pair(ports, "5101");
    let (mut second_viewer, mut second_server) = pair(ports, "5102");
    exchange(&mut viewer, &mut server);
    exchange(&mut second_viewer, &mut second_server);
    poll_query(ports.management, &cookie, "/api/v1/summary", |value| {
        value["usage"]["relay_connections"]["current"] == 4
    });

    // L is below the four live sockets; admission changes without revocation.
    reload(&config, &limited_config(ports, 2, 8, 2));
    let summary = query(ports.management, &cookie, "/api/v1/summary");
    assert_eq!(summary["usage"]["relay_connections"]["limit"], 2);
    assert_eq!(summary["usage"]["relay_connections"]["current"], 4);
    assert_budget_refusal(
        &log,
        ports.tcp_server,
        "tcp_server",
        "relay_connection_limit",
        2,
    );
    assert_budget_refusal(&log, ports.ws, "ws", "relay_connection_limit", 2);
    assert_budget_refusal(&log, ports.wss, "wss", "relay_connection_limit", 2);
    exchange(&mut viewer, &mut server);
    exchange(&mut second_viewer, &mut second_server);
    // The independent admin budget still serves queries at exhausted L.
    assert_eq!(
        query(ports.management, &cookie, "/api/v1/session")["account"],
        "alice"
    );

    let held_management = connect_from([127, 0, 0, 1], ports.management);
    let mut held_status = connect_from([127, 0, 0, 1], ports.status);
    let mut second_status = connect_from([127, 0, 0, 1], ports.status);
    // The query's own permit makes four while all three idle holders are alive.
    poll_query(ports.management, &cookie, "/api/v1/summary", |value| {
        value["usage"]["admin_connections"]["current"] == 4
    });
    reload(&config, &limited_config(ports, 2, 2, 2));
    let summary = finish_http(
        held_management,
        "GET",
        "/api/v1/summary",
        &[("Cookie", &cookie)],
        "",
    );
    assert_eq!(summary.status, 200);
    assert_eq!(summary.body["usage"]["admin_connections"]["limit"], 2);
    assert_eq!(summary.body["usage"]["relay_connections"]["limit"], 2);
    assert!(
        summary.body["usage"]["admin_connections"]["current"]
            .as_u64()
            .is_some_and(|n| n >= 3)
    );
    assert!(
        !closed_within(&mut held_status, SETTLE),
        "Lm reduction revoked status"
    );
    assert!(
        !closed_within(&mut second_status, SETTLE),
        "Lm reduction revoked status"
    );
    assert_budget_refusal(
        &log,
        ports.management,
        "management",
        "admin_connection_limit",
        2,
    );
    assert_budget_refusal(&log, ports.status, "status", "admin_connection_limit", 2);
    exchange(&mut viewer, &mut server);

    // Raising L admits a complete new relay while Lm remains exhausted.
    reload(&config, &limited_config(ports, 8, 2, 2));
    let (mut independent_viewer, mut independent_server) = pair(ports, "5201");
    exchange(&mut independent_viewer, &mut independent_server);
    assert_budget_refusal(
        &log,
        ports.management,
        "management",
        "admin_connection_limit",
        2,
    );
    // Raising Lm then admits management and status without releasing holders.
    reload(&config, &limited_config(ports, 8, 8, 2));
    let mut raised_relay = tcp_viewer(ports.tcp_viewer);
    assert!(!closed_within(&mut raised_relay, SETTLE));
    assert_eq!(
        query(ports.management, &cookie, "/api/v1/summary")["usage"]["admin_connections"]["limit"],
        8
    );
    let mut status = connect_from([127, 0, 0, 1], ports.status);
    status
        .write_all(b"GET /livez HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .unwrap_or_else(|error| panic!("status: {error}"));
    assert_eq!(status_of(&read_head(&mut status)), 200);
    drop((raised_relay, status));

    // Lower again, then release below each new limit. Next admissions recover.
    reload(&config, &limited_config(ports, 2, 2, 2));
    drop((
        viewer,
        server,
        second_viewer,
        second_server,
        independent_viewer,
        independent_server,
        held_status,
        second_status,
    ));
    // Existing tasks retire asynchronously; the remaining budget is observed
    // through the real summary before testing the next relay connection.
    let deadline = Instant::now() + EXIT_LIMIT;
    loop {
        let mut stream = connect_from([127, 0, 0, 1], ports.management);
        if !closed_within(&mut stream, Duration::from_millis(30)) {
            let summary = finish_http(stream, "GET", "/api/v1/summary", &[("Cookie", &cookie)], "");
            assert_eq!(summary.status, 200);
            if summary.body["usage"]["relay_connections"]["current"] == 0 {
                break;
            }
        }
        assert!(
            Instant::now() < deadline,
            "released budgets did not recover"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let (mut recovered_viewer, mut recovered_server) = pair(ports, "5301");
    exchange(&mut recovered_viewer, &mut recovered_server);
    assert_eq!(
        query(ports.management, &cookie, "/api/v1/summary")["usage"]["relay_connections"]["limit"],
        2
    );
}

/// spec: operations.md:OPS-054
/// spec: operations.md:OPS-055
/// spec: operations.md:OPS-056
#[test]
fn lowered_waiting_limit_keeps_deadlines_and_matching_then_allows_registration_after_release() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let mut text = setting(limited_config(ports, 12, 8, 4), "timeouts.wait_secs", "18");
    let config = authenticated_setup("lower-waiting", &text);
    let log = log_of(&config);
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);
    let (cookie, _, _) = login(ports.management);
    let mut first = waiting_server(ports, "5401");
    let mut second = waiting_server(ports, "5402");
    let mut deadline_witness = waiting_server(ports, "5403");
    wait_for_count(&log, 3, |line| event_is(line, "waiting_registered"));
    let registered_by = Instant::now();
    let before = query(ports.management, &cookie, "/api/v1/waiting");
    // A restarted old W would now expire later than the original deadline.
    std::thread::sleep(Duration::from_secs(3));
    text = setting(text, "limits.waiting", "1");
    text = setting(text, "timeouts.wait_secs", "10");
    reload(&config, &text);
    let after = query(ports.management, &cookie, "/api/v1/waiting");
    let before_items = before["items"]
        .as_array()
        .unwrap_or_else(|| panic!("waiting items"));
    let after_items = after["items"]
        .as_array()
        .unwrap_or_else(|| panic!("waiting items"));
    assert_eq!(before_items.len(), after_items.len());
    for old in before_items {
        let kept = after_items
            .iter()
            .find(|item| item["relay_id"] == old["relay_id"])
            .unwrap_or_else(|| panic!("waiter removed: {old}"));
        assert_eq!(kept["connected_at"], old["connected_at"]);
        assert_eq!(kept["role"], old["role"]);
        assert!(kept["waited_ms"].as_u64() >= old["waited_ms"].as_u64());
    }
    assert!(!closed_within(&mut first, SETTLE));
    assert!(!closed_within(&mut second, SETTLE));
    let mut overflow = waiting_server(ports, "5501");
    assert!(closed_within(&mut overflow, EXIT_LIMIT));
    let rejection = wait_for_line(&log, |line| event_is(line, "connection_rejected"));
    assert!(rejection.contains("\"limit\":1"), "{rejection}");
    assert_eq!(
        query(ports.management, &cookie, "/api/v1/waiting")["items"]
            .as_array()
            .map(Vec::len),
        Some(3)
    );

    // Immediate matches do not require a waiting slot, even while count > Lw.
    let mut first_match = tcp_viewer(ports.tcp_viewer);
    first_match
        .write_all(&request("5401"))
        .unwrap_or_else(|error| panic!("match: {error}"));
    exchange(&mut first_match, &mut first);
    let mut second_match = tcp_viewer(ports.tcp_viewer);
    second_match
        .write_all(&request("5402"))
        .unwrap_or_else(|error| panic!("match: {error}"));
    exchange(&mut second_match, &mut second);
    poll_query(ports.management, &cookie, "/api/v1/summary", |value| {
        value["counts"]["waiting"] == 1
    });
    // The third original waiter retains W=18, not the reloaded W=10.
    let wait_to = registered_by + Duration::from_secs(11);
    if let Some(remaining) = wait_to.checked_duration_since(Instant::now()) {
        std::thread::sleep(remaining);
    }
    assert!(
        !closed_within(&mut deadline_witness, Duration::from_millis(50)),
        "old W shortened"
    );
    assert!(
        closed_within(&mut deadline_witness, Duration::from_secs(9)),
        "old W extended/reset"
    );
    assert!(
        registered_by.elapsed() >= Duration::from_secs(17),
        "old W was replaced by new W"
    );
    assert!(
        registered_by.elapsed() < Duration::from_secs(20),
        "old deadline changed"
    );
    poll_query(ports.management, &cookie, "/api/v1/summary", |value| {
        value["counts"]["waiting"] == 0
    });
    let mut recovered = waiting_server(ports, "5601");
    wait_for_count(&log, 4, |line| event_is(line, "waiting_registered"));
    let new_registered_by = Instant::now();
    assert!(!closed_within(&mut recovered, SETTLE));
    let mut at_limit = waiting_server(ports, "5602");
    assert!(
        closed_within(&mut at_limit, EXIT_LIMIT),
        "count == Lw must refuse registration"
    );
    text = setting(text, "limits.waiting", "2");
    reload(&config, &text);
    let mut raised = waiting_server(ports, "5603");
    wait_for_count(&log, 5, |line| event_is(line, "waiting_registered"));
    let mut raised_match = tcp_viewer(ports.tcp_viewer);
    raised_match
        .write_all(&request("5603"))
        .unwrap_or_else(|error| panic!("raised match: {error}"));
    exchange(&mut raised_match, &mut raised);
    assert!(
        closed_within(&mut recovered, Duration::from_secs(11)),
        "new waiter did not use W=10"
    );
    assert!(
        new_registered_by.elapsed() >= Duration::from_secs(9),
        "new waiter expired too early"
    );
    assert!(
        new_registered_by.elapsed() < Duration::from_secs(12),
        "raising Lw reset the waiting deadline"
    );
    exchange(&mut first_match, &mut first);
    exchange(&mut second_match, &mut second);
}

fn session_for_viewer(port: u16, cookie: &str, source_port: u16) -> String {
    let sessions = poll_query(port, cookie, "/api/v1/sessions", |value| {
        value["items"].as_array().is_some_and(|items| {
            items
                .iter()
                .any(|item| item["viewer_address"]["port"] == source_port)
        })
    });
    sessions["items"]
        .as_array()
        .unwrap_or_else(|| panic!("sessions"))
        .iter()
        .find(|item| item["viewer_address"]["port"] == source_port)
        .and_then(|item| item["session_id"].as_str())
        .unwrap_or_else(|| panic!("session id: {sessions}"))
        .to_owned()
}

fn close_boundary_scenario(name: &str, edit: bool) {
    let _lock = lock_ports();
    let ports = Ports::free();
    let mut text = setting(
        config_text(ports, VIEWER_ONLY, CERT, KEY),
        "timeouts.close_secs",
        "8",
    );
    text = setting(text, "management.config_edit", "true");
    text = setting(text, "dashboard.poll_secs", "3");
    text = setting(text, "dashboard.query_timeout_secs", "4");
    let config = authenticated_setup(name, &text);
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);
    let (cookie, csrf, initial) = login(ports.management);
    assert_eq!(initial["dashboard"]["terminate_timeout_secs"], 13);
    let (mut old_viewer, old_server) = pair(ports, "5701");
    let (mut preexisting_viewer, preexisting_server) = pair(ports, "5702");
    let old_id = session_for_viewer(
        ports.management,
        &cookie,
        old_viewer
            .local_addr()
            .unwrap_or_else(|error| panic!("address: {error}"))
            .port(),
    );
    let preexisting_id = session_for_viewer(
        ports.management,
        &cookie,
        preexisting_viewer
            .local_addr()
            .unwrap_or_else(|error| panic!("address: {error}"))
            .port(),
    );
    let old_path = format!("/api/v1/sessions/{old_id}");
    let preexisting_path = format!("/api/v1/sessions/{preexisting_id}");
    let old_started = Instant::now();
    old_server
        .shutdown(std::net::Shutdown::Write)
        .unwrap_or_else(|error| panic!("old EOF: {error}"));
    poll_query(ports.management, &cookie, &old_path, |value| {
        value["session"]["state"] == "closing"
    });
    if edit {
        let view = query(ports.management, &cookie, "/api/v1/config");
        let reply = http(ports.management, "PATCH", "/api/v1/config",
            &[("Cookie", &cookie), ("X-CSRF-Token", &csrf), ("Content-Type", "application/json")],
            &serde_json::json!({"revision": view["revision"], "changes": {"timeouts.close_secs": 1}}).to_string());
        assert_eq!(reply.status, 200, "{}", reply.body);
        assert_eq!(
            reply.body["applied"],
            serde_json::json!(["timeouts.close_secs"])
        );
    } else {
        text = setting(text, "timeouts.close_secs", "1");
        reload(&config, &text);
    }
    let current = query(ports.management, &cookie, "/api/v1/session");
    assert_eq!(current["dashboard"]["terminate_timeout_secs"], 6);
    assert_eq!(
        current["dashboard"]["poll_secs"],
        initial["dashboard"]["poll_secs"]
    );
    assert_eq!(
        current["dashboard"]["query_timeout_secs"],
        initial["dashboard"]["query_timeout_secs"]
    );
    let (_, _, new_login) = login(ports.management);
    assert_eq!(new_login["dashboard"]["terminate_timeout_secs"], 6);
    assert_eq!(new_login["dashboard"]["poll_secs"], 3);
    assert_eq!(new_login["dashboard"]["query_timeout_secs"], 4);

    // A session established under old E but first closed after reload uses new E.
    let new_started = Instant::now();
    preexisting_server
        .shutdown(std::net::Shutdown::Write)
        .unwrap_or_else(|error| panic!("new EOF: {error}"));
    poll_query(ports.management, &cookie, &preexisting_path, |value| {
        value["session"]["state"] == "closing"
    });
    poll_query(
        ports.management,
        &cookie,
        "/api/v1/closed-sessions",
        |value| {
            value["items"].as_array().is_some_and(|items| {
                items.iter().any(|item| {
                    item["session_id"] == preexisting_id && item["close_cause"] == "close_timeout"
                })
            })
        },
    );
    assert!(
        new_started.elapsed() >= Duration::from_millis(850),
        "new E expired early"
    );
    assert!(
        new_started.elapsed() < Duration::from_secs(4),
        "new close retained old E"
    );
    assert_eq!(
        query(ports.management, &cookie, &old_path)["session"]["state"],
        "closing",
        "started close was shortened to new E"
    );
    poll_query(
        ports.management,
        &cookie,
        "/api/v1/closed-sessions",
        |value| {
            value["items"].as_array().is_some_and(|items| {
                items.iter().any(|item| {
                    item["session_id"] == old_id && item["close_cause"] == "close_timeout"
                })
            })
        },
    );
    assert!(
        old_started.elapsed() >= Duration::from_secs(7),
        "old E changed"
    );
    assert!(
        old_started.elapsed() < Duration::from_secs(11),
        "old E restarted or extended"
    );
    assert!(closed_within(&mut old_viewer, SETTLE));
    assert!(closed_within(&mut preexisting_viewer, SETTLE));
}

/// spec: management.md:MGT-041
#[test]
fn edited_close_timeout_updates_session_hint_but_not_started_close() {
    close_boundary_scenario("edited-close", true);
}

/// spec: operations.md:OPS-057
#[test]
fn reloaded_close_timeout_changes_new_closes_and_response_hints_only() {
    close_boundary_scenario("watched-close", false);
}

/// spec: operations.md:OPS-050 partial
#[test]
fn runtime_groups_take_effect_only_at_their_documented_boundaries() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let mut text = config_text(ports, VIEWER_ONLY, CERT, KEY);
    text = setting(text, "timeouts.handshake_secs", "12");
    text = setting(text, "timeouts.tls_upgrade_secs", "12");
    text = setting(text, "limits.ws_message_bytes", "131072");
    text = setting(text, "limits.buffer_bytes", "32768");
    let config = setup("runtime-boundaries", &text);
    let _log = log_of(&config);
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);

    let mut old_initial = tcp_viewer(ports.tcp_viewer);
    let mut old_upgrade = connect_from([127, 0, 0, 1], ports.ws);
    let mut old_management = connect_from([127, 0, 0, 1], ports.management);
    let mut old_wss_upgrade = connect_from([127, 0, 0, 1], ports.wss);
    let held_for_upgrade = connect_from([127, 0, 0, 1], ports.ws);
    let held_for_message = connect_from([127, 0, 0, 1], ports.ws);
    let mut old_ws = WebSocket::upgrade(Box::new(connect_from([127, 0, 0, 1], ports.ws)));
    assert_eq!(old_ws.read_data(12), GREETING);
    old_ws.send_binary(&request("5001"));
    let mut old_server = waiting_server(ports, "5001");
    ws_exchange(&mut old_ws, &mut old_server);
    // Old raw transports remain accepted while the candidate is published.
    // Runtime unit tests separately pin the accept/upgrade generation race.
    std::thread::sleep(Duration::from_millis(100));
    for (key, value) in [
        ("timeouts.handshake_secs", "1"),
        ("timeouts.tls_upgrade_secs", "1"),
        ("limits.ws_message_bytes", "65536"),
        ("limits.buffer_bytes", "16384"),
        ("keepalive.ws_ping_secs", "5"),
        ("keepalive.ws_pong_timeout_secs", "5"),
    ] {
        text = setting(text, key, value);
    }
    let applied = reload(&config, &text);
    assert_eq!(applied["applied"].as_array().map(Vec::len), Some(6));
    let mut new_ws = WebSocket::upgrade(Box::new(held_for_message));
    assert_eq!(new_ws.read_data(12), GREETING);
    new_ws.send_binary(&request("5002"));
    let mut new_server = waiting_server(ports, "5002");
    ws_exchange(&mut new_ws, &mut new_server);

    // H starts at initial exchange, not at transport accept.
    let mut new_initial = tcp_viewer(ports.tcp_viewer);
    let initial_at = Instant::now();
    assert!(closed_within(&mut new_initial, Duration::from_secs(3)));
    assert!(initial_at.elapsed() >= Duration::from_millis(850));
    let mut post_upgrade = WebSocket::upgrade(Box::new(held_for_upgrade));
    assert_eq!(post_upgrade.read_data(12), GREETING);
    let (_, opcode, payload) = post_upgrade.frame();
    assert_eq!(opcode, 8);
    assert_eq!(&payload[..2], &4009_u16.to_be_bytes());
    assert_eq!(&payload[2..], b"handshake_timeout");

    // New U applies independently to WS, WSS TLS, and management first-head
    // deadlines. Hold old sockets throughout the new-deadline observation.
    let mut new_upgrade = connect_from([127, 0, 0, 1], ports.ws);
    let mut new_wss_upgrade = connect_from([127, 0, 0, 1], ports.wss);
    let mut new_management = connect_from([127, 0, 0, 1], ports.management);
    assert!(closed_within(&mut new_upgrade, Duration::from_secs(3)));
    assert!(closed_within(&mut new_wss_upgrade, Duration::from_secs(2)));
    assert!(closed_within(&mut new_management, Duration::from_secs(2)));
    for stream in [
        &mut old_initial,
        &mut old_upgrade,
        &mut old_management,
        &mut old_wss_upgrade,
    ] {
        assert!(
            !closed_within(stream, Duration::from_millis(50)),
            "started H/U was shortened"
        );
    }

    // Upgraded-before retains M=128 KiB. New upgrades, even on previously
    // accepted transports, enforce M=64 KiB. The payload also exceeds B.
    let large = vec![0x5a; 70_000];
    let reader = std::thread::spawn(move || {
        let mut received = vec![0; 70_000];
        old_server
            .read_exact(&mut received)
            .unwrap_or_else(|error| panic!("old M stream: {error}"));
        received
    });
    old_ws.send_binary(&large);
    assert_eq!(
        reader
            .join()
            .unwrap_or_else(|error| panic!("old M reader: {error:?}")),
        large
    );
    // M is checked from the declared length, before a rejected payload is
    // delivered. Do not race a large client write against the close response.
    let mut header = vec![0x82, 0xff];
    header.extend_from_slice(&70_000_u64.to_be_bytes());
    header.extend_from_slice(&[0x13, 0x57, 0x9b, 0xdf]);
    new_ws
        .wire
        .write_all(&header)
        .unwrap_or_else(|error| panic!("new M header: {error}"));
    new_ws
        .wire
        .flush()
        .unwrap_or_else(|error| panic!("new M flush: {error}"));
    let (_, opcode, payload) = new_ws.frame();
    assert_eq!(opcode, 8);
    assert_eq!(
        &payload[..2],
        &1009_u16.to_be_bytes(),
        "new M must reject oversized message"
    );
    // Deliberately partial: TCP K readback, Kw scheduling, C attempt timing,
    // and started G have adapter/app/platform-specific coverage.
}

/// spec: operations.md:OPS-067
#[test]
fn trusted_proxies_reload_and_edit_apply_to_new_checks_and_keep_open_connections() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let mut text = config_text(ports, VIEWER_ONLY, CERT, KEY);
    text = setting(text, "management.config_edit", "true");
    text = setting(text, "relay.ws.trusted_proxies", r#"["0.0.0.0/0"]"#);
    text = setting(text, "relay.wss.trusted_proxies", r#"["::/0"]"#);
    text = setting(
        text,
        "relay.ws.viewer.allow",
        r#"["198.51.100.7", "2001:db8::7"]"#,
    );
    text = setting(text, "relay.ws.server.enabled", "true");
    text = setting(text, "relay.ws.server.allow", r#"["127.0.0.1"]"#);
    text = setting(text, "relay.tcp_server.allow", r#"["127.0.0.1"]"#);
    let config = authenticated_setup("ops067-trusted-proxies", &text);
    let log = log_of(&config);
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);

    // Startup phase warning for covering whole family
    wait_for_count(&log, 2, |line| {
        event_is(line, "trust_all_proxies") && field(line, "phase") == Some("startup")
    });

    // Pre-existing session established under initial proxy trust
    let socket = connect_from([127, 0, 0, 1], ports.ws);
    let (status, ws) = ws_upgrade_with_headers(
        Box::new(socket),
        "/relay/viewer",
        &[("X-Forwarded-For", "2001:db8::7")],
    );
    assert_eq!(status, 101);
    let mut preexisting_ws = ws.unwrap();
    assert_eq!(preexisting_ws.read_data(12), GREETING);
    preexisting_ws.send_binary(&request("6701"));
    let mut preexisting_server = waiting_server(ports, "6701");
    ws_exchange(&mut preexisting_ws, &mut preexisting_server);

    // File watch reload replacing trusted list
    text = setting(text, "relay.ws.trusted_proxies", r#"["127.0.0.1"]"#);
    text = setting(text, "relay.wss.trusted_proxies", r#"["127.0.0.1"]"#);
    let applied = reload(&config, &text);
    let applied_keys = applied["applied"].as_array().unwrap();
    assert!(applied_keys.iter().any(|k| k == "relay.ws.trusted_proxies"));
    assert!(
        applied_keys
            .iter()
            .any(|k| k == "relay.wss.trusted_proxies")
    );

    // Existing session survives reload
    ws_exchange(&mut preexisting_ws, &mut preexisting_server);

    // File watch reload with whole family warning at reload phase
    text = setting(text, "relay.ws.trusted_proxies", r#"["0.0.0.0/0"]"#);
    reload(&config, &text);
    wait_for_line(&log, |line| {
        event_is(line, "trust_all_proxies")
            && field(line, "phase") == Some("reload")
            && field(line, "listener") == Some("ws")
    });

    // Management API dynamic PATCH edit replacing trusted list
    let (cookie, csrf, _) = login(ports.management);
    let view = query(ports.management, &cookie, "/api/v1/config");
    let revision = view["revision"].as_str().unwrap();
    let patch_body = serde_json::json!({
        "revision": revision,
        "changes": {
            "relay.ws.trusted_proxies": ["127.0.0.1"]
        }
    })
    .to_string();
    let reply = http(
        ports.management,
        "PATCH",
        "/api/v1/config",
        &[
            ("Cookie", &cookie),
            ("X-CSRF-Token", &csrf),
            ("Content-Type", "application/json"),
        ],
        &patch_body,
    );
    assert_eq!(reply.status, 200, "{}", reply.body);
    assert!(
        reply.body["applied"]
            .as_array()
            .unwrap()
            .iter()
            .any(|k| k == "relay.ws.trusted_proxies")
    );

    // Existing connection continues to survive
    ws_exchange(&mut preexisting_ws, &mut preexisting_server);

    // Under trusted list ["127.0.0.1"], a new connection through proxy succeeds
    let socket = connect_from([127, 0, 0, 1], ports.ws);
    let (status, new_ws) = ws_upgrade_with_headers(
        Box::new(socket),
        "/relay/viewer",
        &[("X-Forwarded-For", "198.51.100.7")],
    );
    assert_eq!(status, 101);
    drop(new_ws);

    // PATCH trusted list to empty: 127.0.0.1 is no longer trusted
    let view2 = query(ports.management, &cookie, "/api/v1/config");
    let revision2 = view2["revision"].as_str().unwrap();
    let patch_body2 = serde_json::json!({
        "revision": revision2,
        "changes": {
            "relay.ws.trusted_proxies": []
        }
    })
    .to_string();
    let reply2 = http(
        ports.management,
        "PATCH",
        "/api/v1/config",
        &[
            ("Cookie", &cookie),
            ("X-CSRF-Token", &csrf),
            ("Content-Type", "application/json"),
        ],
        &patch_body2,
    );
    assert_eq!(reply2.status, 200, "{}", reply2.body);

    // Existing connection still survives
    ws_exchange(&mut preexisting_ws, &mut preexisting_server);

    // Now from 127.0.0.1, X-Forwarded-For is ignored because 127.0.0.1 is not in trusted_proxies.
    // Socket IP 127.0.0.1 is not in viewer.allow ("198.51.100.7"), so 403 Forbidden.
    let socket = connect_from([127, 0, 0, 1], ports.ws);
    let (status, refused_ws) = ws_upgrade_with_headers(
        Box::new(socket),
        "/relay/viewer",
        &[("X-Forwarded-For", "198.51.100.7")],
    );
    assert_eq!(status, 403);
    assert!(refused_ws.is_none());

    // PATCH with whole-family trusted proxy emits trust_all_proxies reload warning
    let view3 = query(ports.management, &cookie, "/api/v1/config");
    let revision3 = view3["revision"].as_str().unwrap();
    let patch_body3 = serde_json::json!({
        "revision": revision3,
        "changes": {
            "relay.ws.trusted_proxies": ["0.0.0.0/0"]
        }
    })
    .to_string();
    let reply3 = http(
        ports.management,
        "PATCH",
        "/api/v1/config",
        &[
            ("Cookie", &cookie),
            ("X-CSRF-Token", &csrf),
            ("Content-Type", "application/json"),
        ],
        &patch_body3,
    );
    assert_eq!(reply3.status, 200, "{}", reply3.body);
    wait_for_line(&log, |line| {
        event_is(line, "trust_all_proxies")
            && field(line, "phase") == Some("reload")
            && field(line, "item") == Some("0.0.0.0/0")
    });

    // Pre-existing session survives throughout
    ws_exchange(&mut preexisting_ws, &mut preexisting_server);
    drop(preexisting_ws);
    drop(preexisting_server);
}

fn ticket_jwks(fixture: &str, kid: &str) -> String {
    let pem =
        std::fs::read(tls_file(fixture)).unwrap_or_else(|error| panic!("read {fixture}: {error}"));
    let private = rustls::pki_types::PrivateKeyDer::from_pem_slice(&pem)
        .unwrap_or_else(|error| panic!("parse {fixture}: {error}"));
    let signer = rustls::crypto::aws_lc_rs::sign::any_supported_type(&private)
        .unwrap_or_else(|error| panic!("signing key {fixture}: {error}"));
    let public = signer
        .public_key()
        .unwrap_or_else(|| panic!("public key {fixture}"));
    let point = &public.as_ref()[public.as_ref().len() - 65..];
    let encode = |bytes: &[u8]| {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut encoded = String::new();
        for chunk in bytes.chunks(3) {
            let mut block = [0u8; 3];
            block[..chunk.len()].copy_from_slice(chunk);
            let value = u32::from_be_bytes([0, block[0], block[1], block[2]]);
            for index in 0..=chunk.len() {
                encoded.push(char::from(
                    ALPHABET[((value >> (18 - index * 6)) & 63) as usize],
                ));
            }
        }
        encoded
    };
    serde_json::json!({"keys": [{
        "kty": "EC", "crv": "P-256", "kid": kid,
        "x": encode(&point[1..33]), "y": encode(&point[33..65])
    }]})
    .to_string()
}

fn ticket_config(role: &str, enabled: bool, file: &str) -> String {
    format!(
        "\n[relay.ticket.{role}]\nenabled = {enabled}\nissuer = \"issuer.example\"\nrelay_host = \"relay.example\"\npublic_keys_file = \"{file}\"\nmax_lifetime_secs = 60\n"
    )
}

/// spec: operations.md:OPS-070 partial
/// spec: operations.md:OPS-072 partial
#[test]
fn ticket_key_file_watcher_applies_rotation_and_rejects_every_invalid_candidate() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let text =
        config_text(ports, VIEWER_ONLY, CERT, KEY) + &ticket_config("viewer", true, "viewer.jwks");
    let config = setup("ticket-key-rotation", &text);
    let key_file = config.with_file_name("viewer.jwks");
    let old = ticket_jwks(KEY, "old");
    let new = ticket_jwks("other-key-p256.pem", "new");
    write(&key_file, &old);
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);
    let log = log_of(&config);
    // A connection that predates every rotation must survive every rejection.
    let (mut held, mut held_server) = pair(ports, "7201");
    exchange(&mut held, &mut held_server);
    let valid: serde_json::Value = serde_json::from_str(&new).unwrap();
    let key = valid["keys"][0].clone();
    let mut non_p256 = key.clone();
    non_p256["crv"] = "P-384".into();
    let mut private = key.clone();
    private["d"] = "PRIVATE_KEY_CONTENT_MUST_NOT_APPEAR".into();
    let invalid = [
        None,
        Some("NOT_JSON_TICKET_KEY_CONTENT".to_owned()),
        Some(r#"{"keys":[]}"#.to_owned()),
        Some(serde_json::json!({"keys": [non_p256]}).to_string()),
        Some(serde_json::json!({"keys": [private]}).to_string()),
        Some(serde_json::json!({"keys": [key.clone(), key]}).to_string()),
        Some("X".repeat(65_537)),
    ];
    for (index, candidate) in invalid.into_iter().enumerate() {
        match candidate {
            Some(bytes) => write(&key_file, &bytes),
            None => std::fs::remove_file(&key_file).unwrap(),
        }
        wait_for_count(&log, index + 1, |line| {
            reason_is(line, "reload_rejected", "ticket_keys_invalid")
        });
        assert_eq!(logged(&log, |line| event_is(line, "reload_applied")), 0);
    }
    write(&key_file, &new);
    let applied = wait_for_line(&log, |line| event_is(line, "reload_applied"));
    let applied: serde_json::Value = serde_json::from_str(&applied).unwrap();
    assert_eq!(
        applied["applied"],
        serde_json::json!(["relay.ticket.viewer.public_keys_file"])
    );
    assert_eq!(applied["trigger"], "file_watch");
    // Rewriting equal keys with a different JSON layout is not an applied change.
    write(&key_file, &serde_json::to_string_pretty(&valid).unwrap());
    wait_for_count(&log, 9, |line| event_is(line, "reload_change_detected"));
    write(
        &config,
        &(text.clone() + &ticket_config("server", true, "viewer.jwks")),
    );
    wait_for_count(&log, 8, |line| {
        reason_is(line, "reload_rejected", "ticket_keys_invalid")
    });
    let lines = std::fs::read_to_string(&log).unwrap();
    assert!(lines.lines().any(|line| {
        reason_is(line, "reload_rejected", "ticket_keys_invalid")
            && field(line, "config_key") == Some("relay.ticket.server.public_keys_file")
    }));
    assert!(!lines.contains("PRIVATE_KEY_CONTENT_MUST_NOT_APPEAR"));
    assert!(!lines.contains("NOT_JSON_TICKET_KEY_CONTENT"));
    assert!(!lines.contains(&old));
    assert!(!lines.contains(&new));
    // Disabling ticket is another dynamic change, never a restart requirement.
    write(
        &config,
        &text.replace(
            "[relay.ticket.viewer]\nenabled = true",
            "[relay.ticket.viewer]\nenabled = false",
        ),
    );
    wait_for_count(&log, 2, |line| event_is(line, "reload_applied"));
    exchange(&mut held, &mut held_server);
    stop(guard.child());
}

/// spec: operations.md:OPS-071
#[test]
fn ticket_tcp_and_plaintext_warnings_cover_both_roles_at_start_and_reload() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let base = config_text(ports, VIEWER_ONLY, CERT, KEY)
        + "\n[relay.ws.server]\nenabled = true\nallow = [\"127.0.0.1\"]\n";
    let enabled = base.clone()
        + &ticket_config("viewer", true, "viewer.jwks")
        + &ticket_config("server", true, "server.jwks");
    let config = setup("ticket-warnings", &enabled);
    write(
        &config.with_file_name("viewer.jwks"),
        &ticket_jwks(KEY, "viewer"),
    );
    write(
        &config.with_file_name("server.jwks"),
        &ticket_jwks("other-key-p256.pem", "server"),
    );
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);
    let log = log_of(&config);
    for (role, listener) in [("viewer", "tcp_viewer"), ("server", "tcp_server")] {
        for (event, listener) in [
            ("ticket_tcp_bypass", listener),
            ("ticket_plaintext_ws", "ws"),
        ] {
            wait_for_line(&log, |line| {
                event_is(line, event)
                    && field(line, "role") == Some(role)
                    && field(line, "listener") == Some(listener)
                    && field(line, "phase") == Some("startup")
            });
        }
    }
    let disabled = base
        + &ticket_config("viewer", false, "viewer.jwks")
        + &ticket_config("server", false, "server.jwks");
    write(&config, &disabled);
    wait_for_count(&log, 1, |line| event_is(line, "reload_applied"));
    write(&config, &enabled);
    wait_for_count(&log, 2, |line| event_is(line, "reload_applied"));
    for (role, listener) in [("viewer", "tcp_viewer"), ("server", "tcp_server")] {
        for (event, listener) in [
            ("ticket_tcp_bypass", listener),
            ("ticket_plaintext_ws", "ws"),
        ] {
            wait_for_line(&log, |line| {
                event_is(line, event)
                    && field(line, "role") == Some(role)
                    && field(line, "listener") == Some(listener)
                    && field(line, "phase") == Some("reload")
            });
        }
    }
    assert_eq!(logged(&log, |line| event_is(line, "ticket_tcp_bypass")), 4);
    assert_eq!(
        logged(&log, |line| event_is(line, "ticket_plaintext_ws")),
        4
    );
    assert_eq!(
        logged(&log, |line| event_is(line, "reload_restart_required")),
        0
    );
    stop(guard.child());
}

fn ticket_base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut encoded = String::new();
    for chunk in bytes.chunks(3) {
        let mut block = [0u8; 3];
        block[..chunk.len()].copy_from_slice(chunk);
        let value = u32::from_be_bytes([0, block[0], block[1], block[2]]);
        for index in 0..=chunk.len() {
            encoded.push(char::from(
                ALPHABET[((value >> (18 - index * 6)) & 63) as usize],
            ));
        }
    }
    encoded
}

fn ticket_wall_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_else(|error| panic!("wall clock: {error}"))
        .as_secs()
}

/// Signs with the existing test TLS fixture; rustls returns DER, JWS uses R || S.
fn signed_viewer_ticket(fixture: &str, kid: &str, jti: &str, issuer: &str, host: &str) -> String {
    let pem =
        std::fs::read(tls_file(fixture)).unwrap_or_else(|error| panic!("read {fixture}: {error}"));
    let private = rustls::pki_types::PrivateKeyDer::from_pem_slice(&pem)
        .unwrap_or_else(|error| panic!("parse {fixture}: {error}"));
    let key = rustls::crypto::aws_lc_rs::sign::any_supported_type(&private)
        .unwrap_or_else(|error| panic!("signing key {fixture}: {error}"));
    let signer = key
        .choose_scheme(&[rustls::SignatureScheme::ECDSA_NISTP256_SHA256])
        .unwrap_or_else(|| panic!("P-256 scheme for {fixture}"));
    let issued = ticket_wall_secs() + 5;
    let header = serde_json::json!({"alg": "ES256", "kid": kid}).to_string();
    let claims = serde_json::json!({
        "iss": issuer, "aud": "vnc-relay/viewer", "rh": host, "rid": "7202",
        "jti": jti, "iat": issued, "exp": issued + 10
    })
    .to_string();
    let input = format!(
        "{}.{}",
        ticket_base64(header.as_bytes()),
        ticket_base64(claims.as_bytes())
    );
    let der = signer
        .sign(input.as_bytes())
        .unwrap_or_else(|error| panic!("sign: {error}"));
    assert_eq!(der[0], 0x30);
    let mut raw = [0u8; 64];
    let mut offset = 2;
    for half in raw.chunks_mut(32) {
        assert_eq!(der[offset], 0x02);
        let len = usize::from(der[offset + 1]);
        offset += 2;
        let integer = &der[offset..offset + len];
        let integer = integer.strip_prefix(&[0]).unwrap_or(integer);
        assert!(integer.len() <= 32);
        half[32 - integer.len()..].copy_from_slice(integer);
        offset += len;
    }
    format!("{input}.{}", ticket_base64(&raw))
}

fn assert_viewer_key_works(ports: Ports, fixture: &str, kid: &str, jti: &str) {
    let ticket = signed_viewer_ticket(fixture, kid, jti, "issuer.example", "relay.example");
    let protocol = format!("binary, ticket.{ticket}");
    let (status, viewer) = ws_upgrade_with_headers(
        Box::new(connect_from([127, 0, 0, 1], ports.ws)),
        "/relay/viewer",
        &[("Sec-WebSocket-Protocol", &protocol)],
    );
    assert_eq!(status, 101);
    drop(viewer);
}

fn ticket_startup_output(config: &Path) -> std::process::Output {
    let mut guard = Guard(Some(spawn(config)));
    let deadline = Instant::now() + EXIT_LIMIT;
    loop {
        if guard
            .child()
            .try_wait()
            .unwrap_or_else(|error| panic!("try_wait: {error}"))
            .is_some()
        {
            let child = guard
                .0
                .take()
                .unwrap_or_else(|| panic!("child already taken"));
            return child
                .wait_with_output()
                .unwrap_or_else(|error| panic!("output: {error}"));
        }
        assert!(Instant::now() < deadline, "startup did not fail");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// spec: operations.md:OPS-070
#[test]
fn full_ticket_validation_at_startup_and_reload_preserves_the_last_valid_generation() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let base = config_text(ports, VIEWER_ONLY, CERT, KEY);
    let enabled = ticket_config("viewer", true, "viewer.jwks");
    let old = ticket_jwks(KEY, "old");
    let key: serde_json::Value =
        serde_json::from_str::<serde_json::Value>(&old).unwrap()["keys"][0].clone();
    let mut wrong_curve = key.clone();
    wrong_curve["crv"] = "P-384".into();
    let mut private = key.clone();
    private["d"] = "PRIVATE_TICKET_KEY_MUST_NOT_BE_LOGGED".into();
    let mut cases: Vec<(String, Option<String>, &str, &str)> = Vec::new();
    for (assignment, leaf) in [
        ("issuer = \"issuer.example\"\n", "issuer"),
        ("relay_host = \"relay.example\"\n", "relay_host"),
        ("public_keys_file = \"viewer.jwks\"\n", "public_keys_file"),
    ] {
        cases.push((
            enabled.replace(assignment, ""),
            Some(old.clone()),
            "config_missing_key",
            leaf,
        ));
    }
    for value in [9, 601] {
        cases.push((
            enabled.replace(
                "max_lifetime_secs = 60",
                &format!("max_lifetime_secs = {value}"),
            ),
            Some(old.clone()),
            "config_value_out_of_range",
            "max_lifetime_secs",
        ));
    }
    cases.push((
        enabled.replace("issuer.example", ""),
        Some(old.clone()),
        "config_value_out_of_range",
        "issuer",
    ));
    for contents in [
        None,
        Some("INVALID_JWKS_MUST_NOT_BE_LOGGED".to_owned()),
        Some(r#"{"keys":[]}"#.to_owned()),
        Some(serde_json::json!({"keys": [wrong_curve]}).to_string()),
        Some(serde_json::json!({"keys": [private]}).to_string()),
        Some(serde_json::json!({"keys": [key.clone(), key]}).to_string()),
        Some("X".repeat(65_537)),
    ] {
        cases.push((
            enabled.clone(),
            contents,
            "ticket_keys_invalid",
            "public_keys_file",
        ));
    }
    cases.push((
        enabled.clone() + &ticket_config("server", true, "viewer.jwks"),
        Some(old.clone()),
        "ticket_keys_invalid",
        "server.public_keys_file",
    ));
    for (index, (suffix, contents, code, leaf)) in cases.iter().enumerate() {
        let config = setup(
            &format!("full-ticket-startup-{index}"),
            &(base.clone() + suffix),
        );
        if let Some(contents) = contents {
            write(&config.with_file_name("viewer.jwks"), contents);
        }
        // A held first-bind port distinguishes pre-bind validation from bind failure.
        let held = TcpListener::bind(("127.0.0.1", ports.status)).unwrap();
        let output = ticket_startup_output(&config);
        drop(held);
        assert_eq!(output.status.code(), Some(3));
        let stderr = String::from_utf8(output.stderr).unwrap();
        let config_key = if leaf.starts_with("server.") {
            format!("relay.ticket.{leaf}")
        } else {
            format!("relay.ticket.viewer.{leaf}")
        };
        assert!(
            stderr.contains(&format!("error[{code}] key={config_key}:")),
            "{stderr}"
        );
        let log = String::from_utf8(output.stdout).unwrap()
            + &std::fs::read_to_string(log_of(&config)).unwrap_or_default();
        assert_eq!(
            log.lines()
                .filter(|line| {
                    reason_is(line, "startup_failed", code)
                        && field(line, "config_key") == Some(config_key.as_str())
                })
                .count(),
            1
        );
        assert!(!log.contains("listener_ready"));
        for text in [&stderr, &log] {
            assert!(!text.contains("PRIVATE_TICKET_KEY_MUST_NOT_BE_LOGGED"));
            assert!(!text.contains("INVALID_JWKS_MUST_NOT_BE_LOGGED"));
            assert!(!text.contains(&old));
        }
        for port in ports.all() {
            assert!(TcpListener::bind(("127.0.0.1", port)).is_ok());
        }
    }
    // Both lifetime boundaries start normally; the disabled-role shared-key boundary does too.
    for lifetime in [10, 600] {
        let text = base.clone()
            + &enabled.replace(
                "max_lifetime_secs = 60",
                &format!("max_lifetime_secs = {lifetime}"),
            )
            + &ticket_config("server", false, "viewer.jwks");
        let config = setup(&format!("full-ticket-valid-{lifetime}"), &text);
        write(&config.with_file_name("viewer.jwks"), &old);
        let mut guard = Guard(Some(spawn(&config)));
        wait_listening(guard.child(), ports);
        stop(guard.child());
    }
    let config = authenticated_setup("full-ticket-reload-validation", &(base.clone() + &enabled));
    write(&config.with_file_name("viewer.jwks"), &old);
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);
    let ready = ticket_wall_secs();
    wait_until(guard.child(), || ticket_wall_secs() > ready);
    let (cookie, _, _) = login(ports.management);
    let (mut viewer, mut server) = pair(ports, "7202");
    exchange(&mut viewer, &mut server);
    let log = log_of(&config);
    for (index, (suffix, contents, code, _)) in cases.iter().enumerate() {
        // Preparing these unreferenced files before changing the config avoids intermediate candidates.
        let path = config.with_file_name(format!("candidate-{index}.jwks"));
        if let Some(contents) = contents {
            write(&path, contents);
        }
        let suffix = suffix.replace("viewer.jwks", &format!("candidate-{index}.jwks"));
        write(&config, &(base.clone() + &suffix));
        wait_for_count(&log, index + 1, |line| event_is(line, "reload_rejected"));
        let latest = std::fs::read_to_string(&log)
            .unwrap()
            .lines()
            .rfind(|line| event_is(line, "reload_rejected"))
            .unwrap()
            .to_owned();
        assert_eq!(field(&latest, "reason"), Some(*code));
        exchange(&mut viewer, &mut server);
        assert_viewer_key_works(ports, KEY, "old", &format!("retained-{index}"));
        let view = query(ports.management, &cookie, "/api/v1/config");
        assert_eq!(
            effective(&view, "relay.ticket.viewer.issuer"),
            "issuer.example"
        );
        assert_eq!(
            effective(&view, "relay.ticket.viewer.max_lifetime_secs"),
            60
        );
        assert_eq!(effective(&view, "relay.ticket.server.enabled"), false);
        assert_eq!(logged(&log, |line| event_is(line, "reload_applied")), 0);
    }
    let valid = setting(
        base + &enabled,
        "relay.ticket.viewer.max_lifetime_secs",
        "10",
    );
    let applied = reload(&config, &valid);
    assert_eq!(
        applied["applied"],
        serde_json::json!(["relay.ticket.viewer.max_lifetime_secs"])
    );
    exchange(&mut viewer, &mut server);
    assert_viewer_key_works(ports, KEY, "old", "retained-final");
    stop(guard.child());
}

fn ticket_config_item<'a>(view: &'a serde_json::Value, key: &str) -> &'a serde_json::Value {
    view["items"]
        .as_array()
        .unwrap_or_else(|| panic!("config items: {view}"))
        .iter()
        .find(|item| item["key"] == key)
        .unwrap_or_else(|| panic!("config item {key}"))
}

/// spec: operations.md:OPS-072
#[test]
fn full_ticket_dynamic_reload_rotation_and_management_capabilities() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let mut text = config_text(ports, VIEWER_ONLY, CERT, KEY)
        + &ticket_config("viewer", true, "viewer.jwks")
        + &ticket_config("server", true, "server.jwks");
    for (key, value) in [
        ("management.config_edit", "true"),
        ("relay.ws.server.enabled", "true"),
        ("relay.ws.server.allow", "[\"127.0.0.1\"]"),
        ("relay.wss.server.enabled", "true"),
        ("relay.wss.server.allow", "[\"127.0.0.1\"]"),
        ("mode1.enabled", "true"),
        ("mode1.allow_targets", "[]"),
    ] {
        text = setting(text, key, value);
    }
    let config = authenticated_setup("full-ticket-dynamic", &text);
    let old = ticket_jwks(KEY, "old");
    let server_keys = ticket_jwks("other-key-p256.pem", "server");
    for path in ["viewer.jwks", "viewer-next.jwks"] {
        write(&config.with_file_name(path), &old);
    }
    for path in ["server.jwks", "server-next.jwks"] {
        write(&config.with_file_name(path), &server_keys);
    }
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);
    let ready = ticket_wall_secs();
    wait_until(guard.child(), || ticket_wall_secs() > ready);
    let (cookie, _, _) = login(ports.management);
    let view = query(ports.management, &cookie, "/api/v1/config");
    let items = view["items"].as_array().unwrap();
    assert_eq!(
        items
            .iter()
            .filter(|item| item["reload"] == "dynamic")
            .count(),
        43
    );
    assert_eq!(
        items.iter().filter(|item| item["editable"] == true).count(),
        37
    );
    let (mut viewer, mut server) = pair(ports, "7203");
    exchange(&mut viewer, &mut server);
    for role in ["viewer", "server"] {
        for (leaf, value) in [
            ("enabled", "false"),
            ("issuer", "\"other-issuer.example\""),
            ("relay_host", "\"other-relay.example\""),
            (
                "public_keys_file",
                if role == "viewer" {
                    "\"viewer-next.jwks\""
                } else {
                    "\"server-next.jwks\""
                },
            ),
            ("max_lifetime_secs", "10"),
        ] {
            let key = format!("relay.ticket.{role}.{leaf}");
            text = setting(text, &key, value);
            let applied = reload(&config, &text);
            assert_eq!(applied["applied"], serde_json::json!([key]));
            exchange(&mut viewer, &mut server);
        }
    }
    // Return to an enabled Viewer with claims matching the old key verification helper.
    for (leaf, value) in [
        ("issuer", "\"issuer.example\""),
        ("relay_host", "\"relay.example\""),
        ("enabled", "true"),
    ] {
        text = setting(text, &format!("relay.ticket.viewer.{leaf}"), value);
    }
    reload(&config, &text);
    assert_viewer_key_works(ports, KEY, "old", "before-rotation");
    let log = log_of(&config);
    let path = config.with_file_name("viewer-next.jwks");
    let new = ticket_jwks("other-key-p256.pem", "rotated");
    let before = logged(&log, |line| event_is(line, "reload_applied"));
    write(&path, &new);
    wait_for_count(&log, before + 1, |line| event_is(line, "reload_applied"));
    let line = std::fs::read_to_string(&log)
        .unwrap()
        .lines()
        .rfind(|line| event_is(line, "reload_applied"))
        .unwrap()
        .to_owned();
    let event: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(
        event["applied"],
        serde_json::json!(["relay.ticket.viewer.public_keys_file"])
    );
    assert_viewer_key_works(ports, "other-key-p256.pem", "rotated", "after-rotation");
    exchange(&mut viewer, &mut server);
    write(&path, "INVALID_ROTATION_MUST_NOT_BE_LOGGED");
    wait_for_count(&log, 1, |line| {
        reason_is(line, "reload_rejected", "ticket_keys_invalid")
    });
    assert_viewer_key_works(ports, "other-key-p256.pem", "rotated", "after-rejection");
    exchange(&mut viewer, &mut server);
    assert_eq!(
        logged(&log, |line| event_is(line, "reload_applied")),
        before + 1
    );
    assert!(
        !std::fs::read_to_string(&log)
            .unwrap()
            .contains("INVALID_ROTATION_MUST_NOT_BE_LOGGED")
    );
    stop(guard.child());
    // Listener switches require restart. Exercise each role's OR condition independently.
    for (index, (viewer_ws, viewer_wss, server_ws, server_wss)) in [
        (false, false, true, true),
        (true, true, false, false),
        (true, false, false, true),
        (false, true, true, false),
        (false, false, false, false),
    ]
    .into_iter()
    .enumerate()
    {
        let mut candidate = text.clone();
        for (key, value) in [
            ("relay.ws.viewer.enabled", viewer_ws),
            ("relay.wss.viewer.enabled", viewer_wss),
            ("relay.ws.server.enabled", server_ws),
            ("relay.wss.server.enabled", server_wss),
        ] {
            candidate = setting(candidate, key, if value { "true" } else { "false" });
        }
        let config = authenticated_setup(&format!("full-ticket-capabilities-{index}"), &candidate);
        write(&config.with_file_name("viewer-next.jwks"), &old);
        write(&config.with_file_name("server-next.jwks"), &server_keys);
        let mut guard = Guard(Some(spawn(&config)));
        wait_until(guard.child(), || {
            connects(ports.management) && connects(ports.status)
        });
        let (cookie, _, _) = login(ports.management);
        let view = query(ports.management, &cookie, "/api/v1/config");
        for (role, editable) in [
            ("viewer", viewer_ws || viewer_wss),
            ("server", server_ws || server_wss),
        ] {
            for leaf in [
                "enabled",
                "issuer",
                "relay_host",
                "max_lifetime_secs",
                "public_keys_file",
            ] {
                let item = ticket_config_item(&view, &format!("relay.ticket.{role}.{leaf}"));
                assert_eq!(item["reload"], "dynamic");
                assert_eq!(item["editable"], leaf != "public_keys_file" && editable);
            }
        }
        stop(guard.child());
    }
}
