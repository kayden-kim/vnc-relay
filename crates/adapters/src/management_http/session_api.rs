//! The session routes of the management API: login, the session view and
//! logout (management-api-v1 §1.2–§1.5). The checks run in the order of
//! management-api-v1 §1.3. Other `/api/v1/` paths answer `404` for a valid
//! session until tasks 4.4 and 4.5 add their routes.

use std::sync::Arc;

use axum::body::Body;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, SET_COOKIE};
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use vnc_relay_app::management::login::LoggedIn;
use vnc_relay_app::management::session::AuthorizedAdmin;
use vnc_relay_app::management::{AuthError, SessionView};
use vnc_relay_domain::peer_ip::PeerIp;

use super::{
    DashboardHints, SESSION_COOKIE, Shared, clear_session_cookie, json_error, json_response,
    method_not_allowed, session_cookie, unauthenticated, unauthorized,
};
use crate::clock::timestamp;

const SESSION_PATH: &str = "/api/v1/session";

/// Largest login body that is read (management-api-v1 §1.5).
const LOGIN_BODY_LIMIT: usize = 16_384;

/// The message of every refused login. It does not tell the cases apart
/// (management-api-v1 §1.5). 구현 판단(사용자 결정 없음): the wording is ours.
const LOGIN_FAILED_MESSAGE: &str = "로그인할 수 없다.";

const POLL_MESSAGE: &str = "X-Relay-Poll은 값 1을 가진 GET 요청에만 쓸 수 있다.";
const BODY_MESSAGE: &str = "요청 본문은 account와 password 문자열을 가진 JSON이어야 한다.";
const MEDIA_TYPE_MESSAGE: &str = "요청 본문은 application/json이어야 한다.";
const ACCOUNT_MESSAGE: &str =
    "계정 이름은 소문자 영문, 숫자, 점, 밑줄, 하이픈으로 1~32자여야 한다.";

/// Answers one `/api/v1/` request. A login does not need a session. Every other
/// request needs a valid session before its path and method are judged.
pub(super) async fn answer(shared: &Shared, peer: PeerIp, parts: Parts, body: Body) -> Response {
    if parts.uri.path() == SESSION_PATH && parts.method == Method::POST {
        return login(shared, peer, &parts.headers, body).await;
    }
    let mut admin = match shared.auth.authorize(session_cookie(&parts.headers)) {
        Ok(admin) => admin,
        Err(error) => return refuse(error, shared.secure, &parts.headers),
    };
    if parts.uri.path() == SESSION_PATH {
        if matches!(parts.method, Method::POST | Method::DELETE)
            && let Err(error) = shared.auth.check_csrf(&admin, csrf_header(&parts.headers))
        {
            return refuse(error, shared.secure, &parts.headers);
        }
        return match parts.method {
            Method::GET => get_session(shared, &mut admin, &parts.headers, body).await,
            Method::DELETE => delete_session(shared, &admin, &parts.headers, body).await,
            _ => method_not_allowed(
                "GET, POST, DELETE",
                "이 경로는 GET, POST, DELETE 요청만 받는다.",
            ),
        };
    }
    super::query_api::answer(shared, &mut admin, parts, body).await
}

/// `POST /api/v1/session`: checks the body, then runs the login on a blocking
/// thread. The password check takes measurable time.
async fn login(shared: &Shared, peer: PeerIp, headers: &HeaderMap, body: Body) -> Response {
    if headers.contains_key("x-relay-poll") {
        return bad_request(POLL_MESSAGE);
    }
    if !is_json(headers) {
        return unsupported_media_type();
    }
    let bytes = match axum::body::to_bytes(body, LOGIN_BODY_LIMIT).await {
        Ok(bytes) => bytes,
        Err(_) => return bad_request(BODY_MESSAGE),
    };
    let Some((account, password)) = credentials(&bytes) else {
        return bad_request(BODY_MESSAGE);
    };
    let prior = session_cookie(headers).map(str::to_owned);
    let auth = Arc::clone(&shared.auth);
    let outcome = tokio::task::spawn_blocking(move || {
        auth.login(&account, &password, peer, prior.as_deref())
    })
    .await;
    match outcome {
        Ok(Ok(logged_in)) => logged_in_response(shared, logged_in),
        Ok(Err(error)) => refuse(error, shared.secure, headers),
        Err(_) => internal_error(),
    }
}

/// `GET /api/v1/session`: the session view. A user action extends the idle
/// deadline. An automatic poll (`X-Relay-Poll: 1`) does not.
async fn get_session(
    shared: &Shared,
    admin: &mut AuthorizedAdmin,
    headers: &HeaderMap,
    body: Body,
) -> Response {
    let Some(poll) = poll_flag(headers) else {
        return bad_request(POLL_MESSAGE);
    };
    if !media_type_allows(body, headers).await {
        return unsupported_media_type();
    }
    if !poll {
        shared.auth.touch(admin);
    }
    session_response(StatusCode::OK, admin.session(), shared.dashboard, true)
}

