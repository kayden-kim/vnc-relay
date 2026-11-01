//! Bounded request reading under a single absolute H deadline.

use std::future::poll_fn;
use std::pin::pin;
use std::task::Poll;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::watch;
use vnc_relay_domain::entry_point::{EntryPoint, Role};
use vnc_relay_domain::handshake::{
    HANDSHAKE_REQUEST_LEN, HandshakeParser, HandshakePolicy, HandshakeRequest, VIEWER_GREETING,
};

use super::RelayError;
use crate::events::AbortReason;
use crate::ports::{Clock, RelayLifecycle, RelayStreamError};

pub(super) async fn exchange<S>(
    stream: &mut S,
    entry_point: EntryPoint,
    modes: HandshakePolicy,
    buffer_bytes: usize,
    deadline: Duration,
    clock: &dyn Clock,
    mut shutdown: watch::Receiver<RelayLifecycle>,
) -> Result<(HandshakeRequest, Vec<u8>), RelayError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut stopped = pin!(shutdown.wait_for(|phase| *phase != RelayLifecycle::Running));
    let mut timeout = pin!(tokio::time::sleep(
        deadline.saturating_sub(clock.monotonic()),
    ));
    let mut exchange = pin!(read_request(stream, entry_point, modes, buffer_bytes));
    poll_fn(|cx| {
        if stopped.as_mut().poll(cx).is_ready() {
            return Poll::Ready(Err(RelayError::Aborted(AbortReason::ServerShutdown)));
        }
        if clock.monotonic() >= deadline || timeout.as_mut().poll(cx).is_ready() {
            return Poll::Ready(Err(RelayError::Aborted(AbortReason::HandshakeTimeout)));
        }
        let result = exchange.as_mut().poll(cx);
        if result.is_ready() {
            if stopped.as_mut().poll(cx).is_ready() {
                return Poll::Ready(Err(RelayError::Aborted(AbortReason::ServerShutdown)));
            }
            if clock.monotonic() >= deadline || timeout.as_mut().poll(cx).is_ready() {
                return Poll::Ready(Err(RelayError::Aborted(AbortReason::HandshakeTimeout)));
            }
        }
        result
    })
    .await
}

async fn read_request<S>(
    stream: &mut S,
    entry_point: EntryPoint,
    modes: HandshakePolicy,
    buffer_bytes: usize,
) -> Result<(HandshakeRequest, Vec<u8>), RelayError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    if entry_point.role() == Role::Viewer {
        stream.write_all(VIEWER_GREETING).await.map_err(io_error)?;
        stream.flush().await.map_err(io_error)?;
    }
    let mut parser = HandshakeParser::new();
    let mut remaining = HANDSHAKE_REQUEST_LEN;
    let mut scratch = [0_u8; 1024];
    loop {
        let capacity = scratch.len().min(remaining.saturating_add(buffer_bytes));
        let count = stream
            .read(&mut scratch[..capacity])
            .await
            .map_err(io_error)?;
        if count == 0 {
            return Err(RelayError::Aborted(AbortReason::PeerEof));
        }
        let consumed = parser.feed(&scratch[..count]);
        remaining -= consumed;
        if parser.is_complete() {
            let request = parser
                .finish(entry_point, modes)
                .map_err(RelayError::Rejected)?;
            return Ok((request, scratch[consumed..count].to_vec()));
        }
    }
}

