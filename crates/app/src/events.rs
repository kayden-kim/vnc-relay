//! Operation log events (ops-info-v1 §1.5) and their `level` (§1.3).
//! Design: docs/architectures/ops-log.md.

use std::fmt;

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
    /// Last event: resources are released and the process ends.
    ServerStopped,
}

impl Event {
    /// The `event` field.
    pub fn name(&self) -> &'static str {
        match self {
            Self::ServerStarting => "server_starting",
            Self::ConfigWarning { warning, .. } => warning.code(),
            Self::StartupFailed { .. } => "startup_failed",
            Self::ServerStopped => "server_stopped",
        }
    }

    /// The `level` field of ops-info-v1 §1.5.
    pub fn level(&self) -> Level {
        match self {
            Self::ServerStarting | Self::ServerStopped => Level::Info,
            Self::ConfigWarning { .. } => Level::Warn,
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
