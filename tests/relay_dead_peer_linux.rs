//! Real TCP keepalive failures inside an unprivileged Linux network namespace.
#![cfg(target_os = "linux")]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const BIN: &str = env!("CARGO_BIN_EXE_vnc-relay");
const CHILD_TEST: &str = "VNC_RELAY_DEAD_PEER_NETNS_TEST";
const K: Duration = Duration::from_secs(11); // idle 10 + interval 1 × probes 1.
// Linux TCP timer granularity, process scheduling, and 10 ms log polling only.
const TIMER_MARGIN: Duration = Duration::from_secs(5);
const IO_LIMIT: Duration = Duration::from_secs(5);
static SERIAL: AtomicU64 = AtomicU64::new(0);

fn checked_output(program: &str, args: &[&str]) -> Output {
    let output = Command::new(program).args(args).output().unwrap_or_else(|error| {
        panic!("required Linux tool `{program}` is missing or cannot run: {error}; see docs/how-to/set-up-and-verify.md §3 (Q93)")
    });
    assert!(
        output.status.success(),
        "required Linux tool `{program}` failed ({:?}): {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn in_namespace(name: &str, run: impl FnOnce()) {
    if std::env::var(CHILD_TEST).as_deref() == Ok(name) {
        checked_output("ip", &["link", "set", "lo", "up"]);
        run();
        return;
    }
    // Check before unshare so a missing binary is named, not hidden by the child.
    checked_output("unshare", &["--version"]);
    checked_output("ip", &["-Version"]);
    checked_output("nft", &["--version"]);
    let executable = std::env::current_exe()
        .unwrap_or_else(|error| panic!("locate current test executable: {error}"));
    let output = Command::new("unshare")
        .args(["--user", "--map-root-user", "--net"])
        .arg(executable)
        .args(["--exact", name, "--nocapture"])
        .env(CHILD_TEST, name)
        .output()
        .unwrap_or_else(|error| panic!("required Linux tool `unshare` cannot run: {error}"));
    assert!(
        output.status.success(),
        "Linux netns test failed; unprivileged user/network namespaces are required (check `unshare -rn true`):\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\":\"");
    let rest = &line[line.find(&needle)? + needle.len()..];
    Some(&rest[..rest.find('"')?])
}

fn matches(line: &str, key: &str, value: &str) -> bool {
    field(line, key) == Some(value)
}

fn required_field(line: &str, key: &str) -> String {
    field(line, key)
        .unwrap_or_else(|| panic!("missing {key}: {line}"))
        .to_owned()
}

fn request(stream: &mut TcpStream, text: &str) -> u64 {
    assert!(text.len() <= 250);
    let mut bytes = [0; 250];
    bytes[..text.len()].copy_from_slice(text.as_bytes());
    let last_activity = utc_millis();
    stream
        .write_all(&bytes)
        .unwrap_or_else(|error| panic!("write initial request: {error}"));
    last_activity
}

const DAY_MILLIS: u64 = 86_400_000;

fn utc_millis() -> u64 {
    (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|error| panic!("system clock before Unix epoch: {error}"))
        .as_millis()
        % u128::from(DAY_MILLIS)) as u64
}

fn assert_detection_time(event: &str, drop_base: u64, last_activity: u64) {
    let ts = required_field(event, "ts");
    let component = |range: std::ops::Range<usize>| -> u64 {
        ts.get(range)
            .unwrap_or_else(|| panic!("invalid log timestamp: {ts}"))
            .parse()
            .unwrap_or_else(|error| panic!("invalid log timestamp {ts}: {error}"))
    };
    let ended = component(11..13) * 3_600_000
        + component(14..16) * 60_000
        + component(17..19) * 1000
        + component(20..23);
    let elapsed = (ended + DAY_MILLIS - drop_base) % DAY_MILLIS;
    assert!(
        elapsed <= (K + TIMER_MARGIN).as_millis() as u64,
        "logged cleanup exceeded K plus timer margin ({elapsed} ms): {event}"
    );
    // The initial request/target connect is the last real activity on the failed
    // connection; no test payload is sent before DROP. Allow 1 s timer rounding.
    let since_activity = (ended + DAY_MILLIS - last_activity) % DAY_MILLIS;
    assert!(
        since_activity >= 9000,
        "cleanup preceded TCP idle detection ({since_activity} ms): {event}"
    );
}

fn configure_stream(stream: &TcpStream) {
    stream
        .set_read_timeout(Some(IO_LIMIT))
        .unwrap_or_else(|error| panic!("set read timeout: {error}"));
    stream
        .set_write_timeout(Some(IO_LIMIT))
        .unwrap_or_else(|error| panic!("set write timeout: {error}"));
    stream
        .set_nodelay(true)
        .unwrap_or_else(|error| panic!("set TCP_NODELAY: {error}"));
}

