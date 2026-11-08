//! The shared WS and WSS listeners (websocket-relay-v1 §1.1). Each
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
use vnc_relay_domain::entry_point::{EntryPoint, Listener, Role};
use vnc_relay_domain::peer_ip::PeerIp;

use crate::listener::Listener as SocketListener;
use crate::net::{AcceptPolicy, Accepted, accept_connections};
use crate::target::TcpTargetConnector;
use crate::tls::TlsSource;
use crate::tls_listener::{HandshakeFailure, TlsIo};
use crate::upgrade::{HeadRead, Refusal, check, read_head, switching_protocols};
use crate::websocket_stream::WsRelayStream;

/// What a WS or WSS entry point applies to each connection.
#[derive(Debug, Clone)]
pub struct WsPolicy {
    /// Live settings source and the count-only L budget. Accept fixes B/U/K;
    /// upgrade fixes M/Kw, then initial exchange fixes H after the 101 flush.
    pub accept: AcceptPolicy,
}

/// A shared WS or WSS listener. `tls` is the WSS acceptor source and is
/// `None` for WS.
pub struct WsEntryPoint<L = TcpListener> {
    relay_listener: Listener,
    listener: L,
    tls: Option<TlsSource>,
    policy: WsPolicy,
    service: Arc<RelayService>,
    runtime: Arc<RelayRuntime<TcpTargetConnector>>,
    clock: Arc<dyn Clock>,
}

