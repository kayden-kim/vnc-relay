//! The management listener: HTTPS through the built-in TLS acceptor, or plain
//! HTTP on loopback only (config-v3 §1.2 `[management]`, operations.md OPS-034).
//! It answers login, session, query and termination routes and serves the
//! dashboard files (management-api-v1 §1.5–§1.11).
//! Design: docs/architectures/management-listener.md,
//! management-session.md and management-terminate.md.

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::extract::connect_info::Connected;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::header::{
    ALLOW, CACHE_CONTROL, CONTENT_LENGTH, CONTENT_SECURITY_POLICY, CONTENT_TYPE, COOKIE, LOCATION,
    REFERRER_POLICY, SET_COOKIE, WWW_AUTHENTICATE, X_CONTENT_TYPE_OPTIONS,
};
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::serve::IncomingStream;
use serde_json::{Value, json};
use tokio::net::TcpListener;
use vnc_relay_app::management::AdminAuth;
use vnc_relay_app::management::config::ConfigManagement;
use vnc_relay_app::management::query::ManagementQueryService;
use vnc_relay_app::management::terminate::ManagementTerminateService;
use vnc_relay_app::ports::EventSink;
use vnc_relay_domain::budget::ConnectionBudget;
use vnc_relay_domain::peer_ip::PeerIp;

use crate::admission::AdmittingListener;
use crate::dashboard_assets::{Asset, find_asset};
use crate::listener::Listener;
use crate::request_deadline::{DeadlineListener, IdleControl};
use crate::tls::TlsSource;
use crate::tls_listener::TlsListener;

mod config_api;
mod query_api;
mod session_api;
mod terminate_api;

/// `listener` of the management listener in operation events (ops-info-v1 §1.5).
const LISTENER: &str = "management";

/// Name of the admin session cookie (management-api-v1 §1.4.1).
const SESSION_COOKIE: &str = "relay_admin_session";

/// Clears the session cookie on plain HTTP. `Secure` is set only over HTTPS.
const CLEAR_SESSION_COOKIE: &str =
    "relay_admin_session=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict";
const CLEAR_SESSION_COOKIE_SECURE: &str =
    "relay_admin_session=; Path=/; Max-Age=0; HttpOnly; SameSite=Strict; Secure";

/// Required application services, shared by all management connections.
pub struct ManagementServices {
    pub auth: Arc<AdminAuth>,
    pub query: Arc<ManagementQueryService>,
    pub terminate: Arc<ManagementTerminateService>,
    pub config: Arc<dyn ConfigManagement>,
}

/// A management listener. Connections run TLS when `tls` is set and plain
/// HTTP otherwise.
pub struct ManagementListener<L = TcpListener> {
    listener: L,
    tls: Option<TlsSource>,
    admin_budget: Arc<ConnectionBudget>,
    runtime: Arc<dyn vnc_relay_app::reload::ports::RuntimeSettingsSource>,
    services: ManagementServices,
    dashboard: DashboardHints,
}

impl<L: Listener> ManagementListener<L> {
    /// `admin_budget` counts Lm. The runtime source supplies Lm and U at accept.
    /// `services` uses the actual relay runtime for queries and termination.
    pub fn new(
        listener: L,
        tls: Option<TlsSource>,
        admin_budget: Arc<ConnectionBudget>,
        runtime: Arc<dyn vnc_relay_app::reload::ports::RuntimeSettingsSource>,
        dashboard: DashboardHints,
        services: ManagementServices,
    ) -> Self {
        Self {
            listener,
            tls,
            admin_budget,
            runtime,
            services,
            dashboard,
        }
    }

    /// Answers management requests until the task that runs it is aborted.
    pub async fn run(self, log: Arc<dyn EventSink>) {
        let secure = self.tls.is_some();
        let admitting = AdmittingListener::new(
            self.listener,
            self.admin_budget,
            self.runtime,
            Arc::clone(&log),
            LISTENER,
        );
        // One fallback judges path and method itself, so every method reaches
        // the same rules (management-api-v1 §1.3, §1.11). The peer address
        // reaches the login limit through `ConnectInfo`.
        let shared = Shared {
            secure,
            auth: self.services.auth,
            query: self.services.query,
            terminate: self.services.terminate,
            config: self.services.config,
            dashboard: self.dashboard,
        };
        let router = Router::new()
            .fallback(answer)
            .with_state(shared)
            .into_make_service_with_connect_info::<PeerAddr>();
        // Fatal accepts report to the supervisor; existing HTTP connections
        // remain usable until relay cleanup ends and the owner aborts this task.
        match self.tls {
            Some(source) => {
                let listener = TlsListener::new(admitting, source, log, LISTENER);
                let listener = DeadlineListener::new(listener);
                let _ = axum::serve(listener, router).await;
            }
            None => {
                let listener = DeadlineListener::new(admitting);
                let _ = axum::serve(listener, router).await;
            }
        }
    }
}

/// The peer of a management connection, as `ConnectInfo` gives it to the
/// handlers. axum implements `Connected` for `TcpListener` only. The listener
/// here is wrapped in `DeadlineListener`, so this type supplies the impl. It
/// keeps the normalized peer IP (domain `PeerIp`, ops-info-v1 §1.10).
#[derive(Debug, Clone)]
struct PeerAddr {
    ip: PeerIp,
    idle: IdleControl,
}

impl<L> Connected<IncomingStream<'_, DeadlineListener<L>>> for PeerAddr
where
    L: axum::serve::Listener<Addr = crate::admission::AdmittedAddr>,
{
    fn connect_info(stream: IncomingStream<'_, DeadlineListener<L>>) -> Self {
        Self {
            ip: PeerIp::from(stream.remote_addr().socket),
            idle: stream.io().idle_control(),
        }
    }
}

