//! WebSocket stream adapter implementing `RelayStream` (docs/architectures/websocket-stream.md).
//!
//! Provides `WsRelayStream<S>` which wraps an underlying I/O stream (plain TCP
//! or TLS over TCP) and handles streaming RFC 6455 frames without whole-frame
//! or whole-message buffering.

pub mod frame;
mod keepalive;
#[cfg(test)]
mod tests;

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use vnc_relay_app::events::ConnectionErrorKind;
use vnc_relay_app::ports::{RelayCloseAction, RelayStream, RelayStreamError};
use vnc_relay_app::settings::KeepaliveSettings;
use vnc_relay_domain::session::ViewerTransport;

use self::frame::{
    ControlOp, DecodeState, FrameDecoder, MAX_NON_DATA_FRAMES_PER_POLL, OutboundDataFrame,
    PendingClose, PendingPong, encode_close, encode_data_header,
};
use self::keepalive::Keepalive;

/// An adapter wrapping a stream with streaming RFC 6455 WebSocket framing,
/// implementing the common `RelayStream` port.
pub struct WsRelayStream<S> {
    io: S,
    transport: ViewerTransport,
    max_message_size: usize,
    budget_limit: usize,
    decoder: FrameDecoder,
    upgrade_suffix: Option<Vec<u8>>,
    suffix_pos: usize,
    outbound_frame: Option<OutboundDataFrame>,
    active_pong: Option<PendingPong>,
    pong_flush_pending: bool,
    pending_close: Option<PendingClose>,
    close_sent: bool,
    terminal_close: bool,
    raw_terminal_disposal: bool,
    transport_eof: bool,
    keepalive: Keepalive,
}

impl<S> WsRelayStream<S> {
    /// Creates a new `WsRelayStream`.
    ///
    /// - `io`: the underlying transport (e.g. `Wire`).
    /// - `upgrade_suffix`: raw bytes following the HTTP upgrade head (at most 1023 bytes).
    /// - `max_message_size`: M (maximum logical message length).
    /// - `budget_limit`: B (maximum holding budget per direction).
    /// - `transport`: `ViewerTransport::Ws` or `ViewerTransport::Wss`.
    /// - `keepalive`: validated Kw durations (each 5..=300 seconds, config-v2).
    pub fn new(
        io: S,
        upgrade_suffix: Vec<u8>,
        max_message_size: usize,
        budget_limit: usize,
        transport: ViewerTransport,
        keepalive: KeepaliveSettings,
    ) -> Self {
        let suffix = if upgrade_suffix.is_empty() {
            None
        } else {
            Some(upgrade_suffix)
        };
        Self {
            io,
            transport,
            max_message_size,
            budget_limit,
            decoder: FrameDecoder::new(max_message_size),
            upgrade_suffix: suffix,
            suffix_pos: 0,
            outbound_frame: None,
            active_pong: None,
            pong_flush_pending: false,
            pending_close: None,
            close_sent: false,
            terminal_close: false,
            raw_terminal_disposal: false,
            transport_eof: false,
            keepalive: Keepalive::new(keepalive),
        }
    }

    /// The viewer transport kind (Ws or Wss).
    pub fn transport(&self) -> ViewerTransport {
        self.transport
    }

    /// Returns the remaining payload length of an outbound frame whose header
    /// is already written (Q83 / websocket-stream §5.1); 0 if no frame is in flight
    /// or the header has not been written yet.
    pub fn pending_write_len(&self) -> usize {
        match &self.outbound_frame {
            Some(f) if f.header_written > 0 => f.remaining_payload_len(),
            _ => 0,
        }
    }

    /// Total undelivered bytes currently held inside the adapter in the receive
    /// direction (unconsumed upgrade suffix + decoder buffer bytes). Always ≤ B.
    pub(crate) fn holding_rx_bytes(&self) -> usize {
        let suffix_held = self
            .upgrade_suffix
            .as_ref()
            .map_or(0, |s| s.len().saturating_sub(self.suffix_pos));
        suffix_held + self.decoder.holding_bytes()
    }

