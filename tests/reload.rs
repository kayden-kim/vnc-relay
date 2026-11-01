//! Partial automatic reload of a running `vnc-relay serve`, run against the
//! real binary with the file watcher on: the scenarios of
//! docs/specs/operations.md (OPS-003, OPS-024 to OPS-028, OPS-035, OPS-038,
//! OPS-040) and docs/specs/websocket.md (WS-022). Results are read from the
//! operation log that every test configures as `relay.log`.

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

/// D of the test configs (`settle_secs = 1` in `config_text`), in milliseconds.
const RELOAD_SETTLE_MS: u64 = 1_000;

const VIEWER_ONLY: &str = r#"["127.0.0.1"]"#;
const SECOND_ONLY: &str = r#"["127.0.0.2"]"#;
const RANGE: &str = r#"["127.0.0.2-127.0.0.3"]"#;
const ANY: &str = r#"["0.0.0.0/0"]"#;
const NONE: &str = "[]";

const CERT: &str = "server-chain.pem";
const KEY: &str = "server-key-p256.pem";

/// Config key of the WSS key file, named by rejected TLS candidates.
const KEY_KEY: &str = "relay.wss_viewer.key_file";

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
    ws_viewer: u16,
    wss_viewer: u16,
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
            ws_viewer: port(4),
            wss_viewer: port(5),
        }
    }

    fn all(self) -> [u16; 6] {
        [
            self.status,
            self.management,
            self.tcp_viewer,
            self.tcp_server,
            self.ws_viewer,
            self.wss_viewer,
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

[relay.ws_viewer]
enabled = true
listen = "127.0.0.1:{ws_viewer}"
allow = {allow}

[relay.wss_viewer]
enabled = true
listen = "127.0.0.1:{wss_viewer}"
allow = {allow}
cert_file = '{cert_path}'
key_file = '{key_path}'

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
        ws_viewer = ports.ws_viewer,
        wss_viewer = ports.wss_viewer,
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

/// Milliseconds from the `ts` of `from` to the `ts` of `to`, within one day.
fn millis_between(from: &str, to: &str) -> u64 {
    const DAY_MS: u64 = 86_400_000;
    let of_day = |line: &str| {
        let ts = field(line, "ts").unwrap_or_else(|| panic!("no ts in {line}"));
        // `YYYY-MM-DDTHH:MM:SS.mmmZ`, always UTC with milliseconds.
        let part = |range: std::ops::Range<usize>| -> u64 {
            ts[range]
                .parse()
                .unwrap_or_else(|error| panic!("ts {ts}: {error}"))
        };
        ((part(11..13) * 60 + part(14..16)) * 60 + part(17..19)) * 1000 + part(20..23)
    };
    (of_day(to) + DAY_MS - of_day(from)) % DAY_MS
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

/// A WebSocket upgrade to `/relay` with the RFC 6455 sample key.
fn upgrade_request() -> Vec<u8> {
    b"GET /relay HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"
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

/// spec: operations.md:OPS-025 partial
/// spec: operations.md:OPS-040 partial
#[test]
fn allow_change_keeps_waiting_connections_and_applies_to_new_decisions() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = setup("allow-change", &config_text(ports, VIEWER_ONLY, CERT, KEY));
    let log = log_of(&config);
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);

    let accepted = |line: &str| is(line, "connection_accepted", "tcp_viewer", "127.0.0.1");
    let before = logged(&log, accepted);
    let mut waiting = connect_from([127, 0, 0, 1], ports.tcp_viewer);
    wait_for_count(&log, before + 1, accepted);

    write(&config, &config_text(ports, RANGE, CERT, KEY));
    let applied = wait_for_line(&log, |line| event_is(line, "reload_applied"));
    assert!(applied.contains("relay.tcp_viewer.allow"), "{applied}");
    assert!(applied.contains("relay.ws_viewer.allow"), "{applied}");

    // The connection that the removed source holds stays open.
    assert!(
        !closed_within(&mut waiting, SETTLE),
        "waiting connection was closed"
    );
    // New decisions use the new list: 127.0.0.1 and 127.0.0.4 are refused,
    // 127.0.0.2 is accepted.
    assert!(closed_within(
        &mut connect_from([127, 0, 0, 1], ports.tcp_viewer),
        EXIT_LIMIT
    ));
    let mut second = connect_from([127, 0, 0, 2], ports.tcp_viewer);
    assert!(!closed_within(&mut second, SETTLE), "127.0.0.2 refused");
    assert!(closed_within(
        &mut connect_from([127, 0, 0, 4], ports.tcp_viewer),
        EXIT_LIMIT
    ));
    wait_for_line(&log, |line| {
        is(line, "connection_rejected", "tcp_viewer", "127.0.0.4")
    });
    // The WebSocket and WSS entry points follow the same list.
    assert!(closed_within(
        &mut connect_from([127, 0, 0, 1], ports.ws_viewer),
        EXIT_LIMIT
    ));
    assert!(closed_within(
        &mut connect_from([127, 0, 0, 1], ports.wss_viewer),
        EXIT_LIMIT
    ));
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
    let mut tls = tls_client(connect_from([127, 0, 0, 1], ports.wss_viewer));
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
    assert_eq!(wss_leaf(ports.wss_viewer), old, "old certificate replaced");
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
                ports.ws_viewer,
                ports.wss_viewer,
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
    let mut open = tls_client(connect_from([127, 0, 0, 1], ports.wss_viewer));
    assert_eq!(handshake(&mut open), old);

    write(
        &config,
        &config_text(
            ports,
            VIEWER_ONLY,
            "ecdsa-p384-chain.pem",
            "ecdsa-p384-key.pem",
        ),
    );
    let detected = wait_for_line(&log, |line| event_is(line, "reload_change_detected"));
    let applied = wait_for_line(&log, |line| event_is(line, "reload_applied"));
    assert!(applied.contains("relay.wss_viewer.cert_file"), "{applied}");
    // D is measured from the detection to the result.
    let elapsed = millis_between(&detected, &applied);
    assert!(
        elapsed <= RELOAD_SETTLE_MS,
        "result {elapsed} ms after the detection, D is {RELOAD_SETTLE_MS} ms: {applied}"
    );

    assert_eq!(
        wss_leaf(ports.wss_viewer),
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
    assert_eq!(
        wss_leaf(ports.wss_viewer),
        old,
        "rejected certificate applied"
    );
    assert!(!connects(moved));

    // Candidate 2: the port is reverted and only the allow list changes.
    write(&config, &config_text(ports, SECOND_ONLY, CERT, KEY));
    wait_for_count(&log, 1, |line| event_is(line, "reload_applied"));
    assert!(
        !closed_within(&mut connect_from([127, 0, 0, 2], ports.tcp_viewer), SETTLE),
        "127.0.0.2 refused after the IP change"
    );
    // The wss allow list is now 127.0.0.2 only, so the check connects from it.
    let leaf = handshake(&mut tls_client(connect_from(
        [127, 0, 0, 2],
        ports.wss_viewer,
    )));
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

    // One candidate changes the listen address, the accounts file, a limit, and the allow list.
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
    for key in [
        "relay.tcp_viewer.listen",
        "admin.accounts_file",
        "limits.admin_connections",
    ] {
        assert!(line.contains(key), "{key} missing from {line}");
    }
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

/// spec: websocket.md:WS-022 partial
#[test]
fn removed_source_is_refused_at_upgrade_on_connections_opened_before_the_removal() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = setup("ws-removal", &config_text(ports, SECOND_ONLY, CERT, KEY));
    let log = log_of(&config);
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);

    // A connection that completes an upgrade before the removal.
    let mut upgraded = connect_from([127, 0, 0, 2], ports.ws_viewer);
    upgraded
        .write_all(&upgrade_request())
        .unwrap_or_else(|error| panic!("request: {error}"));
    let head = read_head(&mut upgraded);
    assert_eq!(status_of(&head), 101, "{head}");

    // A plain HTTP connection and a TLS connection that send nothing yet.
    let mut held_http = connect_from([127, 0, 0, 2], ports.ws_viewer);
    let mut held_tls = tls_client(connect_from([127, 0, 0, 2], ports.wss_viewer));
    handshake(&mut held_tls);
    wait_for_count(&log, 2, |line| {
        is(line, "connection_accepted", "ws_viewer", "127.0.0.2")
    });
    wait_for_count(&log, 1, |line| {
        is(line, "connection_accepted", "wss_viewer", "127.0.0.2")
    });

    write(&config, &config_text(ports, VIEWER_ONLY, CERT, KEY));
    let applied = wait_for_line(&log, |line| event_is(line, "reload_applied"));
    assert!(applied.contains("relay.ws_viewer.allow"), "{applied}");
    assert!(applied.contains("relay.wss_viewer.allow"), "{applied}");

    // The upgraded connection stays open.
    assert!(
        !closed_within(&mut upgraded, SETTLE),
        "upgraded connection was closed"
    );

    // Upgrades on the held connections are refused at the upgrade re-check.
    held_http
        .write_all(&upgrade_request())
        .unwrap_or_else(|error| panic!("request: {error}"));
    let head = read_head(&mut held_http);
    assert_eq!(status_of(&head), 403, "{head}");
    wait_for_line(&log, |line| {
        is(line, "connection_rejected", "ws_viewer", "127.0.0.2")
            && field(line, "stage") == Some("upgrade")
            && field(line, "reason") == Some("ip_not_allowed")
    });

    held_tls
        .write_all(&upgrade_request())
        .unwrap_or_else(|error| panic!("request: {error}"));
    let head = read_head(&mut held_tls);
    assert_eq!(status_of(&head), 403, "{head}");
    wait_for_line(&log, |line| {
        is(line, "connection_rejected", "wss_viewer", "127.0.0.2")
            && field(line, "stage") == Some("upgrade")
            && field(line, "reason") == Some("ip_not_allowed")
    });

    // A new connection from the removed source is refused at accept.
    assert!(closed_within(
        &mut connect_from([127, 0, 0, 2], ports.ws_viewer),
        EXIT_LIMIT
    ));
    wait_for_line(&log, |line| {
        is(line, "connection_rejected", "ws_viewer", "127.0.0.2")
            && field(line, "stage") == Some("accept")
    });
}
