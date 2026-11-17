#![cfg(windows)]

use std::fs::{self, OpenOptions};
use std::io;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// spec: windows-tray.md:TRAY-002 partial
#[test]
fn second_process_exits_successfully_before_creating_an_icon()
-> Result<(), Box<dyn std::error::Error>> {
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let root = std::env::temp_dir().join(format!(
        "vnc-relay-tray-child-{}-{nonce}",
        std::process::id()
    ));
    let directory = root.join("vnc-relay");
    fs::create_dir_all(&directory)?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(directory.join("tray.lock"))?;
    lock.try_lock()?;
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut child = Command::new(env!("CARGO_BIN_EXE_vnc-relay-tray"))
            .env("LOCALAPPDATA", &root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = child.try_wait()? {
                assert_eq!(status.code(), Some(0));
                return Ok(());
            }
            if Instant::now() >= deadline {
                child.kill()?;
                child.wait()?;
                return Err(
                    io::Error::new(io::ErrorKind::TimedOut, "second tray did not exit").into(),
                );
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    })();
    drop(lock);
    fs::remove_dir_all(root)?;
    result
}
