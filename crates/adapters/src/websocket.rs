//! The WS and WSS viewer entry points (websocket-relay-v1 §1.1). Each
//! connection passes the source IP check at accept. Then, within U counted
//! from the accept, it runs the TLS handshake (WSS only) and the upgrade
//! request. After the 101 is flushed, the common relay service owns the
//! initial exchange and session. See docs/architectures/websocket-entry-point.md.

use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::time::Instant;
use vnc_relay_app::events::{AbortReason, ConnectionErrorKind, Event, Stage};
use vnc_relay_app::ports::{Clock, EventSink};
use vnc_relay_app::relay::{ConnectionContext, RelayRuntime, RelayService};
use vnc_relay_domain::budget::BudgetPermit;
use vnc_relay_domain::entry_point::EntryPoint;
use vnc_relay_domain::origin::Origin;
use vnc_relay_domain::session::ViewerTransport;

use crate::listener::Listener;
use crate::net::{AcceptPolicy, Accepted, accept_connections};
use crate::target::TcpTargetConnector;
use crate::tls::TlsSource;
use crate::tls_listener::{HandshakeFailure, TlsIo};
use crate::upgrade::{HeadRead, Refusal, check, read_head, switching_protocols};
use crate::websocket_stream::WsRelayStream;

/// What a WS or WSS entry point applies to each connection.
#[derive(Debug, Clone)]
pub struct WsPolicy {
    /// The allow list, L, and H. For a WebSocket connection H counts from the
    /// 101, not from the accept, because the initial exchange starts after the
    /// upgrade. The `handshake` field's doc in `AcceptPolicy` describes the
    /// TCP case.
    pub accept: AcceptPolicy,
    /// U. It counts from the accept and covers the TLS handshake and the
    /// upgrade request (websocket-relay-v1 §1.1).
    pub upgrade: Duration,
    /// Origins a browser may send, from `allowed_origins`. An empty list
    /// refuses every Origin.
    pub origins: Vec<Origin>,
    /// M, the validated maximum logical WebSocket message length.
    pub message_bytes: usize,
    /// B, the validated relay buffer size per direction.
    pub buffer_bytes: usize,
}

/// A WS or WSS viewer entry point. `tls` is the WSS acceptor source and is
/// `None` for WS.
pub struct WsEntryPoint<L = TcpListener> {
    entry_point: EntryPoint,
    listener: L,
    tls: Option<TlsSource>,
    policy: WsPolicy,
    service: Arc<RelayService>,
    runtime: Arc<RelayRuntime<TcpTargetConnector>>,
    clock: Arc<dyn Clock>,
}

impl<L: Listener> WsEntryPoint<L> {
    /// Wraps a bound listener of a WS or WSS viewer entry point.
    pub fn new(
        entry_point: EntryPoint,
        listener: L,
        tls: Option<TlsSource>,
        policy: WsPolicy,
        service: Arc<RelayService>,
        runtime: Arc<RelayRuntime<TcpTargetConnector>>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            entry_point,
            listener,
            tls,
            policy,
            service,
            runtime,
            clock,
        }
    }

    /// Accepts connections until `stop` holds true, with the same accept loop
    /// as the TCP entry points.
    pub async fn run(self, log: Arc<dyn EventSink>, stop: watch::Receiver<bool>) {
        let accept = self.policy.accept.clone();
        let shared = Arc::new(WsShared {
            tls: self.tls,
            log: Arc::clone(&log),
            policy: self.policy,
            service: self.service,
            runtime: self.runtime,
            clock: self.clock,
        });
        accept_connections(
            self.entry_point,
            self.listener,
            accept,
            log,
            stop,
            move |stream, conn, permit, stop| {
                connection(stream, conn, permit, stop, Arc::clone(&shared))
            },
        )
        .await;
    }
}

/// What every connection of one entry point shares.
struct WsShared {
    tls: Option<TlsSource>,
    log: Arc<dyn EventSink>,
    policy: WsPolicy,
    service: Arc<RelayService>,
    runtime: Arc<RelayRuntime<TcpTargetConnector>>,
    clock: Arc<dyn Clock>,
}

/// The socket of one connection: plain TCP for WS, TLS over TCP for WSS.
enum Wire {
    Plain(TcpStream),
    /// Boxed: the rustls state is over 1 KiB and would set the size of every `Wire`.
    Tls(Box<TlsIo<TcpStream>>),
}

