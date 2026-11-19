//! Reads current-process statistics for [the metrics module design](../../../docs/architectures/metrics.md).
//! See [metrics-v1 §1.3](../../../docs/contracts/metrics-v1.md#13-메트릭-목록).

use vnc_relay_app::ports::{ProcessStats, ProcessStatsSource};

#[cfg(any(target_os = "linux", test))]
mod linux;
#[cfg(windows)]
#[expect(
    unsafe_code,
    reason = "Win32 process statistics for metrics; see docs/conventions/rust.md"
)]
mod windows;

/// Reads each supported process statistic without logging read failures.
/// See metrics-v1 §1.1 and §1.3.
#[derive(Debug, Default)]
pub struct OsProcessStats;

impl OsProcessStats {
    /// Creates a stateless reader. See docs/architectures/metrics.md §2.
    pub fn new() -> Self {
        Self
    }
}

impl ProcessStatsSource for OsProcessStats {
    fn read(&self) -> ProcessStats {
        #[cfg(target_os = "linux")]
        {
            linux::read()
        }
        #[cfg(windows)]
        {
            windows::read()
        }
        #[cfg(not(any(target_os = "linux", windows)))]
        {
            ProcessStats {
                cpu: None,
                memory_usage: None,
                memory_virtual: None,
                threads: None,
                open: vnc_relay_app::ports::OpenCount::NotSupported,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(any(target_os = "linux", windows))]
    use super::*;
    #[cfg(any(target_os = "linux", windows))]
    use vnc_relay_app::ports::OpenCount;

    /// spec: operations.md:OPS-090 partial
    #[cfg(target_os = "linux")]
    #[test]
    fn reads_current_linux_process_statistics() {
        let stats = OsProcessStats::new().read();
        assert!(stats.cpu.is_some());
        assert!(stats.memory_usage.is_some_and(|value| value > 0));
        assert!(stats.memory_virtual.is_some_and(|value| value > 0));
        assert!(stats.threads.is_some_and(|value| value >= 1));
        assert!(matches!(stats.open, OpenCount::FileDescriptors(Some(value)) if value > 0));
    }

    /// spec: operations.md:OPS-090 partial
    #[cfg(windows)]
    #[test]
    fn reads_current_windows_process_statistics() {
        let stats = OsProcessStats::new().read();
        assert!(stats.cpu.is_some());
        assert!(stats.memory_usage.is_some_and(|value| value > 0));
        assert!(stats.memory_virtual.is_some_and(|value| value > 0));
        assert!(stats.threads.is_some_and(|value| value >= 1));
        assert!(matches!(stats.open, OpenCount::Handles(Some(value)) if value > 0));
    }
}
