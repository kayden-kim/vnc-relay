//! Process-level target policy, failure, and shared socket-budget acceptance tests.
//! Linux DNS failure uses child-local glibc resolver options, not external DNS.

#[cfg(target_os = "linux")]
use std::collections::HashSet;
use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};

const BIN: &str = env!("CARGO_BIN_EXE_vnc-relay");
const LIMIT: Duration = Duration::from_secs(20);
static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

type Tls = StreamOwned<ClientConnection, TcpStream>;

fn field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let start = line.find(&format!("\"{name}\":\""))? + name.len() + 4;
    let rest = &line[start..];
    rest.find('"').map(|end| &rest[..end])
}

fn number(line: &str, name: &str) -> u64 {
    let prefix = format!("\"{name}\":");
    let start = line
        .find(&prefix)
        .unwrap_or_else(|| panic!("numeric log field"))
        + prefix.len();
    line[start..]
        .split(|ch: char| !ch.is_ascii_digit())
        .next()
        .unwrap_or_else(|| panic!("numeric value"))
        .parse()
        .unwrap_or_else(|error| panic!(concat!("unsigned numeric log value", ": {:?}"), error))
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("crates/adapters/tests/fixtures/tls")
        .join(name)
}

fn write_private(path: &Path, text: &str) {
    std::fs::write(path, text)
        .unwrap_or_else(|error| panic!(concat!("write test configuration", ": {:?}"), error));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .unwrap_or_else(|error| panic!(concat!("private test file", ": {:?}"), error));
    }
}

struct Relay {
    child: Child,
    dir: PathBuf,
    viewer: SocketAddr,
    management: SocketAddr,
    status: SocketAddr,
}

