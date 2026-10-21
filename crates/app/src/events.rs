//! Operation log events (ops-info-v1 §1.5) and their `level` (§1.3).
//! Design: docs/architectures/ops-log.md, docs/architectures/tcp-entry-point.md,
//! docs/architectures/status-listener.md.

use std::fmt;
use std::net::SocketAddr;

use vnc_relay_domain::conn_id::ConnId;
use vnc_relay_domain::entry_point::EntryPoint;
use vnc_relay_domain::peer_ip::PeerIp;

use crate::settings::ConfigWarning;

/// Value of `log_contract` in `server_starting` (ops-info-v1 §1.5).
pub const LOG_CONTRACT: &str = "ops-info/1.2.0";

/// `level` values of ops-info-v1 §1.3. v1 has no other value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warn,
    Error,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

/// `phase` values of ops-info-v1 §1.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Startup,
    Reload,
}

impl Phase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::Reload => "reload",
        }
    }
}

/// `stage` values of `connection_rejected` and `connection_aborted`
/// (ops-info-v1 §1.6). Tasks that add a stage add its value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Accept,
    InitialExchange,
    Tls,
    Upgrade,
}

impl Stage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accept => "accept",
            Self::InitialExchange => "initial_exchange",
            Self::Tls => "tls",
            Self::Upgrade => "upgrade",
        }
    }
}

/// `reason` values of `connection_rejected` (ops-info-v1 §1.6). A reason
/// that comes from a limit carries the limit, written as `limit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    /// The source IP is not in the allow list of the entry point.
    IpNotAllowed,
    /// Every place of the relay connection limit L is held.
    RelayConnectionLimit { limit: u32 },
    /// Every place of the management and status connection limit Lm is
    /// held.
    AdminConnectionLimit { limit: u32 },
    /// The client offered no TLS 1.2 or later. The connection closes with
    /// no plaintext fallback (OPS-036).
    TlsVersionUnsupported,
    /// The request is not a WebSocket upgrade request, or its head is
    /// malformed (websocket-relay-v1 §1.1).
    InvalidUpgrade,
    /// The Origin header is not in the allowed list (websocket-relay-v1
    /// §1.3).
    OriginNotAllowed,
    /// The client offered subprotocols and none of them is `binary`
    /// (websocket-relay-v1 §1.2).
    SubprotocolNotSupported,
}

impl RejectReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IpNotAllowed => "ip_not_allowed",
            Self::RelayConnectionLimit { .. } => "relay_connection_limit",
            Self::AdminConnectionLimit { .. } => "admin_connection_limit",
            Self::TlsVersionUnsupported => "tls_version_unsupported",
            Self::InvalidUpgrade => "invalid_upgrade",
            Self::OriginNotAllowed => "origin_not_allowed",
            Self::SubprotocolNotSupported => "subprotocol_not_supported",
        }
    }

    /// The `limit` field: the configured limit that refused the connection.
    pub fn limit(self) -> Option<u32> {
        match self {
            Self::IpNotAllowed
            | Self::TlsVersionUnsupported
            | Self::InvalidUpgrade
            | Self::OriginNotAllowed
            | Self::SubprotocolNotSupported => None,
            Self::RelayConnectionLimit { limit } | Self::AdminConnectionLimit { limit } => {
                Some(limit)
            }
        }
    }
}

/// `reason` values of `connection_aborted` (ops-info-v1 §1.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbortReason {
    /// The initial exchange did not end within H.
    HandshakeTimeout,
    /// The TLS handshake or the upgrade request did not end within U.
    TlsUpgradeTimeout,
    /// The peer closed the connection before the initial exchange ended.
    PeerEof,
    /// A network error ended the connection; written with `error_kind`.
    ConnectionError(ConnectionErrorKind),
    /// The server is stopping and ends unfinished work.
    ServerShutdown,
}

impl AbortReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HandshakeTimeout => "handshake_timeout",
            Self::TlsUpgradeTimeout => "tls_upgrade_timeout",
            Self::PeerEof => "peer_eof",
            Self::ConnectionError(_) => "connection_error",
            Self::ServerShutdown => "server_shutdown",
        }
    }

    /// The `error_kind` field, present for `connection_error` only.
    pub fn error_kind(self) -> Option<ConnectionErrorKind> {
        match self {
            Self::ConnectionError(kind) => Some(kind),
            Self::HandshakeTimeout
            | Self::PeerEof
            | Self::ServerShutdown
            | Self::TlsUpgradeTimeout => None,
        }
    }

    /// `level` of the `connection_aborted` line with this reason.
    fn level(self) -> Level {
        match self {
            Self::PeerEof | Self::ServerShutdown => Level::Info,
            Self::HandshakeTimeout | Self::TlsUpgradeTimeout | Self::ConnectionError(_) => {
                Level::Warn
            }
        }
    }
}

