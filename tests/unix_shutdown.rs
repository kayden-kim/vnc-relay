#![cfg(unix)]

#[path = "common/shutdown.rs"]
mod shutdown;

use std::fs::File;
use std::path::Path;
use std::process::{Child, Command, Stdio};

use shutdown::ServerProcess;

struct UnixProcess {
    child: Child,
}

impl ServerProcess for UnixProcess {
    fn start(config: &Path) -> Self {
        let log = File::create(config.with_file_name("stdout.log"))
            .unwrap_or_else(|error| panic!("create child log: {error}"));
        let errors = File::create(config.with_file_name("stderr.log"))
            .unwrap_or_else(|error| panic!("create child stderr: {error}"));
        let child = Command::new(env!("CARGO_BIN_EXE_vnc-relay"))
            .args(["serve", "--config"])
            .arg(config)
            .stdin(Stdio::null())
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(errors))
            .spawn()
            .unwrap_or_else(|error| panic!("spawn serve: {error}"));
        Self { child }
    }

    fn request_stop(&mut self) {
        let status = Command::new("kill")
            .arg("-TERM")
            .arg(self.child.id().to_string())
            .status()
            .unwrap_or_else(|error| panic!("send SIGTERM: {error}"));
        assert!(status.success(), "SIGTERM request failed: {status}");
    }

    fn try_wait(&mut self) -> Option<i32> {
        self.child
            .try_wait()
            .unwrap_or_else(|error| panic!("wait for serve: {error}"))
            .map(|status| {
                status
                    .code()
                    .unwrap_or_else(|| panic!("serve died from a signal: {status}"))
            })
    }
}

impl Drop for UnixProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// spec: operations.md:OPS-007
#[test]
fn stop_unfinished_work() {
    shutdown::stop_unfinished_work::<UnixProcess>();
}

/// spec: operations.md:OPS-008
#[test]
fn active_session_finishes_in_grace() {
    shutdown::active_session_finishes_in_grace::<UnixProcess>();
}

/// spec: operations.md:OPS-009
#[test]
fn grace_expiry_cleans_session() {
    shutdown::grace_expiry_cleans_session::<UnixProcess>();
}

/// spec: operations.md:OPS-010
#[test]
fn restart_has_fresh_state() {
    shutdown::restart_has_fresh_state::<UnixProcess>();
}

/// spec: operations.md:OPS-042
#[test]
fn second_request_forces_cleanup() {
    shutdown::second_request_forces_cleanup::<UnixProcess>();
}

/// spec: operations.md:OPS-043
/// spec: operations.md:OPS-013 partial
#[test]
fn management_works_during_grace() {
    shutdown::management_works_during_grace::<UnixProcess>();
}

/// spec: operations.md:OPS-043 partial
/// spec: operations.md:OPS-050 partial
#[test]
fn config_edit_works_during_grace_without_changing_the_started_grace_deadline() {
    shutdown::config_edit_works_during_grace_without_changing_the_started_grace_deadline::<
        UnixProcess,
    >();
}