    /// Total undelivered bytes currently held inside the adapter in the transmit
    /// direction (unwritten frame header + pending control frames). Always ≤ B.
    pub(crate) fn holding_tx_bytes(&self) -> usize {
        let header_held = self.outbound_frame.as_ref().map_or(0, |f| {
            (f.header_len as usize).saturating_sub(f.header_written as usize)
        });
        let pong_held = self
            .decoder
            .pending_pong
            .as_ref()
            .map_or(0, |p| (p.len as usize).saturating_sub(p.written as usize));
        let active_pong_held = self
            .active_pong
            .as_ref()
            .map_or(0, |p| p.remaining_bytes().len());
        let close_held = self
            .pending_close
            .as_ref()
            .map_or(0, |c| (c.len as usize).saturating_sub(c.written as usize));
        let ping_held = self
            .keepalive
            .pending_ping
            .map_or(0, |written| 2 - usize::from(written));
        header_held + pong_held + active_pong_held + close_held + ping_held
    }

    /// Reads up to `buf.len()` bytes from `upgrade_suffix` first; once exhausted,
    /// reads directly from `io`.
    fn poll_read_source_raw(
        upgrade_suffix: &mut Option<Vec<u8>>,
        suffix_pos: &mut usize,
        io: &mut S,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>>
    where
        S: AsyncRead + Unpin,
    {
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        if let Some(suffix) = upgrade_suffix {
            let remaining = suffix.len() - *suffix_pos;
            if remaining > 0 {
                let to_copy = remaining.min(buf.len());
                buf[..to_copy].copy_from_slice(&suffix[*suffix_pos..*suffix_pos + to_copy]);
                *suffix_pos += to_copy;
                if *suffix_pos == suffix.len() {
                    *upgrade_suffix = None;
                    *suffix_pos = 0;
                }
                return Poll::Ready(Ok(to_copy));
            }
            *upgrade_suffix = None;
            *suffix_pos = 0;
        }

        let mut read_buf = ReadBuf::new(buf);
        match Pin::new(io).poll_read(cx, &mut read_buf) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(read_buf.filled().len())),
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }

