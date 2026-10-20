//! Contract tests of the operation log writer, `relay_id_ref`, and the
//! status listener (docs/contracts/ops-info-v1.md §1.1-1.6, §1.9, §1.11,
//! §1.12, §2; recorded in §5).

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{Map, Value, json};
use vnc_relay_adapters::crypto::RelayIdKey;
use vnc_relay_adapters::ops_log::JsonLinesEventSink;
use vnc_relay_adapters::status_http::StatusListener;
use vnc_relay_app::events::{AbortReason, ConnectionErrorKind, Event, Phase, RejectReason, Stage};
use vnc_relay_app::lifecycle::LifecycleService;
use vnc_relay_app::ports::EventSink;
use vnc_relay_app::settings::{ConfigWarning, LogOutput};
use vnc_relay_domain::allow_list::IpItem;
use vnc_relay_domain::budget::ConnectionBudget;
use vnc_relay_domain::conn_id::ConnId;
use vnc_relay_domain::entry_point::EntryPoint;
use vnc_relay_domain::peer_ip::PeerIp;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vnc-relay-ops-log-{}-{name}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir)
            .unwrap_or_else(|error| panic!("clear {}: {error}", dir.display()));
    }
    std::fs::create_dir_all(&dir)
        .unwrap_or_else(|error| panic!("create {}: {error}", dir.display()));
    dir
}

fn file_sink(path: &std::path::Path) -> JsonLinesEventSink {
    JsonLinesEventSink::open(&LogOutput::File(path.to_owned()), |error| {
        panic!("log write failed: {error}")
    })
    .unwrap_or_else(|error| panic!("open {}: {error}", path.display()))
}

fn read(path: &std::path::Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

/// Every line of `text` as a JSON object. Fails on a line that is not one.
fn objects(text: &str) -> Vec<Map<String, Value>> {
    assert!(text.ends_with('\n'), "last line has no LF: {text:?}");
    text.lines()
        .map(|line| match serde_json::from_str::<Value>(line) {
            Ok(Value::Object(object)) => object,
            other => panic!("not a JSON object line: {line:?} ({other:?})"),
        })
        .collect()
}

fn text<'a>(object: &'a Map<String, Value>, key: &str) -> &'a str {
    object
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("no string `{key}` in {object:?}"))
}

fn keys(object: &Map<String, Value>) -> Vec<&str> {
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    keys
}

fn failure(reason: &'static str, detail: Option<String>) -> Event {
    Event::StartupFailed {
        reason,
        config_key: None,
        listener: None,
        detail,
    }
}

/// `ts` is `YYYY-MM-DDTHH:MM:SS.mmmZ` (ops-info-v1 §1.2).
fn assert_timestamp(ts: &str) {
    let bytes = ts.as_bytes();
    assert_eq!(bytes.len(), 24, "{ts}");
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
            Some(expected) => assert_eq!(*byte, expected, "{ts}"),
            None => assert!(byte.is_ascii_digit(), "{ts}"),
        }
    }
}

#[test]
fn writes_one_json_object_per_line_with_common_fields_first() {
    let dir = temp_dir("lines");
    let path = dir.join("relay.log");
    let log = file_sink(&path);
    log.record(&Event::ServerStarting);
    log.record(&Event::ConfigWarning {
        warning: ConfigWarning::AllowAllSource {
            entry_point: EntryPoint::TcpViewer,
            item: IpItem::parse("0.0.0.0/0").unwrap_or_else(|e| panic!("{e:?}")),
        },
        phase: Phase::Startup,
    });
    log.record(&Event::StartupFailed {
        reason: "listener_bind_failed",
        config_key: Some("relay.tcp_viewer.listen".to_owned()),
        listener: Some("tcp_viewer"),
        detail: Some("이미 사용 중이다.".to_owned()),
    });
    log.record(&Event::ServerStopped);

    let written = read(&path);
    for line in written.lines() {
        assert!(line.starts_with("{\"ts\":\""), "ts is not first: {line}");
    }
    let lines = objects(&written);
    let summary: Vec<(&str, &str)> = lines
        .iter()
        .map(|object| {
            assert_timestamp(text(object, "ts"));
            (text(object, "level"), text(object, "event"))
        })
        .collect();
    assert_eq!(
        summary,
        [
            ("info", "server_starting"),
            ("warn", "allow_all_source"),
            ("error", "startup_failed"),
            ("info", "server_stopped"),
        ]
    );
    assert_eq!(text(&lines[0], "log_contract"), "ops-info/1.2.0");
    assert_eq!(text(&lines[1], "listener"), "tcp_viewer");
    assert_eq!(text(&lines[1], "item"), "0.0.0.0/0");
    assert_eq!(text(&lines[1], "phase"), "startup");
    assert_eq!(text(&lines[2], "reason"), "listener_bind_failed");
    assert_eq!(text(&lines[2], "config_key"), "relay.tcp_viewer.listen");
    assert_eq!(text(&lines[2], "listener"), "tcp_viewer");
    assert_eq!(text(&lines[2], "detail"), "이미 사용 중이다.");
    assert_eq!(keys(&lines[3]), ["event", "level", "ts"]);
}