/// The dashboard values that the session view carries (management-api-v1 §1.5).
#[derive(Clone)]
pub struct DashboardHints {
    poll_secs: u64,
    query_timeout_secs: u64,
    runtime: Arc<dyn vnc_relay_app::reload::ports::RuntimeSettingsSource>,
}

impl DashboardHints {
    /// P and query timeout remain startup values; E is read for each response.
    pub fn new(
        poll: Duration,
        query_timeout: Duration,
        runtime: Arc<dyn vnc_relay_app::reload::ports::RuntimeSettingsSource>,
    ) -> Self {
        Self {
            poll_secs: poll.as_secs(),
            query_timeout_secs: query_timeout.as_secs(),
            runtime,
        }
    }

    fn terminate_timeout_secs(&self) -> u64 {
        let mut seconds = 0;
        self.runtime.with_current(&mut |settings| {
            seconds = settings.timeouts.close.as_secs().saturating_add(5);
        });
        seconds
    }
}

/// What the handlers need: the connection kind, the admin accounts and
/// sessions, and the dashboard values of the session view.
#[derive(Clone)]
struct Shared {
    /// `true` when the connection runs over TLS.
    secure: bool,
    auth: Arc<AdminAuth>,
    query: Arc<ManagementQueryService>,
    terminate: Arc<ManagementTerminateService>,
    config: Arc<dyn ConfigManagement>,
    dashboard: DashboardHints,
}

/// Answers every request. `/api/v1/` goes to the session API, which judges the
/// session before the path and the method. Static paths go to `static_answer`.
/// Any other path gets a `303` to the login page.
async fn answer(
    State(shared): State<Shared>,
    ConnectInfo(peer): ConnectInfo<PeerAddr>,
    request: Request,
) -> Response {
    let (parts, body) = request.into_parts();
    let path = parts.uri.path();
    if path.starts_with("/api/v1/") {
        return session_api::answer(&shared, &peer, parts, body).await;
    }
    if path == "/" || path.starts_with("/app/") || path.starts_with("/login/") {
        return static_answer(&shared, &parts);
    }
    redirect_to_login()
}

/// The `Content-Security-Policy` of a dashboard file (management-api-v1 §1.11).
const STATIC_CSP: &str = "default-src 'self'; frame-ancestors 'none'";

/// A static path (management-api-v1 §1.11). Only `GET` and `HEAD` are allowed.
/// `/` and `/app/` need a valid session. The check only reads the session, so
/// it does not extend the idle time. Without a session the request goes to the
/// login page, even for a file that does not exist.
fn static_answer(shared: &Shared, parts: &Parts) -> Response {
    if !is_read(&parts.method) {
        return method_not_allowed("GET, HEAD", "이 경로는 GET과 HEAD 요청만 받는다.");
    }
    let path = parts.uri.path();
    let protected = path == "/" || path.starts_with("/app/");
    if protected
        && shared
            .auth
            .authorize(session_cookie(&parts.headers))
            .is_err()
    {
        return redirect_to_login();
    }
    match find_asset(path) {
        Some(asset) => static_file(asset, parts.method == Method::HEAD),
        None => not_found(),
    }
}

/// A `200` with a dashboard file. `HEAD` gets the same headers, including
/// `Content-Length`, and no body.
fn static_file(asset: &'static Asset, head: bool) -> Response {
    let mut response = if head {
        StatusCode::OK.into_response()
    } else {
        asset.body.into_response()
    };
    let headers = response.headers_mut();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static(asset.content_type));
    headers.insert(CONTENT_LENGTH, HeaderValue::from(asset.body.len()));
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    headers.insert(REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    headers.insert(
        CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(STATIC_CSP),
    );
    response
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

/// `404 not_found` for a static path with no file in the dashboard asset table.
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

#[cfg(test)]
mod dashboard_hint_tests {
    /// spec: management.md:MGT-041 partial
    /// spec: operations.md:OPS-057 partial
    #[test]
    fn session_hint_reads_current_close_timeout_without_changing_startup_poll_values() {
        let mut settings = crate::config_file::parse(
            "[admin]\naccounts_file='accounts.toml'\n[management]\nlisten='127.0.0.1:8443'\n[status]\nlisten='127.0.0.1:9090'\n",
            std::path::Path::new("."),
        ).unwrap();
        settings.timeouts.close = std::time::Duration::from_secs(3);
        let candidate = |settings| vnc_relay_app::reload::ConfigCandidate {
            settings,
            presence: Default::default(),
        };
        let source = std::sync::Arc::new(vnc_relay_app::reload::snapshot::ReloadCell::<()>::new(
            vnc_relay_app::reload::snapshot::ReloadSnapshot::new(
                candidate(settings.clone()),
                None,
                None,
            ),
        ));
        let hints = super::DashboardHints::new(
            std::time::Duration::from_secs(2),
            std::time::Duration::from_secs(5),
            source.clone(),
        );
        assert_eq!(hints.terminate_timeout_secs(), 8);
        settings.timeouts.close = std::time::Duration::from_secs(13);
        source.replace(vnc_relay_app::reload::snapshot::ReloadSnapshot::new(
            candidate(settings),
            None,
            None,
        ));
        assert_eq!(hints.terminate_timeout_secs(), 18);
        assert_eq!(hints.poll_secs, 2);
        assert_eq!(hints.query_timeout_secs, 5);
    }
}
