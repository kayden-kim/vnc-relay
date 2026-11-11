//! Development commands for this repository.
//!
//! - `cargo xtask scenarios [--require-all]`: scenario coverage report
//!   (docs/conventions/testing.md section 7).
//! - `cargo xtask layers`: layer dependency check (docs/architectures/code-map.md).
//! - `cargo xtask tokens [--check]`: token CSS from DESIGN.md, and the sign-in page's copies of app components and styles
//!   (docs/conventions/frontend.md section 2).
//! - `cargo xtask dashboard-serve [--port N]`: local preview server for `web/dashboard/`
//!   (docs/architectures/xtask.md).
//! - `cargo xtask notices [--check]`: distribution dependency notices
//!   (docs/how-to/check-dependency-licenses.md).
//! - `cargo xtask package`: release bundle for the host OS (docs/how-to/release.md).
//! - `cargo xtask help-site`: operator help site build and checks
//!   (docs/how-to/publish-help.md).
//! - `cargo xtask help-serve [--port N]`: local preview server for the built help site
//!   (docs/architectures/xtask.md).

mod cargo;
mod dashboard_serve;
mod help_serve;
mod help_site;
mod layers;
mod notices;
mod package;
mod scenarios;
mod static_server;
mod tokens;

use std::fmt;
use std::io::Write;
use std::process::ExitCode;

const USAGE: &str = "usage: cargo xtask scenarios [--require-all] | cargo xtask layers | cargo xtask tokens [--check] | cargo xtask dashboard-serve [--port N] | cargo xtask notices [--check] | cargo xtask package | cargo xtask help-site | cargo xtask help-serve [--port N]";

/// Exit code for a usage error, same as the product CLI (cli-v1 §2.1).
const EXIT_USAGE: u8 = 2;

/// Port of the local dashboard preview when `--port` is not given.
const DEFAULT_DASHBOARD_PORT: u16 = 8080;
/// Port of the local help site preview when `--port` is not given.
const DEFAULT_HELP_PORT: u16 = 8090;

fn usage() -> ExitCode {
    let _ = writeln!(std::io::stderr(), "{USAGE}");
    ExitCode::from(EXIT_USAGE)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["scenarios"] => scenarios::run(false),
        ["scenarios", "--require-all"] => scenarios::run(true),
        ["layers"] => layers::run(),
        ["tokens"] => tokens::run(false),
        ["tokens", "--check"] => tokens::run(true),
        ["notices"] => notices::run(false),
        ["notices", "--check"] => notices::run(true),
        ["package"] => package::run(),
        ["dashboard-serve"] => dashboard_serve::run(DEFAULT_DASHBOARD_PORT),
        ["dashboard-serve", "--port", port] => match port.parse::<u16>() {
            Ok(port) => dashboard_serve::run(port),
            Err(_) => return usage(),
        },
        ["help-site"] => help_site::run(),
        ["help-serve"] => help_serve::run(DEFAULT_HELP_PORT),
        ["help-serve", "--port", port] => match port.parse::<u16>() {
            Ok(port) => help_serve::run(port),
            Err(_) => return usage(),
        },
        _ => return usage(),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "xtask: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Failure that stops a command before it can give a verdict.
#[derive(Debug)]
pub struct Error(String);

impl Error {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self(format!("I/O error: {error}"))
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self(format!("JSON error: {error}"))
    }
}
