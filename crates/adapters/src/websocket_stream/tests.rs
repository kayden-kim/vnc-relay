use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf, duplex};
use vnc_relay_app::events::ConnectionErrorKind;
use vnc_relay_app::ports::{RelayCloseAction, RelayStream, RelayStreamError};
use vnc_relay_domain::session::ViewerTransport;

use super::WsRelayStream;

const KEEPALIVE: vnc_relay_app::settings::KeepaliveSettings =
    vnc_relay_app::settings::KeepaliveSettings {
        tcp_idle: std::time::Duration::from_secs(60),
        tcp_interval: std::time::Duration::from_secs(10),
        tcp_probes: 6,
        ws_ping: std::time::Duration::from_secs(30),
        ws_pong_timeout: std::time::Duration::from_secs(30),
    };

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
        KEEPALIVE,
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
        KEEPALIVE,
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
        KEEPALIVE,
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
    let mut ws = WsRelayStream::new(
        server_io,
        Vec::new(),
        m_limit,
        b_limit,
        ViewerTransport::Ws,
        KEEPALIVE,
    );

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
    let mut ws = WsRelayStream::new(
        server_io,
        Vec::new(),
        m,
        16 * 1024,
        ViewerTransport::Ws,
        KEEPALIVE,
    );

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
        let mut ws = WsRelayStream::new(
            server_io,
            Vec::new(),
            1024,
            16 * 1024,
            ViewerTransport::Ws,
            KEEPALIVE,
        );
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
        let mut ws = WsRelayStream::new(
            server_io,
            Vec::new(),
            1024,
            16 * 1024,
            ViewerTransport::Ws,
            KEEPALIVE,
        );
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
        let mut ws = WsRelayStream::new(
            server_io,
            Vec::new(),
            1024,
            16 * 1024,
            ViewerTransport::Ws,
            KEEPALIVE,
        );
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
        let mut ws = WsRelayStream::new(
            server_io,
            Vec::new(),
            1024,
            16 * 1024,
            ViewerTransport::Ws,
            KEEPALIVE,
        );
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
        let mut ws = WsRelayStream::new(
            server_io,
            Vec::new(),
            1024,
            16 * 1024,
            ViewerTransport::Ws,
            KEEPALIVE,
        );
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
        let mut ws = WsRelayStream::new(
            server_io,
            Vec::new(),
            1024,
            16 * 1024,
            ViewerTransport::Ws,
            KEEPALIVE,
        );
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
        let mut ws = WsRelayStream::new(
            server_io,
            Vec::new(),
            1024,
            16 * 1024,
            ViewerTransport::Ws,
            KEEPALIVE,
        );
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
        let mut ws = WsRelayStream::new(
            server_io,
            Vec::new(),
            1024,
            16 * 1024,
            ViewerTransport::Ws,
            KEEPALIVE,
        );
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
        KEEPALIVE,
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
    let mut ws = WsRelayStream::new(
        server_io,
        Vec::new(),
        m,
        16 * 1024,
        ViewerTransport::Ws,
        KEEPALIVE,
    );

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
            KEEPALIVE,
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
            KEEPALIVE,
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
            KEEPALIVE,
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
    let ws = WsRelayStream::new(
        server_io,
        suffix,
        64 * 1024,
        b,
        ViewerTransport::Ws,
        KEEPALIVE,
    );

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
        KEEPALIVE,
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
        KEEPALIVE,
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
        KEEPALIVE,
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

