//! Wraps the connections of a listener in TLS. The handshake runs inside the
//! first read or write of a connection, so accepting a connection never waits
//! for its handshake. One stalled client does not hold up the others.
//! Design: docs/architectures/management-listener.md.

use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, ready};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_rustls::server::{Accept, TlsStream};
use vnc_relay_app::events::{Event, RejectReason, Stage};
use vnc_relay_app::ports::EventSink;
use vnc_relay_domain::peer_ip::PeerIp;

use crate::tls::TlsSource;

/// A listener whose connections run TLS. It takes each connection from
/// `inner` unchanged and wraps it in a `TlsIo`.
pub(crate) struct TlsListener<L> {
    inner: L,
    source: TlsSource,
    log: Arc<dyn EventSink>,
    /// `listener` value of the operation events.
    name: &'static str,
}

impl<L> TlsListener<L> {
    pub(crate) fn new(
        inner: L,
        source: TlsSource,
        log: Arc<dyn EventSink>,
        name: &'static str,
    ) -> Self {
        Self {
            inner,
            source,
            log,
            name,
        }
    }
}

impl<L> axum::serve::Listener for TlsListener<L>
where
    L: axum::serve::Listener<Addr = SocketAddr>,
{
    type Io = TlsIo<L::Io>;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (TlsIo<L::Io>, SocketAddr) {
        let (stream, addr) = self.inner.accept().await;
        let io = TlsIo::handshaking(
            stream,
            &self.source,
            PeerIp::from(addr),
            Arc::clone(&self.log),
            self.name,
        );
        (io, addr)
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.inner.local_addr()
    }
}

/// One accepted connection. Its handshake runs inside the first read or write.
pub(crate) enum TlsIo<IO> {
    Handshaking(Handshake<IO>),
    Established(TlsStream<Recorded<IO>>),
    /// The handshake failed. Every later call fails too.
    Failed(HandshakeFailure),
}

/// A handshake in progress, with what a refusal needs for its event.
pub(crate) struct Handshake<IO> {
    accept: Accept<Recorded<IO>>,
    capture: Capture,
    peer: PeerIp,
    log: Arc<dyn EventSink>,
    name: &'static str,
}

/// Why a handshake failed. The entry points use it to pick the event of the
/// connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HandshakeFailure {
    /// The client offered no TLS 1.2 or later (OPS-036). The rejection is
    /// logged.
    VersionUnsupported,
    /// rustls refused the handshake, for example after a fatal alert from the
    /// client or when no cipher suite is shared.
    Negotiation,
    /// The client closed the connection before the handshake ended.
    PeerEof,
    /// A socket error ended the handshake.
    Io,
}

impl HandshakeFailure {
    /// Classifies an error that the version check did not claim.
    fn from_error(error: &io::Error) -> Self {
        if error
            .get_ref()
            .is_some_and(|inner| inner.is::<rustls::Error>())
        {
            Self::Negotiation
        } else if error.kind() == io::ErrorKind::UnexpectedEof {
            Self::PeerEof
        } else {
            Self::Io
        }
    }
}

impl<IO: AsyncRead + AsyncWrite + Unpin> TlsIo<IO> {
    /// A connection whose handshake has not started. The handshake runs in the
    /// first read or write, as in `TlsListener::accept`. The acceptor is taken
    /// from `source` at this accept, so the connection uses the acceptor
    /// current then. With no acceptor current, the connection starts failed.
    pub(crate) fn handshaking(
        io: IO,
        source: &TlsSource,
        peer: PeerIp,
        log: Arc<dyn EventSink>,
        name: &'static str,
    ) -> Self {
        let Some(acceptor) = source.acceptor() else {
            // No acceptor is current for this listener. The connection takes
            // the negotiation abort path, and no handshake starts.
            return Self::Failed(HandshakeFailure::Negotiation);
        };
        let capture: Capture = Arc::new(Mutex::new(Some(Vec::new())));
        Self::Handshaking(Handshake {
            accept: acceptor.accept(Recorded {
                io,
                capture: Arc::clone(&capture),
            }),
            capture,
            peer,
            log,
            name,
        })
    }

