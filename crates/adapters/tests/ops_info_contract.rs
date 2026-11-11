//! Contract tests of the operation log writer, `relay_id_ref`, and the
//! status listener (docs/contracts/ops-info-v2.md §1.1-1.6, §1.9, §1.11,
//! §1.12, §2; recorded in §5).

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

use serde_json::{Map, Value, json};
use vnc_relay_adapters::crypto::RelayIdKey;
use vnc_relay_adapters::ops_log::{JsonLinesEventSink, format_line};
use vnc_relay_adapters::status_http::StatusListener;
use vnc_relay_app::events::{
    AbortReason, ConnectionErrorKind, Event, Phase, RejectReason, Stage, TargetFailureReason,
    WaitingRemovalReason,
};
use vnc_relay_app::lifecycle::LifecycleService;
use vnc_relay_app::ports::EventSink;
use vnc_relay_app::reload::ReloadTrigger;
use vnc_relay_app::settings::{ConfigWarning, LogOutput};
use vnc_relay_domain::account::AccountName;
use vnc_relay_domain::allow_list::IpItem;
use vnc_relay_domain::budget::ConnectionBudget;
use vnc_relay_domain::conn_id::ConnId;
use vnc_relay_domain::entry_point::{EntryPoint, Listener, Role};
use vnc_relay_domain::handshake::Mode;
use vnc_relay_domain::peer_ip::PeerIp;
use vnc_relay_domain::session::{CloseCause, SessionId, Side, Transport};

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
    JsonLinesEventSink::open(
        &LogOutput::File(path.to_owned()),
        |error| panic!("log write failed: {error}"),
        |error| panic!("log reopen failed: {error}"),
    )
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

/// `ts` is `YYYY-MM-DDTHH:MM:SS.mmmZ` (ops-info-v2 §1.2).
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

/// spec: operations.md:OPS-069 partial
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
    assert_eq!(text(&lines[0], "log_contract"), "ops-info/2.1.0");
    assert_eq!(text(&lines[1], "listener"), "tcp_viewer");
    assert_eq!(text(&lines[1], "item"), "0.0.0.0/0");
    assert_eq!(text(&lines[1], "phase"), "startup");
    assert_eq!(text(&lines[2], "reason"), "listener_bind_failed");
    assert_eq!(text(&lines[2], "config_key"), "relay.tcp_viewer.listen");
    assert_eq!(text(&lines[2], "listener"), "tcp_viewer");
    assert_eq!(text(&lines[2], "detail"), "이미 사용 중이다.");
    assert_eq!(keys(&lines[3]), ["event", "level", "ts"]);
}