impl Relay {
    fn start(targets: &[String], limit: usize, connect_secs: u64, dns_disabled: bool) -> Self {
        // Reserve every chosen port simultaneously until immediately before spawn.
        let holders: Vec<_> = (0..3)
            .map(|_| {
                TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| {
                    panic!(concat!("reserve listener port", ": {:?}"), error)
                })
            })
            .collect();
        let addresses: Vec<_> = holders
            .iter()
            .map(|listener| {
                listener
                    .local_addr()
                    .unwrap_or_else(|error| panic!(concat!("listener address", ": {:?}"), error))
            })
            .collect();
        let dir = std::env::temp_dir().join(format!(
            "vnc-relay-target-scenarios-{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&dir)
            .unwrap_or_else(|error| panic!(concat!("unique test directory", ": {:?}"), error));
        write_private(
            &dir.join("accounts.toml"),
            "version = 1\n[[account]]\nname = 'alice'\npassword_hash = '$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA'\n",
        );
        let targets = targets
            .iter()
            .map(|target| format!("'{target}'"))
            .collect::<Vec<_>>()
            .join(", ");
        write_private(
            &dir.join("relay.toml"),
            &format!(
                r#"[admin]
accounts_file = 'accounts.toml'
[management]
listen = '{}'
cert_file = '{}'
key_file = '{}'
[status]
listen = '{}'
[relay.tcp_viewer]
enabled = true
listen = '{}'
allow = ['127.0.0.1']
[mode1]
enabled = true
allow_targets = [{targets}]
[limits]
relay_connections = {limit}
waiting = 1
admin_connections = 16
[timeouts]
handshake_secs = 120
tls_upgrade_secs = 120
wait_secs = 10
connect_secs = {connect_secs}
close_secs = 1
[log]
output = 'relay.log'
"#,
                addresses[1],
                fixture("server-chain.pem").display(),
                fixture("server-key-p256.pem").display(),
                addresses[2],
                addresses[0],
            ),
        );
        let mut command = Command::new(BIN);
        command
            .arg("serve")
            .arg("--config")
            .arg(dir.join("relay.toml"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        if dns_disabled {
            // On the supported Ubuntu/glibc environment, retry=0 makes DNS
            // resolution fail without sending a packet. No global resolver edit.
            command
                .env("RES_OPTIONS", "attempts:0")
                .env("LOCALDOMAIN", "");
        }
        drop(holders);
        let child = command
            .spawn()
            .unwrap_or_else(|error| panic!(concat!("start actual relay binary", ": {:?}"), error));
        let mut relay = Self {
            child,
            dir,
            viewer: addresses[0],
            management: addresses[1],
            status: addresses[2],
        };
        relay.wait_line(|line| field(line, "event") == Some("server_ready"));
        relay
    }

    fn log(&self) -> String {
        match std::fs::read_to_string(self.dir.join("relay.log")) {
            Ok(log) => log,
            Err(error) if error.kind() == ErrorKind::NotFound => String::new(),
            Err(error) => panic!("read relay log: {error}"),
        }
    }

    fn wait_line(&mut self, wanted: impl Fn(&str) -> bool) -> String {
        let deadline = Instant::now() + LIMIT;
        loop {
            let log = self.log();
            if let Some(line) = log.lines().find(|line| wanted(line)) {
                return line.to_owned();
            }
            if let Some(status) = self
                .child
                .try_wait()
                .unwrap_or_else(|error| panic!(concat!("poll relay", ": {:?}"), error))
            {
                let mut stderr = String::new();
                self.child
                    .stderr
                    .as_mut()
                    .unwrap_or_else(|| panic!("child stderr"))
                    .read_to_string(&mut stderr)
                    .unwrap_or_else(|error| panic!(concat!("read startup error", ": {:?}"), error));
                panic!("relay exited {status}: {stderr}; log: {log}");
            }
            assert!(Instant::now() < deadline, "missing log event: {log}");
            std::thread::yield_now();
        }
    }

    fn viewer(&mut self) -> (TcpStream, String) {
        let previous = self.log().len();
        let mut viewer = connect(self.viewer);
        let mut greeting = [0; 12];
        viewer
            .read_exact(&mut greeting)
            .unwrap_or_else(|error| panic!(concat!("viewer greeting", ": {:?}"), error));
        assert_eq!(&greeting, b"RFB 000.000\n");
        let deadline = Instant::now() + LIMIT;
        loop {
            let log = self.log();
            if let Some(line) = log[previous..]
                .lines()
                .find(|line| field(line, "event") == Some("connection_accepted"))
            {
                return (
                    viewer,
                    field(line, "conn_id")
                        .unwrap_or_else(|| panic!("accepted conn id"))
                        .to_owned(),
                );
            }
            assert!(Instant::now() < deadline, "missing acceptance: {log}");
            std::thread::yield_now();
        }
    }

    fn event(&mut self, event: &str, conn: &str) -> String {
        self.wait_line(|line| {
            field(line, "event") == Some(event) && field(line, "conn_id") == Some(conn)
        })
    }

    fn session(&mut self, listener: &TcpListener) -> (TcpStream, TcpStream, String) {
        let (mut viewer, conn) = self.viewer();
        request(
            &mut viewer,
            &listener
                .local_addr()
                .unwrap_or_else(|error| panic!(concat!("target address", ": {:?}"), error))
                .to_string(),
        );
        let mut target = accept(listener);
        let started = self.wait_line(|line| {
            field(line, "event") == Some("session_started")
                && field(line, "viewer_conn_id") == Some(conn.as_str())
        });
        assert_eq!(field(&started, "mode"), Some("mode1"));
        let session = field(&started, "session_id")
            .unwrap_or_else(|| panic!("session id"))
            .to_owned();
        round_trip(&mut viewer, &mut target);
        (viewer, target, session)
    }

    fn close_session(&mut self, mut viewer: TcpStream, mut target: TcpStream, session: &str) {
        viewer
            .shutdown(Shutdown::Write)
            .unwrap_or_else(|error| panic!(concat!("viewer FIN", ": {:?}"), error));
        assert_eof(&mut target);
        target
            .shutdown(Shutdown::Write)
            .unwrap_or_else(|error| panic!(concat!("target FIN", ": {:?}"), error));
        assert_eof(&mut viewer);
        self.wait_line(|line| {
            field(line, "event") == Some("session_closed")
                && field(line, "session_id") == Some(session)
        });
    }
}

impl Drop for Relay {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn connect(address: SocketAddr) -> TcpStream {
    let stream = TcpStream::connect_timeout(&address, LIMIT)
        .unwrap_or_else(|error| panic!(concat!("connect loopback socket", ": {:?}"), error));
    stream
        .set_read_timeout(Some(LIMIT))
        .unwrap_or_else(|error| panic!(concat!("read deadline", ": {:?}"), error));
    stream
        .set_write_timeout(Some(LIMIT))
        .unwrap_or_else(|error| panic!(concat!("write deadline", ": {:?}"), error));
    stream
}

fn listener() -> TcpListener {
    let listener = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|error| panic!(concat!("target listener", ": {:?}"), error));
    listener
        .set_nonblocking(true)
        .unwrap_or_else(|error| panic!(concat!("nonblocking accept", ": {:?}"), error));
    listener
}

fn accept(listener: &TcpListener) -> TcpStream {
    let deadline = Instant::now() + LIMIT;
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream
                    .set_nonblocking(false)
                    .unwrap_or_else(|error| panic!("blocking target stream: {error}"));
                stream
                    .set_read_timeout(Some(LIMIT))
                    .unwrap_or_else(|error| {
                        panic!(concat!("target read deadline", ": {:?}"), error)
                    });
                stream
                    .set_write_timeout(Some(LIMIT))
                    .unwrap_or_else(|error| {
                        panic!(concat!("target write deadline", ": {:?}"), error)
                    });
                return stream;
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                assert!(Instant::now() < deadline, "target connection never arrived");
                std::thread::yield_now();
            }
            Err(error) => panic!("target accept: {error}"),
        }
    }
}