#[test]
fn writes_each_config_warning_with_its_contract_fields() {
    let dir = temp_dir("warnings");
    let path = dir.join("relay.log");
    let log = file_sink(&path);
    let listen = "0.0.0.0:8080".parse().unwrap_or_else(|e| panic!("{e}"));
    let warnings = [
        ConfigWarning::PlaintextWsEnabled { listen },
        ConfigWarning::AllowAllTarget {
            item: vnc_relay_domain::target_policy::TargetItem::parse("0.0.0.0/0:5900")
                .unwrap_or_else(|e| panic!("{e:?}")),
        },
        ConfigWarning::StatusListenNotLoopback { listen },
    ];
    for warning in warnings {
        log.record(&Event::ConfigWarning {
            warning,
            phase: Phase::Reload,
        });
    }
    let lines = objects(&read(&path));
    assert_eq!(text(&lines[0], "event"), "plaintext_ws_enabled");
    assert_eq!(text(&lines[0], "listener"), "ws_viewer");
    assert_eq!(text(&lines[0], "listen"), "0.0.0.0:8080");
    assert_eq!(
        keys(&lines[0]),
        ["event", "level", "listen", "listener", "ts"]
    );
    assert_eq!(text(&lines[1], "event"), "allow_all_target");
    assert_eq!(text(&lines[1], "item"), "0.0.0.0/0:5900");
    assert_eq!(text(&lines[1], "phase"), "reload");
    assert_eq!(text(&lines[2], "event"), "status_listen_not_loopback");
    assert_eq!(keys(&lines[2]), ["event", "level", "listen", "ts"]);
    for line in &lines {
        assert_eq!(text(line, "level"), "warn");
    }
}

#[test]
fn writes_connection_events_with_their_contract_fields() {
    let dir = temp_dir("connections");
    let path = dir.join("relay.log");
    let log = file_sink(&path);
    let peer = |text: &str| PeerIp::new(text.parse().unwrap_or_else(|e| panic!("{text}: {e}")));
    let conn_id = ConnId::next();
    log.record(&Event::ConnectionRejected {
        listener: "tcp_viewer",
        stage: Stage::Accept,
        reason: RejectReason::IpNotAllowed,
        peer_ip: peer("::ffff:198.51.100.21"),
    });
    log.record(&Event::ConnectionAccepted {
        conn_id,
        entry_point: EntryPoint::TcpServer,
        peer_ip: peer("2001:db8::5"),
    });
    log.record(&Event::ConnectionAborted {
        conn_id,
        entry_point: EntryPoint::TcpServer,
        stage: Stage::InitialExchange,
        reason: AbortReason::ServerShutdown,
        peer_ip: peer("2001:db8::5"),
        elapsed_ms: 1234,
    });
    let lines = objects(&read(&path));

    // A connection refused at accept has no conn_id (ops-info-v1 §1.5).
    assert_eq!(text(&lines[0], "event"), "connection_rejected");
    assert_eq!(text(&lines[0], "level"), "warn");
    assert_eq!(
        keys(&lines[0]),
        [
            "event", "level", "listener", "peer_ip", "reason", "stage", "ts"
        ]
    );
    assert_eq!(text(&lines[0], "listener"), "tcp_viewer");
    assert_eq!(text(&lines[0], "stage"), "accept");
    assert_eq!(text(&lines[0], "reason"), "ip_not_allowed");
    assert_eq!(text(&lines[0], "peer_ip"), "198.51.100.21");

    assert_eq!(text(&lines[1], "event"), "connection_accepted");
    assert_eq!(text(&lines[1], "level"), "info");
    assert_eq!(
        keys(&lines[1]),
        [
            "conn_id", "event", "level", "listener", "peer_ip", "role", "ts"
        ]
    );
    assert_eq!(text(&lines[1], "conn_id"), conn_id.to_string());
    assert_eq!(text(&lines[1], "listener"), "tcp_server");
    assert_eq!(text(&lines[1], "role"), "server");
    assert_eq!(text(&lines[1], "peer_ip"), "2001:db8::5");

    // server_shutdown is an info line (ops-info-v1 §1.5).
    assert_eq!(text(&lines[2], "event"), "connection_aborted");
    assert_eq!(text(&lines[2], "level"), "info");
    assert_eq!(
        keys(&lines[2]),
        [
            "conn_id",
            "elapsed_ms",
            "event",
            "level",
            "listener",
            "peer_ip",
            "reason",
            "stage",
            "ts"
        ]
    );
    assert_eq!(text(&lines[2], "conn_id"), conn_id.to_string());
    assert_eq!(text(&lines[2], "stage"), "initial_exchange");
    assert_eq!(text(&lines[2], "reason"), "server_shutdown");
    assert_eq!(lines[2].get("elapsed_ms"), Some(&Value::from(1234)));
}

