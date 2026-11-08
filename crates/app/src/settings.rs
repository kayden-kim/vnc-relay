//! Effective settings validated against config-v4 §1, with stable diagnostics.

use std::fmt;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use vnc_relay_domain::account::Argon2Params;
use vnc_relay_domain::allow_list::{AllowList, IpItem};
use vnc_relay_domain::entry_point::{EntryPoint, Listener, Role};
use vnc_relay_domain::origin::Origin;
use vnc_relay_domain::target_policy::{TargetItem, TargetPolicy};

/// Validated settings. Omitted keys hold their defaults. Equality compares
/// values only, so notation, comments, key order, and explicit defaults do not
/// matter (config-v4 §1.8 rule 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub admin: AdminSettings,
    pub relay: RelaySettings,
    pub mode1: Mode1Settings,
    pub mode2: Mode2Settings,
    pub limits: LimitSettings,
    pub timeouts: TimeoutSettings,
    pub keepalive: KeepaliveSettings,
    pub management: ManagementSettings,
    pub dashboard: DashboardSettings,
    pub recent_closed: RecentClosedSettings,
    pub recent_logs: RecentLogsSettings,
    pub status: StatusSettings,
    pub log: LogSettings,
    pub reload: ReloadSettings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminSettings {
    pub accounts_file: PathBuf,
    pub argon2: Argon2Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Argon2Settings {
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

impl Argon2Settings {
    /// Parameters of new hashes made by `admin add` and `admin passwd`.
    pub fn params(self) -> Argon2Params {
        Argon2Params {
            memory_kib: self.memory_kib,
            iterations: self.iterations,
            parallelism: self.parallelism,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelaySettings {
    pub tcp_viewer: EntryPointSettings,
    pub tcp_server: EntryPointSettings,
    pub ws: WsListenerSettings,
    pub wss: WsListenerSettings,
}

impl RelaySettings {
    /// Whether a role is enabled at startup.
    pub fn enabled(&self, entry: EntryPoint) -> bool {
        match entry {
            EntryPoint::TcpViewer => self.tcp_viewer.enabled,
            EntryPoint::TcpServer => self.tcp_server.enabled,
            _ => self.ws_role(entry).is_some_and(|role| role.enabled),
        }
    }

    /// Allow list of one role; absent only when omitted for a disabled role.
    pub fn allow(&self, entry: EntryPoint) -> Option<&AllowList> {
        match entry {
            EntryPoint::TcpViewer => self.tcp_viewer.allow.as_ref(),
            EntryPoint::TcpServer => self.tcp_server.allow.as_ref(),
            _ => self.ws_role(entry).and_then(|role| role.allow.as_ref()),
        }
    }

    /// Role settings of a WS or WSS entry point.
    pub fn ws_role(&self, entry: EntryPoint) -> Option<&WsRoleSettings> {
        let listener = self.websocket(entry.listener())?;
        Some(match entry.role() {
            Role::Viewer => &listener.viewer,
            Role::Server => &listener.server,
        })
    }

    /// Shared WS or WSS listener settings.
    pub fn websocket(&self, listener: Listener) -> Option<&WsListenerSettings> {
        match listener {
            Listener::Ws => Some(&self.ws),
            Listener::Wss => Some(&self.wss),
            Listener::TcpViewer | Listener::TcpServer => None,
        }
    }

    /// Whether this socket has at least one enabled role.
    pub fn listener_enabled(&self, listener: Listener) -> bool {
        match listener {
            Listener::TcpViewer => self.tcp_viewer.enabled,
            Listener::TcpServer => self.tcp_server.enabled,
            Listener::Ws => self.ws.enabled(),
            Listener::Wss => self.wss.enabled(),
        }
    }

    /// Address of the shared socket, including any TCP default port.
    pub fn listen(&self, listener: Listener) -> Option<SocketAddr> {
        match listener {
            Listener::TcpViewer => self.tcp_viewer.listen,
            Listener::TcpServer => self.tcp_server.listen,
            Listener::Ws => self.ws.listen,
            Listener::Wss => self.wss.listen,
        }
    }
}

/// TCP entry settings. A disabled entry retains any explicitly written values.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EntryPointSettings {
    pub enabled: bool,
    pub listen: Option<SocketAddr>,
    pub allow: Option<AllowList>,
}

/// A shared WS or WSS socket with independent role policies (config-v4 §1.2).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WsListenerSettings {
    pub listen: Option<SocketAddr>,
    pub trusted_proxies: AllowList,
    /// WSS only; the WS config table rejects TLS keys.
    pub tls: Option<TlsFiles>,
    pub viewer: WsRoleSettings,
    pub server: WsRoleSettings,
}

impl WsListenerSettings {
    /// A socket opens when either role is enabled.
    pub fn enabled(&self) -> bool {
        self.viewer.enabled || self.server.enabled
    }
}

/// Per-role WS policy; the listener and TLS files are shared.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WsRoleSettings {
    pub enabled: bool,
    pub allow: Option<AllowList>,
    pub allowed_origins: Vec<Origin>,
}

/// Certificate chain and private key paths, resolved against the config
/// file directory. File contents are checked by the TLS adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsFiles {
    pub cert_file: PathBuf,
    pub key_file: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Mode1Settings {
    pub enabled: bool,
    /// Present when `enabled` is true.
    pub allow_targets: Option<TargetPolicy>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mode2Settings {
    pub enabled: bool,
}

/// `[limits]`: L, Lm, Lw, B, M.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LimitSettings {
    pub relay_connections: u32,
    pub admin_connections: u32,
    pub waiting: u32,
    pub buffer_bytes: u32,
    pub ws_message_bytes: u32,
}

/// `[timeouts]`: H, U, W, C, E, G.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeoutSettings {
    pub handshake: Duration,
    pub tls_upgrade: Duration,
    pub wait: Duration,
    pub connect: Duration,
    pub close: Duration,
    pub shutdown_grace: Duration,
}

/// `[keepalive]`: K and Kw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeepaliveSettings {
    pub tcp_idle: Duration,
    pub tcp_interval: Duration,
    pub tcp_probes: u32,
    pub ws_ping: Duration,
    pub ws_pong_timeout: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagementSettings {
    pub listen: SocketAddr,
    /// HTTPS when present. Without it, `listen` is a loopback address.
    pub tls: Option<TlsFiles>,
    /// Allows management API edits only when explicitly enabled at startup.
    pub config_edit: bool,
    pub login_limit: LoginLimitSettings,
    pub session: AdminSessionSettings,
}

/// `[management.login_limit]`: F.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoginLimitSettings {
    pub per_account: u32,
    pub per_source: u32,
    pub window: Duration,
    pub lockout: Duration,
}

