//! Process-level initial exchange and source-boundary acceptance scenarios.

use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

const LIMIT: Duration = Duration::from_secs(15);
static SERIAL: AtomicU64 = AtomicU64::new(0);
// Process listeners must release ephemeral reservations before spawn. Serialize
// these fixtures, as tests/serve.rs does, so they cannot steal each other's ports.
static PORTS: Mutex<()> = Mutex::new(());

fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\":\"");
    let rest = line.split_once(&needle)?.1;
    Some(rest.split_once('"')?.0)
}

fn request(value: &[u8]) -> [u8; 250] {
    let mut bytes = [0; 250];
    bytes[..value.len()].copy_from_slice(value);
    bytes
}

struct Running {
    child: Child,
    dir: PathBuf,
    management: u16,
    status: u16,
    viewer: u16,
    server: u16,
}

impl Running {
    fn start(name: &str, extra: &str, places: u32) -> Self {
        Self::start_with_wait(name, extra, places, 10)
    }

    fn start_with_wait(name: &str, extra: &str, places: u32, wait_secs: u64) -> Self {
        let holders: Vec<_> = (0..4)
            .map(|_| {
                TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| {
                    panic!(concat!("reserve ephemeral port", ": {:?}"), error)
                })
            })
            .collect();
        let ports: Vec<_> = holders
            .iter()
            .map(|listener| {
                listener
                    .local_addr()
                    .unwrap_or_else(|error| panic!(concat!("reserved address", ": {:?}"), error))
                    .port()
            })
            .collect();
        let dir = std::env::temp_dir().join(format!(
            "vnc-relay-entry-{}-{}-{name}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir)
            .unwrap_or_else(|error| panic!(concat!("scenario directory", ": {:?}"), error));
        let accounts = dir.join("accounts.toml");
        std::fs::write(
            &accounts,
            "version = 1\n[[account]]\nname = \"alice\"\npassword_hash = \"$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\"\n",
        ).unwrap_or_else(|error| panic!(concat!("test account", ": {:?}"), error));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&accounts, std::fs::Permissions::from_mode(0o600))
                .unwrap_or_else(|error| panic!(concat!("private test account", ": {:?}"), error));
        }
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
[limits]
relay_connections = {places}
waiting = {places}
[timeouts]
handshake_secs = 1
wait_secs = {wait_secs}
connect_secs = 1
close_secs = 1
[log]
output = "events.jsonl"
{extra}
"#,
            ports[0], ports[1], ports[2], ports[3]
        );
        let path = dir.join("relay.toml");
        std::fs::write(&path, config)
            .unwrap_or_else(|error| panic!(concat!("scenario config", ": {:?}"), error));
        let stderr = std::fs::File::create(dir.join("stderr"))
            .unwrap_or_else(|error| panic!(concat!("stderr file", ": {:?}"), error));
        drop(holders);
        let child = Command::new(env!("CARGO_BIN_EXE_vnc-relay"))
            .args(["serve", "--config"])
            .arg(path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(stderr)
            .spawn()
            .unwrap_or_else(|error| panic!(concat!("start real relay binary", ": {:?}"), error));
        let mut running = Self {
            child,
            dir,
            management: ports[0],
            status: ports[1],
            viewer: ports[2],
            server: ports[3],
        };
        let deadline = Instant::now() + LIMIT;
        loop {
            if running
                .lines()
                .iter()
                .any(|line| field(line, "event") == Some("server_ready"))
            {
                return running;
            }
            let status = running
                .child
                .try_wait()
                .unwrap_or_else(|error| panic!(concat!("process status", ": {:?}"), error));
            assert!(
                status.is_none() && Instant::now() < deadline,
                "relay did not become ready: {status:?}; {}",
                std::fs::read_to_string(running.dir.join("stderr")).unwrap_or_default()
            );
            std::thread::yield_now();
        }
    }

    fn lines(&self) -> Vec<String> {
        std::fs::read_to_string(self.dir.join("events.jsonl"))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    fn wait(&self, after: usize, predicate: impl Fn(&str) -> bool) -> String {
        let deadline = Instant::now() + LIMIT;
        loop {
            let lines = self.lines();
            if let Some(line) = lines.iter().skip(after).find(|line| predicate(line)) {
                return line.clone();
            }
            assert!(Instant::now() < deadline, "missing event; {lines:#?}");
            std::thread::yield_now();
        }
    }

    fn connect(&self, viewer: bool) -> (TcpStream, String) {
        let after = self.lines().len();
        let mut stream =
            TcpStream::connect(("127.0.0.1", if viewer { self.viewer } else { self.server }))
                .unwrap_or_else(|error| panic!(concat!("connect allowed peer", ": {:?}"), error));
        stream
            .set_read_timeout(Some(LIMIT))
            .unwrap_or_else(|error| panic!(concat!("read bound", ": {:?}"), error));
        stream
            .set_write_timeout(Some(LIMIT))
            .unwrap_or_else(|error| panic!(concat!("write bound", ": {:?}"), error));
        stream
            .set_nodelay(true)
            .unwrap_or_else(|error| panic!(concat!("disable Nagle", ": {:?}"), error));
        if viewer {
            let mut greeting = [0; 12];
            stream
                .read_exact(&mut greeting)
                .unwrap_or_else(|error| panic!(concat!("Viewer greeting", ": {:?}"), error));
            assert_eq!(&greeting, b"RFB 000.000\n");
        }
        let accepted = self.wait(after, |line| {
            field(line, "event") == Some("connection_accepted")
                && field(line, "role") == Some(if viewer { "viewer" } else { "server" })
        });
        let id = field(&accepted, "conn_id")
            .unwrap_or_else(|| panic!("accepted connection ID"))
            .to_owned();
        (stream, id)
    }

    fn for_connection(&self, id: &str, event: &str) -> String {
        self.wait(0, |line| {
            field(line, "event") == Some(event) && field(line, "conn_id") == Some(id)
        })
    }

    fn pair(&self, id: &[u8]) -> (TcpStream, TcpStream, String) {
        let after = self.lines().len();
        let (mut server, server_id) = self.connect(false);
        server
            .write_all(&request(id))
            .unwrap_or_else(|error| panic!(concat!("register Server", ": {:?}"), error));
        self.for_connection(&server_id, "waiting_registered");
        let (mut viewer, viewer_id) = self.connect(true);
        viewer
            .write_all(&request(id))
            .unwrap_or_else(|error| panic!(concat!("match Viewer", ": {:?}"), error));
        let started = self.wait(after, |line| {
            field(line, "event") == Some("session_started")
                && field(line, "viewer_conn_id") == Some(&viewer_id)
        });
        (
            viewer,
            server,
            field(&started, "session_id")
                .unwrap_or_else(|| panic!("session ID"))
                .to_owned(),
        )
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn assert_fin(stream: &mut TcpStream) {
    let mut byte = [0];
    assert_eq!(
        stream
            .read(&mut byte)
            .unwrap_or_else(|error| panic!("clean TCP FIN without error bytes: {error:?}")),
        0
    );
}

fn assert_quiet(stream: &mut TcpStream) {
    stream
        .set_read_timeout(Some(Duration::from_millis(40)))
        .unwrap_or_else(|error| panic!(concat!("observation bound", ": {:?}"), error));
    let mut byte = [0];
    let error = stream
        .read(&mut byte)
        .err()
        .unwrap_or_else(|| panic!("no premature or duplicate bytes"));
    assert!(matches!(
        error.kind(),
        ErrorKind::WouldBlock | ErrorKind::TimedOut
    ));
    stream
        .set_read_timeout(Some(LIMIT))
        .unwrap_or_else(|error| panic!(concat!("restore read bound", ": {:?}"), error));
}

fn assert_no_registration(run: &Running, id: &str) {
    for line in run.lines() {
        assert!(
            !(field(&line, "event") == Some("waiting_registered")
                && field(&line, "conn_id") == Some(id)),
            "{line}"
        );
        assert!(
            !(field(&line, "event") == Some("session_started")
                && (field(&line, "viewer_conn_id") == Some(id)
                    || field(&line, "server_conn_id") == Some(id))),
            "{line}"
        );
    }
}

/// spec: relay.md:RLY-001
/// spec: relay.md:RLY-002
/// spec: relay.md:RLY-003
#[test]
fn accepts_both_tcp_roles_and_preserves_split_and_merged_requests_with_prefixes_once() {
    let _ports = PORTS.lock().unwrap_or_else(PoisonError::into_inner);
    let run = Running::start(
        "boundaries",
        "[mode1]\nenabled = false\n[mode2]\nenabled = true",
        8,
    );
    for split in [false, true] {
        let id = "ID:7100";
        let (mut server, server_id) = run.connect(false);
        let mut server_message = request(id.as_bytes()).to_vec();
        server_message.extend_from_slice(b"server-prefix");
        if split {
            for chunk in server_message[..249].chunks(83) {
                server.write_all(chunk).unwrap_or_else(|error| {
                    panic!(concat!("split Server request", ": {:?}"), error)
                });
            }
            // An incomplete exchange cannot register; the final byte is still withheld.
            assert!(
                !run.lines()
                    .iter()
                    .any(|line| field(line, "event") == Some("waiting_registered")
                        && field(line, "conn_id") == Some(&server_id))
            );
            server
                .write_all(&server_message[249..])
                .unwrap_or_else(|error| panic!(concat!("finish Server request", ": {:?}"), error));
        } else {
            server.write_all(&server_message).unwrap_or_else(|error| {
                panic!(concat!("merged Server request and prefix", ": {:?}"), error)
            });
        }
        run.for_connection(&server_id, "waiting_registered");
        let (mut viewer, viewer_id) = run.connect(true);
        let mut viewer_message = request(id.as_bytes()).to_vec();
        viewer_message.extend_from_slice(b"viewer-prefix");
        if split {
            for chunk in viewer_message[..249].chunks(83) {
                viewer.write_all(chunk).unwrap_or_else(|error| {
                    panic!(concat!("split Viewer request", ": {:?}"), error)
                });
                assert_quiet(&mut server);
            }
            viewer
                .write_all(&viewer_message[249..])
                .unwrap_or_else(|error| panic!(concat!("finish Viewer request", ": {:?}"), error));
        } else {
            viewer.write_all(&viewer_message).unwrap_or_else(|error| {
                panic!(concat!("merged Viewer request and prefix", ": {:?}"), error)
            });
        }
        let started = run.wait(0, |line| {
            field(line, "event") == Some("session_started")
                && field(line, "viewer_conn_id") == Some(&viewer_id)
        });
        assert_eq!(field(&started, "server_conn_id"), Some(server_id.as_str()));
        viewer
            .write_all(b"-viewer-after")
            .unwrap_or_else(|error| panic!(concat!("later Viewer bytes", ": {:?}"), error));
        server
            .write_all(b"-server-after")
            .unwrap_or_else(|error| panic!(concat!("later Server bytes", ": {:?}"), error));
        let mut from_viewer = [0; 26];
        let mut from_server = [0; 26];
        server
            .read_exact(&mut from_viewer)
            .unwrap_or_else(|error| panic!(concat!("ordered Viewer stream", ": {:?}"), error));
        viewer
            .read_exact(&mut from_server)
            .unwrap_or_else(|error| panic!(concat!("ordered Server stream", ": {:?}"), error));
        assert_eq!(&from_viewer, b"viewer-prefix-viewer-after");
        assert_eq!(&from_server, b"server-prefix-server-after");
        viewer
            .shutdown(Shutdown::Write)
            .unwrap_or_else(|error| panic!(concat!("Viewer EOF", ": {:?}"), error));
        server
            .shutdown(Shutdown::Write)
            .unwrap_or_else(|error| panic!(concat!("Server EOF", ": {:?}"), error));
        assert_fin(&mut server);
        assert_fin(&mut viewer);
        let session = field(&started, "session_id").unwrap_or_else(|| panic!("session ID"));
        run.wait(0, |line| {
            field(line, "event") == Some("session_closed")
                && field(line, "session_id") == Some(session)
        });
    }
}

/// spec: relay.md:RLY-004
#[test]
fn rejects_every_malformed_initial_value_without_registration_or_target_socket() {
    let _ports = PORTS.lock().unwrap_or_else(PoisonError::into_inner);
    let target = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|error| panic!(concat!("target witness", ": {:?}"), error));
    target
        .set_nonblocking(true)
        .unwrap_or_else(|error| panic!(concat!("nonblocking witness", ": {:?}"), error));
    let run = Running::start(
        "malformed",
        &format!(
            "[mode1]\nenabled = true\nallow_targets = [\"127.0.0.1:{}\"]\n[mode2]\nenabled = true",
            target
                .local_addr()
                .unwrap_or_else(|error| panic!("target address: {error:?}"))
                .port()
        ),
        2,
    );
    for value in [b"".as_slice(), b"ID:not-a-number", b"127.0.0.1:not-a-port"] {
        let (mut stream, id) = run.connect(true);
        stream
            .write_all(&request(value))
            .unwrap_or_else(|error| panic!(concat!("malformed request", ": {:?}"), error));
        assert_fin(&mut stream);
        let rejected = run.for_connection(&id, "connection_rejected");
        assert_eq!(field(&rejected, "stage"), Some("initial_exchange"));
        assert_eq!(field(&rejected, "reason"), Some("handshake_invalid"));
        assert_no_registration(&run, &id);
        assert_eq!(
            target
                .accept()
                .err()
                .unwrap_or_else(|| panic!("no target connection"))
                .kind(),
            ErrorKind::WouldBlock
        );
    }
}

/// spec: relay.md:RLY-005
#[test]
fn incomplete_disconnect_and_h_timeout_have_distinct_causes_and_return_l_for_each_role() {
    let _ports = PORTS.lock().unwrap_or_else(PoisonError::into_inner);
    let run = Running::start(
        "incomplete",
        "[mode1]\nenabled = false\n[mode2]\nenabled = true",
        3,
    );
    // An active pair has no W deadline and permanently occupies two L places.
    let (mut witness_viewer, mut witness_server, witness_session) = run.pair(b"ID:7299");
    let mut witness_is_active = || {
        witness_viewer
            .write_all(b"v")
            .unwrap_or_else(|error| panic!("active witness Viewer write: {error:?}"));
        let mut byte = [0];
        witness_server
            .read_exact(&mut byte)
            .unwrap_or_else(|error| panic!("active witness Server read: {error:?}"));
        assert_eq!(&byte, b"v");
        witness_server
            .write_all(b"s")
            .unwrap_or_else(|error| panic!("active witness Server write: {error:?}"));
        witness_viewer
            .read_exact(&mut byte)
            .unwrap_or_else(|error| panic!("active witness Viewer read: {error:?}"));
        assert_eq!(&byte, b"s");
        assert!(
            !run.lines().iter().any(|line| {
                field(line, "event") == Some("session_closed")
                    && field(line, "session_id") == Some(&witness_session)
            }),
            "the L-slot witness must remain active"
        );
    };
    for viewer in [false, true] {
        for timeout in [false, true] {
            let started = Instant::now();
            let (mut stream, id) = run.connect(viewer);
            stream
                .write_all(b"ID:incomplete")
                .unwrap_or_else(|error| panic!(concat!("partial request", ": {:?}"), error));
            if timeout {
                assert_fin(&mut stream);
                assert!(
                    started.elapsed() >= Duration::from_secs(1),
                    "H must not expire before one second from connection establishment"
                );
            } else {
                stream.shutdown(Shutdown::Both).unwrap_or_else(|error| {
                    panic!(concat!("incomplete disconnect", ": {:?}"), error)
                });
                drop(stream);
            }
            let aborted = run.for_connection(&id, "connection_aborted");
            assert_eq!(field(&aborted, "stage"), Some("initial_exchange"));
            assert_eq!(
                field(&aborted, "reason"),
                Some(if timeout {
                    "handshake_timeout"
                } else {
                    "peer_eof"
                })
            );
            assert!(
                started.elapsed() < LIMIT,
                "initial exchange exceeded its overall bound"
            );
            witness_is_active();
            assert_no_registration(&run, &id);
            // L=3 with a live two-socket session: this third place must have been returned.
            let (mut replacement, replacement_id) = run.connect(false);
            replacement
                .write_all(&request(b"ID:7200"))
                .unwrap_or_else(|error| {
                    panic!(concat!("replacement registration", ": {:?}"), error)
                });
            run.for_connection(&replacement_id, "waiting_registered");
            witness_is_active();
            replacement
                .shutdown(Shutdown::Both)
                .unwrap_or_else(|error| panic!(concat!("release replacement", ": {:?}"), error));
            drop(replacement);
            run.for_connection(&replacement_id, "waiting_removed");
        }
    }
}

/// spec: relay.md:RLY-022
#[test]
fn disabled_modes_reject_before_waiting_registration_or_target_connect() {
    let _ports = PORTS.lock().unwrap_or_else(PoisonError::into_inner);
    let target = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|error| panic!(concat!("target witness", ": {:?}"), error));
    target
        .set_nonblocking(true)
        .unwrap_or_else(|error| panic!(concat!("nonblocking witness", ": {:?}"), error));
    let target_port = target
        .local_addr()
        .unwrap_or_else(|error| panic!(concat!("target address", ": {:?}"), error))
        .port();
    for mode1 in [false, true] {
        let run = Running::start(
            "disabled",
            &format!(
                "[mode1]\nenabled = {mode1}\nallow_targets = [\"127.0.0.1:{target_port}\"]\n[mode2]\nenabled = {}",
                !mode1
            ),
            2,
        );
        for viewer in if mode1 { vec![false, true] } else { vec![true] } {
            let (mut stream, id) = run.connect(viewer);
            let value = if mode1 {
                "ID:7300".to_owned()
            } else {
                format!("127.0.0.1:{target_port}")
            };
            stream
                .write_all(&request(value.as_bytes()))
                .unwrap_or_else(|error| {
                    panic!(concat!("valid disabled-mode request", ": {:?}"), error)
                });
            assert_fin(&mut stream);
            let rejected = run.for_connection(&id, "connection_rejected");
            assert_eq!(field(&rejected, "stage"), Some("initial_exchange"));
            assert_eq!(field(&rejected, "reason"), Some("mode_disabled"));
            assert_eq!(
                field(&rejected, "mode"),
                Some(if mode1 { "mode2" } else { "mode1" })
            );
            assert_no_registration(&run, &id);
            assert_eq!(
                target
                    .accept()
                    .err()
                    .unwrap_or_else(|| panic!("no target socket"))
                    .kind(),
                ErrorKind::WouldBlock
            );
        }
    }
}

