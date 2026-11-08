//! Two bounded relay directions driven by one owner, without socket tasks.
//! Streams and permits remain with the caller until final registry cleanup.

use std::future::{pending, poll_fn};
use std::pin::{Pin, pin};
use std::task::{Context, Poll};
use std::time::Duration;

use tokio::io::ReadBuf;
use tokio::sync::watch;
use vnc_relay_domain::handshake::Mode;
use vnc_relay_domain::session::{CloseCause, Direction, Session, SessionError, Side};

use crate::events::ConnectionErrorKind;
use crate::ports::{Clock, RelayCloseAction, RelayLifecycle, RelayStream, RelayStreamError};

/// Runs until transport cleanup completes or its absolute bound expires.
/// `PeerEof` stays provisional: only the caller can drop streams/permits and
/// call `Session::finish`. Close actions preserve TCP half-close and the
/// asymmetric WS Close handshake without exposing framing to the engine.
/// The returned kind belongs only to a selected connection-error outcome;
/// protocol-specific failures retain their kind without changing domain state.
#[expect(
    clippy::too_many_arguments,
    reason = "the session owner supplies borrowed resources and immutable relay limits"
)]
pub(super) async fn run_session_controlled(
    viewer: &mut dyn RelayStream,
    other: &mut dyn RelayStream,
    viewer_trailing: Vec<u8>,
    other_trailing: Vec<u8>,
    session: &mut Session,
    clock: &dyn Clock,
    mut lifecycle: watch::Receiver<RelayLifecycle>,
    buffer_bytes: [usize; 2],
    settings: &dyn crate::reload::ports::RuntimeSettingsSource,
    mut admin_close: watch::Receiver<Option<(Duration, Duration)>>,
    on_update: impl Fn(&Session, Option<ConnectionErrorKind>),
) -> Result<Option<ConnectionErrorKind>, SessionError> {
    let mut directions = [
        BufferedDirection::new(viewer_trailing, buffer_bytes[0]),
        BufferedDirection::new(other_trailing, buffer_bytes[1]),
    ];
    let other_side = match session.mode() {
        Mode::Mode1 => Side::Target,
        Mode::Mode2 => Side::Server,
    };
    let is_ws = [
        viewer.transport().is_websocket(),
        other.transport().is_websocket(),
    ];
    let mut ws_close_sent = [false; 2];
    let mut terminal = [false; 2];
    let mut terminal_deadline = None;
    let mut watch_closed = false;
    let mut command_closed = false;
    let mut timer_now = Duration::ZERO;
    let mut next = 0;
    let mut operations = 0;
    let mut error_kind = None;
    // Read E only for the first close transition, never at session creation.
    let mut close_timeout = None;

    loop {
        let state = *lifecycle.borrow_and_update();
        let now = clock.monotonic().max(timer_now);
        let admin_started = *admin_close.borrow_and_update();
        if let Some((accepted_at, timeout)) = admin_started
            && session.begin_admin_close(accepted_at)?
        {
            close_timeout.get_or_insert(timeout);
            on_update(session, error_kind);
        }
        let grace_deadline = match state {
            RelayLifecycle::Grace { deadline } => Some(deadline),
            RelayLifecycle::Running | RelayLifecycle::Force => None,
        };
        if close_timeout.is_none()
            && session.state() != vnc_relay_domain::session::SessionState::Active
        {
            settings.with_current(&mut |settings| close_timeout = Some(settings.timeouts.close));
        }
        let timeout = close_timeout.unwrap_or_default();
        let eof_deadline = session
            .first_eof()
            .map(|(_, at)| at.saturating_add(timeout));
        let admin_deadline = admin_started.map(|(at, _)| at.saturating_add(timeout));
        let cleanup_deadline = eof_deadline.or(admin_deadline);
        if cleanup_deadline.is_some_and(|end| now >= end) && session.expire_close(now, timeout)? {
            on_update(session, error_kind);
        }

        if state == RelayLifecycle::Force || grace_deadline.is_some_and(|end| now >= end) {
            if session.force_server_shutdown()? {
                on_update(session, error_kind);
            }
            // Force never waits for a close handshake or a blocked shutdown.
            close_once(
                viewer,
                other,
                close_action(session, error_kind, is_ws),
                &mut ws_close_sent,
            )
            .await;
            return Ok(
                error_kind.filter(|_| session.close_cause() == Some(CloseCause::ConnectionError))
            );
        }
        let closing = session.close_cause().is_some()
            || directions.iter().all(|direction| direction.complete);
        if closing && terminal_deadline.is_none() {
            terminal_deadline =
                Some(cleanup_deadline.unwrap_or_else(|| now.saturating_add(timeout)));
        }
        let close_deadline = cleanup_deadline.or(terminal_deadline);
        let expired = close_deadline.is_some_and(|end| now >= end);
        if closing && (expired || terminal == [true; 2]) {
            if expired {
                // Still initiate each close action, but leave forced disposal to
                // the caller. Cleanup-only errors never replace the relay cause.
                close_once(
                    viewer,
                    other,
                    close_action(session, error_kind, is_ws),
                    &mut ws_close_sent,
                )
                .await;
                // A close implementation may synchronously advance the clock or
                // publish Force. Recheck before returning the unfinished model.
                let latest = *lifecycle.borrow();
                if latest == RelayLifecycle::Force
                    || matches!(latest, RelayLifecycle::Grace { deadline } if clock.monotonic().max(timer_now) >= deadline)
                {
                    if session.force_server_shutdown()? {
                        on_update(session, error_kind);
                    }
                    close_once(
                        viewer,
                        other,
                        close_action(session, error_kind, is_ws),
                        &mut ws_close_sent,
                    )
                    .await;
                }
            }
            return Ok(
                error_kind.filter(|_| session.close_cause() == Some(CloseCause::ConnectionError))
            );
        }

        let deadline = match (close_deadline, grace_deadline) {
            (Some(left), Some(right)) => Some(left.min(right)),
            (left, right) => left.or(right),
        };
        let event = {
            // Both futures survive every Pending poll. Dropping `changed` only
            // after an event is cancellation-safe and cannot lose a transition.
            let mut changed = pin!(lifecycle.changed());
            let mut command_changed = pin!(admin_close.changed());
            let mut timeout = pin!(async {
                match deadline {
                    Some(end) => tokio::time::sleep(end.saturating_sub(now)).await,
                    None => pending::<()>().await,
                }
            });
            poll_fn(|cx| {
                if !command_closed && command_changed.as_mut().poll(cx).is_ready() {
                    return Poll::Ready(Wake::AdminCommand);
                }
                if !watch_closed && changed.as_mut().poll(cx).is_ready() {
                    return Poll::Ready(Wake::Lifecycle);
                }
                if timeout.as_mut().poll(cx).is_ready() {
                    return Poll::Ready(Wake::Deadline);
                }
                // Keep control output and liveness moving independently on
                // both endpoints, even when payload buffers are full.
                for index in [next, 1 - next] {
                    if is_ws[index] && !closing {
                        let stream: &mut dyn RelayStream = if index == 0 {
                            &mut *viewer
                        } else {
                            &mut *other
                        };
                        if let Poll::Ready(Err(error)) = Pin::new(&mut *stream).poll_maintenance(cx)
                        {
                            return Poll::Ready(Wake::Direction(index, Step::input_error(&error)));
                        }
                        if !terminal[index]
                            && let Poll::Ready(Err(error)) = Pin::new(stream).poll_flush(cx)
                        {
                            return Poll::Ready(Wake::Direction(
                                1 - index,
                                Step::output_error(&error),
                            ));
                        }
                    }
                }
                for index in [next, 1 - next] {
                    if closing {
                        if terminal[index] {
                            continue;
                        }
                        let stream: &mut dyn RelayStream = if index == 0 {
                            &mut *viewer
                        } else {
                            &mut *other
                        };
                        let incoming = &mut directions[1 - index];
                        // Stop new messages on both endpoints, but finish any
                        // frame header that already reached a WebSocket peer.
                        if is_ws[index]
                            && session.close_cause() == Some(CloseCause::AdminTerminated)
                            && stream.pending_write_len() > 0
                            && incoming.start < incoming.end
                        {
                            if let Poll::Ready(step) = incoming.poll_started_frame(cx, stream) {
                                return Poll::Ready(Wake::Direction(1 - index, step));
                            }
                            continue;
                        }
                        let Some(action) = close_action(session, error_kind, is_ws) else {
                            return Poll::Ready(Wake::Terminal(index));
                        };
                        if Pin::new(stream).poll_close(cx, action).is_ready() {
                            return Poll::Ready(Wake::Terminal(index));
                        }
                    } else {
                        let output_index = 1 - index;
                        let output_side = if output_index == 0 {
                            Side::Viewer
                        } else {
                            other_side
                        };
                        let output_closed_first = is_ws[output_index]
                            && directions[output_index].eof
                            && session
                                .first_eof()
                                .is_some_and(|(side, _)| side == output_side);
                        let peer_write_closed = directions[output_index].complete;
                        let (input, output): (&mut dyn RelayStream, &mut dyn RelayStream) =
                            if index == 0 {
                                (&mut *viewer, &mut *other)
                            } else {
                                (&mut *other, &mut *viewer)
                            };
                        let step = if output_closed_first {
                            directions[index].poll_step_ws_closing(
                                cx,
                                input,
                                output,
                                ws_close_sent[output_index],
                                peer_write_closed,
                            )
                        } else {
                            directions[index].poll_step(cx, input, output)
                        };
                        if let Poll::Ready(step) = step {
                            return Poll::Ready(Wake::Direction(index, step));
                        }
                    }
                }
                Poll::Pending
            })
            .await
        };
        match event {
            Wake::AdminCommand => {
                command_closed = admin_close.has_changed().is_err();
            }
            Wake::Lifecycle => {
                watch_closed = lifecycle.has_changed().is_err();
            }
            Wake::Deadline => {
                if let Some(end) = deadline {
                    timer_now = timer_now.max(end);
                }
            }
            Wake::Terminal(index) => {
                terminal[index] = true;
                if is_ws[index] {
                    ws_close_sent[index] = true;
                }
            }
            Wake::Direction(index, step) => {
                next = 1 - index;
                let direction = if index == 0 {
                    Direction::ViewerToTarget
                } else {
                    Direction::TargetToViewer
                };
                let input = if index == 0 { Side::Viewer } else { other_side };
                let output = if index == 0 { other_side } else { Side::Viewer };
                if close_timeout.is_none()
                    && matches!(
                        step,
                        Step::Eof
                            | Step::InputDead
                            | Step::OutputDead
                            | Step::InputError(_)
                            | Step::OutputError(_)
                    )
                {
                    settings.with_current(&mut |settings| {
                        close_timeout = Some(settings.timeouts.close)
                    });
                }
                match step {
                    Step::Read => {}
                    Step::Written(count) => {
                        session.record_write(direction, count as u64)?;
                        on_update(session, error_kind);
                    }
                    Step::Eof => {
                        session.observe_eof(input, clock.monotonic().max(timer_now))?;
                        on_update(session, error_kind);
                    }
                    Step::WriteClosed => {
                        if is_ws[1 - index] {
                            ws_close_sent[1 - index] = true;
                            terminal[1 - index] = true;
                        }
                        session.complete_direction(direction)?;
                        on_update(session, error_kind);
                    }
                    Step::CloseSent => {
                        ws_close_sent[1 - index] = true;
                        terminal[1 - index] = true;
                    }
                    Step::InputDead | Step::OutputDead => {
                        let side = if matches!(step, Step::InputDead) {
                            input
                        } else {
                            output
                        };
                        if session.dead_connection(side)? {
                            on_update(session, error_kind);
                        }
                    }
                    Step::InputError(kind) | Step::OutputError(kind) => {
                        let side = if matches!(step, Step::InputError(_)) {
                            input
                        } else {
                            output
                        };
                        if session.io_error(direction, side)? {
                            error_kind = Some(kind);
                            on_update(session, error_kind);
                        }
                    }
                }
            }
        }
        operations += 1;
        if operations == 32 {
            operations = 0;
            // Always-ready in-memory or buffered transports must not monopolize
            // the executor; fairness between directions alone is insufficient.
            let mut yielded = false;
            poll_fn(|cx| {
                if yielded {
                    Poll::Ready(())
                } else {
                    yielded = true;
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            })
            .await;
        }
    }
}

struct BufferedDirection {
    storage: Vec<u8>,
    start: usize,
    end: usize,
    eof: bool,
    complete: bool,
}

impl BufferedDirection {
    fn new(mut trailing: Vec<u8>, buffer_bytes: usize) -> Self {
        debug_assert!(buffer_bytes > 0 && trailing.len() <= buffer_bytes);
        let end = trailing.len();
        // Reserve exactly once in the transferred allocation, never alongside a
        // second B-sized buffer. Avoid Vec's geometric growth beyond B.
        if trailing.capacity() > buffer_bytes {
            trailing.shrink_to(buffer_bytes);
        }
        trailing.reserve_exact(buffer_bytes - end);
        trailing.resize(buffer_bytes, 0);
        Self {
            storage: trailing,
            start: 0,
            end,
            eof: false,
            complete: false,
        }
    }

    fn poll_step(
        &mut self,
        cx: &mut Context<'_>,
        input: &mut dyn RelayStream,
        output: &mut dyn RelayStream,
    ) -> Poll<Step> {
        if self.complete {
            return Poll::Pending;
        }
        if self.start < self.end {
            return match Pin::new(output).poll_write(cx, &self.storage[self.start..self.end]) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(Ok(0)) => Poll::Ready(Step::OutputError(ConnectionErrorKind::IoError)),
                Poll::Ready(Err(error)) => Poll::Ready(Step::output_error(&error)),
                Poll::Ready(Ok(count)) => {
                    self.start += count;
                    Poll::Ready(Step::Written(count))
                }
            };
        }
        if self.eof {
            return match Pin::new(output).poll_close(cx, RelayCloseAction::WriteHalf) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(Err(error)) => Poll::Ready(Step::output_error(&error)),
                Poll::Ready(Ok(())) => {
                    self.complete = true;
                    Poll::Ready(Step::WriteClosed)
                }
            };
        }
        self.start = 0;
        self.end = 0;
        let mut buffer = ReadBuf::new(&mut self.storage);
        match Pin::new(input).poll_read(cx, &mut buffer) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(error)) => Poll::Ready(Step::input_error(&error)),
            Poll::Ready(Ok(())) => {
                self.end = buffer.filled().len();
                if self.end == 0 {
                    self.eof = true;
                    Poll::Ready(Step::Eof)
                } else {
                    Poll::Ready(Step::Read)
                }
            }
        }
    }

    /// Writes only the remaining payload of an already-started WS frame.
    /// The caller proves there is a frame and buffered data; no input is read.
    fn poll_started_frame(&mut self, cx: &mut Context<'_>, ws: &mut dyn RelayStream) -> Poll<Step> {
        let count = (self.end - self.start).min(ws.pending_write_len());
        match Pin::new(ws).poll_write(cx, &self.storage[self.start..self.start + count]) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(0)) => {
                self.start = self.end;
                Poll::Ready(Step::OutputError(ConnectionErrorKind::IoError))
            }
            Poll::Ready(Err(error)) => {
                self.start = self.end;
                Poll::Ready(Step::output_error(&error))
            }
            Poll::Ready(Ok(count)) => {
                self.start += count;
                Poll::Ready(Step::Written(count))
            }
        }
    }

    fn poll_step_ws_closing(
        &mut self,
        cx: &mut Context<'_>,
        peer: &mut dyn RelayStream,
        ws: &mut dyn RelayStream,
        ws_close_sent: bool,
        peer_write_closed: bool,
    ) -> Poll<Step> {
        if self.complete {
            return Poll::Pending;
        }
        // 1. Finish in-flight frame if any
        let pending = ws.pending_write_len();
        if pending > 0 && self.start < self.end {
            return self.poll_started_frame(cx, ws);
        }
        // Discard any remaining buffered bytes, uncounted, and start no new message
        self.start = self.end;

        // Echo only after preceding WS input has reached the peer and its
        // write half has closed. An already-sent Close remains final.
        if !ws_close_sent {
            if !peer_write_closed {
                return Poll::Pending;
            }
            return match Pin::new(ws).poll_close(cx, RelayCloseAction::PeerEof) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(Err(error)) => Poll::Ready(Step::output_error(&error)),
                Poll::Ready(Ok(())) => Poll::Ready(Step::CloseSent),
            };
        }

        // Keep reading the peer until EOF or E, discarding data uncounted.
        if !self.eof {
            let mut read_buf = ReadBuf::new(&mut self.storage);
            return match Pin::new(peer).poll_read(cx, &mut read_buf) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(Err(error)) => Poll::Ready(Step::input_error(&error)),
                Poll::Ready(Ok(())) => {
                    if read_buf.filled().is_empty() {
                        self.eof = true;
                        Poll::Ready(Step::Eof)
                    } else {
                        Poll::Ready(Step::Read)
                    }
                }
            };
        }

        self.complete = true;
        Poll::Ready(Step::WriteClosed)
    }
}