/// `[management.session]`: Si and Sa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdminSessionSettings {
    pub idle: Duration,
    pub max_lifetime: Duration,
}

/// `[dashboard]`: P and the poll query timeout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DashboardSettings {
    pub poll: Duration,
    pub query_timeout: Duration,
}

/// `[recent_closed]`: R.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecentClosedSettings {
    pub max_entries: u32,
}

/// `[recent_logs]`: bounded recent operation-log capacity, used by phase R3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecentLogsSettings {
    pub max_entries: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusSettings {
    pub listen: SocketAddr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogSettings {
    pub output: LogOutput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogOutput {
    Stdout,
    /// Appended to; created when missing.
    File(PathBuf),
}

/// `[reload]`: D and Dp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReloadSettings {
    pub settle: Duration,
    pub poll: Duration,
}

impl Settings {
    /// Warnings of config-v4 §2.2, in a fixed order. The caller supplies `phase`.
    pub fn warnings(&self) -> Vec<ConfigWarning> {
        let mut warnings = Vec::new();
        if let Some(listen) = self.relay.ws.listen.filter(|_| self.relay.ws.enabled()) {
            warnings.push(ConfigWarning::PlaintextWsEnabled { listen });
        }
        for entry_point in EntryPoint::ALL {
            if let Some(allow) = self.relay.allow(entry_point) {
                warnings.extend(allow.whole_family_items().map(|item| {
                    ConfigWarning::AllowAllSource {
                        entry_point,
                        item: item.clone(),
                    }
                }));
            }
        }
        for (listener, settings) in [
            (Listener::Ws, &self.relay.ws),
            (Listener::Wss, &self.relay.wss),
        ] {
            warnings.extend(settings.trusted_proxies.whole_family_items().map(|item| {
                ConfigWarning::TrustAllProxies {
                    listener,
                    item: item.clone(),
                }
            }));
        }
        if let Some(policy) = &self.mode1.allow_targets {
            warnings.extend(
                policy
                    .whole_family_items()
                    .map(|item| ConfigWarning::AllowAllTarget { item: item.clone() }),
            );
        }
        if !self.status.listen.ip().is_loopback() {
            warnings.push(ConfigWarning::StatusListenNotLoopback {
                listen: self.status.listen,
            });
        }
        warnings
    }

    /// The status listener, bound and answering before every other
    /// listener (ops-info-v2 §1.12).
    pub fn status_listener(&self) -> ListenerPlan {
        ListenerPlan {
            name: "status",
            key: "status.listen",
            addr: self.status.listen,
            listener: None,
        }
    }

    /// The management listener, bound after the status listener and before
    /// the entry points (management-api-v1 §1.1).
    pub fn management_listener(&self) -> ListenerPlan {
        ListenerPlan {
            name: "management",
            key: "management.listen",
            addr: self.management.listen,
            listener: None,
        }
    }

    /// Listeners bound after the status and management listeners, in bind
    /// order: TCP Viewer, TCP Server, WS, WSS, when enabled.
    pub fn other_listeners(&self) -> Vec<ListenerPlan> {
        let mut plans = Vec::new();
        for listener in Listener::ALL {
            if let Some(addr) = self
                .relay
                .listen(listener)
                .filter(|_| self.relay.listener_enabled(listener))
            {
                plans.push(ListenerPlan {
                    name: listener.name(),
                    key: listen_key(listener),
                    addr,
                    listener: Some(listener),
                });
            }
        }
        plans
    }
}

/// One listener to bind at startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListenerPlan {
    /// `listener` value of the operation log (ops-info-v2 §1.5).
    pub name: &'static str,
    /// Config key of the address, used as the `key` of diagnostics.
    pub key: &'static str,
    pub addr: SocketAddr,
    /// Relay socket, or `None` for status and management listeners.
    pub listener: Option<Listener>,
}

