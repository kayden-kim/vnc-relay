//! Management API: `GET /api/v1/metrics` (management-api-v1 §1.16). The body
//! is the JSON of metrics-v1 §1.7 with the history of §1.8.
//! Design: docs/architectures/metrics.md §4.

use axum::body::Body;
use axum::http::request::Parts;
use axum::http::{Method, StatusCode};
use axum::response::Response;
use serde_json::{Map, Value, json};
use vnc_relay_app::management::session::AuthorizedAdmin;
use vnc_relay_app::metrics::history::{HISTORY_KEYS, HistoryPoint, HistoryView, HistoryWindow};
use vnc_relay_app::metrics::snapshot::{
    MetricFamily, MetricKind, MetricPoint, MetricsSnapshot, Number, PointValue,
};

use super::logs_api::percent_decode;
use super::session_api::{media_type_allows, poll_flag};
use super::{Shared, json_error, json_response, method_not_allowed};
use crate::clock::timestamp;

const POLL_MESSAGE: &str = "X-Relay-Poll은 값 1을 가진 GET 요청에만 쓸 수 있다.";
const MEDIA_TYPE_MESSAGE: &str = "요청 본문은 application/json이어야 한다.";
const ENCODING_MESSAGE: &str = "잘못된 퍼센트 인코딩이거나 UTF-8 형식이 아닙니다.";

/// Checks in the order of the other GET routes (method, polling header,
/// body, query), then answers with one snapshot and the history of the
/// chosen window after `history_after`. An automatic refresh
/// (`X-Relay-Poll: 1`) does not extend the session (MGT-R16).
pub(super) async fn answer(
    shared: &Shared,
    admin: &mut AuthorizedAdmin,
    parts: Parts,
    body: Body,
) -> Response {
    if parts.method != Method::GET {
        return method_not_allowed("GET", "이 경로는 GET 요청만 받는다.");
    }
    match poll_flag(&parts.headers) {
        None => return json_error(StatusCode::BAD_REQUEST, "invalid_request", POLL_MESSAGE),
        Some(false) => shared.auth.touch(admin),
        Some(true) => {}
    }
    if !media_type_allows(body, &parts.headers).await {
        return json_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "invalid_request",
            MEDIA_TYPE_MESSAGE,
        );
    }
    let (window, after) = match history_query(parts.uri.query()) {
        Ok(query) => query,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, "invalid_request", message),
    };
    let snapshot = shared.metrics.snapshot();
    let history = shared.metrics.history_after(window, after);
    json_response(StatusCode::OK, &metrics_json(&snapshot, window, &history))
}

/// `history_window` and `history_after` of management-api-v1 §1.16, each at
/// most once: the window is `1h` (the default) or `24h`, case-sensitive;
/// `history_after` is `[0-9]+` within `u64`. Every pair is decoded as in
/// §1.15; other parameters are ignored.
fn history_query(query: Option<&str>) -> Result<(HistoryWindow, Option<u64>), &'static str> {
    let mut window = None;
    let mut after = None;
    for pair in query.unwrap_or_default().split('&') {
        if pair.is_empty() {
            continue;
        }
        let (raw_key, raw_value) = pair.split_once('=').unwrap_or((pair, ""));
        let key = percent_decode(raw_key).ok_or(ENCODING_MESSAGE)?;
        let value = percent_decode(raw_value).ok_or(ENCODING_MESSAGE)?;
        match key.as_str() {
            "history_window" => {
                if window.is_some() {
                    return Err("history_window 매개변수가 중복되었습니다.");
                }
                window = Some(match value.as_str() {
                    "1h" => HistoryWindow::Hour,
                    "24h" => HistoryWindow::Day,
                    _ => return Err("history_window는 1h 또는 24h여야 합니다."),
                });
            }
            "history_after" => {
                if after.is_some() {
                    return Err("history_after 매개변수가 중복되었습니다.");
                }
                if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err("history_after는 부호 없는 10진수여야 합니다.");
                }
                after = Some(
                    value
                        .parse::<u64>()
                        .map_err(|_| "history_after 값이 범위를 벗어났습니다.")?,
                );
            }
            _ => {}
        }
    }
    Ok((window.unwrap_or(HistoryWindow::Hour), after))
}

/// The body of metrics-v1 §1.7. The history, up to 1440 points, moves into
/// the body instead of being copied by `json!`.
fn metrics_json(snapshot: &MetricsSnapshot, window: HistoryWindow, history: &HistoryView) -> Value {
    let mut history_object = Map::new();
    history_object.insert(
        "interval_secs".to_owned(),
        Value::from(window.interval().as_secs()),
    );
    history_object.insert("capacity".to_owned(), Value::from(window.capacity()));
    history_object.insert("latest_seq".to_owned(), Value::from(history.latest_seq));
    history_object.insert(
        "points".to_owned(),
        Value::Array(history.points.iter().map(history_point_json).collect()),
    );
    let mut body = Map::new();
    body.insert("as_of".to_owned(), Value::from(timestamp(snapshot.as_of)));
    body.insert(
        "resource".to_owned(),
        json!({
            "service.name": "vnc-relay",
            "service.version": snapshot.service_version,
        }),
    );
    body.insert(
        "metrics".to_owned(),
        Value::Array(snapshot.families.iter().map(family_json).collect()),
    );
    body.insert("history".to_owned(), Value::Object(history_object));
    Value::Object(body)
}

