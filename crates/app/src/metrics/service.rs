//! Reads every metric into one snapshot and keeps the history of the last
//! hour and the last 24 hours (metrics-v1 §1.3, §1.8). Design:
//! docs/architectures/metrics.md §3, §4.

use std::sync::{Arc, Mutex};

use vnc_relay_domain::budget::ConnectionBudget;
use vnc_relay_domain::entry_point::Role;
use vnc_relay_domain::handshake::Mode;
use vnc_relay_domain::session::Transport;

use super::history::{HISTORY_KEYS, History, HistoryView, Window};
use super::snapshot::{MetricFamily, MetricKind, MetricPoint, MetricsSnapshot, Number, PointValue};
use super::{Metrics, SESSION_DURATION_BOUNDS, SHORT_DURATION_BOUNDS, WAITING_DURATION_BOUNDS};
use crate::ports::{AdminSessionSource, Clock, OpenCount, ProcessStatsSource, RelayStateSource};
use crate::reload::ports::RuntimeSettingsSource;

/// Where the current values come from (docs/architectures/metrics.md §3).
pub struct MetricSources {
    pub relay: Arc<dyn RelayStateSource>,
    /// L.
    pub relay_budget: Arc<ConnectionBudget>,
    /// Lm.
    pub admin_budget: Arc<ConnectionBudget>,
    pub settings: Arc<dyn RuntimeSettingsSource>,
    pub admin: Arc<dyn AdminSessionSource>,
    pub process: Arc<dyn ProcessStatsSource>,
}

/// Read side of the metrics: the snapshot both representations write, and
/// the history the dashboard charts.
pub struct MetricsService {
    metrics: Arc<Metrics>,
    sources: MetricSources,
    clock: Arc<dyn Clock>,
    service_version: &'static str,
    labels: Labels,
    history: Mutex<History>,
}

impl MetricsService {
    /// `service_version` is the `vnc-relay --version` text (metrics-v1 §1.2).
    pub fn new(
        metrics: Arc<Metrics>,
        sources: MetricSources,
        clock: Arc<dyn Clock>,
        service_version: &'static str,
    ) -> Self {
        Self {
            metrics,
            sources,
            clock,
            service_version,
            labels: Labels::new(),
            history: Mutex::new(History::new()),
        }
    }