fn listen_key(listener: Listener) -> &'static str {
    match listener {
        Listener::TcpViewer => "relay.tcp_viewer.listen",
        Listener::TcpServer => "relay.tcp_server.listen",
        Listener::Ws => "relay.ws.listen",
        Listener::Wss => "relay.wss.listen",
    }
}

/// Config error codes (config-v4 §2.1) that the config loader reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigError {
    FileUnreadable,
    SyntaxError,
    UnknownKey,
    MissingKey,
    TypeMismatch,
    ValueOutOfRange,
    RelationViolated,
    ListenInvalid,
    ListenConflict,
    IpItemInvalid,
    IpRangeReversed,
    IpRangeFamilyMixed,
    OriginInvalid,
    TargetItemInvalid,
    PlaintextManagementNotLoopback,
    TlsPairIncomplete,
    TlsFileInvalid,
}

impl ConfigError {
    /// The stable code string of the contract.
    pub fn code(self) -> &'static str {
        match self {
            Self::FileUnreadable => "config_file_unreadable",
            Self::SyntaxError => "config_syntax_error",
            Self::UnknownKey => "config_unknown_key",
            Self::MissingKey => "config_missing_key",
            Self::TypeMismatch => "config_type_mismatch",
            Self::ValueOutOfRange => "config_value_out_of_range",
            Self::RelationViolated => "config_relation_violated",
            Self::ListenInvalid => "config_listen_invalid",
            Self::ListenConflict => "config_listen_conflict",
            Self::IpItemInvalid => "config_ip_item_invalid",
            Self::IpRangeReversed => "config_ip_range_reversed",
            Self::IpRangeFamilyMixed => "config_ip_range_family_mixed",
            Self::OriginInvalid => "config_origin_invalid",
            Self::TargetItemInvalid => "config_target_item_invalid",
            Self::PlaintextManagementNotLoopback => "config_plaintext_management_not_loopback",
            Self::TlsPairIncomplete => "config_tls_pair_incomplete",
            Self::TlsFileInvalid => "tls_file_invalid",
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for ConfigError {}

/// One diagnostic of config-v4 §2.1. `key` is the full dotted path with `[i]`
/// for array elements, or empty for file-level errors. `message` has no
/// secret and no file content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDiagnostic {
    pub error: ConfigError,
    pub key: String,
    pub message: String,
}

/// Warnings of config-v4 §2.2. Field values follow ops-info-v2 §1.5.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigWarning {
    PlaintextWsEnabled {
        listen: SocketAddr,
    },
    AllowAllSource {
        entry_point: EntryPoint,
        item: IpItem,
    },
    TrustAllProxies {
        listener: Listener,
        item: IpItem,
    },
    AllowAllTarget {
        item: TargetItem,
    },
    StatusListenNotLoopback {
        listen: SocketAddr,
    },
}

impl ConfigWarning {
    /// The warning code, also the event name in the operation log.
    pub fn code(&self) -> &'static str {
        match self {
            Self::PlaintextWsEnabled { .. } => "plaintext_ws_enabled",
            Self::AllowAllSource { .. } => "allow_all_source",
            Self::TrustAllProxies { .. } => "trust_all_proxies",
            Self::AllowAllTarget { .. } => "allow_all_target",
            Self::StatusListenNotLoopback { .. } => "status_listen_not_loopback",
        }
    }
}
