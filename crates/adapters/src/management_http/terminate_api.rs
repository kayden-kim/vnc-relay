//! Validated administrator termination requests (management-api-v1 §1.9).

use axum::body::Body;
use axum::http::request::Parts;
use axum::http::{Method, StatusCode};
use axum::response::Response;
use serde_json::json;
use vnc_relay_app::management::query::ManagementQueryService;
use vnc_relay_app::management::session::AuthorizedAdmin;
use vnc_relay_app::management::terminate::{TerminateError, TerminateView};

use super::{PeerAddr, Shared, json_error, json_response, method_not_allowed};
use crate::clock::timestamp;

const PREFIX: &str = "/api/v1/sessions/";
const SUFFIX: &str = "/terminate";

fn session_id(path: &str) -> Option<&str> {
    path.strip_prefix(PREFIX)
        .and_then(|rest| rest.strip_suffix(SUFFIX))
        .filter(|id| !id.contains('/'))
}

pub(super) fn is_terminate_path(path: &str) -> bool {
    session_id(path).is_some()
}

/// Authentication and POST/DELETE CSRF have already passed. Every remaining
/// validation happens before runtime acceptance, auth touch or U suspension.
pub(super) async fn answer(
    shared: &Shared,
    peer: &PeerAddr,
    admin: &mut AuthorizedAdmin,
    parts: Parts,
    body: Body,
) -> Response {
    let Some(id) = session_id(parts.uri.path()) else {
        return super::not_found();
    };
    if parts.method != Method::POST {
        return method_not_allowed("POST", "이 경로는 POST 요청만 받는다.");
    }
    if parts.headers.contains_key("x-relay-poll") {
        return invalid_request("X-Relay-Poll은 값 1을 가진 GET 요청에만 쓸 수 있다.");
    }
    if !ManagementQueryService::is_valid_session_id(id) {
        return invalid_request("세션 식별자 형식이 올바르지 않습니다.");
    }
    // A zero-byte limit rejects the first actual data, including chunked data,
    // without buffering it. Empty bodies need no particular Content-Type (Q89).
    if axum::body::to_bytes(body, 0).await.is_err() {
        return invalid_request("관리자 종료 요청에는 본문을 보낼 수 없다.");
    }
    let Some(accepted) =
        shared
            .terminate
            .request_terminate(id, admin.session().account.clone(), peer.ip)
    else {
        return json_error(
            StatusCode::NOT_FOUND,
            "not_found",
            "지정한 세션을 찾을 수 없습니다.",
        );
    };
    shared.auth.touch(admin);
    // This connection alone stays open through E. The IO wrapper resumes U
    // on the first response write, not when this handler constructs a response.
    peer.idle.pause_for_termination();
    completion_response(shared.terminate.complete(accepted).await)
}

fn completion_response(outcome: Result<TerminateView, TerminateError>) -> Response {
    match outcome {
        Ok(view) => success(&view),
        Err(error) => json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            error.code(),
            "세션 종료를 확인할 수 없다.",
        ),
    }
}

fn invalid_request(message: &str) -> Response {
    json_error(StatusCode::BAD_REQUEST, "invalid_request", message)
}

fn success(view: &TerminateView) -> Response {
    let closed = &view.closed_session;
    json_response(
        StatusCode::OK,
        &json!({
            "as_of": timestamp(view.as_of),
            "result": view.result.as_str(),
            "closed_session": {
                "session_id": closed.session_id.as_str(),
                "mode": closed.mode.as_str(),
                "viewer_transport": closed.viewer_transport.as_str(),
                "target_transport": closed.target_transport.as_str(),
                "started_at": timestamp(closed.started_at),
                "ended_at": timestamp(closed.ended_at),
                "bytes_viewer_to_target": closed.bytes_viewer_to_target,
                "bytes_target_to_viewer": closed.bytes_target_to_viewer,
                "close_cause": closed.close_cause.as_str(),
            },
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// spec: management.md:MGT-009 partial
    #[test]
    fn only_exact_terminate_paths_match() {
        assert_eq!(
            session_id("/api/v1/sessions/a_b-1/terminate"),
            Some("a_b-1")
        );
        for path in [
            "/api/v1/sessions/a/terminate/",
            "/api/v1/sessions/a/extra/terminate",
            "/api/v1/sessions/a",
            "/api/v1/sessions/a/TERMINATE",
        ] {
            assert!(!is_terminate_path(path), "{path}");
        }
    }

    /// spec: management.md:MGT-012 partial
    #[tokio::test]
    async fn failed_completion_returns_500_without_a_success_summary() {
        let response = completion_response(Err(TerminateError::Failed));
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let bytes = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap_or_else(|error| panic!("error body: {error}"));
        let body: serde_json::Value =
            serde_json::from_slice(&bytes).unwrap_or_else(|error| panic!("error JSON: {error}"));
        assert_eq!(body["error"]["code"], "terminate_failed");
        assert!(body.get("closed_session").is_none());
        assert!(body.get("result").is_none());
    }

    /// spec: management.md:MGT-051 partial
    #[tokio::test]
    async fn success_response_serializes_closed_session_target_transport() {
        use std::sync::Arc;
        use std::time::{Duration, SystemTime};
        use vnc_relay_app::events::AdminTerminateResult;
        use vnc_relay_app::relay::ClosedSessionSnapshot;
        use vnc_relay_domain::handshake::Mode;
        use vnc_relay_domain::session::{CloseCause, SessionId, Transport};

        for (mode, viewer_transport, target_transport) in [
            (Mode::Mode1, Transport::Tcp, Transport::Tcp),
            (Mode::Mode2, Transport::Ws, Transport::Wss),
        ] {
            let view = TerminateView {
                as_of: SystemTime::UNIX_EPOCH,
                result: AdminTerminateResult::Terminated,
                closed_session: Arc::new(ClosedSessionSnapshot {
                    session_id: SessionId::parse("ses_aaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap(),
                    mode,
                    viewer_transport,
                    target_transport,
                    started_at: SystemTime::UNIX_EPOCH,
                    ended_at: SystemTime::UNIX_EPOCH + Duration::from_secs(1),
                    duration: Duration::from_secs(1),
                    bytes_viewer_to_target: 10,
                    bytes_target_to_viewer: 20,
                    close_cause: CloseCause::AdminTerminated,
                    relay_id_ref: None,
                    side: None,
                }),
            };
            let response = success(&view);
            let bytes = axum::body::to_bytes(response.into_body(), 4096)
                .await
                .expect("body");
            let body: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
            let closed = &body["closed_session"];
            assert_eq!(closed["mode"], mode.as_str());
            assert_eq!(closed["viewer_transport"], viewer_transport.as_str());
            assert_eq!(closed["target_transport"], target_transport.as_str());
            assert!(closed.get("viewer_forwarded_ip").is_none());
            assert!(closed.get("target_forwarded_ip").is_none());
            assert!(
                closed
                    .get("viewer_forwarded_ip_unavailable_reason")
                    .is_none()
            );
            assert!(
                closed
                    .get("target_forwarded_ip_unavailable_reason")
                    .is_none()
            );
        }
    }
}
