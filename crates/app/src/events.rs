//! Operation log events (ops-info-v1 §1.5) and their `level` (§1.3).
//! Design: docs/architectures/ops-log.md, docs/architectures/tcp-entry-point.md,
//! docs/architectures/status-listener.md, docs/architectures/initial-exchange.md.

use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::time::SystemTime;

use vnc_relay_domain::account::AccountName;
use vnc_relay_domain::conn_id::ConnId;
use vnc_relay_domain::entry_point::{EntryPoint, Role};
use vnc_relay_domain::handshake::Mode;
use vnc_relay_domain::login_limit::LimitScope;
use vnc_relay_domain::peer_ip::PeerIp;
use vnc_relay_domain::session::{CloseCause, SessionId, Side, ViewerTransport};

use crate::reload::{ChangeSource, WatchFailure};
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
    Register,
    TargetConnect,
}

impl Stage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accept => "accept",
            Self::InitialExchange => "initial_exchange",
            Self::Tls => "tls",
            Self::Upgrade => "upgrade",
            Self::Register => "register",
            Self::TargetConnect => "target_connect",
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
    /// The initial request has an invalid format (repeater-handshake-v1 §2).
    HandshakeInvalid,
    /// The initial request selected a disabled mode (repeater-handshake-v1 §2).
    ModeDisabled,
    /// The target name, IP, or port is outside the allowed policy.
    TargetNotAllowed,
    /// Every waiting place Lw is held.
    WaitingFull { limit: u32 },
    /// This relay ID and role are already occupied.
    DuplicateId,
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
            Self::HandshakeInvalid => "handshake_invalid",
            Self::ModeDisabled => "mode_disabled",
            Self::TargetNotAllowed => "target_not_allowed",
            Self::WaitingFull { .. } => "waiting_full",
            Self::DuplicateId => "duplicate_id",
        }
    }

    /// The `limit` field: the configured limit that refused the connection.
    pub fn limit(self) -> Option<u32> {
        match self {
            Self::IpNotAllowed
            | Self::TlsVersionUnsupported
            | Self::InvalidUpgrade
            | Self::OriginNotAllowed
            | Self::SubprotocolNotSupported
            | Self::HandshakeInvalid
            | Self::ModeDisabled
            | Self::TargetNotAllowed
            | Self::DuplicateId => None,
            Self::RelayConnectionLimit { limit }
            | Self::AdminConnectionLimit { limit }
            | Self::WaitingFull { limit } => Some(limit),
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
    /// A liveness check found an unresponsive connection.
    DeadConnection,
}

impl AbortReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HandshakeTimeout => "handshake_timeout",
            Self::TlsUpgradeTimeout => "tls_upgrade_timeout",
            Self::PeerEof => "peer_eof",
            Self::ConnectionError(_) => "connection_error",
            Self::ServerShutdown => "server_shutdown",
            Self::DeadConnection => "dead_connection",
        }
    }

    /// The `error_kind` field, present for `connection_error` only.
    pub fn error_kind(self) -> Option<ConnectionErrorKind> {
        match self {
            Self::ConnectionError(kind) => Some(kind),
            Self::HandshakeTimeout
            | Self::PeerEof
            | Self::ServerShutdown
            | Self::DeadConnection
            | Self::TlsUpgradeTimeout => None,
        }
    }

    /// `level` of the `connection_aborted` line with this reason.
    fn level(self) -> Level {
        match self {
            Self::PeerEof | Self::ServerShutdown => Level::Info,
            Self::HandshakeTimeout
            | Self::TlsUpgradeTimeout
            | Self::ConnectionError(_)
            | Self::DeadConnection => Level::Warn,
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
    /// WebSocket frame order or control frame is invalid.
    WsProtocolError,
    /// A text data message is not relay binary payload.
    WsTextMessage,
    /// A logical WebSocket message exceeded M.
    WsMessageTooLarge,
}

impl ConnectionErrorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IoError => "io_error",
            Self::TlsFailed => "tls_failed",
            Self::WsProtocolError => "ws_protocol_error",
            Self::WsTextMessage => "ws_text_message",
            Self::WsMessageTooLarge => "ws_message_too_large",
        }
    }
}

