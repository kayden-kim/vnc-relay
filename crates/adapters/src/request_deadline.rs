// Limits how long a management connection may stay silent before its first
// request head is complete, and how long it may stay idle afterwards.
// Design: docs/architectures/management-listener.md (§4, §6, §7).

use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::time::{Instant, Sleep, sleep_until};

/// The bytes `\r\n\r\n` as a big-endian word. It ends an HTTP/1.1 request head.
const HEAD_END: u32 = u32::from_be_bytes(*b"\r\n\r\n");

/// Adds the U deadline captured by admission, without rereading runtime settings.
pub(crate) struct DeadlineListener<L> {
    inner: L,
}

impl<L> DeadlineListener<L> {
    pub(crate) fn new(inner: L) -> Self {
        Self { inner }
    }
}

impl<L> axum::serve::Listener for DeadlineListener<L>
where
    L: axum::serve::Listener<Addr = crate::admission::AdmittedAddr>,
{
    type Io = Deadlined<L::Io>;
    type Addr = L::Addr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        let (io, addr) = self.inner.accept().await;
        (Deadlined::at(io, addr.upgrade, addr.at), addr)
    }

    fn local_addr(&self) -> io::Result<Self::Addr> {
        self.inner.local_addr()
    }
}

/// Connection-local exception to U for an accepted termination wait (Q92).
/// Only the first actual response write resumes the idle timer.
#[derive(Debug, Clone)]
pub(crate) struct IdleControl(Arc<AtomicBool>);

impl IdleControl {
    pub(crate) fn pause_for_termination(&self) {
        self.0.store(true, Ordering::Release);
    }
}

/// One accepted connection with its timer. Before the first request head is
/// complete, the timer is absolute: it ends `limit` after accept, so a TLS
/// handshake or a head that never finishes is cut off. After that, every
/// transfer restarts the timer, so only idle time counts.
pub(crate) struct Deadlined<IO> {
    io: IO,
    limit: Duration,
    timer: Pin<Box<Sleep>>,
    /// True once `\r\n\r\n` has been read.
    head_done: bool,
    /// The last four bytes read while the head is still open.
    window: u32,
    idle: IdleControl,
}

impl<IO> Deadlined<IO> {
    #[cfg(test)]
    fn new(io: IO, limit: Duration) -> Self {
        Self::at(io, limit, Instant::now())
    }

    fn at(io: IO, limit: Duration, at: Instant) -> Self {
        Self {
            io,
            limit,
            timer: Box::pin(sleep_until(at + limit)),
            head_done: false,
            window: 0,
            idle: IdleControl(Arc::new(AtomicBool::new(false))),
        }
    }

    pub(crate) fn idle_control(&self) -> IdleControl {
        self.idle.clone()
    }

    fn resume_for_response(&mut self) {
        if self.idle.0.swap(false, Ordering::AcqRel) {
            self.restart();
        }
    }

    fn restart(&mut self) {
        self.timer.as_mut().reset(Instant::now() + self.limit);
    }

    /// Scans bytes that the client sent. Only the first request head matters.
    fn note_read(&mut self, bytes: &[u8]) {
        if self.head_done {
            self.restart();
            return;
        }
        for &byte in bytes {
            self.window = (self.window << 8) | u32::from(byte);
            if self.window == HEAD_END {
                self.head_done = true;
                self.restart();
                return;
            }
        }
    }

    fn expired(&mut self, cx: &mut Context<'_>) -> bool {
        !self.idle.0.load(Ordering::Acquire) && self.timer.as_mut().poll(cx).is_ready()
    }
}

fn timed_out() -> io::Error {
    io::ErrorKind::TimedOut.into()
}

impl<IO> AsyncRead for Deadlined<IO>
where
    IO: AsyncRead + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.expired(cx) {
            return Poll::Ready(Err(timed_out()));
        }
        let before = buf.filled().len();
        match Pin::new(&mut self.io).poll_read(cx, buf) {
            Poll::Ready(Ok(())) => {
                let read = &buf.filled()[before..];
                if !read.is_empty() {
                    self.note_read(read);
                }
                Poll::Ready(Ok(()))
            }
            other => other,
        }
    }
}

impl<IO> AsyncWrite for Deadlined<IO>
where
    IO: AsyncWrite + Unpin,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.resume_for_response();
        if self.expired(cx) {
            return Poll::Ready(Err(timed_out()));
        }
        match Pin::new(&mut self.io).poll_write(cx, buf) {
            Poll::Ready(Ok(written)) => {
                if self.head_done && written > 0 {
                    self.restart();
                }
                Poll::Ready(Ok(written))
            }
            other => other,
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.expired(cx) {
            return Poll::Ready(Err(timed_out()));
        }
        Pin::new(&mut self.io).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.io).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use std::task::Waker;

    use tokio::io::AsyncWriteExt;
    use tokio::time::advance;

    /// spec: operations.md:OPS-050 partial
    #[tokio::test(start_paused = true)]
    async fn delayed_wrapping_keeps_the_original_accept_deadline_and_u() {
        let accepted_at = Instant::now();
        let old_u = Duration::from_secs(3);
        advance(Duration::from_secs(2)).await;
        let mut old = Deadlined::at(tokio::io::sink(), old_u, accepted_at);
        let mut new = Deadlined::new(tokio::io::sink(), Duration::from_secs(7));
        let mut cx = Context::from_waker(Waker::noop());
        assert!(!old.expired(&mut cx));
        advance(Duration::from_secs(1)).await;
        assert!(old.expired(&mut cx));
        assert!(!new.expired(&mut cx));
        advance(Duration::from_secs(6)).await;
        assert!(new.expired(&mut cx));
    }
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn accepted_wait_pauses_only_its_connection_until_response_writes() {
        let limit = Duration::from_secs(1);
        let mut waiting = Deadlined::new(tokio::io::sink(), limit);
        let mut ordinary = Deadlined::new(tokio::io::sink(), limit);
        waiting.note_read(b"POST / HTTP/1.1\r\n\r\n");
        ordinary.note_read(b"GET / HTTP/1.1\r\n\r\n");
        let control = waiting.idle_control();
        control.pause_for_termination();
        advance(limit * 3).await;
        let mut cx = Context::from_waker(Waker::noop());
        assert!(!waiting.expired(&mut cx));
        assert!(ordinary.expired(&mut cx));
        // Completing a handler does not resume U. Only writing its response does.
        assert!(control.0.load(Ordering::Acquire));
        waiting
            .write_all(b"HTTP/1.1 200 OK\r\n\r\n")
            .await
            .unwrap_or_else(|error| panic!("response writes with a fresh U: {error}"));
        assert!(!control.0.load(Ordering::Acquire));
        advance(limit - Duration::from_millis(1)).await;
        assert!(!waiting.expired(&mut cx));
        advance(Duration::from_millis(1)).await;
        assert!(waiting.expired(&mut cx));
    }

    #[tokio::test(start_paused = true)]
    async fn body_validation_and_unaccepted_requests_keep_the_usual_u() {
        let limit = Duration::from_secs(1);
        let mut io = Deadlined::new(tokio::io::sink(), limit);
        io.note_read(b"POST / HTTP/1.1\r\nContent-Length: 1\r\n\r\n");
        advance(limit).await;
        let error = io
            .write_all(b"response")
            .await
            .err()
            .unwrap_or_else(|| panic!("ordinary U is not suspended"));
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    }
}