/// spec: operations.md:OPS-059 partial
/// spec: operations.md:OPS-060 partial
/// spec: operations.md:OPS-061 partial
#[test]
fn config_edit_events_have_required_fields_and_never_values() {
    let dir = temp_dir("config-edit-events");
    let path = dir.join("relay.log");
    let log = file_sink(&path);
    let account = AccountName::parse("alice").unwrap_or_else(|error| panic!("account: {error}"));
    let peer = PeerIp::new(
        "::ffff:192.0.2.17"
            .parse()
            .unwrap_or_else(|error| panic!("peer: {error}")),
    );
    let changed = vec![
        "relay.tcp_viewer.allow".to_owned(),
        "mode1.allow_targets".to_owned(),
    ];
    let applied = vec![
        "relay.tcp_viewer.allow".to_owned(),
        "mode1.allow_targets".to_owned(),
        "timeouts.wait_secs".to_owned(),
    ];
    // SHA-256 of the raw bytes "abc"; the log must preserve the exact revision.
    let revision =
        "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".to_owned();
    for trigger in [ReloadTrigger::FileWatch, ReloadTrigger::ConfigEdit] {
        log.record(&Event::ReloadApplied {
            trigger,
            applied: applied.clone(),
        });
    }
    log.record(&Event::ConfigEdited {
        admin_account: account.clone(),
        peer_ip: peer,
        config_keys: changed.clone(),
        revision: revision.clone(),
    });
    let reasons = [
        "config_invalid",
        "config_edit_disabled",
        "config_conflict",
        "config_restart_pending",
        "config_file_invalid",
        "config_write_failed",
    ];
    for reason in reasons {
        for config_keys in [None, Some(Vec::new()), Some(changed.clone())] {
            log.record(&Event::ConfigEditRejected {
                admin_account: account.clone(),
                peer_ip: peer,
                reason: reason.to_owned(),
                config_keys,
            });
        }
    }
    let written = read(&path);
    let lines = objects(&written);
    assert_eq!(lines.len(), 3 + reasons.len() * 3);
    for (line, trigger) in lines[..2].iter().zip(["file_watch", "config_edit"]) {
        assert_eq!(text(line, "event"), "reload_applied");
        assert_eq!(text(line, "level"), "info");
        assert_eq!(text(line, "trigger"), trigger);
        assert_eq!(line.get("applied"), Some(&json!(applied)));
        assert_eq!(keys(line), ["applied", "event", "level", "trigger", "ts"]);
    }
    let edited = &lines[2];
    assert_eq!(text(edited, "event"), "config_edited");
    assert_eq!(text(edited, "level"), "info");
    assert_eq!(text(edited, "admin_account"), "alice");
    assert_eq!(text(edited, "peer_ip"), "192.0.2.17");
    assert_eq!(edited.get("config_keys"), Some(&json!(changed)));
    assert_eq!(text(edited, "revision"), revision);
    let hash = text(edited, "revision")
        .strip_prefix("sha256:")
        .unwrap_or_else(|| panic!("revision has no SHA-256 prefix: {edited:?}"));
    assert_eq!(hash.len(), 64);
    assert!(
        hash.bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    );
    assert_eq!(
        keys(edited),
        [
            "admin_account",
            "config_keys",
            "event",
            "level",
            "peer_ip",
            "revision",
            "ts",
        ]
    );
    for (group, reason) in lines[3..].as_chunks::<3>().0.iter().zip(reasons) {
        for (index, rejected) in group.iter().enumerate() {
            assert_eq!(text(rejected, "event"), "config_edit_rejected");
            assert_eq!(text(rejected, "level"), "warn");
            assert_eq!(text(rejected, "admin_account"), "alice");
            assert_eq!(text(rejected, "peer_ip"), "192.0.2.17");
            assert_eq!(text(rejected, "reason"), reason);
            let mut expected_keys =
                vec!["admin_account", "event", "level", "peer_ip", "reason", "ts"];
            match index {
                0 => assert!(!rejected.contains_key("config_keys")),
                1 => {
                    assert_eq!(rejected.get("config_keys"), Some(&json!([])));
                    expected_keys.push("config_keys");
                }
                _ => {
                    assert_eq!(rejected.get("config_keys"), Some(&json!(changed)));
                    expected_keys.push("config_keys");
                }
            }
            expected_keys.sort_unstable();
            assert_eq!(keys(rejected), expected_keys);
        }
    }
    for line in &lines {
        assert_timestamp(text(line, "ts"));
        assert!(line.values().all(|value| !value.is_null()));
        for forbidden in [
            "value",
            "values",
            "changes",
            "diagnostics",
            "detail",
            "origin",
            "target_host",
            "target_ip",
            "target_port",
            "limit",
            "password",
            "password_hash",
            "cookie",
            "csrf_token",
        ] {
            assert!(!line.contains_key(forbidden), "{forbidden}: {line:?}");
        }
    }
    let ipv6 = objects(&format_line(
        &Event::ConfigEditRejected {
            admin_account: account,
            peer_ip: PeerIp::new(
                "2001:0DB8:0000:0000:0000:0000:0000:0017"
                    .parse()
                    .unwrap_or_else(|error| panic!("IPv6 peer: {error}")),
            ),
            reason: "config_conflict".to_owned(),
            config_keys: None,
        },
        UNIX_EPOCH,
    ))
    .remove(0);
    assert_eq!(text(&ipv6, "peer_ip"), "2001:db8::17");
}

/// spec: operations.md:OPS-058 partial
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
    assert_eq!(text(&lines[0], "listener"), "ws");
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

/// spec: operations.md:OPS-069 partial
#[test]
fn shared_listener_logs_resolve_role_only_after_the_path() {
    for listener in Listener::ALL {
        let conn_id = ConnId::next();
        let accepted = objects(&format_line(
            &Event::ConnectionAccepted {
                conn_id,
                listener,
                peer_ip: relay_peer(),
            },
            UNIX_EPOCH,
        ))
        .remove(0);
        assert_eq!(text(&accepted, "listener"), listener.name());
        assert_eq!(
            accepted.get("role").and_then(Value::as_str),
            listener.fixed_role().map(Role::as_str)
        );
        if listener.fixed_role().is_none() {
            for role in [None, Some(Role::Viewer), Some(Role::Server)] {
                let aborted = objects(&format_line(
                    &Event::ConnectionAborted {
                        conn_id,
                        listener,
                        role,
                        stage: Stage::Upgrade,
                        reason: AbortReason::PeerEof,
                        peer_ip: relay_peer(),
                        forwarded_ip: None,
                        elapsed_ms: 1,
                    },
                    UNIX_EPOCH,
                ))
                .remove(0);
                let rejected = objects(&format_line(
                    &Event::UpgradeRejected {
                        conn_id,
                        listener: listener.name(),
                        role,
                        reason: RejectReason::InvalidUpgrade,
                        peer_ip: relay_peer(),
                        forwarded_ip: None,
                        origin: None,
                        ticket_jti: None,
                    },
                    UNIX_EPOCH,
                ))
                .remove(0);
                for event in [aborted, rejected] {
                    assert_eq!(text(&event, "listener"), listener.name());
                    assert_eq!(
                        event.get("role").and_then(Value::as_str),
                        role.map(Role::as_str)
                    );
                }
            }
        }
    }
    for entry_point in EntryPoint::ALL {
        for phase in [Phase::Startup, Phase::Reload] {
            let warning = objects(&format_line(
                &Event::ConfigWarning {
                    warning: ConfigWarning::AllowAllSource {
                        entry_point,
                        item: IpItem::parse("0.0.0.0/0").unwrap(),
                    },
                    phase,
                },
                UNIX_EPOCH,
            ))
            .remove(0);
            assert_eq!(text(&warning, "listener"), entry_point.listener().name());
            assert_eq!(text(&warning, "phase"), phase.as_str());
            assert_eq!(
                warning.get("role").and_then(Value::as_str),
                entry_point
                    .transport()
                    .is_websocket()
                    .then(|| entry_point.role().as_str())
            );
        }
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
        listener: Listener::TcpServer,
        peer_ip: peer("2001:db8::5"),
    });
    log.record(&Event::ConnectionAborted {
        conn_id,
        listener: Listener::TcpServer,
        role: Some(Role::Server),
        stage: Stage::InitialExchange,
        reason: AbortReason::ServerShutdown,
        peer_ip: peer("2001:db8::5"),
        forwarded_ip: None,
        elapsed_ms: 1234,
    });
    let lines = objects(&read(&path));

    // A connection refused at accept has no conn_id (ops-info-v2 §1.5).
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

    // server_shutdown is an info line (ops-info-v2 §1.5).
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
            "role",
            "stage",
            "ts"
        ]
    );
    assert_eq!(text(&lines[2], "conn_id"), conn_id.to_string());
    assert_eq!(text(&lines[2], "stage"), "initial_exchange");
    assert_eq!(text(&lines[2], "reason"), "server_shutdown");
    assert_eq!(text(&lines[2], "role"), "server");
    assert_eq!(lines[2].get("elapsed_ms"), Some(&Value::from(1234)));
}

