//! Incremental RFC 6455 WebSocket frame decoder and encoder state machine.
//!
//! Handles streaming frame parsing, in-place unmasking, validation, logical
//! message size bounds across fragments, and control frames without whole-frame
//! or whole-message buffering (Q77, Q78).

use std::io::{self, Cursor};

use tokio_tungstenite::tungstenite::protocol::frame::FrameHeader;
use tokio_tungstenite::tungstenite::protocol::frame::coding::{CloseCode, Control, Data, OpCode};
use vnc_relay_app::events::ConnectionErrorKind;
use vnc_relay_app::ports::RelayStreamError;

/// Maximum number of non-data frames (empty messages, pings, unsolicited pongs)
/// processed in one `poll_read` before yielding to the executor.
pub const MAX_NON_DATA_FRAMES_PER_POLL: usize = 32;

/// A control opcode recognized by the decoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlOp {
    Close,
    Ping,
    Pong,
}

/// The state of the streaming frame decoder.
#[derive(Debug)]
pub enum DecodeState {
    /// Accumulating header bytes (up to 14 bytes).
    AwaitingHeader,
    /// Streaming data payload into caller buffer.
    ReadingPayload {
        mask: [u8; 4],
        frame_payload_len: u64,
        frame_payload_read: u64,
        is_final: bool,
    },
    /// Accumulating control frame payload (up to 125 bytes).
    ReadingControl {
        opcode: ControlOp,
        mask: [u8; 4],
        payload_len: u8,
        payload_read: u8,
        buf: [u8; 125],
    },
    /// A valid Close frame was received and processed.
    PeerClosed,
}

/// A pending Pong control frame to be sent to the peer.
#[derive(Debug, Clone, Copy)]
pub struct PendingPong {
    pub buf: [u8; 127],
    pub len: u8,
    pub written: u8,
}

impl PendingPong {
    pub fn is_finished(&self) -> bool {
        self.written >= self.len
    }

    pub fn remaining_bytes(&self) -> &[u8] {
        &self.buf[self.written as usize..self.len as usize]
    }

    pub fn advance(&mut self, n: usize) {
        self.written = (self.written as usize + n).min(self.len as usize) as u8;
    }
}

/// A pending Close control frame to be sent to the peer.
#[derive(Debug, Clone, Copy)]
pub struct PendingClose {
    pub buf: [u8; 32],
    pub len: u8,
    pub written: u8,
}

impl PendingClose {
    pub fn is_finished(&self) -> bool {
        self.written >= self.len
    }

    pub fn remaining_bytes(&self) -> &[u8] {
        &self.buf[self.written as usize..self.len as usize]
    }

    pub fn advance(&mut self, n: usize) {
        self.written = (self.written as usize + n).min(self.len as usize) as u8;
    }
}

/// State of an in-flight outbound data frame.
#[derive(Debug, Clone, Copy)]
pub struct OutboundDataFrame {
    pub header_buf: [u8; 10],
    pub header_len: u8,
    pub header_written: u8,
    pub payload_len: usize,
    pub payload_written: usize,
}

impl OutboundDataFrame {
    pub fn is_header_finished(&self) -> bool {
        self.header_written >= self.header_len
    }

    pub fn remaining_header(&self) -> &[u8] {
        &self.header_buf[self.header_written as usize..self.header_len as usize]
    }

    pub fn advance_header(&mut self, n: usize) {
        self.header_written =
            (self.header_written as usize + n).min(self.header_len as usize) as u8;
    }

    pub fn remaining_payload_len(&self) -> usize {
        self.payload_len.saturating_sub(self.payload_written)
    }

    pub fn advance_payload(&mut self, n: usize) {
        self.payload_written = (self.payload_written + n).min(self.payload_len);
    }

    pub fn is_finished(&self) -> bool {
        self.is_header_finished() && self.payload_written >= self.payload_len
    }
}

