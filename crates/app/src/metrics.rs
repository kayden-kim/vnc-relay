//! Metrics of metrics-v1: recording, current values, and the 10-second
//! history. Design: docs/architectures/metrics.md.
//!
//! `Metrics` only records. `MetricsSink` counts operations-log events on the
//! way to the log writer, and the relay and WebSocket code record the values
//! that have no event. `MetricsService` reads everything into one
//! `MetricsSnapshot`.

pub mod history;
pub mod service;
pub mod snapshot;

#[cfg(test)]
mod tests;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use vnc_relay_domain::entry_point::Listener;
use vnc_relay_domain::handshake::Mode;
use vnc_relay_domain::session::{Direction, Transport};

use crate::events::{Event, RejectReason, Stage, TargetFailureReason};
use crate::ports::{Clock, EventSink};
use snapshot::{MetricPoint, Number, PointValue};

pub use service::{MetricSources, MetricsService};

/// Bounds of `vnc_relay.handshake.duration` and
/// `vnc_relay.target_connect.duration` in seconds (metrics-v1 §1.5).
pub const SHORT_DURATION_BOUNDS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 120.0,
];

/// Bounds of `vnc_relay.waiting.duration` in seconds (metrics-v1 §1.5).
pub const WAITING_DURATION_BOUNDS: &[f64] = &[
    1.0, 5.0, 10.0, 30.0, 60.0, 300.0, 600.0, 1800.0, 3600.0, 7200.0, 21600.0, 43200.0, 86400.0,
];

/// Bounds of `vnc_relay.session.duration` in seconds (metrics-v1 §1.5).
pub const SESSION_DURATION_BOUNDS: &[f64] = &[
    1.0, 10.0, 60.0, 300.0, 600.0, 1800.0, 3600.0, 7200.0, 14400.0, 28800.0, 86400.0, 259200.0,
    604800.0,
];

/// Counters and histograms of metrics-v1 §1.3, from process start
/// (metrics-v1 §1.1 "시작점과 누적"). One instance lives per process.
pub struct Metrics {
    /// `Clock::monotonic` when the instance was made: the `process.uptime`
    /// start point.
    started: Duration,
    connection_accepted: Family<u64, 1>,
    connection_rejected: Family<u64, 3>,
    connection_aborted: Family<u64, 3>,
    handshake_duration: HistogramFamily<2>,
    waiting_duration: HistogramFamily<1>,
    session_started: Family<u64, 3>,
    session_closed: Family<u64, 2>,
    session_duration: HistogramFamily<1>,
    /// Bytes by direction: `[viewer_to_target, target_to_viewer]`. Added
    /// without a lock because the relay adds after every write.
    session_io: [AtomicU64; 2],
    target_connect_duration: HistogramFamily<1>,
    ticket_accepted: Family<u64, 2>,
    websocket_ping_timeouts: Family<u64, 1>,
    admin_logins: Family<u64, 1>,
    admin_lockouts: Family<u64, 1>,
    admin_terminations: Family<u64, 1>,
    config_reloads: Family<u64, 3>,
    config_edits: Family<u64, 2>,
}

impl Metrics {
    /// Every value starts at zero; `clock` fixes the uptime start point.
    pub fn new(clock: &dyn Clock) -> Self {
        Self {
            started: clock.monotonic(),
            connection_accepted: Family::new(["listener"]),
            connection_rejected: Family::new(["listener", "stage", "reason"]),
            connection_aborted: Family::new(["listener", "stage", "reason"]),
            handshake_duration: HistogramFamily::new(["listener", "mode"], SHORT_DURATION_BOUNDS),
            waiting_duration: HistogramFamily::new(["result"], WAITING_DURATION_BOUNDS),
            session_started: Family::new(["mode", "viewer_transport", "target_transport"]),
            session_closed: Family::new(["mode", "close_cause"]),
            session_duration: HistogramFamily::new(["mode"], SESSION_DURATION_BOUNDS),
            session_io: [AtomicU64::new(0), AtomicU64::new(0)],
            target_connect_duration: HistogramFamily::new(["result"], SHORT_DURATION_BOUNDS),
            ticket_accepted: Family::new(["listener", "role"]),
            websocket_ping_timeouts: Family::new(["listener"]),
            admin_logins: Family::new(["result"]),
            admin_lockouts: Family::new(["scope"]),
            admin_terminations: Family::new(["result"]),
            config_reloads: Family::new(["result", "trigger", "reason"]),
            config_edits: Family::new(["result", "reason"]),
        }
    }

