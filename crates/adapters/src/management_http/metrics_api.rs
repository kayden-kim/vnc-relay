//! Management API: `GET /api/v1/metrics` (management-api-v1 §1.16). The body
//! is the JSON of metrics-v1 §1.7 with the history of §1.8.
//! Design: docs/architectures/metrics.md §4.

use axum::body::Body;
use axum::http::request::Parts;
use axum::http::{Method, StatusCode};
use axum::response::Response;
use serde_json::{Map, Value, json};
use vnc_relay_app::management::session::AuthorizedAdmin;
use vnc_relay_app::metrics::history::{
    HISTORY_CAPACITY, HISTORY_INTERVAL, HISTORY_KEYS, HistoryPoint, HistoryView,
};
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
/// body, query), then answers with one snapshot and the history after
/// `history_after`. An automatic refresh (`X-Relay-Poll: 1`) does not extend
/// the session (MGT-R16).
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
    let after = match history_after(parts.uri.query()) {
        Ok(after) => after,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, "invalid_request", message),
    };
    let snapshot = shared.metrics.snapshot();
    let history = shared.metrics.history_after(after);
    json_response(StatusCode::OK, &metrics_json(&snapshot, &history))
}

/// `history_after` of management-api-v1 §1.16: `[0-9]+` within `u64`, at
/// most once. Every pair is decoded as in §1.15; other parameters are
/// ignored.
fn history_after(query: Option<&str>) -> Result<Option<u64>, &'static str> {
    let mut after = None;
    for pair in query.unwrap_or_default().split('&') {
        if pair.is_empty() {
            continue;
        }
        let (raw_key, raw_value) = pair.split_once('=').unwrap_or((pair, ""));
        let key = percent_decode(raw_key).ok_or(ENCODING_MESSAGE)?;
        let value = percent_decode(raw_value).ok_or(ENCODING_MESSAGE)?;
        if key != "history_after" {
            continue;
        }
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
    Ok(after)
}

/// The body of metrics-v1 §1.7.
fn metrics_json(snapshot: &MetricsSnapshot, history: &HistoryView) -> Value {
    json!({
        "as_of": timestamp(snapshot.as_of),
        "resource": {
            "service.name": "vnc-relay",
            "service.version": snapshot.service_version,
        },
        "metrics": snapshot.families.iter().map(family_json).collect::<Vec<_>>(),
        "history": {
            "interval_secs": HISTORY_INTERVAL.as_secs(),
            "capacity": HISTORY_CAPACITY,
            "latest_seq": history.latest_seq,
            "points": history.points.iter().map(history_point_json).collect::<Vec<_>>(),
        },
    })
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
    json!({
        "seq": point.seq,
        "at": timestamp(point.at),
        "values": values,
    })
}

fn number_json(number: Number) -> Value {
    match number {
        Number::Int(value) => Value::from(value),
        Number::Float(value) => Value::from(value),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use vnc_relay_app::metrics::Metrics;
    use vnc_relay_app::ports::{CpuTimes, OpenCount, ProcessStats};

    use super::*;
    use crate::clock::SystemClock;

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
        let body = metrics_json(&service.snapshot(), &service.history_after(None));

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
    fn history_after_accepts_only_one_unsigned_decimal() {
        assert_eq!(history_after(None), Ok(None));
        assert_eq!(history_after(Some("")), Ok(None));
        assert_eq!(history_after(Some("history_after=0")), Ok(Some(0)));
        assert_eq!(
            history_after(Some("other=x&history_after=18446744073709551615")),
            Ok(Some(u64::MAX))
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
        ] {
            assert!(history_after(Some(bad)).is_err(), "{bad}");
        }
    }
}