#[derive(Clone, Copy)]
enum Step {
    Read,
    Written(usize),
    Eof,
    WriteClosed,
    CloseSent,
    InputError(ConnectionErrorKind),
    OutputError(ConnectionErrorKind),
    InputDead,
    OutputDead,
}

impl Step {
    fn input_error(error: &std::io::Error) -> Self {
        if RelayStreamError::is_dead_connection(error) {
            Self::InputDead
        } else {
            Self::InputError(RelayStreamError::classify(error))
        }
    }

    fn output_error(error: &std::io::Error) -> Self {
        if RelayStreamError::is_dead_connection(error) {
            Self::OutputDead
        } else {
            Self::OutputError(RelayStreamError::classify(error))
        }
    }
}

enum Wake {
    Lifecycle,
    AdminCommand,
    Deadline,
    Direction(usize, Step),
    Terminal(usize),
}

fn close_action(
    session: &Session,
    error_kind: Option<ConnectionErrorKind>,
    is_ws: [bool; 2],
) -> Option<RelayCloseAction> {
    Some(match session.close_cause() {
        Some(CloseCause::ConnectionError) => {
            RelayCloseAction::StreamError(error_kind.unwrap_or(ConnectionErrorKind::IoError))
        }
        Some(CloseCause::DeadConnection) => RelayCloseAction::DeadConnection,
        Some(CloseCause::AdminTerminated) => RelayCloseAction::AdminTerminated,
        Some(CloseCause::ServerShutdown) => RelayCloseAction::ServerShutdown,
        Some(CloseCause::InternalError) => return None,
        None | Some(CloseCause::PeerEof | CloseCause::CloseTimeout) => {
            if is_ws != [false; 2]
                && session.first_eof().is_some_and(|(side, _)| {
                    let index = usize::from(side != Side::Viewer);
                    !is_ws[index]
                })
            {
                RelayCloseAction::WriteHalf
            } else {
                RelayCloseAction::PeerEof
            }
        }
    })
}

