//! Real loopback checks for the exact-address target port and TCP relay stream.

use std::future::{Future, poll_fn};
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use socket2::SockRef;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, lookup_host};
use vnc_relay_adapters::net::TcpRelayStream;
use vnc_relay_adapters::target::TcpTargetConnector;
use vnc_relay_app::events::ConnectionErrorKind;
use vnc_relay_app::ports::{RelayCloseAction, RelayStream, TargetConnector, TargetError};
use vnc_relay_app::reload::{
    ConfigCandidate,
    snapshot::{ReloadCell, ReloadSnapshot},
};
use vnc_relay_app::settings::KeepaliveSettings;
use vnc_relay_domain::session::ViewerTransport;

const KEEPALIVE: KeepaliveSettings = KeepaliveSettings {
    tcp_idle: Duration::from_secs(70),
    tcp_interval: Duration::from_secs(11),
    tcp_probes: 7,
    ws_ping: Duration::from_secs(30),
    ws_pong_timeout: Duration::from_secs(30),
};

fn source(keepalive: KeepaliveSettings) -> Arc<ReloadCell<()>> {
    let mut settings = got(vnc_relay_adapters::config_file::parse(
        "[admin]\naccounts_file='accounts.toml'\n[management]\nlisten='127.0.0.1:8443'\n[status]\nlisten='127.0.0.1:9090'\n",
        std::path::Path::new("."),
    ));
    settings.keepalive = keepalive;
    Arc::new(ReloadCell::new(ReloadSnapshot::new(
        ConfigCandidate {
            settings,
            presence: Default::default(),
        },
        None,
        None,
    )))
}

fn got<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("unexpected error: {error:?}"))
}

async fn within<T>(future: impl Future<Output = T>) -> T {
    got(tokio::time::timeout(Duration::from_secs(5), future).await)
}

async fn connected_pair() -> (TcpRelayStream, TcpStream) {
    let listener = got(TcpListener::bind("127.0.0.1:0").await);
    let connector = TcpTargetConnector::new(source(KEEPALIVE));
    let stream = got(within(connector.connect(got(listener.local_addr()))).await);
    let (peer, _) = got(within(listener.accept()).await);
    (stream, peer)
}

/// spec: relay.md:RLY-009 partial
#[tokio::test]
async fn resolves_literal_ips_and_preserves_the_system_candidate_order() {
    let connector = TcpTargetConnector::new(source(KEEPALIVE));
    for text in ["127.0.0.1", "::1"] {
        assert_eq!(
            got(connector.resolve(text).await),
            vec![got(text.parse::<std::net::IpAddr>())]
        );
    }
    let expected: Vec<_> = got(within(lookup_host(("localhost", 0))).await)
        .map(|address| address.ip())
        .collect();
    assert_eq!(got(within(connector.resolve("localhost")).await), expected);
    assert_eq!(
        connector.resolve("invalid\0hostname").await,
        Err(TargetError::ResolveFailed)
    );
}

/// spec: relay.md:RLY-009 partial
#[tokio::test]
async fn connects_only_to_the_supplied_ip_and_port_and_distinguishes_refusal() {
    let listener = got(TcpListener::bind("127.0.0.1:0").await);
    let address = got(listener.local_addr());
    let connector = TcpTargetConnector::new(source(KEEPALIVE));
    let stream = got(within(connector.connect(address)).await);
    assert_eq!(stream.transport(), ViewerTransport::Tcp);
    let stream = stream.into_inner();
    assert_eq!(got(stream.peer_addr()), address);
    let (peer, peer_address) = got(within(listener.accept()).await);
    assert_eq!(peer_address, got(stream.local_addr()));
    drop(peer);
    drop(stream);
    drop(listener);
    assert_eq!(
        within(connector.connect(address)).await.map(|_| ()),
        Err(TargetError::ConnectRefused)
    );
}

#[tokio::test]
async fn applies_keepalive_to_the_connected_target_before_returning_it() {
    let (stream, _peer) = connected_pair().await;
    let stream = stream.into_inner();
    let socket = SockRef::from(&stream);
    assert!(got(socket.keepalive()));
    // socket2 exposes the detailed getters on Linux, but not on Windows.
    #[cfg(target_os = "linux")]
    {
        assert_eq!(got(socket.tcp_keepalive_time()), KEEPALIVE.tcp_idle);
        assert_eq!(got(socket.tcp_keepalive_interval()), KEEPALIVE.tcp_interval);
        assert_eq!(got(socket.tcp_keepalive_retries()), KEEPALIVE.tcp_probes);
    }
}