/// Formats an unmasked server binary frame header for `payload_len`.
pub fn encode_data_header(payload_len: usize) -> ([u8; 10], u8) {
    let mut header_buf = [0u8; 10];
    header_buf[0] = 0x82; // FIN = 1, opcode = Binary (2)
    let header_len = if payload_len <= 125 {
        header_buf[1] = payload_len as u8;
        2
    } else if payload_len <= 65535 {
        header_buf[1] = 126;
        let len_bytes = (payload_len as u16).to_be_bytes();
        header_buf[2] = len_bytes[0];
        header_buf[3] = len_bytes[1];
        4
    } else {
        header_buf[1] = 127;
        let len_bytes = (payload_len as u64).to_be_bytes();
        header_buf[2..10].copy_from_slice(&len_bytes);
        10
    };
    (header_buf, header_len)
}

/// Formats an unmasked server Pong frame with the echoed payload (up to 125 bytes).
pub fn encode_pong(payload: &[u8]) -> PendingPong {
    let payload_len = payload.len().min(125);
    let mut buf = [0u8; 127];
    buf[0] = 0x8A; // FIN = 1, opcode = Pong (10)
    buf[1] = payload_len as u8;
    buf[2..2 + payload_len].copy_from_slice(&payload[..payload_len]);
    PendingPong {
        buf,
        len: (2 + payload_len) as u8,
        written: 0,
    }
}

/// Formats an unmasked server Close frame with status code and reason string.
pub fn encode_close(code: u16, reason: &str) -> PendingClose {
    let reason_bytes = reason.as_bytes();
    let reason_len = reason_bytes.len().min(123);
    let payload_len = 2 + reason_len;
    let mut buf = [0u8; 32];
    buf[0] = 0x88; // FIN = 1, opcode = Close (8)
    buf[1] = payload_len as u8;
    let code_bytes = code.to_be_bytes();
    buf[2] = code_bytes[0];
    buf[3] = code_bytes[1];
    buf[4..4 + reason_len].copy_from_slice(&reason_bytes[..reason_len]);
    PendingClose {
        buf,
        len: (2 + payload_len) as u8,
        written: 0,
    }
}

/// An incremental, streaming RFC 6455 frame decoder.
#[derive(Debug)]
pub struct FrameDecoder {
    pub max_message_size: usize,
    pub current_message_len: usize,
    pub in_fragmented_message: bool,
    pub state: DecodeState,
    pub header_buf: [u8; 14],
    pub header_len: usize,
    pub leftover_buf: [u8; 14],
    pub leftover_pos: usize,
    pub leftover_len: usize,
    pub received_close_code: Option<u16>,
    pub peer_closed: bool,
    pub pending_pong: Option<PendingPong>,
}

impl FrameDecoder {
    pub fn new(max_message_size: usize) -> Self {
        Self {
            max_message_size,
            current_message_len: 0,
            in_fragmented_message: false,
            state: DecodeState::AwaitingHeader,
            header_buf: [0u8; 14],
            header_len: 0,
            leftover_buf: [0u8; 14],
            leftover_pos: 0,
            leftover_len: 0,
            received_close_code: None,
            peer_closed: false,
            pending_pong: None,
        }
    }

    /// Bytes currently held inside the decoder's fixed buffers.
    pub fn holding_bytes(&self) -> usize {
        let control_bytes = match &self.state {
            DecodeState::ReadingControl { payload_read, .. } => *payload_read as usize,
            _ => 0,
        };
        self.header_len + (self.leftover_len - self.leftover_pos) + control_bytes
    }