/// spec: operations.md:OPS-042 partial
#[tokio::test]
async fn shutdown_after_received_close_preserves_the_peer_code() {
    for code in [None, Some(3001_u16)] {
        let (mut client_io, server_io) = duplex(1024);
        let payload = code.map_or_else(Vec::new, |code| code.to_be_bytes().to_vec());
        let frame = make_client_frame(true, (false, false, false), 8, [1, 2, 3, 4], &payload);
        let mut ws = WsRelayStream::new(
            server_io,
            frame,
            64 * 1024,
            16 * 1024,
            ViewerTransport::Ws,
            KEEPALIVE,
        );
        assert_eq!(ws.read(&mut [0; 1]).await.expect("peer Close"), 0);
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        assert!(
            Pin::new(&mut ws)
                .poll_close(&mut cx, RelayCloseAction::ServerShutdown)
                .is_ready()
        );
        assert!(
            Pin::new(&mut ws)
                .poll_close(&mut cx, RelayCloseAction::PeerEof)
                .is_ready()
        );
        drop(ws);
        let mut bytes = Vec::new();
        client_io.read_to_end(&mut bytes).await.expect("one Close");
        let expected_code = code.unwrap_or(1000).to_be_bytes();
        let expected = [0x88, 2, expected_code[0], expected_code[1]];
        assert_eq!(bytes, expected, "Force preserves the peer Close echo");
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
        KEEPALIVE,
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

#[derive(Default)]
struct WriteGate {
    allowance: usize,
    waker: Option<Waker>,
    error: Option<std::io::ErrorKind>,
}

struct ControlledIo {
    inner: tokio::io::DuplexStream,
    gate: std::sync::Arc<std::sync::Mutex<WriteGate>>,
    read_chunk: usize,
}

impl ControlledIo {
    fn new(
        inner: tokio::io::DuplexStream,
        allowance: usize,
    ) -> (Self, std::sync::Arc<std::sync::Mutex<WriteGate>>) {
        let gate = std::sync::Arc::new(std::sync::Mutex::new(WriteGate {
            allowance,
            ..WriteGate::default()
        }));
        (
            Self {
                inner,
                gate: gate.clone(),
                read_chunk: usize::MAX,
            },
            gate,
        )
    }
}

fn grant_writes(gate: &std::sync::Mutex<WriteGate>, allowance: usize) {
    let wake = {
        let mut state = gate.lock().expect("gate");
        state.allowance = allowance;
        state.waker.take()
    };
    if let Some(waker) = wake {
        waker.wake();
    }
}

impl AsyncRead for ControlledIo {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        let want = buf.remaining().min(this.read_chunk);
        let mut limited = ReadBuf::new(buf.initialize_unfilled_to(want));
        match Pin::new(&mut this.inner).poll_read(cx, &mut limited) {
            Poll::Ready(Ok(())) => {
                let count = limited.filled().len();
                buf.advance(count);
                Poll::Ready(Ok(()))
            }
            result => result,
        }
    }
}

impl AsyncWrite for ControlledIo {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        let mut gate = this.gate.lock().expect("gate");
        if let Some(error) = gate.error {
            return if error == std::io::ErrorKind::WriteZero {
                Poll::Ready(Ok(0))
            } else {
                Poll::Ready(Err(error.into()))
            };
        }
        if gate.allowance == 0 {
            gate.waker = Some(cx.waker().clone());
            return Poll::Pending;
        }
        let count = buf.len().min(gate.allowance);
        match Pin::new(&mut this.inner).poll_write(cx, &buf[..count]) {
            Poll::Ready(Ok(written)) => {
                gate.allowance -= written;
                Poll::Ready(Ok(written))
            }
            result => result,
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}

fn poll_idle_read<S: AsyncRead + AsyncWrite + Unpin + Send>(
    ws: &mut WsRelayStream<S>,
    cx: &mut Context<'_>,
) -> Poll<std::io::Result<()>> {
    Pin::new(ws).poll_read(cx, &mut ReadBuf::new(&mut [0; 32]))
}

/// spec: websocket.md:WS-009 partial
#[tokio::test]
async fn reads_only_the_remaining_control_payload_before_the_next_frame() {
    for read_chunk in [1, 2, 125] {
        let (mut peer, io) = duplex(4096);
        let ping = make_client_frame(true, (false, false, false), 9, [1, 2, 3, 4], b"0123456789");
        let (mut io, _) = ControlledIo::new(io, usize::MAX);
        io.read_chunk = read_chunk;
        let mut ws = WsRelayStream::new(
            io,
            ping[..14].to_vec(),
            65536,
            16384,
            ViewerTransport::Ws,
            KEEPALIVE,
        );
        peer.write_all(&ping[14..]).await.expect("Ping remainder");
        peer.write_all(&client_binary_frame(b"next"))
            .await
            .expect("next frame");
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        let mut payload = Vec::new();
        while payload.len() < 4 {
            let mut bytes = [0; 4];
            let mut buf = ReadBuf::new(&mut bytes);
            assert!(matches!(
                Pin::new(&mut ws).poll_read(&mut cx, &mut buf),
                Poll::Ready(Ok(()))
            ));
            assert!(!buf.filled().is_empty());
            payload.extend_from_slice(buf.filled());
        }
        assert_eq!(payload, b"next");
        let mut pong = [0; 12];
        peer.read_exact(&mut pong).await.expect("Pong");
        assert_eq!(&pong, b"\x8a\x0a0123456789");
    }
}

/// spec: websocket.md:WS-009 partial
#[tokio::test]
async fn emits_pong_only_after_partial_data_headers_and_payloads_finish() {
    for allowance in [1, 7] {
        let (mut peer, io) = duplex(4096);
        let (io, gate) = ControlledIo::new(io, allowance);
        let mut ws =
            WsRelayStream::new(io, Vec::new(), 65536, 16384, ViewerTransport::Ws, KEEPALIVE);
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        let payload = b"0123456789abcdefghij";
        let written = match Pin::new(&mut ws).poll_write(&mut cx, payload) {
            Poll::Ready(Ok(count)) => count,
            Poll::Pending => 0,
            other => panic!("unexpected write: {other:?}"),
        };
        peer.write_all(&make_client_frame(
            true,
            (false, false, false),
            9,
            [0; 4],
            b"p",
        ))
        .await
        .expect("Ping");
        assert!(poll_idle_read(&mut ws, &mut cx).is_pending());
        grant_writes(&gate, 4096);
        assert!(matches!(
            Pin::new(&mut ws).poll_flush(&mut cx),
            Poll::Ready(Ok(()))
        ));
        assert!(poll_idle_read(&mut ws, &mut cx).is_pending());
        assert!(
            matches!(Pin::new(&mut ws).poll_write(&mut cx, &payload[written..]), Poll::Ready(Ok(count)) if count == payload.len() - written)
        );
        assert!(poll_idle_read(&mut ws, &mut cx).is_pending());
        drop(ws);
        let mut wire = Vec::new();
        peer.read_to_end(&mut wire).await.expect("wire");
        let mut expected = b"\x82\x140123456789abcdefghij".to_vec();
        expected.extend_from_slice(b"\x8a\x01p");
        assert_eq!(wire, expected);
    }
}

/// spec: websocket.md:WS-009 partial
#[tokio::test]
async fn preserves_an_active_pong_and_coalesces_only_unsent_pongs() {
    for allowance in [0, 1] {
        let (mut peer, io) = duplex(4096);
        let (io, gate) = ControlledIo::new(io, allowance);
        let mut ws =
            WsRelayStream::new(io, Vec::new(), 65536, 16384, ViewerTransport::Ws, KEEPALIVE);
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        for payload in [b"first".as_slice(), b"latest"] {
            peer.write_all(&make_client_frame(
                true,
                (false, false, false),
                9,
                [0; 4],
                payload,
            ))
            .await
            .expect("Ping");
            assert!(poll_idle_read(&mut ws, &mut cx).is_pending());
        }
        grant_writes(&gate, 4096);
        assert!(poll_idle_read(&mut ws, &mut cx).is_pending());
        drop(ws);
        let mut wire = Vec::new();
        peer.read_to_end(&mut wire).await.expect("Pongs");
        let expected = if allowance == 0 {
            b"\x8a\x06latest".as_slice()
        } else {
            b"\x8a\x05first\x8a\x06latest".as_slice()
        };
        assert_eq!(wire, expected);
    }
}

#[tokio::test]
async fn finishes_active_controls_before_normal_and_terminal_close() {
    for terminal in [false, true] {
        for pong_first in [false, true] {
            let (mut peer, io) = duplex(4096);
            let (io, gate) = ControlledIo::new(io, 1);
            let mut ws =
                WsRelayStream::new(io, Vec::new(), 65536, 16384, ViewerTransport::Ws, KEEPALIVE);
            let waker = noop_waker();
            let mut cx = Context::from_waker(&waker);
            let action = if terminal {
                RelayCloseAction::ServerShutdown
            } else {
                RelayCloseAction::WriteHalf
            };
            if !pong_first {
                assert!(Pin::new(&mut ws).poll_close(&mut cx, action).is_pending());
            }
            peer.write_all(&make_client_frame(
                true,
                (false, false, false),
                9,
                [0; 4],
                b"p",
            ))
            .await
            .expect("Ping");
            assert!(poll_idle_read(&mut ws, &mut cx).is_pending());
            assert!(Pin::new(&mut ws).poll_close(&mut cx, action).is_pending());
            grant_writes(&gate, 4096);
            assert!(matches!(
                Pin::new(&mut ws).poll_flush(&mut cx),
                Poll::Ready(Ok(()))
            ));
            let close = Pin::new(&mut ws).poll_close(&mut cx, action);
            assert_eq!(
                close.is_pending(),
                terminal,
                "terminal disposal waits for EOF"
            );
            if terminal {
                peer.shutdown().await.expect("transport EOF");
                assert!(matches!(
                    Pin::new(&mut ws).poll_close(&mut cx, action),
                    Poll::Ready(Ok(()))
                ));
            }
            drop(ws);
            let mut wire = Vec::new();
            peer.read_to_end(&mut wire).await.expect("controls");
            let mut expected = if pong_first {
                b"\x8a\x01p".to_vec()
            } else {
                Vec::new()
            };
            if terminal {
                expected.extend_from_slice(b"\x88\x11\x03\xe9server_shutdown");
            } else {
                expected.extend_from_slice(b"\x88\x02\x03\xe8");
            }
            assert_eq!(wire, expected);
        }
    }
}

#[derive(Default)]
struct WakeCount(std::sync::atomic::AtomicUsize);

impl std::task::Wake for WakeCount {
    fn wake(self: std::sync::Arc<Self>) {
        self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// spec: websocket.md:WS-009 partial
#[tokio::test]
async fn retries_an_idle_pong_on_write_wake_without_application_writes() {
    let (mut peer, io) = duplex(4096);
    let (io, gate) = ControlledIo::new(io, 0);
    let mut ws = WsRelayStream::new(io, Vec::new(), 65536, 16384, ViewerTransport::Ws, KEEPALIVE);
    let wakes = std::sync::Arc::new(WakeCount::default());
    let waker = Waker::from(wakes.clone());
    let mut cx = Context::from_waker(&waker);
    peer.write_all(&make_client_frame(
        true,
        (false, false, false),
        9,
        [0; 4],
        b"p",
    ))
    .await
    .expect("Ping");
    assert!(poll_idle_read(&mut ws, &mut cx).is_pending());
    grant_writes(&gate, 4096);
    assert!(wakes.0.load(std::sync::atomic::Ordering::Relaxed) > 0);
    assert!(poll_idle_read(&mut ws, &mut cx).is_pending());
    let mut pong = [0; 3];
    peer.read_exact(&mut pong)
        .await
        .expect("Pong without inbound activity");
    assert_eq!(&pong, b"\x8a\x01p");
}

#[tokio::test]
async fn propagates_pong_write_errors_from_reads() {
    for kind in [
        std::io::ErrorKind::WriteZero,
        std::io::ErrorKind::BrokenPipe,
    ] {
        let (mut peer, io) = duplex(4096);
        let (io, gate) = ControlledIo::new(io, 4096);
        gate.lock().expect("gate").error = Some(kind);
        let mut ws =
            WsRelayStream::new(io, Vec::new(), 65536, 16384, ViewerTransport::Ws, KEEPALIVE);
        peer.write_all(&make_client_frame(
            true,
            (false, false, false),
            9,
            [0; 4],
            b"",
        ))
        .await
        .expect("Ping");
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        assert!(
            matches!(poll_idle_read(&mut ws, &mut cx), Poll::Ready(Err(error)) if error.kind() == kind)
        );
    }
}

#[tokio::test]
async fn flushes_idle_pongs_through_a_buffered_transport() {
    let (mut peer, io) = duplex(4096);
    let mut ws = WsRelayStream::new(
        tokio::io::BufWriter::new(io),
        Vec::new(),
        65536,
        16384,
        ViewerTransport::Ws,
        KEEPALIVE,
    );
    peer.write_all(&make_client_frame(
        true,
        (false, false, false),
        9,
        [0; 4],
        b"",
    ))
    .await
    .expect("Ping");
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    assert!(poll_idle_read(&mut ws, &mut cx).is_pending());
    let mut pong = [0; 2];
    assert!(matches!(
        Pin::new(&mut peer).poll_read(&mut cx, &mut ReadBuf::new(&mut pong)),
        Poll::Ready(Ok(()))
    ));
    assert_eq!(&pong, b"\x8a\x00");
}

#[tokio::test]
async fn rejects_new_data_after_sending_close() {
    let (_peer, io) = duplex(4096);
    let mut ws = WsRelayStream::new(io, Vec::new(), 65536, 16384, ViewerTransport::Ws, KEEPALIVE);
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    assert!(matches!(
        Pin::new(&mut ws).poll_close(&mut cx, RelayCloseAction::WriteHalf),
        Poll::Ready(Ok(()))
    ));
    assert!(
        matches!(Pin::new(&mut ws).poll_write(&mut cx, b"forbidden"), Poll::Ready(Err(error)) if error.kind() == std::io::ErrorKind::BrokenPipe)
    );
}

#[tokio::test]
async fn terminal_close_completes_on_peer_close_without_transport_eof() {
    for action in [
        RelayCloseAction::AdminTerminated,
        RelayCloseAction::DeadConnection,
        RelayCloseAction::StreamError(ConnectionErrorKind::IoError),
        RelayCloseAction::ServerShutdown,
    ] {
        let (mut peer, io) = duplex(4096);
        let mut ws =
            WsRelayStream::new(io, Vec::new(), 65536, 16384, ViewerTransport::Ws, KEEPALIVE);
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        assert!(Pin::new(&mut ws).poll_close(&mut cx, action).is_pending());
        // Discard well-formed payloads without forwarding them, then recognize
        // the valid Close reply while the peer still keeps TCP writes open.
        peer.write_all(&client_binary_frame(b"discarded after terminal close"))
            .await
            .expect("discarded data");
        peer.write_all(&make_client_frame(
            true,
            (false, false, false),
            8,
            [0; 4],
            &[3, 232],
        ))
        .await
        .expect("Close reply");
        assert!(matches!(
            Pin::new(&mut ws).poll_close(&mut cx, action),
            Poll::Ready(Ok(()))
        ));
        let mut wire = Vec::new();
        peer.read_to_end(&mut wire)
            .await
            .expect("server closes TCP first");
        assert_eq!(wire.first(), Some(&0x88));
        assert_eq!(wire.len(), 2 + usize::from(wire[1]));
        assert!(ws.decoder.peer_closed);
    }
}

#[tokio::test]
async fn valid_terminal_disposal_yields_during_a_data_flood() {
    let (mut peer, io) = duplex(65536);
    let mut ws = WsRelayStream::new(io, Vec::new(), 65536, 16384, ViewerTransport::Ws, KEEPALIVE);
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    let frame = client_binary_frame(&vec![7; 1024]);
    for _ in 0..40 {
        peer.write_all(&frame).await.expect("flood data");
    }
    assert!(
        Pin::new(&mut ws)
            .poll_close(&mut cx, RelayCloseAction::AdminTerminated)
            .is_pending()
    );
    assert!(!ws.transport_eof);
    assert!(!ws.decoder.peer_closed);
    peer.write_all(&make_client_frame(
        true,
        (false, false, false),
        8,
        [0; 4],
        &[3, 232],
    ))
    .await
    .expect("Close reply after flood");
    let mut completed = false;
    for _ in 0..8 {
        if matches!(
            Pin::new(&mut ws).poll_close(&mut cx, RelayCloseAction::AdminTerminated),
            Poll::Ready(Ok(()))
        ) {
            completed = true;
            break;
        }
    }
    assert!(
        completed,
        "bounded disposal eventually reaches the Close reply"
    );
}

#[tokio::test]
async fn rejected_frame_disposal_waits_for_transport_eof_not_apparent_close_bytes() {
    let (mut peer, io) = duplex(4096);
    let mut ws = WsRelayStream::new(io, Vec::new(), 65536, 16384, ViewerTransport::Ws, KEEPALIVE);
    let apparent_close = make_client_frame(true, (false, false, false), 8, [0; 4], &[3, 232]);
    peer.write_all(&make_client_frame(
        true,
        (false, false, false),
        1,
        [0; 4],
        &apparent_close,
    ))
    .await
    .expect("rejected text containing apparent Close");
    let error = ws.read(&mut [0; 1]).await.expect_err("text");
    assert_eq!(
        RelayStreamError::classify(&error),
        ConnectionErrorKind::WsTextMessage
    );
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    let action = RelayCloseAction::StreamError(ConnectionErrorKind::WsTextMessage);
    assert!(Pin::new(&mut ws).poll_close(&mut cx, action).is_pending());
    assert!(!ws.decoder.peer_closed);
    peer.shutdown().await.expect("transport EOF");
    assert!(matches!(
        Pin::new(&mut ws).poll_close(&mut cx, action),
        Poll::Ready(Ok(()))
    ));
}

/// spec: operations.md:OPS-042 partial
#[tokio::test]
async fn shutdown_does_not_replace_an_echo_with_a_partial_header() {
    let (mut peer, io) = duplex(4096);
    let (io, gate) = ControlledIo::new(io, 1);
    let close = make_client_frame(
        true,
        (false, false, false),
        8,
        [0; 4],
        &3001_u16.to_be_bytes(),
    );
    let mut ws = WsRelayStream::new(io, close, 65536, 16384, ViewerTransport::Ws, KEEPALIVE);
    assert_eq!(ws.read(&mut [0; 1]).await.expect("peer Close"), 0);
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    assert!(
        Pin::new(&mut ws)
            .poll_close(&mut cx, RelayCloseAction::PeerEof)
            .is_pending()
    );
    grant_writes(&gate, 4096);
    assert!(matches!(
        Pin::new(&mut ws).poll_close(&mut cx, RelayCloseAction::ServerShutdown),
        Poll::Ready(Ok(()))
    ));
    drop(ws);
    let mut wire = Vec::new();
    peer.read_to_end(&mut wire).await.expect("one echo");
    assert_eq!(wire, [0x88, 2, 0x0b, 0xb9]);
}

fn kw_settings() -> vnc_relay_app::settings::KeepaliveSettings {
    vnc_relay_app::settings::KeepaliveSettings {
        ws_ping: std::time::Duration::from_secs(5),
        ws_pong_timeout: std::time::Duration::from_secs(5),
        ..KEEPALIVE
    }
}

#[tokio::test(start_paused = true)]
async fn kw_expiry_is_codeless_but_other_endpoint_dead_connection_sends_4006() {
    let (mut peer, io) = duplex(4096);
    let mut ws = WsRelayStream::new(
        io,
        Vec::new(),
        65536,
        16384,
        ViewerTransport::Ws,
        kw_settings(),
    );
    let mut cx = Context::from_waker(Waker::noop());
    assert!(Pin::new(&mut ws).poll_maintenance(&mut cx).is_pending());
    tokio::time::advance(std::time::Duration::from_secs(5)).await;
    assert!(Pin::new(&mut ws).poll_maintenance(&mut cx).is_pending());
    let mut ping = [0; 2];
    peer.read_exact(&mut ping)
        .await
        .unwrap_or_else(|error| panic!("server Ping: {error}"));
    assert_eq!(ping, [0x89, 0]);
    tokio::time::advance(std::time::Duration::from_secs(5)).await;
    let error = match Pin::new(&mut ws).poll_maintenance(&mut cx) {
        Poll::Ready(Err(error)) => error,
        other => panic!("Pong deadline must expire: {other:?}"),
    };
    assert!(RelayStreamError::is_dead_connection(&error));
    assert!(matches!(
        Pin::new(&mut ws).poll_close(&mut cx, RelayCloseAction::DeadConnection),
        Poll::Ready(Ok(()))
    ));
    assert_eq!(
        peer.read(&mut ping)
            .await
            .unwrap_or_else(|error| panic!("code-less EOF: {error}")),
        0
    );

    let (mut peer, io) = duplex(4096);
    let mut ws = WsRelayStream::new(
        io,
        Vec::new(),
        65536,
        16384,
        ViewerTransport::Ws,
        kw_settings(),
    );
    assert!(
        Pin::new(&mut ws)
            .poll_close(&mut cx, RelayCloseAction::DeadConnection)
            .is_pending()
    );
    let mut close = [0; 19];
    peer.read_exact(&mut close)
        .await
        .unwrap_or_else(|error| panic!("other dead endpoint Close: {error}"));
    assert_eq!(&close, b"\x88\x11\x0f\xa6dead_connection");
}

#[tokio::test(start_paused = true)]
async fn kw_keeps_deadlines_running_when_b_hides_a_pong() {
    let (mut peer, io) = duplex(65536);
    let mut ws = WsRelayStream::new(
        io,
        Vec::new(),
        65536,
        16384,
        ViewerTransport::Ws,
        kw_settings(),
    );
    let mut cx = Context::from_waker(Waker::noop());
    tokio::time::advance(std::time::Duration::from_secs(5)).await;
    assert!(Pin::new(&mut ws).poll_maintenance(&mut cx).is_pending());
    peer.write_all(&client_binary_frame(&vec![7; 32768]))
        .await
        .unwrap_or_else(|error| panic!("large data before Pong: {error}"));
    peer.write_all(&make_client_frame(
        true,
        (false, false, false),
        10,
        [1; 4],
        b"",
    ))
    .await
    .unwrap_or_else(|error| panic!("hidden Pong: {error}"));
    let mut app_b = vec![0; 16384];
    ws.read_exact(&mut app_b)
        .await
        .unwrap_or_else(|error| panic!("fill B: {error}"));
    // B is full: the owner stops payload reads, but keeps polling maintenance.
    tokio::time::advance(std::time::Duration::from_secs(5)).await;
    match Pin::new(&mut ws).poll_maintenance(&mut cx) {
        Poll::Ready(Err(error)) => assert!(RelayStreamError::is_dead_connection(&error)),
        other => panic!("hidden Pong must not pause its deadline: {other:?}"),
    }
    assert!(ws.holding_rx_bytes() <= 16384);
}

#[tokio::test(start_paused = true)]
async fn kw_waits_for_partial_data_and_flushes_ping_without_interleaving_controls() {
    for allowance in [1, 7] {
        let (mut peer, io) = duplex(4096);
        let (io, gate) = ControlledIo::new(io, allowance);
        let mut ws = WsRelayStream::new(
            io,
            Vec::new(),
            65536,
            16384,
            ViewerTransport::Ws,
            kw_settings(),
        );
        let mut cx = Context::from_waker(Waker::noop());
        let data = b"0123456789";
        let first = match Pin::new(&mut ws).poll_write(&mut cx, data) {
            Poll::Ready(Ok(count)) => count,
            Poll::Pending => 0,
            other => panic!("partial data: {other:?}"),
        };
        tokio::time::advance(std::time::Duration::from_secs(5)).await;
        assert!(Pin::new(&mut ws).poll_maintenance(&mut cx).is_pending());
        grant_writes(&gate, 4096);
        ws.write_all(&data[first..])
            .await
            .unwrap_or_else(|error| panic!("finish data: {error}"));
        assert!(Pin::new(&mut ws).poll_maintenance(&mut cx).is_pending());
        let mut wire = [0; 14];
        peer.read_exact(&mut wire)
            .await
            .unwrap_or_else(|error| panic!("frame then Ping: {error}"));
        assert_eq!(&wire, b"\x82\x0a0123456789\x89\x00");
    }
    let (mut peer, io) = duplex(4096);
    let (io, gate) = ControlledIo::new(io, 1);
    let mut ws = WsRelayStream::new(
        tokio::io::BufWriter::with_capacity(1, io),
        Vec::new(),
        65536,
        16384,
        ViewerTransport::Ws,
        kw_settings(),
    );
    let mut cx = Context::from_waker(Waker::noop());
    tokio::time::advance(std::time::Duration::from_secs(5)).await;
    assert!(Pin::new(&mut ws).poll_maintenance(&mut cx).is_pending());
    peer.write_all(&make_client_frame(
        true,
        (false, false, false),
        9,
        [1; 4],
        b"p",
    ))
    .await
    .unwrap_or_else(|error| panic!("client Ping: {error}"));
    assert!(poll_idle_read(&mut ws, &mut cx).is_pending());
    grant_writes(&gate, 4096);
    assert!(Pin::new(&mut ws).poll_maintenance(&mut cx).is_pending());
    ws.flush()
        .await
        .unwrap_or_else(|error| panic!("flush controls: {error}"));
    let mut wire = [0; 5];
    peer.read_exact(&mut wire)
        .await
        .unwrap_or_else(|error| panic!("Ping then Pong: {error}"));
    assert_eq!(&wire, b"\x89\x00\x8a\x01p");
}

#[tokio::test(start_paused = true)]
async fn kw_finishes_a_started_ping_before_echoing_a_received_close() {
    let (mut peer, io) = duplex(4096);
    let (io, gate) = ControlledIo::new(io, 1);
    let mut ws = WsRelayStream::new(
        io,
        Vec::new(),
        65536,
        16384,
        ViewerTransport::Ws,
        kw_settings(),
    );
    let mut cx = Context::from_waker(Waker::noop());
    tokio::time::advance(std::time::Duration::from_secs(5)).await;
    assert!(Pin::new(&mut ws).poll_maintenance(&mut cx).is_pending());
    peer.write_all(&make_client_frame(
        true,
        (false, false, false),
        8,
        [1; 4],
        &[3, 232],
    ))
    .await
    .unwrap_or_else(|error| panic!("client Close: {error}"));
    assert!(matches!(
        poll_idle_read(&mut ws, &mut cx),
        Poll::Ready(Ok(()))
    ));
    grant_writes(&gate, 4096);
    assert!(matches!(
        Pin::new(&mut ws).poll_close(&mut cx, RelayCloseAction::PeerEof),
        Poll::Ready(Ok(()))
    ));
    let mut wire = [0; 6];
    peer.read_exact(&mut wire)
        .await
        .unwrap_or_else(|error| panic!("Ping before Close: {error}"));
    assert_eq!(&wire, b"\x89\x00\x88\x02\x03\xe8");
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn classifies_underlying_ws_and_wss_socket_timeouts_without_changing_other_errors() {
    struct FailedSocket(std::io::ErrorKind);

    impl AsyncRead for FailedSocket {
        fn poll_read(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
            _: &mut ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            Poll::Ready(Err(self.0.into()))
        }
    }

    impl AsyncWrite for FailedSocket {
        fn poll_write(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
            _: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            Poll::Ready(Err(self.0.into()))
        }
        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Err(self.0.into()))
        }
        fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Err(self.0.into()))
        }
    }

    fn assert_failure<T: std::fmt::Debug>(
        result: Poll<std::io::Result<T>>,
        kind: std::io::ErrorKind,
    ) {
        let error = match result {
            Poll::Ready(Err(error)) => error,
            other => panic!("injected transport error disappeared: {other:?}"),
        };
        let is_dead = matches!(
            kind,
            std::io::ErrorKind::TimedOut
                | std::io::ErrorKind::HostUnreachable
                | std::io::ErrorKind::NetworkUnreachable
        );
        assert_eq!(RelayStreamError::is_dead_connection(&error), is_dead);
        assert_eq!(
            RelayStreamError::classify(&error),
            ConnectionErrorKind::IoError
        );
        if !is_dead {
            assert_eq!(error.kind(), kind);
        }
    }

    for transport in [ViewerTransport::Ws, ViewerTransport::Wss] {
        for kind in [
            std::io::ErrorKind::TimedOut,
            std::io::ErrorKind::HostUnreachable,
            std::io::ErrorKind::NetworkUnreachable,
            std::io::ErrorKind::ConnectionReset,
            std::io::ErrorKind::BrokenPipe,
        ] {
            let mut ws = WsRelayStream::new(
                FailedSocket(kind),
                Vec::new(),
                65536,
                16384,
                transport,
                KEEPALIVE,
            );
            let mut cx = Context::from_waker(Waker::noop());
            let mut bytes = [0; 1];
            assert_failure(
                Pin::new(&mut ws).poll_read(&mut cx, &mut ReadBuf::new(&mut bytes)),
                kind,
            );
            assert_failure(Pin::new(&mut ws).poll_write(&mut cx, b"x"), kind);
            assert_failure(Pin::new(&mut ws).poll_flush(&mut cx), kind);
            assert_failure(Pin::new(&mut ws).poll_shutdown(&mut cx), kind);
        }
    }
}