fn from_denied_source(port: u16) -> TcpStream {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|error| panic!(concat!("client runtime", ": {:?}"), error));
    runtime.block_on(async {
        let socket = tokio::net::TcpSocket::new_v4()
            .unwrap_or_else(|error| panic!(concat!("client socket", ": {:?}"), error));
        socket
            .bind(SocketAddr::from(([127, 0, 0, 2], 0)))
            .unwrap_or_else(|error| panic!(concat!("denied source bind", ": {:?}"), error));
        let stream = tokio::time::timeout(
            LIMIT,
            socket.connect(SocketAddr::from(([127, 0, 0, 1], port))),
        )
        .await
        .unwrap_or_else(|error| panic!("connect bound: {error:?}"))
        .unwrap_or_else(|error| panic!(concat!("denied connect", ": {:?}"), error));
        let stream = stream
            .into_std()
            .unwrap_or_else(|error| panic!(concat!("std socket", ": {:?}"), error));
        stream
            .set_nonblocking(false)
            .unwrap_or_else(|error| panic!(concat!("blocking client", ": {:?}"), error));
        stream
            .set_read_timeout(Some(LIMIT))
            .unwrap_or_else(|error| panic!(concat!("read bound", ": {:?}"), error));
        stream
            .set_write_timeout(Some(LIMIT))
            .unwrap_or_else(|error| panic!(concat!("write bound", ": {:?}"), error));
        stream
    })
}