    /// Every metric of this platform now, in metrics-v1 §1.3 order. The relay
    /// counts come from one registry lock; each recorded family is copied
    /// under its own super::lock (docs/architectures/metrics.md §5).
    pub fn snapshot(&self) -> MetricsSnapshot {
        use MetricKind::{Counter, Gauge, UpDownCounter};

        let metrics = &*self.metrics;
        let labels = &self.labels;
        let counts = self.sources.relay.counts();
        let (relay_limit, admin_limit) = self.limits();
        let process = self.sources.process.read();
        let uptime = self.clock.monotonic().saturating_sub(metrics.started);

        let mut families = Vec::with_capacity(32);
        families.push(family(
            "vnc_relay.connection.accepted",
            Counter,
            "{connection}",
            "Connections accepted by relay listeners.",
            metrics.connection_accepted.counter_points(),
        ));
        families.push(family(
            "vnc_relay.connection.rejected",
            Counter,
            "{connection}",
            "Connections rejected by policy, limit or format.",
            metrics.connection_rejected.counter_points(),
        ));
        families.push(family(
            "vnc_relay.connection.aborted",
            Counter,
            "{connection}",
            "Connections that ended before waiting or a session.",
            metrics.connection_aborted.counter_points(),
        ));
        families.push(family(
            "vnc_relay.connection.in_progress",
            UpDownCounter,
            "{connection}",
            "Connections in the initial exchange or the Mode I target connect.",
            vec![
                point(
                    vec![("stage", Arc::clone(&labels.initial_exchange))],
                    count(counts.handshaking),
                ),
                point(
                    vec![("stage", Arc::clone(&labels.target_connect))],
                    count(counts.target_connecting),
                ),
            ],
        ));
        families.push(family(
            "vnc_relay.relay_connection.usage",
            UpDownCounter,
            "{connection}",
            "Relay connections held against the limit L.",
            single(Number::Int(u64::from(self.sources.relay_budget.current()))),
        ));
        families.push(family(
            "vnc_relay.relay_connection.limit",
            UpDownCounter,
            "{connection}",
            "Relay connection limit L.",
            single(Number::Int(u64::from(relay_limit))),
        ));
        families.push(family(
            "vnc_relay.admin_connection.usage",
            UpDownCounter,
            "{connection}",
            "Management and status connections held against the limit Lm.",
            single(Number::Int(u64::from(self.sources.admin_budget.current()))),
        ));
        families.push(family(
            "vnc_relay.admin_connection.limit",
            UpDownCounter,
            "{connection}",
            "Management and status connection limit Lm.",
            single(Number::Int(u64::from(admin_limit))),
        ));
        families.push(histogram(
            "vnc_relay.handshake.duration",
            "Repeater initial exchange duration.",
            SHORT_DURATION_BOUNDS,
            metrics.handshake_duration.histogram_points(),
        ));
        let mut waiting = Vec::with_capacity(6);
        for (role, role_label) in &labels.roles {
            for (transport, transport_label) in &labels.transports {
                waiting.push(point(
                    vec![
                        ("role", Arc::clone(role_label)),
                        ("transport", Arc::clone(transport_label)),
                    ],
                    count(counts.waiting(*role, *transport)),
                ));
            }
        }
        families.push(family(
            "vnc_relay.waiting.count",
            UpDownCounter,
            "{connection}",
            "Mode II connections waiting for a peer.",
            waiting,
        ));
        families.push(histogram(
            "vnc_relay.waiting.duration",
            "Time a waiting connection waited.",
            WAITING_DURATION_BOUNDS,
            metrics.waiting_duration.histogram_points(),
        ));
        let mut sessions = Vec::with_capacity(12);
        for (mode, mode_label) in &labels.modes {
            for (viewer, viewer_label) in &labels.transports {
                for (target, target_label) in &labels.transports {
                    // A Mode I target is always TCP (metrics-v1 §1.4).
                    if *mode == Mode::Mode1 && *target != Transport::Tcp {
                        continue;
                    }
                    sessions.push(point(
                        vec![
                            ("mode", Arc::clone(mode_label)),
                            ("viewer_transport", Arc::clone(viewer_label)),
                            ("target_transport", Arc::clone(target_label)),
                        ],
                        count(counts.sessions(*mode, *viewer, *target)),
                    ));
                }
            }
        }
        families.push(family(
            "vnc_relay.session.count",
            UpDownCounter,
            "{session}",
            "Active sessions, including sessions being cleaned up.",
            sessions,
        ));
        families.push(family(
            "vnc_relay.session.started",
            Counter,
            "{session}",
            "Active sessions started.",
            metrics.session_started.counter_points(),
        ));
        families.push(family(
            "vnc_relay.session.closed",
            Counter,
            "{session}",
            "Active sessions closed after cleanup.",
            metrics.session_closed.counter_points(),
        ));
        families.push(histogram(
            "vnc_relay.session.duration",
            "Active session duration.",
            SESSION_DURATION_BOUNDS,
            metrics.session_duration.histogram_points(),
        ));
        // Only directions that carried bytes: a counter shows recorded
        // combinations only (metrics-v1 §1.1 "속성 조합").
        let mut io = Vec::with_capacity(2);
        for (label, index) in [(&labels.target_to_viewer, 1), (&labels.viewer_to_target, 0)] {
            let bytes = metrics
                .session_io
                .get(index)
                .map_or(0, |total| total.load(std::sync::atomic::Ordering::Relaxed));
            if bytes > 0 {
                io.push(point(
                    vec![("direction", Arc::clone(label))],
                    Number::Int(bytes),
                ));
            }
        }
        families.push(family(
            "vnc_relay.session.io",
            Counter,
            "By",
            "Bytes written to the opposite transport by active sessions.",
            io,
        ));
        families.push(histogram(
            "vnc_relay.target_connect.duration",
            "Mode I target connect attempt duration.",
            SHORT_DURATION_BOUNDS,
            metrics.target_connect_duration.histogram_points(),
        ));
        families.push(family(
            "vnc_relay.ticket.accepted",
            Counter,
            "{ticket}",
            "Connection tickets accepted.",
            metrics.ticket_accepted.counter_points(),
        ));
        families.push(family(
            "vnc_relay.websocket.ping_timeouts",
            Counter,
            "{connection}",
            "WebSocket connections that missed the Pong deadline.",
            metrics.websocket_ping_timeouts.counter_points(),
        ));
        families.push(family(
            "vnc_relay.admin.logins",
            Counter,
            "{attempt}",
            "Administrator login attempts by result.",
            metrics.admin_logins.counter_points(),
        ));
        families.push(family(
            "vnc_relay.admin.lockouts",
            Counter,
            "{lockout}",
            "Login limits started.",
            metrics.admin_lockouts.counter_points(),
        ));
        families.push(family(
            "vnc_relay.admin.terminations",
            Counter,
            "{request}",
            "Administrator session terminate requests by result.",
            metrics.admin_terminations.counter_points(),
        ));
        families.push(family(
            "vnc_relay.admin_session.count",
            UpDownCounter,
            "{session}",
            "Live administrator sessions.",
            single(count(self.sources.admin.live_sessions())),
        ));
        families.push(family(
            "vnc_relay.config.reloads",
            Counter,
            "{reload}",
            "Configuration reload results.",
            metrics.config_reloads.counter_points(),
        ));
        families.push(family(
            "vnc_relay.config.edits",
            Counter,
            "{request}",
            "Configuration edit results.",
            metrics.config_edits.counter_points(),
        ));
        families.push(family(
            "process.uptime",
            Gauge,
            "s",
            "Time since the process started.",
            single(Number::Float(uptime.as_secs_f64())),
        ));
        families.push(optional(
            "process.cpu.time",
            Counter,
            "s",
            "CPU time used by the process.",
            process.cpu.map(|cpu| {
                vec![
                    point(
                        vec![("cpu.mode", Arc::clone(&labels.system))],
                        Number::Float(cpu.system.as_secs_f64()),
                    ),
                    point(
                        vec![("cpu.mode", Arc::clone(&labels.user))],
                        Number::Float(cpu.user.as_secs_f64()),
                    ),
                ]
            }),
        ));
        families.push(optional(
            "process.memory.usage",
            UpDownCounter,
            "By",
            "Physical memory used by the process.",
            process.memory_usage.map(|bytes| single(Number::Int(bytes))),
        ));
        families.push(optional(
            "process.memory.virtual",
            UpDownCounter,
            "By",
            "Virtual memory committed by the process.",
            process
                .memory_virtual
                .map(|bytes| single(Number::Int(bytes))),
        ));
        families.push(optional(
            "process.thread.count",
            UpDownCounter,
            "{thread}",
            "Threads of the process.",
            process.threads.map(|threads| single(Number::Int(threads))),
        ));
        match process.open {
            OpenCount::FileDescriptors(open) => families.push(optional(
                "process.unix.file_descriptor.count",
                UpDownCounter,
                "{file_descriptor}",
                "Open file descriptors of the process.",
                open.map(|open| single(Number::Int(open))),
            )),
            OpenCount::Handles(open) => families.push(optional(
                "process.windows.handle.count",
                UpDownCounter,
                "{handle}",
                "Open handles of the process.",
                open.map(|open| single(Number::Int(open))),
            )),
            OpenCount::NotSupported => {}
        }

        MetricsSnapshot {
            as_of: self.clock.wall(),
            service_version: self.service_version,
            families,
        }
    }