    /// Whether the handshake has ended in a TLS stream.
    pub(crate) fn is_established(&self) -> bool {
        matches!(self, Self::Established(_))
    }

    /// Why the handshake failed, or `None` while it runs or after it succeeded.
    pub(crate) fn handshake_failure(&self) -> Option<HandshakeFailure> {
        match self {
            Self::Failed(failure) => Some(*failure),
            Self::Handshaking(_) | Self::Established(_) => None,
        }
    }

    /// Drives the handshake. `Ready(Ok)` once the connection is established,
    /// `Ready(Err)` once the handshake has failed.
    fn poll_handshake(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self {
            Self::Established(_) => Poll::Ready(Ok(())),
            Self::Failed(_) => Poll::Ready(Err(not_connected())),
            Self::Handshaking(handshake) => {
                let outcome = ready!(Pin::new(&mut handshake.accept).poll(cx));
                match outcome {
                    Ok(stream) => {
                        // The handshake is over, so stop copying client bytes.
                        *lock(&handshake.capture) = None;
                        *self = Self::Established(stream);
                        Poll::Ready(Ok(()))
                    }
                    Err(error) => {
                        // Logged before the close, so a peer that sees the
                        // close also finds the line.
                        let hello = lock(&handshake.capture).take().unwrap_or_default();
                        let failure = if offers_no_tls12(&hello) {
                            handshake.log.record(&Event::ConnectionRejected {
                                listener: handshake.name,
                                stage: Stage::Tls,
                                reason: RejectReason::TlsVersionUnsupported,
                                peer_ip: handshake.peer,
                            });
                            HandshakeFailure::VersionUnsupported
                        } else {
                            HandshakeFailure::from_error(&error)
                        };
                        *self = Self::Failed(failure);
                        Poll::Ready(Err(error))
                    }
                }
            }
        }
    }
}

/// The client bytes that arrive before the handshake ends, or `None` once it
/// is over or the bytes were taken.
type Capture = Arc<Mutex<Option<Vec<u8>>>>;

/// Most client bytes kept for classification: one TLS record. A ClientHello
/// that spans several records is not classified.
const CAPTURE_LIMIT: usize = 5 + 16_384;

/// Copies the client's bytes into `capture` until the handshake ends.
pub(crate) struct Recorded<IO> {
    io: IO,
    capture: Capture,
}

impl<IO: AsyncRead + Unpin> AsyncRead for Recorded<IO> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let before = buf.filled().len();
        ready!(Pin::new(&mut this.io).poll_read(cx, &mut *buf))?;
        if let Some(bytes) = lock(&this.capture).as_mut() {
            let room = CAPTURE_LIMIT.saturating_sub(bytes.len());
            let fresh = buf.filled().get(before..).unwrap_or_default();
            bytes.extend(fresh.iter().take(room));
        }
        Poll::Ready(Ok(()))
    }
}

impl<IO: AsyncWrite + Unpin> AsyncWrite for Recorded<IO> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().io).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().io).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().io).poll_shutdown(cx)
    }
}

/// Locks a capture. A poisoned lock still holds the bytes, so it is used as is.
fn lock(capture: &Capture) -> MutexGuard<'_, Option<Vec<u8>>> {
    capture.lock().unwrap_or_else(PoisonError::into_inner)
}

/// `ContentType` of a handshake record (RFC 8446 §5.1).
const HANDSHAKE_RECORD: u8 = 0x16;
/// `HandshakeType` of a ClientHello (RFC 8446 §4.1.2).
const CLIENT_HELLO: u8 = 0x01;
/// `ExtensionType` of supported_versions (RFC 8446 §4.2.1).
const SUPPORTED_VERSIONS: u16 = 0x002B;
const TLS12: u16 = 0x0303;
const TLS13: u16 = 0x0304;

/// `true` when the ClientHello in `bytes` offers no TLS 1.2 or later. `false`
/// when it offers one, or when `bytes` do not hold a whole ClientHello.
fn offers_no_tls12(bytes: &[u8]) -> bool {
    offers_tls12(bytes) == Some(false)
}

