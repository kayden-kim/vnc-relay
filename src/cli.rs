//! Command line parsing, diagnostic lines, and exit codes (cli-v1 §1.1, §1.5,
//! §1.9, §2.1). Design: docs/architectures/serve-startup.md,
//! docs/architectures/admin-accounts.md, docs/architectures/windows-service.md.

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use clap::error::{ContextKind, ContextValue, ErrorKind};
use clap::{Arg, ArgAction};

/// Exit codes of cli-v1 §2.1.
pub const EXIT_SUCCESS: u8 = 0;
pub const EXIT_RUNTIME: u8 = 1;
pub const EXIT_USAGE: u8 = 2;
pub const EXIT_CONFIG: u8 = 3;
pub const EXIT_LISTENER: u8 = 4;
pub const EXIT_ACCOUNTS: u8 = 5;

const HELP_COMMANDS: &str = "\
vnc-relay: UltraVNC Repeater 호환 TCP·WS·WSS 중계 서버

사용법:
  vnc-relay serve --config <path>                  VNC Relay를 포그라운드로 실행한다.
  vnc-relay admin add <account> --config <path>    계정을 추가한다.
  vnc-relay admin passwd <account> --config <path> 계정의 비밀번호를 바꾼다.
  vnc-relay admin remove <account> [--yes] --config <path>
                                                   계정을 삭제한다. --yes는 확인을 생략한다.
  vnc-relay admin list --config <path>             계정 이름을 나열한다.
";

/// The `service` lines exist only where the subcommand exists (cli-v1 §1.9).
#[cfg(windows)]
const HELP_SERVICE: &str = "\
  vnc-relay service install --config <path> [--operator <name>]...
                                                   Windows 서비스를 등록한다. 관리자 권한 터미널에서 실행한다.
  vnc-relay service uninstall                      중지된 Windows 서비스를 지운다. 관리자 권한 터미널에서 실행한다.
  vnc-relay service run --config <path>            SCM이 서비스를 시작할 때 쓰는 명령이다. 직접 실행하지 않는다.
";
#[cfg(not(windows))]
const HELP_SERVICE: &str = "";

const HELP_REST: &str = "\
  vnc-relay --help, -h                             이 사용법을 쓴다.
  vnc-relay --version, -V                          버전을 쓴다.

비밀번호는 터미널에서 숨김 입력으로만 받는다. admin 명령은 실행 중인 VNC Relay에 반영되지 않는다.

종료 코드: 0 성공, 1 실행 중 오류, 2 사용법 오류, 3 설정 오류, 4 리스너 준비 실패, 5 계정 파일 오류
";

const USAGE_COMMANDS: &str = "\
사용법: vnc-relay serve --config <path>
       vnc-relay admin add|passwd <account> --config <path>
       vnc-relay admin remove <account> [--yes] --config <path>
       vnc-relay admin list --config <path>
";

#[cfg(windows)]
const USAGE_SERVICE: &str = "\
       vnc-relay service install --config <path> [--operator <name>]...
       vnc-relay service uninstall
       vnc-relay service run --config <path>
";
#[cfg(not(windows))]
const USAGE_SERVICE: &str = "";

const USAGE_REST: &str = "\
       vnc-relay --help
       vnc-relay --version
";

/// The `--help` text.
pub fn help() -> String {
    [HELP_COMMANDS, HELP_SERVICE, HELP_REST].concat()
}

fn usage_text() -> String {
    [USAGE_COMMANDS, USAGE_SERVICE, USAGE_REST].concat()
}

/// Set by `service run` under the SCM, which gives the process no terminal:
/// from then on no diagnostic line is written (cli-v1 §1.9).
static SILENT: AtomicBool = AtomicBool::new(false);

/// Stops all diagnostic lines of this process.
#[cfg(windows)]
pub fn silence() {
    SILENT.store(true, Ordering::Relaxed);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    Version,
    Serve {
        config: PathBuf,
    },
    Admin {
        config: PathBuf,
        action: AdminAction,
    },
    #[cfg(windows)]
    Service(ServiceAction),
}

/// The `admin` subcommands. `account` is the argument as typed; the name
/// rule is checked by `admin::run` (`account_name_invalid`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdminAction {
    Add { account: String },
    Passwd { account: String },
    Remove { account: String, yes: bool },
    List,
}

/// The `service` subcommands (cli-v1 §1.9). `operators` are the names as
/// typed; `service install` looks them up.
#[cfg(windows)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceAction {
    Install {
        config: PathBuf,
        operators: Vec<String>,
    },
    Uninstall,
    Run {
        config: PathBuf,
    },
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
        (false, false, Some(("serve", serve))) => Ok(Command::Serve {
            config: config(serve)?,
        }),
        (false, false, Some(("admin", admin))) => parse_admin(admin),
        #[cfg(windows)]
        (false, false, Some(("service", service))) => parse_service(service),
        (false, false, _) => Err(usage("하위 명령이 필요하다.")),
        _ => Err(usage(
            "`--help`와 `--version`은 다른 인수와 함께 쓸 수 없다.",
        )),
    }
}