fn assert_no_target(listener: &TcpListener) {
    assert_eq!(
        listener
            .accept()
            .err()
            .unwrap_or_else(|| panic!("forbidden target must not receive a socket"))
            .kind(),
        ErrorKind::WouldBlock
    );
}

fn request(viewer: &mut TcpStream, target: &str) {
    assert!(target.len() <= 250);
    let mut bytes = [0; 250];
    bytes[..target.len()].copy_from_slice(target.as_bytes());
    viewer
        .write_all(&bytes)
        .unwrap_or_else(|error| panic!(concat!("complete Mode I request", ": {:?}"), error));
}

fn assert_eof(stream: &mut TcpStream) {
    let mut byte = [0];
    assert_eq!(
        stream
            .read(&mut byte)
            .unwrap_or_else(|error| panic!("normal TCP FIN without error bytes: {error:?}")),
        0
    );
}

fn round_trip(viewer: &mut TcpStream, target: &mut TcpStream) {
    viewer
        .write_all(b"viewer-data")
        .unwrap_or_else(|error| panic!(concat!("viewer payload", ": {:?}"), error));
    let mut bytes = [0; 11];
    target
        .read_exact(&mut bytes)
        .unwrap_or_else(|error| panic!(concat!("target payload", ": {:?}"), error));
    assert_eq!(&bytes, b"viewer-data");
    target
        .write_all(b"target-data")
        .unwrap_or_else(|error| panic!(concat!("target reply", ": {:?}"), error));
    viewer
        .read_exact(&mut bytes)
        .unwrap_or_else(|error| panic!(concat!("viewer reply", ": {:?}"), error));
    assert_eq!(&bytes, b"target-data");
}

fn management_tls(relay: &Relay) -> Tls {
    let mut roots = RootCertStore::empty();
    for cert in CertificateDer::pem_file_iter(fixture("root-ca.pem"))
        .unwrap_or_else(|error| panic!(concat!("CA file", ": {:?}"), error))
    {
        roots
            .add(cert.unwrap_or_else(|error| panic!("CA certificate: {error:?}")))
            .unwrap_or_else(|error| panic!(concat!("trusted test CA", ": {:?}"), error));
    }
    let config = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let connection = ClientConnection::new(
        Arc::new(config),
        ServerName::try_from("localhost")
            .unwrap_or_else(|error| panic!("TLS server name: {error:?}")),
    )
    .unwrap_or_else(|error| panic!(concat!("TLS client", ": {:?}"), error));
    let mut tls = StreamOwned::new(connection, connect(relay.management));
    while tls.conn.is_handshaking() {
        tls.conn
            .complete_io(&mut tls.sock)
            .unwrap_or_else(|error| panic!(concat!("management TLS handshake", ": {:?}"), error));
    }
    // Keep an actual negotiated TLS socket occupied without ending its request.
    tls.write_all(b"GET /api/v1/session HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n")
        .unwrap_or_else(|error| {
            panic!(
                concat!("partial authenticated-boundary request", ": {:?}"),
                error
            )
        });
    tls.flush()
        .unwrap_or_else(|error| panic!(concat!("flush partial request", ": {:?}"), error));
    tls
}

