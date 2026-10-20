//! Account file changes: lock, read again, temporary file, replace
//! (cli-v1 §1.6). Design: docs/architectures/admin-accounts.md.

use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs::{File, OpenOptions};
use std::io::{self, ErrorKind, Write as _};
use std::path::{Path, PathBuf};

use vnc_relay_app::accounts::{
    ACCOUNTS_FILE_KEY, AccountsDiagnostic, AccountsError, AccountsWarning, AdminFailure,
    LoadedAccounts,
};
use vnc_relay_app::ports::AccountStore;
use vnc_relay_domain::account::Account;

use super::HashFloor;

/// The account file of the `admin` commands. Hashes below the approved
/// minimum are accepted, so `admin passwd` can replace them.
#[derive(Debug, Clone)]
pub struct AccountsFile {
    path: PathBuf,
}

impl AccountsFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// `<account file name><suffix>` in the same directory.
    fn sibling(&self, suffix: &str) -> Result<PathBuf, AdminFailure> {
        let Some(name) = self.path.file_name() else {
            return Err(write_failed(format!(
                "계정 파일 경로에 파일 이름이 없다: {}",
                self.path.display()
            )));
        };
        let mut name = OsString::from(name);
        name.push(suffix);
        Ok(self.path.with_file_name(name))
    }

    /// Steps 2 to 6 of cli-v1 §1.6, inside the lock.
    fn replace(
        &self,
        change: &mut dyn FnMut(Vec<Account>) -> Result<Vec<Account>, AdminFailure>,
        warn: &mut dyn FnMut(AccountsWarning),
    ) -> Result<(), AdminFailure> {
        let current = super::read(&self.path, HashFloor::Accept)
            .map_err(AdminFailure::Accounts)?
            .map(|loaded| loaded.accounts)
            .unwrap_or_default();
        let mut next = change(current)?;
        next.sort_by(|a, b| a.name.cmp(&b.name));

        let temp = self.sibling(&format!(".{}.tmp", std::process::id()))?;
        // A file already at the temporary path is not ours; it stays.
        let file = create_owner_only(&temp).map_err(|e| temp_failed("만들", &temp, &e))?;
        if let Err(failure) = fill_and_check(file, &temp, &next) {
            // The file may be partly written; the target is untouched. A
            // temporary file that cannot be removed only takes disk space.
            let _ = std::fs::remove_file(&temp);
            return Err(failure);
        }
        if let Err(error) = std::fs::rename(&temp, &self.path) {
            let _ = std::fs::remove_file(&temp);
            return Err(write_failed(format!(
                "계정 파일을 교체할 수 없다({}): {}",
                reason(&error),
                self.path.display()
            )));
        }
        if let Err(error) = sync_dir(&self.path) {
            warn(AccountsWarning::CleanupFailed(format!(
                "계정 파일은 바꿨지만 디렉터리 내용을 디스크에 반영하지 못했다({}): {}",
                reason(&error),
                self.path.display()
            )));
        }
        Ok(())
    }
}

impl AccountStore for AccountsFile {
    fn read(&self) -> Result<Option<LoadedAccounts>, Vec<AccountsDiagnostic>> {
        super::read(&self.path, HashFloor::Accept)
    }

    fn update(
        &self,
        change: &mut dyn FnMut(Vec<Account>) -> Result<Vec<Account>, AdminFailure>,
        warn: &mut dyn FnMut(AccountsWarning),
    ) -> Result<(), AdminFailure> {
        let lock = self.sibling(".lock")?;
        if let Err(error) = OpenOptions::new().write(true).create_new(true).open(&lock) {
            return Err(if error.kind() == ErrorKind::AlreadyExists {
                AdminFailure::Accounts(vec![AccountsDiagnostic {
                    error: AccountsError::FileLocked,
                    key: ACCOUNTS_FILE_KEY.to_owned(),
                    message: format!(
                        "다른 `admin` 명령이 계정 파일을 고치는 중이다. 실행 중인 `admin` 명령이 없으면 잠금 파일 {}를 지운 뒤 다시 실행한다.",
                        lock.display()
                    ),
                }])
            } else {
                write_failed(format!(
                    "잠금 파일을 만들 수 없다({}): {}",
                    reason(&error),
                    lock.display()
                ))
            });
        }
        let result = self.replace(change, warn);
        if let Err(error) = std::fs::remove_file(&lock) {
            warn(AccountsWarning::CleanupFailed(format!(
                "잠금 파일을 지우지 못했다({}). 실행 중인 `admin` 명령이 없으면 {}를 지운다.",
                reason(&error),
                lock.display()
            )));
        }
        result
    }
}

/// Steps 3 and 4 after the owner-only file exists: written, flushed to disk,
/// read again, and checked.
fn fill_and_check(mut file: File, temp: &Path, accounts: &[Account]) -> Result<(), AdminFailure> {
    file.write_all(render(accounts).as_bytes())
        .map_err(|e| temp_failed("쓸", temp, &e))?;
    file.sync_all()
        .map_err(|e| temp_failed("디스크에 반영할", temp, &e))?;
    drop(file);
    let written = super::read(temp, HashFloor::Accept)
        .ok()
        .flatten()
        .map(|loaded| loaded.accounts);
    if written.as_deref() == Some(accounts) {
        Ok(())
    } else {
        Err(write_failed(format!(
            "다시 읽은 임시 파일이 쓴 내용과 다르다: {}",
            temp.display()
        )))
    }
}

fn temp_failed(step: &str, temp: &Path, error: &io::Error) -> AdminFailure {
    write_failed(format!(
        "임시 파일을 {step} 수 없다({}): {}",
        reason(error),
        temp.display()
    ))
}

/// The fixed write format (docs/architectures/admin-accounts.md §3). Names
/// and hash strings have no quote or backslash, so basic strings hold them
/// as they are.
fn render(accounts: &[Account]) -> String {
    let mut text = String::from("version = 1\n");
    for account in accounts {
        let _ = write!(
            text,
            "\n[[account]]\nname = \"{}\"\npassword_hash = \"{}\"\n",
            account.name,
            account.password_hash.as_str()
        );
    }
    text
}

#[cfg(unix)]
fn create_owner_only(path: &Path) -> io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

/// The new file takes the access control list of its directory (cli-v1 §1.6).
#[cfg(not(unix))]
fn create_owner_only(path: &Path) -> io::Result<File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

/// Step 6: the rename itself reaches the disk.
#[cfg(unix)]
fn sync_dir(path: &Path) -> io::Result<()> {
    let dir = match path.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    };
    File::open(dir)?.sync_all()
}

/// cli-v1 §1.6 step 6 is for Unix only.
#[cfg(not(unix))]
fn sync_dir(_path: &Path) -> io::Result<()> {
    Ok(())
}

fn reason(error: &io::Error) -> String {
    match error.kind() {
        ErrorKind::NotFound => "디렉터리가 없다".to_owned(),
        ErrorKind::PermissionDenied => "권한이 없다".to_owned(),
        ErrorKind::StorageFull => "디스크 공간이 없다".to_owned(),
        _ => error.to_string(),
    }
}

fn write_failed(message: String) -> AdminFailure {
    AdminFailure::Accounts(vec![AccountsDiagnostic {
        error: AccountsError::WriteFailed,
        key: ACCOUNTS_FILE_KEY.to_owned(),
        message: format!("{message}. 기존 계정 파일은 그대로다."),
    }])
}
