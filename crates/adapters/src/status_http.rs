//! The status listener: `/livez` and `/readyz` without login on a listener
//! of its own, with each connection counted in Lm (ops-info-v1 §1.12, §2).
//! Design: docs/architectures/status-listener.md.

use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::header::{ALLOW, CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderValue, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use vnc_relay_app::lifecycle::LifecycleService;
use vnc_relay_app::ports::EventSink;
use vnc_relay_domain::budget::ConnectionBudget;

use crate::admission::AdmittingListener;

/// `listener` of the status listener in operation events (ops-info-v1 §1.5).
const LISTENER: &str = "status";

/// The status listener. It answers from the first moment it runs, so a
/// query during the start sees `starting`.
pub struct StatusListener {
    listener: TcpListener,
    admin_budget: Arc<ConnectionBudget>,
    lifecycle: Arc<LifecycleService>,
}

impl StatusListener {
    /// `admin_budget` is Lm, shared with the management listener.
    pub fn new(
        listener: TcpListener,
        admin_budget: Arc<ConnectionBudget>,
        lifecycle: Arc<LifecycleService>,
    ) -> Self {
        Self {
            listener,
            admin_budget,
            lifecycle,
        }
    }

    /// Answers status queries until the task that runs it is aborted.
    pub async fn run(self, log: Arc<dyn EventSink>) {
        let listener = AdmittingListener::new(self.listener, self.admin_budget, log, LISTENER);
        // One fallback judges method and path itself: a `get` route would
        // also take HEAD, and the stock 404 and 405 have no contract body.
        let router = Router::new().fallback(answer).with_state(self.lifecycle);
        // `serve` ends only on a listener error, and `AdmittingListener`
        // retries every accept error, so there is no error to handle.
        let _ = axum::serve(listener, router).await;
    }
}

async fn answer(
    State(lifecycle): State<Arc<LifecycleService>>,
    method: Method,
    uri: Uri,
) -> Response {
    let (status, body) = match (uri.path(), method == Method::GET) {
        ("/livez", true) => (StatusCode::OK, json!({ "status": "alive" })),
        ("/readyz", true) => match lifecycle.readiness().not_ready_reason() {
            None => (StatusCode::OK, json!({ "status": "ready" })),
            Some(reason) => (
                StatusCode::SERVICE_UNAVAILABLE,
                json!({ "status": "not_ready", "reason": reason }),
            ),
        },
        ("/livez" | "/readyz", false) => (
            StatusCode::METHOD_NOT_ALLOWED,
            error_body("method_not_allowed", "이 경로는 GET 요청만 받는다."),
        ),
        _ => (
            StatusCode::NOT_FOUND,
            error_body("not_found", "정의하지 않은 경로다."),
        ),
    };
    let mut response = (status, body.to_string()).into_response();
    let headers = response.headers_mut();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    if status == StatusCode::METHOD_NOT_ALLOWED {
        headers.insert(ALLOW, HeaderValue::from_static("GET"));
    }
    response
}

/// The error body of ops-info-v1 §2.
fn error_body(code: &str, message: &str) -> Value {
    json!({ "error": { "code": code, "message": message } })
}