    /// A successful Repeater initial exchange: `elapsed` runs from the start
    /// of H to the valid request (metrics-v1 `vnc_relay.handshake.duration`).
    pub fn record_handshake(&self, listener: Listener, mode: Mode, elapsed: Duration) {
        self.handshake_duration.observe(
            [Some(listener.name()), Some(mode.as_str())],
            elapsed.as_secs_f64(),
        );
    }

    /// A finished Mode I target connect attempt: `elapsed` runs from the
    /// start of C, the same span as `target_connect_failed.elapsed_ms`
    /// (metrics-v1 `vnc_relay.target_connect.duration`).
    pub fn record_target_connect(
        &self,
        result: Result<(), TargetFailureReason>,
        elapsed: Duration,
    ) {
        let result = match result {
            Ok(()) => "connected",
            Err(reason) => reason.as_str(),
        };
        self.target_connect_duration
            .observe([Some(result)], elapsed.as_secs_f64());
    }

    /// Bytes an active session wrote to the opposite transport
    /// (metrics-v1 `vnc_relay.session.io`).
    pub fn add_io(&self, direction: Direction, bytes: u64) {
        let index = match direction {
            Direction::ViewerToTarget => 0,
            Direction::TargetToViewer => 1,
        };
        if let Some(total) = self.session_io.get(index) {
            total.fetch_add(bytes, Ordering::Relaxed);
        }
    }

    /// A WebSocket connection that missed its Pong deadline
    /// (metrics-v1 `vnc_relay.websocket.ping_timeouts`). TCP keepalive
    /// failures are not counted here.
    pub fn record_ping_timeout(&self, listener: Listener) {
        self.websocket_ping_timeouts
            .increment([Some(listener.name())]);
    }

