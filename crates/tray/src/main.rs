#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod copy;
#[cfg(windows)]
#[expect(
    unsafe_code,
    reason = "Win32 message loop that tray-icon needs; see docs/conventions/rust.md"
)]
mod message_loop;
#[cfg(windows)]
mod registry;
#[cfg(windows)]
mod rules;
#[cfg(windows)]
mod single_instance;
#[cfg(windows)]
mod ui;
#[cfg(windows)]
mod worker;

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(_) => std::process::ExitCode::FAILURE,
    }
}

#[cfg(windows)]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let local = std::env::var_os("LOCALAPPDATA").ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "LOCALAPPDATA is missing")
    })?;
    let directory = std::path::PathBuf::from(local).join("vnc-relay");
    std::fs::create_dir_all(&directory)?;
    let Some(_instance) = single_instance::acquire(&directory.join("tray.lock"))? else {
        return Ok(());
    };
    ui::run()
}

/// The tray app exists only on Windows 11 (ADR-0010).
#[cfg(not(windows))]
fn main() -> std::process::ExitCode {
    std::process::ExitCode::FAILURE
}
