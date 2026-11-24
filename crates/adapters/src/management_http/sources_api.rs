//! Management API: `GET /api/v1/sources` (management-api-v1 §1.17).
//! Design: docs/architectures/source-stats.md §4.2.

use axum::body::Body;
use axum::http::request::Parts;
use axum::http::{Method, StatusCode};
use axum::response::Response;
use serde_json::{Map, Value, json};
use vnc_relay_application::ip_info::IpDbStatus;
use vnc_relay_application::management::session::AuthorizedAdmin;
use vnc_relay_application::sources::SourceWindow;
use vnc_relay_application::sources::report::{AddressItem, AsnItem, CountryItem, Counts, Report};
use vnc_relay_domain::peer_ip::PeerIp;

use super::Shared;
use super::logs_api;
use super::session_api;
use crate::clock;

const POLL_MESSAGE: &str = "X-Relay-Poll은 값 1을 가진 GET 요청에만 쓸 수 있다.";
const MEDIA_TYPE_MESSAGE: &str = "요청 본문은 application/json이어야 한다.";
const ENCODING_MESSAGE: &str = "잘못된 퍼센트 인코딩이거나 UTF-8 형식이 아닙니다.";

/// Checks in the order of the other GET routes (method, polling header,
/// body, query), then counts the kept events of the window. An automatic
/// refresh (`X-Relay-Poll: 1`) does not extend the session (MGT-R16).
pub(super) async fn answer(
    shared: &Shared,
    admin: &mut AuthorizedAdmin,
    parts: Parts,
    body: Body,
) -> Response {
    if parts.method != Method::GET {
        return super::method_not_allowed("GET", "이 경로는 GET 요청만 받는다.");
    }
    match session_api::poll_flag(&parts.headers) {
        None => return super::json_error(StatusCode::BAD_REQUEST, "invalid_request", POLL_MESSAGE),
        Some(false) => shared.auth.touch(admin),
        Some(true) => {}
    }
    if !session_api::media_type_allows(body, &parts.headers).await {
        return super::json_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "invalid_request",
            MEDIA_TYPE_MESSAGE,
        );
    }
    let window = match window_query(parts.uri.query()) {
        Ok(window) => window,
        Err(message) => {
            return super::json_error(StatusCode::BAD_REQUEST, "invalid_request", message);
        }
    };
    // Counting up to 100,000 events takes a few milliseconds; it runs on a
    // blocking thread so the management runtime keeps answering.
    let sources = std::sync::Arc::clone(&shared.sources);
    match tokio::task::spawn_blocking(move || sources.report(window)).await {
        Ok(report) => super::json_response(StatusCode::OK, &sources_json(&report)),
        Err(_) => super::json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal",
            "접속 출처를 집계하지 못했다.",
        ),
    }
}

/// `window` of management-api-v1 §1.17, at most once: `1h` (the default) or
/// `24h`, case-sensitive. Pairs are decoded as in §1.15; other parameters
/// are ignored.
fn window_query(query: Option<&str>) -> Result<SourceWindow, &'static str> {
    let mut window = None;
    for pair in query.unwrap_or_default().split('&') {
        if pair.is_empty() {
            continue;
        }
        let (raw_key, raw_value) = pair.split_once('=').unwrap_or((pair, ""));
        let key = logs_api::percent_decode(raw_key).ok_or(ENCODING_MESSAGE)?;
        if key != "window" {
            continue;
        }
        let value = logs_api::percent_decode(raw_value).ok_or(ENCODING_MESSAGE)?;
        if window.is_some() {
            return Err("window 매개변수가 중복되었습니다.");
        }
        window = Some(match value.as_str() {
            "1h" => SourceWindow::Hour,
            "24h" => SourceWindow::Day,
            _ => return Err("window는 1h 또는 24h여야 합니다."),
        });
    }
    Ok(window.unwrap_or(SourceWindow::Hour))
}

