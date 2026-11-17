//! A surviving file is harmless: the OS lock belongs to the open handle.

use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::path::Path;

pub fn acquire(path: &Path) -> io::Result<Option<File>> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(error)) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn temporary_path() -> std::path::PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        std::env::temp_dir().join(format!(
            "vnc-relay-tray-lock-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    /// spec: windows-tray.md:TRAY-002 partial
    #[test]
    fn acquires_a_stale_unlocked_file() -> io::Result<()> {
        let path = temporary_path();
        File::create(&path)?;
        let held = acquire(&path)?;
        assert!(held.is_some());
        drop(held);
        std::fs::remove_file(path)
    }

    /// spec: windows-tray.md:TRAY-002 partial
    #[test]
    fn detects_another_handle_and_reacquires_after_release() -> io::Result<()> {
        let path = temporary_path();
        let held = acquire(&path)?;
        assert!(held.is_some());
        assert!(acquire(&path)?.is_none());
        drop(held);
        let again = acquire(&path)?;
        assert!(again.is_some());
        drop(again);
        std::fs::remove_file(path)
    }
}
