use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf, duplex};
use vnc_relay_app::events::ConnectionErrorKind;
use vnc_relay_app::ports::{RelayCloseAction, RelayStream, RelayStreamError};
use vnc_relay_domain::session::ViewerTransport;

use super::WsRelayStream;

fn noop_waker() -> Waker {
    Waker::noop().clone()
}

/// Wrapper around an AsyncWrite stream that restricts writes to at most `max_chunk` bytes.
struct PartialStream<S> {
    inner: S,
    max_chunk: usize,
}

impl<S> PartialStream<S> {
    fn new(inner: S, max_chunk: usize) -> Self {
        Self { inner, max_chunk }
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for PartialStream<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for PartialStream<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let to_write = buf.len().min(self.max_chunk);
        Pin::new(&mut self.inner).poll_write(cx, &buf[..to_write])
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// Helper to build a masked client frame per RFC 6455 §5.2.
fn make_client_frame(
    fin: bool,
    rsv: (bool, bool, bool),
    opcode: u8,
    mask: [u8; 4],
    payload: &[u8],
) -> Vec<u8> {
    let mut out = Vec::new();
    let b0 = (if fin { 0x80 } else { 0 })
        | (if rsv.0 { 0x40 } else { 0 })
        | (if rsv.1 { 0x20 } else { 0 })
        | (if rsv.2 { 0x10 } else { 0 })
        | (opcode & 0x0F);
    out.push(b0);

    let len = payload.len();
    if len <= 125 {
        out.push(0x80 | (len as u8));
    } else if len <= 65535 {
        out.push(0x80 | 126);
        out.extend_from_slice(&(len as u16).to_be_bytes());
    } else {
        out.push(0x80 | 127);
        out.extend_from_slice(&(len as u64).to_be_bytes());
    }

    out.extend_from_slice(&mask);
    for (i, &b) in payload.iter().enumerate() {
        out.push(b ^ mask[i % 4]);
    }
    out
}

fn client_binary_frame(payload: &[u8]) -> Vec<u8> {
    make_client_frame(
        true,
        (false, false, false),
        2,
        [0x12, 0x34, 0x56, 0x78],
        payload,
    )
}

/// spec: relay.md:RLY-001 partial
#[tokio::test]
async fn sends_the_greeting_as_one_binary_message() {
    let (client_io, server_io) = duplex(1024);
    // Underlying transport only writes 3 bytes at a time
    let partial_io = PartialStream::new(server_io, 3);
    let mut ws = WsRelayStream::new(
        partial_io,
        Vec::new(),
        64 * 1024,
        16 * 1024,
        ViewerTransport::Ws,
    );

    let greeting = b"RFB 000.000\n"; // 12 bytes
    ws.write_all(greeting).await.expect("write greeting");
    ws.flush().await.expect("flush");

    // Client reads the framed greeting
    let mut client_io = client_io;
    let mut received = vec![0u8; 14];
    client_io
        .read_exact(&mut received)
        .await
        .expect("read frame");

    // Must be exactly one unmasked binary FIN frame of length 12
    assert_eq!(received[0], 0x82); // FIN = 1, opcode = Binary (2)
    assert_eq!(received[1], 12); // unmasked, length = 12
    assert_eq!(&received[2..14], greeting);
}

/// spec: relay.md:RLY-002 partial
/// spec: websocket.md:WS-007 partial
#[tokio::test]
async fn keeps_initial_exchange_bytes_across_frames_and_messages() {
    let (mut client_io, server_io) = duplex(4096);
    let mut ws = WsRelayStream::new(
        server_io,
        Vec::new(),
        64 * 1024,
        16 * 1024,
        ViewerTransport::Ws,
    );

    // Empty binary message (FIN=1, opcode=2, payload="")
    let empty_frame = make_client_frame(true, (false, false, false), 2, [1, 2, 3, 4], b"");
    // Fragment 1 (FIN=0, opcode=2, payload="first ")
    let frag1 = make_client_frame(false, (false, false, false), 2, [2, 3, 4, 5], b"first ");
    // Empty continuation (FIN=0, opcode=0, payload="")
    let empty_cont = make_client_frame(false, (false, false, false), 0, [3, 4, 5, 6], b"");
    // Fragment 2 (FIN=1, opcode=0, payload="second")
    let frag2 = make_client_frame(true, (false, false, false), 0, [4, 5, 6, 7], b"second");
    // Next binary message (FIN=1, opcode=2, payload=" third")
    let msg2 = make_client_frame(true, (false, false, false), 2, [5, 6, 7, 8], b" third");

    client_io
        .write_all(&empty_frame)
        .await
        .expect("write empty");
    client_io.write_all(&frag1).await.expect("write frag1");
    client_io
        .write_all(&empty_cont)
        .await
        .expect("write empty_cont");
    client_io.write_all(&frag2).await.expect("write frag2");
    client_io.write_all(&msg2).await.expect("write msg2");

    let mut buf = vec![0u8; 18];
    ws.read_exact(&mut buf).await.expect("read payload");
    assert_eq!(&buf, b"first second third");
}

/// spec: websocket.md:WS-018 partial
/// spec: relay.md:RLY-003 partial
#[tokio::test]
async fn preserves_initial_exchange_trailing_bytes_once_within_b() {
    let (mut client_io, server_io) = duplex(4096);
    let suffix_frame = client_binary_frame(b"head-suffix-data");
    let mut ws = WsRelayStream::new(
        server_io,
        suffix_frame,
        64 * 1024,
        16 * 1024,
        ViewerTransport::Ws,
    );

    // Check holding <= B initially
    assert!(ws.holding_rx_bytes() <= 16 * 1024);

    let follow_up = client_binary_frame(b"-streamed-after");
    client_io
        .write_all(&follow_up)
        .await
        .expect("write follow up");

    let mut out = vec![0u8; 31];
    ws.read_exact(&mut out).await.expect("read all");
    assert_eq!(&out, b"head-suffix-data-streamed-after");
    assert_eq!(ws.holding_rx_bytes(), 0);
}

/// spec: websocket.md:WS-011 partial
/// spec: relay.md:RLY-018 partial
#[tokio::test]
async fn streams_a_single_frame_larger_than_b_without_a_message_buffer() {
    let b_limit = 16 * 1024; // 16 KiB
    let m_limit = 1024 * 1024; // 1 MiB
    let frame_len = 64 * 1024; // 64 KiB > B

    let (mut client_io, server_io) = duplex(128 * 1024);
    let mut ws = WsRelayStream::new(server_io, Vec::new(), m_limit, b_limit, ViewerTransport::Ws);

    let payload: Vec<u8> = (0..frame_len).map(|i| (i % 251) as u8).collect();
    let frame = client_binary_frame(&payload);

    tokio::spawn(async move {
        client_io
            .write_all(&frame)
            .await
            .expect("write large frame");
    });

    let mut received = Vec::with_capacity(frame_len);
    let mut chunk = [0u8; 4096];
    while received.len() < frame_len {
        let n = ws.read(&mut chunk).await.expect("read chunk");
        assert!(n > 0);
        received.extend_from_slice(&chunk[..n]);
        assert!(ws.holding_rx_bytes() <= b_limit);
    }

    assert_eq!(received.len(), frame_len);
    assert_eq!(received, payload);
}

/// spec: websocket.md:WS-010 partial
#[tokio::test]
async fn counts_m_across_fragments_and_rejects_m_plus_one() {
    let m = 100;
    let (mut client_io, server_io) = duplex(4096);
    let mut ws = WsRelayStream::new(server_io, Vec::new(), m, 16 * 1024, ViewerTransport::Ws);

    // Fragment 1: 50 bytes (allowed)
    let p1 = vec![b'a'; 50];
    let f1 = make_client_frame(false, (false, false, false), 2, [1, 2, 3, 4], &p1);
    // Fragment 2: 51 bytes (50 + 51 = 101 > M)
    let p2 = vec![b'b'; 51];
    let f2 = make_client_frame(true, (false, false, false), 0, [2, 3, 4, 5], &p2);

    client_io.write_all(&f1).await.expect("write f1");
    client_io.write_all(&f2).await.expect("write f2");

    let mut buf = [0u8; 50];
    ws.read_exact(&mut buf).await.expect("read first 50 bytes");
    assert_eq!(&buf, &p1[..]);

    // Next read detects M+1 on fragment 2 header
    let mut rest = [0u8; 10];
    let err = ws.read(&mut rest).await.expect_err("m+1 rejected");
    assert_eq!(
        RelayStreamError::classify(&err),
        ConnectionErrorKind::WsMessageTooLarge
    );
}

/// spec: websocket.md:WS-008 partial
/// spec: websocket.md:WS-019 partial
#[tokio::test]
async fn rejects_text_and_invalid_frames_with_typed_errors() {
    // 1. Text frame -> WsTextMessage (1003)
    {
        let (mut client_io, server_io) = duplex(1024);
        let mut ws =
            WsRelayStream::new(server_io, Vec::new(), 1024, 16 * 1024, ViewerTransport::Ws);
        let text_frame = make_client_frame(true, (false, false, false), 1, [1, 2, 3, 4], b"hello");
        client_io.write_all(&text_frame).await.expect("write");
        let mut buf = [0u8; 10];
        let err = ws.read(&mut buf).await.expect_err("text error");
        assert_eq!(
            RelayStreamError::classify(&err),
            ConnectionErrorKind::WsTextMessage
        );
    }

    // 2. Unmasked client frame -> WsProtocolError (1002)
    {
        let (mut client_io, server_io) = duplex(1024);
        let mut ws =
            WsRelayStream::new(server_io, Vec::new(), 1024, 16 * 1024, ViewerTransport::Ws);
        // Byte 0: FIN + Binary, Byte 1: unmasked len 5
        let unmasked = [0x82, 0x05, 1, 2, 3, 4, 5];
        client_io.write_all(&unmasked).await.expect("write");
        let mut buf = [0u8; 10];
        let err = ws.read(&mut buf).await.expect_err("unmasked error");
        assert_eq!(
            RelayStreamError::classify(&err),
            ConnectionErrorKind::WsProtocolError
        );
    }

    // 3. RSV1 != 0 -> WsProtocolError (1002)
    {
        let (mut client_io, server_io) = duplex(1024);
        let mut ws =
            WsRelayStream::new(server_io, Vec::new(), 1024, 16 * 1024, ViewerTransport::Ws);
        let rsv_frame = make_client_frame(true, (true, false, false), 2, [1, 2, 3, 4], b"test");
        client_io.write_all(&rsv_frame).await.expect("write");
        let mut buf = [0u8; 10];
        let err = ws.read(&mut buf).await.expect_err("rsv error");
        assert_eq!(
            RelayStreamError::classify(&err),
            ConnectionErrorKind::WsProtocolError
        );
    }

    // 4. Unknown opcode (0x3) -> WsProtocolError (1002)
    {
        let (mut client_io, server_io) = duplex(1024);
        let mut ws =
            WsRelayStream::new(server_io, Vec::new(), 1024, 16 * 1024, ViewerTransport::Ws);
        let bad_opcode = make_client_frame(true, (false, false, false), 3, [1, 2, 3, 4], b"test");
        client_io.write_all(&bad_opcode).await.expect("write");
        let mut buf = [0u8; 10];
        let err = ws.read(&mut buf).await.expect_err("bad opcode error");
        assert_eq!(
            RelayStreamError::classify(&err),
            ConnectionErrorKind::WsProtocolError
        );
    }

    // 5. Continuation when no message started -> WsProtocolError (1002)
    {
        let (mut client_io, server_io) = duplex(1024);
        let mut ws =
            WsRelayStream::new(server_io, Vec::new(), 1024, 16 * 1024, ViewerTransport::Ws);
        let orphaned_cont =
            make_client_frame(true, (false, false, false), 0, [1, 2, 3, 4], b"test");
        client_io.write_all(&orphaned_cont).await.expect("write");
        let mut buf = [0u8; 10];
        let err = ws.read(&mut buf).await.expect_err("cont error");
        assert_eq!(
            RelayStreamError::classify(&err),
            ConnectionErrorKind::WsProtocolError
        );
    }

    // 6. Fragmented control frame (FIN = 0) -> WsProtocolError (1002)
    {
        let (mut client_io, server_io) = duplex(1024);
        let mut ws =
            WsRelayStream::new(server_io, Vec::new(), 1024, 16 * 1024, ViewerTransport::Ws);
        let frag_ping = make_client_frame(false, (false, false, false), 9, [1, 2, 3, 4], b"ping");
        client_io.write_all(&frag_ping).await.expect("write");
        let mut buf = [0u8; 10];
        let err = ws.read(&mut buf).await.expect_err("frag control error");
        assert_eq!(
            RelayStreamError::classify(&err),
            ConnectionErrorKind::WsProtocolError
        );
    }

    // 7. Close payload length 1 -> WsProtocolError (1002)
    {
        let (mut client_io, server_io) = duplex(1024);
        let mut ws =
            WsRelayStream::new(server_io, Vec::new(), 1024, 16 * 1024, ViewerTransport::Ws);
        let bad_close = make_client_frame(true, (false, false, false), 8, [1, 2, 3, 4], &[0x03]);
        client_io.write_all(&bad_close).await.expect("write");
        let mut buf = [0u8; 10];
        let err = ws.read(&mut buf).await.expect_err("bad close len");
        assert_eq!(
            RelayStreamError::classify(&err),
            ConnectionErrorKind::WsProtocolError
        );
    }

    // 8. Close with invalid UTF-8 reason -> WsProtocolError (1002)
    {
        let (mut client_io, server_io) = duplex(1024);
        let mut ws =
            WsRelayStream::new(server_io, Vec::new(), 1024, 16 * 1024, ViewerTransport::Ws);
        let bad_utf8_close = make_client_frame(
            true,
            (false, false, false),
            8,
            [1, 2, 3, 4],
            &[0x03, 0xE8, 0xFF, 0xFE], // 1000 + invalid UTF-8
        );
        client_io.write_all(&bad_utf8_close).await.expect("write");
        let mut buf = [0u8; 10];
        let err = ws.read(&mut buf).await.expect_err("bad close utf8");
        assert_eq!(
            RelayStreamError::classify(&err),
            ConnectionErrorKind::WsProtocolError
        );
    }
}

/// spec: websocket.md:WS-009 partial
#[tokio::test]
async fn answers_interleaved_ping_without_forwarding_control_payload() {
    let (mut client_io, server_io) = duplex(4096);
    let mut ws = WsRelayStream::new(
        server_io,
        Vec::new(),
        64 * 1024,
        16 * 1024,
        ViewerTransport::Ws,
    );

    let frag1 = make_client_frame(false, (false, false, false), 2, [1, 2, 3, 4], b"part1-");
    let ping = make_client_frame(true, (false, false, false), 9, [5, 6, 7, 8], b"echo-me");
    let frag2 = make_client_frame(true, (false, false, false), 0, [9, 0, 1, 2], b"part2");

    client_io.write_all(&frag1).await.expect("write frag1");
    client_io.write_all(&ping).await.expect("write ping");
    client_io.write_all(&frag2).await.expect("write frag2");

    let mut buf = vec![0u8; 11];
    ws.read_exact(&mut buf).await.expect("read data");
    assert_eq!(&buf, b"part1-part2");

    // Client receives Pong response
    let mut pong_buf = [0u8; 9]; // 2 bytes header + 7 bytes payload
    client_io
        .read_exact(&mut pong_buf)
        .await
        .expect("read pong");
    assert_eq!(pong_buf[0], 0x8A); // FIN = 1, opcode = Pong (10)
    assert_eq!(pong_buf[1], 7); // payload length 7
    assert_eq!(&pong_buf[2..], b"echo-me");
}

/// spec: websocket.md:WS-007 partial
#[tokio::test]
async fn bounds_outbound_messages_and_counts_only_written_payload() {
    let m = 30;
    let (mut client_io, server_io) = duplex(4096);
    let mut ws = WsRelayStream::new(server_io, Vec::new(), m, 16 * 1024, ViewerTransport::Ws);

    let large_write = vec![b'x'; 50];
    let written = ws.write(&large_write).await.expect("write");
    // Must be bounded to M = 30
    assert_eq!(written, 30);
    ws.flush().await.expect("flush");

    // Read on client side
    let mut frame_buf = vec![0u8; 32]; // 2 bytes header + 30 bytes payload
    client_io
        .read_exact(&mut frame_buf)
        .await
        .expect("read frame");
    assert_eq!(frame_buf[0], 0x82); // FIN = 1, opcode = Binary
    assert_eq!(frame_buf[1], 30); // length 30
    assert_eq!(&frame_buf[2..], &vec![b'x'; 30][..]);
}

#[tokio::test]
async fn close_mapping_for_every_relay_close_action() {
    let cases = [
        (
            RelayCloseAction::HandshakeInvalid,
            4008,
            "handshake_invalid",
        ),
        (
            RelayCloseAction::HandshakeTimeout,
            4009,
            "handshake_timeout",
        ),
        (RelayCloseAction::ModeDisabled, 1008, "mode_disabled"),
        (RelayCloseAction::RejectDuplicate, 4004, "duplicate_id"),
        (RelayCloseAction::WaitingExpired, 4001, "wait_timeout"),
        (RelayCloseAction::WaitingFull, 4010, "waiting_full"),
        (
            RelayCloseAction::TargetConnectFailed,
            4002,
            "target_connect_failed",
        ),
        (
            RelayCloseAction::TargetNotAllowed,
            4003,
            "target_not_allowed",
        ),
        (
            RelayCloseAction::TargetConnectionLimit,
            4011,
            "relay_connection_limit",
        ),
        (RelayCloseAction::WriteHalf, 1000, ""),
        (RelayCloseAction::PeerEof, 1000, ""),
        (
            RelayCloseAction::StreamError(ConnectionErrorKind::WsProtocolError),
            1002,
            "",
        ),
        (
            RelayCloseAction::StreamError(ConnectionErrorKind::WsTextMessage),
            1003,
            "",
        ),
        (
            RelayCloseAction::StreamError(ConnectionErrorKind::WsMessageTooLarge),
            1009,
            "",
        ),
        (
            RelayCloseAction::StreamError(ConnectionErrorKind::IoError),
            4007,
            "peer_closed",
        ),
        (RelayCloseAction::DeadConnection, 4006, "dead_connection"),
        (RelayCloseAction::AdminTerminated, 4005, "admin_terminated"),
        (RelayCloseAction::ServerShutdown, 1001, "server_shutdown"),
    ];

    for (action, expected_code, expected_reason) in cases {
        let (mut client_io, server_io) = duplex(1024);
        let mut ws = WsRelayStream::new(
            server_io,
            Vec::new(),
            64 * 1024,
            16 * 1024,
            ViewerTransport::Ws,
        );

        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        let res = Pin::new(&mut ws).poll_close(&mut cx, action);
        let terminal = !matches!(
            action,
            RelayCloseAction::WriteHalf | RelayCloseAction::PeerEof
        );
        assert_eq!(res.is_pending(), terminal, "action {action:?}");

        let expected_payload_len = 2 + expected_reason.len();
        let mut read_buf = vec![0u8; 2 + expected_payload_len];
        client_io
            .read_exact(&mut read_buf)
            .await
            .expect("read close frame");

        assert_eq!(read_buf[0], 0x88, "opcode Close");
        assert_eq!(read_buf[1] as usize, expected_payload_len);
        let code = u16::from_be_bytes([read_buf[2], read_buf[3]]);
        assert_eq!(code, expected_code, "action {action:?} code mismatch");
        assert_eq!(
            std::str::from_utf8(&read_buf[4..]).expect("utf8"),
            expected_reason,
            "action {action:?} reason mismatch"
        );
        client_io.shutdown().await.expect("client transport EOF");
        assert!(
            Pin::new(&mut ws).poll_close(&mut cx, action).is_ready(),
            "the caller owns the wait bound"
        );
    }

    // TlsFailed closes transport without sending Close frame
    {
        let (mut client_io, server_io) = duplex(1024);
        let mut ws = WsRelayStream::new(
            server_io,
            Vec::new(),
            64 * 1024,
            16 * 1024,
            ViewerTransport::Wss,
        );
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        let res = Pin::new(&mut ws).poll_close(
            &mut cx,
            RelayCloseAction::StreamError(ConnectionErrorKind::TlsFailed),
        );
        assert!(res.is_ready());
        let mut check_buf = [0u8; 1];
        let n = client_io.read(&mut check_buf).await.expect("read");
        assert_eq!(n, 0, "transport should be closed without close frame");
    }
}

#[tokio::test]
async fn rotates_the_mask_across_every_chunk_split() {
    let mask = [0xAA, 0xBB, 0xCC, 0xDD];
    let payload = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let frame = make_client_frame(true, (false, false, false), 2, mask, payload);

    for chunk_size in [1, 2, 3, 4, 5, 7, 13] {
        let (_client_io, server_io) = duplex(1024);
        let mut ws = WsRelayStream::new(
            server_io,
            frame.clone(),
            64 * 1024,
            16 * 1024,
            ViewerTransport::Ws,
        );

        let mut received = Vec::new();
        let mut buf = vec![0u8; chunk_size];
        while received.len() < payload.len() {
            let n = ws.read(&mut buf).await.expect("read chunk");
            if n == 0 {
                break;
            }
            received.extend_from_slice(&buf[..n]);
        }
        assert_eq!(received, payload, "failed for chunk_size {chunk_size}");
    }
}

#[tokio::test]
async fn keeps_upgrade_suffix_and_app_payload_under_one_b() {
    let b = 16 * 1024;
    let suffix = vec![0u8; 1023]; // maximum suffix
    let (_client_io, server_io) = duplex(1024);
    let ws = WsRelayStream::new(server_io, suffix, 64 * 1024, b, ViewerTransport::Ws);

    assert!(ws.holding_rx_bytes() <= b);
    assert_eq!(ws.holding_rx_bytes(), 1023);
}

#[tokio::test]
async fn yields_during_empty_message_and_control_floods() {
    let (mut client_io, server_io) = duplex(16 * 1024);
    let mut ws = WsRelayStream::new(
        server_io,
        Vec::new(),
        64 * 1024,
        16 * 1024,
        ViewerTransport::Ws,
    );

    // Flood 40 empty binary frames
    let empty_frame = make_client_frame(true, (false, false, false), 2, [1, 2, 3, 4], b"");
    for _ in 0..40 {
        client_io.write_all(&empty_frame).await.expect("write");
    }

    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    let mut buf = [0u8; 10];
    let mut read_buf = ReadBuf::new(&mut buf);

    // Must yield Pending after MAX_NON_DATA_FRAMES_PER_POLL (32) frames
    let poll_res = Pin::new(&mut ws).poll_read(&mut cx, &mut read_buf);
    assert!(
        poll_res.is_pending(),
        "flood must yield and not block task indefinitely"
    );
}

#[tokio::test]
async fn close_actions_are_idempotent() {
    let (mut client_io, server_io) = duplex(1024);
    let mut ws = WsRelayStream::new(
        server_io,
        Vec::new(),
        64 * 1024,
        16 * 1024,
        ViewerTransport::Ws,
    );

    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);

    let res1 = Pin::new(&mut ws).poll_close(&mut cx, RelayCloseAction::ServerShutdown);
    assert!(
        res1.is_pending(),
        "terminal disposal waits for transport EOF"
    );

    let res2 = Pin::new(&mut ws).poll_close(&mut cx, RelayCloseAction::ServerShutdown);
    assert!(res2.is_pending(), "repeat polls never emit another Close");
    drop(ws);

    // Only one Close frame sent
    let mut buf = vec![0u8; 128];
    let n = client_io.read(&mut buf).await.expect("read");
    let close_len = 2 + 2 + "server_shutdown".len();
    assert_eq!(n, close_len, "exactly one close frame must be written");

    // No second Close frame
    let n2 = client_io.read(&mut buf).await.expect("read 2");
    assert_eq!(n2, 0, "no duplicate close frame");
}

/// spec: websocket.md:WS-008 partial
#[tokio::test]
async fn terminal_close_discards_rejected_payload_before_releasing_the_transport() {
    let (mut client_io, server_io) = duplex(1024);
    let mut ws = WsRelayStream::new(
        server_io,
        Vec::new(),
        64 * 1024,
        16 * 1024,
        ViewerTransport::Ws,
    );
    let frame = make_client_frame(
        true,
        (false, false, false),
        1,
        [1, 2, 3, 4],
        b"rejected text payload remains unread in the socket",
    );
    client_io.write_all(&frame).await.expect("text frame");
    let error = ws.read(&mut [0; 64]).await.expect_err("text is rejected");
    assert_eq!(
        RelayStreamError::classify(&error),
        ConnectionErrorKind::WsTextMessage
    );
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    let action = RelayCloseAction::StreamError(ConnectionErrorKind::WsTextMessage);
    assert!(Pin::new(&mut ws).poll_close(&mut cx, action).is_pending());
    let mut response = Vec::new();
    client_io
        .read_to_end(&mut response)
        .await
        .expect("Close precedes transport write EOF");
    assert_eq!(response, [0x88, 2, 0x03, 0xeb]);
    client_io.shutdown().await.expect("client transport EOF");
    assert!(Pin::new(&mut ws).poll_close(&mut cx, action).is_ready());
}

/// spec: operations.md:OPS-009 partial
#[tokio::test]
async fn force_before_a_local_close_sends_1001_even_after_receiving_peer_close() {
    for code in [None, Some(3001_u16)] {
        let (mut client_io, server_io) = duplex(1024);
        let payload = code.map_or_else(Vec::new, |code| code.to_be_bytes().to_vec());
        let frame = make_client_frame(true, (false, false, false), 8, [1, 2, 3, 4], &payload);
        let mut ws =
            WsRelayStream::new(server_io, frame, 64 * 1024, 16 * 1024, ViewerTransport::Ws);
        assert_eq!(ws.read(&mut [0; 1]).await.expect("peer Close"), 0);
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        assert!(
            Pin::new(&mut ws)
                .poll_close(&mut cx, RelayCloseAction::ServerShutdown)
                .is_pending()
        );
        assert!(
            Pin::new(&mut ws)
                .poll_close(&mut cx, RelayCloseAction::PeerEof)
                .is_pending()
        );
        drop(ws);
        let mut bytes = Vec::new();
        client_io.read_to_end(&mut bytes).await.expect("one Close");
        let mut expected = vec![0x88, 17, 0x03, 0xe9];
        expected.extend_from_slice(b"server_shutdown");
        assert_eq!(bytes, expected, "Force sends exactly one Close 1001");
    }
}

/// spec: websocket.md:WS-017 partial
#[tokio::test]
async fn finishes_started_frames_before_replying_to_close_within_e() {
    let (mut client_io, server_io) = duplex(4096);
    // Partial write of 10 bytes max
    let partial_io = PartialStream::new(server_io, 10);
    let mut ws = WsRelayStream::new(
        partial_io,
        Vec::new(),
        64 * 1024,
        16 * 1024,
        ViewerTransport::Ws,
    );

    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);