fn assert_management_unauthenticated(tls: &mut Tls) {
    tls.write_all(b"\r\n")
        .unwrap_or_else(|error| panic!(concat!("finish management request", ": {:?}"), error));
    tls.flush()
        .unwrap_or_else(|error| panic!(concat!("flush management request", ": {:?}"), error));
    let mut headers = Vec::new();
    while !headers.ends_with(b"\r\n\r\n") {
        assert!(
            headers.len() < 16_384,
            "bounded management response headers"
        );
        let mut byte = [0];
        tls.read_exact(&mut byte)
            .unwrap_or_else(|error| panic!(concat!("management response header", ": {:?}"), error));
        headers.push(byte[0]);
    }
    let headers = String::from_utf8(headers)
        .unwrap_or_else(|error| panic!(concat!("HTTP response headers", ": {:?}"), error));
    assert!(headers.starts_with("HTTP/1.1 401 "), "{headers}");
    let length: usize = headers
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .unwrap_or_else(|| panic!("response Content-Length"))
        .1
        .trim()
        .parse()
        .unwrap_or_else(|error| panic!(concat!("response body length", ": {:?}"), error));
    assert!(length < 16_384, "bounded management response body");
    let mut body = vec![0; length];
    tls.read_exact(&mut body).unwrap_or_else(|error| {
        panic!(
            concat!("complete management response body", ": {:?}"),
            error
        )
    });
    let body = String::from_utf8(body)
        .unwrap_or_else(|error| panic!(concat!("management JSON response", ": {:?}"), error));
    assert!(body.contains("unauthenticated"), "{body}");
}

/// spec: relay.md:RLY-007
#[test]
fn forbidden_ip_and_forbidden_port_never_create_target_connections() {
    let allowed = listener();
    let allowed_address = allowed
        .local_addr()
        .unwrap_or_else(|error| panic!(concat!("allowed address", ": {:?}"), error));
    let denied_ip = TcpListener::bind(("127.0.0.2", allowed_address.port()))
        .unwrap_or_else(|error| panic!(concat!("different loopback target IP", ": {:?}"), error));
    denied_ip
        .set_nonblocking(true)
        .unwrap_or_else(|error| panic!(concat!("nonblocking denied IP", ": {:?}"), error));
    let denied_port = listener();
    let mut relay = Relay::start(&[allowed_address.to_string()], 2, 1, false);
    for denied in [&denied_ip, &denied_port] {
        let address = denied
            .local_addr()
            .unwrap_or_else(|error| panic!(concat!("denied address", ": {:?}"), error));
        let (mut viewer, conn) = relay.viewer();
        request(&mut viewer, &address.to_string());
        assert_eof(&mut viewer);
        let rejected = relay.event("connection_rejected", &conn);
        assert_eq!(field(&rejected, "reason"), Some("target_not_allowed"));
        assert_eq!(field(&rejected, "stage"), Some("target_connect"));
        assert_eq!(number(&rejected, "target_port"), u64::from(address.port()));
        assert_no_target(denied);
        assert_no_target(&allowed);
    }
    let (viewer, target, session) = relay.session(&allowed);
    relay.close_session(viewer, target, &session);
}

/// spec: relay.md:RLY-008
#[test]
fn allowed_hostname_cannot_connect_to_a_forbidden_resolved_ip() {
    let ipv4 = listener();
    let port = ipv4
        .local_addr()
        .unwrap_or_else(|error| panic!(concat!("target port", ": {:?}"), error))
        .port();
    let mut listeners = vec![ipv4];
    let addresses: std::collections::HashSet<_> = ("localhost", port)
        .to_socket_addrs()
        .unwrap_or_else(|error| {
            panic!(
                concat!("local resolver supports localhost", ": {:?}"),
                error
            )
        })
        .collect();
    assert!(!addresses.is_empty(), "localhost has resolver candidates");
    for address in addresses {
        assert!(
            address.ip().is_loopback(),
            "localhost fixture must be local: {address}"
        );
        if address
            != listeners[0]
                .local_addr()
                .unwrap_or_else(|error| panic!(concat!("IPv4 listener", ": {:?}"), error))
        {
            let listener = TcpListener::bind(address).unwrap_or_else(|error| {
                panic!(concat!("resolved local target listener", ": {:?}"), error)
            });
            listener.set_nonblocking(true).unwrap_or_else(|error| {
                panic!(concat!("nonblocking local target", ": {:?}"), error)
            });
            listeners.push(listener);
        }
    }
    let mut relay = Relay::start(
        &[format!("localhost:{port}"), format!("192.0.2.1:{port}")],
        2,
        1,
        false,
    );
    let (mut viewer, conn) = relay.viewer();
    request(&mut viewer, &format!("localhost:{port}"));
    assert_eof(&mut viewer);
    let rejected = relay.event("connection_rejected", &conn);
    assert_eq!(field(&rejected, "stage"), Some("target_connect"));
    assert_eq!(field(&rejected, "reason"), Some("target_not_allowed"));
    assert_eq!(field(&rejected, "target_host"), Some("localhost"));
    assert_eq!(number(&rejected, "target_port"), u64::from(port));
    assert!(
        field(&rejected, "target_ip").is_some(),
        "final IP was checked: {rejected}"
    );
    for listener in &listeners {
        assert_no_target(listener);
    }
    assert!(!relay.log().lines().any(|line| {
        matches!(
            field(line, "event"),
            Some("target_connect_failed" | "session_started")
        )
    }));
}