impl<L: SocketListener> WsEntryPoint<L> {
    /// Wraps a bound WS or WSS listener.
    pub fn new(
        relay_listener: Listener,
        listener: L,
        tls: Option<TlsSource>,
        policy: WsPolicy,
        service: Arc<RelayService>,
        runtime: Arc<RelayRuntime<TcpTargetConnector>>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            relay_listener,
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
            self.relay_listener,
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
            conn.listener.name(),
        ))),
        None => Wire::Plain(stream),
    };
    let deadline = conn.at + conn.settings.upgrade;
    let mut role = None;
    let mut forwarded_ip = None;
    // Biased: on every wake-up the stop and U are polled before the upgrade.
    // A poll that is already inside `upgrade` is covered by its own U check
    // right before the answer (websocket-relay-v1 §1.1).
    let finished = tokio::select! {
        biased;
        _ = stop.wait_for(|stopped| *stopped) => None,
        () = tokio::time::sleep_until(deadline) => Some(Ok(Step::TimedOut)),
        done = upgrade(&mut wire, &conn, &shared, &mut role, &mut forwarded_ip) => Some(done),
    };
    let reason = match finished {
        Some(Ok(Step::Switched {
            suffix,
            exchange_at,
            settings,
            handshake,
            entry_point,
        })) => {
            let context = ConnectionContext {
                conn_id: conn.conn_id,
                entry_point,
                peer_ip: conn.peer_ip,
                forwarded_ip,
                peer_addr: conn.peer_addr,
                accepted_wall: conn.accepted_wall,
                accepted_at,
                exchange_at,
                buffer_bytes: conn.settings.buffer_bytes,
                handshake,
            };
            let transport = conn.listener.transport();
            let stream = WsRelayStream::new(
                wire,
                suffix,
                settings.message_bytes,
                conn.settings.buffer_bytes,
                transport,
                settings.keepalive,
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
        log_abort(&shared, &conn, role, forwarded_ip, stage, reason);
    }
}

/// How an upgrade attempt ended when no I/O error ended it first.
enum Step {
    /// The 101 was flushed. Preserve its completion time and any frame bytes.
    Switched {
        suffix: Vec<u8>,
        exchange_at: Duration,
        settings: UpgradedSettings,
        handshake: Duration,
        entry_point: EntryPoint,
    },
    /// A refusal was logged and answered. The connection is closed.
    Refused,
    /// The peer closed before the request head ended.
    PeerEof,
    /// U passed before the answer was written. Nothing is answered; the abort
    /// is logged as `tls_upgrade_timeout`.
    TimedOut,
}

/// M and Kw belong to this upgrade, independently of accept-time B/U/K.
#[derive(Debug, Clone, Copy)]
struct UpgradedSettings {
    message_bytes: usize,
    keepalive: vnc_relay_app::settings::KeepaliveSettings,
}

fn check_upgrade(
    head: &[u8],
    peer_ip: vnc_relay_domain::peer_ip::PeerIp,
    policy: &WsPolicy,
    role: &mut Option<Role>,
    forwarded_ip: &mut Option<PeerIp>,
) -> Result<(crate::upgrade::ValidUpgrade, UpgradedSettings), Refusal> {
    let mut decision = Err(Refusal::Invalid);
    policy.accept.allow.with_current(&mut |settings| {
        let Some(listener) = settings.relay.websocket(policy.accept.allow.listener()) else {
            return;
        };
        decision = check(head, listener, role, peer_ip, forwarded_ip).and_then(|upgrade| {
            let entry = EntryPoint::from_parts(policy.accept.allow.listener(), upgrade.role);
            if entry
                .and_then(|entry| settings.relay.allow(entry))
                .is_some_and(|allow| allow.permits(forwarded_ip.unwrap_or(peer_ip)))
            {
                Ok((
                    upgrade,
                    UpgradedSettings {
                        message_bytes: settings.limits.ws_message_bytes as usize,
                        keepalive: settings.keepalive,
                    },
                ))
            } else {
                Err(Refusal::IpNotAllowed)
            }
        });
    });
    decision
}

/// Reads the request head, checks it, runs the pre-upgrade IP recheck, and
/// writes either the 101 or the refusal. Nothing is written once U is seen
/// to have passed.
async fn upgrade(
    wire: &mut Wire,
    conn: &Accepted,
    shared: &WsShared,
    role: &mut Option<Role>,
    forwarded_ip: &mut Option<PeerIp>,
) -> io::Result<Step> {
    let read = read_head(wire).await?;
    let (decision, suffix) = match read {
        HeadRead::Complete { head, suffix } => {
            let decision = check_upgrade(&head, conn.peer_ip, &shared.policy, role, forwarded_ip);
            (decision, suffix)
        }
        HeadRead::TooLarge => (Err(Refusal::Invalid), Vec::new()),
        HeadRead::Eof if !expired(conn) => return Ok(Step::PeerEof),
        HeadRead::Eof => return Ok(Step::TimedOut),
    };
    // The answer bytes are built before the last U check, so only the refusal
    // log line runs between that check and the write.
    let (answer, refusal) = match decision {
        Ok((upgrade, settings)) => (switching_protocols(&upgrade), Ok((upgrade.role, settings))),
        Err(refusal) => (refusal.answer(), Err(refusal)),
    };
    if expired(conn) {
        return Ok(Step::TimedOut);
    }
    match refusal {
        Ok((selected_role, settings)) => {
            let Some(entry_point) = EntryPoint::from_parts(conn.listener, selected_role) else {
                return Err(io::Error::other("WebSocket listener has no selected role"));
            };
            wire.write_all(&answer).await?;
            wire.flush().await?;
            let exchange_at = shared.clock.monotonic();
            let handshake = exchange_handshake(&shared.policy);
            Ok(Step::Switched {
                suffix,
                exchange_at,
                settings,
                handshake,
                entry_point,
            })
        }
        Err(refusal) => {
            refuse(wire, conn, shared, *role, *forwarded_ip, &refusal, &answer).await;
            Ok(Step::Refused)
        }
    }
}

/// Whether U has passed since accept (websocket-relay-v1 §1.1).
fn expired(conn: &Accepted) -> bool {
    Instant::now() >= conn.at + conn.settings.upgrade
}

fn exchange_handshake(policy: &WsPolicy) -> Duration {
    let mut handshake = Duration::ZERO;
    policy.accept.allow.with_current(&mut |settings| {
        handshake = settings.timeouts.handshake;
    });
    handshake
}

/// Logs a refusal (`connection_rejected`, stage `upgrade`), then writes its
/// answer and closes. The log comes first; the answer is best effort.
async fn refuse(
    wire: &mut Wire,
    conn: &Accepted,
    shared: &WsShared,
    role: Option<Role>,
    forwarded_ip: Option<PeerIp>,
    refusal: &Refusal,
    answer: &[u8],
) {
    shared.log.record(&Event::UpgradeRejected {
        listener: conn.listener.name(),
        role,
        conn_id: conn.conn_id,
        reason: refusal.reason(),
        peer_ip: conn.peer_ip,
        forwarded_ip,
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

fn log_abort(
    shared: &WsShared,
    conn: &Accepted,
    role: Option<Role>,
    forwarded_ip: Option<PeerIp>,
    stage: Stage,
    reason: AbortReason,
) {
    shared.log.record(&Event::ConnectionAborted {
        conn_id: conn.conn_id,
        listener: conn.listener,
        role,
        stage,
        reason,
        peer_ip: conn.peer_ip,
        forwarded_ip,
        elapsed_ms: u64::try_from(conn.at.elapsed().as_millis()).unwrap_or(u64::MAX),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::{AcceptedSettings, AllowSource};
    use vnc_relay_app::reload::{
        ConfigCandidate,
        snapshot::{ReloadCell, ReloadSnapshot},
    };
    use vnc_relay_domain::budget::ConnectionBudget;

    const HEAD: &[u8] = b"GET /relay HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nOrigin: https://old.example\r\n\r\n";

    fn candidate(settings: vnc_relay_app::settings::Settings) -> ConfigCandidate {
        ConfigCandidate {
            settings,
            presence: Default::default(),
        }
    }

    /// spec: operations.md:OPS-050 partial
    /// spec: operations.md:OPS-047 partial
    #[test]
    fn reload_separates_accept_upgrade_and_exchange_values() {
        for entry in [EntryPoint::WsViewer, EntryPoint::WssViewer] {
            let mut settings = crate::config_file::parse(
                "[admin]\naccounts_file='accounts.toml'\n[management]\nlisten='127.0.0.1:8443'\n[status]\nlisten='127.0.0.1:9090'\n",
                std::path::Path::new("."),
            ).unwrap_or_else(|error| panic!("settings: {error:?}"));
            let allow = vnc_relay_domain::allow_list::AllowList::new(vec![
                vnc_relay_domain::allow_list::IpItem::parse("127.0.0.1").unwrap(),
            ]);
            let origin = vnc_relay_domain::origin::Origin::parse("https://old.example").unwrap();
            settings.relay.ws.viewer.enabled = true;
            settings.relay.wss.viewer.enabled = true;
            settings.relay.ws.viewer.allow = Some(allow.clone());
            settings.relay.wss.viewer.allow = Some(allow);
            settings.relay.ws.viewer.allowed_origins = vec![origin.clone()];
            settings.relay.wss.viewer.allowed_origins = vec![origin];
            let accepted = AcceptedSettings::from_settings(&settings);
            let cell = Arc::new(ReloadCell::<tokio_rustls::TlsAcceptor>::new(
                ReloadSnapshot::new(candidate(settings.clone()), None, None),
            ));
            let policy = WsPolicy {
                accept: AcceptPolicy {
                    allow: if entry == EntryPoint::WsViewer {
                        AllowSource::ws(Arc::clone(&cell))
                    } else {
                        AllowSource::wss(Arc::clone(&cell))
                    },
                    relay_budget: ConnectionBudget::new(),
                },
            };
            let peer = vnc_relay_domain::peer_ip::PeerIp::new("127.0.0.1".parse().unwrap());
            let (_, previous) = check_upgrade(HEAD, peer, &policy, &mut None, &mut None).unwrap();
            settings.limits.buffer_bytes *= 2;
            settings.limits.ws_message_bytes *= 2;
            settings.timeouts.handshake = Duration::from_secs(21);
            settings.timeouts.tls_upgrade = Duration::from_secs(22);
            settings.keepalive.tcp_idle = Duration::from_secs(90);
            settings.keepalive.ws_ping = Duration::from_secs(41);
            settings.keepalive.ws_pong_timeout = Duration::from_secs(42);
            cell.replace(ReloadSnapshot::new(candidate(settings.clone()), None, None));
            let (_, upgraded) = check_upgrade(HEAD, peer, &policy, &mut None, &mut None).unwrap();
            assert_eq!(
                accepted.buffer_bytes * 2,
                settings.limits.buffer_bytes as usize
            );
            assert_eq!(accepted.upgrade, Duration::from_secs(10));
            assert_eq!(accepted.handshake, Duration::from_secs(10));
            assert_eq!(accepted.keepalive.tcp_idle, Duration::from_secs(60));
            assert_eq!(
                upgraded.message_bytes,
                settings.limits.ws_message_bytes as usize
            );
            assert_eq!(upgraded.keepalive.ws_ping, Duration::from_secs(41));
            assert_eq!(upgraded.keepalive.ws_pong_timeout, Duration::from_secs(42));
            assert_eq!(previous.keepalive.ws_ping, Duration::from_secs(30));
            assert_eq!(previous.message_bytes * 2, upgraded.message_bytes);
            assert_eq!(exchange_handshake(&policy), Duration::from_secs(21));
            settings.timeouts.handshake = Duration::from_secs(31);
            settings.relay.ws.viewer.allowed_origins.clear();
            settings.relay.wss.viewer.allowed_origins.clear();
            cell.replace(ReloadSnapshot::new(candidate(settings), None, None));
            assert_eq!(exchange_handshake(&policy), Duration::from_secs(31));
            assert!(matches!(
                check_upgrade(HEAD, peer, &policy, &mut None, &mut None),
                Err(Refusal::OriginNotAllowed(_))
            ));
            assert_eq!(upgraded.keepalive.ws_ping, Duration::from_secs(41));
        }
    }
}