    /// Counts one operations-log event (metrics-v1 §1.3 "로그 사건").
    /// Attribute values come from the same `as_str`/`name` functions the
    /// log writer uses.
    fn record_event(&self, event: &Event) {
        match event {
            Event::ConnectionAccepted { listener, .. } => {
                self.connection_accepted.increment([Some(listener.name())]);
            }
            Event::ConnectionRejected {
                listener,
                stage,
                reason,
                ..
            } => self.reject(listener, *stage, *reason),
            Event::UpgradeRejected {
                listener, reason, ..
            } => self.reject(listener, Stage::Upgrade, *reason),
            Event::HandshakeRejected {
                entry_point,
                reason,
                ..
            } => self.reject(
                entry_point.listener().name(),
                Stage::InitialExchange,
                *reason,
            ),
            Event::TicketRelayMismatch { entry_point, .. } => self.reject(
                entry_point.listener().name(),
                Stage::InitialExchange,
                RejectReason::TicketRelayMismatch,
            ),
            Event::TargetRejected {
                entry_point,
                reason,
                ..
            } => self.reject(entry_point.listener().name(), Stage::TargetConnect, *reason),
            Event::RegistrationRejected {
                entry_point,
                reason,
                ..
            } => self.reject(entry_point.listener().name(), Stage::Register, *reason),
            Event::ConnectionAborted {
                listener,
                stage,
                reason,
                ..
            } => self.connection_aborted.increment([
                Some(listener.name()),
                Some(stage.as_str()),
                Some(reason.as_str()),
            ]),
            Event::RelayAborted {
                entry_point,
                stage,
                reason,
                ..
            } => self.connection_aborted.increment([
                Some(entry_point.listener().name()),
                Some(stage.as_str()),
                Some(reason.as_str()),
            ]),
            Event::TicketAccepted { listener, role, .. } => self
                .ticket_accepted
                .increment([Some(listener.name()), Some(role.as_str())]),
            Event::WaitingExpired { waited_ms, .. } => {
                self.waiting_duration
                    .observe([Some("expired")], seconds(*waited_ms));
            }
            Event::WaitingRemoved { waited_ms, .. } => {
                self.waiting_duration
                    .observe([Some("removed")], seconds(*waited_ms));
            }
            Event::SessionStarted {
                mode,
                viewer_transport,
                server_transport,
                waited_ms,
                ..
            } => {
                // A Mode I target is always TCP; Mode II always carries the
                // Server's transport (events.rs `SessionStarted`).
                let target = server_transport.unwrap_or(Transport::Tcp);
                self.session_started.increment([
                    Some(mode.as_str()),
                    Some(viewer_transport.as_str()),
                    Some(target.as_str()),
                ]);
                if let Some(waited_ms) = waited_ms {
                    self.waiting_duration
                        .observe([Some("matched")], seconds(*waited_ms));
                }
            }
            Event::SessionClosed {
                mode,
                close_cause,
                duration_ms,
                ..
            } => {
                self.session_closed
                    .increment([Some(mode.as_str()), Some(close_cause.as_str())]);
                self.session_duration
                    .observe([Some(mode.as_str())], seconds(*duration_ms));
            }
            Event::AdminTerminate { result, .. } => {
                self.admin_terminations.increment([Some(result.as_str())]);
            }
            Event::AdminLoginSucceeded { .. } => self.admin_logins.increment([Some("succeeded")]),
            Event::AdminLoginFailed { reason, .. } => {
                self.admin_logins.increment([Some(reason.as_str())]);
            }
            Event::AdminLoginLimited { scope, .. } => {
                self.admin_lockouts.increment([Some(scope.as_str())]);
            }
            Event::ConfigEdited { .. } => self.config_edits.increment([Some("edited"), None]),
            Event::ConfigEditRejected { reason, .. } => self
                .config_edits
                .increment([Some("rejected"), Some(reason.as_str())]),
            Event::ReloadApplied { trigger, .. } => {
                self.config_reloads
                    .increment([Some("applied"), Some(trigger.as_str()), None]);
            }
            Event::ReloadRejected { reason, .. } => {
                self.config_reloads
                    .increment([Some("rejected"), None, Some(reason.as_str())]);
            }
            Event::ReloadRestartRequired { .. } => {
                self.config_reloads
                    .increment([Some("restart_required"), None, None]);
            }
            // No metric counts these events. Listing them keeps a new event
            // from being skipped silently.
            Event::ServerStarting
            | Event::ConfigWarning { .. }
            | Event::StartupFailed { .. }
            | Event::ServerReady { .. }
            | Event::TargetConnectFailed { .. }
            | Event::WaitingRegistered { .. }
            | Event::ShutdownStarted { .. }
            | Event::ShutdownForced { .. }
            | Event::ReloadChangeDetected { .. }
            | Event::ReloadWatchFailed { .. }
            | Event::LogReopenFailed { .. }
            | Event::IpDbLoaded { .. }
            | Event::IpDbUnavailable { .. }
            | Event::ServerStopped => {}
        }
    }

    fn reject(&self, listener: &str, stage: Stage, reason: RejectReason) {
        self.connection_rejected.increment([
            Some(listener),
            Some(stage.as_str()),
            Some(reason.as_str()),
        ]);
    }
}

/// The operations-log writer with metric counting in front (metrics-v1 §1.1
/// "기록 시점"). Every `EventSink` of the process is this sink, so the counts
/// equal the event counts whether or not the log write succeeds.
pub struct MetricsSink {
    metrics: Arc<Metrics>,
    inner: Arc<dyn EventSink>,
}

impl MetricsSink {
    pub fn new(metrics: Arc<Metrics>, inner: Arc<dyn EventSink>) -> Self {
        Self { metrics, inner }
    }
}