/// spec: relay.md:RLY-023
#[test]
fn rejects_forged_allowed_ip_before_exchange_registration_or_target_and_keeps_session() {
    let _ports = PORTS.lock().unwrap_or_else(PoisonError::into_inner);
    let target = TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|error| panic!(concat!("target witness", ": {:?}"), error));
    target
        .set_nonblocking(true)
        .unwrap_or_else(|error| panic!(concat!("nonblocking witness", ": {:?}"), error));
    let target_port = target
        .local_addr()
        .unwrap_or_else(|error| panic!(concat!("target address", ": {:?}"), error))
        .port();
    let run = Running::start(
        "source",
        &format!(
            "[mode1]\nenabled = true\nallow_targets = [\"127.0.0.1:{target_port}\"]\n[mode2]\nenabled = true"
        ),
        2,
    );
    let (mut viewer, mut server, session) = run.pair(b"ID:7400");
    for (port, listener) in [(run.viewer, "tcp_viewer"), (run.server, "tcp_server")] {
        for value in ["ID:7401".to_owned(), format!("127.0.0.1:{target_port}")] {
            let after = run.lines().len();
            let mut denied = from_denied_source(port);
            let mut claim = request(value.as_bytes()).to_vec();
            claim.extend_from_slice(b"X-Forwarded-For: 127.0.0.1\r\n");
            // Rejection can win the write race; it must never emit a Viewer greeting.
            let _ = denied.write_all(&claim);
            let mut byte = [0];
            match denied.read(&mut byte) {
                Ok(0) => {}
                Ok(_) => panic!("denied peer received an initial-exchange or error byte"),
                Err(error) => assert!(
                    matches!(
                        error.kind(),
                        ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted
                    ),
                    "denied peer must close within the socket deadline: {error:?}"
                ),
            }
            let rejected = run.wait(after, |line| {
                field(line, "event") == Some("connection_rejected")
                    && field(line, "listener") == Some(listener)
            });
            assert_eq!(field(&rejected, "reason"), Some("ip_not_allowed"));
            assert_eq!(field(&rejected, "stage"), Some("accept"));
            assert_eq!(field(&rejected, "conn_id"), None);
            assert_eq!(
                target
                    .accept()
                    .err()
                    .unwrap_or_else(|| panic!("no forbidden target connection"))
                    .kind(),
                ErrorKind::WouldBlock
            );
            for line in run.lines().iter().skip(after) {
                assert_ne!(field(line, "event"), Some("connection_accepted"), "{line}");
                assert_ne!(field(line, "event"), Some("waiting_registered"), "{line}");
                assert_ne!(field(line, "event"), Some("session_started"), "{line}");
            }
            viewer
                .write_all(b"kept-viewer")
                .unwrap_or_else(|error| panic!(concat!("original Viewer lives", ": {:?}"), error));
            let mut forward = [0; 11];
            server
                .read_exact(&mut forward)
                .unwrap_or_else(|error| panic!(concat!("original Server lives", ": {:?}"), error));
            assert_eq!(&forward, b"kept-viewer");
            server
                .write_all(b"kept-server")
                .unwrap_or_else(|error| panic!(concat!("original Server writes", ": {:?}"), error));
            viewer
                .read_exact(&mut forward)
                .unwrap_or_else(|error| panic!(concat!("original Viewer reads", ": {:?}"), error));
            assert_eq!(&forward, b"kept-server");
            assert!(
                !run.lines()
                    .iter()
                    .any(|line| field(line, "event") == Some("session_closed")
                        && field(line, "session_id") == Some(&session))
            );
        }
    }
}

