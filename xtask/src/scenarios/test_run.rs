//! Runs `cargo test --workspace` and records the outcome of each test.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde_json::Value;

use crate::Error;
use crate::cargo::{self, Workspace};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Passed,
    Failed,
    Ignored,
}

/// A test is identified by the root source file of its test target and its
/// path inside that target, for example `tests::pairs_in_order`.
pub type TestKey = (PathBuf, String);

pub struct TestRun {
    pub outcomes: HashMap<TestKey, Outcome>,
    /// Exit status of `cargo test`. False when any test failed, including
    /// tests without markers.
    pub success: bool,
}

pub const TEST_COMMAND: &str = "cargo test --workspace --no-fail-fast";

pub fn run(workspace: &Workspace) -> Result<TestRun, Error> {
    let executables = build(workspace)?;
    let (reader, writer) = std::io::pipe()?;
    let mut command = cargo::command();
    command
        .args([
            "test",
            "--workspace",
            "--no-fail-fast",
            "--",
            "--color",
            "never",
        ])
        .current_dir(&workspace.root)
        .env("CARGO_TERM_COLOR", "never")
        .stdout(writer.try_clone()?)
        .stderr(writer);
    let mut child = command.spawn()?;
    // The command keeps copies of the pipe writer; drop them so reading ends at EOF.
    drop(command);

    let mut parser = OutputParser::new(&executables);
    let mut reader = BufReader::new(reader);
    let mut stderr = std::io::stderr().lock();
    let mut buffer = Vec::new();
    loop {
        buffer.clear();
        if reader.read_until(b'\n', &mut buffer)? == 0 {
            break;
        }
        stderr.write_all(&buffer)?;
        parser.feed(String::from_utf8_lossy(&buffer).trim_end());
    }
    let status = child.wait()?;
    Ok(TestRun {
        outcomes: parser.outcomes,
        success: status.success(),
    })
}

/// Builds every test target and maps executable file stems (`name-hash`) to
/// the root source file of the target.
fn build(workspace: &Workspace) -> Result<HashMap<String, PathBuf>, Error> {
    let output = cargo::command()
        .args([
            "test",
            "--workspace",
            "--no-run",
            "--message-format=json-render-diagnostics",
        ])
        .current_dir(&workspace.root)
        .stderr(Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err(Error::new("building the tests failed"));
    }
    let mut executables = HashMap::new();
    for line in output.stdout.split(|&b| b == b'\n') {
        let Ok(message) = serde_json::from_slice::<Value>(line) else {
            continue;
        };
        if message["reason"] != "compiler-artifact" || message["profile"]["test"] != true {
            continue;
        }
        let (Some(executable), Some(src_path)) = (
            message["executable"].as_str(),
            message["target"]["src_path"].as_str(),
        ) else {
            continue;
        };
        if let Some(stem) = Path::new(executable).file_stem().and_then(|s| s.to_str()) {
            executables.insert(stem.to_owned(), PathBuf::from(src_path));
        }
    }
    Ok(executables)
}

struct OutputParser<'a> {
    executables: &'a HashMap<String, PathBuf>,
    current: Option<&'a PathBuf>,
    outcomes: HashMap<TestKey, Outcome>,
}

impl<'a> OutputParser<'a> {
    fn new(executables: &'a HashMap<String, PathBuf>) -> Self {
        Self {
            executables,
            current: None,
            outcomes: HashMap::new(),
        }
    }

    fn feed(&mut self, line: &str) {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("Running ") {
            // `Running unittests src/lib.rs (target/debug/deps/name-hash)`
            self.current = rest
                .rsplit_once('(')
                .and_then(|(_, executable)| executable.strip_suffix(')'))
                .and_then(|executable| file_stem(executable))
                .and_then(|stem| self.executables.get(stem));
        } else if trimmed.starts_with("Doc-tests ") {
            self.current = None;
        } else if let (Some(src_path), Some((name, outcome))) = (
            self.current,
            line.strip_prefix("test ").and_then(parse_test_line),
        ) {
            self.outcomes
                .insert((src_path.clone(), name.to_owned()), outcome);
        }
    }
}

/// File stem for both `/` and `\` separators, so Windows paths parse on any host.
fn file_stem(path: &str) -> Option<&str> {
    let name = path.rsplit(['/', '\\']).next()?;
    Some(name.strip_suffix(".exe").unwrap_or(name))
}

/// Parses `name ... ok` after the `test ` prefix.
fn parse_test_line(rest: &str) -> Option<(&str, Outcome)> {
    let (name, result) = rest.rsplit_once(" ... ")?;
    let name = name.strip_suffix(" - should panic").unwrap_or(name);
    let outcome = match result.trim() {
        "ok" => Outcome::Passed,
        "FAILED" => Outcome::Failed,
        other if other.starts_with("ignored") => Outcome::Ignored,
        _ => return None,
    };
    Some((name, outcome))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_outcomes_per_test_target() {
        let executables = HashMap::from([
            (
                "vnc_relay_domain-1a2b".to_owned(),
                PathBuf::from("/w/crates/domain/src/lib.rs"),
            ),
            ("relay-3c4d".to_owned(), PathBuf::from("/w/tests/relay.rs")),
        ]);
        let mut parser = OutputParser::new(&executables);
        for line in [
            "     Running unittests src/lib.rs (target/debug/deps/vnc_relay_domain-1a2b)",
            "test matching::tests::pairs ... ok",
            "test matching::tests::panics - should panic ... ok",
            "test result: ok. 2 passed; 0 failed",
            "     Running tests\\relay.rs (target\\debug\\deps\\relay-3c4d.exe)",
            "test pairs_in_order ... FAILED",
            "test needs_ultravnc ... ignored, needs UltraVNC",
            "   Doc-tests vnc_relay_domain",
            "test src/lib.rs - parse (line 3) ... ok",
        ] {
            parser.feed(line);
        }
        let key = |src: &str, name: &str| (PathBuf::from(src), name.to_owned());
        assert_eq!(
            parser.outcomes,
            HashMap::from([
                (
                    key("/w/crates/domain/src/lib.rs", "matching::tests::pairs"),
                    Outcome::Passed
                ),
                (
                    key("/w/crates/domain/src/lib.rs", "matching::tests::panics"),
                    Outcome::Passed
                ),
                (key("/w/tests/relay.rs", "pairs_in_order"), Outcome::Failed),
                (key("/w/tests/relay.rs", "needs_ultravnc"), Outcome::Ignored),
            ])
        );
    }
}
