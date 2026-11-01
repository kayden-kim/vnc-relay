//! Mode II acceptance checks against the real executable and loopback sockets.

use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Barrier, Mutex, PoisonError};
use std::time::{Duration, Instant};

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};

const BIN: &str = env!("CARGO_BIN_EXE_vnc-relay");
const LIMIT: Duration = Duration::from_secs(30);
const GREETING: &[u8] = b"RFB 000.000\n";
static PORTS: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy, Debug)]
enum Transport {
    Tcp,
    Ws,
    Wss,
}

impl Transport {
    fn listener(self) -> &'static str {
        match self {
            Self::Tcp => "tcp_viewer",
            Self::Ws => "ws_viewer",
            Self::Wss => "wss_viewer",
        }
    }
}

fn field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("\"{name}\":\"");
    let rest = &line[line.find(&prefix)? + prefix.len()..];
    Some(&rest[..rest.find('"')?])
}

fn event(line: &str, name: &str) -> bool {
    field(line, "event") == Some(name)
}

fn connection(line: &str, id: &str) -> bool {
    field(line, "conn_id") == Some(id)
}

fn session_uses(line: &str, id: &str) -> bool {
    field(line, "viewer_conn_id") == Some(id) || field(line, "server_conn_id") == Some(id)
}

fn request(id: &str) -> [u8; 250] {
    let text = format!("ID:{id}");
    assert!(text.len() <= 250);
    let mut bytes = [0; 250];
    bytes[..text.len()].copy_from_slice(text.as_bytes());
    bytes
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
            .unwrap_or_else(|error| panic!(concat!("protect test account file", ": {:?}"), error));
    }
}

struct Running {
    child: Child,
    dir: PathBuf,
    ports: [u16; 6],
}

