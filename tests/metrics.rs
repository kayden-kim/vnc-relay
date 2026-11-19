//! The real process: `/metrics` on the status listener and `GET /api/v1/metrics`
//! against the operations log and the management API of the same process
//! (metrics-v1, operations.md §3.8, management.md §2.5).

#[expect(
    dead_code,
    unused_imports,
    reason = "the shared live-process module also carries browser helpers"
)]
#[path = "common/dashboard_live.rs"]
mod live;

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use live::{Admin, LIMIT, PASSWORD, Reply, Server, connect, initial, must, read_reply, wait_for};
use serde_json::{Value, json};

const METRICS_ON: &str = "[metrics]\nenabled = true\n";
/// Relay IDs chosen so that no metric value can contain them by chance.
const PAIR_ID: &str = "246813579";
const WAITING_ID: &str = "975318642";
/// A documentation address outside `mode1.allow_targets`.
const REFUSED_TARGET: &str = "192.0.2.77:5900";

/// A series: the Prometheus name and its labels sorted by name.
type Series = (String, Vec<(String, String)>);

fn status_get(server: &Server, path: &str) -> Reply {
    let mut stream = connect(server.ports[1]);
    must(stream.write_all(
        format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").as_bytes(),
    ));
    read_reply(stream)
}

fn header<'a>(reply: &'a Reply, name: &str) -> Option<&'a str> {
    reply.headers.lines().find_map(|line| {
        line.split_once(':')
            .filter(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.trim())
    })
}

/// Every sample line of the Prometheus text.
fn text_series(text: &str) -> BTreeMap<Series, f64> {
    text.lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (series, value) = line
                .rsplit_once(' ')
                .unwrap_or_else(|| panic!("sample line without value: {line}"));
            let (name, labels) = match series.split_once('{') {
                Some((name, rest)) => {
                    let block = rest.strip_suffix('}').unwrap_or_else(|| panic!("{line}"));
                    let mut labels: Vec<(String, String)> = block
                        .split(',')
                        .map(|pair| {
                            let (key, value) =
                                pair.split_once('=').unwrap_or_else(|| panic!("{line}"));
                            (key.to_owned(), value.trim_matches('"').to_owned())
                        })
                        .collect();
                    labels.sort();
                    (name.to_owned(), labels)
                }
                None => (series.to_owned(), Vec::new()),
            };
            let value = value
                .parse::<f64>()
                .unwrap_or_else(|error| panic!("{line}: {error}"));
            ((name, labels), value)
        })
        .collect()
}

/// The samples of one series name.
fn family(samples: &BTreeMap<Series, f64>, name: &str) -> BTreeMap<Vec<(String, String)>, f64> {
    samples
        .iter()
        .filter(|((series, _), _)| series == name)
        .map(|((_, labels), value)| (labels.clone(), *value))
        .collect()
}

/// The points of one JSON metric with Prometheus label names, by labels.
fn json_family(body: &Value, otel_name: &str) -> BTreeMap<Vec<(String, String)>, f64> {
    let metric = body["metrics"]
        .as_array()
        .and_then(|metrics| metrics.iter().find(|metric| metric["name"] == otel_name))
        .unwrap_or_else(|| panic!("JSON metric {otel_name} missing"));
    metric["points"]
        .as_array()
        .unwrap_or_else(|| panic!("{otel_name} points"))
        .iter()
        .map(|point| {
            let mut labels: Vec<(String, String)> = point["attributes"]
                .as_object()
                .unwrap_or_else(|| panic!("{otel_name} attributes"))
                .iter()
                .map(|(key, value)| {
                    (
                        key.replace('.', "_"),
                        value.as_str().unwrap_or_default().to_owned(),
                    )
                })
                .collect();
            labels.sort();
            let value = point["value"]
                .as_f64()
                .unwrap_or_else(|| panic!("{otel_name} value"));
            (labels, value)
        })
        .collect()
}

fn labels(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    let mut labels: Vec<(String, String)> = pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect();
    labels.sort();
    labels
}