impl EventSink for MetricsSink {
    fn record(&self, event: &Event) {
        self.metrics.record_event(event);
        self.inner.record(event);
    }
}

fn seconds(milliseconds: u64) -> f64 {
    milliseconds as f64 / 1000.0
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    // Nothing under these locks panics; a poisoned value is still whole.
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Attribute values of one point in metrics-v1 §1.3 attribute order. `None`
/// is an attribute this point does not have.
type Key<const N: usize> = [Option<Arc<str>>; N];

/// One counter or histogram: points kept sorted by key, which is the sample
/// order of metrics-v1 §1.6 (`None` sorts before any value). Finding an
/// existing key compares `&str` values and allocates nothing.
struct Family<V, const N: usize> {
    names: [&'static str; N],
    entries: Mutex<Vec<(Key<N>, V)>>,
}

impl<V, const N: usize> Family<V, N> {
    fn new(names: [&'static str; N]) -> Self {
        Self {
            names,
            entries: Mutex::new(Vec::new()),
        }
    }

    fn update(&self, key: [Option<&str>; N], init: impl FnOnce() -> V, apply: impl FnOnce(&mut V)) {
        let mut entries = lock(&self.entries);
        let index = match entries.binary_search_by(|(stored, _)| {
            stored
                .iter()
                .map(|value| value.as_deref())
                .cmp(key.iter().copied())
        }) {
            Ok(index) => index,
            Err(index) => {
                entries.insert(index, (key.map(|value| value.map(Arc::from)), init()));
                index
            }
        };
        if let Some((_, value)) = entries.get_mut(index) {
            apply(value);
        }
    }

    /// Copies every recorded point under one lock.
    fn points(&self, value: impl Fn(&V) -> PointValue) -> Vec<MetricPoint> {
        lock(&self.entries)
            .iter()
            .map(|(key, stored)| MetricPoint {
                attributes: self
                    .names
                    .iter()
                    .zip(key)
                    .filter_map(|(name, value)| {
                        value.as_ref().map(|value| (*name, Arc::clone(value)))
                    })
                    .collect(),
                value: value(stored),
            })
            .collect()
    }
}

impl<const N: usize> Family<u64, N> {
    fn increment(&self, key: [Option<&str>; N]) {
        self.update(key, || 0, |count| *count += 1);
    }

    fn counter_points(&self) -> Vec<MetricPoint> {
        self.points(|count| PointValue::Number(Number::Int(*count)))
    }
}

/// Count, sum, and per-bucket counts of one histogram point.
#[derive(Debug, Clone)]
struct Histogram {
    bucket_counts: Vec<u64>,
    count: u64,
    sum: f64,
}

struct HistogramFamily<const N: usize> {
    bounds: &'static [f64],
    family: Family<Histogram, N>,
}

impl<const N: usize> HistogramFamily<N> {
    fn new(names: [&'static str; N], bounds: &'static [f64]) -> Self {
        Self {
            bounds,
            family: Family::new(names),
        }
    }

    /// Adds `value` to the first bucket whose bound is at least `value`, or
    /// to the last (+Inf) bucket (metrics-v1 §1.5).
    fn observe(&self, key: [Option<&str>; N], value: f64) {
        let bounds = self.bounds;
        let index = bounds.partition_point(|bound| *bound < value);
        self.family.update(
            key,
            || Histogram {
                bucket_counts: vec![0; bounds.len() + 1],
                count: 0,
                sum: 0.0,
            },
            |histogram| {
                if let Some(bucket) = histogram.bucket_counts.get_mut(index) {
                    *bucket += 1;
                }
                histogram.count += 1;
                histogram.sum += value;
            },
        );
    }

    fn histogram_points(&self) -> Vec<MetricPoint> {
        self.family.points(|histogram| PointValue::Histogram {
            count: histogram.count,
            sum: histogram.sum,
            bucket_counts: histogram.bucket_counts.clone(),
        })
    }
}