    /// Reads payload directly from leftover buffer into `dst`, unmasking in place.
    pub fn read_payload_from_leftover(&mut self, dst: &mut [u8]) -> usize {
        if let DecodeState::ReadingPayload {
            mask,
            frame_payload_len,
            ref mut frame_payload_read,
            is_final,
        } = self.state
        {
            let remaining_in_frame = (frame_payload_len - *frame_payload_read) as usize;
            let avail = self.leftover_len - self.leftover_pos;
            let want = dst.len().min(remaining_in_frame).min(avail);
            if want == 0 {
                return 0;
            }
            for (i, b) in dst[..want].iter_mut().enumerate() {
                let raw = self.leftover_buf[self.leftover_pos + i];
                *b = raw ^ mask[((*frame_payload_read + i as u64) % 4) as usize];
            }
            self.leftover_pos += want;
            if self.leftover_pos == self.leftover_len {
                self.leftover_pos = 0;
                self.leftover_len = 0;
            }
            *frame_payload_read += want as u64;
            if *frame_payload_read == frame_payload_len {
                if is_final {
                    self.in_fragmented_message = false;
                    self.current_message_len = 0;
                }
                self.state = DecodeState::AwaitingHeader;
            }
            want
        } else {
            0
        }
    }

    /// Unmasks freshly read payload bytes in `bytes` and advances frame read offset.
    pub fn advance_payload(&mut self, bytes: &mut [u8]) {
        if let DecodeState::ReadingPayload {
            mask,
            frame_payload_len,
            ref mut frame_payload_read,
            is_final,
        } = self.state
        {
            for (i, b) in bytes.iter_mut().enumerate() {
                *b ^= mask[((*frame_payload_read + i as u64) % 4) as usize];
            }
            *frame_payload_read += bytes.len() as u64;
            if *frame_payload_read == frame_payload_len {
                if is_final {
                    self.in_fragmented_message = false;
                    self.current_message_len = 0;
                }
                self.state = DecodeState::AwaitingHeader;
            }
        }
    }

    /// Copies any leftover bytes into `header_buf`.
    pub fn drain_leftover_to_header(&mut self) -> usize {
        let avail = self.leftover_len - self.leftover_pos;
        if avail == 0 || self.header_len >= 14 {
            return 0;
        }
        let to_copy = avail.min(14 - self.header_len);
        self.header_buf[self.header_len..self.header_len + to_copy]
            .copy_from_slice(&self.leftover_buf[self.leftover_pos..self.leftover_pos + to_copy]);
        self.header_len += to_copy;
        self.leftover_pos += to_copy;
        if self.leftover_pos == self.leftover_len {
            self.leftover_pos = 0;
            self.leftover_len = 0;
        }
        to_copy
    }

    /// Appends bytes from `src` to `header_buf`.
    pub fn append_header_bytes(&mut self, src: &[u8]) -> usize {
        let to_copy = src.len().min(14 - self.header_len);
        self.header_buf[self.header_len..self.header_len + to_copy]
            .copy_from_slice(&src[..to_copy]);
        self.header_len += to_copy;
        to_copy
    }

    /// Copies any leftover bytes into control payload buffer.
    pub fn drain_leftover_to_control(&mut self) -> usize {
        if let DecodeState::ReadingControl {
            payload_len,
            ref mut payload_read,
            ref mut buf,
            ..
        } = self.state
        {
            let avail = self.leftover_len - self.leftover_pos;
            let needed = (payload_len - *payload_read) as usize;
            if avail == 0 || needed == 0 {
                return 0;
            }
            let to_copy = avail.min(needed);
            buf[*payload_read as usize..*payload_read as usize + to_copy].copy_from_slice(
                &self.leftover_buf[self.leftover_pos..self.leftover_pos + to_copy],
            );
            *payload_read += to_copy as u8;
            self.leftover_pos += to_copy;
            if self.leftover_pos == self.leftover_len {
                self.leftover_pos = 0;
                self.leftover_len = 0;
            }
            to_copy
        } else {
            0
        }
    }

    /// Appends bytes from `src` to the control buffer.
    pub fn append_control_bytes(&mut self, src: &[u8]) -> usize {
        if let DecodeState::ReadingControl {
            payload_len,
            ref mut payload_read,
            ref mut buf,
            ..
        } = self.state
        {
            let needed = (payload_len - *payload_read) as usize;
            let to_copy = src.len().min(needed);
            buf[*payload_read as usize..*payload_read as usize + to_copy]
                .copy_from_slice(&src[..to_copy]);
            *payload_read += to_copy as u8;
            to_copy
        } else {
            0
        }
    }