fn events(server: &Server) -> Vec<Value> {
    std::fs::read_to_string(server.dir().join("events.jsonl"))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

fn count_events(server: &Server, name: &str, field: &str, value: &str) -> usize {
    events(server)
        .iter()
        .filter(|event| event["event"] == name && event[field] == value)
        .count()
}

/// Events named `name` counted by the values of `fields`, which are also the
/// label names.
fn counted(log: &[Value], name: &str, fields: &[&str]) -> BTreeMap<Vec<(String, String)>, f64> {
    let mut counts = BTreeMap::new();
    for event in log.iter().filter(|event| event["event"] == name) {
        let pairs: Vec<(&str, &str)> = fields
            .iter()
            .map(|field| {
                let value = event[*field]
                    .as_str()
                    .unwrap_or_else(|| panic!("{name} without {field}: {event}"));
                (*field, value)
            })
            .collect();
        *counts.entry(labels(&pairs)).or_insert(0.0) += 1.0;
    }
    counts
}

fn sum_field(log: &[Value], name: &str, field: &str) -> u64 {
    log.iter()
        .filter(|event| event["event"] == name)
        .filter_map(|event| event[field].as_u64())
        .sum()
}

fn close_after_reply(mut stream: TcpStream) {
    let mut rest = Vec::new();
    // A refused connection may end with a reset instead of EOF.
    let _ = stream.read_to_end(&mut rest);
}

fn login_failure(server: &Server) {
    let body = json!({"account": "alice", "password": format!("{PASSWORD}-wrong")}).to_string();
    let reply = server.request(
        "POST",
        "/api/v1/session",
        &[("Content-Type", "application/json")],
        body.as_bytes(),
    );
    assert_eq!(reply.status, 401, "wrong password: {}", reply.body);
}

fn metrics_json(server: &Server, admin: &Admin) -> Value {
    server.query(admin, "/api/v1/metrics")
}

/// Relay and admin activity of known shape, the log as the oracle, and the
/// management API as the second reader.
///
/// spec: operations.md:OPS-083 partial
/// spec: operations.md:OPS-085 partial
/// spec: operations.md:OPS-086 partial
/// spec: operations.md:OPS-087 partial
/// spec: operations.md:OPS-088 partial
/// spec: operations.md:OPS-090 partial
/// spec: operations.md:OPS-091 partial
/// spec: management.md:MGT-057 partial
#[test]
fn metrics_equal_the_log_counts_and_the_management_api() {
    let server = Server::start_custom("metrics-counts", None, false, METRICS_ON);
    let admin = server.login();
    login_failure(&server);

    // Closed before the initial exchange.
    drop(server.handshaking());
    // A valid Mode I request for a target outside the policy.
    close_after_reply({
        let mut viewer = server.handshaking();
        must(viewer.write_all(&initial(REFUSED_TARGET)));
        viewer
    });
    // A valid Mode I request for a closed loopback port.
    let closed_port = must(must(TcpListener::bind("127.0.0.1:0")).local_addr()).port();
    close_after_reply({
        let mut viewer = server.handshaking();
        must(viewer.write_all(&initial(&format!("127.0.0.1:{closed_port}"))));
        viewer
    });
    // A Viewer that waits, then leaves.
    let waiting = server.waiting(WAITING_ID);
    wait_for("Viewer waiting", LIMIT, || {
        server.query(&admin, "/api/v1/waiting")["items"]
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item["relay_id"] == WAITING_ID))
    });
    drop(waiting);
    wait_for("waiting removed", LIMIT, || {
        events(&server)
            .iter()
            .any(|event| event["event"] == "waiting_removed")
    });
    wait_for("refused target logged", LIMIT, || {
        count_events(
            &server,
            "target_connect_failed",
            "reason",
            "connect_refused",
        ) == 1
            && count_events(
                &server,
                "connection_rejected",
                "reason",
                "target_not_allowed",
            ) == 1
            && count_events(&server, "connection_aborted", "reason", "peer_eof") >= 1
    });

    // Mode II session, read while open.
    let mut pair = server.pair(&admin, "II", PAIR_ID);
    pair.transfer(1000, 3000);
    let detail_path = format!("/api/v1/sessions/{}", pair.sid);
    wait_for("session bytes counted", LIMIT, || {
        let detail = server.query(&admin, &detail_path);
        detail["session"]["bytes_viewer_to_target"] == 1000
            && detail["session"]["bytes_target_to_viewer"] == 3000
    });
    let open = text_series(&status_get(&server, "/metrics").body);
    assert_eq!(
        family(&open, "vnc_relay_session_io_bytes_total"),
        BTreeMap::from([
            (labels(&[("direction", "target_to_viewer")]), 3000.0),
            (labels(&[("direction", "viewer_to_target")]), 1000.0),
        ])
    );
    let busy: BTreeMap<_, _> = family(&open, "vnc_relay_session_count")
        .into_iter()
        .filter(|(_, value)| *value != 0.0)
        .collect();
    assert_eq!(
        busy,
        BTreeMap::from([(
            labels(&[
                ("mode", "mode2"),
                ("target_transport", "tcp"),
                ("viewer_transport", "tcp")
            ]),
            1.0
        )])
    );
    let summary = server.query(&admin, "/api/v1/summary");
    let total = |name: &str| family(&open, name).values().sum::<f64>();
    assert_eq!(
        Some(total("vnc_relay_session_count")),
        summary["counts"]["active"].as_f64()
    );
    assert_eq!(
        Some(total("vnc_relay_waiting_count")),
        summary["counts"]["waiting"].as_f64()
    );
    assert_eq!(
        family(&open, "vnc_relay_connection_in_progress"),
        BTreeMap::from([
            (
                labels(&[("stage", "initial_exchange")]),
                summary["counts"]["handshaking"].as_f64().unwrap_or(-1.0)
            ),
            (
                labels(&[("stage", "target_connect")]),
                summary["counts"]["target_connecting"]
                    .as_f64()
                    .unwrap_or(-1.0)
            ),
        ])
    );
    assert_eq!(
        Some(total("vnc_relay_relay_connection_usage")),
        summary["usage"]["relay_connections"]["current"].as_f64()
    );
    assert_eq!(
        Some(total("vnc_relay_relay_connection_limit")),
        summary["usage"]["relay_connections"]["limit"].as_f64()
    );
    pair.close();

    // Mode I session to a loopback target.
    let mut pair = server.pair(&admin, "I", "");
    let target_address = must(pair.target.local_addr()).to_string();
    pair.transfer(10, 20);
    pair.close();
    wait_for("both sessions closed", LIMIT, || {
        events(&server)
            .iter()
            .filter(|event| event["event"] == "session_closed")
            .count()
            == 2
    });

    let first = status_get(&server, "/metrics");
    assert_eq!(first.status, 200);
    let text = first.body.as_str();
    let samples = text_series(text);
    let log = events(&server);

    // Event counts by the same attributes as the log lines (OPS-085, OPS-088).
    assert_eq!(
        family(&samples, "vnc_relay_connection_accepted_total"),
        counted(&log, "connection_accepted", &["listener"])
    );
    assert_eq!(
        family(&samples, "vnc_relay_connection_rejected_total"),
        counted(
            &log,
            "connection_rejected",
            &["listener", "stage", "reason"]
        )
    );
    assert_eq!(
        family(&samples, "vnc_relay_connection_aborted_total"),
        counted(&log, "connection_aborted", &["listener", "stage", "reason"])
    );
    assert_eq!(
        family(&samples, "vnc_relay_session_closed_total"),
        counted(&log, "session_closed", &["mode", "close_cause"])
    );
    let mut logins = counted(&log, "admin_login_failed", &["reason"])
        .into_iter()
        .map(|(pairs, count)| {
            let reason = pairs
                .first()
                .map(|(_, value)| value.clone())
                .unwrap_or_default();
            (vec![("result".to_owned(), reason)], count)
        })
        .collect::<BTreeMap<_, _>>();
    logins.insert(
        labels(&[("result", "succeeded")]),
        log.iter()
            .filter(|event| event["event"] == "admin_login_succeeded")
            .count() as f64,
    );
    assert_eq!(family(&samples, "vnc_relay_admin_logins_total"), logins);
    assert_eq!(
        family(&samples, "vnc_relay_admin_session_count"),
        BTreeMap::from([(Vec::new(), 1.0)])
    );

    // Sessions (OPS-086).
    let mut started = BTreeMap::new();
    for event in log
        .iter()
        .filter(|event| event["event"] == "session_started")
    {
        let target = event["server_transport"].as_str().unwrap_or("tcp");
        let key = labels(&[
            ("mode", event["mode"].as_str().unwrap_or_default()),
            (
                "viewer_transport",
                event["viewer_transport"].as_str().unwrap_or_default(),
            ),
            ("target_transport", target),
        ]);
        *started.entry(key).or_insert(0.0) += 1.0;
    }
    assert_eq!(family(&samples, "vnc_relay_session_started_total"), started);
    assert_eq!(
        family(&samples, "vnc_relay_session_duration_seconds_count"),
        counted(&log, "session_closed", &["mode"])
    );
    assert_eq!(
        family(&samples, "vnc_relay_session_io_bytes_total"),
        BTreeMap::from([
            (
                labels(&[("direction", "target_to_viewer")]),
                sum_field(&log, "session_closed", "bytes_target_to_viewer") as f64
            ),
            (
                labels(&[("direction", "viewer_to_target")]),
                sum_field(&log, "session_closed", "bytes_viewer_to_target") as f64
            ),
        ])
    );
    let waited = |name: &str| {
        log.iter()
            .filter(|event| event["event"] == name)
            .filter_map(|event| event["waited_ms"].as_u64())
            .collect::<Vec<_>>()
    };
    let matched = waited("session_started");
    let removed = waited("waiting_removed");
    assert_eq!(
        family(&samples, "vnc_relay_waiting_duration_seconds_count"),
        BTreeMap::from([
            (labels(&[("result", "matched")]), matched.len() as f64),
            (labels(&[("result", "removed")]), removed.len() as f64),
        ])
    );
    for (result, values) in [("matched", &matched), ("removed", &removed)] {
        let sum = family(&samples, "vnc_relay_waiting_duration_seconds_sum")
            .get(&labels(&[("result", result)]))
            .copied()
            .unwrap_or(-1.0);
        let expected = values.iter().sum::<u64>() as f64 / 1000.0;
        assert!(
            (sum - expected).abs() < 1e-9,
            "{result}: {sum} != {expected}"
        );
    }
    // Successful initial exchanges: three Mode I Viewers, two Mode II
    // Viewers, one Mode II Server.
    assert_eq!(
        family(&samples, "vnc_relay_handshake_duration_seconds_count"),
        BTreeMap::from([
            (
                labels(&[("listener", "tcp_server"), ("mode", "mode2")]),
                1.0
            ),
            (
                labels(&[("listener", "tcp_viewer"), ("mode", "mode1")]),
                3.0
            ),
            (
                labels(&[("listener", "tcp_viewer"), ("mode", "mode2")]),
                2.0
            ),
        ])
    );

    // Mode I target connects (OPS-087): the policy refusal is not timed.
    assert_eq!(
        family(&samples, "vnc_relay_target_connect_duration_seconds_count"),
        BTreeMap::from([
            (labels(&[("result", "connect_refused")]), 1.0),
            (labels(&[("result", "connected")]), 1.0),
        ])
    );

    // Resource and process values (OPS-083, OPS-090).
    let version = env!("CARGO_PKG_VERSION");
    assert_eq!(
        family(&samples, "target_info"),
        BTreeMap::from([(
            labels(&[("service_name", "vnc-relay"), ("service_version", version)]),
            1.0
        )])
    );
    let positive = |name: &str| {
        let value = family(&samples, name)
            .values()
            .copied()
            .next()
            .unwrap_or(-1.0);
        assert!(value > 0.0, "{name} = {value}");
        value
    };
    let uptime = positive("process_uptime_seconds");
    positive("process_memory_usage_bytes");
    positive("process_memory_virtual_bytes");
    positive("process_thread_count");
    let cpu = family(&samples, "process_cpu_time_seconds_total");
    assert_eq!(
        cpu.keys().cloned().collect::<Vec<_>>(),
        [
            labels(&[("cpu_mode", "system")]),
            labels(&[("cpu_mode", "user")])
        ]
    );
    let (handles, descriptors) = (
        family(&samples, "process_windows_handle_count"),
        family(&samples, "process_unix_file_descriptor_count"),
    );
    if cfg!(windows) {
        assert!(handles.values().all(|value| *value > 0.0) && handles.len() == 1);
        assert!(descriptors.is_empty() && !text.contains("process_unix_file_descriptor_count"));
    } else {
        assert!(descriptors.values().all(|value| *value > 0.0) && descriptors.len() == 1);
        assert!(handles.is_empty() && !text.contains("process_windows_handle_count"));
    }
    thread::sleep(Duration::from_millis(50));
    let second = text_series(&status_get(&server, "/metrics").body);
    let later_uptime = family(&second, "process_uptime_seconds")
        .values()
        .copied()
        .next()
        .unwrap_or(-1.0);
    assert!(later_uptime > uptime, "{later_uptime} <= {uptime}");
    for (labels, before) in &cpu {
        let after = family(&second, "process_cpu_time_seconds_total")
            .get(labels)
            .copied()
            .unwrap_or(-1.0);
        assert!(after >= *before, "{labels:?}: {after} < {before}");
    }

    // The management API reads the same values (MGT-057).
    let body = metrics_json(&server, &admin);
    assert_eq!(body["resource"]["service.name"], "vnc-relay");
    assert_eq!(body["resource"]["service.version"], version);
    for (otel, prometheus) in [
        (
            "vnc_relay.connection.accepted",
            "vnc_relay_connection_accepted_total",
        ),
        (
            "vnc_relay.connection.rejected",
            "vnc_relay_connection_rejected_total",
        ),
        (
            "vnc_relay.connection.aborted",
            "vnc_relay_connection_aborted_total",
        ),
        (
            "vnc_relay.connection.in_progress",
            "vnc_relay_connection_in_progress",
        ),
        ("vnc_relay.waiting.count", "vnc_relay_waiting_count"),
        ("vnc_relay.session.count", "vnc_relay_session_count"),
        (
            "vnc_relay.session.started",
            "vnc_relay_session_started_total",
        ),
        ("vnc_relay.session.closed", "vnc_relay_session_closed_total"),
        ("vnc_relay.session.io", "vnc_relay_session_io_bytes_total"),
        ("vnc_relay.admin.logins", "vnc_relay_admin_logins_total"),
        (
            "vnc_relay.admin_session.count",
            "vnc_relay_admin_session_count",
        ),
        (
            "vnc_relay.relay_connection.limit",
            "vnc_relay_relay_connection_limit",
        ),
        (
            "vnc_relay.admin_connection.limit",
            "vnc_relay_admin_connection_limit",
        ),
    ] {
        assert_eq!(
            json_family(&body, otel),
            family(&second, prometheus),
            "{otel}"
        );
    }

    // No private value reaches either body (OPS-091).
    let json_text = body.to_string();
    for private in [
        PAIR_ID,
        WAITING_ID,
        REFUSED_TARGET,
        target_address.as_str(),
        "127.0.0.1",
        "alice",
        "bob",
    ] {
        assert!(!text.contains(private), "/metrics contains {private}");
        assert!(
            !json_text.contains(private),
            "/api/v1/metrics contains {private}"
        );
    }
    let allowed = [
        "listener",
        "stage",
        "reason",
        "mode",
        "role",
        "transport",
        "viewer_transport",
        "target_transport",
        "close_cause",
        "direction",
        "result",
        "scope",
        "trigger",
        "cpu_mode",
        "service_name",
        "service_version",
        "le",
    ];
    for (name, labels) in samples.keys() {
        for (label, _) in labels {
            assert!(
                allowed.contains(&label.as_str()),
                "{name} has label {label}"
            );
        }
    }
}