/// A local backlog is filled with completed connections and deliberately never
/// accepted. A bounded extra connect must time out before it is a valid fixture.
/// This uses no remote address, packet filtering, privileges, or sleeps.
#[cfg(target_os = "linux")]
struct Backlog {
    listener: TcpListener,
    _fillers: Vec<TcpStream>,
}

#[cfg(target_os = "linux")]
impl Backlog {
    fn saturated() -> Self {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .unwrap_or_else(|error| panic!(concat!("backlog runtime", ": {:?}"), error));
        let listener = runtime.block_on(async {
            let socket = tokio::net::TcpSocket::new_v4()
                .unwrap_or_else(|error| panic!(concat!("backlog socket", ": {:?}"), error));
            socket
                .bind(
                    "127.0.0.1:0"
                        .parse()
                        .unwrap_or_else(|error| panic!("loopback address: {error:?}")),
                )
                .unwrap_or_else(|error| panic!(concat!("backlog bind", ": {:?}"), error));
            socket
                .listen(1)
                .unwrap_or_else(|error| panic!("small local backlog: {error:?}"))
                .into_std()
                .unwrap_or_else(|error| panic!(concat!("standard listener", ": {:?}"), error))
        });
        let address = listener
            .local_addr()
            .unwrap_or_else(|error| panic!(concat!("backlog address", ": {:?}"), error));
        let mut fillers = Vec::new();
        for _ in 0..64 {
            match TcpStream::connect_timeout(&address, Duration::from_millis(100)) {
                Ok(stream) => fillers.push(stream),
                Err(error)
                    if matches!(error.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock) =>
                {
                    assert!(
                        !fillers.is_empty(),
                        "backlog contains established connections"
                    );
                    return Self {
                        listener,
                        _fillers: fillers,
                    };
                }
                Err(error) => panic!("OS cannot supply the local pending-connect fixture: {error}"),
            }
        }
        panic!("OS did not enforce a bounded local listen backlog");
    }

    fn address(&self) -> SocketAddr {
        self.listener
            .local_addr()
            .unwrap_or_else(|error| panic!(concat!("backlog target address", ": {:?}"), error))
    }
}

