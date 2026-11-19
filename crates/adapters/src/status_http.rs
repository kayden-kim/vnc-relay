//! The status listener: `/livez`, `/readyz`, and with `[metrics] enabled`
//! also `/metrics`, without login on a listener of its own, with each
//! connection counted in Lm (ops-info-v2 §1.12, §2; metrics-v1 §1.6, §2).
//! Design: docs/architectures/status-listener.md and metrics.md.

use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::header::{ALLOW, CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderValue, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use vnc_relay_app::lifecycle::LifecycleService;
use vnc_relay_app::metrics::MetricsService;
use vnc_relay_app::ports::EventSink;
use vnc_relay_app::reload::ports::RuntimeSettingsSource;
use vnc_relay_domain::budget::ConnectionBudget;

use crate::admission::AdmittingListener;
use crate::listener::Listener;
use crate::metrics_text;

/// `Content-Type` of the Prometheus text format 0.0.4 (metrics-v1 §1.6).
const METRICS_CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";

/// `listener` of the status listener in operation events (ops-info-v2 §1.5).
const LISTENER: &str = "status";

/// The status listener. It answers from the first moment it runs, so a
/// query during the start sees `starting`.
pub struct StatusListener<L = TcpListener> {
    listener: L,
    admin_budget: Arc<ConnectionBudget>,
    state: StatusState,
    runtime: Arc<dyn RuntimeSettingsSource>,
}

/// What the routes read: readiness, and the metrics when `/metrics` is on.
#[derive(Clone)]
struct StatusState {
    lifecycle: Arc<LifecycleService>,
    metrics: Option<Arc<MetricsService>>,
}

impl<L: Listener> StatusListener<L> {
    /// `admin_budget` is Lm, shared with the management listener. `metrics`
    /// is `Some` only when `[metrics] enabled` is true; otherwise
    /// `/metrics` is an undefined path (metrics-v1 §2 `not_found`).
    pub fn new(
        listener: L,
        admin_budget: Arc<ConnectionBudget>,
        lifecycle: Arc<LifecycleService>,
        runtime: Arc<dyn RuntimeSettingsSource>,
        metrics: Option<Arc<MetricsService>>,
    ) -> Self {
        Self {
            listener,
            admin_budget,
            state: StatusState { lifecycle, metrics },
            runtime,
        }
    }

    /// Answers status queries until the task that runs it is aborted.
    pub async fn run(self, log: Arc<dyn EventSink>) {
        let listener = AdmittingListener::new(
            self.listener,
            self.admin_budget,
            self.runtime,
            log,
            LISTENER,
        );
        // One fallback judges method and path itself: a `get` route would
        // also take HEAD, and the stock 404 and 405 have no contract body.
        let router = Router::new().fallback(answer).with_state(self.state);
        // Fatal accepts report to the process supervisor and stop accepting.
        // Existing HTTP connections stay alive until the owner aborts the task.
        let _ = axum::serve(listener, router).await;
    }
}

async fn answer(State(state): State<StatusState>, method: Method, uri: Uri) -> Response {
    let get = method == Method::GET;
    let (status, body) = match (uri.path(), get, &state.metrics) {
        ("/metrics", true, Some(metrics)) => return metrics_response(metrics),
        ("/livez", true, _) => (StatusCode::OK, json!({ "status": "alive" })),
        ("/readyz", true, _) => match state.lifecycle.readiness().not_ready_reason() {
            None => (StatusCode::OK, json!({ "status": "ready" })),
            Some(reason) => (
                StatusCode::SERVICE_UNAVAILABLE,
                json!({ "status": "not_ready", "reason": reason }),
            ),
        },
        ("/livez" | "/readyz", false, _) | ("/metrics", false, Some(_)) => (
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

/// `200` with every metric as Prometheus text (metrics-v1 §1.6).
fn metrics_response(metrics: &MetricsService) -> Response {
    let body = metrics_text::encode(&metrics.snapshot());
    let mut response = (StatusCode::OK, body).into_response();
    let headers = response.headers_mut();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static(METRICS_CONTENT_TYPE));
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// The error body of ops-info-v2 §2.
fn error_body(code: &str, message: &str) -> Value {
    json!({ "error": { "code": code, "message": message } })
}