/// spec: relay.md:RLY-004 partial
/// spec: relay.md:RLY-022 partial
#[test]
fn writes_initial_exchange_rejections_with_conn_id_and_disabled_mode() {
    // Only the operation log is checked, not socket closure or resource release.
    let dir = temp_dir("initial-exchange-rejections");
    let path = dir.join("relay.log");
    let log = file_sink(&path);
    let peer_ip = PeerIp::new([192, 0, 2, 7].into());
    let conn_id = ConnId::next();
    let cases = [
        (
            EntryPoint::TcpServer,
            RejectReason::HandshakeInvalid,
            None,
            "handshake_invalid",
            None,
        ),
        (
            EntryPoint::TcpViewer,
            RejectReason::HandshakeInvalid,
            Some(Mode::Mode1),
            "handshake_invalid",
            Some("mode1"),
        ),
        (
            EntryPoint::TcpServer,
            RejectReason::HandshakeInvalid,
            Some(Mode::Mode2),
            "handshake_invalid",
            Some("mode2"),
        ),
        (
            EntryPoint::TcpViewer,
            RejectReason::ModeDisabled,
            Some(Mode::Mode1),
            "mode_disabled",
            Some("mode1"),
        ),
        (
            EntryPoint::TcpServer,
            RejectReason::ModeDisabled,
            Some(Mode::Mode2),
            "mode_disabled",
            Some("mode2"),
        ),
    ];
    for (entry_point, reason, mode, ..) in cases {
        assert_eq!(reason.limit(), None);
        log.record(&Event::HandshakeRejected {
            conn_id,
            entry_point,
            peer_ip,
            forwarded_ip: None,
            reason,
            mode,
        });
    }

    let written = read(&path);
    let lines = objects(&written);
    assert_eq!(lines.len(), cases.len());
    for ((object, line), (entry_point, _, _, expected_reason, expected_mode)) in
        lines.iter().zip(written.lines()).zip(cases)
    {
        assert_timestamp(text(object, "ts"));
        assert_eq!(text(object, "event"), "connection_rejected");
        assert_eq!(text(object, "level"), "warn");
        assert_eq!(text(object, "conn_id"), conn_id.to_string());
        assert_eq!(text(object, "listener"), entry_point.listener().name());
        assert_eq!(text(object, "role"), entry_point.role().as_str());
        assert_eq!(text(object, "stage"), "initial_exchange");
        assert_eq!(text(object, "reason"), expected_reason);
        assert_eq!(text(object, "peer_ip"), "192.0.2.7");
        assert_eq!(object.get("mode").and_then(Value::as_str), expected_mode);
        assert!(object.values().all(|value| !value.is_null()), "{line}");

        let mode_field =
            expected_mode.map_or_else(String::new, |mode| format!(",\"mode\":\"{mode}\""));
        // The exact line checks field order and excludes request bytes, raw
        // relay IDs, and every absent optional field (ops-info-v2 §1.1, §1.9).
        assert_eq!(
            line,
            format!(
                "{{\"ts\":\"{}\",\"level\":\"warn\",\"event\":\"connection_rejected\",\
                 \"conn_id\":\"{conn_id}\",\"listener\":\"{}\",\"role\":\"{}\",\"stage\":\"initial_exchange\",\
                 \"reason\":\"{expected_reason}\",\"peer_ip\":\"192.0.2.7\"{mode_field}}}",
                text(object, "ts"),
                entry_point.listener().name(),
                entry_point.role().as_str(),
            )
        );
    }
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
            listener: Listener::TcpViewer,
            role: Some(Role::Viewer),
            stage: Stage::InitialExchange,
            reason,
            peer_ip: peer,
            forwarded_ip: None,
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
                let mut candidate = vnc_relay_adapters::config_file::parse_candidate(
                    b"[admin]\naccounts_file = 'accounts.toml'\n[management]\nlisten = '127.0.0.1:18080'\n[status]\nlisten = '127.0.0.1:18081'\n",
                    &std::env::temp_dir(),
                )
                .unwrap_or_else(|diagnostics| panic!("status settings: {diagnostics:?}"));
                candidate.settings.limits.admin_connections = 64;
                let source = Arc::new(vnc_relay_app::reload::snapshot::ReloadCell::<()>::new(
                    vnc_relay_app::reload::snapshot::ReloadSnapshot::new(candidate, None, None),
                ));
                let status = StatusListener::new(
                    listener,
                    ConnectionBudget::new(),
                    lifecycle,
                    source,
                );
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

/// The error body of ops-info-v2 §2: only `code` and a non-empty `message`.
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

/// spec: operations.md:OPS-013
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
        let Err(error) = JsonLinesEventSink::open(&LogOutput::File(path.clone()), |_| {}, |_| {})
        else {
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
    let log = JsonLinesEventSink::open(
        &LogOutput::File("/dev/full".into()),
        move |_| {
            counter.fetch_add(1, Ordering::Relaxed);
        },
        |error| panic!("log reopen failed: {error}"),
    )
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
    // A new process key gives a new reference (ops-info-v2 §1.9).
    assert_ne!(other.reference(relay_id), reference);
    assert_eq!(format!("{key:?}"), "RelayIdKey(<redacted>)");
}

fn relay_peer() -> PeerIp {
    PeerIp::new(
        "::ffff:198.51.100.21"
            .parse()
            .unwrap_or_else(|error| panic!("fixture IP: {error}")),
    )
}

fn assert_relay_fields(event: &Event, expected: Value) {
    let line = format_line(event, UNIX_EPOCH);
    let mut actual = objects(&line).remove(0);
    assert_eq!(actual.remove("ts"), Some(json!("1970-01-01T00:00:00.000Z")));
    assert!(actual.values().all(|value| !value.is_null()));
    assert_eq!(Value::Object(actual), expected);
}

/// spec: relay.md:RLY-007 partial
/// spec: relay.md:RLY-008 partial
/// spec: relay.md:RLY-009 partial
/// spec: operations.md:OPS-023 partial
#[test]
fn writes_target_rejections_failures_and_aborts_with_only_contract_fields() {
    let conn_id = ConnId::next();
    let ip = "192.0.2.80".parse().unwrap();
    for target_ip in [None, Some(ip)] {
        let mut expected = json!({
            "event": "connection_rejected", "level": "warn",
            "conn_id": conn_id.to_string(), "listener": "tcp_viewer",
            "role": "viewer", "mode": "mode1", "peer_ip": "198.51.100.21",
            "stage": "target_connect", "reason": "target_not_allowed",
            "target_host": "host.example", "target_port": 5900
        });
        if target_ip.is_some() {
            expected["target_ip"] = json!("192.0.2.80");
        }
        assert_relay_fields(
            &Event::TargetRejected {
                conn_id,
                entry_point: EntryPoint::TcpViewer,
                peer_ip: relay_peer(),
                forwarded_ip: None,
                reason: RejectReason::TargetNotAllowed,
                target_host: "host.example".to_owned(),
                target_port: 5900,
                target_ip,
            },
            expected,
        );
    }
    assert_relay_fields(
        &Event::TargetRejected {
            conn_id,
            entry_point: EntryPoint::TcpViewer,
            peer_ip: relay_peer(),
            forwarded_ip: None,
            reason: RejectReason::RelayConnectionLimit { limit: 12 },
            target_host: "not-logged.example".to_owned(),
            target_port: 5900,
            target_ip: Some(ip),
        },
        json!({
            "event": "connection_rejected", "level": "warn",
            "conn_id": conn_id.to_string(), "listener": "tcp_viewer",
            "role": "viewer", "mode": "mode1", "peer_ip": "198.51.100.21",
            "stage": "target_connect", "reason": "relay_connection_limit", "limit": 12
        }),
    );
    for (reason, code) in [
        (TargetFailureReason::ResolveFailed, "resolve_failed"),
        (TargetFailureReason::ConnectRefused, "connect_refused"),
        (TargetFailureReason::ConnectTimeout, "connect_timeout"),
        (TargetFailureReason::ConnectError, "connect_error"),
    ] {
        for target_ip in [None, Some(ip)] {
            let mut expected = json!({
                "event": "target_connect_failed", "level": "warn",
                "conn_id": conn_id.to_string(), "listener": "tcp_viewer",
                "viewer_transport": "tcp", "peer_ip": "198.51.100.21",
                "target_host": "host.example", "target_port": 5900,
                "elapsed_ms": 123, "reason": code
            });
            if target_ip.is_some() {
                expected["target_ip"] = json!("192.0.2.80");
            }
            // ConnectError also covers K setup failure: no error_kind,
            // mode, role, or session_id belongs to this event.
            assert_relay_fields(
                &Event::TargetConnectFailed {
                    conn_id,
                    entry_point: EntryPoint::TcpViewer,
                    viewer_transport: Transport::Tcp,
                    peer_ip: relay_peer(),
                    forwarded_ip: None,
                    target_host: "host.example".to_owned(),
                    target_port: 5900,
                    target_ip,
                    elapsed_ms: 123,
                    reason,
                },
                expected,
            );
        }
    }
    for (reason, code, level, kind) in [
        (AbortReason::ServerShutdown, "server_shutdown", "info", None),
        (AbortReason::PeerEof, "peer_eof", "info", None),
        (AbortReason::DeadConnection, "dead_connection", "warn", None),
        (
            AbortReason::ConnectionError(ConnectionErrorKind::IoError),
            "connection_error",
            "warn",
            Some("io_error"),
        ),
    ] {
        let mut expected = json!({
            "event": "connection_aborted", "level": level,
            "conn_id": conn_id.to_string(), "listener": "tcp_viewer",
            "stage": "target_connect", "reason": code,
            "peer_ip": "198.51.100.21", "elapsed_ms": 456,
            "role": "viewer", "mode": "mode1"
        });
        if let Some(kind) = kind {
            expected["error_kind"] = json!(kind);
        }
        assert_relay_fields(
            &Event::RelayAborted {
                conn_id,
                entry_point: EntryPoint::TcpViewer,
                stage: Stage::TargetConnect,
                reason,
                peer_ip: relay_peer(),
                forwarded_ip: None,
                elapsed_ms: 456,
                mode: Mode::Mode1,
            },
            expected,
        );
    }
}

/// spec: relay.md:RLY-013 partial
/// spec: relay.md:RLY-014 partial
/// spec: operations.md:OPS-005 partial
#[test]
fn writes_waiting_events_without_raw_ids_or_unrelated_optional_fields() {
    let key = RelayIdKey::generate().unwrap();
    let raw = b"raw-private-relay-id-987654321";
    let reference = key.reference(raw);
    let conn_id = ConnId::next();
    assert_relay_fields(
        &Event::WaitingRegistered {
            conn_id,
            entry_point: EntryPoint::TcpServer,
            peer_ip: relay_peer(),
            forwarded_ip: None,
            relay_id_ref: reference.clone(),
        },
        json!({
            "event": "waiting_registered", "level": "info",
            "conn_id": conn_id.to_string(), "listener": "tcp_server",
            "role": "server", "mode": "mode2",
            "relay_id_ref": reference.as_str(), "peer_ip": "198.51.100.21"
        }),
    );
    assert_relay_fields(
        &Event::WaitingExpired {
            conn_id,
            role: Role::Server,
            relay_id_ref: reference.clone(),
            waited_ms: 3000,
        },
        json!({
            "event": "waiting_expired", "level": "info",
            "conn_id": conn_id.to_string(), "role": "server",
            "relay_id_ref": reference.as_str(), "waited_ms": 3000
        }),
    );
    for (reason, code, level, kind) in [
        (WaitingRemovalReason::PeerEof, "peer_eof", "info", None),
        (
            WaitingRemovalReason::ServerShutdown,
            "server_shutdown",
            "info",
            None,
        ),
        (
            WaitingRemovalReason::DeadConnection,
            "dead_connection",
            "warn",
            None,
        ),
        (
            WaitingRemovalReason::ConnectionError(ConnectionErrorKind::IoError),
            "connection_error",
            "warn",
            Some("io_error"),
        ),
    ] {
        let event = Event::WaitingRemoved {
            conn_id,
            role: Role::Server,
            relay_id_ref: reference.clone(),
            waited_ms: 17,
            reason,
        };
        let mut expected = json!({
            "event": "waiting_removed", "level": level,
            "conn_id": conn_id.to_string(), "role": "server",
            "relay_id_ref": reference.as_str(), "waited_ms": 17, "reason": code
        });
        if let Some(kind) = kind {
            expected["error_kind"] = json!(kind);
        }
        assert_relay_fields(&event, expected);
        assert!(!format_line(&event, UNIX_EPOCH).contains(std::str::from_utf8(raw).unwrap()));
    }
    for reason in [
        RejectReason::DuplicateId,
        RejectReason::WaitingFull { limit: 8 },
    ] {
        let event = Event::RegistrationRejected {
            conn_id,
            entry_point: EntryPoint::TcpServer,
            peer_ip: relay_peer(),
            forwarded_ip: None,
            reason,
            relay_id_ref: reference.clone(),
        };
        let mut expected = json!({
            "event": "connection_rejected", "level": "warn",
            "conn_id": conn_id.to_string(), "listener": "tcp_server",
            "role": "server", "mode": "mode2", "peer_ip": "198.51.100.21",
            "stage": "register", "reason": reason.as_str()
        });
        if reason == RejectReason::DuplicateId {
            expected["relay_id_ref"] = json!(reference.as_str());
        } else {
            expected["limit"] = json!(8);
        }
        assert_relay_fields(&event, expected);
        assert!(!format_line(&event, UNIX_EPOCH).contains(std::str::from_utf8(raw).unwrap()));
    }
    assert_relay_fields(
        &Event::RelayAborted {
            conn_id,
            entry_point: EntryPoint::TcpServer,
            stage: Stage::Register,
            reason: AbortReason::ServerShutdown,
            peer_ip: relay_peer(),
            forwarded_ip: None,
            elapsed_ms: 12,
            mode: Mode::Mode2,
        },
        json!({
            "event": "connection_aborted", "level": "info",
            "conn_id": conn_id.to_string(), "listener": "tcp_server",
            "role": "server", "mode": "mode2", "peer_ip": "198.51.100.21",
            "stage": "register", "reason": "server_shutdown",
            "elapsed_ms": 12
        }),
    );
}

/// spec: relay.md:RLY-006 partial
/// spec: relay.md:RLY-010 partial
/// spec: relay.md:RLY-011 partial
/// spec: management.md:MGT-009 partial
/// spec: websocket.md:WS-028 partial
#[test]
fn writes_session_start_fields_conditionally_by_mode_and_transport() {
    let session_id = SessionId::from_block([7; 16]);
    let viewer = ConnId::next();
    let server = ConnId::next();
    let key = RelayIdKey::generate().unwrap();
    let raw = b"session-raw-relay-id-123456789";
    let reference = key.reference(raw);
    for (transport, transport_text) in [
        (Transport::Tcp, "tcp"),
        (Transport::Ws, "ws"),
        (Transport::Wss, "wss"),
    ] {
        for mode in [Mode::Mode1, Mode::Mode2] {
            for host in [None, Some("host.example".to_owned())] {
                // Populating both sets proves that irrelevant fields are omitted.
                let event = Event::SessionStarted {
                    session_id: session_id.clone(),
                    mode,
                    viewer_transport: transport,
                    server_transport: Some(transport),
                    viewer_conn_id: viewer,
                    viewer_peer_ip: relay_peer(),
                    viewer_forwarded_ip: None,
                    server_conn_id: Some(server),
                    server_peer_ip: Some(relay_peer()),
                    server_forwarded_ip: None,
                    relay_id_ref: Some(reference.clone()),
                    waited_ms: Some(0),
                    target_ip: Some("::ffff:192.0.2.80".parse().unwrap()),
                    target_port: Some(5900),
                    target_host: host.clone(),
                };
                let mut expected = json!({
                    "event": "session_started", "level": "info",
                    "session_id": session_id.as_str(), "mode": mode.as_str(),
                    "viewer_transport": transport_text,
                    "viewer_conn_id": viewer.to_string(), "viewer_peer_ip": "198.51.100.21"
                });
                match mode {
                    Mode::Mode1 => {
                        expected["target_ip"] = json!("192.0.2.80");
                        expected["target_port"] = json!(5900);
                        if let Some(host) = &host {
                            expected["target_host"] = json!(host);
                        }
                    }
                    Mode::Mode2 => {
                        expected["server_transport"] = json!(transport_text);
                        expected["server_conn_id"] = json!(server.to_string());
                        expected["server_peer_ip"] = json!("198.51.100.21");
                        expected["relay_id_ref"] = json!(reference.as_str());
                        expected["waited_ms"] = json!(0);
                    }
                }
                assert_relay_fields(&event, expected);
                assert!(
                    !format_line(&event, UNIX_EPOCH).contains(std::str::from_utf8(raw).unwrap())
                );
            }
        }
    }
}

/// spec: relay.md:RLY-019 partial
/// spec: relay.md:RLY-020 partial
/// spec: management.md:MGT-023 partial
#[test]
fn writes_session_close_cause_levels_and_canonical_optional_fields() {
    let id = SessionId::from_block([9; 16]);
    let key = RelayIdKey::generate().unwrap();
    let raw = b"closed-raw-relay-id-123456789";
    let reference = key.reference(raw);
    let account = vnc_relay_domain::account::AccountName::parse("admin").unwrap();
    for mode in [Mode::Mode1, Mode::Mode2] {
        for (cause, code, level, needs_side) in [
            (CloseCause::PeerEof, "peer_eof", "info", true),
            (
                CloseCause::ConnectionError,
                "connection_error",
                "warn",
                true,
            ),
            (CloseCause::DeadConnection, "dead_connection", "warn", true),
            (CloseCause::CloseTimeout, "close_timeout", "warn", true),
            (
                CloseCause::AdminTerminated,
                "admin_terminated",
                "info",
                false,
            ),
            (CloseCause::ServerShutdown, "server_shutdown", "warn", false),
            (CloseCause::InternalError, "internal_error", "warn", false),
        ] {
            let side = if mode == Mode::Mode1 {
                Side::Target
            } else {
                Side::Server
            };
            let event = Event::SessionClosed {
                session_id: id.clone(),
                mode,
                viewer_transport: Transport::Tcp,
                started_at: UNIX_EPOCH + Duration::from_millis(1234),
                duration_ms: 987,
                bytes_viewer_to_target: u64::MAX,
                bytes_target_to_viewer: 42,
                close_cause: cause,
                side: Some(side),
                error_kind: Some(ConnectionErrorKind::IoError),
                relay_id_ref: Some(reference.clone()),
                admin_account: Some(account.clone()),
            };
            let mut expected = json!({
                "event": "session_closed", "level": level,
                "session_id": id.as_str(), "mode": mode.as_str(), "viewer_transport": "tcp",
                "started_at": "1970-01-01T00:00:01.234Z", "duration_ms": 987,
                "bytes_viewer_to_target": u64::MAX, "bytes_target_to_viewer": 42,
                "close_cause": code
            });
            if mode == Mode::Mode2 {
                expected["relay_id_ref"] = json!(reference.as_str());
            }
            if needs_side {
                expected["side"] = json!(side.as_str());
            }
            if cause == CloseCause::ConnectionError {
                expected["error_kind"] = json!("io_error");
            }
            if cause == CloseCause::AdminTerminated {
                expected["admin_account"] = json!("admin");
            }
            assert_relay_fields(&event, expected);
            assert!(!format_line(&event, UNIX_EPOCH).contains(std::str::from_utf8(raw).unwrap()));
        }
    }
    // An administrator's E expiry has no preceding EOF and therefore no side.
    assert_relay_fields(
        &Event::SessionClosed {
            session_id: id.clone(),
            mode: Mode::Mode1,
            viewer_transport: Transport::Tcp,
            started_at: UNIX_EPOCH,
            duration_ms: 0,
            bytes_viewer_to_target: 0,
            bytes_target_to_viewer: 0,
            close_cause: CloseCause::CloseTimeout,
            side: None,
            error_kind: None,
            relay_id_ref: None,
            admin_account: None,
        },
        json!({
            "event": "session_closed", "level": "warn", "session_id": id.as_str(),
            "mode": "mode1", "viewer_transport": "tcp", "started_at": "1970-01-01T00:00:00.000Z",
            "duration_ms": 0, "bytes_viewer_to_target": 0, "bytes_target_to_viewer": 0,
            "close_cause": "close_timeout"
        }),
    );
}

#[test]
fn cuts_target_host_on_utf8_boundaries_in_every_target_event() {
    let host = format!("{}가나", "a".repeat(254));
    let conn_id = ConnId::next();
    let events = [
        Event::TargetRejected {
            conn_id,
            entry_point: EntryPoint::TcpViewer,
            peer_ip: relay_peer(),
            forwarded_ip: None,
            reason: RejectReason::TargetNotAllowed,
            target_host: host.clone(),
            target_port: 5900,
            target_ip: None,
        },
        Event::TargetConnectFailed {
            conn_id,
            entry_point: EntryPoint::TcpViewer,
            viewer_transport: Transport::Tcp,
            peer_ip: relay_peer(),
            forwarded_ip: None,
            target_host: host.clone(),
            target_port: 5900,
            target_ip: None,
            elapsed_ms: 0,
            reason: TargetFailureReason::ConnectError,
        },
        Event::SessionStarted {
            session_id: SessionId::from_block([0; 16]),
            mode: Mode::Mode1,
            viewer_transport: Transport::Tcp,
            server_transport: None,
            viewer_conn_id: conn_id,
            viewer_peer_ip: relay_peer(),
            viewer_forwarded_ip: None,
            server_conn_id: None,
            server_peer_ip: None,
            server_forwarded_ip: None,
            relay_id_ref: None,
            waited_ms: None,
            target_ip: Some("192.0.2.80".parse().unwrap()),
            target_port: Some(5900),
            target_host: Some(host),
        },
    ];
    for event in events {
        let line = format_line(&event, UNIX_EPOCH);
        let object = objects(&line).remove(0);
        assert_eq!(text(&object, "target_host"), "a".repeat(254));
        assert!(!line.contains('가'));
        assert!(!line.contains('나'));
    }
}

/// spec: operations.md:OPS-008 partial
/// spec: operations.md:OPS-009 partial
/// spec: operations.md:OPS-042 partial
#[test]
fn writes_requested_shutdown_fields_without_unrelated_optional_fields() {
    assert_relay_fields(
        &Event::ShutdownStarted {
            trigger: vnc_relay_app::events::ShutdownTrigger::Requested,
            grace_secs: 10,
            active_sessions: 2,
        },
        json!({
            "event": "shutdown_started", "level": "info", "trigger": "requested",
            "grace_secs": 10, "active_sessions": 2
        }),
    );
    assert_relay_fields(
        &Event::ShutdownForced {
            remaining_sessions: 2,
        },
        json!({
            "event": "shutdown_forced", "level": "warn", "remaining_sessions": 2
        }),
    );
}

/// spec: websocket.md:WS-019 partial
/// spec: websocket.md:WS-008 partial
/// spec: websocket.md:WS-010 partial
#[test]
fn preserves_typed_websocket_error_kinds_in_all_connection_error_events() {
    let conn_id = ConnId::next();
    let session_id = SessionId::from_block([3; 16]);
    let reference = RelayIdKey::generate()
        .unwrap()
        .reference(b"private-ws-relay-id");
    for (kind, code) in [
        (ConnectionErrorKind::WsProtocolError, "ws_protocol_error"),
        (ConnectionErrorKind::WsTextMessage, "ws_text_message"),
        (
            ConnectionErrorKind::WsMessageTooLarge,
            "ws_message_too_large",
        ),
    ] {
        let events = [
            Event::ConnectionAborted {
                conn_id,
                listener: Listener::Ws,
                role: Some(Role::Viewer),
                stage: Stage::InitialExchange,
                reason: AbortReason::ConnectionError(kind),
                peer_ip: relay_peer(),
                forwarded_ip: None,
                elapsed_ms: 17,
            },
            Event::RelayAborted {
                conn_id,
                entry_point: EntryPoint::WsViewer,
                stage: Stage::TargetConnect,
                reason: AbortReason::ConnectionError(kind),
                peer_ip: relay_peer(),
                forwarded_ip: None,
                elapsed_ms: 17,
                mode: Mode::Mode1,
            },
            Event::WaitingRemoved {
                conn_id,
                role: Role::Viewer,
                relay_id_ref: reference.clone(),
                waited_ms: 17,
                reason: WaitingRemovalReason::ConnectionError(kind),
            },
            Event::SessionClosed {
                session_id: session_id.clone(),
                mode: Mode::Mode2,
                viewer_transport: Transport::Ws,
                started_at: UNIX_EPOCH,
                duration_ms: 17,
                bytes_viewer_to_target: 0,
                bytes_target_to_viewer: 0,
                close_cause: CloseCause::ConnectionError,
                side: Some(Side::Viewer),
                error_kind: Some(kind),
                relay_id_ref: Some(reference.clone()),
                admin_account: None,
            },
        ];
        for event in events {
            let object = objects(&format_line(&event, UNIX_EPOCH)).remove(0);
            assert_eq!(text(&object, "error_kind"), code);
            assert_eq!(text(&object, "level"), "warn");
            assert!(object.values().all(|value| !value.is_null()));
            assert!(!object.contains_key("admin_account"));
            assert!(!object.contains_key("relay_id"));
        }
    }
}

/// spec: operations.md:OPS-011 partial
#[test]
fn administrator_termination_outcomes_use_contract_levels_and_omit_absent_identifiers() {
    use vnc_relay_app::events::AdminTerminateResult;
    use vnc_relay_domain::account::AccountName;

    let account = AccountName::parse("alice").unwrap_or_else(|e| panic!("account: {e}"));
    let peer = PeerIp::new(std::net::Ipv4Addr::LOCALHOST.into());
    let id = SessionId::from_block([11; 16]);
    for result in [
        AdminTerminateResult::Terminated,
        AdminTerminateResult::AlreadyClosed,
        AdminTerminateResult::NotFound,
        AdminTerminateResult::Failed,
    ] {
        let found = result != AdminTerminateResult::NotFound;
        let event = Event::AdminTerminate {
            admin_account: account.clone(),
            peer_ip: peer,
            session_id: found.then(|| id.clone()),
            result,
        };
        let line = format_line(&event, UNIX_EPOCH);
        let object = objects(&line).remove(0);
        assert_eq!(text(&object, "event"), "admin_terminate");
        assert_eq!(text(&object, "admin_account"), "alice");
        assert_eq!(text(&object, "peer_ip"), "127.0.0.1");
        assert_eq!(text(&object, "result"), result.as_str());
        assert_eq!(
            text(&object, "level"),
            if result == AdminTerminateResult::Failed {
                "error"
            } else {
                "info"
            }
        );
        assert_eq!(object.contains_key("session_id"), found);
        assert!(object.values().all(|value| !value.is_null()));
        assert!(!object.contains_key("relay_id"));
        assert!(!object.contains_key("csrf_token"));
        assert!(!object.contains_key("cookie"));
    }
}

/// spec: websocket.md:WS-038 partial
/// spec: websocket.md:WS-045 partial
#[test]
fn writes_ticket_acceptance_with_only_authenticated_contract_fields() {
    for listener in [Listener::Ws, Listener::Wss] {
        for role in [Role::Viewer, Role::Server] {
            let conn_id = ConnId::next();
            for optional in [false, true] {
                let event = Event::TicketAccepted {
                    conn_id,
                    listener,
                    role,
                    peer_ip: relay_peer(),
                    forwarded_ip: optional.then(|| PeerIp::new("198.51.100.7".parse().unwrap())),
                    ticket_jti: "connection-1".into(),
                    ticket_sub: optional.then(|| "session-1".into()),
                    session_expires_at: optional.then_some(UNIX_EPOCH + Duration::from_secs(100)),
                };
                let mut expected = json!({
                    "level":"info", "event":"ticket_accepted", "conn_id":conn_id.to_string(),
                    "listener":listener.name(), "role":role.as_str(), "peer_ip":relay_peer().to_string(),
                    "ticket_jti":"connection-1"
                });
                if optional {
                    expected["forwarded_ip"] = json!("198.51.100.7");
                    expected["ticket_sub"] = json!("session-1");
                    expected["session_expires_at"] = json!("1970-01-01T00:01:40.000Z");
                }
                assert_relay_fields(&event, expected);
            }
        }
    }
}

/// spec: websocket.md:WS-039 partial
/// spec: websocket.md:WS-040 partial
/// spec: websocket.md:WS-041 partial
/// spec: websocket.md:WS-042 partial
/// spec: websocket.md:WS-043 partial
#[test]
fn writes_ticket_upgrade_rejections_without_unrelated_or_unauthenticated_fields() {
    use vnc_relay_domain::ticket::TicketError;
    for reason in [
        TicketError::Missing,
        TicketError::Invalid,
        TicketError::Expired,
        TicketError::IssuedBeforeStart,
        TicketError::Reused,
    ] {
        for authenticated in [false, true] {
            let conn_id = ConnId::next();
            let event = Event::UpgradeRejected {
                conn_id,
                listener: "wss",
                role: Some(Role::Viewer),
                reason: RejectReason::Ticket(reason),
                peer_ip: relay_peer(),
                forwarded_ip: Some(PeerIp::new("198.51.100.7".parse().unwrap())),
                origin: None,
                ticket_jti: authenticated.then(|| "connection-1".into()),
            };
            let mut expected = json!({
                "level":"warn", "event":"connection_rejected", "conn_id":conn_id.to_string(),
                "listener":"wss", "role":"viewer", "stage":"upgrade", "reason":reason.code(),
                "peer_ip":relay_peer().to_string(), "forwarded_ip":"198.51.100.7"
            });
            if authenticated {
                expected["ticket_jti"] = json!("connection-1");
            }
            assert_relay_fields(&event, expected);
        }
    }
}
