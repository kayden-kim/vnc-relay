//! Runtime values that a reload can replace (file-watch-v1 §3).

use std::sync::{Arc, PoisonError, RwLock};

/// Values that a reload can replace (file-watch-v1 §3).
#[derive(Debug, Clone)]
pub struct ReloadSnapshot<A> {
    pub settings: Arc<crate::settings::Settings>,
    pub presence: Arc<super::ConfigPresence>,
    /// Acceptor of the shared WSS listener; `None` when both roles are disabled.
    pub wss_tls: Option<A>,
    /// Acceptor of the management listener; `None` when it serves plain HTTP.
    pub management_tls: Option<A>,
}

impl<A> ReloadSnapshot<A> {
    /// Creates a snapshot from the values of one candidate.
    pub fn new(
        candidate: super::ConfigCandidate,
        wss_tls: Option<A>,
        management_tls: Option<A>,
    ) -> Self {
        Self {
            settings: Arc::new(candidate.settings),
            presence: Arc::new(candidate.presence),
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
        self.publish(Arc::new(next));
    }

    /// Publishes a snapshot allocated during candidate preparation.
    pub(crate) fn publish(&self, next: Arc<ReloadSnapshot<A>>) {
        let mut guard = self.current.write().unwrap_or_else(PoisonError::into_inner);
        *guard = next;
    }
}

impl<A: Send + Sync> super::ports::RuntimeSettingsSource for ReloadCell<A> {
    fn with_current(&self, read: &mut dyn FnMut(&crate::settings::Settings)) {
        let guard = self.current.read().unwrap_or_else(PoisonError::into_inner);
        read(&guard.settings);
    }
}