/// Target connection failure codes (ops-info-v1 §1.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetFailureReason {
    ResolveFailed,
    ConnectRefused,
    ConnectTimeout,
    ConnectError,
}

impl TargetFailureReason {
    /// Returns the contract `reason` value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ResolveFailed => "resolve_failed",
            Self::ConnectRefused => "connect_refused",
            Self::ConnectTimeout => "connect_timeout",
            Self::ConnectError => "connect_error",
        }
    }
}

/// Removal reasons other than waiting expiry (ops-info-v1 §1.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitingRemovalReason {
    PeerEof,
    ConnectionError(ConnectionErrorKind),
    DeadConnection,
    ServerShutdown,
}

impl WaitingRemovalReason {
    /// Returns the contract `reason` value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PeerEof => "peer_eof",
            Self::ConnectionError(_) => "connection_error",
            Self::DeadConnection => "dead_connection",
            Self::ServerShutdown => "server_shutdown",
        }
    }

    /// Only connection errors carry an `error_kind`.
    pub fn error_kind(self) -> Option<ConnectionErrorKind> {
        match self {
            Self::ConnectionError(kind) => Some(kind),
            Self::PeerEof | Self::DeadConnection | Self::ServerShutdown => None,
        }
    }

    fn level(self) -> Level {
        match self {
            Self::PeerEof | Self::ServerShutdown => Level::Info,
            Self::ConnectionError(_) | Self::DeadConnection => Level::Warn,
        }
    }
}

/// The `reason` of `admin_login_failed` (ops-info-v1 §1.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginFailReason {
    InvalidCredentials,
    Limited,
}

impl LoginFailReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidCredentials => "invalid_credentials",
            Self::Limited => "limited",
        }
    }
}

/// Outcome of an authenticated administrator termination (ops-info-v1 §1.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminTerminateResult {
    Terminated,
    AlreadyClosed,
    NotFound,
    Failed,
}