/// `DELETE /api/v1/session`: ends this session only. The response clears the
/// cookie.
async fn delete_session(
    shared: &Shared,
    admin: &AuthorizedAdmin,
    headers: &HeaderMap,
    body: Body,
) -> Response {
    if headers.contains_key("x-relay-poll") {
        return bad_request(POLL_MESSAGE);
    }
    if !media_type_allows(body, headers).await {
        return unsupported_media_type();
    }
    shared.auth.logout(admin);
    let mut response = StatusCode::NO_CONTENT.into_response();
    let response_headers = response.headers_mut();
    response_headers.insert(SET_COOKIE, clear_session_cookie(shared.secure));
    response_headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// The `200` answer to a login: the session view and the new cookie.
fn logged_in_response(shared: &Shared, logged_in: LoggedIn) -> Response {
    let Some(cookie) = session_set_cookie(logged_in.token.as_str(), shared.secure) else {
        return internal_error();
    };
    let mut response =
        session_response(StatusCode::OK, &logged_in.session, shared.dashboard, false);
    response.headers_mut().insert(SET_COOKIE, cookie);
    response
}

/// `Set-Cookie` for the session cookie (management-api-v1 §1.4.1). `Secure` is
/// set only over HTTPS.
fn session_set_cookie(token: &str, secure: bool) -> Option<HeaderValue> {
    let secure_attr = if secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Strict{secure_attr}"
    ))
    .ok()
}

/// The JSON answer for a session. `as_of` is added only when `as_of` is true,
/// which is the case for GET answers (management-api-v1 §1.1).
fn session_response(
    status: StatusCode,
    view: &SessionView,
    dashboard: DashboardHints,
    as_of: bool,
) -> Response {
    let mut body = session_body(view, dashboard);
    if as_of {
        body["as_of"] = json!(timestamp(view.as_of));
    }
    json_response(status, &body)
}

/// The body of management-api-v1 §1.5. 구현 판단(사용자 결정 없음): the
/// contract's dotted `dashboard.*` names are read as one nested `dashboard`
/// object, the same way its other dotted names read.
fn session_body(view: &SessionView, dashboard: DashboardHints) -> Value {
    json!({
        "account": view.account.as_str(),
        "csrf_token": view.csrf.as_str(),
        "idle_expires_at": timestamp(view.idle_expires_at),
        "absolute_expires_at": timestamp(view.absolute_expires_at),
        "dashboard": {
            "poll_secs": dashboard.poll_secs,
            "query_timeout_secs": dashboard.query_timeout_secs,
            "terminate_timeout_secs": dashboard.terminate_timeout_secs,
        },
    })
}

/// Maps a refused check to its error response (management-api-v1 §2.1).
fn refuse(error: AuthError, secure: bool, headers: &HeaderMap) -> Response {
    match error {
        AuthError::Unauthenticated => unauthenticated(secure, headers),
        AuthError::CsrfInvalid => csrf_invalid(),
        AuthError::LoginFailed => {
            unauthorized("login_failed", LOGIN_FAILED_MESSAGE, secure, headers)
        }
        AuthError::InvalidAccount => bad_request(ACCOUNT_MESSAGE),
        // The detail stays inside the process. Only the generic message leaves it.
        AuthError::Internal(_) => internal_error(),
    }
}

/// The `account` and `password` strings of a login body. Any other shape gives
/// `None`. Unknown fields are ignored (management-api-v1 §1.5).
fn credentials(bytes: &[u8]) -> Option<(String, String)> {
    let value: Value = serde_json::from_slice(bytes).ok()?;
    let account = non_empty_string(&value, "account")?;
    let password = non_empty_string(&value, "password")?;
    Some((account, password))
}

fn non_empty_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)?
        .as_str()
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

/// `true` when `Content-Type` names `application/json`. Parameters such as
/// `charset` are ignored.
fn is_json(headers: &HeaderMap) -> bool {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|text| text.split(';').next())
        .is_some_and(|media| media.trim().eq_ignore_ascii_case("application/json"))
}

/// `false` when the request has a body that is not `application/json`
/// (management-api-v1 §1.1). An empty body always passes. A JSON body is
/// ignored after this check (구현 판단(사용자 결정 없음)).
pub(super) async fn media_type_allows(body: Body, headers: &HeaderMap) -> bool {
    axum::body::to_bytes(body, 0).await.is_ok() || is_json(headers)
}

/// The value of `X-CSRF-Token`. A missing, repeated, or non-text value gives
/// `None`, which the check refuses.
fn csrf_header(headers: &HeaderMap) -> Option<&str> {
    let mut values = headers.get_all("x-csrf-token").iter();
    let value = values.next()?;
    if values.next().is_some() {
        return None;
    }
    value.to_str().ok()
}

/// `X-Relay-Poll` (management-api-v1 §1.4.2). Absent gives `Some(false)`, the
/// value `1` gives `Some(true)`, and anything else gives `None`. The caller
/// refuses `None`.
pub(super) fn poll_flag(headers: &HeaderMap) -> Option<bool> {
    let mut values = headers.get_all("x-relay-poll").iter();
    let Some(value) = values.next() else {
        return Some(false);
    };
    if values.next().is_some() || value.as_bytes() != b"1" {
        return None;
    }
    Some(true)
}

fn bad_request(message: &str) -> Response {
    json_error(StatusCode::BAD_REQUEST, "invalid_request", message)
}

fn unsupported_media_type() -> Response {
    json_error(
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        "invalid_request",
        MEDIA_TYPE_MESSAGE,
    )
}

fn csrf_invalid() -> Response {
    json_error(
        StatusCode::FORBIDDEN,
        "csrf_invalid",
        "CSRF 토큰이 없거나 맞지 않다.",
    )
}

fn internal_error() -> Response {
    json_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "internal",
        "서버 내부 오류가 났다.",
    )
}
