//! The history of a few values for the dashboard charts in two windows: the
//! last hour every 10 seconds and the last 24 hours every minute
//! (metrics-v1 §1.8). Design: docs/architectures/metrics.md §3.

use std::collections::VecDeque;
use std::time::{Duration, SystemTime};

use super::snapshot::Number;

/// One-hour points per 24-hour point: the 24-hour history takes the
/// one-hour points whose `seq` is a multiple of this (metrics-v1 §1.8
/// "두 기간의 관계").
const DAY_STRIDE: u64 = 6;

/// A history window of metrics-v1 §1.8, chosen by the `history_window`
/// parameter of management-api-v1 §1.16.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Window {
    /// `1h`: a point every 10 seconds, 360 points.
    Hour,
    /// `24h`: a point every minute, 1440 points.
    Day,
}

impl Window {
    /// Time between two points (metrics-v1 §1.8 "간격과 개수"). Points are
    /// taken at the `Hour` interval.
    pub const fn interval(self) -> Duration {
        match self {
            Self::Hour => Duration::from_secs(10),
            Self::Day => Duration::from_secs(10 * DAY_STRIDE),
        }
    }

    /// Points kept; the oldest goes first when full.
    pub const fn capacity(self) -> usize {
        match self {
            Self::Hour => 360,
            Self::Day => 1440,
        }
    }
}

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
pub struct Point {
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
    pub points: Vec<Point>,
}

/// Both windows. Every point goes into the one-hour history. The points whose
/// one-hour `seq` is a multiple of six, the first one included, also go into
/// the 24-hour history with the same `at` and values. Each window numbers
/// its own points.
#[derive(Debug)]
pub struct History {
    hour: Ring,
    day: Ring,
}

impl History {
    pub fn new() -> Self {
        Self {
            hour: Ring::new(Window::Hour.capacity()),
            day: Ring::new(Window::Day.capacity()),
        }
    }

    /// Adds a one-hour point, and the same point to the 24-hour history when
    /// its one-hour `seq` is a multiple of six.
    pub fn push(&mut self, at: SystemTime, values: [Option<Number>; HISTORY_KEYS.len()]) {
        if self.hour.next_seq.is_multiple_of(DAY_STRIDE) {
            self.day.push(at, values);
        }
        self.hour.push(at, values);
    }

    /// Kept points of `window` with `seq` greater than `after`; every kept
    /// point when `after` is `None` (management-api-v1 §1.16 `history_after`).
    pub fn after(&self, window: Window, after: Option<u64>) -> HistoryView {
        match window {
            Window::Hour => self.hour.after(after),
            Window::Day => self.day.after(after),
        }
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

/// A ring of at most `capacity` points. Sequence numbers start at 0, grow by
/// one per point, and are never reused while the process runs.
#[derive(Debug)]
struct Ring {
    points: VecDeque<Point>,
    capacity: usize,
    next_seq: u64,
}

impl Ring {
    fn new(capacity: usize) -> Self {
        Self {
            points: VecDeque::with_capacity(capacity),
            capacity,
            next_seq: 0,
        }
    }

    /// Adds a point, dropping the oldest when the ring is full.
    fn push(&mut self, at: SystemTime, values: [Option<Number>; HISTORY_KEYS.len()]) {
        if self.points.len() == self.capacity {
            self.points.pop_front();
        }
        self.points.push_back(Point {
            seq: self.next_seq,
            at,
            values,
        });
        self.next_seq += 1;
    }

    fn after(&self, after: Option<u64>) -> HistoryView {
        let start = after.map_or(0, |after| {
            self.points.partition_point(|point| point.seq <= after)
        });
        HistoryView {
            latest_seq: self.points.back().map(|point| point.seq),
            points: self.points.range(start..).cloned().collect(),
        }
    }
}
