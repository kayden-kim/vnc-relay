//! The management listener: HTTPS through the built-in TLS acceptor, or plain
//! HTTP on loopback only (config-v1 §1.2 `[management]`, operations.md OPS-034). It answers
//! the routes that need no session. Login, session views, queries, termination
//! and static files come with tasks 4.3–4.6.
//! Design: docs/architectures/management-listener.md.

use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::header::{
    ALLOW, CACHE_CONTROL, CONTENT_TYPE, COOKIE, LOCATION, SET_COOKIE, WWW_AUTHENTICATE,
};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use serde_json::json;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use vnc_relay_app::ports::EventSink;
use vnc_relay_domain::budget::ConnectionBudget;

use crate::admission::AdmittingListener;
use crate::tls_listener::TlsListener;

/// `listener` of the management listener in operation events (ops-info-v1 §1.5).
const LISTENER: &str = "management";

/// Name of the admin session cookie (management-api-v1 §1.4.1).
const SESSION_COOKIE: &str = "relay_admin_session";

/// Clears the session cookie on plain HTTP. `Secure` is set only over HTTPS.
const CLEAR_SESSION_COOKIE: &str =
    "relay_admin_session=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict";
const CLEAR_SESSION_COOKIE_SECURE: &str =
    "relay_admin_session=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict; Secure";

/// A management listener. Connections run TLS when `tls` is set and plain
/// HTTP otherwise.
pub struct ManagementListener {
    listener: TcpListener,
    tls: Option<TlsAcceptor>,
    admin_budget: Arc<ConnectionBudget>,
}

impl ManagementListener {
    /// `admin_budget` is Lm, shared with the status listener.
    pub fn new(
        listener: TcpListener,
        tls: Option<TlsAcceptor>,
        admin_budget: Arc<ConnectionBudget>,
    ) -> Self {
        Self {
            listener,
            tls,
            admin_budget,
        }
    }

    /// Answers management requests until the task that runs it is aborted.
    pub async fn run(self, log: Arc<dyn EventSink>) {
        let secure = self.tls.is_some();
        let admitting =
            AdmittingListener::new(self.listener, self.admin_budget, Arc::clone(&log), LISTENER);
        // One fallback judges path and method itself, so every method reaches
        // the same rules (management-api-v1 §1.3, §1.11).
        let router = Router::new().fallback(answer).with_state(Scheme { secure });
        // `serve` ends only on a listener error. Both listeners retry or
        // refuse without error, so there is no error to handle here.
        match self.tls {
            Some(acceptor) => {
                let listener = TlsListener::new(admitting, acceptor, log, LISTENER);
                let _ = axum::serve(listener, router).await;
            }
            None => {
                let _ = axum::serve(admitting, router).await;
            }
        }
    }
}

/// What the handlers need from the connection they answer.
#[derive(Clone, Copy)]
struct Scheme {
    /// `true` when the connection runs over TLS.
    secure: bool,
}

/// Answers the routes that need no session. No admin session exists before
/// login (task 4.3), so every request is judged as unauthenticated.
async fn answer(
    State(scheme): State<Scheme>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
) -> Response {
    let path = uri.path();
    // Management API requests need a session before path and method are
    // judged, so an unauthenticated caller cannot learn which paths exist
    // (management-api-v1 §1.3 step 2).
    if path.starts_with("/api/v1/") {
        return unauthenticated(scheme, &headers);
    }
    if path == "/" || path.starts_with("/app/") {
        return if is_read(&method) {
            redirect_to_login()
        } else {
            method_not_allowed()
        };
    }
    if path.starts_with("/login/") {
        return if is_read(&method) {
            not_found()
        } else {
            method_not_allowed()
        };
    }
    redirect_to_login()
}

/// `GET` and `HEAD` are the only methods of static paths (management-api-v1
/// §1.11).
fn is_read(method: &Method) -> bool {
    *method == Method::GET || *method == Method::HEAD
}

/// `401 unauthenticated`. A request that carries the session cookie also
/// clears it (management-api-v1 §1.4.1).
fn unauthenticated(scheme: Scheme, request_headers: &HeaderMap) -> Response {
    let mut response = json_error(
        StatusCode::UNAUTHORIZED,
        "unauthenticated",
        "유효한 관리 세션이 없다.",
    );
    response.headers_mut().insert(
        WWW_AUTHENTICATE,
        HeaderValue::from_static(r#"Session realm="vnc-relay""#),
    );
    if has_session_cookie(request_headers) {
        response
            .headers_mut()
            .insert(SET_COOKIE, clear_session_cookie(scheme));
    }
    response
}

/// `true` when a `Cookie` header names the admin session cookie.
fn has_session_cookie(headers: &HeaderMap) -> bool {
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.split_once('='))
        .any(|(name, _)| name.trim() == SESSION_COOKIE)
}

fn clear_session_cookie(scheme: Scheme) -> HeaderValue {
    if scheme.secure {
        HeaderValue::from_static(CLEAR_SESSION_COOKIE_SECURE)
    } else {
        HeaderValue::from_static(CLEAR_SESSION_COOKIE)
    }
}

/// `303 See Other` to the login page. A static response has no JSON body
/// (management-api-v1 §1.11).
fn redirect_to_login() -> Response {
    let mut response = StatusCode::SEE_OTHER.into_response();
    response
        .headers_mut()
        .insert(LOCATION, HeaderValue::from_static("/login/"));
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// `405` with `Allow` for a static path that takes only `GET` and `HEAD`.
fn method_not_allowed() -> Response {
    let mut response = json_error(
        StatusCode::METHOD_NOT_ALLOWED,
        "invalid_request",
        "이 경로는 GET과 HEAD 요청만 받는다.",
    );
    response
        .headers_mut()
        .insert(ALLOW, HeaderValue::from_static("GET, HEAD"));
    response
}

/// `404 not_found` for a login path with no page in this task.
fn not_found() -> Response {
    json_error(StatusCode::NOT_FOUND, "not_found", "정의하지 않은 경로다.")
}

/// The error body of management-api-v1 §2.1 with the JSON headers of §1.1.
fn json_error(status: StatusCode, code: &str, message: &str) -> Response {
    let body = json!({ "error": { "code": code, "message": message } }).to_string();
    let mut response = (status, body).into_response();
    let headers = response.headers_mut();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}