    /// True if currently in `ReadingControl` and all control payload bytes have been read.
    pub fn is_control_ready(&self) -> bool {
        match &self.state {
            DecodeState::ReadingControl {
                payload_len,
                payload_read,
                ..
            } => *payload_read == *payload_len,
            _ => false,
        }
    }

    /// Finishes the current control frame if all bytes have been collected.
    pub fn finish_control_frame(&mut self) -> io::Result<Option<ControlOp>> {
        match self.state {
            DecodeState::ReadingControl {
                opcode,
                mask,
                payload_len,
                payload_read,
                buf,
            } if payload_read == payload_len => {
                self.process_control_frame(opcode, mask, payload_len, buf)?;
                Ok(Some(opcode))
            }
            _ => Ok(None),
        }
    }

    /// Attempts to parse and validate a frame header from `header_buf[..header_len]`.
    ///
    /// On success, configures `state` and moves any leftover bytes after the header
    /// into `leftover_buf`. Returns `Ok(true)` if a header was parsed, `Ok(false)`
    /// if more header bytes are needed.
    pub fn try_parse_header(&mut self) -> io::Result<bool> {
        let mut cursor = Cursor::new(&self.header_buf[..self.header_len]);
        let parsed = FrameHeader::parse(&mut cursor).map_err(|_| {
            RelayStreamError::new(ConnectionErrorKind::WsProtocolError).into_io_error()
        })?;

        let (hdr, payload_len) = match parsed {
            Some(pair) => pair,
            None => {
                if self.header_len >= 14 {
                    return Err(
                        RelayStreamError::new(ConnectionErrorKind::WsProtocolError).into_io_error()
                    );
                }
                return Ok(false);
            }
        };

        let consumed = cursor.position() as usize;
        let leftover = self.header_len - consumed;
        if leftover > 0 {
            self.leftover_buf[..leftover]
                .copy_from_slice(&self.header_buf[consumed..self.header_len]);
            self.leftover_len = leftover;
            self.leftover_pos = 0;
        }
        self.header_len = 0;

        // 1. Client frame MUST be masked (RFC 6455 §5.1)
        let mask = match hdr.mask {
            Some(m) => m,
            None => {
                return Err(
                    RelayStreamError::new(ConnectionErrorKind::WsProtocolError).into_io_error()
                );
            }
        };

        // 2. RSV bits MUST be 0 (no extensions negotiated)
        if hdr.rsv1 || hdr.rsv2 || hdr.rsv3 {
            return Err(RelayStreamError::new(ConnectionErrorKind::WsProtocolError).into_io_error());
        }

        // 3. Opcode validation & logical message length checks
        match hdr.opcode {
            OpCode::Data(Data::Text) => {
                // First text frame -> 1003
                return Err(
                    RelayStreamError::new(ConnectionErrorKind::WsTextMessage).into_io_error()
                );
            }
            OpCode::Data(Data::Continue) => {
                if !self.in_fragmented_message {
                    return Err(
                        RelayStreamError::new(ConnectionErrorKind::WsProtocolError).into_io_error()
                    );
                }
                if payload_len > usize::MAX as u64 {
                    return Err(
                        RelayStreamError::new(ConnectionErrorKind::WsMessageTooLarge)
                            .into_io_error(),
                    );
                }
                match self.current_message_len.checked_add(payload_len as usize) {
                    Some(new_len) if new_len <= self.max_message_size => {
                        self.current_message_len = new_len;
                    }
                    _ => {
                        return Err(
                            RelayStreamError::new(ConnectionErrorKind::WsMessageTooLarge)
                                .into_io_error(),
                        );
                    }
                }
                if payload_len == 0 {
                    if hdr.is_final {
                        self.in_fragmented_message = false;
                        self.current_message_len = 0;
                    }
                    self.state = DecodeState::AwaitingHeader;
                } else {
                    self.state = DecodeState::ReadingPayload {
                        mask,
                        frame_payload_len: payload_len,
                        frame_payload_read: 0,
                        is_final: hdr.is_final,
                    };
                }
            }
            OpCode::Data(Data::Binary) => {
                if self.in_fragmented_message {
                    return Err(
                        RelayStreamError::new(ConnectionErrorKind::WsProtocolError).into_io_error()
                    );
                }
                if payload_len > usize::MAX as u64 || payload_len as usize > self.max_message_size {
                    return Err(
                        RelayStreamError::new(ConnectionErrorKind::WsMessageTooLarge)
                            .into_io_error(),
                    );
                }
                self.current_message_len = payload_len as usize;
                if !hdr.is_final {
                    self.in_fragmented_message = true;
                }
                if payload_len == 0 {
                    if hdr.is_final {
                        self.in_fragmented_message = false;
                        self.current_message_len = 0;
                    }
                    self.state = DecodeState::AwaitingHeader;
                } else {
                    self.state = DecodeState::ReadingPayload {
                        mask,
                        frame_payload_len: payload_len,
                        frame_payload_read: 0,
                        is_final: hdr.is_final,
                    };
                }
            }
            OpCode::Data(Data::Reserved(_)) => {
                return Err(
                    RelayStreamError::new(ConnectionErrorKind::WsProtocolError).into_io_error()
                );
            }
            OpCode::Control(ctrl) => {
                // Control frames MUST NOT be fragmented and payload MUST be <= 125
                if !hdr.is_final || payload_len > 125 {
                    return Err(
                        RelayStreamError::new(ConnectionErrorKind::WsProtocolError).into_io_error()
                    );
                }
                let op = match ctrl {
                    Control::Close => ControlOp::Close,
                    Control::Ping => ControlOp::Ping,
                    Control::Pong => ControlOp::Pong,
                    Control::Reserved(_) => {
                        return Err(RelayStreamError::new(ConnectionErrorKind::WsProtocolError)
                            .into_io_error());
                    }
                };
                self.state = DecodeState::ReadingControl {
                    opcode: op,
                    mask,
                    payload_len: payload_len as u8,
                    payload_read: 0,
                    buf: [0u8; 125],
                };
            }
        }

        Ok(true)
    }

