//! `service install|uninstall|run` on Windows 11 (cli-v1 §1.9, operations.md
//! §3.7). Design: docs/architectures/windows-service.md.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use vnc_relay_adapters::windows_service::{
    self, DispatchError, Installed, Operator, SERVICE_NAME, Scm, ScmError, UninstallError,
};

use crate::cli::{self, EXIT_RUNTIME, EXIT_USAGE, ServiceAction};
use crate::serve::{self, Host};

/// Runs one `service` command and returns its exit code.
pub fn run(action: ServiceAction) -> ExitCode {
    let result = match action {
        ServiceAction::Install { config, operators } => install(&config, &operators),
        ServiceAction::Uninstall => uninstall(),
        ServiceAction::Run { config } => return run_service(config),
    };
    match result {
        Ok(done) => {
            // The change is made; a closed stdout does not undo it.
            let _ = writeln!(io::stdout().lock(), "서비스 '{SERVICE_NAME}'를 {done}.");
            ExitCode::SUCCESS
        }
        Err(Failure::Reported(code)) => ExitCode::from(code),
        Err(Failure::Error {
            code,
            message,
            exit,
        }) => {
            cli::report("error", code, "", &message);
            ExitCode::from(exit)
        }
    }
}

/// Why a service command stopped.
#[derive(Debug, PartialEq, Eq)]
enum Failure {
    /// The file check already wrote its diagnostics.
    Reported(u8),
    /// One error line of cli-v1 §2.2 to write.
    Error {
        code: &'static str,
        message: String,
        exit: u8,
    },
}

/// `service install`, install steps 1 to 7 of cli-v1 §1.9.
fn install(config: &Path, operators: &[String]) -> Result<&'static str, Failure> {
    let scm = Scm::open_for_install().map_err(scm_failure)?;
    install_with(
        &scm,
        config,
        operators,
        std::env::current_exe,
        serve::check_service_files,
    )?;
    Ok("설치했다")
}

/// `service uninstall`, uninstall steps 1 to 4 of cli-v1 §1.9.
fn uninstall() -> Result<&'static str, Failure> {
    let scm = Scm::open().map_err(scm_failure)?;
    scm.uninstall().map_err(|error| match error {
        UninstallError::NotFound => error_line(
            "service_not_found",
            format!("서비스 '{SERVICE_NAME}'가 없다."),
        ),
        UninstallError::AccessDenied => access_denied(),
        UninstallError::Running => error_line(
            "service_running",
            format!("서비스 '{SERVICE_NAME}'가 중지됨이 아니다. 서비스를 중지한 뒤 다시 실행한다."),
        ),
        UninstallError::Failed(error) => scm_failure(error),
    })?;
    Ok("제거했다")
}

/// `service run`: the server under the SCM. From the moment the SCM calls the
/// service, the process writes nothing to stdout or stderr.
fn run_service(config: PathBuf) -> ExitCode {
    let result = windows_service::run(move |session| {
        cli::silence();
        serve::start(&config, Host::Service(session))
    });
    match result {
        Ok(code) => ExitCode::from(code),
        Err(DispatchError::NotService) => {
            cli::report(
                "error",
                "service_controller_unavailable",
                "",
                &format!(
                    "SCM이 시작한 프로세스가 아니다. 서비스는 `sc.exe start {SERVICE_NAME}`나 트레이 앱으로 시작한다."
                ),
            );
            ExitCode::from(EXIT_USAGE)
        }
        // Only a missing SCM connection proves a terminal. Any other failure
        // may be under the SCM, which gives no stdout or stderr (cli-v1 §1.9).
        Err(DispatchError::Failed(_)) => ExitCode::from(EXIT_RUNTIME),
    }
}

/// The SCM steps of `service install` after step 1. Tests use a fake to make
/// each step fail.
trait Installer {
    type Operator;
    type Created;
    fn service_exists(&self) -> Result<bool, ScmError>;
    fn lookup(&self, name: &str) -> Result<Option<Self::Operator>, ScmError>;
    fn create(&self, exe: &Path, config: &Path) -> Result<Self::Created, ScmError>;
    /// The delayed start of step 5 and the operator rights of step 6.
    fn configure(
        &self,
        created: &Self::Created,
        operators: &[Self::Operator],
    ) -> Result<(), ScmError>;
    fn delete(&self, created: Self::Created) -> Result<(), ScmError>;
}

impl Installer for Scm {
    type Operator = Operator;
    type Created = Installed;

