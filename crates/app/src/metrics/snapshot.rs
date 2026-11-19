//! One read of every metric (metrics-v1 §1.3). The status listener writes it
//! as Prometheus text and the management API as JSON, so both show the same
//! values. Design: docs/architectures/metrics.md §3.

use std::sync::Arc;
use std::time::SystemTime;

/// The instrument kind of metrics-v1 §1.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricKind {
    Counter,
    UpDownCounter,
    Gauge,
    Histogram,
}

impl MetricKind {
    /// The JSON `type` value (metrics-v1 §1.7).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Counter => "counter",
            Self::UpDownCounter => "updowncounter",
            Self::Gauge => "gauge",
            Self::Histogram => "histogram",
        }
    }
}

/// A number as metrics-v1 writes it: counts and bytes as integers, seconds
/// as decimals (metrics-v1 §1.6 "수 표기", §1.7 `points[].value`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Number {
    Int(u64),
    Float(f64),
}

/// The value of one point.
#[derive(Debug, Clone, PartialEq)]
pub enum PointValue {
    Number(Number),
    /// `bucket_counts` holds per-bucket counts, not cumulative ones. Its
    /// length is the bound count + 1, and its sum is `count` (metrics-v1 §1.7).
    Histogram {
        count: u64,
        sum: f64,
        bucket_counts: Vec<u64>,
    },
}

/// One attribute combination of a metric.
#[derive(Debug, Clone, PartialEq)]
pub struct MetricPoint {
    /// The attributes this point has, in metrics-v1 §1.3 order, with their
    /// OpenTelemetry names (`cpu.mode`, not `cpu_mode`). An attribute the
    /// point does not have is left out (metrics-v1 §1.3 `config.reloads`).
    pub attributes: Vec<(&'static str, Arc<str>)>,
    pub value: PointValue,
}

/// One metric of metrics-v1 §1.3.
#[derive(Debug, Clone, PartialEq)]
pub struct MetricFamily {
    /// The OpenTelemetry name, such as `vnc_relay.connection.accepted`.
    pub name: &'static str,
    pub kind: MetricKind,
    /// The unit of metrics-v1 §1.3: `s`, `By`, or a braced unit such as
    /// `{connection}`.
    pub unit: &'static str,
    /// The English one-line `# HELP` text. Not part of the contract.
    pub help: &'static str,
    /// `false` when the value could not be read. `points` is then empty
    /// (metrics-v1 §1.1 "읽지 못한 값").
    pub available: bool,
    /// The bounds of metrics-v1 §1.5 in seconds. Empty unless `kind` is
    /// `Histogram`.
    pub bounds: &'static [f64],
    /// Points in the sample order of metrics-v1 §1.6: attribute values
    /// compared in attribute order, an absent attribute before any value.
    pub points: Vec<MetricPoint>,
}

/// Every metric this platform has, at one moment, in metrics-v1 §1.3 order.
#[derive(Debug, Clone, PartialEq)]
pub struct MetricsSnapshot {
    /// When the snapshot was made (`as_of` of metrics-v1 §1.7).
    pub as_of: SystemTime,
    /// `service.version` of metrics-v1 §1.2: the `vnc-relay --version` text.
    pub service_version: &'static str,
    pub families: Vec<MetricFamily>,
}