struct Relay {
    child: Child,
    dir: PathBuf,
    viewer: u16,
    server: u16,
}

impl Relay {
    fn start(places: u32) -> Self {
        let holders: Vec<_> = (0..4)
            .map(|_| {
                TcpListener::bind("127.0.0.1:0")
                    .unwrap_or_else(|error| panic!("reserve listener port: {error}"))
            })
            .collect();
        let ports: Vec<_> = holders
            .iter()
            .map(|listener| {
                listener
                    .local_addr()
                    .unwrap_or_else(|error| panic!("listener address: {error}"))
                    .port()
            })
            .collect();
        let dir = std::env::temp_dir().join(format!(
            "vnc-relay-netns-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir)
            .unwrap_or_else(|error| panic!("create {}: {error}", dir.display()));
        let accounts = dir.join("accounts.toml");
        std::fs::write(&accounts, "version = 1\n[[account]]\nname = \"alice\"\npassword_hash = \"$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\"\n")
            .unwrap_or_else(|error| panic!("write accounts: {error}"));
        std::fs::set_permissions(&accounts, std::fs::Permissions::from_mode(0o600))
            .unwrap_or_else(|error| panic!("set account permissions: {error}"));
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
[mode1]
enabled = true
allow_targets = ["127.0.0.1:1-65535"]
[mode2]
enabled = true
[limits]
relay_connections = {places}
waiting = {places}
[timeouts]
handshake_secs = 2
wait_secs = 120
connect_secs = 2
close_secs = 1
[keepalive]
tcp_idle_secs = 10
tcp_interval_secs = 1
tcp_probes = 1
[log]
output = "events.jsonl"
"#,
            ports[0], ports[1], ports[2], ports[3]
        );
        let config_path = dir.join("relay.toml");
        std::fs::write(&config_path, config)
            .unwrap_or_else(|error| panic!("write relay config: {error}"));
        let stderr = std::fs::File::create(dir.join("stderr"))
            .unwrap_or_else(|error| panic!("create stderr: {error}"));
        drop(holders);
        let child = Command::new(BIN)
            .args(["serve", "--config"])
            .arg(config_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(stderr)
            .spawn()
            .unwrap_or_else(|error| panic!("spawn relay: {error}"));
        let mut relay = Self {
            child,
            dir,
            viewer: ports[2],
            server: ports[3],
        };
        let deadline = Instant::now() + IO_LIMIT;
        loop {
            if relay
                .lines()
                .iter()
                .any(|line| matches(line, "event", "server_ready"))
            {
                return relay;
            }
            let status = relay
                .child
                .try_wait()
                .unwrap_or_else(|error| panic!("poll relay: {error}"));
            assert!(
                status.is_none() && Instant::now() < deadline,
                "relay not ready: {status:?}; {}",
                std::fs::read_to_string(relay.dir.join("stderr")).unwrap_or_default()
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

    fn wait(&self, after: usize, deadline: Instant, predicate: impl Fn(&str) -> bool) -> String {
        loop {
            let lines = self.lines();
            if let Some(line) = lines.iter().skip(after).find(|line| predicate(line)) {
                return line.clone();
            }
            assert!(
                Instant::now() < deadline,
                "event deadline exceeded; log: {lines:#?}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn connect(&self, viewer: bool) -> (TcpStream, String) {
        let after = self.lines().len();
        let mut stream =
            TcpStream::connect(("127.0.0.1", if viewer { self.viewer } else { self.server }))
                .unwrap_or_else(|error| panic!("connect relay: {error}"));
        configure_stream(&stream);
        if viewer {
            let mut greeting = [0; 12];
            stream
                .read_exact(&mut greeting)
                .unwrap_or_else(|error| panic!("read greeting: {error}"));
            assert_eq!(&greeting, b"RFB 000.000\n");
        }
        let event = self.wait(after, Instant::now() + IO_LIMIT, |line| {
            matches(line, "event", "connection_accepted")
                && matches(line, "role", if viewer { "viewer" } else { "server" })
        });
        (stream, required_field(&event, "conn_id"))
    }

    fn waiting_server(&self, id: &str) -> (TcpStream, String, String, u64) {
        let (mut stream, conn_id) = self.connect(false);
        let last_activity = request(&mut stream, &format!("ID:{id}"));
        let event = self.wait(0, Instant::now() + IO_LIMIT, |line| {
            matches(line, "event", "waiting_registered") && matches(line, "conn_id", &conn_id)
        });
        (
            stream,
            conn_id,
            required_field(&event, "relay_id_ref"),
            last_activity,
        )
    }

    fn mode2(&self, id: &str) -> Session {
        let (server, server_id, _, peer_activity) = self.waiting_server(id);
        let (mut viewer, viewer_id) = self.connect(true);
        let viewer_activity = request(&mut viewer, &format!("ID:{id}"));
        let event = self.wait(0, Instant::now() + IO_LIMIT, |line| {
            matches(line, "event", "session_started")
                && matches(line, "viewer_conn_id", &viewer_id)
                && matches(line, "server_conn_id", &server_id)
        });
        Session {
            viewer,
            peer: server,
            id: required_field(&event, "session_id"),
            viewer_activity,
            peer_activity,
        }
    }

    fn mode1(&self) -> Session {
        let listener =
            TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| panic!("bind target: {error}"));
        listener
            .set_nonblocking(true)
            .unwrap_or_else(|error| panic!("set target nonblocking: {error}"));
        let port = listener
            .local_addr()
            .unwrap_or_else(|error| panic!("target address: {error}"))
            .port();
        let (mut viewer, viewer_id) = self.connect(true);
        let viewer_activity = request(&mut viewer, &format!("127.0.0.1:{port}"));
        let deadline = Instant::now() + IO_LIMIT;
        let peer_activity = utc_millis();
        let peer = loop {
            match listener.accept() {
                Ok((peer, _)) => break peer,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        Instant::now() < deadline,
                        "target connect deadline exceeded"
                    );
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("accept target: {error}"),
            }
        };
        configure_stream(&peer);
        let event = self.wait(0, deadline, |line| {
            matches(line, "event", "session_started") && matches(line, "viewer_conn_id", &viewer_id)
        });
        Session {
            viewer,
            peer,
            id: required_field(&event, "session_id"),
            viewer_activity,
            peer_activity,
        }
    }

    fn assert_at_limit(&self) {
        let after = self.lines().len();
        let mut excess = TcpStream::connect(("127.0.0.1", self.server))
            .unwrap_or_else(|error| panic!("connect above L: {error}"));
        configure_stream(&excess);
        self.wait(after, Instant::now() + IO_LIMIT, |line| {
            matches(line, "event", "connection_rejected")
                && matches(line, "reason", "relay_connection_limit")
        });
        assert_eof(&mut excess);
    }
}

impl Drop for Relay {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

struct Session {
    viewer: TcpStream,
    peer: TcpStream,
    id: String,
    viewer_activity: u64,
    peer_activity: u64,
}

impl Session {
    fn transfer(&mut self) {
        fn direction(sender: &mut TcpStream, receiver: &mut TcpStream, bytes: &[u8]) {
            sender
                .write_all(bytes)
                .unwrap_or_else(|error| panic!("write healthy stream: {error}"));
            let mut received = vec![0; bytes.len()];
            receiver
                .read_exact(&mut received)
                .unwrap_or_else(|error| panic!("read healthy stream: {error}"));
            assert_eq!(received, bytes);
        }
        direction(&mut self.viewer, &mut self.peer, b"healthy viewer bytes");
        direction(&mut self.peer, &mut self.viewer, b"healthy peer bytes");
    }
}

fn assert_eof(stream: &mut TcpStream) {
    let mut byte = [0];
    assert_eq!(
        stream.read(&mut byte).unwrap_or_else(|error| panic!(
            "healthy side must receive EOF, not timeout/error: {error}"
        )),
        0
    );
}

struct PacketDrop;

impl PacketDrop {
    fn install(port: u16) -> Self {
        // No FIN, RST, or failed write is injected. The peer stays open and idle.
        let mut script = String::from("add table inet relay_dead_peer\n");
        for (chain, hook, interface) in [
            ("input", "input", "iifname"),
            ("output", "output", "oifname"),
        ] {
            script.push_str(&format!("add chain inet relay_dead_peer {chain} {{ type filter hook {hook} priority 0; policy accept; }}\n"));
            for direction in ["sport", "dport"] {
                script.push_str(&format!("add rule inet relay_dead_peer {chain} {interface} \"lo\" tcp {direction} {port} counter drop\n"));
            }
        }
        let mut child = Command::new("nft")
            .args(["-f", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|error| panic!("required Linux tool `nft` cannot run: {error}"));
        let mut stdin = child
            .stdin
            .take()
            .unwrap_or_else(|| panic!("nft stdin missing"));
        stdin
            .write_all(script.as_bytes())
            .unwrap_or_else(|error| panic!("write nft DROP rules: {error}"));
        drop(stdin);
        let output = child
            .wait_with_output()
            .unwrap_or_else(|error| panic!("wait nft: {error}"));
        assert!(
            output.status.success(),
            "install nft DROP rules: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        Self
    }

    fn assert_dropped_packets(&self) {
        let output = checked_output("nft", &["list", "table", "inet", "relay_dead_peer"]);
        let rules = String::from_utf8_lossy(&output.stdout);
        let dropped: u64 = rules
            .lines()
            .filter_map(|line| line.split_once("counter packets ").map(|(_, count)| count))
            .map(|count| {
                count
                    .split_whitespace()
                    .next()
                    .unwrap_or_else(|| panic!("missing packet count: {rules}"))
                    .parse::<u64>()
                    .unwrap_or_else(|error| panic!("invalid nft packet count: {error}"))
            })
            .sum();
        assert!(
            dropped > 0,
            "nft must actually DROP keepalive packets: {rules}"
        );
    }
}

impl Drop for PacketDrop {
    fn drop(&mut self) {
        let _ = Command::new("nft")
            .args(["delete", "table", "inet", "relay_dead_peer"])
            .output();
    }
}

/// spec: relay.md:RLY-025
#[test]
fn removes_unresponsive_waiting_server_and_reuses_id_and_l() {
    in_namespace(
        "removes_unresponsive_waiting_server_and_reuses_id_and_l",
        || {
            let relay = Relay::start(1);
            let (dead, conn_id, relay_id_ref, last_activity) = relay.waiting_server("25001");
            relay.assert_at_limit();
            let port = dead
                .local_addr()
                .unwrap_or_else(|error| panic!("dead peer address: {error}"))
                .port();
            let began = Instant::now();
            let drop_base = utc_millis();
            let drop = PacketDrop::install(port);
            let removed = relay.wait(0, began + K + TIMER_MARGIN + IO_LIMIT, |line| {
                matches(line, "event", "waiting_removed") && matches(line, "conn_id", &conn_id)
            });
            assert_detection_time(&removed, drop_base, last_activity);
            assert!(matches(&removed, "reason", "dead_connection"), "{removed}");
            assert!(matches(&removed, "level", "warn"), "{removed}");
            assert!(
                matches(&removed, "relay_id_ref", &relay_id_ref),
                "{removed}"
            );
            drop.assert_dropped_packets();
            let (_replacement, replacement_id, replacement_ref, _) = relay.waiting_server("25001");
            assert_ne!(replacement_id, conn_id);
            assert_eq!(replacement_ref, relay_id_ref);
            relay.assert_at_limit();
            // `dead` is still alive here: the test never closes the failed peer to trigger cleanup.
            assert!(dead.peer_addr().is_ok());
        },
    );
}

/// spec: relay.md:RLY-026
#[test]
fn removes_unresponsive_active_peers_without_harming_other_sessions() {
    in_namespace(
        "removes_unresponsive_active_peers_without_harming_other_sessions",
        || {
            for (mode1, dead_viewer, side) in [
                (false, true, "viewer"),
                (false, false, "server"),
                (true, true, "viewer"),
                (true, false, "target"),
            ] {
                let relay = Relay::start(4);
                let mut healthy = relay.mode2("26001");
                let mut failed = if mode1 {
                    relay.mode1()
                } else {
                    relay.mode2("26002")
                };
                healthy.transfer();
                relay.assert_at_limit();
                let dead = if dead_viewer {
                    &failed.viewer
                } else {
                    &failed.peer
                };
                let port = dead
                    .local_addr()
                    .unwrap_or_else(|error| panic!("dead peer address: {error}"))
                    .port();
                let began = Instant::now();
                let drop_base = utc_millis();
                let drop = PacketDrop::install(port);
                let closed = relay.wait(0, began + K + TIMER_MARGIN + IO_LIMIT, |line| {
                    matches(line, "event", "session_closed")
                        && matches(line, "session_id", &failed.id)
                });
                assert_detection_time(
                    &closed,
                    drop_base,
                    if dead_viewer {
                        failed.viewer_activity
                    } else {
                        failed.peer_activity
                    },
                );
                assert!(
                    matches(&closed, "close_cause", "dead_connection"),
                    "{closed}"
                );
                assert!(matches(&closed, "level", "warn"), "{closed}");
                assert!(matches(&closed, "side", side), "{closed}");
                assert_eof(if dead_viewer {
                    &mut failed.peer
                } else {
                    &mut failed.viewer
                });
                drop.assert_dropped_packets();
                healthy.transfer();
                assert!(
                    !relay
                        .lines()
                        .iter()
                        .any(|line| matches(line, "event", "session_closed")
                            && matches(line, "session_id", &healthy.id)),
                    "unrelated session closed"
                );
                let (_first, _, _, _) = relay.waiting_server("26003");
                let (_second, _, _, _) = relay.waiting_server("26004");
                relay.assert_at_limit();
                assert_eq!(
                    relay
                        .lines()
                        .iter()
                        .filter(|line| matches(line, "event", "session_closed")
                            && matches(line, "session_id", &failed.id))
                        .count(),
                    1
                );
            }
        },
    );
}
