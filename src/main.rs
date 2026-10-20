//! Entry point of the `vnc-relay` executable (cli-v1 §1.1).
//! The `admin` subcommands come with task 4.1.

mod cli;
mod serve;

use std::io::Write;
use std::process::ExitCode;

use cli::Command;

fn main() -> ExitCode {
    match cli::parse(std::env::args_os()) {
        Ok(Command::Help) => write_stdout(cli::HELP),
        Ok(Command::Version) => write_stdout(&format!("vnc-relay {}\n", env!("CARGO_PKG_VERSION"))),
        Ok(Command::Serve { config }) => serve::run(&config),
        Err(error) => {
            cli::report_usage(&error);
            ExitCode::from(cli::EXIT_USAGE)
        }
    }
}

fn write_stdout(text: &str) -> ExitCode {
    match std::io::stdout().write_all(text.as_bytes()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