/// The `200` body of management-api-v1 §1.17.
fn sources_json(report: &Report) -> Value {
    let [country, asn] = &report.databases;
    let mut body = Map::new();
    body.insert(
        "as_of".to_owned(),
        Value::from(clock::timestamp(report.as_of)),
    );
    body.insert("window".to_owned(), Value::from(report.window.as_str()));
    body.insert(
        "window_start".to_owned(),
        Value::from(clock::timestamp(report.window_start)),
    );
    body.insert("capacity".to_owned(), Value::from(report.capacity));
    body.insert("stored".to_owned(), Value::from(report.stored));
    body.insert(
        "dropped_before".to_owned(),
        report
            .dropped_before
            .map_or(Value::Null, |at| Value::from(clock::timestamp(at))),
    );
    body.insert(
        "deny_editable".to_owned(),
        Value::from(report.deny_editable),
    );
    body.insert(
        "databases".to_owned(),
        json!({ "country": database_json(country), "asn": database_json(asn) }),
    );
    body.insert(
        "totals".to_owned(),
        Value::Object(counts_json(&report.totals)),
    );
    body.insert(
        "addresses".to_owned(),
        listing(
            report.addresses.total,
            report.addresses.items.iter().map(address_json),
        ),
    );
    body.insert(
        "countries".to_owned(),
        listing(
            report.countries.total,
            report.countries.items.iter().map(country_json),
        ),
    );
    body.insert(
        "asns".to_owned(),
        listing(report.asns.total, report.asns.items.iter().map(asn_json)),
    );
    Value::Object(body)
}

fn database_json(status: &IpDbStatus) -> Value {
    let in_use = status.in_use.as_ref();
    json!({
        "state": status.state.as_str(),
        "error": status.error.map(|failure| failure.as_str()),
        "database_type": in_use.map(|db| db.metadata.database_type.as_str()),
        "provider": in_use.map(|db| db.metadata.provider.as_str()),
        "build_time": in_use.map(|db| clock::timestamp(db.metadata.build_time)),
        "loaded_at": in_use.map(|db| clock::timestamp(db.loaded_at)),
    })
}

fn listing(total: usize, items: impl Iterator<Item = Value>) -> Value {
    json!({ "total": total, "items": Value::Array(items.collect()) })
}

fn counts_json(counts: &Counts) -> Map<String, Value> {
    let mut object = Map::new();
    object.insert("attempts".to_owned(), Value::from(counts.attempts));
    object.insert("rejected".to_owned(), Value::from(counts.rejected));
    object.insert("aborted".to_owned(), Value::from(counts.aborted));
    object.insert("sessions".to_owned(), Value::from(counts.sessions));
    object
}

fn address_json(item: &AddressItem) -> Value {
    let mut object = Map::new();
    object.insert(
        "ip".to_owned(),
        Value::from(PeerIp::new(item.ip).to_string()),
    );
    object.insert("local".to_owned(), Value::from(item.local));
    object.insert(
        "country".to_owned(),
        item.country
            .map_or(Value::Null, |code| Value::from(code.as_str())),
    );
    object.insert(
        "asn".to_owned(),
        item.asn.map_or(Value::Null, |asn| Value::from(asn.get())),
    );
    object.insert(
        "as_name".to_owned(),
        item.as_name.as_deref().map_or(Value::Null, Value::from),
    );
    object.extend(counts_json(&item.counts));
    object.insert(
        "last_seen_at".to_owned(),
        Value::from(clock::timestamp(item.last_seen_at)),
    );
    object.insert(
        "deny_entries".to_owned(),
        Value::from(item.deny_entries.clone()),
    );
    Value::Object(object)
}

fn country_json(item: &CountryItem) -> Value {
    let mut object = Map::new();
    object.insert(
        "country".to_owned(),
        item.country
            .map_or(Value::Null, |code| Value::from(code.as_str())),
    );
    object.insert("addresses".to_owned(), Value::from(item.addresses));
    object.extend(counts_json(&item.counts));
    Value::Object(object)
}

fn asn_json(item: &AsnItem) -> Value {
    let mut object = Map::new();
    object.insert(
        "asn".to_owned(),
        item.asn.map_or(Value::Null, |asn| Value::from(asn.get())),
    );
    object.insert(
        "as_name".to_owned(),
        item.as_name.as_deref().map_or(Value::Null, Value::from),
    );
    object.insert("addresses".to_owned(), Value::from(item.addresses));
    object.extend(counts_json(&item.counts));
    Value::Object(object)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// spec: management.md:MGT-062 partial
    #[test]
    fn window_is_one_of_two_case_sensitive_values_given_once() {
        assert_eq!(window_query(None), Ok(SourceWindow::Hour));
        assert_eq!(window_query(Some("window=24h")), Ok(SourceWindow::Day));
        assert_eq!(
            window_query(Some("w%69ndow=1h&other=x")),
            Ok(SourceWindow::Hour)
        );
        for bad in [
            "window=1H",
            "window=7d",
            "window=",
            "window",
            "window=1h&window=1h",
        ] {
            assert!(window_query(Some(bad)).is_err(), "{bad}");
        }
        assert_eq!(window_query(Some("window=%ZZ")), Err(ENCODING_MESSAGE));
    }
}
