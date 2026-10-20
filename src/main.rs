//! Entry point of the `vnc-relay` executable.
//!
//! The `serve` and `admin` subcommands do not exist yet. Only `--version` from
//! cli-v1 §1.1 is handled.

use std::io::Write;
use std::process::ExitCode;

/// Exit code for a usage error (cli-v1 §2.1).
const EXIT_USAGE: u8 = 2;

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    match (args.next(), args.next()) {
        (Some(flag), None) if flag == "--version" || flag == "-V" => {
            let version = format!("vnc-relay {}", env!("CARGO_PKG_VERSION"));
            match writeln!(std::io::stdout(), "{version}") {
                Ok(()) => ExitCode::SUCCESS,
                Err(_) => ExitCode::FAILURE,
            }
        }
        _ => {
            // A failed write to stderr leaves nothing else to report; the exit code still signals the error.
            let _ = writeln!(
                std::io::stderr(),
                "error[usage_error]: 지원하는 명령이 아직 없다. `--version`만 쓸 수 있다."
            );
            ExitCode::from(EXIT_USAGE)
        }
    }
}