fn parse_admin(admin: &clap::ArgMatches) -> Result<Command, UsageError> {
    let account = |matches: &clap::ArgMatches| {
        matches
            .get_one::<String>("account")
            .cloned()
            .ok_or_else(|| usage("`<account>`가 필요하다."))
    };
    let (action, matches) = match admin.subcommand() {
        Some(("add", m)) => (
            AdminAction::Add {
                account: account(m)?,
            },
            m,
        ),
        Some(("passwd", m)) => (
            AdminAction::Passwd {
                account: account(m)?,
            },
            m,
        ),
        Some(("remove", m)) => (
            AdminAction::Remove {
                account: account(m)?,
                yes: m.get_flag("yes"),
            },
            m,
        ),
        Some(("list", m)) => (AdminAction::List, m),
        _ => {
            return Err(usage(
                "`admin` 다음에 `add`, `passwd`, `remove`, `list` 중 하나가 필요하다.",
            ));
        }
    };
    Ok(Command::Admin {
        config: config(matches)?,
        action,
    })
}

#[cfg(windows)]
fn parse_service(service: &clap::ArgMatches) -> Result<Command, UsageError> {
    let action = match service.subcommand() {
        Some(("install", m)) => ServiceAction::Install {
            config: config(m)?,
            operators: m
                .get_many::<String>("operator")
                .map(|names| names.cloned().collect())
                .unwrap_or_default(),
        },
        Some(("uninstall", _)) => ServiceAction::Uninstall,
        Some(("run", m)) => ServiceAction::Run { config: config(m)? },
        _ => {
            return Err(usage(
                "`service` 다음에 `install`, `uninstall`, `run` 중 하나가 필요하다.",
            ));
        }
    };
    Ok(Command::Service(action))
}

fn config(matches: &clap::ArgMatches) -> Result<PathBuf, UsageError> {
    matches
        .get_one::<PathBuf>("config")
        .cloned()
        .ok_or_else(|| usage("`--config <path>`가 필요하다."))
}

/// Writes `error[usage_error]: <message>` and the usage text to stderr.
pub fn report_usage(error: &UsageError) {
    report("error", "usage_error", "", &error.message);
    // A failed write to stderr leaves nothing else to report; the exit code still signals the error.
    let _ = io::stderr().write_all(usage_text().as_bytes());
}

/// Writes one diagnostic line of cli-v1 §1.5 to stderr. An empty `key`
/// leaves out ` key=`.
pub fn report(level: &str, code: &str, key: &str, message: &str) {
    if SILENT.load(Ordering::Relaxed) {
        return;
    }
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
    let command = clap::Command::new("vnc-relay")
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
        .subcommand(leaf("serve"))
        .subcommand(
            clap::Command::new("admin")
                .disable_help_flag(true)
                .disable_help_subcommand(true)
                .subcommand(leaf("add").arg(account_arg()))
                .subcommand(leaf("passwd").arg(account_arg()))
                .subcommand(
                    leaf("remove")
                        .arg(account_arg())
                        .arg(Arg::new("yes").long("yes").action(ArgAction::SetTrue)),
                )
                .subcommand(leaf("list")),
        );
    // Ubuntu has no `service`, so the name is an unknown subcommand there.
    #[cfg(windows)]
    let command = command.subcommand(
        clap::Command::new("service")
            .disable_help_flag(true)
            .disable_help_subcommand(true)
            .subcommand(
                leaf("install").arg(
                    Arg::new("operator")
                        .long("operator")
                        .value_name("name")
                        .value_parser(clap::builder::NonEmptyStringValueParser::new())
                        .action(ArgAction::Append),
                ),
            )
            .subcommand(clap::Command::new("uninstall").disable_help_flag(true))
            .subcommand(leaf("run")),
    );
    command
}

/// A subcommand that takes the required `--config <path>`.
fn leaf(name: &'static str) -> clap::Command {
    clap::Command::new(name).disable_help_flag(true).arg(
        Arg::new("config")
            .long("config")
            .value_name("path")
            .value_parser(clap::value_parser!(PathBuf))
            .action(ArgAction::Set)
            .required(true),
    )
}