/// Whether the ClientHello in `bytes` offers TLS 1.2 or later. `None` when
/// `bytes` do not hold a whole ClientHello. `supported_versions` decides when
/// present; otherwise `legacy_version` does (RFC 8446 §4.2.1).
fn offers_tls12(bytes: &[u8]) -> Option<bool> {
    let mut record = Reader(bytes);
    if record.byte()? != HANDSHAKE_RECORD {
        return None;
    }
    record.skip(2)?; // record version
    let mut hello = Reader(record.vector(2)?);
    if hello.byte()? != CLIENT_HELLO {
        return None;
    }
    let mut body = Reader(hello.vector(3)?);
    let legacy_version = body.word()?;
    body.skip(32)?; // random
    body.vector(1)?; // legacy_session_id
    body.vector(2)?; // cipher_suites
    body.vector(1)?; // legacy_compression_methods
    if body.is_empty() {
        return Some(legacy_version >= TLS12);
    }
    let mut extensions = Reader(body.vector(2)?);
    while !extensions.is_empty() {
        let kind = extensions.word()?;
        let data = extensions.vector(2)?;
        if kind == SUPPORTED_VERSIONS {
            let versions = Reader(data).vector(1)?;
            return Some(
                versions
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|&pair| u16::from_be_bytes(pair))
                    .any(|version| version == TLS12 || version == TLS13),
            );
        }
    }
    Some(legacy_version >= TLS12)
}

/// Reads a TLS wire structure front to back. Each read returns `None` when
/// the input is too short.
struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let (head, tail) = self.0.split_at_checked(count)?;
        self.0 = tail;
        Some(head)
    }

    fn skip(&mut self, count: usize) -> Option<()> {
        self.take(count).map(|_| ())
    }

    fn byte(&mut self) -> Option<u8> {
        self.take(1)?.first().copied()
    }

    fn word(&mut self) -> Option<u16> {
        <[u8; 2]>::try_from(self.take(2)?)
            .ok()
            .map(u16::from_be_bytes)
    }

    /// A vector with a `width`-byte big-endian length prefix.
    fn vector(&mut self, width: usize) -> Option<&'a [u8]> {
        let length = self
            .take(width)?
            .iter()
            .fold(0usize, |sum, &byte| (sum << 8) | usize::from(byte));
        self.take(length)
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

fn not_connected() -> io::Error {
    io::Error::from(io::ErrorKind::NotConnected)
}

impl<IO: AsyncRead + AsyncWrite + Unpin> AsyncRead for TlsIo<IO> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.poll_handshake(cx))?;
        match this {
            Self::Established(stream) => Pin::new(stream).poll_read(cx, buf),
            Self::Handshaking(_) | Self::Failed(_) => Poll::Ready(Err(not_connected())),
        }
    }
}

impl<IO: AsyncRead + AsyncWrite + Unpin> AsyncWrite for TlsIo<IO> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        ready!(this.poll_handshake(cx))?;
        match this {
            Self::Established(stream) => Pin::new(stream).poll_write(cx, buf),
            Self::Handshaking(_) | Self::Failed(_) => Poll::Ready(Err(not_connected())),
        }
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        ready!(this.poll_handshake(cx))?;
        match this {
            Self::Established(stream) => Pin::new(stream).poll_write_vectored(cx, bufs),
            Self::Handshaking(_) | Self::Failed(_) => Poll::Ready(Err(not_connected())),
        }
    }

    fn is_write_vectored(&self) -> bool {
        match self {
            Self::Established(stream) => stream.is_write_vectored(),
            Self::Handshaking(_) | Self::Failed(_) => false,
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.poll_handshake(cx))?;
        match this {
            Self::Established(stream) => Pin::new(stream).poll_flush(cx),
            Self::Handshaking(_) | Self::Failed(_) => Poll::Ready(Err(not_connected())),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Established(stream) => Pin::new(stream).poll_shutdown(cx),
            // Nothing was established, so there is no TLS stream to close.
            Self::Handshaking(_) | Self::Failed(_) => Poll::Ready(Ok(())),
        }
    }
}
