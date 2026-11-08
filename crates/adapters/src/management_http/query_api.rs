//! Management query routes (management-api-v1 §1.6–§1.8, §1.10).
//!
//! Answers `/api/v1/summary`, `/api/v1/waiting`, `/api/v1/sessions`,
//! `/api/v1/sessions/{session_id}`, and `/api/v1/closed-sessions`.

use std::net::SocketAddr;

use axum::body::Body;
use axum::http::request::Parts;
use axum::http::{Method, StatusCode};
use axum::response::Response;
use serde_json::json;
use vnc_relay_app::management::query::{
    ActiveSessionItemView, ClosedSessionsView, SessionDetailView, SessionListView,
    SessionQueryError, SummaryView, WaitingListView,
};
use vnc_relay_app::management::session::AuthorizedAdmin;

use super::session_api::{media_type_allows, poll_flag};
use super::{Shared, json_error, json_response, method_not_allowed, not_found};
use crate::clock::timestamp;

const SUMMARY_PATH: &str = "/api/v1/summary";
const WAITING_PATH: &str = "/api/v1/waiting";
const SESSIONS_PATH: &str = "/api/v1/sessions";
const SESSIONS_PREFIX: &str = "/api/v1/sessions/";
const CLOSED_SESSIONS_PATH: &str = "/api/v1/closed-sessions";

const POLL_MESSAGE: &str = "X-Relay-Poll은 값 1을 가진 GET 요청에만 쓸 수 있다.";
const MEDIA_TYPE_MESSAGE: &str = "요청 본문은 application/json이어야 한다.";
const NOT_FOUND_SESSION_MESSAGE: &str = "지정한 세션을 찾을 수 없습니다.";
const INVALID_SESSION_ID_MESSAGE: &str = "세션 식별자 형식이 올바르지 않습니다.";

/// Formats the canonical socket IP, using the same rule as `PeerIp`.
fn format_ip(addr: &SocketAddr) -> String {
    addr.ip().to_canonical().to_string()
}

fn format_address(addr: &SocketAddr) -> serde_json::Value {
    json!({
        "ip": format_ip(addr),
        "port": addr.port(),
    })
}

pub(super) async fn answer(
    shared: &Shared,
    admin: &mut AuthorizedAdmin,
    parts: Parts,
    body: Body,
) -> Response {
    let path = parts.uri.path();
    let is_query_path = path == SUMMARY_PATH
        || path == WAITING_PATH
        || path == SESSIONS_PATH
        || path == CLOSED_SESSIONS_PATH
        || path
            .strip_prefix(SESSIONS_PREFIX)
            .is_some_and(|id| !id.contains('/'));

    if !is_query_path {
        return not_found();
    }

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

    if path == SUMMARY_PATH {
        return summary_response(&shared.query.summary());
    }
    if path == WAITING_PATH {
        return waiting_response(&shared.query.waiting());
    }
    if path == SESSIONS_PATH {
        return sessions_response(&shared.query.sessions());
    }
    if path == CLOSED_SESSIONS_PATH {
        return closed_sessions_response(&shared.query.closed_sessions());
    }
    if let Some(rest) = path.strip_prefix(SESSIONS_PREFIX) {
        if rest.contains('/') {
            return not_found();
        }
        return match shared.query.session_detail(rest) {
            Ok(detail) => session_detail_response(&detail),
            Err(SessionQueryError::InvalidId) => json_error(
                StatusCode::BAD_REQUEST,
                "invalid_request",
                INVALID_SESSION_ID_MESSAGE,
            ),
            Err(SessionQueryError::NotFound) => json_error(
                StatusCode::NOT_FOUND,
                "not_found",
                NOT_FOUND_SESSION_MESSAGE,
            ),
        };
    }

    not_found()
}