#[test]
fn writes_limit_and_error_kind_with_the_reasons_that_carry_them() {
    let dir = temp_dir("connection-reasons");
    let path = dir.join("relay.log");
    let log = file_sink(&path);
    let peer = PeerIp::new([192, 0, 2, 7].into());
    let conn_id = ConnId::next();
    log.record(&Event::ConnectionRejected {
        listener: "tcp_server",
        stage: Stage::Accept,
        reason: RejectReason::RelayConnectionLimit { limit: 2 },
        peer_ip: peer,
    });
    log.record(&Event::ConnectionRejected {
        listener: "status",
        stage: Stage::Accept,
        reason: RejectReason::AdminConnectionLimit { limit: 64 },
        peer_ip: peer,
    });
    for reason in [
        AbortReason::HandshakeTimeout,
        AbortReason::PeerEof,
        AbortReason::ConnectionError(ConnectionErrorKind::IoError),
    ] {
        log.record(&Event::ConnectionAborted {
            conn_id,
            entry_point: EntryPoint::TcpViewer,
            stage: Stage::InitialExchange,
            reason,
            peer_ip: peer,
            elapsed_ms: 10_000,
        });
    }
    let lines = objects(&read(&path));

    let rejected = [
        ("tcp_server", "relay_connection_limit", 2),
        ("status", "admin_connection_limit", 64),
    ];
    for (line, (listener, reason, limit)) in lines.iter().zip(rejected) {
        assert_eq!(text(line, "event"), "connection_rejected");
        assert_eq!(text(line, "listener"), listener);
        assert_eq!(text(line, "reason"), reason);
        assert_eq!(text(line, "level"), "warn");
        assert_eq!(line.get("limit"), Some(&Value::from(limit)));
        assert_eq!(
            keys(line),
            [
                "event", "level", "limit", "listener", "peer_ip", "reason", "stage", "ts"
            ]
        );
    }

    let aborted = [
        ("handshake_timeout", "warn", None),
        ("peer_eof", "info", None),
        ("connection_error", "warn", Some("io_error")),
    ];
    for (line, (reason, level, error_kind)) in lines[2..].iter().zip(aborted) {
        assert_eq!(text(line, "event"), "connection_aborted");
        assert_eq!(text(line, "reason"), reason);
        assert_eq!(text(line, "level"), level);
        assert_eq!(
            line.get("error_kind").and_then(Value::as_str),
            error_kind,
            "{reason}"
        );
        assert!(line.get("limit").is_none(), "{reason}");
    }
    assert_eq!(lines.len(), 5);
}

