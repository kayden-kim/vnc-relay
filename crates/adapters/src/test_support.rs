//! Shared adapter unit-test doubles; no transport or filesystem dependencies.

use std::sync::Arc;

use vnc_relay_application::metrics::{MetricSources, Metrics, MetricsService};
use vnc_relay_application::ports::{
    AdminSessionSource, Clock, ProcessStats, ProcessStatsSource, RelayStateSource,
};
use vnc_relay_application::relay::{Counts, RelaySnapshot};
use vnc_relay_application::reload::ports::RuntimeSettingsSource;
use vnc_relay_application::settings::Settings;
use vnc_relay_domain::budget::ConnectionBudget;

use crate::clock::SystemClock;

/// No relay connections.
struct StubRelayStateSource;

impl RelayStateSource for StubRelayStateSource {
    fn snapshot(&self, _clock: &dyn Clock) -> RelaySnapshot {
        unreachable!("metrics read counts only")
    }

    fn counts(&self) -> Counts {
        Counts::default()
    }
}

struct StubAdminSessionSource;

impl AdminSessionSource for StubAdminSessionSource {
    fn live_sessions(&self) -> usize {
        0
    }
}

/// No settings: limits read as 0.
struct StubRuntimeSettingsSource;

impl RuntimeSettingsSource for StubRuntimeSettingsSource {
    fn with_current(&self, _read: &mut dyn FnMut(&Settings)) {}
}

/// Process values chosen by the test, including unreadable ones.
struct StubProcessStatsSource(ProcessStats);

impl ProcessStatsSource for StubProcessStatsSource {
    fn read(&self) -> ProcessStats {
        self.0
    }
}

/// The metrics read side over `metrics` and the given process values, with
/// every other source empty.
pub(crate) fn metrics_service(metrics: Arc<Metrics>, process: ProcessStats) -> MetricsService {
    metrics_service_with_clock(metrics, process, Arc::new(SystemClock::new()))
}

/// `metrics_service` that reads the time from `clock`.
pub(crate) fn metrics_service_with_clock(
    metrics: Arc<Metrics>,
    process: ProcessStats,
    clock: Arc<dyn Clock>,
) -> MetricsService {
    MetricsService::new(
        metrics,
        MetricSources {
            relay: Arc::new(StubRelayStateSource),
            relay_budget: ConnectionBudget::new(),
            admin_budget: ConnectionBudget::new(),
            settings: Arc::new(StubRuntimeSettingsSource),
            admin: Arc::new(StubAdminSessionSource),
            process: Arc::new(StubProcessStatsSource(process)),
        },
        clock,
        "test",
    )
}