    /// Processes a completed control frame payload.
    pub fn process_control_frame(
        &mut self,
        opcode: ControlOp,
        mask: [u8; 4],
        payload_len: u8,
        mut buf: [u8; 125],
    ) -> io::Result<()> {
        let len = payload_len as usize;
        // In-place unmask
        for (i, b) in buf[..len].iter_mut().enumerate() {
            *b ^= mask[i % 4];
        }

        match opcode {
            ControlOp::Close => {
                // Close payload validation (RFC 6455 §5.5.1, §7.4)
                if len == 1 {
                    return Err(
                        RelayStreamError::new(ConnectionErrorKind::WsProtocolError).into_io_error()
                    );
                }
                let code = if len >= 2 {
                    let raw = u16::from_be_bytes([buf[0], buf[1]]);
                    if !CloseCode::from(raw).is_allowed() {
                        return Err(RelayStreamError::new(ConnectionErrorKind::WsProtocolError)
                            .into_io_error());
                    }
                    // Validate UTF-8 of reason string
                    if std::str::from_utf8(&buf[2..len]).is_err() {
                        return Err(RelayStreamError::new(ConnectionErrorKind::WsProtocolError)
                            .into_io_error());
                    }
                    Some(raw)
                } else {
                    None
                };
                self.received_close_code = code;
                self.peer_closed = true;
                self.state = DecodeState::PeerClosed;
            }
            ControlOp::Ping => {
                // RFC 6455 §5.5.3: Queue one Pong slot, keeping the latest pending Pong
                self.pending_pong = Some(encode_pong(&buf[..len]));
                self.state = DecodeState::AwaitingHeader;
            }
            ControlOp::Pong => {
                // Unsolicited Pong: ignored
                self.state = DecodeState::AwaitingHeader;
            }
        }

        Ok(())
    }
}
