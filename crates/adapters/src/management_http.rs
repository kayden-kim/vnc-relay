//! The management listener: HTTPS through the built-in TLS acceptor, or plain
//! HTTP on loopback only (config-v1 §1.2 `[management]`, operations.md OPS-034).
//! It answers the login and session routes (management-api-v1 §1.5). Queries
//! and termination come with tasks 4.4–4.5, static files with 4.6.
//! Design: docs/architectures/management-listener.md and
//! docs/architectures/management-session.md.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::extract::connect_info::Connected;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::header::{
    ALLOW, CACHE_CONTROL, CONTENT_TYPE, COOKIE, LOCATION, SET_COOKIE, WWW_AUTHENTICATE,
};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::serve::IncomingStream;
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use vnc_relay_app::management::AdminAuth;
use vnc_relay_app::ports::EventSink;
use vnc_relay_domain::budget::ConnectionBudget;
use vnc_relay_domain::peer_ip::PeerIp;

use crate::admission::AdmittingListener;
use crate::request_deadline::DeadlineListener;
use crate::tls_listener::TlsListener;

mod session_api;

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
    tls_upgrade: Duration,
    auth: Arc<AdminAuth>,
    dashboard: DashboardHints,
}

impl ManagementListener {
    /// `admin_budget` is Lm, shared with the status listener. `tls_upgrade` is U:
    /// the limit on the TLS handshake and on reading each request head. `auth`
    /// holds the admin accounts and the admin sessions.
    pub fn new(
        listener: TcpListener,
        tls: Option<TlsAcceptor>,
        admin_budget: Arc<ConnectionBudget>,
        tls_upgrade: Duration,
        auth: Arc<AdminAuth>,
        dashboard: DashboardHints,
    ) -> Self {
        Self {
            listener,
            tls,
            admin_budget,
            tls_upgrade,
            auth,
            dashboard,
        }
    }

    /// Answers management requests until the task that runs it is aborted.
    pub async fn run(self, log: Arc<dyn EventSink>) {
        let secure = self.tls.is_some();
        let admitting =
            AdmittingListener::new(self.listener, self.admin_budget, Arc::clone(&log), LISTENER);
        // One fallback judges path and method itself, so every method reaches
        // the same rules (management-api-v1 §1.3, §1.11). The peer address
        // reaches the login limit through `ConnectInfo`.
        let shared = Shared {
            secure,
            auth: self.auth,
            dashboard: self.dashboard,
        };
        let router = Router::new()
            .fallback(answer)
            .with_state(shared)
            .into_make_service_with_connect_info::<PeerAddr>();
        // `serve` ends only on a listener error. Both listeners retry or
        // refuse without error, so there is no error to handle here.
        match self.tls {
            Some(acceptor) => {
                let listener = TlsListener::new(admitting, acceptor, log, LISTENER);
                let listener = DeadlineListener::new(listener, self.tls_upgrade);
                let _ = axum::serve(listener, router).await;
            }
            None => {
                let listener = DeadlineListener::new(admitting, self.tls_upgrade);
                let _ = axum::serve(listener, router).await;
            }
        }
    }
}

/// The peer of a management connection, as `ConnectInfo` gives it to the
/// handlers. axum implements `Connected` for `TcpListener` only. The listener
/// here is wrapped in `DeadlineListener`, so this type supplies the impl. It
/// keeps the normalized peer IP (domain `PeerIp`, ops-info-v1 §1.10).
#[derive(Debug, Clone, Copy)]
struct PeerAddr(PeerIp);

impl<L> Connected<IncomingStream<'_, DeadlineListener<L>>> for PeerAddr
where
    L: axum::serve::Listener<Addr = SocketAddr>,
{
    fn connect_info(stream: IncomingStream<'_, DeadlineListener<L>>) -> Self {
        Self(PeerIp::from(*stream.remote_addr()))
    }
}

/// The dashboard values that the session view carries (management-api-v1 §1.5).
#[derive(Debug, Clone, Copy)]
pub struct DashboardHints {
    poll_secs: u64,
    query_timeout_secs: u64,
    terminate_timeout_secs: u64,
}