fn family_json(family: &MetricFamily) -> Value {
    let mut object = Map::new();
    object.insert("name".to_owned(), Value::from(family.name));
    object.insert("type".to_owned(), Value::from(family.kind.as_str()));
    object.insert("unit".to_owned(), Value::from(family.unit));
    object.insert("available".to_owned(), Value::from(family.available));
    if family.kind == MetricKind::Histogram {
        object.insert("bounds".to_owned(), Value::from(family.bounds));
    }
    object.insert(
        "points".to_owned(),
        Value::Array(family.points.iter().map(point_json).collect()),
    );
    Value::Object(object)
}

fn point_json(point: &MetricPoint) -> Value {
    let attributes: Map<String, Value> = point
        .attributes
        .iter()
        .map(|(name, value)| ((*name).to_owned(), Value::from(&**value)))
        .collect();
    match &point.value {
        PointValue::Number(number) => json!({
            "attributes": attributes,
            "value": number_json(*number),
        }),
        PointValue::Histogram {
            count,
            sum,
            bucket_counts,
        } => json!({
            "attributes": attributes,
            "count": count,
            "sum": sum,
            "bucket_counts": bucket_counts,
        }),
    }
}

fn history_point_json(point: &HistoryPoint) -> Value {
    let values: Map<String, Value> = HISTORY_KEYS
        .iter()
        .zip(&point.values)
        .map(|(key, value)| ((*key).to_owned(), value.map_or(Value::Null, number_json)))
        .collect();
    let mut object = Map::new();
    object.insert("seq".to_owned(), Value::from(point.seq));
    object.insert("at".to_owned(), Value::from(timestamp(point.at)));
    object.insert("values".to_owned(), Value::Object(values));
    Value::Object(object)
}