    // 1. Start writing a 20-byte data frame. Due to partial write (12 bytes max),
    // 2 bytes header + 10 bytes payload are written, leaving 10 bytes pending.
    let payload = b"0123456789abcdefghij"; // 20 bytes
    let written = Pin::new(&mut ws).poll_write(&mut cx, payload);
    assert!(matches!(written, Poll::Ready(Ok(10))));
    assert_eq!(ws.pending_write_len(), 10);

    // 2. Client sends a Close frame with code 1000
    let close_frame =
        make_client_frame(true, (false, false, false), 8, [1, 2, 3, 4], &[0x03, 0xE8]);
    client_io
        .write_all(&close_frame)
        .await
        .expect("write close");

    // 3. Server reads client close frame -> returns EOF
    let mut read_buf = [0u8; 10];
    let mut rb = ReadBuf::new(&mut read_buf);
    let read_poll = Pin::new(&mut ws).poll_read(&mut cx, &mut rb);
    assert!(matches!(read_poll, Poll::Ready(Ok(()))));

    // 4. Server attempts poll_close while frame is still pending -> returns Pending
    let close_poll = Pin::new(&mut ws).poll_close(&mut cx, RelayCloseAction::PeerEof);
    assert!(
        close_poll.is_pending(),
        "poll_close must return Pending while outbound frame is pending"
    );