impl AdminTerminateResult {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Terminated => "terminated",
            Self::AlreadyClosed => "already_closed",
            Self::NotFound => "not_found",
            Self::Failed => "failed",
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
    /// A config warning (config-v2 §2.2). The event name is the warning code.
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
    /// The server refused an accepted connection during the initial exchange
    /// (ops-info-v1 §1.5-1.6). The event name is `connection_rejected`, and
    /// `stage` is always `initial_exchange`. No request or raw relay ID is held.
    HandshakeRejected {
        conn_id: ConnId,
        entry_point: EntryPoint,
        peer_ip: PeerIp,
        reason: RejectReason,
        /// The caller must set this for `ModeDisabled`; omit it when unknown.
        mode: Option<Mode>,
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
    /// Mode I policy or target socket budget refusal (ops-info-v1 §1.6).
    TargetRejected {
        conn_id: ConnId,
        entry_point: EntryPoint,
        peer_ip: PeerIp,
        reason: RejectReason,
        target_host: String,
        target_port: u16,
        target_ip: Option<IpAddr>,
    },
    /// A Mode II registration refusal (ops-info-v1 §1.6).
    RegistrationRejected {
        conn_id: ConnId,
        entry_point: EntryPoint,
        peer_ip: PeerIp,
        reason: RejectReason,
        relay_id_ref: RelayIdRef,
    },
    /// An unfinished relay connection with a known mode (ops-info-v1 §1.5).
    RelayAborted {
        conn_id: ConnId,
        entry_point: EntryPoint,
        stage: Stage,
        reason: AbortReason,
        peer_ip: PeerIp,
        elapsed_ms: u64,
        mode: Mode,
    },
    /// Mode I DNS or target connection failure; never carries `error_kind`.
    TargetConnectFailed {
        conn_id: ConnId,
        entry_point: EntryPoint,
        viewer_transport: ViewerTransport,
        peer_ip: PeerIp,
        target_host: String,
        target_port: u16,
        target_ip: Option<IpAddr>,
        elapsed_ms: u64,
        reason: TargetFailureReason,
    },
    /// A Mode II connection entered the waiting table (ops-info-v1 §1.5).
    WaitingRegistered {
        conn_id: ConnId,
        entry_point: EntryPoint,
        peer_ip: PeerIp,
        relay_id_ref: RelayIdRef,
    },
    /// W expired; no additional aborted or removed event is required.
    WaitingExpired {
        conn_id: ConnId,
        role: Role,
        relay_id_ref: RelayIdRef,
        waited_ms: u64,
    },
    /// A waiting connection ended without matching or expiring.
    WaitingRemoved {
        conn_id: ConnId,
        role: Role,
        relay_id_ref: RelayIdRef,
        waited_ms: u64,
        reason: WaitingRemovalReason,
    },
    /// An active session was registered (ops-info-v1 §1.5).
    /// Only fields belonging to `mode` are serialized.
    SessionStarted {
        session_id: SessionId,
        mode: Mode,
        viewer_transport: ViewerTransport,
        viewer_conn_id: ConnId,
        viewer_peer_ip: PeerIp,
        server_conn_id: Option<ConnId>,
        server_peer_ip: Option<PeerIp>,
        relay_id_ref: Option<RelayIdRef>,
        waited_ms: Option<u64>,
        target_ip: Option<IpAddr>,
        target_port: Option<u16>,
        target_host: Option<String>,
    },
    /// Finalized session summary, after sockets and permits are released.
    /// Cause-specific fields and the Mode II reference follow ops-info-v1 §1.6.
    SessionClosed {
        session_id: SessionId,
        mode: Mode,
        viewer_transport: ViewerTransport,
        started_at: SystemTime,
        duration_ms: u64,
        bytes_viewer_to_target: u64,
        bytes_target_to_viewer: u64,
        close_cause: CloseCause,
        side: Option<Side>,
        error_kind: Option<ConnectionErrorKind>,
        relay_id_ref: Option<RelayIdRef>,
        admin_account: Option<AccountName>,
    },
    /// Recorded after final cleanup, without leaking absent request IDs.
    AdminTerminate {
        admin_account: AccountName,
        peer_ip: PeerIp,
        session_id: Option<SessionId>,
        result: AdminTerminateResult,
    },
    /// The server entered shutdown grace (ops-info-v1 §1.5).
    ShutdownStarted {
        grace_secs: u64,
        active_sessions: usize,
    },
    /// Remaining sessions are about to be forcibly closed (ops-info-v1 §1.5).
    ShutdownForced { remaining_sessions: usize },
    /// An administrator logged in and got a session (ops-info-v1 §1.5,
    /// management.md MGT-017). The login event carries no session ID.
    AdminLoginSucceeded {
        admin_account: AccountName,
        peer_ip: PeerIp,
    },
    /// A login attempt was refused (ops-info-v1 §1.5, MGT-018, MGT-022).
    /// `scope` is set for `Limited`. `admin_account` is set only when the
    /// typed name is a real account (ops-info-v1 §1.8).
    AdminLoginFailed {
        reason: LoginFailReason,
        scope: Option<LimitScope>,
        admin_account: Option<AccountName>,
        peer_ip: PeerIp,
    },
    /// A login lock started. One event for each lock (ops-info-v1 §1.5).
    AdminLoginLimited {
        scope: LimitScope,
        lockout_secs: u64,
        admin_account: Option<AccountName>,
        peer_ip: PeerIp,
    },
    /// A change was detected in the config file or a TLS file (ops-info-v1 §1.5).
    ReloadChangeDetected { source: ChangeSource },
    /// Reload targets were applied; `applied` lists the changed config keys (ops-info-v1 §1.5).
    ReloadApplied { applied: Vec<&'static str> },
    /// A candidate was refused; the running values are kept (ops-info-v1 §1.5).
    ReloadRejected {
        reason: String,
        config_key: Option<String>,
        detail: Option<String>,
    },
    /// A key that cannot change at runtime differs. Values are never written (ops-info-v1 §1.5).
    ReloadRestartRequired { config_keys: Vec<String> },
    /// The config file or a TLS file cannot be watched (ops-info-v1 §1.5).
    ReloadWatchFailed {
        reason: WatchFailure,
        detail: Option<String>,
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
            Self::ConnectionRejected { .. }
            | Self::UpgradeRejected { .. }
            | Self::HandshakeRejected { .. }
            | Self::TargetRejected { .. }
            | Self::RegistrationRejected { .. } => "connection_rejected",
            Self::ConnectionAborted { .. } | Self::RelayAborted { .. } => "connection_aborted",
            Self::TargetConnectFailed { .. } => "target_connect_failed",
            Self::WaitingRegistered { .. } => "waiting_registered",
            Self::WaitingExpired { .. } => "waiting_expired",
            Self::WaitingRemoved { .. } => "waiting_removed",
            Self::SessionStarted { .. } => "session_started",
            Self::SessionClosed { .. } => "session_closed",
            Self::AdminTerminate { .. } => "admin_terminate",
            Self::ShutdownStarted { .. } => "shutdown_started",
            Self::ShutdownForced { .. } => "shutdown_forced",
            Self::AdminLoginSucceeded { .. } => "admin_login_succeeded",
            Self::AdminLoginFailed { .. } => "admin_login_failed",
            Self::AdminLoginLimited { .. } => "admin_login_limited",
            Self::ReloadChangeDetected { .. } => "reload_change_detected",
            Self::ReloadApplied { .. } => "reload_applied",
            Self::ReloadRejected { .. } => "reload_rejected",
            Self::ReloadRestartRequired { .. } => "reload_restart_required",
            Self::ReloadWatchFailed { .. } => "reload_watch_failed",
            Self::ServerStopped => "server_stopped",
        }
    }

    /// The `level` field of ops-info-v1 §1.5.
    pub fn level(&self) -> Level {
        match self {
            Self::ServerStarting
            | Self::ServerReady { .. }
            | Self::ServerStopped
            | Self::ConnectionAccepted { .. }
            | Self::WaitingRegistered { .. }
            | Self::WaitingExpired { .. }
            | Self::SessionStarted { .. } => Level::Info,
            Self::ConfigWarning { .. }
            | Self::ConnectionRejected { .. }
            | Self::UpgradeRejected { .. }
            | Self::HandshakeRejected { .. }
            | Self::TargetRejected { .. }
            | Self::RegistrationRejected { .. }
            | Self::TargetConnectFailed { .. }
            | Self::ShutdownForced { .. } => Level::Warn,
            Self::ConnectionAborted { reason, .. } | Self::RelayAborted { reason, .. } => {
                reason.level()
            }
            Self::WaitingRemoved { reason, .. } => reason.level(),
            Self::SessionClosed { close_cause, .. } => match close_cause {
                CloseCause::PeerEof | CloseCause::AdminTerminated => Level::Info,
                CloseCause::ConnectionError
                | CloseCause::DeadConnection
                | CloseCause::CloseTimeout
                | CloseCause::ServerShutdown
                | CloseCause::InternalError => Level::Warn,
            },
            Self::ShutdownStarted { .. } => Level::Info,
            Self::AdminTerminate { result, .. } => {
                if *result == AdminTerminateResult::Failed {
                    Level::Error
                } else {
                    Level::Info
                }
            }
            Self::AdminLoginSucceeded { .. } => Level::Info,
            Self::AdminLoginFailed { .. } | Self::AdminLoginLimited { .. } => Level::Warn,
            Self::StartupFailed { .. } => Level::Error,
            Self::ReloadChangeDetected { .. } | Self::ReloadApplied { .. } => Level::Info,
            Self::ReloadRejected { .. } | Self::ReloadRestartRequired { .. } => Level::Warn,
            Self::ReloadWatchFailed { .. } => Level::Error,
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