/// spec: operations.md:OPS-050 partial
#[tokio::test]
async fn reloaded_keepalive_is_used_only_for_new_target_sockets() {
    let listener = got(TcpListener::bind("127.0.0.1:0").await);
    let source = source(KEEPALIVE);
    let connector = TcpTargetConnector::new(source.clone());
    let first = got(within(connector.connect(got(listener.local_addr()))).await).into_inner();
    let (_first_peer, _) = got(within(listener.accept()).await);
    let mut next = source.current().settings.as_ref().clone();
    next.keepalive.tcp_idle = Duration::from_secs(90);
    next.keepalive.tcp_interval = Duration::from_secs(13);
    next.keepalive.tcp_probes = 9;
    source.replace(ReloadSnapshot::new(
        ConfigCandidate {
            settings: next,
            presence: Default::default(),
        },
        None,
        None,
    ));
    let second = got(within(connector.connect(got(listener.local_addr()))).await).into_inner();
    let (_second_peer, _) = got(within(listener.accept()).await);
    assert!(got(SockRef::from(&first).keepalive()));
    assert!(got(SockRef::from(&second).keepalive()));
    #[cfg(target_os = "linux")]
    {
        let old = SockRef::from(&first);
        let new = SockRef::from(&second);
        assert_eq!(got(old.tcp_keepalive_time()), KEEPALIVE.tcp_idle);
        assert_eq!(got(old.tcp_keepalive_interval()), KEEPALIVE.tcp_interval);
        assert_eq!(got(old.tcp_keepalive_retries()), KEEPALIVE.tcp_probes);
        assert_eq!(got(new.tcp_keepalive_time()), Duration::from_secs(90));
        assert_eq!(got(new.tcp_keepalive_interval()), Duration::from_secs(13));
        assert_eq!(got(new.tcp_keepalive_retries()), 9);
    }
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn closes_the_new_target_socket_when_keepalive_cannot_be_applied() {
    let listener = got(TcpListener::bind("127.0.0.1:0").await);
    let connector = TcpTargetConnector::new(source(KeepaliveSettings {
        // Linux rejects TCP_KEEPIDLE=0; no external network fault is needed.
        tcp_idle: Duration::ZERO,
        ..KEEPALIVE
    }));
    assert_eq!(
        within(connector.connect(got(listener.local_addr())))
            .await
            .map(|_| ()),
        Err(TargetError::KeepaliveFailed)
    );
    let (mut peer, _) = got(within(listener.accept()).await);
    let mut byte = [0; 1];
    assert_eq!(got(within(peer.read(&mut byte)).await), 0);
}

/// spec: relay.md:RLY-019 partial
#[tokio::test]
async fn write_half_close_delivers_payload_and_keeps_reverse_reads_open() {
    let (mut stream, mut peer) = connected_pair().await;
    got(stream.write_all(b"viewer payload").await);
    got(stream.flush().await);
    got(within(poll_fn(|cx| {
        Pin::new(&mut stream).poll_close(cx, RelayCloseAction::WriteHalf)
    }))
    .await);
    let mut delivered = Vec::new();
    got(within(peer.read_to_end(&mut delivered)).await);
    assert_eq!(delivered, b"viewer payload");
    got(peer.write_all(b"remaining reverse payload").await);
    got(peer.shutdown().await);
    let mut reverse = Vec::new();
    got(within(stream.read_to_end(&mut reverse)).await);
    assert_eq!(reverse, b"remaining reverse payload");
}

#[tokio::test]
async fn every_terminal_close_action_shuts_down_the_socket_before_owner_drop() {
    for action in [
        RelayCloseAction::HandshakeInvalid,
        RelayCloseAction::HandshakeTimeout,
        RelayCloseAction::ModeDisabled,
        RelayCloseAction::RejectDuplicate,
        RelayCloseAction::WaitingExpired,
        RelayCloseAction::WaitingFull,
        RelayCloseAction::TargetConnectFailed,
        RelayCloseAction::TargetNotAllowed,
        RelayCloseAction::TargetConnectionLimit,
        RelayCloseAction::PeerEof,
        RelayCloseAction::StreamError(ConnectionErrorKind::IoError),
        RelayCloseAction::StreamError(ConnectionErrorKind::TlsFailed),
        RelayCloseAction::StreamError(ConnectionErrorKind::WsProtocolError),
        RelayCloseAction::StreamError(ConnectionErrorKind::WsTextMessage),
        RelayCloseAction::StreamError(ConnectionErrorKind::WsMessageTooLarge),
        RelayCloseAction::DeadConnection,
        RelayCloseAction::AdminTerminated,
        RelayCloseAction::ServerShutdown,
    ] {
        let (mut stream, mut peer) = connected_pair().await;
        got(within(poll_fn(|cx| Pin::new(&mut stream).poll_close(cx, action))).await);
        let mut byte = [0; 1];
        assert_eq!(got(within(peer.read(&mut byte)).await), 0, "{action:?}");
        drop(stream);
    }
}

#[tokio::test]
async fn wraps_an_already_admitted_socket_without_changing_its_identity() {
    let listener = got(TcpListener::bind("127.0.0.1:0").await);
    let address: SocketAddr = got(listener.local_addr());
    let socket = got(within(TcpStream::connect(address)).await);
    let original_local_address = got(socket.local_addr());
    let stream = TcpRelayStream::new(socket);
    assert_eq!(stream.transport(), ViewerTransport::Tcp);
    let socket = stream.into_inner();
    assert_eq!(got(socket.local_addr()), original_local_address);
    assert_eq!(got(socket.peer_addr()), address);
}
