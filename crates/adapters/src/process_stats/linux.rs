//! Reads Linux procfs values. See metrics-v1 §1.3.

use std::time::Duration;

use vnc_relay_app::ports::CpuTimes;
#[cfg(target_os = "linux")]
use vnc_relay_app::ports::{OpenCount, ProcessStats};

// Linux procfs CPU times use USER_HZ, which is 100, independently of the
// kernel's scheduler tick rate. std has no sysconf; no libc dependency is needed.
const USER_HZ: u64 = 100;

#[derive(Debug, Default, PartialEq, Eq)]
struct StatusValues {
    memory_usage: Option<u64>,
    memory_virtual: Option<u64>,
    threads: Option<u64>,
}

#[cfg(target_os = "linux")]
pub(super) fn read() -> ProcessStats {
    let cpu = std::fs::read_to_string("/proc/self/stat")
        .ok()
        .and_then(|text| parse_stat(&text));
    let status = std::fs::read_to_string("/proc/self/status")
        .map(|text| parse_status(&text))
        .unwrap_or_default();
    // The read_dir directory handle is itself included in /proc/self/fd.
    // Any entry error makes this count unavailable, not a partial count.
    let descriptors = std::fs::read_dir("/proc/self/fd")
        .ok()
        .and_then(|mut entries| {
            entries
                .try_fold(0_u64, |count, entry| entry.map(|_| count + 1))
                .ok()
        });
    ProcessStats {
        cpu,
        memory_usage: status.memory_usage,
        memory_virtual: status.memory_virtual,
        threads: status.threads,
        open: OpenCount::FileDescriptors(descriptors),
    }
}

fn parse_stat(text: &str) -> Option<CpuTimes> {
    // comm (field 2) can contain spaces and parentheses, so only its final
    // closing parenthesis separates it from the remaining numeric fields.
    let (_, fields) = text.rsplit_once(')')?;
    let mut fields = fields.split_whitespace();
    // The suffix starts at state (field 3); utime is field 14.
    let user = fields.nth(11)?.parse::<u64>().ok()?;
    let system = fields.next()?.parse::<u64>().ok()?;
    Some(CpuTimes {
        user: ticks_duration(user),
        system: ticks_duration(system),
    })
}

fn ticks_duration(ticks: u64) -> Duration {
    Duration::new(
        ticks / USER_HZ,
        ((ticks % USER_HZ) * (1_000_000_000 / USER_HZ)) as u32,
    )
}

fn parse_status(text: &str) -> StatusValues {
    let mut status = StatusValues::default();
    for line in text.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        match name {
            "VmRSS" => status.memory_usage = parse_kibibytes(value),
            "VmSize" => status.memory_virtual = parse_kibibytes(value),
            "Threads" => status.threads = value.trim().parse().ok(),
            _ => {}
        }
    }
    status
}

fn parse_kibibytes(text: &str) -> Option<u64> {
    let mut fields = text.split_whitespace();
    let value = fields.next()?.parse::<u64>().ok()?;
    if fields.next()? != "kB" || fields.next().is_some() {
        return None;
    }
    value.checked_mul(1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cpu_times_after_comm_with_spaces_and_parentheses() {
        let text = "42 (relay worker ) (main)) S 1 2 3 4 5 6 7 8 9 10 123 456 0 0\n";
        assert_eq!(
            parse_stat(text),
            Some(CpuTimes {
                user: Duration::from_millis(1230),
                system: Duration::from_millis(4560),
            })
        );
    }

    #[test]
    fn rejects_missing_cpu_fields_and_bad_numbers() {
        for text in [
            "",
            "42 (relay",
            "42 (relay) S 1 2 3 4 5 6 7 8 9 10",
            "42 (relay) S 1 2 3 4 5 6 7 8 9 10 123",
            "42 (relay) S 1 2 3 4 5 6 7 8 9 10 bad 456",
            "42 (relay) S 1 2 3 4 5 6 7 8 9 10 123 bad",
            "42 (relay) S 1 2 3 4 5 6 7 8 9 10 -1 456",
        ] {
            assert_eq!(parse_stat(text), None, "input: {text}");
        }
    }

    #[test]
    fn converts_large_tick_counts_without_overflow() {
        assert_eq!(
            ticks_duration(u64::MAX),
            Duration::new(u64::MAX / 100, ((u64::MAX % 100) * 10_000_000) as u32)
        );
    }

    #[test]
    fn parses_memory_in_bytes_and_thread_count() {
        assert_eq!(
            parse_status("Name:\trelay\nVmSize:\t 2048 kB\nVmRSS:\t123 kB\nThreads:\t7\n"),
            StatusValues {
                memory_usage: Some(123 * 1024),
                memory_virtual: Some(2048 * 1024),
                threads: Some(7),
            }
        );
    }

    #[test]
    fn leaves_only_unreadable_status_values_unavailable() {
        assert_eq!(parse_status("Name: relay\n"), StatusValues::default());
        assert_eq!(
            parse_status("VmRSS: bad kB\nVmSize: 2048 kB\nThreads: 7\n"),
            StatusValues {
                memory_usage: None,
                memory_virtual: Some(2048 * 1024),
                threads: Some(7),
            }
        );
        assert_eq!(
            parse_status("VmRSS: 123 kB\nVmSize: bad kB\nThreads: bad\n"),
            StatusValues {
                memory_usage: Some(123 * 1024),
                memory_virtual: None,
                threads: None,
            }
        );
    }

    #[test]
    fn rejects_missing_units_overflow_and_malformed_status_values() {
        for text in [
            "VmRSS: 12\nVmSize: 12 MB\nThreads: -1\n",
            "VmRSS: 18446744073709551615 kB\nVmSize: 12 kB extra\nThreads:\n",
            "VmRSS:\nVmSize: -1 kB\nThreads: 7 extra\n",
        ] {
            assert_eq!(parse_status(text), StatusValues::default(), "input: {text}");
        }
    }
}
