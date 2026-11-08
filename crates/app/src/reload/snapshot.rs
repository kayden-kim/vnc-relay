//! Runtime values that a reload can replace (file-watch-v1 §3).

use std::sync::{Arc, PoisonError, RwLock};

use vnc_relay_domain::allow_list::AllowList;

/// Allow lists of the four relay entry points (config-v3 §1.8).
///
/// A missing or disabled list is `AllowList::default()`, which rejects everything.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RelayAllow {
    pub tcp_viewer: AllowList,
    pub tcp_server: AllowList,
    pub ws_viewer: AllowList,
    pub wss_viewer: AllowList,
}

/// Values that a reload can replace (file-watch-v1 §3).
#[derive(Debug, Clone)]
pub struct ReloadSnapshot<A> {
    pub relay_allow: RelayAllow,
    /// Acceptor of the WSS viewer listener; `None` when WSS is disabled.
    pub wss_tls: Option<A>,
    /// Acceptor of the management listener; `None` when it serves plain HTTP.
    pub management_tls: Option<A>,
}

impl<A> ReloadSnapshot<A> {
    /// Creates a snapshot from the values of one candidate.
    pub fn new(relay_allow: RelayAllow, wss_tls: Option<A>, management_tls: Option<A>) -> Self {
        Self {
            relay_allow,
            wss_tls,
            management_tls,
        }
    }
}

/// Holds the current snapshot (file-watch-v1 §5).
///
/// Readers take the `Arc` once per new connection or handshake. Existing connections
/// keep the values they took, so a replacement never changes them.
pub struct ReloadCell<A> {
    current: RwLock<Arc<ReloadSnapshot<A>>>,
}

impl<A> ReloadCell<A> {
    /// Creates the cell with the snapshot of the running settings.
    pub fn new(snapshot: ReloadSnapshot<A>) -> Self {
        Self {
            current: RwLock::new(Arc::new(snapshot)),
        }
    }

    /// Returns the current snapshot. A poisoned lock still yields the last value.
    pub fn current(&self) -> Arc<ReloadSnapshot<A>> {
        let guard = self.current.read().unwrap_or_else(PoisonError::into_inner);
        Arc::clone(&guard)
    }

    /// Replaces the snapshot. New readers see `next`; existing readers keep theirs.
    pub fn replace(&self, next: ReloadSnapshot<A>) {
        let mut guard = self.current.write().unwrap_or_else(PoisonError::into_inner);
        *guard = Arc::new(next);
    }
}
