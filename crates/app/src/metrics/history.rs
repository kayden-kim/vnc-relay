//! The 10-second history of a few values for the dashboard charts
//! (metrics-v1 §1.8). Design: docs/architectures/metrics.md §3.

use std::collections::VecDeque;
use std::time::{Duration, SystemTime};

use super::snapshot::Number;

/// Time between two points (metrics-v1 §1.8 "간격과 개수").
pub const HISTORY_INTERVAL: Duration = Duration::from_secs(10);

/// Points kept; the oldest goes first when full (metrics-v1 §1.8).
pub const HISTORY_CAPACITY: usize = 360;

/// Series keys of a point's `values`, in this order (metrics-v1 §1.8).
pub const HISTORY_KEYS: [&str; 9] = [
    "session_count",
    "waiting_count",
    "connection_accepted",
    "connection_rejected",
    "connection_aborted",
    "session_io_viewer_to_target",
    "session_io_target_to_viewer",
    "process_cpu_time",
    "process_memory_usage",
];

/// One history point. `values[i]` is the value of `HISTORY_KEYS[i]`; `None`
/// is a value that could not be read (JSON `null`).
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryPoint {
    pub seq: u64,
    pub at: SystemTime,
    pub values: [Option<Number>; HISTORY_KEYS.len()],
}

/// The points after a given sequence number, with the newest kept number.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryView {
    /// `seq` of the newest kept point; `None` before the first point.
    pub latest_seq: Option<u64>,
    /// Points in ascending `seq` order.
    pub points: Vec<HistoryPoint>,
}

/// A ring of at most `HISTORY_CAPACITY` points. Sequence numbers start at 0,
/// grow by one per point, and are never reused while the process runs.
#[derive(Debug, Default)]
pub struct History {
    points: VecDeque<HistoryPoint>,
    next_seq: u64,
}

impl History {
    pub fn new() -> Self {
        Self {
            points: VecDeque::with_capacity(HISTORY_CAPACITY),
            next_seq: 0,
        }
    }

    /// Adds a point, dropping the oldest when the ring is full.
    pub fn push(&mut self, at: SystemTime, values: [Option<Number>; HISTORY_KEYS.len()]) {
        if self.points.len() == HISTORY_CAPACITY {
            self.points.pop_front();
        }
        self.points.push_back(HistoryPoint {
            seq: self.next_seq,
            at,
            values,
        });
        self.next_seq += 1;
    }

    /// Kept points with `seq` greater than `after`; every kept point when
    /// `after` is `None` (management-api-v1 §1.16 `history_after`).
    pub fn after(&self, after: Option<u64>) -> HistoryView {
        let start = after.map_or(0, |after| {
            self.points.partition_point(|point| point.seq <= after)
        });
        HistoryView {
            latest_seq: self.points.back().map(|point| point.seq),
            points: self.points.range(start..).cloned().collect(),
        }
    }
}