fn account_arg() -> Arg {
    Arg::new("account")
        .value_name("account")
        .action(ArgAction::Set)
        .required(true)
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
        assert_eq!(
            run(&["start", "--config", "a.toml"]),
            Err("알 수 없는 하위 명령 `start`이다.".to_owned())
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

    fn admin(config: &str, action: AdminAction) -> Result<Command, String> {
        Ok(Command::Admin {
            config: PathBuf::from(config),
            action,
        })
    }

    /// spec: operations.md:OPS-030 partial
    #[test]
    fn parses_admin_subcommands_with_their_own_options() {
        let account = "alice".to_owned();
        assert_eq!(
            run(&["admin", "add", "alice", "--config", "a.toml"]),
            admin(
                "a.toml",
                AdminAction::Add {
                    account: account.clone()
                }
            )
        );
        assert_eq!(
            run(&["admin", "passwd", "--config=a.toml", "alice"]),
            admin(
                "a.toml",
                AdminAction::Passwd {
                    account: account.clone()
                }
            )
        );
        assert_eq!(
            run(&["admin", "remove", "alice", "--config", "a.toml"]),
            admin(
                "a.toml",
                AdminAction::Remove {
                    account: account.clone(),
                    yes: false
                }
            )
        );
        assert_eq!(
            run(&["admin", "remove", "--yes", "alice", "--config", "a.toml"]),
            admin(
                "a.toml",
                AdminAction::Remove {
                    account: account.clone(),
                    yes: true
                }
            )
        );
        assert_eq!(
            run(&["admin", "list", "--config", "a.toml"]),
            admin("a.toml", AdminAction::List)
        );
        // The name rule is not a usage error; `admin::run` checks it.
        assert_eq!(
            run(&["admin", "add", "Alice!", "--config", "a.toml"]),
            admin(
                "a.toml",
                AdminAction::Add {
                    account: "Alice!".to_owned()
                }
            )
        );

        let errors = [
            (
                &["admin"][..],
                "`admin` 다음에 `add`, `passwd`, `remove`, `list` 중 하나가 필요하다.",
            ),
            (
                &["admin", "--config", "a.toml"],
                "알 수 없는 인수 `--config`다.",
            ),
            (
                &["admin", "rename", "a", "--config", "a.toml"],
                "알 수 없는 하위 명령 `rename`이다.",
            ),
            (
                &["admin", "add", "--config", "a.toml"],
                "`<account>`가 필요하다.",
            ),
            (&["admin", "add", "alice"], "`--config <path>`가 필요하다."),
            (&["admin", "list"], "`--config <path>`가 필요하다."),
            (
                &["admin", "add", "alice", "bob", "--config", "a.toml"],
                "알 수 없는 인수 `bob`다.",
            ),
            (
                &["admin", "add", "alice", "--yes", "--config", "a.toml"],
                "알 수 없는 인수 `--yes`다.",
            ),
            (
                &["admin", "list", "alice", "--config", "a.toml"],
                "알 수 없는 인수 `alice`다.",
            ),
            (
                &[
                    "admin", "remove", "alice", "--yes", "--yes", "--config", "a.toml",
                ],
                "`--yes`는 한 번만 쓸 수 있다.",
            ),
            (
                &["admin", "list", "--config", "a.toml", "--config", "b.toml"],
                "`--config <path>`는 한 번만 쓸 수 있다.",
            ),
            (
                &["admin", "list", "--config", "a.toml", "--help"],
                "알 수 없는 인수 `--help`다.",
            ),
        ];
        for (args, message) in errors {
            assert_eq!(run(args), Err(message.to_owned()), "{args:?}");
        }
    }

    /// spec: operations.md:OPS-081 partial
    #[cfg(windows)]
    #[test]
    fn parses_service_subcommands_on_windows() {
        let install = |operators: &[&str]| {
            Ok(Command::Service(ServiceAction::Install {
                config: PathBuf::from("a.toml"),
                operators: operators.iter().map(|name| (*name).to_owned()).collect(),
            }))
        };
        assert_eq!(
            run(&["service", "install", "--config", "a.toml"]),
            install(&[])
        );
        assert_eq!(
            run(&[
                "service",
                "install",
                "--operator",
                r"PC01\alice",
                "--config=a.toml",
                r"--operator=PC01\relay-operators",
            ]),
            install(&[r"PC01\alice", r"PC01\relay-operators"])
        );
        assert_eq!(
            run(&["service", "uninstall"]),
            Ok(Command::Service(ServiceAction::Uninstall))
        );
        assert_eq!(
            run(&["service", "run", "--config", "a.toml"]),
            Ok(Command::Service(ServiceAction::Run {
                config: PathBuf::from("a.toml")
            }))
        );

        let errors = [
            (
                &["service"][..],
                "`service` 다음에 `install`, `uninstall`, `run` 중 하나가 필요하다.",
            ),
            (&["service", "install"], "`--config <path>`가 필요하다."),
            (&["service", "run"], "`--config <path>`가 필요하다."),
            (
                &["service", "uninstall", "--config", "a.toml"],
                "알 수 없는 인수 `--config`다.",
            ),
            (
                &["service", "run", "--config", "a.toml", "--operator", "a"],
                "알 수 없는 인수 `--operator`다.",
            ),
            (
                &["service", "install", "--config", "a.toml", "--operator"],
                "`--operator <name>`에 값이 필요하다.",
            ),
            (
                &["service", "install", "--config", "a.toml", "--operator="],
                "`--operator <name>`에 값이 필요하다.",
            ),
            (&["service", "start"], "알 수 없는 하위 명령 `start`이다."),
        ];
        for (args, message) in errors {
            assert_eq!(run(args), Err(message.to_owned()), "{args:?}");
        }
        assert!(
            help().contains("vnc-relay service install --config <path> [--operator <name>]...")
        );
    }

    /// spec: operations.md:OPS-081 partial
    #[cfg(not(windows))]
    #[test]
    fn has_no_service_subcommand_outside_windows() {
        assert_eq!(
            run(&["service", "install", "--config", "a.toml"]),
            Err("알 수 없는 하위 명령 `service`이다.".to_owned())
        );
        assert!(!help().contains("service"));
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