    /// Adds one history point read the same way as `snapshot`
    /// (metrics-v1 §1.8 "값"); every sixth one also goes into the 24-hour
    /// history. The serve task calls it every 10 seconds from readiness on.
    pub fn sample_history(&self) {
        let snapshot = self.snapshot();
        let values = history_values(&snapshot);
        super::lock(&self.history).push(snapshot.as_of, values);
    }

    /// History points of `window` after `after` (management-api-v1 §1.16
    /// `history_window`, `history_after`); every kept point when `None`.
    pub fn history_after(&self, window: Window, after: Option<u64>) -> HistoryView {
        super::lock(&self.history).after(window, after)
    }

    /// L and Lm in force now (metrics-v1 `*.limit`).
    fn limits(&self) -> (u32, u32) {
        let mut limits = (0, 0);
        self.sources.settings.with_current(&mut |settings| {
            limits = (
                settings.limits.relay_connections,
                settings.limits.admin_connections,
            );
        });
        limits
    }
}

/// Attribute values that every snapshot writes, made once so a snapshot
/// clones `Arc`s instead of allocating strings. Lists are in the sample
/// order of metrics-v1 §1.6.
struct Labels {
    initial_exchange: Arc<str>,
    target_connect: Arc<str>,
    roles: [(Role, Arc<str>); 2],
    transports: [(Transport, Arc<str>); 3],
    modes: [(Mode, Arc<str>); 2],
    target_to_viewer: Arc<str>,
    viewer_to_target: Arc<str>,
    system: Arc<str>,
    user: Arc<str>,
}