/// spec: relay.md:RLY-009
#[cfg(target_os = "linux")]
#[test]
fn distinguishes_resolver_failure_refusal_and_c_timeout_and_reclaims_every_socket() {
    const CONNECT_SECS: u64 = 10;
    const INODE_ACQUISITION: Duration = Duration::from_secs(3);
    assert!(INODE_ACQUISITION < Duration::from_secs(CONNECT_SECS));
    // A malformed name cannot reach the resolver because the handshake validates
    // label lengths and syntax. Child-only glibc attempts:0 supplies the failure
    // instead; unlike .invalid it cannot depend on upstream DNS or connectivity.
    let recovery = listener();
    let refused = listener();
    let refused_address = refused
        .local_addr()
        .unwrap_or_else(|error| panic!(concat!("refused address", ": {:?}"), error));
    drop(refused);
    let backlog = Backlog::saturated();
    let missing = format!("relay-target-{}.example", std::process::id());
    let mut relay = Relay::start(
        &[
            recovery
                .local_addr()
                .unwrap_or_else(|error| panic!(concat!("recovery address", ": {:?}"), error))
                .to_string(),
            refused_address.to_string(),
            backlog.address().to_string(),
            format!("{missing}:5900"),
        ],
        2,
        CONNECT_SECS,
        true,
    );
    let baseline_sockets = child_socket_inodes(&relay);
    for (address, reason) in [
        (format!("{missing}:5900"), "resolve_failed"),
        (refused_address.to_string(), "connect_refused"),
        (backlog.address().to_string(), "connect_timeout"),
    ] {
        let (mut viewer, conn) = relay.viewer();
        let began = Instant::now();
        request(&mut viewer, &address);
        let connecting_sockets = if reason == "connect_timeout" {
            // Leave C ample time after the parent's bounded /proc observation.
            let deadline = began + INODE_ACQUISITION;
            loop {
                assert!(
                    Instant::now() < deadline,
                    "child-owned SYN_SENT inode not acquired within {INODE_ACQUISITION:?}: {}",
                    relay.log()
                );
                let sockets = connecting_socket_inodes(&relay, backlog.address());
                if !sockets.is_empty() {
                    assert!(
                        sockets.is_disjoint(&baseline_sockets),
                        "pending target sockets must be new child descriptors"
                    );
                    break sockets;
                }
                std::thread::yield_now();
            }
        } else {
            HashSet::new()
        };
        assert_eof(&mut viewer);
        let failed = relay.event("target_connect_failed", &conn);
        assert_eq!(field(&failed, "reason"), Some(reason), "{failed}");
        assert_eq!(field(&failed, "viewer_transport"), Some("tcp"));
        assert_eq!(field(&failed, "listener"), Some("tcp_viewer"));
        if reason == "connect_timeout" {
            assert!(
                number(&failed, "elapsed_ms") >= CONNECT_SECS * 1000,
                "C must elapse: {failed}"
            );
            assert!(began.elapsed() >= Duration::from_secs(CONNECT_SECS));
            assert!(began.elapsed() < LIMIT, "C is bounded");
        }
        // A returned permit does not prove that the OS socket was closed.
        // Check the child's fd links directly, including the exact SYN_SENT
        // inode observed above: it can vanish from the TCP table before the
        // child actually drops its descriptor. DNS/refusal must also leave no
        // new socket descriptors behind. TIME_WAIT entries owned by the kernel
        // and the test's backlog filler sockets do not count as child fds.
        let deadline = Instant::now() + LIMIT;
        loop {
            let remaining = child_socket_inodes(&relay);
            if remaining.is_subset(&baseline_sockets) && remaining.is_disjoint(&connecting_sockets)
            {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "{reason} left child socket descriptors: new={:?}, connecting={:?}",
                remaining.difference(&baseline_sockets).collect::<Vec<_>>(),
                remaining
                    .intersection(&connecting_sockets)
                    .collect::<Vec<_>>()
            );
            std::thread::yield_now();
        }
        // L=2: succeeding here requires reclamation of the failed Viewer and
        // the created/connecting target permit, not merely a free listener.
        let (viewer, target, session) = relay.session(&recovery);
        relay.close_session(viewer, target, &session);
    }
    let reasons: Vec<_> = relay
        .log()
        .lines()
        .filter(|line| field(line, "event") == Some("target_connect_failed"))
        .map(|line| {
            field(line, "reason")
                .unwrap_or_else(|| panic!("failure reason"))
                .to_owned()
        })
        .collect();
    assert_eq!(
        reasons,
        ["resolve_failed", "connect_refused", "connect_timeout"]
    );
}

#[cfg(target_os = "linux")]
fn child_socket_inodes(relay: &Relay) -> HashSet<u64> {
    let entries = std::fs::read_dir(format!("/proc/{}/fd", relay.child.id()))
        .unwrap_or_else(|error| panic!("read child socket descriptors: {error}"));
    let mut inodes = HashSet::new();
    for entry in entries {
        let entry = entry.unwrap_or_else(|error| panic!("read child descriptor entry: {error}"));
        let link = match std::fs::read_link(entry.path()) {
            Ok(link) => link,
            // The child can close a descriptor between read_dir and read_link.
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(error) => panic!("read child descriptor link: {error}"),
        };
        if let Some(inode) = link
            .to_str()
            .and_then(|link| link.strip_prefix("socket:["))
            .and_then(|link| link.strip_suffix(']'))
        {
            inodes.insert(
                inode
                    .parse()
                    .unwrap_or_else(|error| panic!("child socket inode: {error}")),
            );
        }
    }
    inodes
}

#[cfg(target_os = "linux")]
fn proc_tcp_remote(address: SocketAddr) -> String {
    let ip = match address.ip() {
        std::net::IpAddr::V4(ip) => format!("{:08X}", u32::from_ne_bytes(ip.octets())),
        std::net::IpAddr::V6(ip) => {
            let mut encoded = String::with_capacity(32);
            for word in ip.octets().as_chunks::<4>().0 {
                use std::fmt::Write as _;
                write!(encoded, "{:08X}", u32::from_ne_bytes(*word))
                    .unwrap_or_else(|error| panic!("encode Linux IPv6 address: {error}"));
            }
            encoded
        }
    };
    format!("{ip}:{:04X}", address.port())
}

