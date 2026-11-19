//! Reads Win32 current-process values. See metrics-v1 §1.3.

use std::mem::size_of;
use std::ptr;
use std::time::Duration;

use vnc_relay_app::ports::{CpuTimes, OpenCount, ProcessStats};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_NO_MORE_FILES, FILETIME, GetLastError, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
};
use windows_sys::Win32::System::ProcessStatus::{
    K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentProcessId, GetProcessHandleCount, GetProcessTimes,
};

pub(super) fn read() -> ProcessStats {
    // SAFETY: GetCurrentProcess has no preconditions. Its pseudo handle remains
    // valid for this process and must not be closed.
    let process = unsafe { GetCurrentProcess() };
    let cpu = read_cpu(process);
    let (memory_usage, memory_virtual) = read_memory(process);
    let open = OpenCount::Handles(read_handles(process));
    ProcessStats {
        cpu,
        memory_usage,
        memory_virtual,
        threads: read_threads(),
        open,
    }
}

fn read_cpu(process: HANDLE) -> Option<CpuTimes> {
    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    // SAFETY: process is the current-process pseudo handle. Each output points
    // to a separate, initialized FILETIME that stays writable through the call.
    if unsafe { GetProcessTimes(process, &mut creation, &mut exit, &mut kernel, &mut user) } == 0 {
        return None;
    }
    Some(CpuTimes {
        user: filetime_duration(user),
        system: filetime_duration(kernel),
    })
}

fn filetime_duration(time: FILETIME) -> Duration {
    let ticks = (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime);
    // FILETIME uses 100 ns ticks. Split before multiplying to avoid overflow.
    Duration::new(ticks / 10_000_000, ((ticks % 10_000_000) * 100) as u32)
}

fn read_memory(process: HANDLE) -> (Option<u64>, Option<u64>) {
    let size = size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
    let mut counters = PROCESS_MEMORY_COUNTERS_EX {
        cb: size,
        ..Default::default()
    };
    // SAFETY: process is the current-process pseudo handle. The repr(C) EX
    // structure starts with PROCESS_MEMORY_COUNTERS fields. Both size arguments
    // describe the full writable EX buffer, so the API also fills PrivateUsage.
    if unsafe { K32GetProcessMemoryInfo(process, ptr::from_mut(&mut counters).cast(), size) } == 0 {
        return (None, None);
    }
    (
        Some(counters.WorkingSetSize as u64),
        Some(counters.PrivateUsage as u64),
    )
}

fn read_handles(process: HANDLE) -> Option<u64> {
    let mut count = 0;
    // SAFETY: process is the current-process pseudo handle, and count points
    // to a writable u32 for the duration of the call.
    if unsafe { GetProcessHandleCount(process, &mut count) } == 0 {
        return None;
    }
    Some(u64::from(count))
}

struct ThreadSnapshot(HANDLE);

impl Drop for ThreadSnapshot {
    fn drop(&mut self) {
        // SAFETY: this guard owns a successful CreateToolhelp32Snapshot handle,
        // never the current-process pseudo handle. It closes the snapshot once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

fn read_threads() -> Option<u64> {
    // SAFETY: these flags request a read-only system thread snapshot. The
    // process ID argument is ignored for TH32CS_SNAPTHREAD.
    let handle = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if handle == INVALID_HANDLE_VALUE {
        return None;
    }
    let snapshot = ThreadSnapshot(handle);
    // SAFETY: GetCurrentProcessId has no preconditions.
    let process_id = unsafe { GetCurrentProcessId() };
    let size = size_of::<THREADENTRY32>() as u32;
    let mut entry = THREADENTRY32 {
        dwSize: size,
        ..Default::default()
    };
    // SAFETY: snapshot owns a valid thread snapshot. entry is a writable
    // THREADENTRY32 with dwSize set to its actual size.
    if unsafe { Thread32First(snapshot.0, &mut entry) } == 0 {
        return None;
    }
    let mut count = 0_u64;
    loop {
        if entry.th32OwnerProcessID == process_id {
            count += 1;
        }
        entry.dwSize = size;
        // SAFETY: snapshot remains open, and entry is a writable THREADENTRY32
        // with dwSize reset before each call.
        if unsafe { Thread32Next(snapshot.0, &mut entry) } == 0 {
            // SAFETY: GetLastError has no preconditions and immediately follows
            // the failed enumeration call on the same thread.
            return if unsafe { GetLastError() } == ERROR_NO_MORE_FILES {
                Some(count)
            } else {
                None
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_filetime_ticks_to_seconds_and_nanoseconds() {
        assert_eq!(
            filetime_duration(FILETIME {
                dwLowDateTime: 12_345_678,
                dwHighDateTime: 0,
            }),
            Duration::new(1, 234_567_800)
        );
        assert_eq!(
            filetime_duration(FILETIME {
                dwLowDateTime: u32::MAX,
                dwHighDateTime: u32::MAX,
            }),
            Duration::new(
                u64::MAX / 10_000_000,
                ((u64::MAX % 10_000_000) * 100) as u32
            )
        );
    }
}
