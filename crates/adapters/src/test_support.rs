//! Shared adapter unit-test doubles; no transport or filesystem dependencies.

use std::sync::Arc;

use vnc_relay_app::metrics::{MetricSources, Metrics, MetricsService};
use vnc_relay_app::ports::{
    AdminSessionSource, Clock, ProcessStats, ProcessStatsSource, RelayStateSource,
};
use vnc_relay_app::relay::{RelayCounts, RelaySnapshot};
use vnc_relay_app::reload::ports::RuntimeSettingsSource;
use vnc_relay_app::settings::Settings;
use vnc_relay_domain::budget::ConnectionBudget;

use crate::clock::SystemClock;

/// No relay connection, no administrator session, and no settings: limits
/// read as 0.
struct Empty;

impl RelayStateSource for Empty {
    fn snapshot(&self, _clock: &dyn Clock) -> RelaySnapshot {
        unreachable!("metrics read counts only")
    }

    fn counts(&self) -> RelayCounts {
        RelayCounts::default()
    }
}

impl AdminSessionSource for Empty {
    fn live_sessions(&self) -> usize {
        0
    }
}

impl RuntimeSettingsSource for Empty {
    fn with_current(&self, _read: &mut dyn FnMut(&Settings)) {}
}

/// Process values chosen by the test, including unreadable ones.
struct FixedProcess(ProcessStats);

impl ProcessStatsSource for FixedProcess {
    fn read(&self) -> ProcessStats {
        self.0
    }
}

/// The metrics read side over `metrics` and the given process values, with
/// every other source empty.
pub(crate) fn metrics_service(metrics: Arc<Metrics>, process: ProcessStats) -> MetricsService {
    MetricsService::new(
        metrics,
        MetricSources {
            relay: Arc::new(Empty),
            relay_budget: ConnectionBudget::new(),
            admin_budget: ConnectionBudget::new(),
            settings: Arc::new(Empty),
            admin: Arc::new(Empty),
            process: Arc::new(FixedProcess(process)),
        },
        Arc::new(SystemClock::new()),
        "test",
    )
}