#[test]
fn writes_server_ready_with_the_name_and_address_of_every_listener() {
    let dir = temp_dir("ready");
    let path = dir.join("relay.log");
    file_sink(&path).record(&Event::ServerReady {
        listeners: vec![
            ("status", SocketAddr::from(([127, 0, 0, 1], 9000))),
            ("tcp_viewer", SocketAddr::from(([0, 0, 0, 0], 5900))),
            (
                "tcp_server",
                "[::]:5500".parse().unwrap_or_else(|e| panic!("{e}")),
            ),
        ],
    });
    let lines = objects(&read(&path));

    assert_eq!(text(&lines[0], "event"), "server_ready");
    assert_eq!(text(&lines[0], "level"), "info");
    assert_eq!(keys(&lines[0]), ["event", "level", "listeners", "ts"]);
    assert_eq!(
        lines[0].get("listeners"),
        Some(&json!([
            { "name": "status", "listen": "127.0.0.1:9000" },
            { "name": "tcp_viewer", "listen": "0.0.0.0:5900" },
            { "name": "tcp_server", "listen": "[::]:5500" },
        ]))
    );
}

/// A status listener on 127.0.0.1 in a thread of its own. Dropping it
/// stops the listener.
struct StatusServer {
    addr: SocketAddr,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl StatusServer {
    fn start(lifecycle: Arc<LifecycleService>, log: Arc<dyn EventSink>) -> Self {
        let (addr_tx, addr_rx) = std::sync::mpsc::channel();
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .enable_time()
                .build()
                .unwrap_or_else(|error| panic!("runtime: {error}"));
            runtime.block_on(async move {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                    .await
                    .unwrap_or_else(|error| panic!("bind: {error}"));
                let addr = listener
                    .local_addr()
                    .unwrap_or_else(|error| panic!("local_addr: {error}"));
                addr_tx
                    .send(addr)
                    .unwrap_or_else(|error| panic!("send: {error}"));
                let status = StatusListener::new(listener, ConnectionBudget::new(64), lifecycle);
                tokio::select! {
                    () = status.run(log) => {}
                    _ = stopped => {}
                }
            });
        });
        let addr = addr_rx
            .recv()
            .unwrap_or_else(|error| panic!("status listener did not start: {error}"));
        Self {
            addr,
            stop: Some(stop),
            thread: Some(thread),
        }
    }
}

impl Drop for StatusServer {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// One HTTP reply, read until the server closes the connection.
struct Reply {
    status: u16,
    /// Header names in lower case.
    headers: Vec<(String, String)>,
    body: String,
}

impl Reply {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    fn json(&self) -> Value {
        serde_json::from_str(&self.body)
            .unwrap_or_else(|error| panic!("body is not JSON ({error}): {:?}", self.body))
    }
}

/// Sends `method target` with `extra` header lines and `Connection: close`.
fn send(addr: SocketAddr, method: &str, target: &str, extra: &str) -> Reply {
    exchange(
        addr,
        &format!("{method} {target} HTTP/1.1\r\nHost: status\r\n{extra}Connection: close\r\n\r\n"),
    )
}

/// Sends the whole `request` and reads the reply until the server closes.
fn exchange(addr: SocketAddr, request: &str) -> Reply {
    let mut stream =
        TcpStream::connect(addr).unwrap_or_else(|error| panic!("connect {addr}: {error}"));
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap_or_else(|error| panic!("read timeout: {error}"));
    stream
        .write_all(request.as_bytes())
        .unwrap_or_else(|error| panic!("write: {error}"));
    let mut raw = String::new();
    stream
        .read_to_string(&mut raw)
        .unwrap_or_else(|error| panic!("read: {error}"));
    let (head, body) = raw
        .split_once("\r\n\r\n")
        .unwrap_or_else(|| panic!("no header end: {raw:?}"));
    let mut head = head.lines();
    let status = head
        .next()
        .and_then(|line| line.split(' ').nth(1))
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status line: {raw:?}"));
    let headers = head
        .map(|line| {
            let (name, value) = line
                .split_once(':')
                .unwrap_or_else(|| panic!("bad header: {line:?}"));
            (name.to_ascii_lowercase(), value.trim().to_owned())
        })
        .collect();
    Reply {
        status,
        headers,
        body: body.to_owned(),
    }
}

fn assert_json_reply(reply: &Reply, status: u16, body: &Value) {
    assert_eq!(reply.status, status, "{}", reply.body);
    assert_eq!(reply.header("content-type"), Some("application/json"));
    assert_eq!(reply.header("cache-control"), Some("no-store"));
    assert_eq!(&reply.json(), body);
}

/// The error body of ops-info-v1 §2: only `code` and a non-empty `message`.
fn assert_error_reply(reply: &Reply, status: u16, code: &str) {
    let body = reply.json();
    let message = body
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("no message: {body}"));
    assert!(!message.is_empty());
    assert_json_reply(
        reply,
        status,
        &json!({ "error": { "code": code, "message": message } }),
    );
}

