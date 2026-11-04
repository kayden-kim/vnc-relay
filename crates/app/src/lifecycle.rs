//! Readiness of the server (operations spec §2, ops-info-v1 §1.12 and §2).
//! Design: docs/architectures/status-listener.md.

use std::sync::atomic::{AtomicU8, Ordering};

/// What `/readyz` reports. The first shutdown cause remains until process exit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Readiness {
    /// The config is valid and the required listeners are being prepared.
    Starting,
    /// Every required listener is ready.
    Ready,
    /// A shutdown began; no new relay connection is taken.
    ShuttingDown,
    /// A required listener failed; fatal grace and cleanup retain this reason.
    ListenerFailed,
}

impl Readiness {
    /// The `reason` of a not-ready `/readyz` answer (ops-info-v1 §2), or
    /// `None` when the server is ready.
    pub fn not_ready_reason(self) -> Option<&'static str> {
        match self {
            Self::Starting => Some("starting"),
            Self::Ready => None,
            Self::ShuttingDown => Some("shutting_down"),
            Self::ListenerFailed => Some("listener_failed"),
        }
    }

    fn from_u8(value: u8) -> Self {
        match value {
            STARTING => Self::Starting,
            READY => Self::Ready,
            LISTENER_FAILED => Self::ListenerFailed,
            _ => Self::ShuttingDown,
        }
    }
}

const STARTING: u8 = 0;
const READY: u8 = 1;
const SHUTTING_DOWN: u8 = 2;
const LISTENER_FAILED: u8 = 3;

/// The readiness of one server process, shared by the start sequence and
/// the status listener.
#[derive(Debug)]
pub struct LifecycleService {
    state: AtomicU8,
}

impl Default for LifecycleService {
    fn default() -> Self {
        Self::new()
    }
}

impl LifecycleService {
    /// A new process starts in [`Readiness::Starting`].
    pub fn new() -> Self {
        Self {
            state: AtomicU8::new(STARTING),
        }
    }

    pub fn readiness(&self) -> Readiness {
        Readiness::from_u8(self.state.load(Ordering::Acquire))
    }

    /// Moves from starting to ready. A shutdown that began first stays.
    pub fn mark_ready(&self) {
        // A failed exchange means the state already left `Starting`, which
        // is the case this method leaves alone.
        let _ = self
            .state
            .compare_exchange(STARTING, READY, Ordering::AcqRel, Ordering::Acquire);
    }

    /// Begins requested shutdown without replacing an earlier fatal cause.
    pub fn begin_shutdown(&self) {
        self.begin_stop(SHUTTING_DOWN);
    }

    /// Begins fatal shutdown unless requested shutdown already started.
    /// See ops-info-v1 §1.12.
    pub fn listener_failed(&self) {
        self.begin_stop(LISTENER_FAILED);
    }

    fn begin_stop(&self, stop: u8) {
        let _ = self
            .state
            .try_update(Ordering::AcqRel, Ordering::Acquire, |state| {
                matches!(state, STARTING | READY).then_some(stop)
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// spec: operations.md:OPS-013 partial
    #[test]
    fn tells_starting_ready_and_shutting_down_apart() {
        let lifecycle = LifecycleService::new();
        assert_eq!(lifecycle.readiness(), Readiness::Starting);
        assert_eq!(lifecycle.readiness().not_ready_reason(), Some("starting"));

        lifecycle.mark_ready();
        assert_eq!(lifecycle.readiness(), Readiness::Ready);
        assert_eq!(lifecycle.readiness().not_ready_reason(), None);

        lifecycle.begin_shutdown();
        assert_eq!(lifecycle.readiness(), Readiness::ShuttingDown);
        assert_eq!(
            lifecycle.readiness().not_ready_reason(),
            Some("shutting_down")
        );
    }

    /// spec: operations.md:OPS-013 partial
    #[test]
    fn a_shutdown_during_the_start_is_never_undone_by_ready() {
        let lifecycle = LifecycleService::new();
        lifecycle.begin_shutdown();
        lifecycle.mark_ready();
        assert_eq!(lifecycle.readiness(), Readiness::ShuttingDown);
    }
}