impl Wire {
    /// The stage of an abort: `tls` until the TLS handshake is done, `upgrade`
    /// after it. Plain TCP is always `upgrade`.
    fn stage(&self) -> Stage {
        match self {
            Self::Plain(_) => Stage::Upgrade,
            Self::Tls(tls) if tls.is_established() => Stage::Upgrade,
            Self::Tls(_) => Stage::Tls,
        }
    }

    fn handshake_failure(&self) -> Option<HandshakeFailure> {
        match self {
            Self::Plain(_) => None,
            Self::Tls(tls) => tls.handshake_failure(),
        }
    }
}

impl AsyncRead for Wire {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_read(cx, buf),
            Self::Tls(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for Wire {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_write(cx, buf),
            Self::Tls(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_write_vectored(cx, bufs),
            Self::Tls(stream) => Pin::new(stream).poll_write_vectored(cx, bufs),
        }
    }

    fn is_write_vectored(&self) -> bool {
        match self {
            Self::Plain(stream) => stream.is_write_vectored(),
            Self::Tls(stream) => stream.is_write_vectored(),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_flush(cx),
            Self::Tls(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_shutdown(cx),
            Self::Tls(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}

/// Runs one accepted connection through the TLS handshake (WSS only), the
/// upgrade request, and common relay pipeline. The place of L goes back before
/// an abort line is logged.
async fn connection(
    stream: TcpStream,
    conn: Accepted,
    permit: BudgetPermit,
    mut stop: watch::Receiver<bool>,
    shared: Arc<WsShared>,
) {
    let accepted_at = shared.clock.monotonic().saturating_sub(conn.at.elapsed());
    let mut wire = match &shared.tls {
        Some(source) => Wire::Tls(Box::new(TlsIo::handshaking(
            stream,
            source,
            conn.peer_ip,
            Arc::clone(&shared.log),
            conn.entry_point.name(),
        ))),
        None => Wire::Plain(stream),
    };
    let deadline = conn.at + shared.policy.upgrade;
    // Biased: on every wake-up the stop and U are polled before the upgrade.
    // A poll that is already inside `upgrade` is covered by its own U check
    // right before the answer (websocket-relay-v1 §1.1).
    let finished = tokio::select! {
        biased;
        _ = stop.wait_for(|stopped| *stopped) => None,
        () = tokio::time::sleep_until(deadline) => Some(Ok(Step::TimedOut)),
        done = upgrade(&mut wire, &conn, &shared) => Some(done),
    };
    let reason = match finished {
        Some(Ok(Step::Switched {
            suffix,
            exchange_at,
        })) => {
            let context = ConnectionContext {
                conn_id: conn.conn_id,
                entry_point: conn.entry_point,
                peer_ip: conn.peer_ip,
                peer_addr: conn.peer_addr,
                accepted_wall: conn.accepted_wall,
                accepted_at,
                exchange_at,
            };
            let transport = match conn.entry_point {
                EntryPoint::WssViewer => ViewerTransport::Wss,
                _ => ViewerTransport::Ws,
            };
            let stream = WsRelayStream::new(
                wire,
                suffix,
                shared.policy.message_bytes,
                shared.policy.buffer_bytes,
                transport,
                shared.policy.accept.keepalive,
            );
            let lifecycle = shared.runtime.lifecycle();
            let _handshaking = shared.runtime.track_handshaking();
            let prepared = shared
                .service
                .handle_connection(stream, context, permit, lifecycle)
                .await;
            drop(_handshaking);
            if let Ok(prepared) = prepared {
                let _ = shared.service.dispatch(prepared, &shared.runtime).await;
            }
            return;
        }
        Some(Ok(Step::Refused)) => None,
        Some(Ok(Step::TimedOut)) => Some(AbortReason::TlsUpgradeTimeout),
        Some(Ok(Step::PeerEof)) => Some(AbortReason::PeerEof),
        Some(Err(_io_error)) => io_reason(&wire),
        None => Some(AbortReason::ServerShutdown),
    };
    let stage = wire.stage();
    drop(wire);
    drop(permit);
    if let Some(reason) = reason {
        log_abort(&shared, &conn, stage, reason);
    }
}

/// How an upgrade attempt ended when no I/O error ended it first.
enum Step {
    /// The 101 was flushed. Preserve its completion time and any frame bytes.
    Switched {
        suffix: Vec<u8>,
        exchange_at: Duration,
    },
    /// A refusal was logged and answered. The connection is closed.
    Refused,
    /// The peer closed before the request head ended.
    PeerEof,
    /// U passed before the answer was written. Nothing is answered; the abort
    /// is logged as `tls_upgrade_timeout`.
    TimedOut,
}

/// Reads the request head, checks it, runs the pre-upgrade IP recheck, and
/// writes either the 101 or the refusal. Nothing is written once U is seen
/// to have passed.
async fn upgrade(wire: &mut Wire, conn: &Accepted, shared: &WsShared) -> io::Result<Step> {
    let read = read_head(wire).await?;
    let (decision, suffix) = match read {
        HeadRead::Complete { head, suffix } => {
            let decision = check(&head, &shared.policy.origins).and_then(|upgrade| {
                if shared.policy.accept.allow.permits(conn.peer_ip) {
                    Ok(upgrade)
                } else {
                    Err(Refusal::IpNotAllowed)
                }
            });
            (decision, suffix)
        }
        HeadRead::TooLarge => (Err(Refusal::Invalid), Vec::new()),
        HeadRead::Eof if !expired(conn, shared) => return Ok(Step::PeerEof),
        HeadRead::Eof => return Ok(Step::TimedOut),
    };
    // The answer bytes are built before the last U check, so only the refusal
    // log line runs between that check and the write.
    let (answer, refusal) = match decision {
        Ok(upgrade) => (switching_protocols(&upgrade), None),
        Err(refusal) => (refusal.answer(), Some(refusal)),
    };
    if expired(conn, shared) {
        return Ok(Step::TimedOut);
    }
    match refusal {
        None => {
            wire.write_all(&answer).await?;
            wire.flush().await?;
            let exchange_at = shared.clock.monotonic();
            Ok(Step::Switched {
                suffix,
                exchange_at,
            })
        }
        Some(refusal) => {
            refuse(wire, conn, shared, &refusal, &answer).await;
            Ok(Step::Refused)
        }
    }
}

/// Whether U has passed since accept (websocket-relay-v1 §1.1).
fn expired(conn: &Accepted, shared: &WsShared) -> bool {
    Instant::now() >= conn.at + shared.policy.upgrade
}

/// Logs a refusal (`connection_rejected`, stage `upgrade`), then writes its
/// answer and closes. The log comes first; the answer is best effort.
async fn refuse(
    wire: &mut Wire,
    conn: &Accepted,
    shared: &WsShared,
    refusal: &Refusal,
    answer: &[u8],
) {
    shared.log.record(&Event::UpgradeRejected {
        listener: conn.entry_point.name(),
        conn_id: conn.conn_id,
        reason: refusal.reason(),
        peer_ip: conn.peer_ip,
        origin: refusal.origin().map(str::to_owned),
    });
    let _ = wire.write_all(answer).await;
    let _ = wire.shutdown().await;
}

/// The abort reason of an I/O error on the wire. A TLS version refusal has no
/// line here: the TLS layer already logged it as `connection_rejected`.
fn io_reason(wire: &Wire) -> Option<AbortReason> {
    match wire.handshake_failure() {
        Some(HandshakeFailure::VersionUnsupported) => None,
        Some(HandshakeFailure::Negotiation) => {
            Some(AbortReason::ConnectionError(ConnectionErrorKind::TlsFailed))
        }
        Some(HandshakeFailure::PeerEof) => Some(AbortReason::PeerEof),
        Some(HandshakeFailure::Io) | None => {
            Some(AbortReason::ConnectionError(ConnectionErrorKind::IoError))
        }
    }
}

fn log_abort(shared: &WsShared, conn: &Accepted, stage: Stage, reason: AbortReason) {
    shared.log.record(&Event::ConnectionAborted {
        conn_id: conn.conn_id,
        entry_point: conn.entry_point,
        stage,
        reason,
        peer_ip: conn.peer_ip,
        elapsed_ms: u64::try_from(conn.at.elapsed().as_millis()).unwrap_or(u64::MAX),
    });
}