    fn service_exists(&self) -> Result<bool, ScmError> {
        Scm::service_exists(self)
    }

    fn lookup(&self, name: &str) -> Result<Option<Operator>, ScmError> {
        windows_service::lookup_operator(name)
    }

    fn create(&self, exe: &Path, config: &Path) -> Result<Installed, ScmError> {
        Scm::create(self, exe, config)
    }

    fn configure(&self, created: &Installed, operators: &[Operator]) -> Result<(), ScmError> {
        created.set_delayed_start()?;
        created.grant(operators)
    }

    fn delete(&self, created: Installed) -> Result<(), ScmError> {
        created.delete()
    }
}

/// Install steps 2 to 6. A step that fails after the registration deletes it
/// (cli-v1 §1.9 "설치 순서").
fn install_with<I: Installer>(
    scm: &I,
    config: &Path,
    names: &[String],
    exe: impl FnOnce() -> io::Result<PathBuf>,
    check: impl FnOnce(&Path) -> Result<(), u8>,
) -> Result<(), Failure> {
    if scm.service_exists().map_err(scm_failure)? {
        return Err(exists());
    }
    let mut operators = Vec::with_capacity(names.len());
    for name in names {
        match scm.lookup(name).map_err(scm_failure)? {
            Some(operator) => operators.push(operator),
            None => {
                return Err(Failure::Error {
                    code: "service_operator_unknown",
                    message: format!(
                        "`--operator` 이름 `{name}`을(를) Windows 사용자나 그룹으로 찾지 못했다."
                    ),
                    exit: EXIT_USAGE,
                });
            }
        }
    }
    // The check reports a path that cannot become absolute as it is.
    let config = std::path::absolute(config).unwrap_or_else(|_| config.to_path_buf());
    check(&config).map_err(Failure::Reported)?;
    let exe = exe().map_err(|error| failed(os_code(&error)))?;
    let created = scm.create(&exe, &config).map_err(|error| {
        if error.is_service_exists() {
            exists()
        } else {
            scm_failure(error)
        }
    })?;
    if let Err(error) = scm.configure(&created, &operators) {
        let undo = match scm.delete(created) {
            Ok(()) => "만든 서비스 등록을 지웠다.".to_owned(),
            Err(undo) => format!(
                "만든 서비스 등록을 지우지 못했다(OS 오류 {}). 관리자 권한 터미널에서 `sc.exe delete {SERVICE_NAME}`로 지운다.",
                undo.0
            ),
        };
        return Err(error_line(
            "service_failed",
            format!("SCM 호출이 실패했다(OS 오류 {}). {undo}", error.0),
        ));
    }
    Ok(())
}

fn exists() -> Failure {
    error_line(
        "service_exists",
        format!(
            "서비스 '{SERVICE_NAME}'가 이미 있다. 다시 설치하려면 먼저 `vnc-relay service uninstall`을 실행한다."
        ),
    )
}

fn access_denied() -> Failure {
    error_line(
        "service_access_denied",
        "서비스를 다룰 권한이 없다. 관리자 권한 터미널에서 실행한다.".to_owned(),
    )
}

/// Access denied, or any other SCM failure with its OS error number.
fn scm_failure(error: ScmError) -> Failure {
    if error.is_access_denied() {
        access_denied()
    } else {
        failed(error.0)
    }
}

fn failed(code: i32) -> Failure {
    error_line(
        "service_failed",
        format!("SCM 호출이 실패했다(OS 오류 {code})."),
    )
}

fn error_line(code: &'static str, message: String) -> Failure {
    Failure::Error {
        code,
        message,
        exit: EXIT_RUNTIME,
    }
}

