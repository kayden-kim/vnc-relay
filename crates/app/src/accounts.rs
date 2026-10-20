//! Administrator account file codes (cli-v1 §2.2) and the start rule that
//! needs at least one account (operations spec §3.1 item 5).

use std::fmt;

use vnc_relay_domain::account::Account;

/// Config key of the account file path. File-level diagnostics and warnings
/// use it as their `key` (cli-v1 §1.3).
pub const ACCOUNTS_FILE_KEY: &str = "admin.accounts_file";

/// Account file error codes. Every one ends `serve` with exit code 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountsError {
    FileMissing,
    FileUnreadable,
    FileInvalid,
    HashBelowMinimum,
    NoAccounts,
}

impl AccountsError {
    /// The stable code string of the contract.
    pub fn code(self) -> &'static str {
        match self {
            Self::FileMissing => "accounts_file_missing",
            Self::FileUnreadable => "accounts_file_unreadable",
            Self::FileInvalid => "accounts_file_invalid",
            Self::HashBelowMinimum => "accounts_hash_below_minimum",
            Self::NoAccounts => "accounts_none",
        }
    }
}

impl fmt::Display for AccountsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for AccountsError {}

/// One account file diagnostic. `key` follows cli-v1 §1.3: a path inside the
/// account file, [`ACCOUNTS_FILE_KEY`] for file-level errors, or empty for
/// syntax and encoding errors. `message` never quotes a hash or file content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountsDiagnostic {
    pub error: AccountsError,
    pub key: String,
    pub message: String,
}

/// Account file warnings (cli-v1 §1.6). Their key is [`ACCOUNTS_FILE_KEY`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountsWarning {
    /// Group or other users have some permission on the file (Ubuntu).
    FilePermissive,
}

impl AccountsWarning {
    pub fn code(self) -> &'static str {
        match self {
            Self::FilePermissive => "accounts_file_permissive",
        }
    }
}

/// `serve` does not start without an account.
pub fn require_any(accounts: &[Account]) -> Result<(), AccountsDiagnostic> {
    if accounts.is_empty() {
        Err(AccountsDiagnostic {
            error: AccountsError::NoAccounts,
            key: ACCOUNTS_FILE_KEY.to_owned(),
            message: "관리자 계정이 없다. `vnc-relay admin add`로 계정을 만든 뒤 다시 시작한다."
                .to_owned(),
        })
    } else {
        Ok(())
    }
}