fn io_error(error: std::io::Error) -> RelayError {
    RelayError::Aborted(if RelayStreamError::is_dead_connection(&error) {
        AbortReason::DeadConnection
    } else {
        AbortReason::ConnectionError(RelayStreamError::classify(&error))
    })
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::net::{IpAddr, Ipv4Addr};
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::task::Context;
    use std::time::SystemTime;

    use tokio::io::{DuplexStream, ReadBuf, duplex};
    use tokio::task::JoinHandle;
    use tokio::time::{Instant, advance};
    use vnc_relay_domain::budget::ConnectionBudget;
    use vnc_relay_domain::conn_id::ConnId;
    use vnc_relay_domain::handshake::{HandshakeError, Mode};
    use vnc_relay_domain::peer_ip::PeerIp;

    use super::*;
    use crate::events::{ConnectionErrorKind, Event, RejectReason, Stage};
    use crate::ports::{EventSink, RelayCloseAction, RelayStream};
    use crate::relay::{ConnectionContext, PreparedConnection, RelayService};
    use vnc_relay_domain::session::ViewerTransport;

    const H: Duration = Duration::from_secs(10);
    const ENABLED: HandshakePolicy = HandshakePolicy {
        mode1_enabled: true,
        mode2_enabled: true,
    };

    struct TestClock {
        epoch: Instant,
        offset_ms: AtomicU64,
    }

    impl TestClock {
        fn jump(&self, duration: Duration) {
            self.offset_ms.fetch_add(
                u64::try_from(duration.as_millis()).expect("test duration"),
                Ordering::Relaxed,
            );
        }
    }

    impl Clock for TestClock {
        fn monotonic(&self) -> Duration {
            self.epoch.elapsed() + Duration::from_millis(self.offset_ms.load(Ordering::Relaxed))
        }

        fn wall(&self) -> SystemTime {
            SystemTime::UNIX_EPOCH
        }
    }

    #[derive(Default)]
    struct Recorder {
        events: Mutex<Vec<Event>>,
        resources: Mutex<Option<(Arc<AtomicBool>, Arc<ConnectionBudget>)>>,
    }

    impl EventSink for Recorder {
        fn record(&self, event: &Event) {
            if let Some((dropped, budget)) = &*self.resources.lock().expect("resource lock") {
                assert!(
                    dropped.load(Ordering::SeqCst),
                    "stream must already be dropped"
                );
                drop(budget.try_acquire().expect("L must already be returned"));
            }
            self.events.lock().expect("events lock").push(event.clone());
        }
    }

    impl Recorder {
        fn snapshot(&self) -> Vec<Event> {
            self.events.lock().expect("events lock").clone()
        }
    }

    struct Harness {
        service: RelayService,
        clock: Arc<TestClock>,
        events: Arc<Recorder>,
        budget: Arc<ConnectionBudget>,
        shutdown: watch::Sender<RelayLifecycle>,
        close_actions: Arc<Mutex<Vec<RelayCloseAction>>>,
    }

    impl Harness {
        fn new(buffer_bytes: usize, modes: HandshakePolicy) -> Arc<Self> {
            let clock = Arc::new(TestClock {
                epoch: Instant::now(),
                offset_ms: AtomicU64::new(0),
            });
            let events = Arc::new(Recorder::default());
            let (shutdown, _) = watch::channel(RelayLifecycle::Running);
            Arc::new(Self {
                service: RelayService::new(
                    modes,
                    H,
                    buffer_bytes,
                    Duration::from_secs(3),
                    clock.clone(),
                    events.clone(),
                ),
                clock,
                events,
                budget: ConnectionBudget::new(1),
                shutdown,
                close_actions: Arc::new(Mutex::new(Vec::new())),
            })
        }

        fn start<S>(
            self: &Arc<Self>,
            stream: S,
            entry_point: EntryPoint,
        ) -> JoinHandle<Result<PreparedConnection<TestStream<S>>, RelayError>>
        where
            S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
        {
            let context = ConnectionContext {
                conn_id: ConnId::next(),
                entry_point,
                peer_ip: PeerIp::new(IpAddr::V4(Ipv4Addr::LOCALHOST)),
                peer_addr: std::net::SocketAddr::from((Ipv4Addr::LOCALHOST, 12345)),
                accepted_wall: self.clock.wall(),
                accepted_at: self.clock.monotonic(),
                exchange_at: self.clock.monotonic(),
            };
            let permit = self.budget.try_acquire().expect("test L slot");
            let shutdown = self.shutdown.subscribe();
            let harness = self.clone();
            let stream = TestStream {
                stream,
                actions: self.close_actions.clone(),
                budget: self.budget.clone(),
                block_close: false,
                failure: None,
            };
            tokio::spawn(async move {
                harness
                    .service
                    .handle_connection(stream, context, permit, shutdown)
                    .await
            })
        }
    }

    struct TestStream<S> {
        stream: S,
        actions: Arc<Mutex<Vec<RelayCloseAction>>>,
        budget: Arc<ConnectionBudget>,
        block_close: bool,
        failure: Option<ConnectionErrorKind>,
    }

    impl<S: AsyncRead + Unpin> AsyncRead for TestStream<S> {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buffer: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            if let Some(kind) = self.failure.take() {
                return Poll::Ready(Err(RelayStreamError::new(kind).into_io_error()));
            }
            Pin::new(&mut self.stream).poll_read(cx, buffer)
        }
    }

    impl<S: AsyncWrite + Unpin> AsyncWrite for TestStream<S> {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            bytes: &[u8],
        ) -> Poll<io::Result<usize>> {
            Pin::new(&mut self.stream).poll_write(cx, bytes)
        }
        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Pin::new(&mut self.stream).poll_flush(cx)
        }
        fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Pin::new(&mut self.stream).poll_shutdown(cx)
        }
    }

    impl<S: AsyncRead + AsyncWrite + Unpin + Send> RelayStream for TestStream<S> {
        fn transport(&self) -> ViewerTransport {
            ViewerTransport::Tcp
        }
        fn pending_write_len(&self) -> usize {
            0
        }
        fn poll_close(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            action: RelayCloseAction,
        ) -> Poll<io::Result<()>> {
            assert!(
                self.budget.try_acquire().is_none(),
                "close must precede L release"
            );
            self.actions.lock().expect("close actions").push(action);
            if self.block_close {
                return Poll::Pending;
            }
            Pin::new(&mut self.stream).poll_shutdown(cx)
        }
    }

    fn request(value: &[u8]) -> [u8; HANDSHAKE_REQUEST_LEN] {
        let mut bytes = [0; HANDSHAKE_REQUEST_LEN];
        bytes[..value.len()].copy_from_slice(value);
        bytes
    }

    fn assert_abort(harness: &Harness, reason: AbortReason) {
        let events = harness.events.snapshot();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            Event::ConnectionAborted {
                stage: Stage::InitialExchange,
                reason: actual,
                ..
            } if actual == reason
        ));
        assert!(harness.budget.try_acquire().is_some());
    }

    /// spec: relay.md:RLY-001 partial
    #[tokio::test(start_paused = true)]
    async fn sends_the_viewer_greeting_once_and_sends_none_to_the_server() {
        for entry_point in [EntryPoint::TcpViewer, EntryPoint::TcpServer] {
            let harness = Harness::new(8, ENABLED);
            let (stream, mut peer) = duplex(4096);
            let task = harness.start(stream, entry_point);
            if entry_point == EntryPoint::TcpViewer {
                let mut greeting = [0; 12];
                peer.read_exact(&mut greeting).await.expect("greeting");
                assert_eq!(&greeting, VIEWER_GREETING);
            }
            peer.write_all(&request(b"ID:0123")).await.expect("request");
            let prepared = task.await.expect("task").expect("prepared");
            assert_eq!(prepared.context().entry_point, entry_point);
            assert!(matches!(prepared.request(), HandshakeRequest::Mode2(_)));
            assert!(
                tokio::time::timeout(Duration::from_millis(1), peer.read(&mut [0; 1]))
                    .await
                    .is_err()
            );
            assert!(harness.events.snapshot().is_empty());
            assert!(harness.budget.try_acquire().is_none());
            drop(prepared);
            assert!(harness.budget.try_acquire().is_some());
        }
    }

    /// spec: relay.md:RLY-002 partial
    #[tokio::test(start_paused = true)]
    async fn parses_split_and_coalesced_requests_identically() {
        let mut expected = None;
        for size in [1, 7, 249, 250] {
            let harness = Harness::new(8, ENABLED);
            let (stream, mut peer) = duplex(4096);
            let task = harness.start(stream, EntryPoint::TcpServer);
            for chunk in request(b"ID:0123").chunks(size) {
                peer.write_all(chunk).await.expect("chunk");
                tokio::task::yield_now().await;
            }
            let prepared = task.await.expect("task").expect("prepared");
            if let Some(expected) = &expected {
                assert_eq!(prepared.request(), expected);
            } else {
                expected = Some(prepared.request().clone());
            }
            assert!(harness.events.snapshot().is_empty());
        }
    }

    /// spec: relay.md:RLY-003 partial
    #[tokio::test(start_paused = true)]
    async fn preserves_trailing_bytes_and_counts_them_against_b() {
        for buffer_bytes in [0, 4, 8] {
            let harness = Harness::new(buffer_bytes, ENABLED);
            let (stream, mut peer) = duplex(4096);
            let mut input = request(b"ID:0123").to_vec();
            input.extend_from_slice(b"abcdefghNEXT");
            peer.write_all(&input).await.expect("coalesced input");
            let prepared = harness
                .start(stream, EntryPoint::TcpServer)
                .await
                .expect("task")
                .expect("prepared");
            assert_eq!(prepared.trailing_bytes(), &b"abcdefgh"[..buffer_bytes]);
            assert_eq!(prepared.buffered_bytes(), buffer_bytes);
            assert_eq!(prepared.remaining_buffer_bytes(), 0);
            let (mut stream, _, _, prefix, permit) = prepared.into_parts();
            assert!(harness.budget.try_acquire().is_none());
            let mut remainder = vec![0; 12 - buffer_bytes];
            stream
                .read_exact(&mut remainder)
                .await
                .expect("unread suffix");
            let mut delivered = prefix;
            delivered.extend(remainder);
            assert_eq!(delivered, b"abcdefghNEXT");
            drop(stream);
            drop(permit);
            assert!(harness.budget.try_acquire().is_some());
            assert!(harness.events.snapshot().is_empty());
        }
    }

    /// spec: relay.md:RLY-004 partial
    /// spec: relay.md:RLY-022 partial
    #[tokio::test(start_paused = true)]
    async fn rejects_invalid_and_disabled_requests_without_preparing_a_connection() {
        let disabled = HandshakePolicy {
            mode1_enabled: false,
            mode2_enabled: false,
        };
        for (entry_point, value, modes, expected) in [
            (
                EntryPoint::TcpServer,
                b"invalid".as_slice(),
                ENABLED,
                HandshakeError::Invalid,
            ),
            (
                EntryPoint::TcpViewer,
                b"".as_slice(),
                ENABLED,
                HandshakeError::Invalid,
            ),
            (
                EntryPoint::TcpViewer,
                b"".as_slice(),
                disabled,
                HandshakeError::ModeDisabled(Mode::Mode1),
            ),
            (
                EntryPoint::TcpServer,
                b"ID:bad".as_slice(),
                disabled,
                HandshakeError::ModeDisabled(Mode::Mode2),
            ),
            (
                EntryPoint::TcpServer,
                b"".as_slice(),
                disabled,
                HandshakeError::Invalid,
            ),
        ] {
            let harness = Harness::new(8, modes);
            let (stream, mut peer) = duplex(4096);
            peer.write_all(&request(value)).await.expect("request");
            assert_eq!(
                harness
                    .start(stream, entry_point)
                    .await
                    .expect("task")
                    .unwrap_err(),
                RelayError::Rejected(expected)
            );
            let mut output = Vec::new();
            peer.read_to_end(&mut output).await.expect("closed");
            let greeting: &[u8] = if entry_point == EntryPoint::TcpViewer {
                VIEWER_GREETING
            } else {
                b""
            };
            assert_eq!(output, greeting);
            let (expected_reason, expected_mode) = match expected {
                HandshakeError::ModeDisabled(mode) => (RejectReason::ModeDisabled, Some(mode)),
                _ => (RejectReason::HandshakeInvalid, None),
            };
            let events = harness.events.snapshot();
            assert_eq!(events.len(), 1);
            assert!(matches!(
                events[0],
                Event::HandshakeRejected { entry_point: actual, reason, mode, .. }
                    if actual == entry_point && reason == expected_reason && mode == expected_mode
            ));
            let action = if matches!(expected, HandshakeError::ModeDisabled(_)) {
                RelayCloseAction::ModeDisabled
            } else {
                RelayCloseAction::HandshakeInvalid
            };
            assert_eq!(*harness.close_actions.lock().expect("actions"), [action]);
            assert!(harness.budget.try_acquire().is_some());
        }
    }

    /// spec: relay.md:RLY-005 partial
    /// spec: operations.md:OPS-006 partial
    #[tokio::test(start_paused = true)]
    async fn distinguishes_peer_eof_from_the_absolute_h_timeout() {
        for count in [0, 1, 249] {
            let harness = Harness::new(8, ENABLED);
            let (stream, mut peer) = duplex(4096);
            peer.write_all(&request(b"ID:1")[..count])
                .await
                .expect("partial");
            drop(peer);
            assert_eq!(
                harness
                    .start(stream, EntryPoint::TcpServer)
                    .await
                    .expect("task")
                    .unwrap_err(),
                RelayError::Aborted(AbortReason::PeerEof)
            );
            assert_abort(&harness, AbortReason::PeerEof);
        }
        let harness = Harness::new(8, ENABLED);
        let (stream, mut peer) = duplex(4096);
        peer.write_all(b"ID:").await.expect("partial");
        let task = harness.start(stream, EntryPoint::TcpServer);
        tokio::task::yield_now().await;
        advance(H).await;
        assert_eq!(
            task.await.expect("task").unwrap_err(),
            RelayError::Aborted(AbortReason::HandshakeTimeout)
        );
        assert_abort(&harness, AbortReason::HandshakeTimeout);
        assert_eq!(
            *harness.close_actions.lock().expect("actions"),
            [RelayCloseAction::HandshakeTimeout]
        );
        assert_eq!(peer.read(&mut [0; 1]).await.expect("closed"), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn does_not_restart_h_on_partial_reads_or_greeting_writes() {
        let harness = Harness::new(8, ENABLED);
        let (stream, mut peer) = duplex(4096);
        let task = harness.start(stream, EntryPoint::TcpServer);
        for chunk in request(b"ID:1")[..249].chunks(83) {
            peer.write_all(chunk).await.expect("partial");
            tokio::task::yield_now().await;
            advance(Duration::from_secs(4)).await;
        }
        assert_eq!(
            task.await.expect("task").unwrap_err(),
            RelayError::Aborted(AbortReason::HandshakeTimeout)
        );
        assert_abort(&harness, AbortReason::HandshakeTimeout);

        let harness = Harness::new(8, ENABLED);
        let (stream, mut peer) = duplex(1);
        let task = harness.start(stream, EntryPoint::TcpViewer);
        for _ in 0..3 {
            peer.read_exact(&mut [0; 1]).await.expect("greeting byte");
            advance(Duration::from_secs(4)).await;
        }
        assert_eq!(
            task.await.expect("task").unwrap_err(),
            RelayError::Aborted(AbortReason::HandshakeTimeout)
        );
        assert_abort(&harness, AbortReason::HandshakeTimeout);

        for elapsed in [
            H - Duration::from_millis(1),
            H,
            H + Duration::from_millis(1),
        ] {
            let harness = Harness::new(8, ENABLED);
            let (stream, mut peer) = duplex(4096);
            let task = harness.start(stream, EntryPoint::TcpServer);
            harness.clock.jump(elapsed);
            peer.write_all(&request(b"ID:1"))
                .await
                .expect("ready request");
            let result = task.await.expect("task");
            if elapsed < H {
                assert!(result.is_ok());
                assert!(harness.events.snapshot().is_empty());
            } else {
                assert_eq!(
                    result.unwrap_err(),
                    RelayError::Aborted(AbortReason::HandshakeTimeout)
                );
                assert_abort(&harness, AbortReason::HandshakeTimeout);
            }
        }
    }

    struct TrackedStream {
        stream: DuplexStream,
        dropped: Arc<AtomicBool>,
        budget: Arc<ConnectionBudget>,
        fail_read: bool,
        fail_write: bool,
    }

    impl Drop for TrackedStream {
        fn drop(&mut self) {
            assert!(self.budget.try_acquire().is_none(), "stream must precede L");
            self.dropped.store(true, Ordering::SeqCst);
        }
    }

    impl AsyncRead for TrackedStream {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buffer: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            if self.fail_read {
                Poll::Ready(Err(io::Error::from(io::ErrorKind::ConnectionReset)))
            } else {
                Pin::new(&mut self.stream).poll_read(cx, buffer)
            }
        }
    }

    impl AsyncWrite for TrackedStream {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            bytes: &[u8],
        ) -> Poll<io::Result<usize>> {
            if self.fail_write {
                Poll::Ready(Err(io::Error::from(io::ErrorKind::BrokenPipe)))
            } else {
                Pin::new(&mut self.stream).poll_write(cx, bytes)
            }
        }

        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Pin::new(&mut self.stream).poll_flush(cx)
        }

        fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Pin::new(&mut self.stream).poll_shutdown(cx)
        }
    }

    impl RelayStream for TrackedStream {
        fn transport(&self) -> ViewerTransport {
            ViewerTransport::Tcp
        }
        fn pending_write_len(&self) -> usize {
            0
        }
        fn poll_close(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            _: RelayCloseAction,
        ) -> Poll<io::Result<()>> {
            assert!(
                self.budget.try_acquire().is_none(),
                "close must precede L release"
            );
            Pin::new(&mut self.stream).poll_shutdown(cx)
        }
    }

    #[tokio::test(start_paused = true)]
    async fn releases_the_stream_and_permit_before_logging_an_abort() {
        for case in 0..5 {
            let harness = Harness::new(8, ENABLED);
            let dropped = Arc::new(AtomicBool::new(false));
            *harness.events.resources.lock().expect("resource lock") =
                Some((dropped.clone(), harness.budget.clone()));
            let (stream, peer) = duplex(4096);
            let stream = TrackedStream {
                stream,
                dropped,
                budget: harness.budget.clone(),
                fail_read: case == 2,
                fail_write: case == 3,
            };
            let entry_point = if case == 3 {
                EntryPoint::TcpViewer
            } else {
                EntryPoint::TcpServer
            };
            let task = harness.start(stream, entry_point);
            let mut peer = Some(peer);
            let reason = match case {
                0 => {
                    peer.take();
                    AbortReason::PeerEof
                }
                1 => {
                    advance(H).await;
                    AbortReason::HandshakeTimeout
                }
                2 | 3 => AbortReason::ConnectionError(ConnectionErrorKind::IoError),
                _ => {
                    harness.shutdown.send_replace(RelayLifecycle::Force);
                    AbortReason::ServerShutdown
                }
            };
            assert_eq!(
                task.await.expect("task").unwrap_err(),
                RelayError::Aborted(reason)
            );
            assert_abort(&harness, reason);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn drops_the_stream_before_l_when_preparation_is_cancelled() {
        for poll_once in [false, true] {
            let harness = Harness::new(8, ENABLED);
            let dropped = Arc::new(AtomicBool::new(false));
            let (stream, _peer) = duplex(4096);
            let stream = TrackedStream {
                stream,
                dropped: dropped.clone(),
                budget: harness.budget.clone(),
                fail_read: false,
                fail_write: false,
            };
            let context = ConnectionContext {
                conn_id: ConnId::next(),
                entry_point: EntryPoint::TcpServer,
                peer_ip: PeerIp::new(IpAddr::V4(Ipv4Addr::LOCALHOST)),
                peer_addr: std::net::SocketAddr::from((Ipv4Addr::LOCALHOST, 12345)),
                accepted_wall: SystemTime::UNIX_EPOCH,
                accepted_at: Duration::ZERO,
                exchange_at: Duration::ZERO,
            };
            let mut future = Box::pin(harness.service.handle_connection(
                stream,
                context,
                harness.budget.try_acquire().expect("test L slot"),
                harness.shutdown.subscribe(),
            ));
            if poll_once {
                poll_fn(|cx| {
                    assert!(future.as_mut().poll(cx).is_pending());
                    Poll::Ready(())
                })
                .await;
            }
            drop(future);
            assert!(dropped.load(Ordering::SeqCst));
            assert!(harness.budget.try_acquire().is_some());
        }
    }

    /// spec: relay.md:RLY-004 partial
    /// spec: relay.md:RLY-005 partial
    /// spec: operations.md:OPS-006 partial
    #[tokio::test(start_paused = true)]
    async fn rejection_and_h_timeout_close_before_release_but_never_hold_l_beyond_e() {
        for timeout in [false, true] {
            let harness = Harness::new(8, ENABLED);
            let (stream, mut peer) = duplex(4096);
            let invalid = request(b"invalid");
            let input: &[u8] = if timeout { b"ID:" } else { &invalid };
            peer.write_all(input).await.expect("input");
            let stream = TestStream {
                stream,
                actions: harness.close_actions.clone(),
                budget: harness.budget.clone(),
                block_close: true,
                failure: None,
            };
            let context = ConnectionContext {
                conn_id: ConnId::next(),
                entry_point: EntryPoint::TcpServer,
                peer_ip: PeerIp::new(IpAddr::V4(Ipv4Addr::LOCALHOST)),
                peer_addr: std::net::SocketAddr::from((Ipv4Addr::LOCALHOST, 12345)),
                accepted_wall: harness.clock.wall(),
                accepted_at: harness.clock.monotonic(),
                exchange_at: harness.clock.monotonic(),
            };
            let mut future = Box::pin(harness.service.handle_connection(
                stream,
                context,
                harness.budget.try_acquire().expect("L"),
                harness.shutdown.subscribe(),
            ));
            poll_fn(|cx| {
                assert!(future.as_mut().poll(cx).is_pending());
                Poll::Ready(())
            })
            .await;
            if timeout {
                advance(H).await;
                poll_fn(|cx| {
                    assert!(future.as_mut().poll(cx).is_pending());
                    Poll::Ready(())
                })
                .await;
            }
            let action = if timeout {
                RelayCloseAction::HandshakeTimeout
            } else {
                RelayCloseAction::HandshakeInvalid
            };
            assert!(
                harness
                    .close_actions
                    .lock()
                    .expect("actions")
                    .contains(&action)
            );
            assert!(harness.budget.try_acquire().is_none());
            assert!(harness.events.snapshot().is_empty());
            advance(Duration::from_secs(3)).await;
            assert!(future.await.is_err());
            assert!(harness.budget.try_acquire().is_some());
            assert_eq!(peer.read(&mut [0; 1]).await.expect("stream dropped"), 0);
            assert_eq!(harness.events.snapshot().len(), 1);
        }
    }

    /// spec: websocket.md:WS-019 partial
    /// spec: websocket.md:WS-008 partial
    /// spec: websocket.md:WS-010 partial
    #[tokio::test(start_paused = true)]
    async fn typed_transport_error_retains_kind_in_close_action_and_abort_event() {
        for kind in [
            ConnectionErrorKind::WsProtocolError,
            ConnectionErrorKind::WsTextMessage,
            ConnectionErrorKind::WsMessageTooLarge,
        ] {
            let harness = Harness::new(8, ENABLED);
            let (stream, _peer) = duplex(4096);
            let stream = TestStream {
                stream,
                actions: harness.close_actions.clone(),
                budget: harness.budget.clone(),
                block_close: false,
                failure: Some(kind),
            };
            let context = ConnectionContext {
                conn_id: ConnId::next(),
                entry_point: EntryPoint::TcpServer,
                peer_ip: PeerIp::new(IpAddr::V4(Ipv4Addr::LOCALHOST)),
                peer_addr: std::net::SocketAddr::from((Ipv4Addr::LOCALHOST, 12345)),
                accepted_wall: harness.clock.wall(),
                accepted_at: harness.clock.monotonic(),
                exchange_at: harness.clock.monotonic(),
            };
            let result = harness
                .service
                .handle_connection(
                    stream,
                    context,
                    harness.budget.try_acquire().expect("L"),
                    harness.shutdown.subscribe(),
                )
                .await;
            assert_eq!(
                result.unwrap_err(),
                RelayError::Aborted(AbortReason::ConnectionError(kind))
            );
            assert_eq!(
                *harness.close_actions.lock().expect("actions"),
                [RelayCloseAction::StreamError(kind)]
            );
            assert_abort(&harness, AbortReason::ConnectionError(kind));
        }
    }

    /// spec: operations.md:OPS-007 partial
    #[tokio::test(start_paused = true)]
    async fn force_polls_initial_shutdown_close_once_without_waiting_or_holding_l() {
        for block_close in [false, true] {
            let harness = Harness::new(8, ENABLED);
            let (stream, mut peer) = duplex(4096);
            let stream = TestStream {
                stream,
                actions: harness.close_actions.clone(),
                budget: harness.budget.clone(),
                block_close,
                failure: None,
            };
            let context = ConnectionContext {
                conn_id: ConnId::next(),
                entry_point: EntryPoint::TcpServer,
                peer_ip: PeerIp::new(IpAddr::V4(Ipv4Addr::LOCALHOST)),
                peer_addr: std::net::SocketAddr::from((Ipv4Addr::LOCALHOST, 12345)),
                accepted_wall: harness.clock.wall(),
                accepted_at: harness.clock.monotonic(),
                exchange_at: harness.clock.monotonic(),
            };
            harness.shutdown.send_replace(RelayLifecycle::Force);
            let result = harness
                .service
                .handle_connection(
                    stream,
                    context,
                    harness.budget.try_acquire().expect("L"),
                    harness.shutdown.subscribe(),
                )
                .await;
            assert_eq!(
                result.unwrap_err(),
                RelayError::Aborted(AbortReason::ServerShutdown)
            );
            assert_eq!(
                *harness.close_actions.lock().expect("actions"),
                [RelayCloseAction::ServerShutdown]
            );
            assert_eq!(harness.clock.monotonic(), Duration::ZERO);
            assert!(harness.budget.try_acquire().is_some());
            assert_eq!(peer.read(&mut [0; 1]).await.expect("stream dropped"), 0);
            assert_abort(&harness, AbortReason::ServerShutdown);
        }
    }

    /// spec: relay.md:RLY-004 partial
    /// spec: operations.md:OPS-006 partial
    /// spec: operations.md:OPS-007 partial
    #[tokio::test(start_paused = true)]
    async fn pending_initial_failure_close_stops_at_grace_deadline_or_second_shutdown() {
        for second_shutdown in [false, true] {
            let harness = Harness::new(8, ENABLED);
            let (stream, mut peer) = duplex(4096);
            peer.write_all(&request(b"invalid"))
                .await
                .expect("invalid request");
            let stream = TestStream {
                stream,
                actions: harness.close_actions.clone(),
                budget: harness.budget.clone(),
                block_close: true,
                failure: None,
            };
            let context = ConnectionContext {
                conn_id: ConnId::next(),
                entry_point: EntryPoint::TcpServer,
                peer_ip: PeerIp::new(IpAddr::V4(Ipv4Addr::LOCALHOST)),
                peer_addr: std::net::SocketAddr::from((Ipv4Addr::LOCALHOST, 12345)),
                accepted_wall: harness.clock.wall(),
                accepted_at: harness.clock.monotonic(),
                exchange_at: harness.clock.monotonic(),
            };
            let mut future = Box::pin(harness.service.handle_connection(
                stream,
                context,
                harness.budget.try_acquire().expect("L"),
                harness.shutdown.subscribe(),
            ));
            poll_fn(|cx| {
                assert!(future.as_mut().poll(cx).is_pending());
                Poll::Ready(())
            })
            .await;
            assert!(
                harness
                    .close_actions
                    .lock()
                    .expect("actions")
                    .contains(&RelayCloseAction::HandshakeInvalid)
            );
            assert!(harness.budget.try_acquire().is_none());

            // Enter Grace phase with G = 1s, which is well before E = 3s
            harness.shutdown.send_replace(RelayLifecycle::Grace {
                deadline: harness.clock.monotonic() + Duration::from_secs(1),
            });
            poll_fn(|cx| {
                assert!(future.as_mut().poll(cx).is_pending());
                Poll::Ready(())
            })
            .await;

            if second_shutdown {
                // Second shutdown arrives immediately (Force)
                harness.shutdown.send_replace(RelayLifecycle::Force);
                assert!(future.await.is_err());
            } else {
                // Advance to G deadline
                advance(Duration::from_secs(1)).await;
                assert!(future.await.is_err());
            }
            assert!(harness.budget.try_acquire().is_some());
            assert_eq!(peer.read(&mut [0; 1]).await.expect("stream dropped"), 0);
            assert_eq!(harness.events.snapshot().len(), 1);
        }
    }
}
