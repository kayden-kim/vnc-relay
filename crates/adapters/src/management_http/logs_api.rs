//! Management API: `GET /api/v1/logs` (management-api-v1 §1.15).
//!
//! Provides the in-memory recent operation logs buffer, supporting filtering by
//! level, event, value search (`q`), session_id, and directional cursor
//! pagination (`before` / `after`).

use std::collections::BTreeSet;
use std::time::SystemTime;

use axum::body::Body;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::request::Parts;
use axum::http::{HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use vnc_relay_app::management::query::ManagementQueryService;
use vnc_relay_app::management::session::AuthorizedAdmin;

use super::session_api::{media_type_allows, poll_flag};
use super::{Shared, json_error, method_not_allowed};
use crate::ops_log::LogsFilter;

const POLL_MESSAGE: &str = "X-Relay-Poll은 값 1을 가진 GET 요청에만 쓸 수 있다.";
const MEDIA_TYPE_MESSAGE: &str = "요청 본문은 application/json이어야 한다.";

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
        Some(false) => {
            shared.auth.touch(admin);
        }
        Some(true) => {}
    }

    if !media_type_allows(body, &parts.headers).await {
        return json_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "invalid_request",
            MEDIA_TYPE_MESSAGE,
        );
    }

    let filter = match parse_query(parts.uri.query()) {
        Ok(filter) => filter,
        Err(message) => {
            return json_error(StatusCode::BAD_REQUEST, "invalid_request", message);
        }
    };

    let response = shared.logs.query(&filter, SystemTime::now());
    let body = response.to_json_string();
    let mut rep = (StatusCode::OK, body).into_response();
    let headers = rep.headers_mut();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    rep
}

fn parse_query(query_str: Option<&str>) -> Result<LogsFilter, &'static str> {
    let mut filter = LogsFilter {
        limit: 200,
        ..Default::default()
    };
    let Some(query) = query_str else {
        return Ok(filter);
    };
    if query.is_empty() {
        return Ok(filter);
    }

    let mut seen_q = false;
    let mut seen_session_id = false;
    let mut seen_before = false;
    let mut seen_after = false;
    let mut seen_limit = false;

    let mut levels = BTreeSet::new();
    let mut has_levels = false;
    let mut events = BTreeSet::new();
    let mut has_events = false;

    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (raw_key, raw_val) = match pair.split_once('=') {
            Some((k, v)) => (k, v),
            None => (pair, ""),
        };
        let key =
            percent_decode(raw_key).ok_or("잘못된 퍼센트 인코딩이거나 UTF-8 형식이 아닙니다.")?;
        let val =
            percent_decode(raw_val).ok_or("잘못된 퍼센트 인코딩이거나 UTF-8 형식이 아닙니다.")?;

        match key.as_str() {
            "level" => {
                if val.is_empty() {
                    return Err("level 값이 비어 있습니다.");
                }
                for item in val.split(',') {
                    if item.is_empty() {
                        return Err("level 항목이 비어 있습니다.");
                    }
                    if item != "info" && item != "warn" && item != "error" {
                        return Err("지원하지 않는 level 값입니다.");
                    }
                    levels.insert(item.to_owned());
                }
                has_levels = true;
            }
            "event" => {
                if val.is_empty() {
                    return Err("event 값이 비어 있습니다.");
                }
                for item in val.split(',') {
                    if item.is_empty() {
                        return Err("event 항목이 비어 있습니다.");
                    }
                    if item.len() > 64
                        || !item
                            .bytes()
                            .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_'))
                    {
                        return Err("event 형식이 올바르지 않습니다.");
                    }
                    events.insert(item.to_owned());
                }
                has_events = true;
            }
            "q" => {
                if seen_q {
                    return Err("q 매개변수가 중복되었습니다.");
                }
                seen_q = true;
                let count = val.chars().count();
                if !(1..=256).contains(&count) {
                    return Err("q 값의 길이는 1~256자여야 합니다.");
                }
                filter.q = Some(val);
            }
            "session_id" => {
                if seen_session_id {
                    return Err("session_id 매개변수가 중복되었습니다.");
                }
                seen_session_id = true;
                if !ManagementQueryService::is_valid_session_id(&val) {
                    return Err("세션 식별자 형식이 올바르지 않습니다.");
                }
                filter.session_id = Some(val);
            }
            "before" => {
                if seen_before {
                    return Err("before 매개변수가 중복되었습니다.");
                }
                seen_before = true;
                if val.is_empty() || !val.bytes().all(|b| b.is_ascii_digit()) {
                    return Err("before는 부호 없는 10진수여야 합니다.");
                }
                let n: u64 = val
                    .parse()
                    .map_err(|_| "before 값이 범위를 벗어났습니다.")?;
                if n < 1 {
                    return Err("before는 1 이상이어야 합니다.");
                }
                filter.before = Some(n);
            }
            "after" => {
                if seen_after {
                    return Err("after 매개변수가 중복되었습니다.");
                }
                seen_after = true;
                if val.is_empty() || !val.bytes().all(|b| b.is_ascii_digit()) {
                    return Err("after는 부호 없는 10진수여야 합니다.");
                }
                let n: u64 = val.parse().map_err(|_| "after 값이 범위를 벗어났습니다.")?;
                filter.after = Some(n);
            }
            "limit" => {
                if seen_limit {
                    return Err("limit 매개변수가 중복되었습니다.");
                }
                seen_limit = true;
                if val.is_empty() || !val.bytes().all(|b| b.is_ascii_digit()) {
                    return Err("limit는 정수여야 합니다.");
                }
                let n: usize = val.parse().map_err(|_| "limit 값이 범위를 벗어났습니다.")?;
                if !(1..=1000).contains(&n) {
                    return Err("limit는 1~1000 범위여야 합니다.");
                }
                filter.limit = n;
            }
            _ => {
                // 알 수 없는 매개변수는 무시한다 (계약 §3 Producer 호환 규칙).
            }
        }
    }

    if filter.before.is_some() && filter.after.is_some() {
        return Err("before와 after는 동시에 지정할 수 없습니다.");
    }
    if has_levels {
        filter.levels = Some(levels);
    }
    if has_events {
        filter.events = Some(events);
    }

    Ok(filter)
}

fn percent_decode(input: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(input.len());
    let mut iter = input.bytes();
    while let Some(b) = iter.next() {
        if b == b'%' {
            let h1 = iter.next()?;
            let h2 = iter.next()?;
            let v1 = hex_val(h1)?;
            let v2 = hex_val(h2)?;
            bytes.push((v1 << 4) | v2);
        } else if b == b'+' {
            bytes.push(b' ');
        } else {
            bytes.push(b);
        }
    }
    String::from_utf8(bytes).ok()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