/// spec: operations.md:OPS-013 partial
/// spec: operations.md:OPS-032 partial
#[test]
fn answers_liveness_and_readiness_in_each_state_with_only_the_contract_fields() {
    let dir = temp_dir("status-states");
    let log: Arc<dyn EventSink> = Arc::new(file_sink(&dir.join("relay.log")));
    let lifecycle = Arc::new(LifecycleService::new());
    let server = StatusServer::start(Arc::clone(&lifecycle), log);
    let alive = json!({ "status": "alive" });

    assert_json_reply(&send(server.addr, "GET", "/livez", ""), 200, &alive);
    assert_json_reply(
        &send(server.addr, "GET", "/readyz", ""),
        503,
        &json!({ "status": "not_ready", "reason": "starting" }),
    );

    lifecycle.mark_ready();
    assert_json_reply(&send(server.addr, "GET", "/livez", ""), 200, &alive);
    assert_json_reply(
        &send(server.addr, "GET", "/readyz", ""),
        200,
        &json!({ "status": "ready" }),
    );

    lifecycle.begin_shutdown();
    assert_json_reply(&send(server.addr, "GET", "/livez", ""), 200, &alive);
    assert_json_reply(
        &send(server.addr, "GET", "/readyz", ""),
        503,
        &json!({ "status": "not_ready", "reason": "shutting_down" }),
    );
}

/// spec: operations.md:OPS-032 partial
#[test]
fn ignores_credentials_and_queries_and_refuses_other_methods_and_paths() {
    let dir = temp_dir("status-requests");
    let log: Arc<dyn EventSink> = Arc::new(file_sink(&dir.join("relay.log")));
    let lifecycle = Arc::new(LifecycleService::new());
    lifecycle.mark_ready();
    let server = StatusServer::start(lifecycle, log);
    let ready = json!({ "status": "ready" });

    let credentials = "Authorization: Basic YWxpY2U6cGFzcw==\r\nCookie: session=abc\r\n";
    assert_json_reply(
        &send(server.addr, "GET", "/readyz?verbose=1", credentials),
        200,
        &ready,
    );
    let with_body = "GET /readyz HTTP/1.1\r\nHost: status\r\nContent-Type: application/json\r\n\
                     Content-Length: 2\r\nConnection: close\r\n\r\n{}";
    assert_json_reply(&exchange(server.addr, with_body), 200, &ready);

    for method in ["POST", "PUT", "DELETE", "HEAD"] {
        for path in ["/livez", "/readyz"] {
            let reply = send(server.addr, method, path, "Content-Length: 0\r\n");
            assert_eq!(reply.status, 405, "{method} {path}");
            assert_eq!(reply.header("allow"), Some("GET"), "{method} {path}");
            assert_eq!(reply.header("content-type"), Some("application/json"));
            assert_eq!(reply.header("cache-control"), Some("no-store"));
            if method != "HEAD" {
                assert_error_reply(&reply, 405, "method_not_allowed");
            }
        }
    }
    for (method, path) in [
        ("GET", "/"),
        ("GET", "/livez/"),
        ("GET", "/metrics"),
        ("POST", "/metrics"),
    ] {
        let reply = send(server.addr, method, path, "Content-Length: 0\r\n");
        assert_error_reply(&reply, 404, "not_found");
        assert_eq!(reply.header("allow"), None, "{method} {path}");
    }
}

#[test]
fn leaves_out_absent_optional_fields_instead_of_writing_null() {
    let dir = temp_dir("optional");
    let path = dir.join("relay.log");
    file_sink(&path).record(&failure("runtime_unavailable", None));
    let lines = objects(&read(&path));
    assert_eq!(keys(&lines[0]), ["event", "level", "reason", "ts"]);
}

