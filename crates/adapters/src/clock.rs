//! The process clock (app `Clock` port) and the UTC timestamp text of
//! ops-info-v1 §1.2 and management-api-v1 §1.1. Design:
//! docs/architectures/management-session.md.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use vnc_relay_app::ports::Clock;

/// The process clock. `monotonic` counts from the moment the clock is made;
/// `wall` reads the system time.
#[derive(Debug, Clone, Copy)]
pub struct SystemClock {
    started: Instant,
}

impl SystemClock {
    /// A clock that starts counting now.
    pub fn new() -> Self {
        Self {
            started: Instant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    fn monotonic(&self) -> Duration {
        self.started.elapsed()
    }

    fn wall(&self) -> SystemTime {
        SystemTime::now()
    }
}

/// UTC ISO 8601 with milliseconds and `Z` (ops-info-v1 §1.2). Times before
/// 1970 do not occur on a running server and are written as 1970.
pub fn timestamp(at: SystemTime) -> String {
    let since = at.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = since.as_secs();
    let (year, month, day) = civil_date(secs / 86_400);
    let time = secs % 86_400;
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        time / 3600,
        time % 3600 / 60,
        time % 60,
        since.subsec_millis()
    )
}

/// Gregorian (year, month, day) of a day count since 1970-01-01. Counts in
/// 400-year eras of 146 097 days that start on 0000-03-01, so a leap day is
/// the last day of its era year.
fn civil_date(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let day_of_era = z % 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = if month_from_march < 10 {
        month_from_march + 3
    } else {
        month_from_march - 9
    };
    let year = era * 400 + year_of_era + u64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_utc_timestamps_with_milliseconds_across_leap_years() {
        let cases = [
            (0, "1970-01-01T00:00:00.000Z"),
            (951_868_800_000, "2000-03-01T00:00:00.000Z"),
            (1_709_251_199_999, "2024-02-29T23:59:59.999Z"),
            (1_792_112_523_456, "2026-10-16T01:02:03.456Z"),
            (4_107_585_600_001, "2100-03-01T12:00:00.001Z"),
        ];
        for (millis, text) in cases {
            assert_eq!(timestamp(at_millis(millis)), text, "{millis}");
        }
    }

    fn at_millis(millis: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_millis(millis)
    }
}