impl Labels {
    fn new() -> Self {
        Self {
            initial_exchange: Arc::from("initial_exchange"),
            target_connect: Arc::from("target_connect"),
            roles: [Role::Server, Role::Viewer].map(|role| (role, Arc::from(role.as_str()))),
            transports: [Transport::Tcp, Transport::Ws, Transport::Wss]
                .map(|transport| (transport, Arc::from(transport.as_str()))),
            modes: [Mode::Mode1, Mode::Mode2].map(|mode| (mode, Arc::from(mode.as_str()))),
            target_to_viewer: Arc::from("target_to_viewer"),
            viewer_to_target: Arc::from("viewer_to_target"),
            system: Arc::from("system"),
            user: Arc::from("user"),
        }
    }
}

fn family(
    name: &'static str,
    kind: MetricKind,
    unit: &'static str,
    help: &'static str,
    points: Vec<MetricPoint>,
) -> MetricFamily {
    MetricFamily {
        name,
        kind,
        unit,
        help,
        available: true,
        bounds: &[],
        points,
    }
}

/// A family whose value may be unreadable: `None` gives `available = false`
/// and no points (metrics-v1 §1.1 "읽지 못한 값").
fn optional(
    name: &'static str,
    kind: MetricKind,
    unit: &'static str,
    help: &'static str,
    points: Option<Vec<MetricPoint>>,
) -> MetricFamily {
    let available = points.is_some();
    MetricFamily {
        available,
        ..family(name, kind, unit, help, points.unwrap_or_default())
    }
}

fn histogram(
    name: &'static str,
    help: &'static str,
    bounds: &'static [f64],
    points: Vec<MetricPoint>,
) -> MetricFamily {
    MetricFamily {
        bounds,
        ..family(name, MetricKind::Histogram, "s", help, points)
    }
}

fn point(attributes: Vec<(&'static str, Arc<str>)>, value: Number) -> MetricPoint {
    MetricPoint {
        attributes,
        value: PointValue::Number(value),
    }
}

fn single(value: Number) -> Vec<MetricPoint> {
    vec![point(Vec::new(), value)]
}

fn count(value: usize) -> Number {
    Number::Int(value as u64)
}

/// The series values of metrics-v1 §1.8 from one snapshot, in
/// `HISTORY_KEYS` order.
fn history_values(snapshot: &MetricsSnapshot) -> [Option<Number>; HISTORY_KEYS.len()] {
    let find = |name: &str| {
        snapshot
            .families
            .iter()
            .find(|family| family.name == name && family.available)
    };
    let sum = |name: &str| find(name).map(|family| sum_points(family, |_| true));
    let io = |direction: &str| {
        find("vnc_relay.session.io").map(|family| {
            sum_points(family, |point| {
                point
                    .attributes
                    .iter()
                    .any(|(name, value)| *name == "direction" && &**value == direction)
            })
        })
    };
    [
        sum("vnc_relay.session.count"),
        sum("vnc_relay.waiting.count"),
        sum("vnc_relay.connection.accepted"),
        sum("vnc_relay.connection.rejected"),
        sum("vnc_relay.connection.aborted"),
        io("viewer_to_target"),
        io("target_to_viewer"),
        sum("process.cpu.time"),
        sum("process.memory.usage"),
    ]
}

/// The sum of the selected points: an integer sum for integer points, a
/// decimal sum when any point is decimal.
fn sum_points(family: &MetricFamily, select: impl Fn(&MetricPoint) -> bool) -> Number {
    let mut int = 0_u64;
    let mut float = 0.0_f64;
    let mut decimal = false;
    for point in family.points.iter().filter(|point| select(point)) {
        match point.value {
            PointValue::Number(Number::Int(value)) => int += value,
            PointValue::Number(Number::Float(value)) => {
                decimal = true;
                float += value;
            }
            PointValue::Histogram { .. } => {}
        }
    }
    if decimal {
        Number::Float(float + int as f64)
    } else {
        Number::Int(int)
    }
}