#[test]
fn escapes_line_breaks_and_cuts_detail_to_512_bytes() {
    let dir = temp_dir("escape");
    let path = dir.join("relay.log");
    let log = file_sink(&path);
    log.record(&failure(
        "runtime_unavailable",
        Some("첫 줄\n둘째 줄\r\t\"따옴표\"\u{1}".to_owned()),
    ));
    let long = "가".repeat(400);
    log.record(&failure("runtime_unavailable", Some(long.clone())));

    let written = read(&path);
    assert_eq!(written.lines().count(), 2, "{written}");
    let lines = objects(&written);
    assert_eq!(
        text(&lines[0], "detail"),
        "첫 줄\n둘째 줄\r\t\"따옴표\"\u{1}"
    );
    let detail = text(&lines[1], "detail");
    // 170 syllables are 510 bytes; one more would pass 512.
    assert_eq!(detail.len(), 510);
    assert!(long.starts_with(detail));
}

#[test]
fn appends_to_an_existing_file_and_creates_a_missing_one() {
    let dir = temp_dir("append");
    let path = dir.join("relay.log");
    std::fs::write(&path, "earlier line\n").unwrap_or_else(|e| panic!("write: {e}"));
    file_sink(&path).record(&Event::ServerStopped);
    let written = read(&path);
    assert!(written.starts_with("earlier line\n{"), "{written}");
    assert_eq!(written.lines().count(), 2);

    let created = dir.join("new.log");
    file_sink(&created).record(&Event::ServerStopped);
    assert_eq!(objects(&read(&created)).len(), 1);
}

#[test]
fn reports_log_file_unavailable_without_the_path() {
    let dir = temp_dir("unavailable");
    let cases = [dir.join("missing-dir").join("relay.log"), dir.clone()];
    for path in cases {
        let Err(error) = JsonLinesEventSink::open(&LogOutput::File(path.clone()), |_| {}) else {
            panic!("opened {}", path.display());
        };
        assert_eq!(error.code(), "log_file_unavailable");
        let name = dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_else(|| panic!("dir name"));
        assert!(!error.cause().contains(name), "{}", error.cause());
        assert!(!error.to_string().contains(name), "{error}");
    }
}

#[test]
fn keeps_lines_whole_when_threads_record_at_once() {
    let dir = temp_dir("threads");
    let path = dir.join("relay.log");
    let log = Arc::new(file_sink(&path));
    let threads: Vec<_> = (0..8)
        .map(|thread| {
            let log = Arc::clone(&log);
            std::thread::spawn(move || {
                for i in 0..200 {
                    let detail = format!("{thread}-{i}\n{}", "x".repeat(300));
                    log.record(&failure("runtime_unavailable", Some(detail)));
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap_or_else(|_| panic!("writer panicked"));
    }
    let lines = objects(&read(&path));
    assert_eq!(lines.len(), 1600);
    let mut seen: Vec<&str> = lines
        .iter()
        .map(|line| {
            let detail = text(line, "detail");
            detail.split_once('\n').map_or(detail, |(id, _)| id)
        })
        .collect();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), 1600);
}

#[cfg(target_os = "linux")]
#[test]
fn drops_lines_and_notifies_only_the_first_write_failure() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let notices = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&notices);
    let log = JsonLinesEventSink::open(&LogOutput::File("/dev/full".into()), move |_| {
        counter.fetch_add(1, Ordering::Relaxed);
    })
    .unwrap_or_else(|error| panic!("open /dev/full: {error}"));
    for _ in 0..3 {
        log.record(&Event::ServerStopped);
    }
    assert_eq!(notices.load(Ordering::Relaxed), 1);
}

/// spec: operations.md:OPS-012 partial
#[test]
fn relay_id_ref_is_a_keyed_16_hex_digest_without_the_relay_id() {
    let key = RelayIdKey::generate().unwrap_or_else(|error| panic!("{error}"));
    let other = RelayIdKey::generate().unwrap_or_else(|error| panic!("{error}"));
    let relay_id = b"1234";
    let reference = key.reference(relay_id);
    let shown = reference.as_str();
    assert_eq!(shown.len(), 16);
    assert!(
        shown
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "{shown}"
    );
    // Same key, same relay ID: one session can be followed in the log.
    assert_eq!(key.reference(relay_id), reference);
    assert_ne!(key.reference(b"1235"), reference);
    // A new process key gives a new reference (ops-info-v1 §1.9).
    assert_ne!(other.reference(relay_id), reference);
    assert_eq!(format!("{key:?}"), "RelayIdKey(<redacted>)");
}