fn os_code(error: &io::Error) -> i32 {
    error.raw_os_error().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::cli::{EXIT_ACCOUNTS, EXIT_CONFIG};

    const ACCESS_DENIED: ScmError = ScmError(5);

    /// A fake SCM that records each call and fails the chosen steps.
    #[derive(Default)]
    struct FakeScm {
        exists: bool,
        unknown: &'static [&'static str],
        create_fails: Option<ScmError>,
        configure_fails: Option<ScmError>,
        delete_fails: Option<ScmError>,
        calls: RefCell<Vec<String>>,
    }

    impl FakeScm {
        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }
    }

    impl Installer for FakeScm {
        type Operator = String;
        type Created = ();

        fn service_exists(&self) -> Result<bool, ScmError> {
            self.calls.borrow_mut().push("exists".to_owned());
            Ok(self.exists)
        }

        fn lookup(&self, name: &str) -> Result<Option<String>, ScmError> {
            self.calls.borrow_mut().push(format!("lookup {name}"));
            Ok((!self.unknown.contains(&name)).then(|| name.to_owned()))
        }

        fn create(&self, exe: &Path, config: &Path) -> Result<(), ScmError> {
            self.calls
                .borrow_mut()
                .push(format!("create {} {}", exe.display(), config.display()));
            self.create_fails.map_or(Ok(()), Err)
        }

        fn configure(&self, _created: &(), operators: &[String]) -> Result<(), ScmError> {
            self.calls
                .borrow_mut()
                .push(format!("configure {}", operators.join(",")));
            self.configure_fails.map_or(Ok(()), Err)
        }

        fn delete(&self, _created: ()) -> Result<(), ScmError> {
            self.calls.borrow_mut().push("delete".to_owned());
            self.delete_fails.map_or(Ok(()), Err)
        }
    }

    fn install(scm: &FakeScm, names: &[&str], check: Result<(), u8>) -> Result<(), Failure> {
        install_at(scm, Path::new(r"C:\relay\relay.toml"), names, check)
    }

    fn install_at(
        scm: &FakeScm,
        config: &Path,
        names: &[&str],
        check: Result<(), u8>,
    ) -> Result<(), Failure> {
        let names: Vec<String> = names.iter().map(|name| (*name).to_owned()).collect();
        install_with(
            scm,
            config,
            &names,
            || Ok(PathBuf::from(r"C:\relay\vnc-relay.exe")),
            |config| {
                scm.calls
                    .borrow_mut()
                    .push(format!("check {}", config.display()));
                check
            },
        )
    }

    fn code_of(result: &Result<(), Failure>) -> Option<(&'static str, u8)> {
        match result {
            Err(Failure::Error { code, exit, .. }) => Some((code, *exit)),
            _ => None,
        }
    }

    /// spec: operations.md:OPS-074 partial
    #[test]
    fn install_runs_the_steps_in_order_and_grants_every_operator() {
        let scm = FakeScm::default();
        assert_eq!(install(&scm, &[r"PC01\alice", r"PC01\ops"], Ok(())), Ok(()));
        assert_eq!(
            scm.calls(),
            [
                "exists",
                r"lookup PC01\alice",
                r"lookup PC01\ops",
                r"check C:\relay\relay.toml",
                r"create C:\relay\vnc-relay.exe C:\relay\relay.toml",
                r"configure PC01\alice,PC01\ops",
            ]
        );

        // A relative `--config` is checked and registered as an absolute path.
        let scm = FakeScm::default();
        assert_eq!(
            install_at(&scm, Path::new("relay.toml"), &[], Ok(())),
            Ok(())
        );
        let absolute = std::env::current_dir()
            .unwrap_or_else(|error| panic!("{error}"))
            .join("relay.toml");
        assert_eq!(
            scm.calls()[1..],
            [
                format!("check {}", absolute.display()),
                format!(r"create C:\relay\vnc-relay.exe {}", absolute.display()),
                "configure ".to_owned(),
            ]
        );
    }

    /// spec: operations.md:OPS-075 partial
    #[test]
    fn install_refuses_before_registering_anything() {
        let scm = FakeScm {
            exists: true,
            ..FakeScm::default()
        };
        let result = install(&scm, &[], Ok(()));
        assert_eq!(code_of(&result), Some(("service_exists", 1)));
        assert_eq!(scm.calls(), ["exists"]);

        let scm = FakeScm {
            unknown: &["nobody"],
            ..FakeScm::default()
        };
        let result = install(&scm, &["alice", "nobody"], Ok(()));
        assert_eq!(code_of(&result), Some(("service_operator_unknown", 2)));
        let Err(Failure::Error { message, .. }) = &result else {
            panic!("{result:?}");
        };
        assert!(message.contains("`nobody`"), "{message}");
        assert_eq!(scm.calls(), ["exists", "lookup alice", "lookup nobody"]);

        for exit in [EXIT_CONFIG, EXIT_ACCOUNTS] {
            let scm = FakeScm::default();
            assert_eq!(install(&scm, &[], Err(exit)), Err(Failure::Reported(exit)));
            assert_eq!(scm.calls(), ["exists", r"check C:\relay\relay.toml"]);
        }

        let scm = FakeScm {
            create_fails: Some(ScmError(1073)),
            ..FakeScm::default()
        };
        assert_eq!(
            code_of(&install(&scm, &[], Ok(()))),
            Some(("service_exists", 1))
        );
        let scm = FakeScm {
            create_fails: Some(ACCESS_DENIED),
            ..FakeScm::default()
        };
        assert_eq!(
            code_of(&install(&scm, &[], Ok(()))),
            Some(("service_access_denied", 1))
        );
    }

    /// spec: operations.md:OPS-075 partial
    #[test]
    fn install_deletes_its_registration_when_a_later_step_fails() {
        let scm = FakeScm {
            configure_fails: Some(ScmError(1332)),
            ..FakeScm::default()
        };
        let result = install(&scm, &["alice"], Ok(()));
        assert_eq!(code_of(&result), Some(("service_failed", 1)));
        let Err(Failure::Error { message, .. }) = &result else {
            panic!("{result:?}");
        };
        assert!(message.contains("OS 오류 1332"), "{message}");
        assert!(message.contains("지웠다"), "{message}");
        assert_eq!(scm.calls().last().map(String::as_str), Some("delete"));

        let scm = FakeScm {
            configure_fails: Some(ScmError(1332)),
            delete_fails: Some(ScmError(1072)),
            ..FakeScm::default()
        };
        let result = install(&scm, &[], Ok(()));
        let Err(Failure::Error { code, message, .. }) = &result else {
            panic!("{result:?}");
        };
        assert_eq!(*code, "service_failed");
        assert!(message.contains("OS 오류 1072"), "{message}");
        assert!(message.contains("sc.exe delete vnc-relay"), "{message}");
    }

    /// The check of install step 4 with real files: the codes of `serve`.
    mod check {
        use std::path::PathBuf;

        use crate::cli::{EXIT_ACCOUNTS, EXIT_CONFIG};
        use crate::serve::check_service_files;

        const ACCOUNTS: &str = "version = 1\n\n[[account]]\nname = \"alice\"\npassword_hash = \"$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\"\n";

        fn config(log: &str) -> String {
            format!(
                "[admin]\naccounts_file = \"admin-accounts.toml\"\n\n[management]\nlisten = \"127.0.0.1:15900\"\n\n[status]\nlisten = \"127.0.0.1:15902\"\n\n[log]\n{log}\n\n[relay.tcp_viewer]\nenabled = true\nlisten = \"127.0.0.1:15901\"\nallow = [\"127.0.0.1\"]\n"
            )
        }

        /// A new directory with the config and, when given, the account file.
        fn setup(name: &str, config: &str, accounts: Option<&str>) -> (PathBuf, PathBuf) {
            let dir = std::env::temp_dir()
                .join(format!("vnc-relay-service-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap_or_else(|error| panic!("{error}"));
            std::fs::write(dir.join("relay.toml"), config)
                .unwrap_or_else(|error| panic!("{error}"));
            if let Some(accounts) = accounts {
                std::fs::write(dir.join("admin-accounts.toml"), accounts)
                    .unwrap_or_else(|error| panic!("{error}"));
            }
            (dir.clone(), dir.join("relay.toml"))
        }

        /// spec: operations.md:OPS-075 partial
        #[test]
        fn uses_the_start_checks_and_needs_a_log_file() {
            let cases = [
                (
                    "valid",
                    config("output = \"relay.log\""),
                    Some(ACCOUNTS),
                    Ok(()),
                ),
                (
                    "stdout",
                    config("output = \"stdout\""),
                    Some(ACCOUNTS),
                    Err(EXIT_CONFIG),
                ),
                (
                    "default-stdout",
                    config(""),
                    Some(ACCOUNTS),
                    Err(EXIT_CONFIG),
                ),
                (
                    "bad-key",
                    config("output = \"relay.log\"\nbogus = 1"),
                    Some(ACCOUNTS),
                    Err(EXIT_CONFIG),
                ),
                (
                    "no-accounts",
                    config("output = \"relay.log\""),
                    None,
                    Err(EXIT_ACCOUNTS),
                ),
            ];
            for (name, text, accounts, expected) in cases {
                let (dir, path) = setup(name, &text, accounts);
                assert_eq!(check_service_files(&path), expected, "{name}");
                // The check opens no log file.
                assert!(!dir.join("relay.log").exists(), "{name}");
                let _ = std::fs::remove_dir_all(&dir);
            }
        }
    }
}
