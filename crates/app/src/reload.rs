//! Automatic partial reload of the config file and TLS files (TODO 5.2, file-watch-v1).

pub mod ports;
pub mod service;
pub mod snapshot;

/// What noticed a change (ops-info-v1 §1.5 `reload_change_detected`).
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

/// Why watching stopped working (ops-info-v1 §1.5 `reload_watch_failed`).
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
