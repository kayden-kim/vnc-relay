//! Ports: traits for outside functions that app use cases call. adapters
//! implements them (docs/architectures/code-map.md, layer "app").

use std::fmt;
use std::time::{Duration, SystemTime};

use vnc_relay_domain::account::{Account, Argon2Params, Password, PasswordHash};

use crate::accounts::{AccountsDiagnostic, AccountsWarning, AdminFailure, LoadedAccounts};
use crate::events::Event;

/// Records operation log events (ops-info-v1). Recording does not fail for
/// the caller: a sink that cannot write drops the line (ops-info-v1 §1.1),
/// so a log failure does not change the flow of a use case.
pub trait EventSink: Send + Sync {
    fn record(&self, event: &Event);
}

/// The administrator account file (cli-v1 §1.3, §1.6).
pub trait AccountStore {
    /// Reads and checks the file. `Ok(None)` when the file does not exist.
    fn read(&self) -> Result<Option<LoadedAccounts>, Vec<AccountsDiagnostic>>;

    /// Changes the file in one locked step: takes the lock, reads the file
    /// again (a missing file is an empty list), applies `change`, and
    /// replaces the file with the result. A `change` failure leaves the file
    /// as it was. Steps that fail after the replacement, and a lock that
    /// cannot be removed, go to `warn`.
    fn update(
        &self,
        change: &mut dyn FnMut(Vec<Account>) -> Result<Vec<Account>, AdminFailure>,
        warn: &mut dyn FnMut(AccountsWarning),
    ) -> Result<(), AdminFailure>;
}

/// Operator input of the `admin` commands (cli-v1 §1.5).
pub trait AccountInput {
    /// Writes `prompt` and reads one line without showing the typed text.
    fn hidden(&self, prompt: &str) -> Result<String, InputError>;

    /// Writes `prompt` and reads one visible line. `Ok(None)` at the end of
    /// input.
    fn line(&self, prompt: &str) -> Result<Option<String>, InputError>;

    /// The operator pressed Ctrl+C after the input was opened.
    fn canceled(&self) -> bool;
}

/// Why operator input gave no text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputError {
    /// Standard input is not a terminal.
    NotInteractive,
    /// Ctrl+C, or the end of input before any text.
    Canceled,
    /// The typed bytes are not UTF-8.
    InvalidText,
    /// Input cannot be read at all. The message says why (`runtime_unavailable`).
    Unavailable(String),
}

/// Makes password hashes (cli-v1 §1.4).
pub trait PasswordHasher {
    /// An Argon2id hash of `password` with a new salt and `params`.
    fn hash(&self, password: &Password, params: Argon2Params) -> Result<PasswordHash, HashError>;
}

/// Hashing failed: the OS random source or the hash memory was not
/// available. `0` is a Korean sentence without secret values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashError(pub String);

impl fmt::Display for HashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for HashError {}

/// Time for administrator sessions and the login limit. Deadlines use
/// `monotonic`. `wall` is only used to report expiry times. The app never
/// calls `Instant::now` or `SystemTime::now` (`crates/app/clippy.toml`).
pub trait Clock: Send + Sync {
    /// Time since a fixed start point. It never goes backwards.
    fn monotonic(&self) -> Duration;

    /// Wall-clock time for the expiry values in the session response.
    fn wall(&self) -> SystemTime;
}

/// Checks a typed password against a stored hash (management-api-v1 §1.5).
pub trait PasswordVerifier: Send + Sync {
    /// `Ok(false)` for a mismatch. `Err` only when the check itself failed.
    fn verify(&self, typed: &str, hash: &PasswordHash) -> Result<bool, HashError>;
}

/// Makes admin session tokens and compares them (management-api-v1 §1.4.3).
pub trait TokenSource: Send + Sync {
    /// A new random token of 64 lowercase hex digits.
    fn new_token(&self) -> Result<String, RandomError>;

    /// True when both texts are equal. The comparison takes the same time
    /// whatever the first difference is.
    fn same_token(&self, expected: &str, given: &str) -> bool;
}

/// The OS random source could not give bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RandomError(pub String);

impl fmt::Display for RandomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RandomError {}