#[cfg(target_os = "linux")]
fn connecting_socket_inodes(relay: &Relay, address: SocketAddr) -> HashSet<u64> {
    let owned = child_socket_inodes(relay);
    let remote = proc_tcp_remote(address);
    let mapped = match address.ip() {
        std::net::IpAddr::V4(ip) => Some(proc_tcp_remote(SocketAddr::new(
            ip.to_ipv6_mapped().into(),
            address.port(),
        ))),
        std::net::IpAddr::V6(_) => None,
    };
    let mut connecting = HashSet::new();
    for table in ["tcp", "tcp6"] {
        // These tables are namespace-wide, even under /proc/<pid>/net.
        // Only an inode also present in this child's fd links is evidence.
        let table = std::fs::read_to_string(format!("/proc/{}/net/{table}", relay.child.id()))
            .unwrap_or_else(|error| panic!("read Linux TCP state table: {error}"));
        for line in table.lines().skip(1) {
            let mut columns = line.split_whitespace();
            let target = columns.nth(2);
            let state = columns.next();
            if (target == Some(remote.as_str())
                || mapped
                    .as_deref()
                    .is_some_and(|mapped| target == Some(mapped)))
                && state == Some("02")
            {
                let inode = columns
                    .nth(5)
                    .unwrap_or_else(|| panic!("Linux TCP socket inode column"))
                    .parse::<u64>()
                    .unwrap_or_else(|error| panic!("Linux TCP socket inode: {error}"));
                if owned.contains(&inode) {
                    connecting.insert(inode);
                }
            }
        }
    }
    connecting
}

/// spec: operations.md:OPS-023
#[cfg(target_os = "linux")]
#[test]
fn counts_connecting_and_connected_target_sockets_but_not_management_tls() {
    const CONNECT_SECS: u64 = 30;
    const INODE_ACQUISITION: Duration = Duration::from_secs(3);
    assert!(INODE_ACQUISITION < Duration::from_secs(CONNECT_SECS));
    let active_listener = listener();
    let recovery = listener();
    let backlog = Backlog::saturated();
    let mut relay = Relay::start(
        &[
            active_listener
                .local_addr()
                .unwrap_or_else(|error| panic!(concat!("active target", ": {:?}"), error))
                .to_string(),
            recovery
                .local_addr()
                .unwrap_or_else(|error| panic!(concat!("recovery target", ": {:?}"), error))
                .to_string(),
            backlog.address().to_string(),
        ],
        5,
        CONNECT_SECS,
        false,
    );
    // More negotiated management TLS connections than the entire L budget.
    // They stay open throughout the boundary tests and use only independent Lm.
    let mut management: Vec<_> = (0..6).map(|_| management_tls(&relay)).collect();
    let mut status = connect(relay.status);
    status
        .write_all(b"GET /livez HTTP/1.1\r\nHost: localhost\r\n")
        .unwrap_or_else(|error| panic!(concat!("hold status socket", ": {:?}"), error));
    let (mut active_viewer, mut active_target, active_session) = relay.session(&active_listener);
    let (mut pending, pending_conn) = relay.viewer();
    let began = Instant::now();
    request(&mut pending, &backlog.address().to_string());
    let deadline = began + INODE_ACQUISITION;
    loop {
        assert!(
            Instant::now() < deadline,
            "child-owned SYN_SENT inode not acquired within {INODE_ACQUISITION:?}: {}",
            relay.log()
        );
        if !connecting_socket_inodes(&relay, backlog.address()).is_empty() {
            break;
        }
        std::thread::yield_now();
    }
    // Active pair (2) + pending Viewer/target (2) + accepted contender (1) = L.
    let (mut contender, contender_conn) = relay.viewer();
    let mut over_limit = connect(relay.viewer);
    assert_eof(&mut over_limit);
    let rejected = relay.wait_line(|line| {
        field(line, "event") == Some("connection_rejected")
            && field(line, "stage") == Some("accept")
            && field(line, "reason") == Some("relay_connection_limit")
    });
    assert_eq!(number(&rejected, "limit"), 5);
    // The accepted contender has no second slot: it must not dial this listener.
    request(
        &mut contender,
        &recovery
            .local_addr()
            .unwrap_or_else(|error| panic!(concat!("recovery address", ": {:?}"), error))
            .to_string(),
    );
    assert_eof(&mut contender);
    let rejected = relay.event("connection_rejected", &contender_conn);
    assert_eq!(field(&rejected, "stage"), Some("target_connect"));
    assert_eq!(field(&rejected, "reason"), Some("relay_connection_limit"));
    assert_eq!(number(&rejected, "limit"), 5);
    assert_no_target(&recovery);
    assert_no_target(&active_listener);
    round_trip(&mut active_viewer, &mut active_target);
    // A still-pending target socket owns the fourth place even though no
    // session_started exists for it. Releasing the local backlog ends the fault.
    assert!(!relay.log().lines().any(|line| {
        field(line, "event") == Some("session_started")
            && field(line, "viewer_conn_id") == Some(pending_conn.as_str())
    }));
    drop(backlog);
    assert_eof(&mut pending);
    relay.event("target_connect_failed", &pending_conn);
    let (viewer, target, session) = relay.session(&recovery);
    relay.close_session(viewer, target, &session);
    round_trip(&mut active_viewer, &mut active_target);
    // Negotiated management TLS sockets remain usable at/after relay exhaustion.
    for tls in &mut management {
        assert_management_unauthenticated(tls);
    }
    assert!(!relay.log().lines().any(|line| {
        field(line, "event") == Some("session_closed")
            && field(line, "session_id") == Some(active_session.as_str())
    }));
    relay.close_session(active_viewer, active_target, &active_session);
}

