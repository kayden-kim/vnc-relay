//! Runs `cargo test --workspace` and records the outcome of each test.

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

use crate::Error;
use crate::cargo::{self, Workspace};
use crate::progress::StatusLine;

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

pub fn run(workspace: &Workspace, status: &StatusLine) -> Result<TestRun, Error> {
    let (executables, programs) = build(workspace)?;
    let total = list_counts(&programs);
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

    status.begin_tests(total);
    let mut parser = OutputParser::new(&executables);
    let mut reader = BufReader::new(reader);
    let mut buffer = Vec::new();
    loop {
        buffer.clear();
        if reader.read_until(b'\n', &mut buffer)? == 0 {
            break;
        }
        match parser.feed(String::from_utf8_lossy(&buffer).trim_end()) {
            Event::Target(name) => status.set_current(name),
            Event::Result(outcome) => status.finish_test(outcome == Outcome::Failed),
            Event::Other => {}
        }
        // The counts are updated first, so this line is followed by the new numbers.
        status.log(&buffer)?;
    }
    status.end_tests();
    let exit = child.wait()?;
    Ok(TestRun {
        outcomes: parser.outcomes,
        success: exit.success(),
    })
}

/// Builds every test target. Returns the executable file stems (`name-hash`)
/// mapped to the root source file of the target, and the executable paths.
fn build(workspace: &Workspace) -> Result<(HashMap<String, PathBuf>, Vec<PathBuf>), Error> {
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
    let mut programs = Vec::new();
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
        let Some(stem) = Path::new(executable).file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if executables
            .insert(stem.to_owned(), PathBuf::from(src_path))
            .is_none()
        {
            programs.push(PathBuf::from(executable));
        }
    }
    Ok((executables, programs))
}

/// Number of tests the executables list with `--list`, which runs no test.
/// `None` when a listing fails, so the progress shows the finished count only.
fn list_counts(programs: &[PathBuf]) -> Option<usize> {
    let mut total = 0;
    for program in programs {
        let output = Command::new(program)
            .arg("--list")
            .stderr(Stdio::null())
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        total += count_listed(&String::from_utf8_lossy(&output.stdout));
    }
    Some(total)
}

/// Counts the `name: test` lines of `--list` output. `name: benchmark` lines
/// and the summary line are not tests.
fn count_listed(output: &str) -> usize {
    output
        .lines()
        .filter(|line| line.ends_with(": test"))
        .count()
}

/// What one line of `cargo test` output means for the progress line.
#[derive(Debug, PartialEq, Eq)]
enum Event<'a> {
    /// A `Running` or `Doc-tests` line. The target as `cargo test` names it,
    /// or `None` for documentation tests, which are not counted.
    Target(Option<&'a str>),
    /// The result of a test in a known test target.
    Result(Outcome),
    Other,
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

    fn feed<'l>(&mut self, line: &'l str) -> Event<'l> {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("Running ") {
            // `Running unittests src/lib.rs (target/debug/deps/name-hash)`
            self.current = rest
                .rsplit_once('(')
                .and_then(|(_, executable)| executable.strip_suffix(')'))
                .and_then(|executable| file_stem(executable))
                .and_then(|stem| self.executables.get(stem));
            Event::Target(
                rest.rsplit_once(" (")
                    .map(|(target, _)| target.strip_prefix("unittests ").unwrap_or(target)),
            )
        } else if trimmed.starts_with("Doc-tests ") {
            self.current = None;
            Event::Target(None)
        } else if let (Some(src_path), Some((name, outcome))) = (
            self.current,
            line.strip_prefix("test ").and_then(parse_test_line),
        ) {
            self.outcomes
                .insert((src_path.clone(), name.to_owned()), outcome);
            Event::Result(outcome)
        } else {
            Event::Other
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

    #[test]
    fn feed_names_targets_and_reports_results_for_the_progress_line() {
        let executables = HashMap::from([
            ("a-1".to_owned(), PathBuf::from("/w/crates/a/src/lib.rs")),
            ("b-2".to_owned(), PathBuf::from("/w/tests/b.rs")),
        ]);
        let mut parser = OutputParser::new(&executables);
        let events: Vec<Event> = [
            "     Running unittests src/lib.rs (target/debug/deps/a-1)",
            "test x ... ok",
            "     Running tests\\b.rs (target\\debug\\deps\\b-2.exe)",
            "test y ... FAILED",
            "test z ... ignored",
            "test result: FAILED. 0 passed",
            "     Running tests/unknown.rs (target/debug/deps/unknown-9)",
            "test w ... ok",
            "   Doc-tests a",
            "test src/lib.rs - f (line 1) ... ok",
        ]
        .into_iter()
        .map(|line| parser.feed(line))
        .collect();
        assert_eq!(
            events,
            [
                Event::Target(Some("src/lib.rs")),
                Event::Result(Outcome::Passed),
                Event::Target(Some("tests\\b.rs")),
                Event::Result(Outcome::Failed),
                Event::Result(Outcome::Ignored),
                Event::Other,
                Event::Target(Some("tests/unknown.rs")),
                Event::Other,
                Event::Target(None),
                Event::Other,
            ]
        );
    }

    #[test]
    fn count_listed_counts_tests_but_not_benchmarks_or_the_summary() {
        let listing =
            "a::first: test\na::second: test\nb::speed: benchmark\n\n2 tests, 1 benchmark\n";
        assert_eq!(count_listed(listing), 2);
        assert_eq!(count_listed(""), 0);
    }
}