fn summary_response(view: &SummaryView) -> Response {
    // Design §7: counts and budgets have no failure path; the contract requires the unavailable_reason fields, which are null when current is present.
    let body = json!({
        "as_of": timestamp(view.as_of),
        "server": {
            "state": view.server.state,
            "uptime_ms": view.server.uptime_ms,
        },
        "counts": {
            "waiting": view.counts.waiting,
            "active": view.counts.active,
            "handshaking": view.counts.handshaking,
            "handshaking_unavailable_reason": serde_json::Value::Null,
            "target_connecting": view.counts.target_connecting,
            "target_connecting_unavailable_reason": serde_json::Value::Null,
        },
        "usage": {
            "relay_connections": {
                "current": view.usage.relay_connections.current,
                "current_unavailable_reason": serde_json::Value::Null,
                "limit": view.usage.relay_connections.limit,
            },
            "admin_connections": {
                "current": view.usage.admin_connections.current,
                "current_unavailable_reason": serde_json::Value::Null,
                "limit": view.usage.admin_connections.limit,
            },
        },
    });
    json_response(StatusCode::OK, &body)
}

fn waiting_response(view: &WaitingListView) -> Response {
    let items: Vec<_> = view
        .items
        .iter()
        .map(|item| {
            json!({
                "role": item.role,
                "relay_id": item.relay_id,
                "connected_at": timestamp(item.connected_at),
                "waited_ms": item.waited_ms,
            })
        })
        .collect();
    let body = json!({
        "as_of": timestamp(view.as_of),
        "items": items,
    });
    json_response(StatusCode::OK, &body)
}

fn session_item_json(item: &ActiveSessionItemView) -> serde_json::Value {
    json!({
        "session_id": item.session_id,
        "state": item.state,
        "mode": item.mode,
        "viewer_transport": item.viewer_transport,
        "target_transport": item.target_transport,
        "started_at": timestamp(item.started_at),
        "bytes_viewer_to_target": item.bytes_viewer_to_target,
        "bytes_target_to_viewer": item.bytes_target_to_viewer,
        "viewer_address": format_address(&item.viewer_address),
        "target_address": format_address(&item.target_address),
        "viewer_forwarded_ip": item.viewer_forwarded_ip.map(|ip| ip.to_string()),
        "target_forwarded_ip": item.target_forwarded_ip.map(|ip| ip.to_string()),
    })
}

fn sessions_response(view: &SessionListView) -> Response {
    let items: Vec<_> = view.items.iter().map(session_item_json).collect();
    let body = json!({
        "as_of": timestamp(view.as_of),
        "items": items,
    });
    json_response(StatusCode::OK, &body)
}

fn session_detail_response(view: &SessionDetailView) -> Response {
    let body = json!({
        "as_of": timestamp(view.as_of),
        "session": session_item_json(&view.session),
    });
    json_response(StatusCode::OK, &body)
}

