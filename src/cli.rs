//! Command line parsing, diagnostic lines, and exit codes (cli-v1 §1.1, §1.5,
//! §2.1). Design: docs/architectures/serve-startup.md.

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::PathBuf;

use clap::error::{ContextKind, ContextValue, ErrorKind};
use clap::{Arg, ArgAction};

/// Exit codes of cli-v1 §2.1.
pub const EXIT_RUNTIME: u8 = 1;
pub const EXIT_USAGE: u8 = 2;
pub const EXIT_CONFIG: u8 = 3;
pub const EXIT_LISTENER: u8 = 4;
pub const EXIT_ACCOUNTS: u8 = 5;

pub const HELP: &str = "\
vnc-relay: UltraVNC Repeater 호환 TCP·WS·WSS 중계 서버

사용법:
  vnc-relay serve --config <path>   서버를 포그라운드로 실행한다.
  vnc-relay --help, -h              이 사용법을 쓴다.
  vnc-relay --version, -V           버전을 쓴다.

종료 코드: 0 성공, 1 실행 중 오류, 2 사용법 오류, 3 설정 오류, 4 리스너 준비 실패, 5 계정 파일 오류
";

const USAGE: &str = "\
사용법: vnc-relay serve --config <path>
       vnc-relay --help
       vnc-relay --version
";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    Version,
    Serve { config: PathBuf },
}

/// A usage error. `message` is the Korean sentence of the `usage_error` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageError {
    pub message: String,
}

/// Parses the whole argument list, program name first.
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, UsageError> {
    let matches = command()
        .try_get_matches_from(args)
        .map_err(|error| usage(&clap_message(&error)))?;
    let help = matches.get_flag("help");
    let version = matches.get_flag("version");
    match (help, version, matches.subcommand()) {
        (true, false, None) => Ok(Command::Help),
        (false, true, None) => Ok(Command::Version),
        (false, false, Some(("serve", serve))) => serve
            .get_one::<PathBuf>("config")
            .map(|config| Command::Serve {
                config: config.clone(),
            })
            .ok_or_else(|| usage("`--config <path>`가 필요하다.")),
        (false, false, _) => Err(usage("하위 명령이 필요하다.")),
        _ => Err(usage(
            "`--help`와 `--version`은 다른 인수와 함께 쓸 수 없다.",
        )),
    }
}

/// Writes `error[usage_error]: <message>` and the usage text to stderr.
pub fn report_usage(error: &UsageError) {
    report("error", "usage_error", "", &error.message);
    // A failed write to stderr leaves nothing else to report; the exit code still signals the error.
    let _ = io::stderr().write_all(USAGE.as_bytes());
}

/// Writes one diagnostic line of cli-v1 §1.5 to stderr. An empty `key`
/// leaves out ` key=`.
pub fn report(level: &str, code: &str, key: &str, message: &str) {
    let mut stderr = io::stderr().lock();
    // A failed write to stderr leaves nothing else to report; the exit code still signals the error.
    let _ = write_diagnostic(&mut stderr, level, code, key, message);
}

fn write_diagnostic(
    out: &mut impl Write,
    level: &str,
    code: &str,
    key: &str,
    message: &str,
) -> io::Result<()> {
    if key.is_empty() {
        writeln!(out, "{level}[{code}]: {message}")
    } else {
        writeln!(out, "{level}[{code}] key={key}: {message}")
    }
}

fn usage(message: &str) -> UsageError {
    UsageError {
        message: message.to_owned(),
    }
}

/// clap's own help, version, and messages are off: output follows cli-v1
/// §1.5 in Korean.
fn command() -> clap::Command {
    clap::Command::new("vnc-relay")
        .disable_help_flag(true)
        .disable_version_flag(true)
        .disable_help_subcommand(true)
        .arg(
            Arg::new("help")
                .short('h')
                .long("help")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("version")
                .short('V')
                .long("version")
                .action(ArgAction::SetTrue),
        )
        .subcommand(
            clap::Command::new("serve").disable_help_flag(true).arg(
                Arg::new("config")
                    .long("config")
                    .value_name("path")
                    .value_parser(clap::value_parser!(PathBuf))
                    .action(ArgAction::Set)
                    .required(true),
            ),
        )
}