fn assert_round_trip(viewer: &mut TcpStream, server: &mut TcpStream) {
    viewer
        .write_all(b"viewer-still-live")
        .unwrap_or_else(|error| panic!("Viewer writes after competing startup: {error:?}"));
    let mut bytes = [0; 17];
    server
        .read_exact(&mut bytes)
        .unwrap_or_else(|error| panic!("Server receives after competing startup: {error:?}"));
    assert_eq!(&bytes, b"viewer-still-live");
    server
        .write_all(b"server-still-live")
        .unwrap_or_else(|error| panic!("Server writes after competing startup: {error:?}"));
    viewer
        .read_exact(&mut bytes)
        .unwrap_or_else(|error| panic!("Viewer receives after competing startup: {error:?}"));
    assert_eq!(&bytes, b"server-still-live");
}

fn assert_http_serves(port: u16, path: &str, status: &str) {
    let mut stream = TcpStream::connect_timeout(&([127, 0, 0, 1], port).into(), LIMIT)
        .unwrap_or_else(|error| panic!("first instance HTTP listener accepts: {error:?}"));
    stream
        .set_read_timeout(Some(LIMIT))
        .unwrap_or_else(|error| panic!("HTTP read bound: {error:?}"));
    stream
        .set_write_timeout(Some(LIMIT))
        .unwrap_or_else(|error| panic!("HTTP write bound: {error:?}"));
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
    )
    .unwrap_or_else(|error| panic!("first instance HTTP request: {error:?}"));
    let deadline = Instant::now() + LIMIT;
    let mut headers = Vec::new();
    while !headers.ends_with(b"\r\n\r\n") {
        assert!(
            headers.len() < 16_384 && Instant::now() < deadline,
            "bounded HTTP response head"
        );
        let mut byte = [0];
        stream
            .read_exact(&mut byte)
            .unwrap_or_else(|error| panic!("first instance HTTP response: {error:?}"));
        headers.push(byte[0]);
    }
    let headers = String::from_utf8(headers)
        .unwrap_or_else(|error| panic!("HTTP response head text: {error:?}"));
    assert!(headers.starts_with(status), "{headers}");
}

