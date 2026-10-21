//! Saves and gives back the Windows console input mode around a hidden read
//! (docs/architectures/admin-accounts.md §5). rpassword changes the mode and
//! gives it back only when its read returns, so a Ctrl+C during a blocked read
//! would leave the changed mode in place.

use std::io;
use std::ptr;

use windows_sys::Win32::Foundation::{
    CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::Console::{CONSOLE_MODE, GetConsoleMode, SetConsoleMode};

/// The console input mode as it was before a hidden read, with the console
/// input handle that was used to read it.
pub struct SavedInputMode {
    handle: HANDLE,
    mode: CONSOLE_MODE,
}

impl SavedInputMode {
    /// Opens the console input (`CONIN$`, as rpassword does) and reads its mode.
    pub fn save() -> io::Result<Self> {
        let path: Vec<u16> = "CONIN$".encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: `path` is a NUL-terminated UTF-16 string that lives for the
        // whole call. The other arguments are flags, or null where CreateFileW
        // accepts null, as for opening a console device by name.
        let handle = unsafe {
            CreateFileW(
                path.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                ptr::null(),
                OPEN_EXISTING,
                0,
                ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        let mut mode: CONSOLE_MODE = 0;
        // SAFETY: `handle` is the valid open handle from above, and `mode` is
        // a writable u32 for the duration of the call.
        if unsafe { GetConsoleMode(handle, &mut mode) } == 0 {
            let error = io::Error::last_os_error();
            // SAFETY: `handle` was opened above and is closed only here.
            let _ = unsafe { CloseHandle(handle) };
            return Err(error);
        }
        Ok(Self { handle, mode })
    }

    /// Sets the console input mode back to the saved one.
    pub fn restore(&self) -> io::Result<()> {
        // SAFETY: `self.handle` is open until `drop` runs, which is after this call.
        if unsafe { SetConsoleMode(self.handle, self.mode) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
}

impl Drop for SavedInputMode {
    fn drop(&mut self) {
        // SAFETY: `self.handle` was opened by `save` and is closed only here.
        let _ = unsafe { CloseHandle(self.handle) };
    }
}
