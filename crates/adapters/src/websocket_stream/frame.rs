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
    pub buf: [u8; 127],
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

/// Formats an unmasked server Close frame, truncating the reason at a UTF-8
/// boundary to fit the RFC 6455 control payload limit of 125 bytes.
pub fn encode_close(code: u16, reason: &str) -> PendingClose {
    let reason_bytes = reason.as_bytes();
    let mut reason_len = reason_bytes.len().min(123);
    while !reason.is_char_boundary(reason_len) {
        reason_len -= 1;
    }
    let payload_len = 2 + reason_len;
    let mut buf = [0u8; 127];
    buf[0] = 0x88; // FIN = 1, opcode = Close (8)
    buf[1] = payload_len as u8;
    let code_bytes = code.to_be_bytes();
    buf[2] = code_bytes[0];
    buf[3] = code_bytes[1];
    for (dst, src) in buf
        .iter_mut()
        .skip(4)
        .zip(reason_bytes.iter().take(reason_len))
    {
        *dst = *src;
    }
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
        let header_bytes = self.header_buf.get(..self.header_len).ok_or_else(|| {
            RelayStreamError::new(ConnectionErrorKind::WsProtocolError).into_io_error()
        })?;
        let mut cursor = Cursor::new(header_bytes);
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

        // RFC 6455 §5.2 requires the shortest length encoding and a zero high
        // bit for the 64-bit form. Inspect the raw header before clearing it or
        // interpreting a malformed length as a logical message size violation.
        let invalid_length = match header_bytes {
            [_, marker, hi, lo, ..] if marker & 0x7f == 126 => u16::from_be_bytes([*hi, *lo]) < 126,
            [_, marker, a, b, c, d, e, f, g, h, ..] if marker & 0x7f == 127 => {
                a & 0x80 != 0 || u64::from_be_bytes([*a, *b, *c, *d, *e, *f, *g, *h]) < 65536
            }
            _ => false,
        };
        if invalid_length {
            return Err(RelayStreamError::new(ConnectionErrorKind::WsProtocolError).into_io_error());
        }

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

#[cfg(test)]
mod tests {
    use super::{DecodeState, FrameDecoder, encode_close};
    use vnc_relay_app::events::ConnectionErrorKind;
    use vnc_relay_app::ports::RelayStreamError;

    fn masked_header(opcode: u8, marker: u8, extended: &[u8]) -> Vec<u8> {
        let mut header = vec![opcode, 0x80 | marker];
        header.extend_from_slice(extended);
        header.extend_from_slice(&[0x12, 0x34, 0x56, 0x78]);
        header
    }

    fn assert_incomplete(decoder: &mut FrameDecoder, expected_len: usize) {
        assert!(!decoder.try_parse_header().expect("incomplete header"));
        assert_eq!(decoder.header_len, expected_len);
        assert_eq!(decoder.current_message_len, 0);
        assert!(matches!(decoder.state, DecodeState::AwaitingHeader));
    }

    /// spec: websocket.md:WS-019 partial
    #[test]
    fn rejects_noncanonical_lengths_at_every_header_split() {
        let encodings = [
            (126, 0_u16.to_be_bytes().to_vec()),
            (126, 1_u16.to_be_bytes().to_vec()),
            (126, 125_u16.to_be_bytes().to_vec()),
            (127, 0_u64.to_be_bytes().to_vec()),
            (127, 125_u64.to_be_bytes().to_vec()),
            (127, 126_u64.to_be_bytes().to_vec()),
            (127, 65535_u64.to_be_bytes().to_vec()),
            (127, (1_u64 << 63).to_be_bytes().to_vec()),
            (127, ((1_u64 << 63) | 65536).to_be_bytes().to_vec()),
            (127, u64::MAX.to_be_bytes().to_vec()),
        ];
        for opcode in [0x82, 0x88, 0x89, 0x8a] {
            for (marker, extended) in &encodings {
                let header = masked_header(opcode, *marker, extended);
                for split in 0..=header.len() {
                    // M = 0 also proves malformed lengths take precedence over
                    // the otherwise applicable WsMessageTooLarge classification.
                    let mut decoder = FrameDecoder::new(0);
                    assert_eq!(decoder.append_header_bytes(&header[..split]), split);
                    if split < header.len() {
                        assert_incomplete(&mut decoder, split);
                        assert_eq!(
                            decoder.append_header_bytes(&header[split..]),
                            header.len() - split
                        );
                    }
                    let error = decoder.try_parse_header().expect_err("invalid length");
                    assert_eq!(
                        RelayStreamError::classify(&error),
                        ConnectionErrorKind::WsProtocolError,
                        "opcode={opcode:#x}, marker={marker}, split={split}"
                    );
                    assert_eq!(decoder.header_len, header.len());
                    assert_eq!(decoder.current_message_len, 0);
                    assert!(matches!(decoder.state, DecodeState::AwaitingHeader));
                }
                let mut decoder = FrameDecoder::new(0);
                for (offset, byte) in header.iter().enumerate() {
                    assert_eq!(decoder.append_header_bytes(&[*byte]), 1);
                    if offset + 1 < header.len() {
                        assert_incomplete(&mut decoder, offset + 1);
                    } else {
                        let error = decoder.try_parse_header().expect_err("invalid length");
                        assert_eq!(
                            RelayStreamError::classify(&error),
                            ConnectionErrorKind::WsProtocolError
                        );
                    }
                }
            }
        }
    }

    /// spec: websocket.md:WS-010 partial
    #[test]
    fn accepts_canonical_length_boundaries_and_distinguishes_m_plus_one() {
        let encodings = [
            (125_usize, 125, Vec::new()),
            (126, 126, 126_u16.to_be_bytes().to_vec()),
            (65535, 126, 65535_u16.to_be_bytes().to_vec()),
            (65536, 127, 65536_u64.to_be_bytes().to_vec()),
            (65537, 127, 65537_u64.to_be_bytes().to_vec()),
        ];
        for (payload_len, marker, extended) in encodings {
            let header = masked_header(0x82, marker, &extended);
            for split in 0..=header.len() {
                for max_message_size in [payload_len, payload_len - 1] {
                    let mut decoder = FrameDecoder::new(max_message_size);
                    assert_eq!(decoder.append_header_bytes(&header[..split]), split);
                    if split < header.len() {
                        assert_incomplete(&mut decoder, split);
                        assert_eq!(
                            decoder.append_header_bytes(&header[split..]),
                            header.len() - split
                        );
                    }
                    let result = decoder.try_parse_header();
                    if max_message_size < payload_len {
                        let error = result.expect_err("canonical M+1 header");
                        assert_eq!(
                            RelayStreamError::classify(&error),
                            ConnectionErrorKind::WsMessageTooLarge
                        );
                    } else {
                        assert!(result.expect("canonical header"));
                        assert_eq!(decoder.current_message_len, payload_len);
                        match decoder.state {
                            DecodeState::ReadingPayload {
                                frame_payload_len,
                                frame_payload_read,
                                ..
                            } => {
                                assert_eq!(frame_payload_len, payload_len as u64);
                                assert_eq!(frame_payload_read, 0);
                            }
                            state => panic!("expected payload state, got {state:?}"),
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn encodes_close_reason_beyond_the_old_28_byte_limit() {
        let reason = "x".repeat(29);
        let close = encode_close(1000, &reason);
        let bytes = close.remaining_bytes();
        assert_eq!(bytes.len(), 33);
        assert_eq!(&bytes[..4], &[0x88, 31, 0x03, 0xe8]);
        assert_eq!(&bytes[4..], reason.as_bytes());
        assert_eq!(close.written, 0);
    }

    #[test]
    fn encodes_maximum_close_payload_and_truncates_on_utf8_boundaries() {
        let reasons = [
            ("x".repeat(123), "x".repeat(123)),
            ("x".repeat(124), "x".repeat(123)),
            ("界".repeat(41), "界".repeat(41)),
            ("界".repeat(42), "界".repeat(41)),
            (format!("{}界", "x".repeat(122)), "x".repeat(122)),
            (format!("{}é", "x".repeat(122)), "x".repeat(122)),
            ("😀".repeat(31), "😀".repeat(30)),
        ];
        for (reason, expected) in reasons {
            let mut close = encode_close(1000, &reason);
            let bytes = close.remaining_bytes();
            assert_eq!(bytes.len(), 4 + expected.len());
            assert!(bytes.len() <= 127);
            assert_eq!(bytes[0], 0x88);
            assert_eq!(bytes[1] as usize, 2 + expected.len());
            assert_eq!(&bytes[2..4], &1000_u16.to_be_bytes());
            assert_eq!(
                std::str::from_utf8(&bytes[4..]).expect("UTF-8 Close reason"),
                expected
            );
            close.advance(1);
            assert_eq!(close.remaining_bytes().len(), 3 + expected.len());
            close.advance(usize::from(close.len));
            assert!(close.is_finished());
            assert!(close.remaining_bytes().is_empty());
        }
    }
}