/// `error_kind` values of ops-info-v1 §1.7. Tasks that meet another kind
/// add its value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionErrorKind {
    /// A socket read or write failed, or the peer reset the connection.
    IoError,
    /// The TLS handshake failed in the negotiation, for example an alert
    /// from the client or no shared cipher suite.
    TlsFailed,
}

impl ConnectionErrorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IoError => "io_error",
            Self::TlsFailed => "tls_failed",
        }
    }
}

/// One operation log event. Each variant is one event of ops-info-v1 §1.5.
/// Fields carry no secret, no raw relay ID, and no config value other than
/// the ones the contract names (§1.11). Feature tasks add variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// First event of the start: the config is valid and the log is open.
    ServerStarting,
    /// A config warning (config-v1 §2.2). The event name is the warning code.
    ConfigWarning {
        warning: ConfigWarning,
        phase: Phase,
    },
    /// One line per diagnostic that stopped the start (ops-info-v1 §1.6).
    /// `detail` holds no config value, path, or file content.
    StartupFailed {
        reason: &'static str,
        config_key: Option<String>,
        listener: Option<&'static str>,
        detail: Option<String>,
    },
    /// The server entered the ready state: every required listener is
    /// bound. Each item is the `listener` name and the bound address.
    ServerReady {
        listeners: Vec<(&'static str, SocketAddr)>,
    },
    /// A relay listener accepted a connection that passed the source IP
    /// check and took a place of L. `role` comes from the entry point.
    ConnectionAccepted {
        conn_id: ConnId,
        entry_point: EntryPoint,
        peer_ip: PeerIp,
    },
    /// The server refused a connection. At `Stage::Accept` the connection
    /// has no `conn_id` yet. `listener` is the ops-info-v1 §1.5 name of the
    /// listener that refused it.
    ConnectionRejected {
        listener: &'static str,
        stage: Stage,
        reason: RejectReason,
        peer_ip: PeerIp,
    },
    /// The server refused an upgrade request on an accepted WS or WSS
    /// connection (websocket-relay-v1 §1.1, §1.3). The event name is
    /// `connection_rejected`, and `stage` is always `upgrade`.
    UpgradeRejected {
        listener: &'static str,
        conn_id: ConnId,
        reason: RejectReason,
        peer_ip: PeerIp,
        /// The raw Origin header value; present for `origin_not_allowed`.
        origin: Option<String>,
    },
    /// An accepted connection ended before it waited or joined a session.
    ConnectionAborted {
        conn_id: ConnId,
        entry_point: EntryPoint,
        stage: Stage,
        reason: AbortReason,
        peer_ip: PeerIp,
        elapsed_ms: u64,
    },
    /// Last event: resources are released and the process ends.
    ServerStopped,
}

impl Event {
    /// The `event` field.
    pub fn name(&self) -> &'static str {
        match self {
            Self::ServerStarting => "server_starting",
            Self::ServerReady { .. } => "server_ready",
            Self::ConfigWarning { warning, .. } => warning.code(),
            Self::StartupFailed { .. } => "startup_failed",
            Self::ConnectionAccepted { .. } => "connection_accepted",
            Self::ConnectionRejected { .. } | Self::UpgradeRejected { .. } => "connection_rejected",
            Self::ConnectionAborted { .. } => "connection_aborted",
            Self::ServerStopped => "server_stopped",
        }
    }

    /// The `level` field of ops-info-v1 §1.5.
    pub fn level(&self) -> Level {
        match self {
            Self::ServerStarting
            | Self::ServerReady { .. }
            | Self::ServerStopped
            | Self::ConnectionAccepted { .. } => Level::Info,
            Self::ConfigWarning { .. }
            | Self::ConnectionRejected { .. }
            | Self::UpgradeRejected { .. } => Level::Warn,
            Self::ConnectionAborted { reason, .. } => reason.level(),
            Self::StartupFailed { .. } => Level::Error,
        }
    }
}

/// `relay_id_ref` of ops-info-v1 §1.9: the first 8 bytes of the keyed hash
/// of a relay ID as 16 lowercase hex digits. Only a hash prefix makes one,
/// so a raw relay ID cannot reach a log line through this type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RelayIdRef(String);

impl RelayIdRef {
    pub fn from_hash_prefix(prefix: [u8; 8]) -> Self {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut text = String::with_capacity(16);
        for byte in prefix {
            text.push(char::from(HEX[usize::from(byte >> 4)]));
            text.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        Self(text)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RelayIdRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