fn number_json(number: Number) -> Value {
    match number {
        Number::Int(value) => Value::from(value),
        Number::Float(value) => Value::from(value),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, SystemTime};

    use vnc_relay_app::metrics::Metrics;
    use vnc_relay_app::ports::{Clock, CpuTimes, OpenCount, ProcessStats};
    use vnc_relay_domain::session::Direction;

    use super::*;
    use crate::clock::SystemClock;

    /// Wall time the test sets; monotonic time stays at zero.
    struct SetWall(Mutex<SystemTime>);

    impl Clock for SetWall {
        fn monotonic(&self) -> Duration {
            Duration::ZERO
        }

        fn wall(&self) -> SystemTime {
            *self.0.lock().expect("clock lock")
        }
    }

    /// spec: management.md:MGT-060 partial
    #[test]
    fn an_unreadable_value_is_unavailable_without_points_and_null_in_history() {
        let process = ProcessStats {
            cpu: Some(CpuTimes {
                user: Duration::from_millis(1500),
                system: Duration::from_millis(250),
            }),
            memory_usage: None,
            memory_virtual: Some(104_857_600),
            threads: Some(9),
            open: OpenCount::Handles(Some(42)),
        };
        let metrics = Arc::new(Metrics::new(&SystemClock::new()));
        let service = crate::test_support::metrics_service(metrics, process);
        service.sample_history();
        let body = metrics_json(
            &service.snapshot(),
            HistoryWindow::Hour,
            &service.history_after(HistoryWindow::Hour, None),
        );

        let metric = |name: &str| {
            body["metrics"]
                .as_array()
                .and_then(|metrics| metrics.iter().find(|metric| metric["name"] == name))
                .cloned()
                .unwrap_or_else(|| panic!("metric {name}"))
        };
        let unreadable = metric("process.memory.usage");
        assert_eq!(unreadable["available"], false);
        assert_eq!(unreadable["points"], json!([]));
        for name in ["process.memory.virtual", "process.thread.count"] {
            assert_eq!(metric(name)["available"], true, "{name}");
        }
        let unavailable: Vec<&Value> = body["metrics"]
            .as_array()
            .map(|metrics| {
                metrics
                    .iter()
                    .filter(|metric| metric["available"] == false)
                    .collect()
            })
            .unwrap_or_default();
        assert_eq!(unavailable.len(), 1);

        let values = &body["history"]["points"][0]["values"];
        assert_eq!(values["process_memory_usage"], Value::Null);
        assert_eq!(values["process_cpu_time"], json!(1.75));
        assert_eq!(values["session_count"], json!(0));
    }

    /// spec: management.md:MGT-059 partial
    #[test]
    fn history_query_accepts_one_window_and_one_unsigned_decimal() {
        use HistoryWindow::{Day, Hour};
        assert_eq!(history_query(None), Ok((Hour, None)));
        assert_eq!(history_query(Some("")), Ok((Hour, None)));
        assert_eq!(history_query(Some("history_after=0")), Ok((Hour, Some(0))));
        assert_eq!(
            history_query(Some("other=x&history_after=18446744073709551615")),
            Ok((Hour, Some(u64::MAX)))
        );
        assert_eq!(history_query(Some("history_window=1h")), Ok((Hour, None)));
        assert_eq!(
            history_query(Some("history_after=7&history_window=24h")),
            Ok((Day, Some(7)))
        );
        // Pairs are decoded first, as in §1.15.
        assert_eq!(
            history_query(Some("history_window=%32%34h")),
            Ok((Day, None))
        );
        for bad in [
            "history_after=",
            "history_after",
            "history_after=-1",
            "history_after=+1",
            "history_after=%201",
            "history_after=1.0",
            "history_after=1e3",
            "history_after=18446744073709551616",
            "history_after=1&history_after=2",
            "history_after=%ZZ",
            "history_window=",
            "history_window",
            "history_window=1H",
            "history_window=24",
            "history_window=7d",
            "history_window=%201h",
            "history_window=1h&history_window=1h",
            "history_window=1h&history_window=24h",
        ] {
            assert!(history_query(Some(bad)).is_err(), "{bad}");
        }
    }

    /// spec: management.md:MGT-061
    #[test]
    fn history_window_24h_answers_every_sixth_point_of_the_last_day() {
        let start = SystemTime::UNIX_EPOCH + Duration::from_secs(1_790_000_000);
        let clock = Arc::new(SetWall(Mutex::new(start)));
        let metrics = Arc::new(Metrics::new(&SystemClock::new()));
        let process = ProcessStats {
            cpu: None,
            memory_usage: None,
            memory_virtual: None,
            threads: None,
            open: OpenCount::NotSupported,
        };
        let service = crate::test_support::metrics_service_with_clock(
            Arc::clone(&metrics),
            process,
            Arc::clone(&clock) as Arc<dyn Clock>,
        );
        // One-hour point n is taken at 10n seconds after n bytes reached the
        // viewer, so a 24-hour point shows which one-hour point it came from.
        let sample = |n: u64| {
            *clock.0.lock().expect("clock lock") = start + Duration::from_secs(10 * n);
            service.sample_history();
            metrics.add_io(Direction::TargetToViewer, 1);
        };
        let history = |query: &str| {
            let (window, after) = history_query(Some(query)).expect("valid query");
            let mut body = metrics_json(
                &service.snapshot(),
                window,
                &service.history_after(window, after),
            );
            body["history"].take()
        };
        // 24-hour point `seq` is one-hour point 6 × `seq`.
        let check = |history: &Value, index: usize, seq: u64| {
            let point = &history["points"][index];
            assert_eq!(point["seq"], seq, "24-hour point {seq}");
            let at = timestamp(start + Duration::from_secs(60 * seq));
            assert_eq!(point["at"], Value::from(at), "24-hour point {seq}");
            let bytes = &point["values"]["session_io_target_to_viewer"];
            assert_eq!(*bytes, 6 * seq, "24-hour point {seq}");
        };

        for n in 0..7 {
            sample(n);
        }
        let day = history("history_window=24h");
        assert_eq!(day["interval_secs"], 60);
        assert_eq!(day["capacity"], 1440);
        assert_eq!(day["latest_seq"], 1);
        let hour = history("history_window=1h");
        // The first point is one-hour point 0, the next one-hour point 6,
        // with the same time and values.
        for (seq, hour_index) in [(0, 0), (1, 6)] {
            check(&day, seq as usize, seq);
            let mut same = hour["points"][hour_index].clone();
            same["seq"] = Value::from(seq);
            assert_eq!(day["points"][seq as usize], same, "24-hour point {seq}");
        }

        // 6 × 1440 + 1 one-hour points give 1441 24-hour points; the oldest
        // goes.
        for n in 7..8641 {
            sample(n);
        }
        let day = history("history_window=24h");
        assert_eq!(day["latest_seq"], 1440);
        assert_eq!(day["points"].as_array().map(Vec::len), Some(1440));
        for (index, seq) in (1..=1440).enumerate() {
            check(&day, index, seq);
        }
        let newer = history("history_window=24h&history_after=1438");
        check(&newer, 0, 1439);
        check(&newer, 1, 1440);
        assert_eq!(newer["points"].as_array().map(Vec::len), Some(2));
        let none = history("history_after=1440&history_window=24h");
        assert_eq!(none["latest_seq"], 1440);
        assert_eq!(none["points"], json!([]));

        // The one-hour history is the same with `1h` and without the
        // parameter.
        let hour = history("history_window=1h");
        assert_eq!(hour["interval_secs"], 10);
        assert_eq!(hour["capacity"], 360);
        assert_eq!(hour["latest_seq"], 8640);
        assert_eq!(hour["points"].as_array().map(Vec::len), Some(360));
        assert_eq!(history(""), hour);
    }
}
