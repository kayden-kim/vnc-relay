//! The WS and WSS viewer entry points (websocket-relay-v1 §1.1). Each
//! connection passes the source IP check at accept. Then, within U counted
//! from the accept, it runs the TLS handshake (WSS only) and the upgrade
//! request. The 101 starts the hold of H, which the initial exchange (TODO
//! 2.7) will replace. See docs/architectures/websocket-entry-point.md.

use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::time::Instant;
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, Role};
use vnc_relay_app::events::{AbortReason, ConnectionErrorKind, Event, Stage};
use vnc_relay_app::ports::EventSink;
use vnc_relay_domain::budget::BudgetPermit;
use vnc_relay_domain::entry_point::EntryPoint;
use vnc_relay_domain::origin::Origin;

use crate::net::{AcceptPolicy, Accepted, accept_connections};
use crate::tls_listener::{HandshakeFailure, TlsIo};
use crate::upgrade::{HeadRead, Refusal, check, read_head, switching_protocols};

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
}

/// A WS or WSS viewer entry point. `tls` is the WSS acceptor and is `None`
/// for WS.
pub struct WsEntryPoint {
    entry_point: EntryPoint,
    listener: TcpListener,
    tls: Option<TlsAcceptor>,
    policy: WsPolicy,
}

impl WsEntryPoint {
    /// Wraps a bound listener of a WS or WSS viewer entry point.
    pub fn new(
        entry_point: EntryPoint,
        listener: TcpListener,
        tls: Option<TlsAcceptor>,
        policy: WsPolicy,
    ) -> Self {
        Self {
            entry_point,
            listener,
            tls,
            policy,
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
    tls: Option<TlsAcceptor>,
    log: Arc<dyn EventSink>,
    policy: WsPolicy,
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
/// upgrade request, and the hold after 101. The place of L goes back before
/// an abort line is logged.
async fn connection(
    stream: TcpStream,
    conn: Accepted,
    permit: BudgetPermit,
    mut stop: watch::Receiver<bool>,
    shared: Arc<WsShared>,
) {
    let mut wire = match &shared.tls {
        Some(acceptor) => Wire::Tls(Box::new(TlsIo::handshaking(
            stream,
            acceptor,
            conn.peer_ip,
            Arc::clone(&shared.log),
            conn.entry_point.name(),
        ))),
        None => Wire::Plain(stream),
    };
    let deadline = conn.at + shared.policy.upgrade;
    let finished = tokio::select! {
        _ = stop.wait_for(|stopped| *stopped) => None,
        done = tokio::time::timeout_at(deadline, upgrade(&mut wire, &conn, &shared)) => Some(done),
    };
    let reason = match finished {
        Some(Ok(Ok(Step::Switched))) => {
            hold_upgraded(wire, permit, &conn, &shared, stop).await;
            return;
        }
        Some(Ok(Ok(Step::Refused))) => None,
        Some(Ok(Ok(Step::PeerEof))) => Some(AbortReason::PeerEof),
        Some(Ok(Err(_io_error))) => io_reason(&wire),
        Some(Err(_elapsed)) => Some(AbortReason::TlsUpgradeTimeout),
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
    /// The 101 was written. The connection goes on to the hold.
    Switched,
    /// A refusal was logged and answered. The connection is closed.
    Refused,
    /// The peer closed before the request head ended.
    PeerEof,
}

/// Reads the request head, checks it, runs the pre-upgrade IP recheck, and
/// writes either the 101 or the refusal.
async fn upgrade(wire: &mut Wire, conn: &Accepted, shared: &WsShared) -> io::Result<Step> {
    let head = match read_head(wire).await? {
        HeadRead::Complete(head) => head,
        HeadRead::Eof => return Ok(Step::PeerEof),
        HeadRead::TooLarge => return Ok(refuse(wire, conn, shared, Refusal::Invalid).await),
    };
    let upgrade = match check(&head, &shared.policy.origins) {
        Ok(upgrade) => upgrade,
        Err(refusal) => return Ok(refuse(wire, conn, shared, refusal).await),
    };
    // websocket-relay-v1 §1.1: the source IP is checked again before the 101.
    if !shared.policy.accept.allow.permits(conn.peer_ip) {
        return Ok(refuse(wire, conn, shared, Refusal::IpNotAllowed).await);
    }
    wire.write_all(&switching_protocols(&upgrade)).await?;
    wire.flush().await?;
    Ok(Step::Switched)
}

/// Logs a refusal (`connection_rejected`, stage `upgrade`), then writes its
/// answer and closes. The log comes first; the answer is best effort.
async fn refuse(wire: &mut Wire, conn: &Accepted, shared: &WsShared, refusal: Refusal) -> Step {
    shared.log.record(&Event::UpgradeRejected {
        listener: conn.entry_point.name(),
        conn_id: conn.conn_id,
        reason: refusal.reason(),
        peer_ip: conn.peer_ip,
        origin: refusal.origin().map(str::to_owned),
    });
    let _ = wire.write_all(&refusal.answer()).await;
    let _ = wire.shutdown().await;
    Step::Refused
}

/// Holds an upgraded connection until H passes from the 101, the server
/// stops, or the Close frame is sent. No frame is read yet, so no frame
/// format is guessed.
async fn hold_upgraded(
    wire: Wire,
    permit: BudgetPermit,
    conn: &Accepted,
    shared: &WsShared,
    mut stop: watch::Receiver<bool>,
) {
    let until = Instant::now() + shared.policy.accept.handshake;
    let mut ws = WebSocketStream::from_raw_socket(wire, Role::Server, None).await;
    let (reason, code, text) = tokio::select! {
        _ = stop.wait_for(|stopped| *stopped) => {
            (AbortReason::ServerShutdown, CloseCode::Away, "server_shutdown")
        }
        () = tokio::time::sleep_until(until) => {
            (AbortReason::HandshakeTimeout, CloseCode::from(4009), "handshake_timeout")
        }
    };
    // A peer that already left makes the Close fail; the abort line is logged anyway.
    let _ = ws
        .close(Some(CloseFrame {
            code,
            reason: text.into(),
        }))
        .await;
    drop(ws);
    drop(permit);
    log_abort(shared, conn, Stage::InitialExchange, reason);
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