impl Running {
    fn start(name: &str, waiting: usize, wait_secs: u64, close_secs: u64) -> Self {
        // Keep reservations distinct until the entire configuration is written.
        let holders: Vec<_> = (0..6)
            .map(|_| {
                TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| {
                    panic!(concat!("reserve ephemeral listener", ": {:?}"), error)
                })
            })
            .collect();
        let ports = std::array::from_fn(|i| {
            holders[i]
                .local_addr()
                .unwrap_or_else(|error| panic!(concat!("reserved address", ": {:?}"), error))
                .port()
        });
        let dir =
            std::env::temp_dir().join(format!("vnc-relay-matching-{}-{name}", std::process::id()));
        std::fs::create_dir(&dir).unwrap_or_else(|error| {
            panic!(
                concat!("create isolated matching directory", ": {:?}"),
                error
            )
        });
        write_private(
            &dir.join("admin-accounts.toml"),
            "version = 1\n[[account]]\nname = 'alice'\npassword_hash = '$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA'\n",
        );
        let config = format!(
            r#"[admin]
accounts_file = 'admin-accounts.toml'
[management]
listen = '127.0.0.1:{}'
[status]
listen = '127.0.0.1:{}'
[relay.tcp_viewer]
enabled = true
listen = '127.0.0.1:{}'
allow = ['127.0.0.1']
[relay.tcp_server]
enabled = true
listen = '127.0.0.1:{}'
allow = ['127.0.0.1']
[relay.ws_viewer]
enabled = true
listen = '127.0.0.1:{}'
allow = ['127.0.0.1']
[relay.wss_viewer]
enabled = true
listen = '127.0.0.1:{}'
allow = ['127.0.0.1']
cert_file = '{}'
key_file = '{}'
[mode1]
enabled = false
[mode2]
enabled = true
[limits]
relay_connections = 64
waiting = {waiting}
buffer_bytes = 16384
ws_message_bytes = 65536
[timeouts]
handshake_secs = 20
tls_upgrade_secs = 10
wait_secs = {wait_secs}
connect_secs = 1
close_secs = {close_secs}
[log]
output = 'relay.log'
"#,
            ports[0],
            ports[1],
            ports[2],
            ports[3],
            ports[4],
            ports[5],
            fixture("server-chain.pem").display(),
            fixture("server-key-p256.pem").display(),
        );
        let config_path = dir.join("relay.toml");
        write_private(&config_path, &config);
        drop(holders);
        let child = Command::new(BIN)
            .args(["serve", "--config"])
            .arg(config_path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|error| {
                panic!(concat!("start real relay executable", ": {:?}"), error)
            });
        let mut running = Self { child, dir, ports };
        running.wait_for(|lines| lines.iter().any(|line| event(line, "server_ready")));
        running
    }

    fn logs(&self) -> Vec<String> {
        let text = match std::fs::read_to_string(self.dir.join("relay.log")) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => return Vec::new(),
            Err(error) => panic!("read relay log: {error}"),
        };
        // Ignore an in-progress final write, never parse a truncated event.
        text.rsplit_once('\n')
            .map_or_else(Vec::new, |(complete, _)| {
                complete.lines().map(str::to_owned).collect()
            })
    }

    fn wait_for(&mut self, predicate: impl Fn(&[String]) -> bool) -> Vec<String> {
        let deadline = Instant::now() + LIMIT;
        loop {
            let lines = self.logs();
            if predicate(&lines) {
                return lines;
            }
            if let Some(status) = self
                .child
                .try_wait()
                .unwrap_or_else(|error| panic!(concat!("inspect child", ": {:?}"), error))
            {
                let mut stderr = String::new();
                self.child
                    .stderr
                    .as_mut()
                    .unwrap_or_else(|| panic!("child stderr"))
                    .read_to_string(&mut stderr)
                    .unwrap_or_else(|error| {
                        panic!(concat!("read child diagnostics", ": {:?}"), error)
                    });
                panic!("relay exited {status}: {stderr}; logs: {lines:?}");
            }
            assert!(Instant::now() < deadline, "event deadline; logs: {lines:?}");
            std::thread::yield_now();
        }
    }

    fn wait_event(&mut self, name: &str, conn: &str) -> String {
        self.wait_for(|lines| {
            lines
                .iter()
                .any(|line| event(line, name) && connection(line, conn))
        })
        .into_iter()
        .find(|line| event(line, name) && connection(line, conn))
        .unwrap_or_else(|| panic!("matching connection event"))
    }

    fn started(&mut self, conn: &str) -> String {
        self.wait_for(|lines| {
            lines
                .iter()
                .any(|line| event(line, "session_started") && session_uses(line, conn))
        })
        .into_iter()
        .find(|line| event(line, "session_started") && session_uses(line, conn))
        .unwrap_or_else(|| panic!("session started"))
    }

    fn closed(&mut self, started: &str) {
        let id = field(started, "session_id").unwrap_or_else(|| panic!("session identifier"));
        self.wait_for(|lines| {
            lines
                .iter()
                .any(|line| event(line, "session_closed") && field(line, "session_id") == Some(id))
        });
    }

    fn open(&mut self, viewer: bool, transport: Transport) -> Peer {
        assert!(viewer || matches!(transport, Transport::Tcp));
        let listener = if viewer {
            transport.listener()
        } else {
            "tcp_server"
        };
        let previous: Vec<_> = self
            .logs()
            .iter()
            .filter(|line| {
                event(line, "connection_accepted") && field(line, "listener") == Some(listener)
            })
            .map(|line| {
                field(line, "conn_id")
                    .unwrap_or_else(|| panic!("accepted conn_id"))
                    .to_owned()
            })
            .collect();
        let port = if viewer {
            match transport {
                Transport::Tcp => self.ports[2],
                Transport::Ws => self.ports[4],
                Transport::Wss => self.ports[5],
            }
        } else {
            self.ports[3]
        };
        let tcp = TcpStream::connect_timeout(&([127, 0, 0, 1], port).into(), LIMIT)
            .unwrap_or_else(|error| panic!(concat!("connect real loopback peer", ": {:?}"), error));
        tcp.set_read_timeout(Some(LIMIT))
            .unwrap_or_else(|error| panic!(concat!("bounded socket read", ": {:?}"), error));
        tcp.set_write_timeout(Some(LIMIT))
            .unwrap_or_else(|error| panic!(concat!("bounded socket write", ": {:?}"), error));
        tcp.set_nodelay(true)
            .unwrap_or_else(|error| panic!(concat!("disable client Nagle delay", ": {:?}"), error));
        let socket = match transport {
            Transport::Tcp => Socket::Tcp(tcp),
            Transport::Ws => Socket::WebSocket(WebSocket::upgrade(Box::new(tcp))),
            Transport::Wss => {
                let mut roots = RootCertStore::empty();
                for cert in CertificateDer::pem_file_iter(fixture("root-ca.pem"))
                    .unwrap_or_else(|error| panic!(concat!("open fixture root", ": {:?}"), error))
                {
                    roots
                        .add(cert.unwrap_or_else(|error| {
                            panic!(concat!("root PEM certificate", ": {:?}"), error)
                        }))
                        .unwrap_or_else(|error| {
                            panic!(concat!("trusted test root", ": {:?}"), error)
                        });
                }
                let config = ClientConfig::builder_with_provider(Arc::new(
                    rustls::crypto::aws_lc_rs::default_provider(),
                ))
                .with_safe_default_protocol_versions()
                .unwrap_or_else(|error| panic!(concat!("supported TLS versions", ": {:?}"), error))
                .with_root_certificates(roots)
                .with_no_client_auth();
                let tls = ClientConnection::new(
                    Arc::new(config),
                    ServerName::try_from("localhost").unwrap_or_else(|error| {
                        panic!(concat!("fixture hostname", ": {:?}"), error)
                    }),
                )
                .unwrap_or_else(|error| panic!(concat!("verified TLS client", ": {:?}"), error));
                Socket::WebSocket(WebSocket::upgrade(Box::new(StreamOwned::new(tls, tcp))))
            }
        };
        let mut peer = Peer {
            socket,
            conn: String::new(),
        };
        if viewer {
            assert_eq!(peer.read_data(GREETING.len()), GREETING);
        }
        let lines = self.wait_for(|lines| {
            lines.iter().any(|line| {
                event(line, "connection_accepted")
                    && field(line, "listener") == Some(listener)
                    && field(line, "conn_id")
                        .is_some_and(|id| !previous.iter().any(|old| old == id))
            })
        });
        peer.conn = lines
            .iter()
            .find_map(|line| {
                (event(line, "connection_accepted") && field(line, "listener") == Some(listener))
                    .then(|| field(line, "conn_id"))
                    .flatten()
                    .filter(|id| !previous.iter().any(|old| old == id))
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| panic!("new accepted connection"));
        peer
    }

    fn register(&mut self, viewer: bool, transport: Transport, id: &str, data: &[u8]) -> Peer {
        let mut peer = self.open(viewer, transport);
        peer.send_request(id, data);
        self.wait_event("waiting_registered", &peer.conn);
        peer
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

trait Wire: Read + Write + Send {}
impl<T: Read + Write + Send> Wire for T {}

struct WebSocket {
    wire: Box<dyn Wire>,
}

impl WebSocket {
    fn upgrade(mut wire: Box<dyn Wire>) -> Self {
        wire.write_all(b"GET /relay HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n").unwrap_or_else(|error| panic!(concat!("send real WebSocket upgrade", ": {:?}"), error));
        wire.flush()
            .unwrap_or_else(|error| panic!(concat!("flush upgrade", ": {:?}"), error));
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            wire.read_exact(&mut byte)
                .unwrap_or_else(|error| panic!(concat!("read upgrade response", ": {:?}"), error));
            head.push(byte[0]);
            assert!(head.len() < 16384, "bounded HTTP response header");
        }
        assert!(head.starts_with(b"HTTP/1.1 101 "), "{head:?}");
        Self { wire }
    }

    fn send(&mut self, opcode: u8, payload: &[u8]) {
        assert!(payload.len() <= u16::MAX as usize);
        let mut bytes = vec![0x80 | opcode];
        if payload.len() < 126 {
            bytes.push(
                0x80 | u8::try_from(payload.len())
                    .unwrap_or_else(|error| panic!(concat!("short frame", ": {:?}"), error)),
            );
        } else {
            bytes.push(0x80 | 126);
            bytes.extend_from_slice(
                &u16::try_from(payload.len())
                    .unwrap_or_else(|error| panic!(concat!("frame length", ": {:?}"), error))
                    .to_be_bytes(),
            );
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
            .unwrap_or_else(|error| panic!(concat!("send masked client frame", ": {:?}"), error));
        self.wire
            .flush()
            .unwrap_or_else(|error| panic!(concat!("flush client frame", ": {:?}"), error));
    }

    fn frame(&mut self) -> (u8, Vec<u8>) {
        loop {
            let mut head = [0; 2];
            self.wire.read_exact(&mut head).unwrap_or_else(|error| {
                panic!(concat!("read server frame header", ": {:?}"), error)
            });
            assert_eq!(head[0] & 0xf0, 0x80, "final frame without extensions");
            assert_eq!(head[1] & 0x80, 0, "server frames are unmasked");
            let len = match head[1] & 0x7f {
                126 => {
                    let mut bytes = [0; 2];
                    self.wire
                        .read_exact(&mut bytes)
                        .unwrap_or_else(|error| panic!(concat!("frame length", ": {:?}"), error));
                    usize::from(u16::from_be_bytes(bytes))
                }
                127 => {
                    let mut bytes = [0; 8];
                    self.wire
                        .read_exact(&mut bytes)
                        .unwrap_or_else(|error| panic!(concat!("frame length", ": {:?}"), error));
                    usize::try_from(u64::from_be_bytes(bytes)).unwrap_or_else(|error| {
                        panic!(concat!("length fits usize", ": {:?}"), error)
                    })
                }
                len => usize::from(len),
            };
            assert!(len <= 65536, "server frame remains within configured M");
            let mut payload = vec![0; len];
            self.wire
                .read_exact(&mut payload)
                .unwrap_or_else(|error| panic!(concat!("server frame payload", ": {:?}"), error));
            match head[0] & 0x0f {
                9 => self.send(10, &payload),
                10 => {}
                opcode => return (opcode, payload),
            }
        }
    }
}

enum Socket {
    Tcp(TcpStream),
    WebSocket(WebSocket),
}

struct Peer {
    socket: Socket,
    conn: String,
}

impl Peer {
    fn send(&mut self, bytes: &[u8]) {
        match &mut self.socket {
            Socket::Tcp(stream) => stream
                .write_all(bytes)
                .unwrap_or_else(|error| panic!(concat!("write TCP stream bytes", ": {:?}"), error)),
            Socket::WebSocket(ws) => ws.send(2, bytes),
        }
    }

    fn send_request(&mut self, id: &str, trailing: &[u8]) {
        let mut bytes = request(id).to_vec();
        bytes.extend_from_slice(trailing);
        self.send(&bytes);
    }

    fn read_data(&mut self, len: usize) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(len);
        match &mut self.socket {
            Socket::Tcp(stream) => {
                bytes.resize(len, 0);
                stream.read_exact(&mut bytes).unwrap_or_else(|error| {
                    panic!(concat!("read exact relayed TCP bytes", ": {:?}"), error)
                });
            }
            Socket::WebSocket(ws) => {
                while bytes.len() < len {
                    let (opcode, payload) = ws.frame();
                    assert_eq!(opcode, 2, "relay data is binary");
                    bytes.extend_from_slice(&payload);
                }
                assert_eq!(bytes.len(), len, "no unexpected relayed data");
            }
        }
        bytes
    }

    fn reject(&mut self, code: u16, reason: &str) {
        match &mut self.socket {
            Socket::Tcp(stream) => {
                let mut byte = [0];
                assert_eq!(
                    stream.read(&mut byte).unwrap_or_else(|error| panic!(
                        concat!("TCP rejection must be FIN, not reset", ": {:?}"),
                        error
                    )),
                    0
                );
            }
            Socket::WebSocket(ws) => {
                let (opcode, payload) = ws.frame();
                assert_eq!(opcode, 8, "rejection sends WebSocket Close");
                let mut expected = code.to_be_bytes().to_vec();
                expected.extend_from_slice(reason.as_bytes());
                assert_eq!(payload, expected, "exact close code and contracted reason");
                ws.send(8, &payload);
            }
        }
    }

    fn shutdown(&mut self) {
        match &mut self.socket {
            Socket::Tcp(stream) => stream
                .shutdown(Shutdown::Both)
                .unwrap_or_else(|error| panic!(concat!("disconnect TCP peer", ": {:?}"), error)),
            Socket::WebSocket(ws) => ws.send(8, &1000_u16.to_be_bytes()),
        }
    }
}

fn assert_pair(started: &str, viewer: &Peer, server: &Peer) {
    assert_eq!(field(started, "viewer_conn_id"), Some(viewer.conn.as_str()));
    assert_eq!(field(started, "server_conn_id"), Some(server.conn.as_str()));
    assert_eq!(field(started, "mode"), Some("mode2"));
}

fn assert_no_registration(lines: &[String], conn: &str) {
    assert!(
        !lines
            .iter()
            .any(|line| event(line, "waiting_registered") && connection(line, conn)),
        "rejected connection registered: {lines:?}"
    );
}

/// spec: relay.md:RLY-012
#[test]
fn different_ids_same_roles_and_leading_zero_collisions_never_cross_match() {
    let _ports = PORTS.lock().unwrap_or_else(PoisonError::into_inner);
    let mut relay = Running::start("exact-ids", 8, 60, 2);
    let mut one = relay.register(false, Transport::Tcp, "1", b"server-1");
    let mut zero_one = relay.register(true, Transport::Tcp, "01", b"viewer-01");
    let mut two = relay.register(false, Transport::Tcp, "2", b"server-2");
    let mut three = relay.register(false, Transport::Tcp, "3", b"server-3");
    let mut duplicate_server = relay.open(false, Transport::Tcp);
    duplicate_server.send_request("1", &[]);
    let mut duplicate_viewer = relay.open(true, Transport::Tcp);
    duplicate_viewer.send_request("01", &[]);
    for duplicate in [&mut duplicate_server, &mut duplicate_viewer] {
        duplicate.reject(4004, "duplicate_id");
        let rejected = relay.wait_event("connection_rejected", &duplicate.conn);
        assert_eq!(field(&rejected, "reason"), Some("duplicate_id"));
        assert_eq!(field(&rejected, "stage"), Some("register"));
        assert_no_registration(&relay.logs(), &duplicate.conn);
    }
    assert_eq!(
        relay
            .logs()
            .iter()
            .filter(|line| event(line, "session_started"))
            .count(),
        0
    );

    let mut viewer_one = relay.open(true, Transport::Tcp);
    viewer_one.send_request("1", b"viewer-1");
    assert_pair(&relay.started(&viewer_one.conn), &viewer_one, &one);
    assert_eq!(viewer_one.read_data(8), b"server-1");
    assert_eq!(one.read_data(8), b"viewer-1");

    let mut server_zero_one = relay.open(false, Transport::Tcp);
    server_zero_one.send_request("01", b"server-01");
    assert_pair(
        &relay.started(&server_zero_one.conn),
        &zero_one,
        &server_zero_one,
    );
    assert_eq!(zero_one.read_data(9), b"server-01");
    assert_eq!(server_zero_one.read_data(9), b"viewer-01");

    for (id, server) in [("2", &mut two), ("3", &mut three)] {
        let mut viewer = relay.open(true, Transport::Tcp);
        viewer.send_request(id, &[b'V', id.as_bytes()[0]]);
        assert_pair(&relay.started(&viewer.conn), &viewer, server);
        assert_eq!(viewer.read_data(8), format!("server-{id}").as_bytes());
        assert_eq!(server.read_data(2), [b'V', id.as_bytes()[0]]);
    }
    let lines = relay.logs();
    assert_eq!(
        lines
            .iter()
            .filter(|line| event(line, "session_started"))
            .count(),
        4
    );
    for peer in [&one, &zero_one, &two, &three] {
        assert_eq!(
            lines
                .iter()
                .filter(|line| event(line, "session_started") && session_uses(line, &peer.conn))
                .count(),
            1
        );
    }
}

/// spec: relay.md:RLY-013
#[test]
fn simultaneous_same_id_arrivals_form_one_pair_and_reject_every_other_connection() {
    let _ports = PORTS.lock().unwrap_or_else(PoisonError::into_inner);
    let mut relay = Running::start("simultaneous", 16, 60, 2);
    let mut peers = Vec::new();
    for viewer in [false, true] {
        for index in 0..4_u8 {
            peers.push((viewer, index, relay.open(viewer, Transport::Tcp)));
        }
    }
    let gate = Arc::new(Barrier::new(peers.len() + 1));
    let threads: Vec<_> = peers
        .into_iter()
        .map(|(viewer, index, mut peer)| {
            let gate = Arc::clone(&gate);
            std::thread::spawn(move || {
                gate.wait();
                peer.send_request("12345", &[]);
                (viewer, index, peer)
            })
        })
        .collect();
    gate.wait();
    let mut peers: Vec<_> = threads
        .into_iter()
        .map(|thread| {
            thread
                .join()
                .unwrap_or_else(|error| panic!(concat!("arrival writer", ": {:?}"), error))
        })
        .collect();
    let lines = relay.wait_for(|lines| {
        lines
            .iter()
            .filter(|line| event(line, "session_started"))
            .count()
            == 1
            && lines
                .iter()
                .filter(|line| {
                    event(line, "connection_rejected")
                        && field(line, "reason") == Some("duplicate_id")
                })
                .count()
                == 6
    });
    let started = lines
        .iter()
        .find(|line| event(line, "session_started"))
        .unwrap_or_else(|| panic!("single winning pair"));
    let viewer_id = field(started, "viewer_conn_id")
        .unwrap_or_else(|| panic!("winning viewer"))
        .to_owned();
    let server_id = field(started, "server_conn_id")
        .unwrap_or_else(|| panic!("winning server"))
        .to_owned();
    for (_, _, peer) in &mut peers {
        let uses = lines
            .iter()
            .filter(|line| event(line, "session_started") && session_uses(line, &peer.conn))
            .count();
        if peer.conn == viewer_id || peer.conn == server_id {
            assert_eq!(uses, 1);
        } else {
            assert_eq!(uses, 0);
            peer.reject(4004, "duplicate_id");
            assert_no_registration(&lines, &peer.conn);
            assert_eq!(
                lines
                    .iter()
                    .filter(|line| event(line, "connection_rejected")
                        && connection(line, &peer.conn)
                        && field(line, "reason") == Some("duplicate_id"))
                    .count(),
                1
            );
        }
    }
    let viewer_at = peers
        .iter()
        .position(|(_, _, peer)| peer.conn == viewer_id)
        .unwrap_or_else(|| panic!("viewer peer"));
    let server_at = peers
        .iter()
        .position(|(_, _, peer)| peer.conn == server_id)
        .unwrap_or_else(|| panic!("server peer"));
    assert!(peers[viewer_at].0);
    assert!(!peers[server_at].0);
    peers[viewer_at].2.send(b"only-viewer");
    peers[server_at].2.send(b"only-server");
    assert_eq!(peers[server_at].2.read_data(11), b"only-viewer");
    assert_eq!(peers[viewer_at].2.read_data(11), b"only-server");
    peers[viewer_at].2.shutdown();
    peers[server_at].2.shutdown();
    relay.closed(started);
    let lines = relay.logs();
    assert_eq!(
        lines
            .iter()
            .filter(|line| event(line, "session_started"))
            .count(),
        1
    );
    assert_eq!(
        lines
            .iter()
            .filter(|line| event(line, "session_closed"))
            .count(),
        1
    );
}

/// spec: relay.md:RLY-014
#[test]
fn arrival_races_disconnect_and_wait_expiry_with_exactly_one_valid_transition() {
    let _ports = PORTS.lock().unwrap_or_else(PoisonError::into_inner);
    let mut relay = Running::start("waiting-races", 8, 10, 2);
    for (expires, after_deadline) in [(false, false), (true, false), (true, true)] {
        let id = if after_deadline {
            "7203"
        } else if expires {
            "7202"
        } else {
            "7201"
        };
        let mut old = relay.open(false, Transport::Tcp);
        let earliest_registration = Instant::now();
        old.send_request(id, &[]);
        relay.wait_event("waiting_registered", &old.conn);
        let latest_registration = Instant::now();
        let mut incoming = relay.open(true, Transport::Tcp);
        if expires {
            // This is intentionally a timing test. The request is released at
            // the real W boundary, racing the executable's timer task.
            let deadline = if after_deadline {
                latest_registration
            } else {
                earliest_registration
            } + Duration::from_secs(10);
            std::thread::sleep(deadline.saturating_duration_since(Instant::now()));
            incoming.send_request(id, &[]);
        } else {
            let gate = Arc::new(Barrier::new(2));
            std::thread::scope(|scope| {
                let other = Arc::clone(&gate);
                let old = &mut old;
                scope.spawn(move || {
                    other.wait();
                    old.shutdown();
                });
                gate.wait();
                incoming.send_request(id, &[]);
            });
        }
        let lines = relay.wait_for(|lines| {
            lines.iter().any(|line| {
                (event(line, "waiting_registered") && connection(line, &incoming.conn))
                    || (event(line, "session_started") && session_uses(line, &incoming.conn))
            })
        });
        let matched = lines
            .iter()
            .find(|line| event(line, "session_started") && session_uses(line, &incoming.conn))
            .cloned();
        if after_deadline {
            assert!(
                matched.is_none(),
                "a connection already past W must not match"
            );
        }
        if let Some(started) = &matched {
            assert_pair(started, &incoming, &old);
            if expires {
                let prefix = "\"waited_ms\":";
                let start =
                    started.find(prefix).unwrap_or_else(|| panic!("waited_ms")) + prefix.len();
                let waited = started[start..]
                    .split(|c: char| !c.is_ascii_digit())
                    .next()
                    .unwrap_or_else(|| panic!("numeric waited_ms"))
                    .parse::<u64>()
                    .unwrap_or_else(|error| panic!(concat!("waited_ms integer", ": {:?}"), error));
                assert!(
                    waited < 10_000,
                    "an expired connection cannot be activated: {started}"
                );
            }
            incoming.shutdown();
            if expires {
                old.shutdown();
            }
            relay.closed(started);
        } else {
            relay.wait_for(|lines| {
                lines.iter().any(|line| {
                    (event(line, "waiting_removed") || event(line, "waiting_expired"))
                        && connection(line, &old.conn)
                })
            });
            incoming.shutdown();
            relay.wait_event("waiting_removed", &incoming.conn);
        }
        let lines = relay.logs();
        let removals = lines
            .iter()
            .filter(|line| {
                (event(line, "waiting_removed") || event(line, "waiting_expired"))
                    && connection(line, &old.conn)
            })
            .count();
        let matches = lines
            .iter()
            .filter(|line| event(line, "session_started") && session_uses(line, &old.conn))
            .count();
        assert_eq!(
            removals + matches,
            1,
            "one linearized transition for old waiter: {lines:?}"
        );
        if removals == 1 {
            assert_eq!(matches, 0, "a removed connection must never match");
        }
        // Once the raced pair is gone, neither a stale waiter nor its cleanup
        // may consume the next exact-ID connection.
        let mut replacement = relay.register(false, Transport::Tcp, id, b"new-server");
        let mut viewer = relay.open(true, Transport::Tcp);
        viewer.send_request(id, b"new-viewer");
        assert_pair(&relay.started(&viewer.conn), &viewer, &replacement);
        assert_eq!(viewer.read_data(10), b"new-server");
        assert_eq!(replacement.read_data(10), b"new-viewer");
        viewer.shutdown();
        replacement.shutdown();
        let started = relay.started(&viewer.conn);
        relay.closed(&started);
    }
}

/// spec: relay.md:RLY-015
#[test]
fn tcp_ws_and_wss_duplicates_close_without_registering_or_replacing_the_existing_pair() {
    let _ports = PORTS.lock().unwrap_or_else(PoisonError::into_inner);
    for transport in [Transport::Tcp, Transport::Ws, Transport::Wss] {
        let mut relay = Running::start(&format!("duplicates-{transport:?}"), 1, 60, 2);
        // Server-first duplicates must obey the same policy as Viewer-first
        // duplicates; roles come from the actual entry points.
        let mut first_server = relay.register(false, Transport::Tcp, "8100", b"server-first");
        let mut duplicate_server = relay.open(false, Transport::Tcp);
        duplicate_server.send_request("8100", &[]);
        duplicate_server.reject(4004, "duplicate_id");
        assert_eq!(
            field(
                &relay.wait_event("connection_rejected", &duplicate_server.conn),
                "reason"
            ),
            Some("duplicate_id")
        );
        assert_no_registration(&relay.logs(), &duplicate_server.conn);
        let mut first_viewer = relay.open(true, transport);
        first_viewer.send_request("8100", b"viewer-second");
        assert_pair(
            &relay.started(&first_viewer.conn),
            &first_viewer,
            &first_server,
        );
        assert_eq!(first_viewer.read_data(12), b"server-first");
        assert_eq!(first_server.read_data(13), b"viewer-second");
        let mut viewer = relay.register(true, transport, "8101", b"first-viewer");
        let mut duplicate = relay.open(true, transport);
        duplicate.send_request("8101", &[]);
        duplicate.reject(4004, "duplicate_id");
        let rejected = relay.wait_event("connection_rejected", &duplicate.conn);
        assert_eq!(field(&rejected, "reason"), Some("duplicate_id"));
        assert_eq!(field(&rejected, "stage"), Some("register"));
        assert_no_registration(&relay.logs(), &duplicate.conn);

        let mut server = relay.open(false, Transport::Tcp);
        server.send_request("8101", b"first-server");
        let started = relay.started(&server.conn);
        assert_pair(&started, &viewer, &server);
        assert_eq!(viewer.read_data(12), b"first-server");
        assert_eq!(server.read_data(12), b"first-viewer");

        for viewer_role in [true, false] {
            let mut duplicate = relay.open(
                viewer_role,
                if viewer_role {
                    transport
                } else {
                    Transport::Tcp
                },
            );
            duplicate.send_request("8101", &[]);
            duplicate.reject(4004, "duplicate_id");
            let rejected = relay.wait_event("connection_rejected", &duplicate.conn);
            assert_eq!(field(&rejected, "reason"), Some("duplicate_id"));
            assert_no_registration(&relay.logs(), &duplicate.conn);
        }
        // Lw=1: active-ID rejections must leave its entire waiting capacity
        // available for an unrelated ID, not quietly park a rejected socket.
        let mut unrelated = relay.register(false, Transport::Tcp, "8102", b"unrelated");
        let mut other_viewer = relay.open(true, Transport::Tcp);
        other_viewer.send_request("8102", b"other");
        assert_pair(
            &relay.started(&other_viewer.conn),
            &other_viewer,
            &unrelated,
        );
        assert_eq!(other_viewer.read_data(9), b"unrelated");
        assert_eq!(unrelated.read_data(5), b"other");
        viewer.send(b"still-original-viewer");
        server.send(b"still-original-server");
        assert_eq!(server.read_data(21), b"still-original-viewer");
        assert_eq!(viewer.read_data(21), b"still-original-server");
        let lines = relay.logs();
        assert_eq!(
            lines
                .iter()
                .filter(|line| event(line, "session_started") && session_uses(line, &viewer.conn))
                .count(),
            1
        );
        assert!(!lines.iter().any(|line| event(line, "session_closed")
            && field(line, "session_id") == field(&started, "session_id")));
    }
}

/// spec: relay.md:RLY-016 partial
#[test]
fn reconnect_gets_a_new_session_id_after_a_deliberately_delayed_close_exchange() {
    let _ports = PORTS.lock().unwrap_or_else(PoisonError::into_inner);
    let mut relay = Running::start("delayed-reconnect", 4, 60, 10);
    let mut old_viewer = relay.register(true, Transport::Wss, "9101", b"old-viewer");
    let mut old_server = relay.open(false, Transport::Tcp);
    old_server.send_request("9101", b"old-server");
    let old_started = relay.started(&old_server.conn);
    assert_pair(&old_started, &old_viewer, &old_server);
    assert_eq!(old_viewer.read_data(10), b"old-server");
    assert_eq!(old_server.read_data(10), b"old-viewer");
    let Socket::Tcp(stream) = &old_server.socket else {
        panic!("TCP server");
    };
    stream.shutdown(Shutdown::Write).unwrap_or_else(|error| {
        panic!(
            concat!("start previous session EOF cleanup", ": {:?}"),
            error
        )
    });
    let Socket::WebSocket(ws) = &mut old_viewer.socket else {
        panic!("WSS viewer");
    };
    let (opcode, close) = ws.frame();
    assert_eq!(opcode, 8);
    assert_eq!(close, 1000_u16.to_be_bytes());
    // Intentionally withhold Close acknowledgement. The prior cleanup really
    // is pending here; do not replace this condition with an ordinary reconnect.
    assert!(
        !relay.logs().iter().any(|line| event(line, "session_closed")
            && field(line, "session_id") == field(&old_started, "session_id"))
    );
    let mut premature = relay.open(false, Transport::Tcp);
    premature.send_request("9101", &[]);
    premature.reject(4004, "duplicate_id");
    assert_eq!(
        field(
            &relay.wait_event("connection_rejected", &premature.conn),
            "reason"
        ),
        Some("duplicate_id")
    );
    assert_no_registration(&relay.logs(), &premature.conn);
    let Socket::WebSocket(ws) = &mut old_viewer.socket else {
        panic!("WSS viewer");
    };
    ws.send(8, &close);
    old_server.shutdown();
    relay.closed(&old_started);

    let mut server = relay.register(false, Transport::Tcp, "9101", b"new-server");
    let mut viewer = relay.open(true, Transport::Wss);
    viewer.send_request("9101", b"new-viewer");
    let new_started = relay.started(&viewer.conn);
    assert_pair(&new_started, &viewer, &server);
    assert_ne!(
        field(&old_started, "session_id"),
        field(&new_started, "session_id")
    );
    assert_ne!(old_server.conn, server.conn);
    assert_ne!(old_viewer.conn, viewer.conn);
    assert_eq!(viewer.read_data(10), b"new-server");
    assert_eq!(server.read_data(10), b"new-viewer");
    // Releasing all remaining old client resources cannot disturb the new pair.
    drop(old_viewer);
    drop(old_server);
    viewer.send(b"new-state");
    server.send(b"new-state");
    assert_eq!(viewer.read_data(9), b"new-state");
    assert_eq!(server.read_data(9), b"new-state");
    // This black-box case alone is partial: ID occupancy ends only when active
    // cleanup completes. The full real-socket test in the adapters integration
    // target relay_delayed_cleanup_scenarios separately parks a retired waiting
    // owner across ID reuse, then resumes it.
}

/// spec: operations.md:OPS-005
#[test]
fn waiting_limit_rejects_overflow_and_releases_capacity_after_waiter_disconnect() {
    let _ports = PORTS.lock().unwrap_or_else(PoisonError::into_inner);
    let mut relay = Running::start("waiting-limit", 1, 60, 2);
    let mut first = relay.register(false, Transport::Tcp, "10101", &[]);
    let mut overflow = relay.open(false, Transport::Tcp);
    overflow.send_request("10102", &[]);
    overflow.reject(4010, "waiting_full");
    let rejected = relay.wait_event("connection_rejected", &overflow.conn);
    assert_eq!(field(&rejected, "stage"), Some("register"));
    assert_eq!(field(&rejected, "reason"), Some("waiting_full"));
    assert!(
        rejected.contains("\"limit\":1"),
        "configured Lw in rejection: {rejected}"
    );
    assert_no_registration(&relay.logs(), &overflow.conn);
    assert!(
        !relay
            .logs()
            .iter()
            .any(|line| event(line, "session_started"))
    );
    first.shutdown();
    let removed = relay.wait_event("waiting_removed", &first.conn);
    assert_eq!(field(&removed, "reason"), Some("peer_eof"));

    let mut replacement = relay.register(false, Transport::Tcp, "10102", b"capacity-returned");
    let mut viewer = relay.open(true, Transport::Tcp);
    viewer.send_request("10102", b"match-while-full");
    let started = relay.started(&viewer.conn);
    assert_pair(&started, &viewer, &replacement);
    assert_eq!(viewer.read_data(17), b"capacity-returned");
    assert_eq!(replacement.read_data(16), b"match-while-full");
    // Matching also relinquishes Lw, independently of L's active sockets.
    let mut next = relay.register(false, Transport::Tcp, "10103", &[]);
    next.shutdown();
    relay.wait_event("waiting_removed", &next.conn);
}
