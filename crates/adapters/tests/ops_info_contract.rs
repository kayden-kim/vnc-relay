//! Contract tests of the operation log writer and `relay_id_ref`
//! (docs/contracts/ops-info-v1.md §1.1-1.6, §1.9, §1.11; recorded in §5).

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Map, Value};
use vnc_relay_adapters::crypto::RelayIdKey;
use vnc_relay_adapters::ops_log::JsonLinesEventSink;
use vnc_relay_app::events::{AbortReason, Event, Phase, RejectReason, Stage};
use vnc_relay_app::ports::EventSink;
use vnc_relay_app::settings::{ConfigWarning, LogOutput};
use vnc_relay_domain::allow_list::IpItem;
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
        entry_point: EntryPoint::TcpViewer,
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
