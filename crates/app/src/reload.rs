//! Automatic partial reload of the config file and TLS files (TODO 5.2, file-watch-v1).

pub mod ports;
pub mod service;
pub mod snapshot;
pub mod ticket_keys;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::settings::{ConfigDiagnostic, Settings};

pub type SharedReload<A> = Arc<Mutex<service::ReloadService<A>>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadTrigger {
    FileWatch,
    ConfigEdit,
}

impl ReloadTrigger {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FileWatch => "file_watch",
            Self::ConfigEdit => "config_edit",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadResult {
    Applied,
    Rejected,
    RestartRequired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastReload {
    pub at: SystemTime,
    pub result: ReloadResult,
    pub trigger: ReloadTrigger,
    pub config_keys: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigPresence {
    pub keys: BTreeSet<String>,
    pub inactive_paths: BTreeMap<String, PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigCandidate {
    pub settings: Settings,
    pub presence: ConfigPresence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFileState {
    InEffect,
    Pending,
    RestartRequired,
    Invalid,
    Unreadable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileClassification {
    pub state: ConfigFileState,
    pub config_keys: Vec<String>,
    pub diagnostics: Vec<ConfigDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrepareReloadError {
    Invalid(Vec<ConfigDiagnostic>),
    RestartRequired(Vec<String>),
}

/// What noticed a change (ops-info-v2 §1.5 `reload_change_detected`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeSource {
    /// The OS file notification API.
    OsNotify,
    /// The periodic poll.
    Poll,
}

impl ChangeSource {
    /// Value of the `source` field of `reload_change_detected`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OsNotify => "os_notify",
            Self::Poll => "poll",
        }
    }
}

/// Why watching stopped working (ops-info-v2 §1.5 `reload_watch_failed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchFailure {
    /// The OS notification API reported an error.
    NotifyError,
    /// The periodic poll failed.
    PollError,
}

impl WatchFailure {
    /// Value of the `reason` field of `reload_watch_failed`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotifyError => "notify_error",
            Self::PollError => "poll_error",
        }
    }
}
