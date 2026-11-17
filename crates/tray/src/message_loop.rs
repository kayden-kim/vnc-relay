//! The only direct Win32 boundary: tray-icon requires a same-thread pump.

use std::io;
use std::ptr;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MSG, MsgWaitForMultipleObjects, PM_REMOVE, PeekMessageW, QS_ALLINPUT,
    TranslateMessage, WM_QUIT,
};

pub fn run(mut wake: impl FnMut() -> bool) -> io::Result<()> {
    loop {
        // SAFETY: Zero handles permits a null handle array. This only waits for
        // this thread's messages, with a bounded timeout and no borrowed data.
        let result = unsafe { MsgWaitForMultipleObjects(0, ptr::null(), 0, 100, QS_ALLINPUT) };
        if result == u32::MAX {
            return Err(io::Error::last_os_error());
        }
        let mut message = MSG::default();
        // SAFETY: `message` is writable for the entire call. Null HWND includes
        // all messages owned by this UI thread; PM_REMOVE consumes each once.
        while unsafe { PeekMessageW(&mut message, ptr::null_mut(), 0, 0, PM_REMOVE) } != 0 {
            if message.message == WM_QUIT {
                return Ok(());
            }
            // SAFETY: This is the valid MSG just returned by PeekMessageW. Both
            // calls execute on the thread that owns the tray and menu windows.
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        if !wake() {
            return Ok(());
        }
    }
}
