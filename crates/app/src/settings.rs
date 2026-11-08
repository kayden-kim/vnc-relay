//! Effective settings: config values after every check of config-v3 §1, plus
//! the config error and warning codes of config-v3 §2.

use std::fmt;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use vnc_relay_domain::account::Argon2Params;
use vnc_relay_domain::allow_list::{AllowList, IpItem};
use vnc_relay_domain::entry_point::EntryPoint;
use vnc_relay_domain::origin::Origin;
use vnc_relay_domain::target_policy::{TargetItem, TargetPolicy};

/// Validated settings. Omitted keys hold their defaults. Equality compares
/// values only, so notation, comments, key order, and explicit defaults do not
/// matter (config-v3 §1.8 rule 2).
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
    pub ws_viewer: EntryPointSettings,
    pub wss_viewer: EntryPointSettings,
}

impl RelaySettings {
    pub fn get(&self, entry_point: EntryPoint) -> &EntryPointSettings {
        match entry_point {
            EntryPoint::TcpViewer => &self.tcp_viewer,
            EntryPoint::TcpServer => &self.tcp_server,
            EntryPoint::WsViewer => &self.ws_viewer,
            EntryPoint::WssViewer => &self.wss_viewer,
        }
    }
}

/// One entry point. When `enabled` is true, `listen` and `allow` are present,
/// and `tls` is present for `wss_viewer`. A disabled entry point keeps the
/// values written for it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EntryPointSettings {
    pub enabled: bool,
    /// Default port already applied for TCP entry points.
    pub listen: Option<SocketAddr>,
    pub allow: Option<AllowList>,
    /// WS and WSS only; empty for TCP entry points.
    pub allowed_origins: Vec<Origin>,
    /// WSS only.
    pub tls: Option<TlsFiles>,
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
    /// Warnings of config-v3 §2.2 for these settings, in a fixed order. The
    /// caller records them with its own `phase`. Only `allow_all_source` is
    /// also a reload warning; a reload caller keeps that kind only.
    pub fn warnings(&self) -> Vec<ConfigWarning> {
        let mut warnings = Vec::new();
        if let Some(listen) = self
            .relay
            .ws_viewer
            .listen
            .filter(|_| self.relay.ws_viewer.enabled)
        {
            warnings.push(ConfigWarning::PlaintextWsEnabled { listen });
        }
        for entry_point in EntryPoint::ALL {
            if let Some(allow) = &self.relay.get(entry_point).allow {
                warnings.extend(allow.whole_family_items().map(|item| {
                    ConfigWarning::AllowAllSource {
                        entry_point,
                        item: item.clone(),
                    }
                }));
            }
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
    /// listener (ops-info-v1 §1.12).
    pub fn status_listener(&self) -> ListenerPlan {
        ListenerPlan {
            name: "status",
            key: "status.listen",
            addr: self.status.listen,
            entry_point: None,
        }
    }

    /// The management listener, bound after the status listener and before
    /// the entry points (management-api-v1 §1.1).
    pub fn management_listener(&self) -> ListenerPlan {
        ListenerPlan {
            name: "management",
            key: "management.listen",
            addr: self.management.listen,
            entry_point: None,
        }
    }

    /// Listeners bound after the status and management listeners, in bind
    /// order: the enabled entry points.
    pub fn other_listeners(&self) -> Vec<ListenerPlan> {
        let mut plans = Vec::new();
        for entry_point in EntryPoint::ALL {
            let settings = self.relay.get(entry_point);
            if let Some(addr) = settings.listen.filter(|_| settings.enabled) {
                plans.push(ListenerPlan {
                    name: entry_point.name(),
                    key: listen_key(entry_point),
                    addr,
                    entry_point: Some(entry_point),
                });
            }
        }
        plans
    }
}

/// One listener to bind at startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListenerPlan {
    /// `listener` value of the operation log (ops-info-v1 §1.5).
    pub name: &'static str,
    /// Config key of the address, used as the `key` of diagnostics.
    pub key: &'static str,
    pub addr: SocketAddr,
    /// The entry point, or `None` for the status and management listeners.
    pub entry_point: Option<EntryPoint>,
}

fn listen_key(entry_point: EntryPoint) -> &'static str {
    match entry_point {
        EntryPoint::TcpViewer => "relay.tcp_viewer.listen",
        EntryPoint::TcpServer => "relay.tcp_server.listen",
        EntryPoint::WsViewer => "relay.ws_viewer.listen",
        EntryPoint::WssViewer => "relay.wss_viewer.listen",
    }
}

/// Config error codes (config-v3 §2.1) that the config loader reports.
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

/// One diagnostic of config-v3 §2.1. `key` is the full dotted path with `[i]`
/// for array elements, or empty for file-level errors. `message` has no
/// secret and no file content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDiagnostic {
    pub error: ConfigError,
    pub key: String,
    pub message: String,
}

/// Warnings of config-v3 §2.2. Field values follow ops-info-v1 §1.5.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigWarning {
    PlaintextWsEnabled {
        listen: SocketAddr,
    },
    AllowAllSource {
        entry_point: EntryPoint,
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
            Self::AllowAllTarget { .. } => "allow_all_target",
            Self::StatusListenNotLoopback { .. } => "status_listen_not_loopback",
        }
    }
}
