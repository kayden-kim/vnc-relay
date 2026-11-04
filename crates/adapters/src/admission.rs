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
use vnc_relay_domain::budget::{BudgetPermit, ConnectionBudget};
use vnc_relay_domain::peer_ip::PeerIp;

use crate::listener::{Listener, is_fatal_accept_error};
use crate::net::ACCEPT_RETRY_DELAY;

/// Takes a place of Lm right after each accept. A connection without a
/// place is logged and closed before any byte is read (ops-info-v1 §2).
pub(crate) struct AdmittingListener<L> {
    listener: L,
    budget: Arc<ConnectionBudget>,
    log: Arc<dyn EventSink>,
    /// `listener` value of the operation events: `status` or `management`.
    name: &'static str,
}

impl<L: Listener> AdmittingListener<L> {
    pub(crate) fn new(
        listener: L,
        budget: Arc<ConnectionBudget>,
        log: Arc<dyn EventSink>,
        name: &'static str,
    ) -> Self {
        Self {
            listener,
            budget,
            log,
            name,
        }
    }
}

impl<L: Listener> axum::serve::Listener for AdmittingListener<L> {
    type Io = Admitted;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Admitted, SocketAddr) {
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
                listener: self.name,
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
