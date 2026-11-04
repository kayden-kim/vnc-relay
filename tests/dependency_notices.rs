//! OPS-021's dependency notices only. The project's MIT LICENSE remains a
//! separate release prerequisite. xtask itself is excluded by the scanner.

use std::process::Command;

/// spec: operations.md:OPS-021 partial
#[test]
fn distribution_notices_match_locked_dependencies_and_license_texts() {
    // Do not assume a target directory or a prebuilt xtask executable. Cargo
    // releases its build lock before executing tests. This subprocess cannot
    // access the network; both target registries must be fetched during setup.
    let output = Command::new(env!("CARGO"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            "run",
            "--offline",
            "--locked",
            "--quiet",
            "-p",
            "xtask",
            "--",
            "notices",
            "--check",
        ])
        .output()
        .unwrap_or_else(|error| panic!("runs offline notice check: {error}"));
    assert!(
        output.status.success(),
        "notice check failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
