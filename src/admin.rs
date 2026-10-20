//! `admin add|passwd|remove|list` (cli-v1 §1.5).
//! Design: docs/architectures/admin-accounts.md.

use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use vnc_relay_adapters::accounts_file::{AccountsFile, Argon2Hasher};
use vnc_relay_adapters::config_file;
use vnc_relay_adapters::terminal::Terminal;
use vnc_relay_app::accounts::{
    self, ACCOUNTS_FILE_KEY, AccountsWarning, AdminFailure, AdminService,
};
use vnc_relay_app::settings::AdminSettings;
use vnc_relay_domain::account::AccountName;

use crate::cli::{self, AdminAction, EXIT_ACCOUNTS, EXIT_CONFIG, EXIT_RUNTIME, EXIT_USAGE};

const RESTART_NOTICE: &str =
    "실행 중인 서버에는 반영되지 않는다. 서버를 정상 종료하고 다시 시작해야 한다.";

/// Runs one `admin` command and returns its exit code.
pub fn run(config: &Path, action: &AdminAction) -> ExitCode {
    match action {
        AdminAction::List => list(config),
        AdminAction::Add { account } => {
            change(config, account, "을(를) 추가했다.", |s, n, w| {
                s.add(n, w)
            })
        }
        AdminAction::Passwd { account } => change(
            config,
            account,
            "의 비밀번호를 바꿨다.",
            |s, n, w| s.passwd(n, w),
        ),
        AdminAction::Remove { account, yes } => {
            change(config, account, "을(를) 삭제했다.", |s, n, w| {
                s.remove(n, *yes, w)
            })
        }
    }
}

/// `add`, `passwd`, `remove`: name rule, config, Ctrl+C detector, then the
/// procedure. `done` ends the success sentence after the quoted name.
fn change(
    config: &Path,
    account: &str,
    done: &str,
    procedure: impl FnOnce(
        &AdminService<'_>,
        &AccountName,
        &mut dyn FnMut(AccountsWarning),
    ) -> Result<(), AdminFailure>,
) -> ExitCode {
    let name = match AccountName::parse(account) {
        Ok(name) => name,
        Err(error) => {
            cli::report("error", "account_name_invalid", "", &format!("{error}."));
            return ExitCode::from(EXIT_USAGE);
        }
    };
    let settings = match load_settings(config) {
        Ok(settings) => settings,
        Err(code) => return code,
    };
    // Before any input, so that Ctrl+C never ends the process with the
    // terminal in hidden-input mode.
    let terminal = match Terminal::open() {
        Ok(terminal) => terminal,
        Err(message) => return report_failure(&AdminFailure::RuntimeUnavailable(message)),
    };
    let store = AccountsFile::new(settings.accounts_file);
    let service = AdminService {
        store: &store,
        input: &terminal,
        hasher: &Argon2Hasher,
        params: settings.argon2.params(),
    };
    match procedure(&service, &name, &mut report_warning) {
        Ok(()) => {
            write_stdout(&format!("계정 '{name}'{done}\n"));
            notice(RESTART_NOTICE);
            ExitCode::SUCCESS
        }
        Err(failure) => report_failure(&failure),
    }
}

fn list(config: &Path) -> ExitCode {
    let settings = match load_settings(config) {
        Ok(settings) => settings,
        Err(code) => return code,
    };
    let store = AccountsFile::new(settings.accounts_file.clone());
    match accounts::list(&store, &mut report_warning) {
        Ok(Some(names)) => {
            let text: String = names.iter().map(|name| format!("{name}\n")).collect();
            write_stdout(&text);
            ExitCode::SUCCESS
        }
        Ok(None) => {
            notice(&format!(
                "계정 파일이 없어 계정이 없다: {}. `vnc-relay admin add`로 계정을 만든다.",
                settings.accounts_file.display()
            ));
            ExitCode::SUCCESS
        }
        Err(failure) => report_failure(&failure),
    }
}

/// The `[admin]` table, or the config exit code after writing every
/// diagnostic.
fn load_settings(config: &Path) -> Result<AdminSettings, ExitCode> {
    config_file::load_admin(config).map_err(|diagnostics| {
        for d in &diagnostics {
            cli::report("error", d.error.code(), &d.key, &d.message);
        }
        ExitCode::from(EXIT_CONFIG)
    })
}

fn report_failure(failure: &AdminFailure) -> ExitCode {
    failure.report(&mut |code, key, message| cli::report("error", code, key, message));
    ExitCode::from(match failure {
        AdminFailure::NotInteractive => EXIT_USAGE,
        AdminFailure::Accounts(_) => EXIT_ACCOUNTS,
        AdminFailure::AccountExists(_)
        | AdminFailure::AccountNotFound(_)
        | AdminFailure::PasswordInvalid(_)
        | AdminFailure::PasswordMismatch
        | AdminFailure::InputCanceled
        | AdminFailure::RuntimeUnavailable(_) => EXIT_RUNTIME,
    })
}

fn report_warning(warning: AccountsWarning) {
    cli::report(
        "warning",
        warning.code(),
        ACCOUNTS_FILE_KEY,
        &warning.message(),
    );
}

fn notice(message: &str) {
    // A failed write to stderr leaves nothing else to report.
    let _ = writeln!(io::stderr().lock(), "notice: {message}");
}

fn write_stdout(text: &str) {
    // The change is saved; a closed stdout does not undo it.
    let _ = io::stdout().lock().write_all(text.as_bytes());
}
