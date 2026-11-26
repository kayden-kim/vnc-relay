//! `cargo xtask gate`: the full completion check of
//! docs/how-to/set-up-and-verify.md section 3.4. It runs the six checks in
//! order and stops at the first failure. Progress lines go to a terminal only,
//! so redirected output and CI logs keep just the result line
//! (docs/architectures/xtask.md, section 4).

use std::io::Write;
use std::time::Instant;

use crate::cargo::{self, Workspace};
use crate::progress::{self, StatusLine};
use crate::{Error, layers, scenarios, tokens};

/// How a check runs.
enum Run {
    /// A `cargo` subcommand in a child process.
    Cargo(&'static [&'static str]),
    /// The `xtask` commands run in this process. A nested `cargo xtask` would
    /// rebuild the running executable, which Windows does not allow.
    Layers,
    Tokens,
    Scenarios,
}

struct Check {
    /// The command as a person types it.
    command: &'static str,
    run: Run,
}

/// The checks of section 3.4, in order.
const CHECKS: [Check; 6] = [
    Check {
        command: "cargo fmt --all --check",
        run: Run::Cargo(&["fmt", "--all", "--check"]),
    },
    Check {
        command: "cargo clippy --workspace --all-targets -- -D warnings",
        run: Run::Cargo(&[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ]),
    },
    Check {
        command: "cargo xtask layers",
        run: Run::Layers,
    },
    Check {
        command: "cargo xtask tokens --check",
        run: Run::Tokens,
    },
    Check {
        command: "cargo xtask scenarios",
        run: Run::Scenarios,
    },
    Check {
        command: "cargo deny check licenses",
        run: Run::Cargo(&["deny", "check", "licenses"]),
    },
];

impl Check {
    fn run(&self, workspace: &Workspace, label: &str) -> Result<bool, Error> {
        match &self.run {
            Run::Cargo(args) => Ok(cargo::command()
                .args(*args)
                .current_dir(&workspace.root)
                .status()?
                .success()),
            Run::Layers => layers::run(),
            Run::Tokens => tokens::run(true),
            Run::Scenarios => scenarios::run(false, &StatusLine::new(label)),
        }
    }
}

pub fn run() -> Result<bool, Error> {
    let workspace = cargo::metadata()?;
    let started = Instant::now();
    let count = CHECKS.len();
    for (index, check) in CHECKS.iter().enumerate() {
        let label = format!("[{}/{count}]", index + 1);
        progress_line(&format!("{label} {}", check.command));
        let check_started = Instant::now();
        let passed = match check.run(&workspace, &label) {
            Ok(passed) => passed,
            Err(error) => {
                let _ = writeln!(std::io::stderr(), "xtask: {error}");
                false
            }
        };
        let result = if passed { "통과" } else { "실패" };
        let elapsed = progress::format_elapsed(check_started.elapsed());
        progress_line(&format!("{label} {result} ({elapsed})"));
        if !passed {
            let _ = writeln!(
                std::io::stderr(),
                "gate: 실패 — {label} {} ({})",
                check.command,
                progress::format_elapsed(started.elapsed())
            );
            return Ok(false);
        }
    }
    writeln!(
        std::io::stdout(),
        "gate: 통과 ({count}검사, {})",
        progress::format_elapsed(started.elapsed())
    )?;
    Ok(true)
}

/// A line for the person watching the output: the check number and its result.
fn progress_line(text: &str) {
    let _ = writeln!(std::io::stderr(), "{text}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shown_command_is_the_command_that_runs() {
        for check in &CHECKS {
            if let Run::Cargo(args) = &check.run {
                assert_eq!(check.command, format!("cargo {}", args.join(" ")));
            }
        }
    }
}