impl DashboardHints {
    /// `close` is `[timeouts] close_secs`. The terminate wait adds 5 seconds.
    pub fn new(poll: Duration, query_timeout: Duration, close: Duration) -> Self {
        Self {
            poll_secs: poll.as_secs(),
            query_timeout_secs: query_timeout.as_secs(),
            terminate_timeout_secs: close.as_secs().saturating_add(5),
        }
    }
}

/// What the handlers need: the connection kind, the admin accounts and
/// sessions, and the dashboard values of the session view.
#[derive(Clone)]
struct Shared {
    /// `true` when the connection runs over TLS.
    secure: bool,
    auth: Arc<AdminAuth>,
    dashboard: DashboardHints,
}

/// Answers every request. `/api/v1/` goes to the session API, which judges the
/// session before the path and the method. Static paths keep their rules.
async fn answer(
    State(shared): State<Shared>,
    ConnectInfo(peer): ConnectInfo<PeerAddr>,
    request: Request,
) -> Response {
    let (parts, body) = request.into_parts();
    let path = parts.uri.path();
    if path.starts_with("/api/v1/") {
        return session_api::answer(&shared, peer.0, parts, body).await;
    }
    if path == "/" || path.starts_with("/app/") {
        return if is_read(&parts.method) {
            redirect_to_login()
        } else {
            method_not_allowed("GET, HEAD", "이 경로는 GET과 HEAD 요청만 받는다.")
        };
    }
    if path.starts_with("/login/") {
        return if is_read(&parts.method) {
            not_found()
        } else {
            method_not_allowed("GET, HEAD", "이 경로는 GET과 HEAD 요청만 받는다.")
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
fn unauthenticated(secure: bool, request_headers: &HeaderMap) -> Response {
    unauthorized(
        "unauthenticated",
        "유효한 관리 세션이 없다.",
        secure,
        request_headers,
    )
}

/// A `401` with `WWW-Authenticate` (management-api-v1 §2.1). A request that
/// carries the session cookie also clears it (management-api-v1 §1.4.1).
fn unauthorized(code: &str, message: &str, secure: bool, request_headers: &HeaderMap) -> Response {
    let mut response = json_error(StatusCode::UNAUTHORIZED, code, message);
    response.headers_mut().insert(
        WWW_AUTHENTICATE,
        HeaderValue::from_static(r#"Session realm="vnc-relay""#),
    );
    if session_cookie(request_headers).is_some() {
        response
            .headers_mut()
            .insert(SET_COOKIE, clear_session_cookie(secure));
    }
    response
}

/// The value of the admin session cookie in a request, when it has one.
fn session_cookie(headers: &HeaderMap) -> Option<&str> {
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.split_once('='))
        .find(|(name, _)| name.trim() == SESSION_COOKIE)
        .map(|(_, value)| value.trim())
}

fn clear_session_cookie(secure: bool) -> HeaderValue {
    if secure {
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

/// `405` with `Allow` for a path that takes only the methods in `allow`.
fn method_not_allowed(allow: &'static str, message: &str) -> Response {
    let mut response = json_error(StatusCode::METHOD_NOT_ALLOWED, "invalid_request", message);
    response
        .headers_mut()
        .insert(ALLOW, HeaderValue::from_static(allow));
    response
}

/// `404 not_found` for a static login path with no page in this task.
fn not_found() -> Response {
    json_error(StatusCode::NOT_FOUND, "not_found", "정의하지 않은 경로다.")
}

/// A JSON response with the headers of management-api-v1 §1.1.
fn json_response(status: StatusCode, body: &Value) -> Response {
    let mut response = (status, body.to_string()).into_response();
    let headers = response.headers_mut();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// The error body of management-api-v1 §2.1.
fn json_error(status: StatusCode, code: &str, message: &str) -> Response {
    json_response(
        status,
        &json!({ "error": { "code": code, "message": message } }),
    )
}