// Windows listen() documents WSAECONNREFUSED for a full queue, so it cannot
// provide the pending-connect premise of the full Linux test above. Keep the
// independently automatable budget/TLS/session assertions running on both OSes.
/// spec: operations.md:OPS-023 partial
#[test]
fn target_budget_denial_preserves_sessions_and_excludes_management_tls() {
    let active_listener = listener();
    let denied_listener = listener();
    let mut relay = Relay::start(
        &[
            active_listener
                .local_addr()
                .unwrap_or_else(|error| panic!(concat!("active address", ": {:?}"), error))
                .to_string(),
            denied_listener
                .local_addr()
                .unwrap_or_else(|error| panic!(concat!("denied address", ": {:?}"), error))
                .to_string(),
        ],
        3,
        1,
        false,
    );
    let mut management: Vec<_> = (0..4).map(|_| management_tls(&relay)).collect();
    let mut status = connect(relay.status);
    status
        .write_all(b"GET /livez HTTP/1.1\r\nHost: localhost\r\n")
        .unwrap_or_else(|error| panic!(concat!("hold independent status socket", ": {:?}"), error));
    let (mut viewer, mut target, session) = relay.session(&active_listener);
    let (mut contender, conn) = relay.viewer();
    request(
        &mut contender,
        &denied_listener
            .local_addr()
            .unwrap_or_else(|error| panic!(concat!("denied target", ": {:?}"), error))
            .to_string(),
    );
    assert_eof(&mut contender);
    let rejected = relay.event("connection_rejected", &conn);
    assert_eq!(field(&rejected, "stage"), Some("target_connect"));
    assert_eq!(field(&rejected, "reason"), Some("relay_connection_limit"));
    assert_eq!(number(&rejected, "limit"), 3);
    assert_no_target(&denied_listener);
    assert_no_target(&active_listener);
    round_trip(&mut viewer, &mut target);
    for tls in &mut management {
        assert_management_unauthenticated(tls);
    }
    assert!(!relay.log().lines().any(|line| {
        field(line, "event") == Some("session_closed")
            && field(line, "session_id") == Some(session.as_str())
    }));
    relay.close_session(viewer, target, &session);
    let (viewer, target, session) = relay.session(&denied_listener);
    relay.close_session(viewer, target, &session);
}

/// spec: relay.md:RLY-009 partial
#[test]
fn connection_refusal_is_reported_and_releases_both_socket_places() {
    let recovery = listener();
    let refused = listener();
    let address = refused
        .local_addr()
        .unwrap_or_else(|error| panic!(concat!("refused address", ": {:?}"), error));
    drop(refused);
    let mut relay = Relay::start(
        &[
            recovery
                .local_addr()
                .unwrap_or_else(|error| panic!(concat!("recovery address", ": {:?}"), error))
                .to_string(),
            address.to_string(),
        ],
        2,
        5,
        false,
    );
    let (mut viewer, conn) = relay.viewer();
    request(&mut viewer, &address.to_string());
    assert_eof(&mut viewer);
    let failed = relay.event("target_connect_failed", &conn);
    assert_eq!(field(&failed, "reason"), Some("connect_refused"));
    assert_eq!(field(&failed, "target_ip"), Some("127.0.0.1"));
    assert_eq!(number(&failed, "target_port"), u64::from(address.port()));
    let (viewer, target, session) = relay.session(&recovery);
    relay.close_session(viewer, target, &session);
}
