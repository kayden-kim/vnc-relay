//! Shared helpers for the real-process settings view (SCR-004) and edit (SCR-005)
//! live browser tests in `settings_view`, `settings_edit`, and `settings_recovery`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

use super::live::{Browser, PASSWORD, Server, must};

// Keep real file bytes pending without depending on OS notification kinds.
// file_watch.rs: ReloadWatcher::episode compares stamps() across GAP=100ms;
// stamp_of includes modified time and length. Touching every 50ms prevents that
// stable pair even if metadata-only notifications do not restart QUIET=300ms.
// No byte writes race GET/PATCH. Drop stops/joins; nothing after the test relies
// on whether the watcher later applies the external file change.
pub(super) struct PendingFile {
    stop: Arc<AtomicBool>,
    task: Option<std::thread::JoinHandle<()>>,
}

impl PendingFile {
    pub(super) fn write(server: &Server, contents: &str) -> Self {
        server.config_write(contents);
        let file = must(
            std::fs::OpenOptions::new()
                .write(true)
                .open(server.config_path()),
        );
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let task = std::thread::spawn(move || {
            while !stopping.load(Ordering::Acquire) {
                must(file.set_modified(SystemTime::now()));
                std::thread::sleep(Duration::from_millis(50));
            }
        });
        Self {
            stop,
            task: Some(task),
        }
    }
}

impl Drop for PendingFile {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(task) = self.task.take() {
            must(task.join());
        }
    }
}

pub(super) struct PermissionGuard {
    dir: PathBuf,
    active: bool,
}

impl PermissionGuard {
    pub fn deny(dir: &Path) -> Self {
        #[cfg(windows)]
        {
            let username = std::env::var("USERNAME").unwrap_or_else(|_| "CURRENT_USER".into());
            let status = std::process::Command::new("icacls")
                .arg(dir)
                .arg("/deny")
                .arg(format!("{username}:(WD,AD)"))
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .unwrap_or_else(|error| panic!("icacls deny: {error}"));
            assert!(status.success(), "icacls deny failed");
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perm = std::fs::Permissions::from_mode(0o555);
            std::fs::set_permissions(dir, perm)
                .unwrap_or_else(|error| panic!("set 0o555: {error}"));
        }
        let mut guard = Self {
            dir: dir.to_path_buf(),
            active: true,
        };
        let probe = dir.join(".permission_probe");
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&probe)
        {
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {}
            Ok(_) => {
                let _ = std::fs::remove_file(&probe);
                guard.restore();
                panic!(
                    "PermissionGuard::deny did not restrict directory write access; root CAP_DAC_OVERRIDE on Linux or USERNAME mismatch on Windows"
                );
            }
            Err(error) => {
                guard.restore();
                panic!("PermissionGuard probe check unexpected error: {error}");
            }
        }
        guard
    }

    pub fn restore(&mut self) {
        if self.active {
            #[cfg(windows)]
            {
                let username = std::env::var("USERNAME").unwrap_or_else(|_| "CURRENT_USER".into());
                let _ = std::process::Command::new("icacls")
                    .arg(&self.dir)
                    .arg("/remove:d")
                    .arg(&username)
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status();
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perm = std::fs::Permissions::from_mode(0o755);
                let _ = std::fs::set_permissions(&self.dir, perm);
            }
            self.active = false;
        }
    }
}

impl Drop for PermissionGuard {
    fn drop(&mut self) {
        self.restore();
    }
}

pub(super) fn script_str(value: &str) -> String {
    must(serde_json::to_string(value))
}

pub(super) fn login_alice(browser: &mut Browser, server: &Server) {
    browser.navigate(&server.url("/login/"));
    browser.fill("#login-account", "alice");
    browser.fill("#login-password", PASSWORD);
    browser.click("#login-submit");
    browser.wait_with_server(
        "document.querySelector('#operations-refresh') !== null",
        server,
    );
    browser.wait(
        "!document.body.innerText.includes('Loading') && !document.body.innerText.includes('조회 중')",
    );
}

pub(super) fn open_settings(browser: &mut Browser, server: &Server) {
    browser.click("a.navbar-item[href='/#/settings']");
    browser.wait_with_server(
        "location.hash === '#/settings' && document.querySelector('#settings-refresh') !== null",
        server,
    );
    browser.wait(
        "!document.body.innerText.includes('Loading') && !document.body.innerText.includes('조회 중')",
    );
}

pub(super) fn open_settings_edit(browser: &mut Browser, server: &Server) {
    browser.click("#settings-edit");
    browser.wait_with_server(
        "location.hash === '#/settings/edit' && document.querySelector('#settings-review') !== null",
        server,
    );
    browser.wait(
        "!document.body.innerText.includes('Loading') && !document.body.innerText.includes('조회 중')",
    );
}

pub(super) fn set_field_value(browser: &mut Browser, key: &str, value: &str) {
    let key_js = script_str(key);
    let val_js = script_str(value);
    browser.eval(&format!(
        r#"(() => {{
            const el = document.querySelector('[data-config-key="' + {key_js} + '"]');
            if (!el) throw new Error("field not found: " + {key_js});
            el.focus();
            el.value = {val_js};
            el.dispatchEvent(new Event('input', {{ bubbles: true }}));
            el.dispatchEvent(new Event('change', {{ bubbles: true }}));
        }})()"#
    ));
}

pub(super) fn get_field_value(browser: &mut Browser, key: &str) -> String {
    let key_js = script_str(key);
    browser
        .eval(&format!(
            "document.querySelector('[data-config-key=\"' + {key_js} + '\"]')?.value ?? ''"
        ))
        .as_str()
        .unwrap_or("")
        .to_owned()
}