fn closed_sessions_response(view: &ClosedSessionsView) -> Response {
    let items: Vec<_> = view
        .items
        .iter()
        .map(|item| {
            json!({
                "session_id": item.session_id,
                "mode": item.mode,
                "viewer_transport": item.viewer_transport,
                "target_transport": item.target_transport,
                "started_at": timestamp(item.started_at),
                "ended_at": timestamp(item.ended_at),
                "bytes_viewer_to_target": item.bytes_viewer_to_target,
                "bytes_target_to_viewer": item.bytes_target_to_viewer,
                "close_cause": item.close_cause,
            })
        })
        .collect();
    let body = json!({
        "as_of": timestamp(view.as_of),
        "max_entries": view.max_entries,
        "items": items,
    });
    json_response(StatusCode::OK, &body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use vnc_relay_app::management::query::ClosedSessionItemView;
    use vnc_relay_domain::peer_ip::PeerIp;

    async fn response_json(response: Response) -> serde_json::Value {
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .expect("response body");
        serde_json::from_slice(&bytes).expect("response JSON")
    }

    fn assert_forwarded_fields(
        item: &serde_json::Value,
        viewer_ip: Option<PeerIp>,
        target_ip: Option<PeerIp>,
    ) {
        assert_eq!(
            item.get("viewer_forwarded_ip"),
            Some(&json!(viewer_ip.map(|ip| ip.to_string())))
        );
        assert_eq!(
            item.get("target_forwarded_ip"),
            Some(&json!(target_ip.map(|ip| ip.to_string())))
        );
        assert!(item.get("viewer_forwarded_ip_unavailable_reason").is_none());
        assert!(item.get("target_forwarded_ip_unavailable_reason").is_none());
    }

    /// spec: management.md:MGT-051 partial
    /// spec: websocket.md:WS-032 partial
    #[tokio::test]
    async fn session_outputs_keep_socket_addresses_and_required_nullable_forwarded_ips() {
        let forwarded_v4 = PeerIp::new("::ffff:192.0.2.10".parse().unwrap());
        let forwarded_v6 = PeerIp::new("2001:0DB8::20".parse().unwrap());
        let endpoints = [
            ("tcp", None),
            ("ws", None),
            ("wss", None),
            ("ws", Some(forwarded_v4)),
            ("wss", Some(forwarded_v6)),
        ];
        for mode in ["mode1", "mode2"] {
            for &(viewer_transport, viewer_ip) in &endpoints {
                for &(target_transport, target_ip) in &endpoints {
                    if mode == "mode1" && (target_transport != "tcp" || target_ip.is_some()) {
                        continue;
                    }
                    let item = ActiveSessionItemView {
                        session_id: "ses_aaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
                        state: "active",
                        mode,
                        viewer_transport,
                        target_transport,
                        started_at: SystemTime::UNIX_EPOCH,
                        bytes_viewer_to_target: 10,
                        bytes_target_to_viewer: 20,
                        viewer_address: "[::ffff:198.51.100.1]:5901".parse().unwrap(),
                        target_address: "[2001:db8::2]:5902".parse().unwrap(),
                        viewer_forwarded_ip: viewer_ip,
                        target_forwarded_ip: target_ip,
                    };
                    let detail = response_json(session_detail_response(&SessionDetailView {
                        as_of: SystemTime::UNIX_EPOCH,
                        session: item.clone(),
                    }))
                    .await;
                    let list = response_json(sessions_response(&SessionListView {
                        as_of: SystemTime::UNIX_EPOCH,
                        items: vec![item],
                    }))
                    .await;
                    assert_eq!(list["items"][0], detail["session"]);
                    let active = &detail["session"];
                    assert_forwarded_fields(active, viewer_ip, target_ip);
                    assert_eq!(
                        active["viewer_address"],
                        json!({"ip": "198.51.100.1", "port": 5901})
                    );
                    assert_eq!(
                        active["target_address"],
                        json!({"ip": "2001:db8::2", "port": 5902})
                    );
                    let closed = response_json(closed_sessions_response(&ClosedSessionsView {
                        as_of: SystemTime::UNIX_EPOCH,
                        max_entries: 1,
                        items: vec![ClosedSessionItemView {
                            session_id: "ses_aaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
                            mode,
                            viewer_transport,
                            target_transport,
                            started_at: SystemTime::UNIX_EPOCH,
                            ended_at: SystemTime::UNIX_EPOCH,
                            bytes_viewer_to_target: 10,
                            bytes_target_to_viewer: 20,
                            close_cause: "peer_eof",
                        }],
                    }))
                    .await;
                    assert_eq!(closed["items"][0]["target_transport"], target_transport);
                    assert!(closed["items"][0].get("viewer_forwarded_ip").is_none());
                    assert!(closed["items"][0].get("target_forwarded_ip").is_none());
                }
            }
        }
    }

    /// spec: management.md:MGT-015 partial
    #[test]
    fn formats_ipv4_mapped_ipv6_as_ipv4_without_changing_port() {
        let addr: SocketAddr = "[::ffff:192.0.2.1]:5901"
            .parse()
            .unwrap_or_else(|e| panic!("parse address: {e}"));

        assert_eq!(
            format_address(&addr),
            json!({"ip": "192.0.2.1", "port": 5901})
        );
    }

    /// spec: management.md:MGT-015 partial
    #[test]
    fn preserves_plain_ipv6_and_port() {
        let addr: SocketAddr = "[2001:db8::5]:5902"
            .parse()
            .unwrap_or_else(|e| panic!("parse address: {e}"));

        assert_eq!(
            format_address(&addr),
            json!({"ip": "2001:db8::5", "port": 5902})
        );
    }

    /// spec: management.md:MGT-015 partial
    #[test]
    fn preserves_plain_ipv4_and_port() {
        let addr: SocketAddr = "192.0.2.5:5903"
            .parse()
            .unwrap_or_else(|e| panic!("parse address: {e}"));

        assert_eq!(
            format_address(&addr),
            json!({"ip": "192.0.2.5", "port": 5903})
        );
    }
}
