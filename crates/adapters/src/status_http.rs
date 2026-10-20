//! The status listener: `/livez` and `/readyz` without login on a listener
//! of its own, with each connection counted in Lm (ops-info-v1 §1.12, §2).
//! Design: docs/architectures/status-listener.md.

use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::Router;
use axum::extract::State;
use axum::http::header::{ALLOW, CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderValue, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use vnc_relay_app::events::{Event, RejectReason, Stage};
use vnc_relay_app::lifecycle::LifecycleService;
use vnc_relay_app::ports::EventSink;
use vnc_relay_domain::budget::{BudgetPermit, ConnectionBudget};
use vnc_relay_domain::peer_ip::PeerIp;

use crate::net::ACCEPT_RETRY_DELAY;

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
        let listener = AdmittingListener {
            listener: self.listener,
            budget: self.admin_budget,
            log,
        };
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

/// Takes a place of Lm right after each accept. A connection without a
/// place is logged and closed before any HTTP is read (ops-info-v1 §2).
struct AdmittingListener {
    listener: TcpListener,
    budget: Arc<ConnectionBudget>,
    log: Arc<dyn EventSink>,
}

impl axum::serve::Listener for AdmittingListener {
    type Io = Admitted;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Admitted, SocketAddr) {
        loop {
            let (stream, addr) = match self.listener.accept().await {
                Ok(accepted) => accepted,
                Err(_) => {
                    // ops-info-v1 has no event for a failed accept.
                    tokio::time::sleep(ACCEPT_RETRY_DELAY).await;
                    continue;
                }
            };
            if let Some(permit) = self.budget.try_acquire() {
                return (
                    Admitted {
                        stream,
                        _permit: permit,
                    },
                    addr,
                );
            }
            // Logged before the close, so a peer that sees the close also
            // finds the line.
            self.log.record(&Event::ConnectionRejected {
                listener: LISTENER,
                stage: Stage::Accept,
                reason: RejectReason::AdminConnectionLimit {
                    limit: self.budget.limit(),
                },
                peer_ip: PeerIp::from(addr),
            });
            drop(stream);
        }
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }
}

/// An accepted status connection with its place of Lm. The place goes back
/// when the connection is dropped.
struct Admitted {
    stream: TcpStream,
    _permit: BudgetPermit,
}

impl AsyncRead for Admitted {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().stream).poll_read(cx, buf)
    }
}

impl AsyncWrite for Admitted {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().stream).poll_write(cx, buf)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().stream).poll_write_vectored(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.stream.is_write_vectored()
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().stream).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().stream).poll_shutdown(cx)
    }
}