fn clap_message(error: &clap::Error) -> String {
    let context = |kind| match error.get(kind) {
        Some(ContextValue::String(text)) => Some(text.clone()),
        Some(ContextValue::Strings(texts)) => Some(texts.join(", ")),
        _ => None,
    };
    let arg = context(ContextKind::InvalidArg).unwrap_or_default();
    match error.kind() {
        ErrorKind::InvalidSubcommand => match context(ContextKind::InvalidSubcommand) {
            Some(name) => format!("알 수 없는 하위 명령 `{name}`이다."),
            None => "알 수 없는 하위 명령이다.".to_owned(),
        },
        ErrorKind::UnknownArgument => format!("알 수 없는 인수 `{arg}`다."),
        ErrorKind::MissingRequiredArgument => format!("`{arg}`가 필요하다."),
        ErrorKind::ArgumentConflict if context(ContextKind::PriorArg) == Some(arg.clone()) => {
            format!("`{arg}`는 한 번만 쓸 수 있다.")
        }
        ErrorKind::ArgumentConflict => format!("`{arg}`는 다른 인수와 함께 쓸 수 없다."),
        ErrorKind::InvalidValue | ErrorKind::NoEquals => format!("`{arg}`에 값이 필요하다."),
        ErrorKind::InvalidUtf8 => "인수가 UTF-8이 아니다.".to_owned(),
        _ => "명령행 인수가 올바르지 않다.".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(args: &[&str]) -> Result<Command, String> {
        parse(
            std::iter::once("vnc-relay")
                .chain(args.iter().copied())
                .map(OsString::from),
        )
        .map_err(|error| error.message)
    }

    fn serve(config: &str) -> Result<Command, String> {
        Ok(Command::Serve {
            config: PathBuf::from(config),
        })
    }

    #[test]
    fn accepts_both_config_spellings() {
        assert_eq!(run(&["serve", "--config", "a.toml"]), serve("a.toml"));
        assert_eq!(run(&["serve", "--config=a.toml"]), serve("a.toml"));
    }

    #[test]
    fn help_and_version_stand_alone() {
        assert_eq!(run(&["--help"]), Ok(Command::Help));
        assert_eq!(run(&["-h"]), Ok(Command::Help));
        assert_eq!(run(&["--version"]), Ok(Command::Version));
        assert_eq!(run(&["-V"]), Ok(Command::Version));
        for args in [
            &["--help", "--version"][..],
            &["--help", "serve", "--config", "a.toml"],
            &["--version", "serve", "--config", "a.toml"],
        ] {
            assert!(run(args).is_err(), "{args:?}");
        }
        assert_eq!(
            run(&["serve", "--config", "a.toml", "--help"]),
            Err("알 수 없는 인수 `--help`다.".to_owned())
        );
    }

    #[test]
    fn rejects_missing_repeated_or_empty_config() {
        assert_eq!(
            run(&["serve"]),
            Err("`--config <path>`가 필요하다.".to_owned())
        );
        assert_eq!(
            run(&["serve", "--config", "a.toml", "--config", "b.toml"]),
            Err("`--config <path>`는 한 번만 쓸 수 있다.".to_owned())
        );
        assert!(run(&["serve", "--config"]).is_err());
        assert!(run(&["serve", "--config="]).is_err());
    }

    #[test]
    fn rejects_unknown_commands_and_arguments() {
        assert_eq!(run(&[]), Err("하위 명령이 필요하다.".to_owned()));
        // `admin` stays unknown until task 4.1.
        assert_eq!(
            run(&["admin", "list", "--config", "a.toml"]),
            Err("알 수 없는 하위 명령 `admin`이다.".to_owned())
        );
        assert_eq!(
            run(&["serve", "--config", "a.toml", "--verbose"]),
            Err("알 수 없는 인수 `--verbose`다.".to_owned())
        );
        assert_eq!(
            run(&["serve", "--config", "a.toml", "extra"]),
            Err("알 수 없는 인수 `extra`다.".to_owned())
        );
    }

    #[test]
    fn leaves_out_key_when_empty() {
        let mut out = Vec::new();
        write_diagnostic(&mut out, "error", "runtime_unavailable", "", "m")
            .unwrap_or_else(|e| panic!("{e}"));
        write_diagnostic(
            &mut out,
            "warning",
            "accounts_file_permissive",
            "admin.accounts_file",
            "m",
        )
        .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            String::from_utf8_lossy(&out),
            "error[runtime_unavailable]: m\nwarning[accounts_file_permissive] key=admin.accounts_file: m\n"
        );
    }
}