async fn close_once<'a>(
    viewer: &'a mut dyn RelayStream,
    other: &'a mut dyn RelayStream,
    action: Option<RelayCloseAction>,
    ws_close_sent: &mut [bool; 2],
) {
    let Some(action) = action else { return };
    poll_fn(|cx| {
        for (index, stream) in [&mut *viewer, &mut *other].into_iter().enumerate() {
            if !ws_close_sent[index] {
                let is_ws = stream.transport().is_websocket();
                if let Poll::Ready(Ok(())) = Pin::new(stream).poll_close(cx, action) {
                    ws_close_sent[index] = is_ws;
                }
            }
        }
        Poll::Ready(())
    })
    .await;
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::io;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::task::Waker;
    use std::time::SystemTime;

    use tokio::io::{AsyncRead, AsyncWrite};
    use tokio::time::{Instant, advance};
    use vnc_relay_domain::session::{SessionId, SessionState, Transport};

    use super::*;
    use crate::relay::test_support::TestSettings;

    #[expect(
        clippy::too_many_arguments,
        reason = "test helper preserves the uncommanded engine fixture"
    )]
    async fn run_session(
        viewer: &mut dyn RelayStream,
        other: &mut dyn RelayStream,
        viewer_trailing: Vec<u8>,
        other_trailing: Vec<u8>,
        session: &mut Session,
        clock: &dyn Clock,
        lifecycle: watch::Receiver<RelayLifecycle>,
        buffer_bytes: usize,
        close_timeout: Duration,
        on_update: impl Fn(&Session),
    ) -> Result<Option<ConnectionErrorKind>, SessionError> {
        let (_command, receiver) = watch::channel(None);
        let settings = TestSettings::new();
        settings.change(|settings| settings.timeouts.close = close_timeout);
        run_session_controlled(
            viewer,
            other,
            viewer_trailing,
            other_trailing,
            session,
            clock,
            lifecycle,
            [buffer_bytes; 2],
            settings.as_ref(),
            receiver,
            |session, _| on_update(session),
        )
        .await
    }

    /// spec: operations.md:OPS-050 partial
    /// spec: operations.md:OPS-057 partial
    /// spec: management.md:MGT-041 partial
    #[tokio::test(start_paused = true)]
    async fn reload_reads_e_at_first_close_and_keeps_started_close_fixed() {
        let settings = TestSettings::new();
        let clock = TestClock::new();
        let mut session = model(Mode::Mode2);
        let (_lifecycle, lifecycle) = watch::channel(RelayLifecycle::Running);
        let (_command, command) = watch::channel(None);
        let (mut viewer, viewer_control) = stream(&[], false);
        let (mut other, other_control) = stream(&[], false);
        other_control.change(|state| state.half = CloseBehavior::Pending);
        let mut relay = pin!(run_session_controlled(
            &mut viewer,
            &mut other,
            Vec::new(),
            Vec::new(),
            &mut session,
            clock.as_ref(),
            lifecycle,
            [4, 8],
            settings.as_ref(),
            command,
            |_, _| {},
        ));
        pump(&mut relay).await;
        advance(Duration::from_secs(3)).await;
        settings.change(|settings| settings.timeouts.close = Duration::from_secs(5));
        viewer_control.feed(&[], true);
        pump(&mut relay).await;
        settings.change(|settings| settings.timeouts.close = Duration::from_secs(1));
        advance(Duration::from_secs(4)).await;
        poll_fn(|cx| {
            assert!(
                relay.as_mut().poll(cx).is_pending(),
                "started E must remain five seconds"
            );
            Poll::Ready(())
        })
        .await;
        advance(Duration::from_secs(1)).await;
        relay.await.expect("old E expires");
    }

    /// spec: operations.md:OPS-050 partial
    #[tokio::test(start_paused = true)]
    async fn mode_two_directions_keep_distinct_accepted_buffer_limits() {
        let settings = TestSettings::new();
        let clock = TestClock::new();
        let mut session = model(Mode::Mode2);
        let (_lifecycle, lifecycle) = watch::channel(RelayLifecycle::Running);
        let (_command, command) = watch::channel(None);
        let (mut viewer, viewer_control) = stream(b"viewer payload", true);
        let (mut other, other_control) = stream(b"server payload", true);
        run_session_controlled(
            &mut viewer,
            &mut other,
            Vec::new(),
            Vec::new(),
            &mut session,
            clock.as_ref(),
            lifecycle,
            [3, 7],
            settings.as_ref(),
            command,
            |_, _| {},
        )
        .await
        .expect("relay");
        assert_eq!(viewer_control.0.lock().expect("viewer").max_read, 3);
        assert_eq!(other_control.0.lock().expect("server").max_read, 7);
        assert_eq!(viewer_control.output(), b"server payload");
        assert_eq!(other_control.output(), b"viewer payload");
    }

    const E: Duration = Duration::from_secs(10);

    struct TestClock {
        epoch: Instant,
        offset_ms: AtomicU64,
    }

    impl TestClock {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                epoch: Instant::now(),
                offset_ms: AtomicU64::new(0),
            })
        }

        fn jump(&self, by: Duration) {
            self.offset_ms.fetch_add(
                by.as_millis().try_into().expect("test duration"),
                Ordering::Relaxed,
            );
        }
    }

    impl Clock for TestClock {
        fn monotonic(&self) -> Duration {
            self.epoch.elapsed() + Duration::from_millis(self.offset_ms.load(Ordering::Relaxed))
        }

        fn wall(&self) -> SystemTime {
            SystemTime::UNIX_EPOCH + self.monotonic()
        }
    }

    #[derive(Clone, Copy, Default)]
    enum CloseBehavior {
        #[default]
        Ready,
        Pending,
        Error,
    }

    struct StreamState {
        transport: Transport,
        pending_write: usize,
        input: VecDeque<u8>,
        output: Vec<u8>,
        eof: bool,
        read_error: bool,
        typed_error: Option<ConnectionErrorKind>,
        read_chunk: usize,
        read_total: usize,
        max_read: usize,
        read_calls: usize,
        write_chunk: usize,
        write_blocked: bool,
        write_zero: bool,
        error_after: Option<usize>,
        pending_pong: VecDeque<u8>,
        control_output: Vec<u8>,
        control_allowance: usize,
        flush_error: Option<ConnectionErrorKind>,
        maintenance_error: Option<RelayStreamError>,
        half: CloseBehavior,
        terminal: CloseBehavior,
        require_frame_finished: bool,
        actions: Vec<RelayCloseAction>,
        half_closed: bool,
        waker: Option<Waker>,
        jump_on_read: Option<(Arc<TestClock>, Duration)>,
        jump_on_write: Option<(Arc<TestClock>, Duration)>,
        jump_on_half: Option<(Arc<TestClock>, Duration)>,
        force_on_terminal: Option<watch::Sender<RelayLifecycle>>,
    }

    impl Default for StreamState {
        fn default() -> Self {
            Self {
                transport: Transport::Tcp,
                pending_write: 0,
                input: VecDeque::new(),
                output: Vec::new(),
                eof: false,
                read_error: false,
                typed_error: None,
                read_chunk: usize::MAX,
                read_total: 0,
                max_read: 0,
                read_calls: 0,
                write_chunk: usize::MAX,
                write_blocked: false,
                write_zero: false,
                error_after: None,
                pending_pong: VecDeque::new(),
                control_output: Vec::new(),
                control_allowance: usize::MAX,
                flush_error: None,
                maintenance_error: None,
                half: CloseBehavior::Ready,
                terminal: CloseBehavior::Ready,
                require_frame_finished: false,
                actions: Vec::new(),
                half_closed: false,
                waker: None,
                jump_on_read: None,
                jump_on_write: None,
                jump_on_half: None,
                force_on_terminal: None,
            }
        }
    }

    struct TestStream(Arc<Mutex<StreamState>>);

    #[derive(Clone)]
    struct Control(Arc<Mutex<StreamState>>);

    impl Control {
        fn change(&self, change: impl FnOnce(&mut StreamState)) {
            let wake = {
                let mut state = self.0.lock().expect("stream state");
                change(&mut state);
                state.waker.take()
            };
            if let Some(waker) = wake {
                waker.wake();
            }
        }

        fn output(&self) -> Vec<u8> {
            self.0.lock().expect("stream state").output.clone()
        }

        fn actions(&self) -> Vec<RelayCloseAction> {
            self.0.lock().expect("stream state").actions.clone()
        }

        fn feed(&self, bytes: &[u8], eof: bool) {
            self.change(|state| {
                state.input.extend(bytes);
                state.eof = eof;
            });
        }
    }

    fn stream(input: &[u8], eof: bool) -> (TestStream, Control) {
        let state = Arc::new(Mutex::new(StreamState::default()));
        let control = Control(state.clone());
        control.feed(input, eof);
        (TestStream(state), control)
    }

    impl AsyncRead for TestStream {
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buffer: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            let mut state = self.0.lock().expect("stream state");
            state.waker = Some(cx.waker().clone());
            state.read_calls += 1;
            state.max_read = state.max_read.max(buffer.remaining());
            if state.read_error {
                return Poll::Ready(Err(stream_failure(state.typed_error)));
            }
            let count = buffer
                .remaining()
                .min(state.read_chunk)
                .min(state.input.len());
            if count > 0 {
                for _ in 0..count {
                    buffer.put_slice(&[state.input.pop_front().expect("counted input")]);
                }
                state.read_total += count;
                if let Some((clock, by)) = state.jump_on_read.take() {
                    clock.jump(by);
                }
                return Poll::Ready(Ok(()));
            }
            if state.eof {
                Poll::Ready(Ok(()))
            } else {
                Poll::Pending
            }
        }
    }

    impl AsyncWrite for TestStream {
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            bytes: &[u8],
        ) -> Poll<io::Result<usize>> {
            let mut state = self.0.lock().expect("stream state");
            state.waker = Some(cx.waker().clone());
            if state.write_blocked {
                return Poll::Pending;
            }
            if state
                .error_after
                .is_some_and(|limit| state.output.len() >= limit)
            {
                return Poll::Ready(Err(stream_failure(state.typed_error)));
            }
            if state.write_zero {
                return Poll::Ready(Ok(0));
            }
            let remaining = state
                .error_after
                .map_or(usize::MAX, |limit| limit - state.output.len());
            let count = bytes.len().min(state.write_chunk).min(remaining);
            state.output.extend_from_slice(&bytes[..count]);
            state.pending_write = state.pending_write.saturating_sub(count);
            if let Some((clock, by)) = state.jump_on_write.take() {
                clock.jump(by);
            }
            Poll::Ready(Ok(count))
        }

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            let mut state = self.0.lock().expect("stream state");
            if let Some(kind) = state.flush_error {
                return Poll::Ready(Err(stream_failure(Some(kind))));
            }
            // Match the WS adapter: controls never interrupt a data frame.
            if state.pending_write != 0 {
                return Poll::Ready(Ok(()));
            }
            while state.control_allowance > 0 {
                let Some(byte) = state.pending_pong.pop_front() else {
                    break;
                };
                state.control_output.push(byte);
                state.control_allowance -= 1;
            }
            if state.pending_pong.is_empty() {
                Poll::Ready(Ok(()))
            } else {
                state.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }

        fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
            panic!("engine must use transport close actions")
        }
    }

    impl RelayStream for TestStream {
        fn transport(&self) -> Transport {
            self.0.lock().expect("stream state").transport
        }

        fn pending_write_len(&self) -> usize {
            self.0.lock().expect("stream state").pending_write
        }

        fn poll_maintenance(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            let mut state = self.0.lock().expect("stream state");
            state.waker = Some(cx.waker().clone());
            match state.maintenance_error {
                Some(error) => Poll::Ready(Err(error.into_io_error())),
                None => Poll::Pending,
            }
        }

        fn poll_close(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            action: RelayCloseAction,
        ) -> Poll<io::Result<()>> {
            let mut state = self.0.lock().expect("stream state");
            state.waker = Some(cx.waker().clone());
            state.actions.push(action);
            if state.require_frame_finished && state.pending_write > 0 {
                return Poll::Pending;
            }
            let behavior = if action == RelayCloseAction::WriteHalf {
                if let Some((clock, by)) = state.jump_on_half.take() {
                    clock.jump(by);
                }
                if matches!(state.half, CloseBehavior::Ready) {
                    state.half_closed = true;
                }
                state.half
            } else {
                if let Some(sender) = state.force_on_terminal.take() {
                    sender.send_replace(RelayLifecycle::Force);
                }
                state.terminal
            };
            match behavior {
                CloseBehavior::Ready => Poll::Ready(Ok(())),
                CloseBehavior::Pending => Poll::Pending,
                CloseBehavior::Error => Poll::Ready(Err(io::Error::other("close failure"))),
            }
        }
    }

    fn model(mode: Mode) -> Session {
        Session::new(
            SessionId::from_block([0; 16]),
            mode,
            Transport::Tcp,
            Transport::Tcp,
            SystemTime::UNIX_EPOCH,
            Duration::ZERO,
        )
    }

    async fn pump(
        future: &mut Pin<
            &mut impl Future<Output = Result<Option<ConnectionErrorKind>, SessionError>>,
        >,
    ) {
        for _ in 0..16 {
            poll_fn(|cx| {
                assert!(
                    future.as_mut().poll(cx).is_pending(),
                    "session must remain open"
                );
                Poll::Ready(())
            })
            .await;
        }
    }

    fn finish(session: &mut Session, clock: &TestClock) -> CloseCause {
        session
            .finish(clock.wall(), clock.monotonic())
            .expect("caller finishes after cleanup")
            .close_cause()
    }

    #[tokio::test(start_paused = true)]
    async fn drives_queued_and_partial_pongs_while_payload_writes_are_backpressured() {
        for frame_in_flight in [false, true] {
            let clock = TestClock::new();
            let mut session = model(Mode::Mode1);
            let (_sender, receiver) = watch::channel(RelayLifecycle::Running);
            let (mut viewer, viewer_control) = stream(b"full", false);
            let (mut other, other_control) = stream(&[], false);
            viewer_control.change(|state| {
                state.transport = Transport::Ws;
                state.pending_pong.extend(b"\x8a\x01p");
                state.pending_write = if frame_in_flight { 4 } else { 0 };
                state.control_allowance = if frame_in_flight { 3 } else { 1 };
            });
            other_control.change(|state| state.write_blocked = true);
            {
                let mut relay = pin!(run_session(
                    &mut viewer,
                    &mut other,
                    Vec::new(),
                    if frame_in_flight {
                        b"data".to_vec()
                    } else {
                        Vec::new()
                    },
                    &mut session,
                    clock.as_ref(),
                    receiver,
                    4,
                    E,
                    |_| {}
                ));
                pump(&mut relay).await;
                {
                    let state = viewer_control.0.lock().expect("viewer");
                    assert_eq!(state.read_total, 4, "one full B is held for TCP");
                    assert_eq!(state.read_calls, 1, "control output never reads past B");
                    assert_eq!(state.pending_write, 0, "reverse frame is finished");
                    assert_eq!(
                        state.control_output,
                        if frame_in_flight {
                            b"\x8a\x01p".as_slice()
                        } else {
                            b"\x8a".as_slice()
                        }
                    );
                }
                // Only control-write readiness changes; no more TCP payload
                // arrives and the TCP payload write remains blocked.
                viewer_control.change(|state| state.control_allowance = 3);
                pump(&mut relay).await;
                let state = viewer_control.0.lock().expect("viewer");
                assert_eq!(state.control_output, b"\x8a\x01p");
                assert!(state.pending_pong.is_empty());
                assert_eq!(state.read_calls, 1);
                assert!(other_control.output().is_empty());
            }
            assert_eq!(session.close_cause(), None);
            assert_eq!(session.first_eof(), None);
            assert_eq!(session.bytes(Direction::ViewerToTarget), 0);
            assert_eq!(
                session.bytes(Direction::TargetToViewer),
                if frame_in_flight { 4 } else { 0 }
            );
            assert_eq!(clock.monotonic(), Duration::ZERO);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn classifies_control_flush_errors_as_viewer_output_errors() {
        for kind in [
            ConnectionErrorKind::IoError,
            ConnectionErrorKind::WsProtocolError,
        ] {
            let clock = TestClock::new();
            let mut session = model(Mode::Mode1);
            let (_sender, receiver) = watch::channel(RelayLifecycle::Running);
            let (mut viewer, viewer_control) = stream(&[], false);
            let (mut other, _) = stream(&[], false);
            viewer_control.change(|state| {
                state.transport = Transport::Ws;
                state.flush_error = Some(kind);
            });
            let result = run_session(
                &mut viewer,
                &mut other,
                Vec::new(),
                Vec::new(),
                &mut session,
                clock.as_ref(),
                receiver,
                4,
                E,
                |_| {},
            )
            .await
            .expect("session cleanup");
            assert_eq!(result, Some(kind));
            assert_eq!(session.close_cause(), Some(CloseCause::ConnectionError));
            assert_eq!(session.close_side(), Some(Side::Viewer));
            assert_eq!(session.bytes(Direction::TargetToViewer), 0);
        }
    }

    /// spec: relay.md:RLY-003 partial
    /// spec: relay.md:RLY-017 partial
    #[tokio::test(start_paused = true)]
    async fn delivers_both_trailing_buffers_once_before_stream_bytes() {
        for mode in [Mode::Mode1, Mode::Mode2] {
            let clock = TestClock::new();
            let mut session = model(mode);
            let (_sender, receiver) = watch::channel(RelayLifecycle::Running);
            let (mut viewer, viewer_control) = stream(b"viewer-stream", true);
            let (mut other, other_control) = stream(b"other-stream", true);
            viewer_control.change(|state| {
                state.write_chunk = 2;
                state.read_chunk = 3;
            });
            other_control.change(|state| {
                state.write_chunk = 3;
                state.read_chunk = 2;
            });
            run_session(
                &mut viewer,
                &mut other,
                b"viewer-first".to_vec(),
                b"other-first".to_vec(),
                &mut session,
                clock.as_ref(),
                receiver,
                16,
                E,
                |_| {},
            )
            .await
            .expect("relay");
            assert_eq!(other_control.output(), b"viewer-firstviewer-stream");
            assert_eq!(viewer_control.output(), b"other-firstother-stream");
            assert_eq!(session.bytes(Direction::ViewerToTarget), 25);
            assert_eq!(session.bytes(Direction::TargetToViewer), 23);
            assert_eq!(session.state(), SessionState::Closing);
            assert_eq!(
                session.close_cause(),
                None,
                "EOF is provisional until ownership cleanup"
            );
            drop((viewer, other));
            assert_eq!(finish(&mut session, &clock), CloseCause::PeerEof);
        }
    }

    /// spec: relay.md:RLY-018 partial
    /// spec: relay.md:RLY-017 partial
    #[tokio::test(start_paused = true)]
    async fn bounds_reads_and_preserves_reverse_progress_under_backpressure() {
        let clock = TestClock::new();
        let mut session = model(Mode::Mode2);
        let (_sender, receiver) = watch::channel(RelayLifecycle::Running);
        let payload: Vec<_> = (0..80).collect();
        let (mut viewer, viewer_control) = stream(&payload, true);
        let (mut other, other_control) = stream(b"reverse", false);
        other_control.change(|state| state.write_blocked = true);
        {
            let mut relay = pin!(run_session(
                &mut viewer,
                &mut other,
                Vec::new(),
                Vec::new(),
                &mut session,
                clock.as_ref(),
                receiver,
                8,
                E,
                |_| {}
            ));
            pump(&mut relay).await;
            assert_eq!(
                viewer_control.output(),
                b"reverse",
                "blocked forward writes cannot block reverse"
            );
            assert_eq!(viewer_control.0.lock().expect("state").read_total, 8);
            assert_eq!(viewer_control.0.lock().expect("state").max_read, 8);
            other_control.change(|state| {
                state.write_blocked = false;
                state.write_chunk = 1;
                state.eof = true;
            });
            relay.await.expect("relay");
        }
        assert_eq!(other_control.output(), payload);
        assert_eq!(session.bytes(Direction::ViewerToTarget), 80);
        assert_eq!(finish(&mut session, &clock), CloseCause::PeerEof);
    }

    #[test]
    fn reuses_transferred_storage_without_a_second_b_allocation() {
        let mut bytes = Vec::with_capacity(32);
        bytes.extend_from_slice(b"prefix");
        let original = bytes.as_ptr();
        let direction = BufferedDirection::new(bytes, 32);
        assert_eq!(direction.storage.as_ptr(), original);
        assert_eq!(&direction.storage[..direction.end], b"prefix");
        assert_eq!(direction.storage.len(), 32);
        assert_eq!(direction.storage.capacity(), 32);
        let small = BufferedDirection::new(vec![1, 2, 3], 5);
        assert_eq!(small.storage.len(), 5);
        assert_eq!(small.storage.capacity(), 5);
    }

    /// spec: management.md:MGT-005 partial
    /// spec: relay.md:RLY-020 partial
    #[tokio::test(start_paused = true)]
    async fn counts_only_successful_payload_writes() {
        for failed_direction in 0..2 {
            let clock = TestClock::new();
            let mut session = model(Mode::Mode1);
            let (_sender, receiver) = watch::channel(RelayLifecycle::Running);
            let (mut viewer, viewer_control) = stream(b"later-viewer", false);
            let (mut other, other_control) = stream(b"later-target", false);
            let failed = if failed_direction == 0 {
                &other_control
            } else {
                &viewer_control
            };
            failed.change(|state| {
                state.write_chunk = 2;
                state.error_after = Some(3);
            });
            let updates = Mutex::new(Vec::new());
            run_session(
                &mut viewer,
                &mut other,
                b"viewer-trailing".to_vec(),
                b"target-trailing".to_vec(),
                &mut session,
                clock.as_ref(),
                receiver,
                16,
                E,
                |value| {
                    updates.lock().expect("updates").push((
                        value.bytes(Direction::ViewerToTarget),
                        value.bytes(Direction::TargetToViewer),
                        value.close_cause(),
                    ));
                },
            )
            .await
            .expect("relay failure is a model transition");
            let direction = if failed_direction == 0 {
                Direction::ViewerToTarget
            } else {
                Direction::TargetToViewer
            };
            assert_eq!(session.bytes(direction), 3);
            assert_eq!(session.close_cause(), Some(CloseCause::ConnectionError));
            assert_eq!(
                session.close_side(),
                Some(if failed_direction == 0 {
                    Side::Target
                } else {
                    Side::Viewer
                })
            );
            let updates = updates.lock().expect("updates");
            let selected: Vec<_> = updates
                .iter()
                .map(|value| {
                    if failed_direction == 0 {
                        value.0
                    } else {
                        value.1
                    }
                })
                .collect();
            assert!(
                selected.contains(&2) && selected.contains(&3),
                "every partial write is published"
            );
            assert!(selected.windows(2).all(|values| values[0] <= values[1]));
            assert_eq!(failed.output().len(), 3);
            assert!(
                viewer_control
                    .actions()
                    .contains(&RelayCloseAction::StreamError(ConnectionErrorKind::IoError))
            );
            assert!(
                other_control
                    .actions()
                    .contains(&RelayCloseAction::StreamError(ConnectionErrorKind::IoError))
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn treats_zero_writes_as_output_side_errors() {
        let clock = TestClock::new();
        let mut session = model(Mode::Mode2);
        let (_sender, receiver) = watch::channel(RelayLifecycle::Running);
        let (mut viewer, _) = stream(&[], false);
        let (mut other, other_control) = stream(&[], false);
        other_control.change(|state| state.write_zero = true);
        run_session(
            &mut viewer,
            &mut other,
            b"x".to_vec(),
            Vec::new(),
            &mut session,
            clock.as_ref(),
            receiver,
            4,
            E,
            |_| {},
        )
        .await
        .expect("relay");
        assert_eq!(session.close_side(), Some(Side::Server));
        assert_eq!(session.bytes(Direction::ViewerToTarget), 0);
        assert_eq!(finish(&mut session, &clock), CloseCause::ConnectionError);
    }

    /// spec: relay.md:RLY-019 partial
    #[tokio::test(start_paused = true)]
    async fn half_close_allows_a_late_response_in_either_direction() {
        for first in 0..2 {
            let clock = TestClock::new();
            let mut session = model(Mode::Mode2);
            let (_sender, receiver) = watch::channel(RelayLifecycle::Running);
            let (mut viewer, viewer_control) =
                stream(if first == 0 { b"request" } else { b"" }, first == 0);
            let (mut other, other_control) =
                stream(if first == 1 { b"request" } else { b"" }, first == 1);
            {
                let mut relay = pin!(run_session(
                    &mut viewer,
                    &mut other,
                    Vec::new(),
                    Vec::new(),
                    &mut session,
                    clock.as_ref(),
                    receiver,
                    8,
                    E,
                    |_| {}
                ));
                pump(&mut relay).await;
                let responder = if first == 0 {
                    &other_control
                } else {
                    &viewer_control
                };
                assert_eq!(responder.output(), b"request");
                assert!(responder.0.lock().expect("state").half_closed);
                assert_eq!(
                    viewer_control
                        .actions()
                        .iter()
                        .filter(|&&action| action != RelayCloseAction::WriteHalf)
                        .count(),
                    0
                );
                advance(E - Duration::from_millis(1)).await;
                responder.feed(b"late-response", true);
                relay.await.expect("reply drains before E");
            }
            let requester = if first == 0 {
                &viewer_control
            } else {
                &other_control
            };
            assert_eq!(requester.output(), b"late-response");
            assert_eq!(finish(&mut session, &clock), CloseCause::PeerEof);
        }
    }

    /// spec: relay.md:RLY-019 partial
    #[tokio::test(start_paused = true)]
    async fn expires_e_at_equality_including_pending_shutdown_and_final_close() {
        for blocked_stage in 0..3 {
            let clock = TestClock::new();
            let mut session = model(Mode::Mode1);
            let (_sender, receiver) = watch::channel(RelayLifecycle::Running);
            let (mut viewer, viewer_control) = stream(&[], true);
            let (mut other, other_control) = stream(&[], blocked_stage != 0);
            if blocked_stage == 1 {
                other_control.change(|state| state.half = CloseBehavior::Pending);
            }
            if blocked_stage == 2 {
                viewer_control.change(|state| state.terminal = CloseBehavior::Pending);
            }
            {
                let mut relay = pin!(run_session(
                    &mut viewer,
                    &mut other,
                    Vec::new(),
                    Vec::new(),
                    &mut session,
                    clock.as_ref(),
                    receiver,
                    4,
                    E,
                    |_| {}
                ));
                pump(&mut relay).await;
                advance(E).await;
                relay.await.expect("E bounds all cleanup stages");
            }
            assert_eq!(session.close_cause(), Some(CloseCause::CloseTimeout));
            assert_eq!(session.close_side(), Some(Side::Viewer));
            assert_eq!(clock.monotonic(), E);
            assert_eq!(finish(&mut session, &clock), CloseCause::CloseTimeout);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn checks_e_after_successful_writes_and_shutdowns() {
        for jump_during_write in [false, true] {
            let clock = TestClock::new();
            let mut session = model(Mode::Mode1);
            let (_sender, receiver) = watch::channel(RelayLifecycle::Running);
            let (mut viewer, viewer_control) = stream(&[], true);
            let (mut other, other_control) = stream(&[], false);
            {
                let mut relay = pin!(run_session(
                    &mut viewer,
                    &mut other,
                    Vec::new(),
                    Vec::new(),
                    &mut session,
                    clock.as_ref(),
                    receiver,
                    8,
                    E,
                    |_| {}
                ));
                pump(&mut relay).await;
                if jump_during_write {
                    viewer_control.change(|state| state.jump_on_write = Some((clock.clone(), E)));
                    other_control.feed(b"reply", true);
                } else {
                    viewer_control.change(|state| state.jump_on_half = Some((clock.clone(), E)));
                    other_control.feed(&[], true);
                }
                relay.await.expect("post-operation deadline");
            }
            assert_eq!(session.close_cause(), Some(CloseCause::CloseTimeout));
            assert_eq!(
                viewer_control.output(),
                if jump_during_write {
                    &b"reply"[..]
                } else {
                    &b""[..]
                }
            );
            assert_eq!(
                session.bytes(Direction::TargetToViewer),
                if jump_during_write { 5 } else { 0 }
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn never_reads_over_undelivered_trailing_data() {
        let clock = TestClock::new();
        let mut session = model(Mode::Mode1);
        let (_sender, receiver) = watch::channel(RelayLifecycle::Running);
        let (mut viewer, viewer_control) = stream(b"next", true);
        let (mut other, other_control) = stream(b"answer", true);
        other_control.change(|state| state.write_blocked = true);
        {
            let mut relay = pin!(run_session(
                &mut viewer,
                &mut other,
                b"full".to_vec(),
                Vec::new(),
                &mut session,
                clock.as_ref(),
                receiver,
                4,
                E,
                |_| {}
            ));
            pump(&mut relay).await;
            assert_eq!(viewer_control.0.lock().expect("state").read_calls, 0);
            assert_eq!(viewer_control.output(), b"answer");
            other_control.change(|state| state.write_blocked = false);
            relay.await.expect("trailing drains");
        }
        assert_eq!(other_control.output(), b"fullnext");
    }

    #[tokio::test(start_paused = true)]
    async fn does_not_restart_e_when_the_other_eof_arrives() {
        let clock = TestClock::new();
        let mut session = model(Mode::Mode1);
        let (_sender, receiver) = watch::channel(RelayLifecycle::Running);
        let (mut viewer, viewer_control) = stream(&[], true);
        let (mut other, other_control) = stream(&[], false);
        viewer_control.change(|state| state.terminal = CloseBehavior::Pending);
        {
            let mut relay = pin!(run_session(
                &mut viewer,
                &mut other,
                Vec::new(),
                Vec::new(),
                &mut session,
                clock.as_ref(),
                receiver,
                4,
                E,
                |_| {}
            ));
            pump(&mut relay).await;
            advance(E - Duration::from_secs(1)).await;
            other_control.feed(&[], true);
            pump(&mut relay).await;
            advance(Duration::from_secs(1)).await;
            relay.await.expect("the original E expires");
        }
        assert_eq!(session.first_eof(), Some((Side::Viewer, Duration::ZERO)));
        assert_eq!(session.close_cause(), Some(CloseCause::CloseTimeout));
        assert_eq!(clock.monotonic(), E);
    }

    /// spec: relay.md:RLY-020 partial
    #[tokio::test(start_paused = true)]
    async fn errors_on_the_remaining_direction_replace_provisional_eof() {
        for shutdown_error in [false, true] {
            let clock = TestClock::new();
            let mut session = model(Mode::Mode2);
            let (_sender, receiver) = watch::channel(RelayLifecycle::Running);
            let (mut viewer, viewer_control) = stream(&[], true);
            let (mut other, other_control) = stream(&[], false);
            {
                let mut relay = pin!(run_session(
                    &mut viewer,
                    &mut other,
                    Vec::new(),
                    Vec::new(),
                    &mut session,
                    clock.as_ref(),
                    receiver,
                    4,
                    E,
                    |_| {}
                ));
                pump(&mut relay).await;
                if shutdown_error {
                    viewer_control.change(|state| state.half = CloseBehavior::Error);
                    other_control.feed(&[], true);
                } else {
                    other_control.change(|state| state.read_error = true);
                }
                assert_eq!(
                    relay.await.expect("failed direction"),
                    Some(ConnectionErrorKind::IoError)
                );
            }
            assert_eq!(session.close_cause(), Some(CloseCause::ConnectionError));
            assert_eq!(
                session.close_side(),
                Some(if shutdown_error {
                    Side::Viewer
                } else {
                    Side::Server
                })
            );
            assert!(
                viewer_control
                    .actions()
                    .contains(&RelayCloseAction::StreamError(ConnectionErrorKind::IoError))
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn cleanup_errors_do_not_replace_completed_normal_directions() {
        let clock = TestClock::new();
        let mut session = model(Mode::Mode1);
        let (_sender, receiver) = watch::channel(RelayLifecycle::Running);
        let (mut viewer, viewer_control) = stream(b"v", true);
        let (mut other, other_control) = stream(b"t", true);
        viewer_control.change(|state| state.terminal = CloseBehavior::Error);
        other_control.change(|state| state.terminal = CloseBehavior::Error);
        assert_eq!(
            run_session(
                &mut viewer,
                &mut other,
                Vec::new(),
                Vec::new(),
                &mut session,
                clock.as_ref(),
                receiver,
                4,
                E,
                |_| {}
            )
            .await
            .expect("cleanup errors are not transfer errors"),
            None
        );
        assert_eq!(session.close_cause(), None);
        drop((viewer, other));
        assert_eq!(finish(&mut session, &clock), CloseCause::PeerEof);
    }

    /// spec: operations.md:OPS-008 partial
    #[tokio::test(start_paused = true)]
    async fn forwards_during_grace_and_finishes_naturally_before_g() {
        let clock = TestClock::new();
        let mut session = model(Mode::Mode1);
        let (sender, receiver) = watch::channel(RelayLifecycle::Running);
        let (mut viewer, viewer_control) = stream(&[], false);
        let (mut other, other_control) = stream(&[], false);
        {
            let mut relay = pin!(run_session(
                &mut viewer,
                &mut other,
                Vec::new(),
                Vec::new(),
                &mut session,
                clock.as_ref(),
                receiver,
                4,
                E,
                |_| {}
            ));
            pump(&mut relay).await;
            sender.send_replace(RelayLifecycle::Grace {
                deadline: Duration::from_secs(20),
            });
            viewer_control.feed(b"during-grace", false);
            other_control.feed(b"reply", false);
            pump(&mut relay).await;
            assert_eq!(other_control.output(), b"during-grace");
            assert_eq!(viewer_control.output(), b"reply");
            advance(Duration::from_secs(19)).await;
            viewer_control.feed(&[], true);
            other_control.feed(&[], true);
            relay.await.expect("natural EOF before G");
        }
        drop((viewer, other));
        assert_eq!(finish(&mut session, &clock), CloseCause::PeerEof);
    }

    /// spec: operations.md:OPS-009 partial
    #[tokio::test(start_paused = true)]
    async fn g_expiry_preserves_eof_and_never_waits_for_transport_close() {
        for grace in [Duration::ZERO, Duration::from_secs(3)] {
            let clock = TestClock::new();
            let mut session = model(Mode::Mode1);
            let (sender, receiver) = watch::channel(RelayLifecycle::Running);
            let (mut viewer, viewer_control) = stream(&[], true);
            let (mut other, other_control) = stream(&[], false);
            viewer_control.change(|state| state.terminal = CloseBehavior::Pending);
            other_control.change(|state| state.terminal = CloseBehavior::Pending);
            let updates = Mutex::new(Vec::new());
            {
                let mut relay = pin!(run_session(
                    &mut viewer,
                    &mut other,
                    Vec::new(),
                    Vec::new(),
                    &mut session,
                    clock.as_ref(),
                    receiver,
                    4,
                    E,
                    |value| updates.lock().expect("updates").push(value.close_cause())
                ));
                pump(&mut relay).await;
                sender.send_replace(RelayLifecycle::Grace { deadline: grace });
                if !grace.is_zero() {
                    pump(&mut relay).await;
                    advance(grace).await;
                }
                assert_eq!(relay.await.expect("G is a hard bound"), None);
            }
            assert_eq!(clock.monotonic(), grace);
            assert_eq!(session.close_cause(), Some(CloseCause::PeerEof));
            assert_eq!(session.close_side(), Some(Side::Viewer));
            assert!(
                viewer_control
                    .actions()
                    .contains(&RelayCloseAction::PeerEof)
            );
            assert!(other_control.actions().contains(&RelayCloseAction::PeerEof));
        }
    }

    /// spec: operations.md:OPS-009 partial
    /// spec: operations.md:OPS-042 partial
    #[tokio::test(start_paused = true)]
    async fn force_preserves_an_error_whose_terminal_close_is_still_pending() {
        for second_request in [false, true] {
            for kind in [
                ConnectionErrorKind::IoError,
                ConnectionErrorKind::WsProtocolError,
                ConnectionErrorKind::WsTextMessage,
                ConnectionErrorKind::WsMessageTooLarge,
            ] {
                let clock = TestClock::new();
                let mut session = model(Mode::Mode1);
                let (sender, receiver) = watch::channel(RelayLifecycle::Running);
                let (mut viewer, viewer_control) = stream(&[], false);
                let (mut other, other_control) = stream(&[], false);
                viewer_control.change(|state| {
                    state.read_error = true;
                    state.typed_error = Some(kind);
                    state.terminal = CloseBehavior::Pending;
                });
                other_control.change(|state| state.terminal = CloseBehavior::Pending);
                let grace = Duration::from_secs(3);
                {
                    let mut relay = pin!(run_session(
                        &mut viewer,
                        &mut other,
                        Vec::new(),
                        Vec::new(),
                        &mut session,
                        clock.as_ref(),
                        receiver,
                        4,
                        E,
                        |_| {}
                    ));
                    pump(&mut relay).await;
                    if second_request {
                        sender.send_replace(RelayLifecycle::Force);
                    } else {
                        sender.send_replace(RelayLifecycle::Grace { deadline: grace });
                        pump(&mut relay).await;
                        advance(grace).await;
                    }
                    assert_eq!(relay.await.expect("Force or G bounds cleanup"), Some(kind));
                }
                assert_eq!(session.close_cause(), Some(CloseCause::ConnectionError));
                assert_eq!(session.close_side(), Some(Side::Viewer));
                assert_eq!(
                    clock.monotonic(),
                    if second_request {
                        Duration::ZERO
                    } else {
                        grace
                    }
                );
                assert!(
                    other_control
                        .actions()
                        .contains(&RelayCloseAction::StreamError(kind))
                );
            }
        }
    }

    #[tokio::test(start_paused = true)]
    async fn zero_grace_stops_new_reads_and_terminal_force_is_observed() {
        for force_in_close in [false, true] {
            let clock = TestClock::new();
            let mut session = model(Mode::Mode1);
            let state = if force_in_close {
                RelayLifecycle::Running
            } else {
                RelayLifecycle::Grace {
                    deadline: Duration::ZERO,
                }
            };
            let (sender, receiver) = watch::channel(state);
            let (mut viewer, viewer_control) = stream(b"unread", force_in_close);
            let (mut other, other_control) = stream(&[], force_in_close);
            if force_in_close {
                viewer_control.change(|state| state.force_on_terminal = Some(sender.clone()));
            }
            run_session(
                &mut viewer,
                &mut other,
                Vec::new(),
                Vec::new(),
                &mut session,
                clock.as_ref(),
                receiver,
                8,
                E,
                |_| {},
            )
            .await
            .expect("force");
            let expected_cause = if force_in_close {
                CloseCause::PeerEof
            } else {
                CloseCause::ServerShutdown
            };
            assert_eq!(session.close_cause(), Some(expected_cause));
            if !force_in_close {
                assert_eq!(viewer_control.0.lock().expect("state").read_calls, 0);
                assert!(other_control.output().is_empty());
            }
            assert!(viewer_control.actions().contains(&if force_in_close {
                RelayCloseAction::PeerEof
            } else {
                RelayCloseAction::ServerShutdown
            }));
            assert!(other_control.actions().contains(&if force_in_close {
                RelayCloseAction::PeerEof
            } else {
                RelayCloseAction::ServerShutdown
            }));
        }
    }

    #[tokio::test(start_paused = true)]
    async fn bounds_terminal_error_cleanup_by_e_without_relabelling_the_error() {
        let clock = TestClock::new();
        let mut session = model(Mode::Mode1);
        let (_sender, receiver) = watch::channel(RelayLifecycle::Running);
        let (mut viewer, viewer_control) = stream(&[], false);
        let (mut other, other_control) = stream(&[], false);
        viewer_control.change(|state| {
            state.read_error = true;
            state.terminal = CloseBehavior::Pending;
        });
        other_control.change(|state| state.terminal = CloseBehavior::Pending);
        {
            let mut relay = pin!(run_session(
                &mut viewer,
                &mut other,
                Vec::new(),
                Vec::new(),
                &mut session,
                clock.as_ref(),
                receiver,
                4,
                E,
                |_| {}
            ));
            assert_eq!(
                relay.as_mut().await.expect("bounded error cleanup"),
                Some(ConnectionErrorKind::IoError)
            );
        }
        assert_eq!(clock.monotonic(), E);
        assert_eq!(session.close_cause(), Some(CloseCause::ConnectionError));
    }

    /// spec: relay.md:RLY-021 partial
    /// spec: relay.md:RLY-023 partial
    /// spec: relay.md:RLY-026 partial
    #[tokio::test(start_paused = true)]
    async fn one_failed_session_cannot_close_or_mix_an_independent_session() {
        let clock = TestClock::new();
        let mut first_model = model(Mode::Mode1);
        let mut second_model = model(Mode::Mode2);
        let (_first_sender, first_receiver) = watch::channel(RelayLifecycle::Running);
        let (_second_sender, second_receiver) = watch::channel(RelayLifecycle::Running);
        let (mut first_viewer, first_control) = stream(&[], false);
        let (mut first_other, _) = stream(&[], false);
        let (mut second_viewer, second_viewer_control) = stream(b"before", false);
        let (mut second_other, second_other_control) = stream(b"reply-before", false);
        {
            let mut first = pin!(run_session(
                &mut first_viewer,
                &mut first_other,
                Vec::new(),
                Vec::new(),
                &mut first_model,
                clock.as_ref(),
                first_receiver,
                8,
                E,
                |_| {}
            ));
            let mut second = pin!(run_session(
                &mut second_viewer,
                &mut second_other,
                Vec::new(),
                Vec::new(),
                &mut second_model,
                clock.as_ref(),
                second_receiver,
                8,
                E,
                |_| {}
            ));
            pump(&mut first).await;
            pump(&mut second).await;
            first_control.change(|state| state.read_error = true);
            first.await.expect("first session ends");
            assert!(second_viewer_control.actions().is_empty());
            assert!(second_other_control.actions().is_empty());
            second_viewer_control.feed(b"-after", true);
            second_other_control.feed(b"-reply-after", true);
            second.await.expect("second session independently drains");
        }
        assert_eq!(first_model.close_cause(), Some(CloseCause::ConnectionError));
        assert_eq!(second_other_control.output(), b"before-after");
        assert_eq!(second_viewer_control.output(), b"reply-before-reply-after");
        drop((second_viewer, second_other));
        assert_eq!(finish(&mut second_model, &clock), CloseCause::PeerEof);
    }

    /// spec: relay.md:RLY-027 partial
    #[tokio::test(start_paused = true)]
    async fn does_not_turn_long_idle_time_into_dead_connection() {
        let clock = TestClock::new();
        let mut session = model(Mode::Mode1);
        let (sender, receiver) = watch::channel(RelayLifecycle::Running);
        let (mut viewer, viewer_control) = stream(&[], false);
        let (mut other, other_control) = stream(&[], false);
        {
            let mut relay = pin!(run_session(
                &mut viewer,
                &mut other,
                Vec::new(),
                Vec::new(),
                &mut session,
                clock.as_ref(),
                receiver,
                4,
                E,
                |_| {}
            ));
            pump(&mut relay).await;
            // Closing the notification channel is not a forced shutdown.
            drop(sender);
            advance(Duration::from_secs(86_400)).await;
            pump(&mut relay).await;
            assert!(viewer_control.actions().is_empty());
            assert!(other_control.actions().is_empty());
            viewer_control.feed(b"still-live", true);
            other_control.feed(b"response", true);
            relay.await.expect("idle sessions still forward");
        }
        assert_eq!(other_control.output(), b"still-live");
        assert_eq!(viewer_control.output(), b"response");
        drop((viewer, other));
        assert_eq!(finish(&mut session, &clock), CloseCause::PeerEof);
    }

    #[tokio::test(start_paused = true)]
    async fn preserves_an_external_dead_connection_verdict_without_inventing_a_timer() {
        let clock = TestClock::new();
        let mut session = model(Mode::Mode2);
        session
            .dead_connection(Side::Server)
            .expect("external verdict");
        let (_sender, receiver) = watch::channel(RelayLifecycle::Running);
        let (mut viewer, viewer_control) = stream(b"not-relayed", false);
        let (mut other, other_control) = stream(&[], false);
        run_session(
            &mut viewer,
            &mut other,
            Vec::new(),
            Vec::new(),
            &mut session,
            clock.as_ref(),
            receiver,
            16,
            E,
            |_| {},
        )
        .await
        .expect("cleanup");
        assert_eq!(session.close_cause(), Some(CloseCause::DeadConnection));
        assert!(
            viewer_control
                .actions()
                .contains(&RelayCloseAction::DeadConnection)
        );
        assert!(
            other_control
                .actions()
                .contains(&RelayCloseAction::DeadConnection)
        );
        assert_eq!(viewer_control.0.lock().expect("state").read_calls, 0);
        assert!(other_control.output().is_empty());
    }

    /// spec: management.md:MGT-005 partial
    #[tokio::test(start_paused = true)]
    async fn publishes_saturating_counters_without_resetting_prior_progress() {
        let clock = TestClock::new();
        let mut session = model(Mode::Mode1);
        session
            .record_write(Direction::ViewerToTarget, u64::MAX - 1)
            .expect("prior progress");
        let (_sender, receiver) = watch::channel(RelayLifecycle::Running);
        let (mut viewer, _) = stream(&[], true);
        let (mut other, _) = stream(&[], true);
        let updates = Mutex::new(Vec::new());
        run_session(
            &mut viewer,
            &mut other,
            b"payload".to_vec(),
            Vec::new(),
            &mut session,
            clock.as_ref(),
            receiver,
            8,
            E,
            |value| {
                updates
                    .lock()
                    .expect("updates")
                    .push(value.bytes(Direction::ViewerToTarget))
            },
        )
        .await
        .expect("relay");
        assert_eq!(session.bytes(Direction::ViewerToTarget), u64::MAX);
        assert!(
            updates
                .lock()
                .expect("updates")
                .iter()
                .all(|&bytes| bytes == u64::MAX)
        );
    }

    fn stream_failure(kind: Option<ConnectionErrorKind>) -> io::Error {
        match kind {
            Some(kind) => RelayStreamError::new(kind).into_io_error(),
            None => io::Error::other("transport failure"),
        }
    }

    /// spec: websocket.md:WS-019 partial
    /// spec: websocket.md:WS-008 partial
    /// spec: websocket.md:WS-010 partial
    #[tokio::test(start_paused = true)]
    async fn retains_typed_stream_errors_for_the_caller_and_close_actions() {
        for kind in [
            ConnectionErrorKind::WsProtocolError,
            ConnectionErrorKind::WsTextMessage,
            ConnectionErrorKind::WsMessageTooLarge,
        ] {
            for write_error in [false, true] {
                let clock = TestClock::new();
                let mut session = model(Mode::Mode1);
                let (_sender, receiver) = watch::channel(RelayLifecycle::Running);
                let (mut viewer, viewer_control) = stream(&[], false);
                let (mut other, other_control) = stream(&[], false);
                viewer_control.change(|state| {
                    state.transport = Transport::Ws;
                    state.typed_error = Some(kind);
                    if write_error {
                        state.error_after = Some(0);
                    } else {
                        state.read_error = true;
                    }
                });
                let reverse_trailing = if write_error {
                    b"payload".to_vec()
                } else {
                    Vec::new()
                };
                let outcome = run_session(
                    &mut viewer,
                    &mut other,
                    Vec::new(),
                    reverse_trailing,
                    &mut session,
                    clock.as_ref(),
                    receiver,
                    8,
                    E,
                    |_| {},
                )
                .await
                .expect("typed error");
                assert_eq!(outcome, Some(kind));
                assert_eq!(session.close_cause(), Some(CloseCause::ConnectionError));
                assert_eq!(session.close_side(), Some(Side::Viewer));
                assert!(
                    viewer_control
                        .actions()
                        .contains(&RelayCloseAction::StreamError(kind))
                );
                assert!(
                    other_control
                        .actions()
                        .contains(&RelayCloseAction::StreamError(kind))
                );
                assert_eq!(session.bytes(Direction::TargetToViewer), 0);
            }
        }
    }

    /// spec: websocket.md:WS-016 partial
    #[tokio::test(start_paused = true)]
    async fn tcp_io_error_sends_the_websocket_peer_error_action() {
        let clock = TestClock::new();
        let mut session = model(Mode::Mode1);
        let (_sender, receiver) = watch::channel(RelayLifecycle::Running);
        let (mut viewer, viewer_control) = stream(&[], false);
        let (mut other, other_control) = stream(&[], false);
        viewer_control.change(|state| state.transport = Transport::Ws);
        other_control.change(|state| state.read_error = true);
        assert_eq!(
            run_session(
                &mut viewer,
                &mut other,
                Vec::new(),
                Vec::new(),
                &mut session,
                clock.as_ref(),
                receiver,
                32,
                E,
                |_| {},
            )
            .await
            .expect("peer TCP error"),
            Some(ConnectionErrorKind::IoError)
        );
        assert_eq!(
            viewer_control.actions(),
            [RelayCloseAction::StreamError(ConnectionErrorKind::IoError)]
        );
        assert_eq!(session.close_side(), Some(Side::Target));
        assert_eq!(finish(&mut session, &clock), CloseCause::ConnectionError);
    }

    /// spec: websocket.md:WS-016 partial
    /// spec: websocket.md:WS-017 partial
    #[tokio::test(start_paused = true)]
    async fn client_close_finishes_only_the_started_frame_and_discards_tcp_data() {
        for pending_write in [0, 5] {
            let clock = TestClock::new();
            let mut session = model(Mode::Mode2);
            let (_sender, receiver) = watch::channel(RelayLifecycle::Running);
            let (mut viewer, viewer_control) = stream(&[], true);
            let (mut other, other_control) = stream(b"discard-after-close", false);
            viewer_control.change(|state| {
                state.transport = Transport::Ws;
                state.pending_write = pending_write;
                state.write_chunk = 2;
            });
            {
                let mut relay = pin!(run_session(
                    &mut viewer,
                    &mut other,
                    Vec::new(),
                    b"frame-extra".to_vec(),
                    &mut session,
                    clock.as_ref(),
                    receiver,
                    32,
                    E,
                    |_| {}
                ));
                pump(&mut relay).await;
                assert!(other_control.output().is_empty());
                assert!(other_control.0.lock().expect("state").half_closed);
                assert_eq!(viewer_control.output(), &b"frame"[..pending_write]);
                assert_eq!(viewer_control.actions(), [RelayCloseAction::PeerEof]);
                assert_eq!(
                    other_control.0.lock().expect("state").read_total,
                    b"discard-after-close".len()
                );
                other_control.feed(b"also-discarded", true);
                relay.await.expect("TCP EOF after Close");
            }
            assert_eq!(session.bytes(Direction::ViewerToTarget), 0);
            assert_eq!(
                session.bytes(Direction::TargetToViewer),
                pending_write as u64
            );
            assert_eq!(finish(&mut session, &clock), CloseCause::PeerEof);
        }
    }

    /// spec: websocket.md:WS-016 partial
    #[tokio::test(start_paused = true)]
    async fn tcp_eof_sends_close_after_drain_and_forwards_client_data_until_close() {
        let clock = TestClock::new();
        let mut session = model(Mode::Mode1);
        let (_sender, receiver) = watch::channel(RelayLifecycle::Running);
        let (mut viewer, viewer_control) = stream(&[], false);
        let (mut other, other_control) = stream(b"tcp-data", true);
        viewer_control.change(|state| state.transport = Transport::Wss);
        {
            let mut relay = pin!(run_session(
                &mut viewer,
                &mut other,
                Vec::new(),
                b"trailing-".to_vec(),
                &mut session,
                clock.as_ref(),
                receiver,
                32,
                E,
                |_| {}
            ));
            pump(&mut relay).await;
            assert_eq!(viewer_control.output(), b"trailing-tcp-data");
            assert_eq!(viewer_control.actions(), [RelayCloseAction::WriteHalf]);
            viewer_control.feed(b"after-server-close", false);
            pump(&mut relay).await;
            assert_eq!(other_control.output(), b"after-server-close");
            viewer_control.feed(&[], true);
            relay.await.expect("client Close after TCP EOF");
        }
        assert_eq!(viewer_control.actions(), [RelayCloseAction::WriteHalf]);
        assert_eq!(session.bytes(Direction::ViewerToTarget), 18);
        assert_eq!(session.bytes(Direction::TargetToViewer), 17);
        assert_eq!(finish(&mut session, &clock), CloseCause::PeerEof);
    }

    /// spec: websocket.md:WS-017 partial
    #[tokio::test(start_paused = true)]
    async fn websocket_close_waits_expire_at_the_same_absolute_e_in_both_orders() {
        for client_first in [false, true] {
            let clock = TestClock::new();
            let mut session = model(Mode::Mode1);
            let (_sender, receiver) = watch::channel(RelayLifecycle::Running);
            let (mut viewer, viewer_control) = stream(&[], client_first);
            let (mut other, other_control) = stream(&[], !client_first);
            viewer_control.change(|state| state.transport = Transport::Ws);
            let close = if client_first {
                RelayCloseAction::PeerEof
            } else {
                RelayCloseAction::WriteHalf
            };
            {
                let mut relay = pin!(run_session(
                    &mut viewer,
                    &mut other,
                    Vec::new(),
                    Vec::new(),
                    &mut session,
                    clock.as_ref(),
                    receiver,
                    32,
                    E,
                    |_| {}
                ));
                pump(&mut relay).await;
                assert_eq!(viewer_control.actions(), [close]);
                advance(E / 2).await;
                if client_first {
                    other_control.feed(b"discard", false);
                } else {
                    viewer_control.feed(b"forward", false);
                }
                pump(&mut relay).await;
                advance(E / 2).await;
                relay.await.expect("absolute E expiry");
            }
            assert_eq!(viewer_control.actions(), [close]);
            assert_eq!(clock.monotonic(), E);
            assert_eq!(finish(&mut session, &clock), CloseCause::CloseTimeout);
        }
    }

    /// spec: websocket.md:WS-017 partial
    /// spec: operations.md:OPS-009 partial
    #[tokio::test(start_paused = true)]
    async fn force_after_either_close_order_never_sends_a_second_websocket_close() {
        for client_first in [false, true] {
            let clock = TestClock::new();
            let mut session = model(Mode::Mode1);
            let (sender, receiver) = watch::channel(RelayLifecycle::Running);
            let (mut viewer, viewer_control) = stream(&[], client_first);
            let (mut other, other_control) = stream(&[], !client_first);
            viewer_control.change(|state| state.transport = Transport::Ws);
            let close = if client_first {
                RelayCloseAction::PeerEof
            } else {
                RelayCloseAction::WriteHalf
            };
            {
                let mut relay = pin!(run_session(
                    &mut viewer,
                    &mut other,
                    Vec::new(),
                    Vec::new(),
                    &mut session,
                    clock.as_ref(),
                    receiver,
                    32,
                    E,
                    |_| {}
                ));
                pump(&mut relay).await;
                assert_eq!(viewer_control.actions(), [close]);
                sender.send_replace(RelayLifecycle::Force);
                relay.await.expect("Force ends the wait immediately");
            }
            assert_eq!(viewer_control.actions(), [close]);
            assert!(other_control.actions().contains(&if client_first {
                RelayCloseAction::PeerEof
            } else {
                RelayCloseAction::WriteHalf
            }));
            assert_eq!(clock.monotonic(), Duration::ZERO);
            assert_eq!(finish(&mut session, &clock), CloseCause::PeerEof);
        }
    }

    /// spec: websocket.md:WS-016 partial
    /// spec: operations.md:OPS-042 partial
    #[tokio::test(start_paused = true)]
    async fn force_before_first_close_poll_preserves_the_selected_websocket_action() {
        for (kind, eof_side, expected_action) in [
            (None, None, RelayCloseAction::ServerShutdown),
            (None, Some(Side::Viewer), RelayCloseAction::PeerEof),
            (None, Some(Side::Target), RelayCloseAction::WriteHalf),
            (
                Some(ConnectionErrorKind::WsProtocolError),
                None,
                RelayCloseAction::StreamError(ConnectionErrorKind::WsProtocolError),
            ),
            (
                Some(ConnectionErrorKind::WsTextMessage),
                None,
                RelayCloseAction::StreamError(ConnectionErrorKind::WsTextMessage),
            ),
            (
                Some(ConnectionErrorKind::WsMessageTooLarge),
                None,
                RelayCloseAction::StreamError(ConnectionErrorKind::WsMessageTooLarge),
            ),
        ] {
            let clock = TestClock::new();
            let mut session = model(Mode::Mode1);
            let fresh = kind.is_none() && eof_side.is_none();
            let (sender, receiver) = watch::channel(if fresh {
                RelayLifecycle::Force
            } else {
                RelayLifecycle::Running
            });
            let (mut viewer, viewer_control) = stream(&[], eof_side == Some(Side::Viewer));
            let (mut other, other_control) = stream(&[], eof_side == Some(Side::Target));
            viewer_control.change(|state| {
                state.transport = Transport::Ws;
                state.read_error = kind.is_some();
                state.typed_error = kind;
                state.half = CloseBehavior::Pending;
                state.terminal = CloseBehavior::Pending;
            });
            other_control.change(|state| {
                state.half = CloseBehavior::Pending;
                state.terminal = CloseBehavior::Pending;
            });
            let outcome = run_session(
                &mut viewer,
                &mut other,
                Vec::new(),
                Vec::new(),
                &mut session,
                clock.as_ref(),
                receiver,
                32,
                E,
                |_| {
                    assert!(viewer_control.actions().is_empty());
                    assert!(other_control.actions().is_empty());
                    sender.send_replace(RelayLifecycle::Force);
                },
            )
            .await
            .expect("Force does not wait for a first close poll");
            assert_eq!(outcome, kind);
            assert_eq!(viewer_control.actions(), [expected_action]);
            assert_eq!(other_control.actions(), [expected_action]);
            assert_eq!(clock.monotonic(), Duration::ZERO);
            assert_eq!(
                session.close_cause(),
                Some(if kind.is_some() {
                    CloseCause::ConnectionError
                } else if fresh {
                    CloseCause::ServerShutdown
                } else {
                    CloseCause::PeerEof
                })
            );
            assert_eq!(
                session.close_side(),
                if kind.is_some() {
                    Some(Side::Viewer)
                } else {
                    eof_side
                }
            );
        }
    }

    /// spec: websocket.md:WS-008 partial
    /// spec: operations.md:OPS-042 partial
    #[tokio::test(start_paused = true)]
    async fn force_published_by_expired_cleanup_keeps_the_typed_error_action() {
        let clock = TestClock::new();
        let mut session = model(Mode::Mode1);
        let (sender, receiver) = watch::channel(RelayLifecycle::Running);
        let (mut viewer, viewer_control) = stream(&[], false);
        let (mut other, other_control) = stream(&[], false);
        let kind = ConnectionErrorKind::WsTextMessage;
        viewer_control.change(|state| {
            state.transport = Transport::Ws;
            state.read_error = true;
            state.typed_error = Some(kind);
            state.terminal = CloseBehavior::Pending;
        });
        other_control.change(|state| state.terminal = CloseBehavior::Pending);
        {
            let mut relay = pin!(run_session(
                &mut viewer,
                &mut other,
                Vec::new(),
                Vec::new(),
                &mut session,
                clock.as_ref(),
                receiver,
                32,
                E,
                |_| {}
            ));
            pump(&mut relay).await;
            viewer_control.change(|state| state.force_on_terminal = Some(sender.clone()));
            advance(E).await;
            assert_eq!(
                relay.await.expect("expired cleanup rechecks Force"),
                Some(kind)
            );
        }
        assert_eq!(*sender.borrow(), RelayLifecycle::Force);
        assert_eq!(session.close_cause(), Some(CloseCause::ConnectionError));
        assert_eq!(session.close_side(), Some(Side::Viewer));
        assert_eq!(clock.monotonic(), E);
        for control in [viewer_control, other_control] {
            assert!(
                control
                    .actions()
                    .iter()
                    .all(|action| *action == RelayCloseAction::StreamError(kind))
            );
        }
    }

    /// spec: websocket.md:WS-016 partial
    #[tokio::test(start_paused = true)]
    async fn late_client_close_cannot_replace_tcp_first_pending_write_half_with_an_echo() {
        for (mode, tcp_side) in [(Mode::Mode1, Side::Target), (Mode::Mode2, Side::Server)] {
            let clock = TestClock::new();
            let mut session = model(mode);
            let (_sender, receiver) = watch::channel(RelayLifecycle::Running);
            let (mut viewer, viewer_control) = stream(&[], false);
            let (mut other, _) = stream(b"before-fin", true);
            viewer_control.change(|state| {
                state.transport = Transport::Ws;
                state.half = CloseBehavior::Pending;
            });
            {
                let mut relay = pin!(run_session(
                    &mut viewer,
                    &mut other,
                    Vec::new(),
                    Vec::new(),
                    &mut session,
                    clock.as_ref(),
                    receiver,
                    32,
                    E,
                    |_| {}
                ));
                pump(&mut relay).await;
                assert_eq!(viewer_control.output(), b"before-fin");
                assert!(!viewer_control.actions().is_empty());
                advance(E / 2).await;
                viewer_control.feed(&[], true);
                pump(&mut relay).await;
                assert!(
                    viewer_control
                        .actions()
                        .iter()
                        .all(|action| *action == RelayCloseAction::WriteHalf)
                );
                viewer_control.change(|state| state.half = CloseBehavior::Ready);
                assert_eq!(
                    relay.await.expect("the original TCP-first close completes"),
                    None
                );
            }
            assert!(
                viewer_control
                    .actions()
                    .iter()
                    .all(|action| *action == RelayCloseAction::WriteHalf)
            );
            assert_eq!(session.first_eof(), Some((tcp_side, Duration::ZERO)));
            assert_eq!(clock.monotonic(), E / 2);
            assert_eq!(finish(&mut session, &clock), CloseCause::PeerEof);
            assert_eq!(session.close_side(), Some(tcp_side));
        }
    }

    /// spec: websocket.md:WS-017 partial
    #[tokio::test(start_paused = true)]
    async fn an_inflight_frame_blocked_after_client_close_cannot_extend_e() {
        let clock = TestClock::new();
        let mut session = model(Mode::Mode1);
        let (_sender, receiver) = watch::channel(RelayLifecycle::Running);
        let (mut viewer, viewer_control) = stream(&[], true);
        let (mut other, _) = stream(&[], false);
        viewer_control.change(|state| {
            state.transport = Transport::Ws;
            state.pending_write = 5;
            state.write_blocked = true;
        });
        {
            let mut relay = pin!(run_session(
                &mut viewer,
                &mut other,
                Vec::new(),
                b"frame".to_vec(),
                &mut session,
                clock.as_ref(),
                receiver,
                32,
                E,
                |_| {}
            ));
            pump(&mut relay).await;
            advance(E).await;
            relay.await.expect("inflight frame shares E");
        }
        assert!(viewer_control.output().is_empty());
        assert_eq!(session.bytes(Direction::TargetToViewer), 0);
        assert_eq!(finish(&mut session, &clock), CloseCause::CloseTimeout);
    }

    #[tokio::test(start_paused = true)]
    async fn a_read_completing_at_e_does_not_start_another_payload_write() {
        let clock = TestClock::new();
        let mut session = model(Mode::Mode1);
        let (_sender, receiver) = watch::channel(RelayLifecycle::Running);
        let (mut viewer, viewer_control) = stream(&[], true);
        let (mut other, other_control) = stream(&[], false);
        {
            let mut relay = pin!(run_session(
                &mut viewer,
                &mut other,
                Vec::new(),
                Vec::new(),
                &mut session,
                clock.as_ref(),
                receiver,
                8,
                E,
                |_| {}
            ));
            pump(&mut relay).await;
            other_control.change(|state| state.jump_on_read = Some((clock.clone(), E)));
            other_control.feed(b"too-late", false);
            relay.await.expect("E after read");
        }
        assert!(viewer_control.output().is_empty());
        assert_eq!(session.bytes(Direction::TargetToViewer), 0);
        assert_eq!(session.close_cause(), Some(CloseCause::CloseTimeout));
    }

    /// spec: relay.md:RLY-019 partial
    #[tokio::test(start_paused = true)]
    async fn e_includes_draining_the_other_directions_pending_payload() {
        let clock = TestClock::new();
        let mut session = model(Mode::Mode1);
        let (_sender, receiver) = watch::channel(RelayLifecycle::Running);
        let (mut viewer, viewer_control) = stream(&[], true);
        let (mut other, _) = stream(&[], false);
        viewer_control.change(|state| state.write_blocked = true);
        {
            let mut relay = pin!(run_session(
                &mut viewer,
                &mut other,
                Vec::new(),
                b"pending".to_vec(),
                &mut session,
                clock.as_ref(),
                receiver,
                8,
                E,
                |_| {}
            ));
            pump(&mut relay).await;
            advance(E).await;
            relay.await.expect("drain is bounded by first EOF");
        }
        assert!(viewer_control.output().is_empty());
        assert_eq!(session.bytes(Direction::TargetToViewer), 0);
        assert_eq!(session.close_cause(), Some(CloseCause::CloseTimeout));
        assert_eq!(clock.monotonic(), E);
    }

    /// spec: operations.md:OPS-009 partial
    #[tokio::test(start_paused = true)]
    async fn e_expires_at_equality_when_e_and_g_reach_deadline_together() {
        let clock = TestClock::new();
        let mut session = model(Mode::Mode1);
        let (_sender, receiver) = watch::channel(RelayLifecycle::Grace { deadline: E });
        let (mut viewer, _) = stream(&[], true);
        let (mut other, other_control) = stream(&[], false);
        other_control.change(|state| state.terminal = CloseBehavior::Pending);
        {
            let mut relay = pin!(run_session(
                &mut viewer,
                &mut other,
                Vec::new(),
                Vec::new(),
                &mut session,
                clock.as_ref(),
                receiver,
                4,
                E,
                |_| {}
            ));
            pump(&mut relay).await;
            advance(E).await;
            relay.await.expect("equality expiry");
        }
        assert_eq!(session.close_cause(), Some(CloseCause::CloseTimeout));
        assert_eq!(session.close_side(), Some(Side::Viewer));
    }

    /// Covers administrator cleanup at E-1 ms and exact E, including no new
    /// payload reads and a cleanup-only transport error that does not fake failure.
    /// spec: relay.md:RLY-020 partial
    /// spec: management.md:MGT-009 partial
    #[tokio::test(start_paused = true)]
    async fn admin_cleanup_uses_acceptance_deadline_and_stops_payload_before_closing() {
        for behavior in [
            CloseBehavior::Ready,
            CloseBehavior::Error,
            CloseBehavior::Pending,
        ] {
            let clock = TestClock::new();
            let mut session = model(Mode::Mode1);
            let (_lifecycle, receiver) = watch::channel(RelayLifecycle::Running);
            let (_command, command) = watch::channel(Some((Duration::ZERO, E)));
            let (mut viewer, viewer_control) = stream(b"must-not-read", false);
            let (mut other, other_control) = stream(b"must-not-read-either", false);
            viewer_control.change(|state| state.terminal = behavior);
            other_control.change(|state| state.terminal = behavior);
            let settings = TestSettings::new();
            settings.change(|settings| settings.timeouts.close = E);
            {
                let mut relay = pin!(run_session_controlled(
                    &mut viewer,
                    &mut other,
                    Vec::new(),
                    Vec::new(),
                    &mut session,
                    clock.as_ref(),
                    receiver,
                    [8; 2],
                    settings.as_ref(),
                    command,
                    |_, _| {},
                ));
                if matches!(behavior, CloseBehavior::Pending) {
                    poll_fn(|cx| {
                        assert!(relay.as_mut().poll(cx).is_pending());
                        Poll::Ready(())
                    })
                    .await;
                    advance(E - Duration::from_millis(1)).await;
                    poll_fn(|cx| {
                        assert!(relay.as_mut().poll(cx).is_pending());
                        Poll::Ready(())
                    })
                    .await;
                    advance(Duration::from_millis(1)).await;
                }
                assert_eq!(
                    relay.await.unwrap_or_else(|e| panic!("admin cleanup: {e}")),
                    None
                );
            }
            let pending = matches!(behavior, CloseBehavior::Pending);
            assert_eq!(
                session.close_cause(),
                Some(if pending {
                    CloseCause::CloseTimeout
                } else {
                    CloseCause::AdminTerminated
                })
            );
            assert_eq!(session.close_side(), None);
            assert_eq!(session.bytes(Direction::ViewerToTarget), 0);
            assert_eq!(session.bytes(Direction::TargetToViewer), 0);
            assert_eq!(
                viewer_control
                    .0
                    .lock()
                    .unwrap_or_else(|e| panic!("viewer: {e}"))
                    .read_calls,
                0
            );
            assert_eq!(
                other_control
                    .0
                    .lock()
                    .unwrap_or_else(|e| panic!("other: {e}"))
                    .read_calls,
                0
            );
            assert!(
                viewer_control
                    .actions()
                    .contains(&RelayCloseAction::AdminTerminated)
            );
            assert!(
                other_control
                    .actions()
                    .contains(&RelayCloseAction::AdminTerminated)
            );
        }
    }

    /// The existing adapter maps AdminTerminated to Close 4005; here the
    /// controlled transport refuses that close until the started frame drains.
    /// spec: management.md:MGT-009 partial
    /// spec: relay.md:RLY-020 partial
    #[tokio::test(start_paused = true)]
    async fn admin_finishes_only_started_ws_frame_before_terminal_close_without_waiting_for_e() {
        let clock = TestClock::new();
        let mut session = model(Mode::Mode1);
        session
            .record_write(Direction::TargetToViewer, 2)
            .unwrap_or_else(|e| panic!("prior write: {e}"));
        let (_lifecycle, receiver) = watch::channel(RelayLifecycle::Running);
        let (_command, command) = watch::channel(Some((Duration::ZERO, E)));
        let (mut viewer, viewer_control) = stream(b"unread", false);
        let (mut other, other_control) = stream(b"no-new-read", false);
        viewer_control.change(|state| {
            state.transport = Transport::Ws;
            state.pending_write = 4;
            state.output.extend_from_slice(b"he");
            state.write_chunk = 1;
            state.require_frame_finished = true;
        });
        let settings = TestSettings::new();
        settings.change(|settings| settings.timeouts.close = E);
        run_session_controlled(
            &mut viewer,
            &mut other,
            Vec::new(),
            b"aderNEW".to_vec(),
            &mut session,
            clock.as_ref(),
            receiver,
            [8; 2],
            settings.as_ref(),
            command,
            |_, _| {},
        )
        .await
        .unwrap_or_else(|e| panic!("admin frame cleanup: {e}"));
        assert_eq!(viewer_control.output(), b"header");
        assert_eq!(session.bytes(Direction::TargetToViewer), 6);
        assert_eq!(session.bytes(Direction::ViewerToTarget), 0);
        assert_eq!(session.close_cause(), Some(CloseCause::AdminTerminated));
        assert_eq!(clock.monotonic(), Duration::ZERO);
        assert_eq!(
            viewer_control.actions(),
            vec![RelayCloseAction::AdminTerminated]
        );
        assert!(
            other_control
                .actions()
                .contains(&RelayCloseAction::AdminTerminated)
        );
        assert_eq!(
            viewer_control
                .0
                .lock()
                .unwrap_or_else(|e| panic!("{e}"))
                .read_calls,
            0
        );
        assert_eq!(
            other_control
                .0
                .lock()
                .unwrap_or_else(|e| panic!("{e}"))
                .read_calls,
            0
        );
    }

    /// spec: websocket.md:WS-028 partial
    #[tokio::test(start_paused = true)]
    async fn server_controls_flush_with_full_payload_buffers_on_tcp_and_ws_peers() {
        for peer_transport in [Transport::Tcp, Transport::Wss] {
            for frame_in_flight in [false, true] {
                let clock = TestClock::new();
                let mut session = model(Mode::Mode2);
                let (_lifecycle, receiver) = watch::channel(RelayLifecycle::Running);
                let (mut viewer, viewer_control) = stream(&[], false);
                let (mut server, server_control) = stream(b"full", false);
                viewer_control.change(|state| {
                    state.transport = peer_transport;
                    state.write_blocked = true;
                });
                server_control.change(|state| {
                    state.transport = Transport::Ws;
                    state.pending_pong.extend(b"\x8a\x01p");
                    state.pending_write = if frame_in_flight { 4 } else { 0 };
                    state.control_allowance = if frame_in_flight { 3 } else { 1 };
                });
                {
                    let mut relay = pin!(run_session(
                        &mut viewer,
                        &mut server,
                        if frame_in_flight {
                            b"data".to_vec()
                        } else {
                            Vec::new()
                        },
                        Vec::new(),
                        &mut session,
                        clock.as_ref(),
                        receiver,
                        4,
                        E,
                        |_| {},
                    ));
                    pump(&mut relay).await;
                    {
                        let state = server_control.0.lock().unwrap();
                        assert_eq!(state.read_total, 4);
                        assert_eq!(state.read_calls, 1, "maintenance must not bypass B");
                        assert_eq!(state.pending_write, 0);
                    }
                    server_control.change(|state| state.control_allowance = 3);
                    pump(&mut relay).await;
                    assert_eq!(
                        server_control.0.lock().unwrap().control_output,
                        b"\x8a\x01p"
                    );
                    assert!(viewer_control.output().is_empty());
                }
                assert_eq!(session.close_cause(), None);
                assert_eq!(session.bytes(Direction::TargetToViewer), 0);
                assert_eq!(
                    session.bytes(Direction::ViewerToTarget),
                    if frame_in_flight { 4 } else { 0 }
                );
            }
        }
    }

    /// spec: websocket.md:WS-028 partial
    #[tokio::test(start_paused = true)]
    async fn server_liveness_and_control_failures_preserve_server_side() {
        for peer_transport in [Transport::Tcp, Transport::Wss] {
            for dead in [false, true] {
                let clock = TestClock::new();
                let mut session = model(Mode::Mode2);
                let (_lifecycle, receiver) = watch::channel(RelayLifecycle::Running);
                let (mut viewer, viewer_control) = stream(b"full", false);
                let (mut server, server_control) = stream(b"full", false);
                viewer_control.change(|state| {
                    state.transport = peer_transport;
                    state.write_blocked = true;
                });
                server_control.change(|state| {
                    state.transport = Transport::Ws;
                    state.write_blocked = true;
                });
                let result;
                {
                    let mut relay = pin!(run_session(
                        &mut viewer,
                        &mut server,
                        Vec::new(),
                        Vec::new(),
                        &mut session,
                        clock.as_ref(),
                        receiver,
                        4,
                        E,
                        |_| {},
                    ));
                    pump(&mut relay).await;
                    server_control.change(|state| {
                        if dead {
                            state.maintenance_error = Some(RelayStreamError::DeadConnection);
                        } else {
                            state.flush_error = Some(ConnectionErrorKind::WsProtocolError);
                        }
                    });
                    result = relay.await.expect("Server control failure cleanup");
                }
                assert_eq!(session.close_side(), Some(Side::Server));
                assert_eq!(
                    session.close_cause(),
                    Some(if dead {
                        CloseCause::DeadConnection
                    } else {
                        CloseCause::ConnectionError
                    })
                );
                assert_eq!(
                    result,
                    if dead {
                        None
                    } else {
                        Some(ConnectionErrorKind::WsProtocolError)
                    }
                );
                assert_eq!(session.bytes(Direction::ViewerToTarget), 0);
                assert_eq!(session.bytes(Direction::TargetToViewer), 0);
                assert!(!viewer_control.actions().is_empty());
                assert!(!server_control.actions().is_empty());
            }
        }
    }

    /// spec: websocket.md:WS-028 partial
    /// spec: websocket.md:WS-017 partial
    #[tokio::test(start_paused = true)]
    async fn server_websocket_close_waits_share_one_absolute_deadline_in_both_orders() {
        for peer_transport in [Transport::Tcp, Transport::Wss] {
            for server_first in [false, true] {
                let clock = TestClock::new();
                let mut session = model(Mode::Mode2);
                let (_lifecycle, receiver) = watch::channel(RelayLifecycle::Running);
                let (mut viewer, viewer_control) = stream(&[], !server_first);
                let (mut server, server_control) = stream(&[], server_first);
                viewer_control.change(|state| state.transport = peer_transport);
                server_control.change(|state| state.transport = Transport::Ws);
                run_session(
                    &mut viewer,
                    &mut server,
                    Vec::new(),
                    Vec::new(),
                    &mut session,
                    clock.as_ref(),
                    receiver,
                    32,
                    E,
                    |_| {},
                )
                .await
                .expect("unanswered Server-side Close expires at E");
                assert_eq!(clock.monotonic(), E);
                assert_eq!(session.close_cause(), Some(CloseCause::CloseTimeout));
                assert_eq!(
                    session.close_side(),
                    Some(if server_first {
                        Side::Server
                    } else {
                        Side::Viewer
                    })
                );
                assert_eq!(session.bytes(Direction::ViewerToTarget), 0);
                assert_eq!(session.bytes(Direction::TargetToViewer), 0);
            }
        }
    }

    /// spec: websocket.md:WS-028 partial
    /// spec: websocket.md:WS-016 partial
    /// spec: websocket.md:WS-017 partial
    #[tokio::test(start_paused = true)]
    async fn either_websocket_role_closes_after_only_its_started_frame() {
        for close_index in [0, 1] {
            for peer_transport in [Transport::Tcp, Transport::Wss] {
                for pending_write in [0, 5] {
                    let clock = TestClock::new();
                    let mut session = model(Mode::Mode2);
                    let (_lifecycle, receiver) = watch::channel(RelayLifecycle::Running);
                    let (mut viewer, viewer_control) = stream(&[], close_index == 0);
                    let (mut server, server_control) = stream(&[], close_index == 1);
                    let controls = [&viewer_control, &server_control];
                    controls[close_index].change(|state| {
                        state.transport = Transport::Ws;
                        state.pending_write = pending_write;
                        state.write_chunk = 2;
                        state.write_blocked = true;
                    });
                    controls[1 - close_index].change(|state| {
                        state.transport = peer_transport;
                        state.input.extend(b"discard-after-close");
                    });
                    {
                        let mut relay = pin!(run_session(
                            &mut viewer,
                            &mut server,
                            if close_index == 1 {
                                b"frame-extra".to_vec()
                            } else {
                                Vec::new()
                            },
                            if close_index == 0 {
                                b"frame-extra".to_vec()
                            } else {
                                Vec::new()
                            },
                            &mut session,
                            clock.as_ref(),
                            receiver,
                            32,
                            E,
                            |_| {},
                        ));
                        pump(&mut relay).await;
                        controls[close_index].change(|state| state.write_blocked = false);
                        pump(&mut relay).await;
                        assert_eq!(controls[close_index].output(), &b"frame"[..pending_write]);
                        assert!(controls[1 - close_index].output().is_empty());
                        assert_eq!(controls[close_index].actions(), [RelayCloseAction::PeerEof]);
                        assert_eq!(
                            controls[1 - close_index].actions(),
                            [RelayCloseAction::WriteHalf]
                        );
                        controls[1 - close_index].feed(&[], true);
                        relay.await.expect("other endpoint EOF after Close");
                    }
                    let direction = if close_index == 0 {
                        Direction::TargetToViewer
                    } else {
                        Direction::ViewerToTarget
                    };
                    assert_eq!(session.bytes(direction), pending_write as u64);
                    assert_eq!(finish(&mut session, &clock), CloseCause::PeerEof);
                }
            }
        }
    }

    /// spec: websocket.md:WS-028 partial
    /// spec: management.md:MGT-009 partial
    #[tokio::test(start_paused = true)]
    async fn admin_finishes_started_frames_on_both_websocket_roles_without_new_reads() {
        let clock = TestClock::new();
        let mut session = model(Mode::Mode2);
        let (_lifecycle, receiver) = watch::channel(RelayLifecycle::Running);
        let (_command, command) = watch::channel(Some((Duration::ZERO, E)));
        let (mut viewer, viewer_control) = stream(b"unread-viewer", false);
        let (mut server, server_control) = stream(b"unread-server", false);
        for control in [&viewer_control, &server_control] {
            control.change(|state| {
                state.transport = Transport::Ws;
                state.pending_write = 4;
                state.write_chunk = 1;
                state.require_frame_finished = true;
            });
        }
        let settings = TestSettings::new();
        settings.change(|settings| settings.timeouts.close = E);
        run_session_controlled(
            &mut viewer,
            &mut server,
            b"sendNEW".to_vec(),
            b"backNEW".to_vec(),
            &mut session,
            clock.as_ref(),
            receiver,
            [8; 2],
            settings.as_ref(),
            command,
            |_, _| {},
        )
        .await
        .expect("both started frames finish before admin Close");
        assert_eq!(viewer_control.output(), b"back");
        assert_eq!(server_control.output(), b"send");
        assert_eq!(session.bytes(Direction::ViewerToTarget), 4);
        assert_eq!(session.bytes(Direction::TargetToViewer), 4);
        assert_eq!(session.close_cause(), Some(CloseCause::AdminTerminated));
        assert_eq!(clock.monotonic(), Duration::ZERO);
        for control in [&viewer_control, &server_control] {
            assert_eq!(control.actions(), [RelayCloseAction::AdminTerminated]);
            assert_eq!(control.0.lock().unwrap().read_calls, 0);
        }
    }
}
