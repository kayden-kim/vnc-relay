//! Administrator account file reader (cli-v1 §1.3, §1.4, §1.6), writer
//! (`store`), and password hasher (`hasher`).
//! Design: docs/architectures/serve-startup.md, docs/architectures/admin-accounts.md.

mod hasher;
mod store;

use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::path::Path;

use toml::{Table, Value};
use vnc_relay_app::accounts::{
    ACCOUNTS_FILE_KEY, AccountsDiagnostic, AccountsError, AccountsWarning, LoadedAccounts,
};
use vnc_relay_domain::account::{
    Account, AccountName, Argon2Params, Argon2ParamsError, PasswordHash,
};

use crate::toml_text::error_position;
pub use hasher::Argon2Hasher;
pub use store::AccountsFile;

/// What to do with a hash whose parameters are below the approved minimum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashFloor {
    /// `serve`: `accounts_hash_below_minimum`.
    Reject,
    /// `admin`: accepted without a warning, so `admin passwd` can replace it.
    Accept,
}

/// Reads and checks the account file at `path`. A file without accounts is
/// valid here; `serve` rejects it with `accounts::require_any`.
pub fn load(path: &Path, floor: HashFloor) -> Result<LoadedAccounts, Vec<AccountsDiagnostic>> {
    read(path, floor)?.ok_or_else(|| {
        vec![AccountsDiagnostic {
            error: AccountsError::FileMissing,
            key: ACCOUNTS_FILE_KEY.to_owned(),
            message: format!(
                "계정 파일이 없다: {}. `vnc-relay admin add`로 계정을 만든다.",
                path.display()
            ),
        }]
    })
}

/// Like [`load`], but a missing file is `Ok(None)`.
fn read(path: &Path, floor: HashFloor) -> Result<Option<LoadedAccounts>, Vec<AccountsDiagnostic>> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            let reason = match error.kind() {
                ErrorKind::PermissionDenied => "읽을 권한이 없다",
                ErrorKind::IsADirectory => "파일이 아니라 디렉터리다",
                _ => "읽을 수 없다",
            };
            return Err(vec![AccountsDiagnostic {
                error: AccountsError::FileUnreadable,
                key: ACCOUNTS_FILE_KEY.to_owned(),
                message: format!("계정 파일을 읽을 수 없다({reason}): {}", path.display()),
            }]);
        }
    };
    let text = String::from_utf8(bytes).map_err(|_| {
        vec![AccountsDiagnostic {
            error: AccountsError::FileInvalid,
            key: String::new(),
            message: "계정 파일이 UTF-8 텍스트가 아니다.".to_owned(),
        }]
    })?;
    let accounts = parse(&text, floor)?;
    let warnings = if is_permissive(path) {
        vec![AccountsWarning::FilePermissive]
    } else {
        Vec::new()
    };
    Ok(Some(LoadedAccounts { accounts, warnings }))
}

/// Checks account file text and returns every diagnostic found, or the
/// accounts in file order.
pub fn parse(text: &str, floor: HashFloor) -> Result<Vec<Account>, Vec<AccountsDiagnostic>> {
    let table: Table = text.parse().map_err(|error: toml::de::Error| {
        let message = match error_position(text, &error) {
            Some((line, column)) => {
                format!("계정 파일의 TOML 구문 오류 또는 중복 키다({line}행 {column}열).")
            }
            None => "계정 파일의 TOML 구문 오류 또는 중복 키다.".to_owned(),
        };
        vec![invalid(String::new(), message)]
    })?;
    let mut problems = Vec::new();
    match table.get("version") {
        Some(Value::Integer(1)) => {}
        Some(Value::Integer(_)) => problems.push(invalid(
            "version".to_owned(),
            "지원하지 않는 계정 파일 형식 버전이다. 값은 1이어야 한다.".to_owned(),
        )),
        Some(_) => problems.push(invalid(
            "version".to_owned(),
            "`version`은 정수여야 한다.".to_owned(),
        )),
        None => problems.push(invalid(
            "version".to_owned(),
            "`version`이 없다. 값은 1이어야 한다.".to_owned(),
        )),
    }
    for key in table
        .keys()
        .filter(|key| !["version", "account"].contains(&key.as_str()))
    {
        problems.push(unknown_key(key.clone()));
    }
    let mut accounts = Vec::new();
    match table.get("account") {
        None => {}
        Some(Value::Array(items)) => {
            let mut first_index: BTreeMap<String, usize> = BTreeMap::new();
            for (index, item) in items.iter().enumerate() {
                if let Some(account) =
                    read_account(index, item, floor, &mut first_index, &mut problems)
                {
                    accounts.push(account);
                }
            }
        }
        Some(_) => problems.push(invalid(
            "account".to_owned(),
            "`account`는 테이블 배열(`[[account]]`)이어야 한다.".to_owned(),
        )),
    }
    if problems.is_empty() {
        Ok(accounts)
    } else {
        Err(problems)
    }
}