    /// Drives Pong output without crossing an in-flight frame boundary.
    /// The decoder owns only a replaceable, unsent Pong; active bytes stay here.
    fn poll_flush_pending_pong(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>>
    where
        S: AsyncWrite + Unpin,
    {
        if self.outbound_frame.is_some()
            || self
                .keepalive
                .pending_ping
                .is_some_and(|written| written > 0)
        {
            return Poll::Ready(Ok(()));
        }
        loop {
            if self.active_pong.is_none() && self.pending_close.is_none() && !self.close_sent {
                self.active_pong = self.decoder.pending_pong.take();
            }
            let Some(pong) = self.active_pong.as_mut() else {
                break;
            };
            while !pong.is_finished() {
                match Pin::new(&mut self.io).poll_write(cx, pong.remaining_bytes()) {
                    Poll::Ready(Ok(0)) => {
                        return Poll::Ready(Err(io::Error::from(io::ErrorKind::WriteZero)));
                    }
                    Poll::Ready(Ok(n)) => pong.advance(n),
                    Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                    Poll::Pending => {
                        // No prefix is on the wire: this Pong can still coalesce.
                        if pong.written == 0 {
                            self.decoder.pending_pong = self.active_pong.take();
                        }
                        return Poll::Pending;
                    }
                }
            }
            self.active_pong = None;
            self.pong_flush_pending = true;
        }
        if self.pong_flush_pending {
            match Pin::new(&mut self.io).poll_flush(cx) {
                Poll::Ready(Ok(())) => self.pong_flush_pending = false,
                result => return result,
            }
        }
        Poll::Ready(Ok(()))
    }

    /// Advances Kw without reading ahead of the app's bounded payload buffer.
    fn poll_keepalive(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>>
    where
        S: AsyncWrite + Unpin,
    {
        if self.keepalive.expired {
            return Poll::Ready(Err(RelayStreamError::DeadConnection.into_io_error()));
        }
        let closing = self.close_sent || self.pending_close.is_some() || self.decoder.peer_closed;
        if closing
            && !self
                .keepalive
                .pending_ping
                .is_some_and(|written| written > 0)
        {
            return Poll::Pending;
        }
        if !closing {
            self.keepalive.poll(cx);
            if self.keepalive.expired {
                return Poll::Ready(Err(RelayStreamError::DeadConnection.into_io_error()));
            }
        }
        if self.outbound_frame.is_some() {
            return Poll::Pending;
        }
        match self.poll_flush_pending_pong(cx) {
            Poll::Ready(Ok(())) => {}
            result => return result,
        }
        if let Some(mut written) = self.keepalive.pending_ping {
            while written < 2 {
                match Pin::new(&mut self.io).poll_write(cx, &[0x89, 0][usize::from(written)..]) {
                    Poll::Ready(Ok(0)) => return Poll::Ready(Err(io::ErrorKind::WriteZero.into())),
                    Poll::Ready(Ok(count)) => {
                        written += count as u8;
                        self.keepalive.pending_ping = Some(written);
                    }
                    Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                    Poll::Pending => return Poll::Pending,
                }
            }
            match Pin::new(&mut self.io).poll_flush(cx) {
                Poll::Ready(Ok(())) => self.keepalive.sent(tokio::time::Instant::now()),
                result => return result,
            }
            if !closing {
                self.keepalive.poll(cx);
            }
        }
        Poll::Pending
    }

    /// Flushes any pending Close control frame to the transport.
    fn poll_flush_pending_close(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>>
    where
        S: AsyncWrite + Unpin,
    {
        if self.outbound_frame.is_some() {
            return Poll::Ready(Ok(()));
        }
        // An already-started Pong must finish before the Close header.
        match self.poll_flush_pending_pong(cx) {
            Poll::Ready(Ok(())) => {}
            result => return result,
        }
        self.decoder.pending_pong = None;
        if let Some(mut close) = self.pending_close {
            while !close.is_finished() {
                let rem = close.remaining_bytes();
                match Pin::new(&mut self.io).poll_write(cx, rem) {
                    Poll::Ready(Ok(0)) => {
                        return Poll::Ready(Err(io::Error::from(io::ErrorKind::WriteZero)));
                    }
                    Poll::Ready(Ok(n)) => {
                        close.advance(n);
                    }
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                    Poll::Pending => {
                        self.pending_close = Some(close);
                        return Poll::Pending;
                    }
                }
            }
            self.pending_close = None;
            self.close_sent = true;
        }
        Poll::Ready(Ok(()))
    }
    /// Finishes valid Close exchanges; rejected frames use bounded raw disposal.
    /// The app owns the absolute E/G/Force bounds in either case.
    fn poll_terminal_disposal(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        match Pin::new(&mut self.io).poll_flush(cx) {
            Poll::Ready(Ok(())) => {}
            result => return result,
        }
        if !self.raw_terminal_disposal {
            let mut discard = [0; 1024];
            for _ in 0..MAX_NON_DATA_FRAMES_PER_POLL {
                if self.decoder.peer_closed || self.transport_eof {
                    return Pin::new(&mut self.io).poll_shutdown(cx);
                }
                let mut buffer = ReadBuf::new(&mut discard);
                match Pin::new(&mut *self).poll_read(cx, &mut buffer) {
                    Poll::Ready(Ok(())) => {}
                    Poll::Ready(Err(_)) if self.transport_eof => {
                        return Pin::new(&mut self.io).poll_shutdown(cx);
                    }
                    Poll::Ready(Err(error))
                        if matches!(
                            RelayStreamError::classify(&error),
                            ConnectionErrorKind::WsProtocolError
                                | ConnectionErrorKind::WsTextMessage
                                | ConnectionErrorKind::WsMessageTooLarge
                        ) =>
                    {
                        self.raw_terminal_disposal = true;
                        return self.poll_terminal_disposal(cx);
                    }
                    result => return result,
                }
            }
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        match Pin::new(&mut self.io).poll_shutdown(cx) {
            Poll::Ready(Ok(())) => {}
            result => return result,
        }
        let mut discard = [0; 1024];
        for _ in 0..MAX_NON_DATA_FRAMES_PER_POLL {
            let mut buffer = ReadBuf::new(&mut discard);
            match Pin::new(&mut self.io).poll_read(cx, &mut buffer) {
                Poll::Ready(Ok(())) if buffer.filled().is_empty() => return Poll::Ready(Ok(())),
                Poll::Ready(Ok(())) => {}
                result => return result,
            }
        }
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin + Send> AsyncRead for WsRelayStream<S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let initial_filled = buf.filled().len();
        if let Poll::Ready(Err(error)) = this.poll_keepalive(cx) {
            return Poll::Ready(Err(error));
        }
        if buf.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }

        let mut non_data_frames = 0;

        loop {
            // Assert holding bounds (Q78, websocket-stream.md §3.2)
            debug_assert!(
                this.holding_rx_bytes() <= this.budget_limit,
                "rx holding {} exceeded budget B {}",
                this.holding_rx_bytes(),
                this.budget_limit
            );
            // A write wake must retry control progress even on an idle read.
            // Pending does not prevent independent inbound payload progress.
            if let Poll::Ready(Err(error)) = this.poll_flush_pending_pong(cx) {
                return Poll::Ready(Err(error));
            }

            // 1. If peer already closed and no data remains, return EOF
            if this.decoder.peer_closed {
                match this.decoder.state {
                    DecodeState::ReadingPayload { .. } => {}
                    _ => {
                        return Poll::Ready(Ok(()));
                    }
                }
            }

            // 2. State-driven processing
            match this.decoder.state {
                DecodeState::PeerClosed => {
                    return Poll::Ready(Ok(()));
                }
                DecodeState::ReadingPayload {
                    frame_payload_len,
                    frame_payload_read,
                    is_final,
                    ..
                } => {
                    let remaining_in_frame = (frame_payload_len - frame_payload_read) as usize;
                    if remaining_in_frame == 0 {
                        if is_final {
                            this.decoder.in_fragmented_message = false;
                            this.decoder.current_message_len = 0;
                        }
                        this.decoder.state = DecodeState::AwaitingHeader;
                        continue;
                    }

                    let want = buf.remaining().min(remaining_in_frame);
                    if want == 0 {
                        return Poll::Ready(Ok(()));
                    }

                    // First try leftover
                    let unfilled = buf.initialize_unfilled_to(want);
                    let n = this
                        .decoder
                        .read_payload_from_leftover(&mut unfilled[..want]);
                    if n > 0 {
                        buf.advance(n);
                        return Poll::Ready(Ok(()));
                    }

                    // Otherwise read from source
                    let read_res = Self::poll_read_source_raw(
                        &mut this.upgrade_suffix,
                        &mut this.suffix_pos,
                        &mut this.io,
                        cx,
                        &mut unfilled[..want],
                    );
                    match read_res {
                        Poll::Ready(Ok(0)) => {
                            this.transport_eof = true;
                            // Socket EOF mid-payload without a Close frame is an IoError
                            return Poll::Ready(Err(RelayStreamError::new(
                                ConnectionErrorKind::IoError,
                            )
                            .into_io_error()));
                        }
                        Poll::Ready(Ok(n)) => {
                            this.decoder.advance_payload(&mut unfilled[..n]);
                            buf.advance(n);
                            return Poll::Ready(Ok(()));
                        }
                        Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                        Poll::Pending => {
                            if buf.filled().len() > initial_filled {
                                return Poll::Ready(Ok(()));
                            }
                            return Poll::Pending;
                        }
                    }
                }
                DecodeState::ReadingControl { .. } => {
                    while !this.decoder.is_control_ready() {
                        let n = this.decoder.drain_leftover_to_control();
                        if n > 0 {
                            continue;
                        }

                        let needed = match this.decoder.state {
                            DecodeState::ReadingControl {
                                payload_len,
                                payload_read,
                                ..
                            } => usize::from(payload_len - payload_read),
                            _ => 0,
                        };
                        let mut temp = [0u8; 125];
                        let to_read = needed;
                        let read_res = Self::poll_read_source_raw(
                            &mut this.upgrade_suffix,
                            &mut this.suffix_pos,
                            &mut this.io,
                            cx,
                            &mut temp[..to_read],
                        );
                        match read_res {
                            Poll::Ready(Ok(0)) => {
                                this.transport_eof = true;
                                return Poll::Ready(Err(RelayStreamError::new(
                                    ConnectionErrorKind::IoError,
                                )
                                .into_io_error()));
                            }
                            Poll::Ready(Ok(n)) => {
                                this.decoder.append_control_bytes(&temp[..n]);
                            }
                            Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                            Poll::Pending => {
                                if buf.filled().len() > initial_filled {
                                    return Poll::Ready(Ok(()));
                                }
                                return Poll::Pending;
                            }
                        }
                    }

                    // Control frame is complete
                    let op = this.decoder.finish_control_frame()?;
                    if op == Some(ControlOp::Pong) {
                        this.keepalive.pong();
                    }

                    if op == Some(ControlOp::Close) {
                        return Poll::Ready(Ok(()));
                    }

                    non_data_frames += 1;
                    if non_data_frames >= MAX_NON_DATA_FRAMES_PER_POLL {
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }
                }
                DecodeState::AwaitingHeader => {
                    loop {
                        let n = this.decoder.drain_leftover_to_header();
                        if n > 0 && this.decoder.try_parse_header()? {
                            break;
                        }

                        let mut temp = [0u8; 14];
                        let to_read = 14 - this.decoder.header_len;
                        let read_res = Self::poll_read_source_raw(
                            &mut this.upgrade_suffix,
                            &mut this.suffix_pos,
                            &mut this.io,
                            cx,
                            &mut temp[..to_read],
                        );
                        match read_res {
                            Poll::Ready(Ok(0)) => {
                                this.transport_eof = true;
                                if this.decoder.peer_closed {
                                    return Poll::Ready(Ok(()));
                                }
                                return Poll::Ready(Err(RelayStreamError::new(
                                    ConnectionErrorKind::IoError,
                                )
                                .into_io_error()));
                            }
                            Poll::Ready(Ok(n)) => {
                                this.decoder.append_header_bytes(&temp[..n]);
                                if this.decoder.try_parse_header()? {
                                    break;
                                }
                            }
                            Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                            Poll::Pending => {
                                if buf.filled().len() > initial_filled {
                                    return Poll::Ready(Ok(()));
                                }
                                return Poll::Pending;
                            }
                        }
                    }

                    if matches!(this.decoder.state, DecodeState::AwaitingHeader) {
                        non_data_frames += 1;
                        if non_data_frames >= MAX_NON_DATA_FRAMES_PER_POLL {
                            cx.waker().wake_by_ref();
                            return Poll::Pending;
                        }
                    }
                }
            }
        }
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin + Send> AsyncWrite for WsRelayStream<S> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        if let Poll::Ready(Err(error)) = this.poll_keepalive(cx) {
            return Poll::Ready(Err(error));
        }
        if this
            .keepalive
            .pending_ping
            .is_some_and(|written| written > 0)
        {
            return Poll::Pending;
        }

        // Assert tx holding bounds
        debug_assert!(
            this.holding_tx_bytes() <= this.budget_limit,
            "tx holding {} exceeded budget B {}",
            this.holding_tx_bytes(),
            this.budget_limit
        );

        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        if this.close_sent || this.pending_close.is_some() {
            return Poll::Ready(Err(io::Error::from(io::ErrorKind::BrokenPipe)));
        }

        // 1. If no outbound frame is currently in progress:
        if this.outbound_frame.is_none() {
            match this.poll_flush_pending_pong(cx) {
                Poll::Ready(Ok(())) => {}
                Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                Poll::Pending => return Poll::Pending,
            }

            // Create new outbound binary frame
            let payload_len = buf.len().min(this.max_message_size);
            let (header_buf, header_len) = encode_data_header(payload_len);
            this.outbound_frame = Some(OutboundDataFrame {
                header_buf,
                header_len,
                header_written: 0,
                payload_len,
                payload_written: 0,
            });
        }

        // 2. Drive outbound frame
        let mut frame = match this.outbound_frame.take() {
            Some(f) => f,
            None => return Poll::Ready(Ok(0)),
        };

        // 2a. Finish header if needed
        while !frame.is_header_finished() {
            let rem = frame.remaining_header();
            match Pin::new(&mut this.io).poll_write(cx, rem) {
                Poll::Ready(Ok(0)) => {
                    this.outbound_frame = Some(frame);
                    return Poll::Ready(Err(io::Error::from(io::ErrorKind::WriteZero)));
                }
                Poll::Ready(Ok(n)) => {
                    frame.advance_header(n);
                }
                Poll::Ready(Err(e)) => {
                    this.outbound_frame = Some(frame);
                    return Poll::Ready(Err(e));
                }
                Poll::Pending => {
                    this.outbound_frame = Some(frame);
                    return Poll::Pending;
                }
            }
        }

        // 2b. Write payload
        let remaining_payload = frame.remaining_payload_len();
        let to_write = buf.len().min(remaining_payload);
        if to_write == 0 {
            if frame.is_finished() {
                this.outbound_frame = None;
            } else {
                this.outbound_frame = Some(frame);
            }
            return Poll::Ready(Ok(0));
        }

        match Pin::new(&mut this.io).poll_write(cx, &buf[..to_write]) {
            Poll::Ready(Ok(0)) => {
                this.outbound_frame = Some(frame);
                Poll::Ready(Err(io::Error::from(io::ErrorKind::WriteZero)))
            }
            Poll::Ready(Ok(n)) => {
                frame.advance_payload(n);
                if frame.is_finished() {
                    this.outbound_frame = None;
                } else {
                    this.outbound_frame = Some(frame);
                }
                Poll::Ready(Ok(n))
            }
            Poll::Ready(Err(e)) => {
                this.outbound_frame = Some(frame);
                Poll::Ready(Err(e))
            }
            Poll::Pending => {
                this.outbound_frame = Some(frame);
                Poll::Pending
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if let Poll::Ready(Err(error)) = this.poll_keepalive(cx) {
            return Poll::Ready(Err(error));
        }
        if this
            .keepalive
            .pending_ping
            .is_some_and(|written| written > 0)
        {
            return Poll::Pending;
        }
        if this.outbound_frame.is_none() {
            match this.poll_flush_pending_pong(cx) {
                Poll::Ready(Ok(())) => {}
                result => return result,
            }
            if this.pending_close.is_some() {
                match this.poll_flush_pending_close(cx) {
                    Poll::Ready(Ok(())) => {}
                    result => return result,
                }
            }
        }
        Pin::new(&mut this.io).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        Pin::new(&mut this.io).poll_shutdown(cx)
    }
}

impl<S: AsyncRead + AsyncWrite + Unpin + Send> RelayStream for WsRelayStream<S> {
    fn transport(&self) -> ViewerTransport {
        self.transport
    }

    fn pending_write_len(&self) -> usize {
        self.pending_write_len()
    }

    fn poll_maintenance(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.get_mut().poll_keepalive(cx)
    }

    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        action: RelayCloseAction,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        // Our own expired Pong deadline is an abnormal, code-less shutdown.
        // 4006 remains reserved for a dead TCP endpoint on the other side.
        if this.keepalive.expired {
            return Pin::new(&mut this.io).poll_shutdown(cx);
        }
        // Finish any started Ping before a Close or another control frame.
        if this
            .keepalive
            .pending_ping
            .is_some_and(|written| written > 0)
            && let Poll::Ready(Err(error)) = this.poll_keepalive(cx)
        {
            return Poll::Ready(Err(error));
        }
        if this
            .keepalive
            .pending_ping
            .is_some_and(|written| written > 0)
        {
            return Poll::Pending;
        }
        this.keepalive.pending_ping = None;

        // Contract (Q83 / websocket-stream.md §5.1 / §9.4):
        // Close is only sent once pending_write_len() == 0; the caller must finish
        // the remainder of an in-flight frame via poll_write first.
        // Return Pending so Close is not emitted mid-frame.
        if this.pending_write_len() > 0 {
            return Poll::Pending;
        }
        // A header that has not reached the wire may be abandoned.
        this.outbound_frame = None;

        // A completed Close is never replaced or emitted a second time.
        if this.close_sent {
            return if this.terminal_close {
                this.poll_terminal_disposal(cx)
            } else {
                Pin::new(&mut this.io).poll_flush(cx)
            };
        }
        // A shutdown request cannot replace a Close already received from the
        // peer; the existing asymmetric handshake retains its echo action.
        let action = if action == RelayCloseAction::ServerShutdown && this.decoder.peer_closed {
            RelayCloseAction::PeerEof
        } else {
            action
        };

        // Map action to Close code and fixed reason per websocket-stream.md §5.2
        if this.pending_close.is_none() {
            let close_spec = match action {
                RelayCloseAction::HandshakeInvalid => Some((4008, "handshake_invalid")),
                RelayCloseAction::HandshakeTimeout => Some((4009, "handshake_timeout")),
                RelayCloseAction::ModeDisabled => Some((1008, "mode_disabled")),
                RelayCloseAction::RejectDuplicate => Some((4004, "duplicate_id")),
                RelayCloseAction::WaitingExpired => Some((4001, "wait_timeout")),
                RelayCloseAction::WaitingFull => Some((4010, "waiting_full")),
                RelayCloseAction::TargetConnectFailed => Some((4002, "target_connect_failed")),
                RelayCloseAction::TargetNotAllowed => Some((4003, "target_not_allowed")),
                RelayCloseAction::TargetConnectionLimit => Some((4011, "relay_connection_limit")),
                RelayCloseAction::WriteHalf => Some((1000, "")),
                RelayCloseAction::PeerEof => {
                    // Echo stored received code (1000 if none; 1005/1006/1015 are never sent)
                    let code = match this.decoder.received_close_code {
                        Some(c) if c != 1005 && c != 1006 && c != 1015 => c,
                        _ => 1000,
                    };
                    Some((code, ""))
                }
                RelayCloseAction::StreamError(ConnectionErrorKind::WsProtocolError) => {
                    Some((1002, ""))
                }
                RelayCloseAction::StreamError(ConnectionErrorKind::WsTextMessage) => {
                    Some((1003, ""))
                }
                RelayCloseAction::StreamError(ConnectionErrorKind::WsMessageTooLarge) => {
                    Some((1009, ""))
                }
                RelayCloseAction::StreamError(ConnectionErrorKind::IoError) => {
                    Some((4007, "peer_closed"))
                }
                RelayCloseAction::StreamError(ConnectionErrorKind::TlsFailed) => None,
                RelayCloseAction::DeadConnection => Some((4006, "dead_connection")),
                RelayCloseAction::AdminTerminated => Some((4005, "admin_terminated")),
                RelayCloseAction::ServerShutdown => Some((1001, "server_shutdown")),
            };

            match close_spec {
                Some((code, reason)) => {
                    this.pending_close = Some(encode_close(code, reason));
                    this.terminal_close = !matches!(
                        action,
                        RelayCloseAction::WriteHalf | RelayCloseAction::PeerEof
                    );
                    this.raw_terminal_disposal = matches!(
                        action,
                        RelayCloseAction::StreamError(
                            ConnectionErrorKind::WsProtocolError
                                | ConnectionErrorKind::WsTextMessage
                                | ConnectionErrorKind::WsMessageTooLarge
                        )
                    );
                }
                None => {
                    this.close_sent = true;
                    return Pin::new(&mut this.io).poll_shutdown(cx);
                }
            }
        }

        match this.poll_flush_pending_close(cx) {
            Poll::Ready(Ok(())) if this.terminal_close => this.poll_terminal_disposal(cx),
            Poll::Ready(Ok(())) => Pin::new(&mut this.io).poll_flush(cx),
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}