struct CompetingProcess(Child);

impl Drop for CompetingProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// spec: operations.md:OPS-039
#[test]
fn second_instance_releases_resources_without_disturbing_waiting_or_active_sessions() {
    let _ports = PORTS.lock().unwrap_or_else(PoisonError::into_inner);
    let scenario_deadline = Instant::now() + Duration::from_secs(60);
    let mut run = Running::start_with_wait(
        "competing-instance",
        "[mode1]\nenabled = false\n[mode2]\nenabled = true",
        8,
        120,
    );
    let (mut waiting_server, waiting_id) = run.connect(false);
    waiting_server
        .write_all(&request(b"ID:7500"))
        .unwrap_or_else(|error| panic!("first instance waiting registration: {error:?}"));
    run.for_connection(&waiting_id, "waiting_registered");
    let (mut active_viewer, mut active_server, active_session) = run.pair(b"ID:7501");
    assert_round_trip(&mut active_viewer, &mut active_server);

    let before_second = run.lines().len();
    let stderr_path = run.dir.join("second-stderr");
    let stdout_path = run.dir.join("second-stdout");
    let stderr = std::fs::File::create(&stderr_path)
        .unwrap_or_else(|error| panic!("second process stderr file: {error:?}"));
    let stdout = std::fs::File::create(&stdout_path)
        .unwrap_or_else(|error| panic!("second process stdout file: {error:?}"));
    let mut second = CompetingProcess(
        Command::new(env!("CARGO_BIN_EXE_vnc-relay"))
            .args(["serve", "--config"])
            .arg(run.dir.join("relay.toml"))
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr)
            .spawn()
            .unwrap_or_else(|error| panic!("start competing instance: {error:?}")),
    );
    let exit_deadline = Instant::now() + LIMIT;
    let status = loop {
        if let Some(status) = second
            .0
            .try_wait()
            .unwrap_or_else(|error| panic!("inspect competing instance: {error:?}"))
        {
            break status;
        }
        assert!(
            Instant::now() < exit_deadline,
            "second process must exit and release its resources"
        );
        std::thread::yield_now();
    };
    assert_eq!(status.code(), Some(4));
    let stderr = std::fs::read_to_string(stderr_path)
        .unwrap_or_else(|error| panic!("read second process diagnostics: {error:?}"));
    let diagnostics: Vec<_> = stderr.lines().collect();
    let expected = [
        ("status", "status.listen"),
        ("management", "management.listen"),
        ("tcp_viewer", "relay.tcp_viewer.listen"),
        ("tcp_server", "relay.tcp_server.listen"),
    ];
    assert_eq!(diagnostics.len(), expected.len(), "{stderr}");
    for (line, (_, key)) in diagnostics.iter().zip(expected) {
        assert!(
            line.starts_with(&format!("error[listener_bind_failed] key={key}: ")),
            "{line}"
        );
    }
    assert_eq!(
        std::fs::read_to_string(stdout_path)
            .unwrap_or_else(|error| panic!("read second process stdout: {error:?}")),
        ""
    );
    let second_logs: Vec<_> = run
        .lines()
        .into_iter()
        .skip(before_second)
        .filter(|line| {
            matches!(
                field(line, "event"),
                Some("server_starting" | "startup_failed" | "server_ready")
            )
        })
        .collect();
    assert_eq!(second_logs.len(), 5, "{second_logs:?}");
    assert_eq!(field(&second_logs[0], "event"), Some("server_starting"));
    for (line, (listener, key)) in second_logs.iter().skip(1).zip(expected) {
        assert_eq!(field(line, "event"), Some("startup_failed"));
        assert_eq!(field(line, "reason"), Some("listener_bind_failed"));
        assert_eq!(field(line, "listener"), Some(listener));
        assert_eq!(field(line, "config_key"), Some(key));
    }
    assert!(
        run.child
            .try_wait()
            .unwrap_or_else(|error| panic!("first process status: {error:?}"))
            .is_none()
    );
    assert_round_trip(&mut active_viewer, &mut active_server);
    assert!(
        !run.lines()
            .iter()
            .any(|line| field(line, "event") == Some("session_closed")
                && field(line, "session_id") == Some(&active_session))
    );

    assert!(
        Instant::now() < scenario_deadline,
        "waiting witness stays inside W=120s"
    );
    let (mut waiting_viewer, viewer_id) = run.connect(true);
    waiting_viewer
        .write_all(&request(b"ID:7500"))
        .unwrap_or_else(|error| panic!("match original waiting connection: {error:?}"));
    let matched = run.wait(before_second, |line| {
        field(line, "event") == Some("session_started")
            && field(line, "viewer_conn_id") == Some(&viewer_id)
    });
    assert_eq!(field(&matched, "server_conn_id"), Some(waiting_id.as_str()));
    assert_round_trip(&mut waiting_viewer, &mut waiting_server);

    // Process exit plus successful service on every first-instance port is
    // the observable resource-release witness; no second owner survives.
    assert_http_serves(run.status, "/livez", "HTTP/1.1 200 ");
    assert_http_serves(run.management, "/api/v1/session", "HTTP/1.1 401 ");
    let (mut fresh_viewer, mut fresh_server, _) = run.pair(b"ID:7502");
    assert_round_trip(&mut fresh_viewer, &mut fresh_server);
    assert!(
        Instant::now() < scenario_deadline,
        "bounded competing-process scenario"
    );
}
