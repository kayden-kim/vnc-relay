//! System tests of the `vnc-relay` command line outside `serve`
//! (docs/contracts/cli-v1.md §1.1, §1.5, §2.1).

use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_vnc-relay");

fn run(args: &[&str]) -> Output {
    Command::new(BIN)
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("run {BIN}: {error}"))
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn help_and_version_write_to_stdout_with_exit_code_0() {
    let help = run(&["--help"]);
    assert_eq!(help.status.code(), Some(0));
    assert!(text(&help.stdout).contains("vnc-relay serve --config <path>"));
    assert_eq!(text(&help.stderr), "");

    let version = run(&["-V"]);
    assert_eq!(version.status.code(), Some(0));
    assert_eq!(
        text(&version.stdout),
        format!("vnc-relay {}\n", env!("CARGO_PKG_VERSION"))
    );
    assert_eq!(text(&version.stderr), "");
}

#[test]
fn usage_errors_exit_with_code_2_and_usage_on_stderr() {
    for args in [
        &[][..],
        &["admin", "list", "--config", "relay.toml"],
        &["serve"],
        &["serve", "--config", "a.toml", "--config", "b.toml"],
        &["--help", "serve", "--config", "relay.toml"],
        &["start"],
    ] {
        let output = run(args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert_eq!(text(&output.stdout), "", "{args:?}");
        let stderr = text(&output.stderr);
        assert!(
            stderr.starts_with("error[usage_error]: "),
            "{args:?}: {stderr}"
        );
        assert!(
            stderr.contains("사용법: vnc-relay serve --config <path>"),
            "{stderr}"
        );
    }
}
