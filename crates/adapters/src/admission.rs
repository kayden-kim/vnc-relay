//! Lm admission for the admin listeners (status and management). Each accepted
//! connection takes a place of Lm, and the place goes back when the connection
//! is dropped (ops-info-v1 §1.5, OPS-033).
//! Design: docs/architectures/management-listener.md.

use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
use vnc_relay_app::events::{Event, RejectReason, Stage};
use vnc_relay_app::ports::EventSink;
use vnc_relay_app::reload::ports::RuntimeSettingsSource;
use vnc_relay_domain::budget::{BudgetPermit, ConnectionBudget};
use vnc_relay_domain::peer_ip::PeerIp;

use crate::listener::{Listener, is_fatal_accept_error};
use crate::net::ACCEPT_RETRY_DELAY;

/// Takes a place of Lm right after each accept. A connection without a
/// place is logged and closed before any byte is read (ops-info-v1 §2).
pub(crate) struct AdmittingListener<L> {
    listener: L,
    budget: Arc<ConnectionBudget>,
    runtime: Arc<dyn RuntimeSettingsSource>,
    log: Arc<dyn EventSink>,
    /// `listener` value of the operation events: `status` or `management`.
    name: &'static str,
}

impl<L: Listener> AdmittingListener<L> {
    pub(crate) fn new(
        listener: L,
        budget: Arc<ConnectionBudget>,
        runtime: Arc<dyn RuntimeSettingsSource>,
        log: Arc<dyn EventSink>,
        name: &'static str,
    ) -> Self {
        Self {
            listener,
            budget,
            runtime,
            log,
            name,
        }
    }
}

impl<L: Listener> axum::serve::Listener for AdmittingListener<L> {
    type Io = Admitted;
    type Addr = AdmittedAddr;

    async fn accept(&mut self) -> (Admitted, AdmittedAddr) {
        loop {
            let (stream, addr) = match self.listener.accept().await {
                Ok(accepted) => accepted,
                Err(error) => {
                    if is_fatal_accept_error(&error) {
                        // Axum's infallible accept port cannot return an error.
                        // The supervisor owns shutdown and aborts this task last.
                        return std::future::pending().await;
                    }
                    tokio::time::sleep(ACCEPT_RETRY_DELAY).await;
                    continue;
                }
            };
            let at = tokio::time::Instant::now();
            let (mut permit, mut limit, mut upgrade) = (None, 0, std::time::Duration::ZERO);
            self.runtime.with_current(&mut |settings| {
                limit = settings.limits.admin_connections;
                upgrade = settings.timeouts.tls_upgrade;
                permit = self.budget.try_acquire(limit);
            });
            if let Some(permit) = permit {
                return (
                    Admitted {
                        stream,
                        _permit: permit,
                    },
                    AdmittedAddr {
                        socket: addr,
                        at,
                        upgrade,
                    },
                );
            }
            // Logged before the close, so a peer that sees the close also
            // finds the line.
            self.log.record(&Event::ConnectionRejected {
                listener: self.name,
                stage: Stage::Accept,
                reason: RejectReason::AdminConnectionLimit { limit },
                peer_ip: PeerIp::from(addr),
            });
            drop(stream);
        }
    }

    fn local_addr(&self) -> io::Result<AdmittedAddr> {
        self.listener.local_addr().map(|socket| AdmittedAddr {
            socket,
            at: tokio::time::Instant::now(),
            upgrade: std::time::Duration::ZERO,
        })
    }
}

/// Accept-time metadata crosses the optional TLS wrapper without rereading U.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AdmittedAddr {
    pub(crate) socket: SocketAddr,
    pub(crate) at: tokio::time::Instant,
    pub(crate) upgrade: std::time::Duration,
}

/// An accepted admin connection with its place of Lm. The place goes back
/// when the connection is dropped.
pub(crate) struct Admitted {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;
    use vnc_relay_app::reload::{
        ConfigCandidate,
        snapshot::{ReloadCell, ReloadSnapshot},
    };

    struct Events(tokio::sync::mpsc::UnboundedSender<Event>);

    impl EventSink for Events {
        fn record(&self, event: &Event) {
            self.0.send(event.clone()).unwrap();
        }
    }

    /// spec: operations.md:OPS-050 partial
    /// spec: operations.md:OPS-051 partial
    /// spec: operations.md:OPS-052 partial
    /// spec: operations.md:OPS-053 partial
    #[tokio::test]
    async fn reloaded_admin_limit_preserves_connections_and_captures_u_at_accept() {
        let mut settings = crate::config_file::parse(
            "[admin]\naccounts_file='accounts.toml'\n[management]\nlisten='127.0.0.1:8443'\n[status]\nlisten='127.0.0.1:9090'\n",
            std::path::Path::new("."),
        ).unwrap();
        settings.limits.admin_connections = 2;
        settings.timeouts.tls_upgrade = std::time::Duration::from_secs(3);
        let candidate = |settings| ConfigCandidate {
            settings,
            presence: Default::default(),
        };
        let source = Arc::new(ReloadCell::<()>::new(ReloadSnapshot::new(
            candidate(settings.clone()),
            None,
            None,
        )));
        let budget = ConnectionBudget::new();
        let socket = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = socket.local_addr().unwrap();
        let (events, mut received) = tokio::sync::mpsc::unbounded_channel();
        let mut listener = AdmittingListener::new(
            socket,
            budget.clone(),
            source.clone(),
            Arc::new(Events(events)),
            "management",
        );
        let _first_client = TcpStream::connect(address).await.unwrap();
        let (first, accepted) = axum::serve::Listener::accept(&mut listener).await;
        assert_eq!(accepted.upgrade, std::time::Duration::from_secs(3));
        settings.limits.admin_connections = 1;
        settings.timeouts.tls_upgrade = std::time::Duration::from_secs(7);
        source.replace(ReloadSnapshot::new(candidate(settings), None, None));
        assert_eq!(budget.current(), 1);
        let mut refused = TcpStream::connect(address).await.unwrap();
        let task = tokio::spawn(async move { axum::serve::Listener::accept(&mut listener).await });
        let event = tokio::time::timeout(std::time::Duration::from_secs(5), received.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(
            event,
            Event::ConnectionRejected {
                reason: RejectReason::AdminConnectionLimit { limit: 1 },
                ..
            }
        ));
        let mut byte = [0];
        assert_eq!(
            tokio::io::AsyncReadExt::read(&mut refused, &mut byte)
                .await
                .unwrap(),
            0
        );
        assert_eq!(budget.current(), 1);
        drop(first);
        let _next_client = TcpStream::connect(address).await.unwrap();
        let (next, accepted_next) = tokio::time::timeout(std::time::Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(accepted.upgrade, std::time::Duration::from_secs(3));
        assert_eq!(accepted_next.upgrade, std::time::Duration::from_secs(7));
        assert_eq!(budget.current(), 1);
        drop(next);
        assert_eq!(budget.current(), 0);
    }
}