/// Reads `account[index]`. `first_index` maps each name seen so far to the
/// first account that had it; a later duplicate is reported at itself.
fn read_account(
    index: usize,
    item: &Value,
    floor: HashFloor,
    first_index: &mut BTreeMap<String, usize>,
    problems: &mut Vec<AccountsDiagnostic>,
) -> Option<Account> {
    let path = format!("account[{index}]");
    let Value::Table(table) = item else {
        problems.push(invalid(path, "계정 항목은 테이블이어야 한다.".to_owned()));
        return None;
    };
    for key in table
        .keys()
        .filter(|key| !["name", "password_hash"].contains(&key.as_str()))
    {
        problems.push(unknown_key(format!("{path}.{key}")));
    }
    let name = string_field(table, &path, "name", problems).and_then(|text| {
        let key = format!("{path}.name");
        let name = match AccountName::parse(text) {
            Ok(name) => name,
            Err(error) => {
                problems.push(invalid(key, format!("{error}.")));
                return None;
            }
        };
        if let Some(first) = first_index.get(name.as_str()) {
            problems.push(invalid(
                key,
                format!("같은 이름의 계정이 이미 있다(account[{first}])."),
            ));
            return None;
        }
        first_index.insert(name.as_str().to_owned(), index);
        Some(name)
    });
    let password_hash = string_field(table, &path, "password_hash", problems)
        .and_then(|text| read_hash(text, format!("{path}.password_hash"), floor, problems));
    Some(Account {
        name: name?,
        password_hash: password_hash?,
    })
}

fn string_field<'a>(
    table: &'a Table,
    path: &str,
    field: &str,
    problems: &mut Vec<AccountsDiagnostic>,
) -> Option<&'a str> {
    let key = format!("{path}.{field}");
    match table.get(field) {
        Some(Value::String(text)) => Some(text),
        Some(_) => {
            problems.push(invalid(key, format!("`{field}`은 문자열이어야 한다.")));
            None
        }
        None => {
            problems.push(invalid(key, format!("필수 키 `{field}`가 없다.")));
            None
        }
    }
}

/// Messages name the wrong part only and never quote the hash string.
fn read_hash(
    text: &str,
    key: String,
    floor: HashFloor,
    problems: &mut Vec<AccountsDiagnostic>,
) -> Option<PasswordHash> {
    let hash = match PasswordHash::parse(text) {
        Ok(hash) => hash,
        Err(error) => {
            problems.push(invalid(
                key,
                format!("해시 문자열이 올바르지 않다: {error}."),
            ));
            return None;
        }
    };
    let params = hash.params();
    let error = match (params.check_range(), floor) {
        (Ok(()), _) => return Some(hash),
        // The admin accepts a hash below the minimum so that `admin passwd`
        // can replace it. A parameter above the maximum is refused even when
        // another parameter is also below its minimum.
        (Err(_), HashFloor::Accept) if !params.exceeds_maximum() => return Some(hash),
        (Err(_), HashFloor::Accept) => Argon2ParamsError::AboveMaximum,
        (Err(error), HashFloor::Reject) => error,
    };
    let (code, hint) = match error {
        Argon2ParamsError::BelowMinimum => (
            AccountsError::HashBelowMinimum,
            " `vnc-relay admin passwd`로 해시를 다시 만든다.",
        ),
        Argon2ParamsError::AboveMaximum => (AccountsError::FileInvalid, ""),
    };
    problems.push(AccountsDiagnostic {
        error: code,
        key,
        message: format!(
            "해시 매개변수 m={}, t={}, p={}가 {error}. 승인 범위는 m={}~{}, t={}~{}, p={}~{}다.{hint}",
            params.memory_kib,
            params.iterations,
            params.parallelism,
            Argon2Params::MEMORY_KIB.start(),
            Argon2Params::MEMORY_KIB.end(),
            Argon2Params::ITERATIONS.start(),
            Argon2Params::ITERATIONS.end(),
            Argon2Params::PARALLELISM.start(),
            Argon2Params::PARALLELISM.end(),
        ),
    });
    None
}

fn invalid(key: String, message: String) -> AccountsDiagnostic {
    AccountsDiagnostic {
        error: AccountsError::FileInvalid,
        key,
        message,
    }
}

fn unknown_key(key: String) -> AccountsDiagnostic {
    invalid(key, "알 수 없는 키다.".to_owned())
}

/// Group or other users have any permission bit (cli-v1 §1.6). Unknown
/// metadata gives no warning.
#[cfg(unix)]
fn is_permissive(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).is_ok_and(|metadata| metadata.permissions().mode() & 0o077 != 0)
}

/// Windows gives no permission warning; the operator sets the access
/// control list (cli-v1 §1.6, user decision 2026-10-20).
#[cfg(not(unix))]
fn is_permissive(_path: &Path) -> bool {
    false
}