    // 5. Server completes the remaining 10 bytes of payload
    let written_rest = Pin::new(&mut ws).poll_write(&mut cx, &payload[10..]);
    assert!(matches!(written_rest, Poll::Ready(Ok(10))));
    assert_eq!(ws.pending_write_len(), 0);

    // 6. Now poll_close succeeds and writes the Close frame
    let close_poll2 = Pin::new(&mut ws).poll_close(&mut cx, RelayCloseAction::PeerEof);
    assert!(close_poll2.is_ready());

    // 7. Client reads: first the complete 20-byte data frame, then the Close frame
    let mut client_frame = vec![0u8; 2 + 20]; // 2 bytes header + 20 payload
    client_io
        .read_exact(&mut client_frame)
        .await
        .expect("read data frame");
    assert_eq!(client_frame[0], 0x82);
    assert_eq!(client_frame[1], 20);
    assert_eq!(&client_frame[2..], payload);

    let mut client_close = vec![0u8; 4]; // 2 bytes header + 2 bytes code (1000)
    client_io
        .read_exact(&mut client_close)
        .await
        .expect("read close frame");
    assert_eq!(client_close[0], 0x88);
    assert_eq!(client_close[1], 2);
    assert_eq!(u16::from_be_bytes([client_close[2], client_close[3]]), 1000);
}