/// spec: operations.md:OPS-082 partial
/// spec: operations.md:OPS-092 partial
/// spec: management.md:MGT-057 partial
/// spec: management.md:MGT-058 partial
/// spec: management.md:MGT-061 partial
#[test]
fn metrics_stay_off_the_status_listener_until_a_restart_with_the_key() {
    let server = Server::start("metrics-off", None);

    let reply = status_get(&server, "/metrics");
    assert_eq!(reply.status, 404);
    let error: Value = must(serde_json::from_str(&reply.body));
    assert_eq!(error["error"]["code"], "not_found");
    assert_eq!(status_get(&server, "/livez").status, 200);
    assert_eq!(status_get(&server, "/readyz").status, 200);

    // The management API answers with metrics whatever the key says.
    let admin = server.login();
    let body = metrics_json(&server, &admin);
    assert!(
        body["metrics"]
            .as_array()
            .is_some_and(|metrics| !metrics.is_empty())
    );
    assert_eq!(body["history"]["interval_secs"], 10);
    assert_eq!(body["history"]["capacity"], 360);
    // The first point is taken at ready, in both windows.
    assert_eq!(body["history"]["points"][0]["seq"], 0);
    let day = server.query(&admin, "/api/v1/metrics?history_window=24h");
    assert_eq!(day["history"]["interval_secs"], 60);
    assert_eq!(day["history"]["capacity"], 1440);
    assert_eq!(day["history"]["points"][0]["seq"], 0);
    assert_eq!(
        day["history"]["points"][0]["at"],
        body["history"]["points"][0]["at"]
    );

    let mut config = server.config_text();
    config.push_str(METRICS_ON);
    server.config_write(config);
    wait_for("restart required", LIMIT, || {
        events(&server).iter().any(|event| {
            event["event"] == "reload_restart_required"
                && event["config_keys"]
                    .as_array()
                    .is_some_and(|keys| keys.iter().any(|key| key == "metrics.enabled"))
        })
    });
    assert_eq!(status_get(&server, "/metrics").status, 404);
    drop(server);

    let restarted = Server::start_custom("metrics-on", None, false, METRICS_ON);
    let reply = status_get(&restarted, "/metrics");
    assert_eq!(reply.status, 200);
    assert_eq!(
        header(&reply, "content-type"),
        Some("text/plain; version=0.0.4; charset=utf-8")
    );
    let admin = restarted.login();
    let body = metrics_json(&restarted, &admin);
    assert_eq!(body["history"]["points"][0]["seq"], 0);
}
