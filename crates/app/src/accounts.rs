//! Administrator account file codes (cli-v1 §2.2), the start rule that
//! needs at least one account (operations spec §3.1 item 5), and the
//! `admin` procedures (cli-v1 §1.5). Design: docs/architectures/admin-accounts.md.

use std::fmt;

use vnc_relay_domain::account::{Account, AccountName, Argon2Params, Password, PasswordHash};

use crate::ports::{AccountInput, AccountStore, InputError, PasswordHasher};

/// Config key of the account file path. File-level diagnostics and warnings
/// use it as their `key` (cli-v1 §1.3).
pub const ACCOUNTS_FILE_KEY: &str = "admin.accounts_file";

/// Account file error codes. Every one ends the command with exit code 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountsError {
    FileMissing,
    FileUnreadable,
    FileInvalid,
    HashBelowMinimum,
    NoAccounts,
    FileLocked,
    WriteFailed,
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
            Self::FileLocked => "accounts_file_locked",
            Self::WriteFailed => "accounts_file_write_failed",
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

/// Account file warnings (cli-v1 §1.5, §1.6). Their key is
/// [`ACCOUNTS_FILE_KEY`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountsWarning {
    /// Group or other users have some permission on the file (Ubuntu).
    FilePermissive,
    /// `admin remove` left the file without accounts.
    LastAccountRemoved,
    /// A step after the replacement failed. The text names the step, the
    /// path, and the OS error.
    CleanupFailed(String),
}

impl AccountsWarning {
    pub fn code(&self) -> &'static str {
        match self {
            Self::FilePermissive => "accounts_file_permissive",
            Self::LastAccountRemoved => "last_account_removed",
            Self::CleanupFailed(_) => "accounts_file_cleanup_failed",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::FilePermissive => {
                "그룹이나 다른 사용자에게 계정 파일 권한이 있다. 권한을 0600으로 바꾸기를 권장한다."
                    .to_owned()
            }
            Self::LastAccountRemoved => "계정이 하나도 남지 않았다. `vnc-relay serve`는 계정 없이 시작하지 않는다. `vnc-relay admin add`로 계정을 만든다.".to_owned(),
            Self::CleanupFailed(detail) => detail.clone(),
        }
    }
}

/// Accounts read from a valid file, in file order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedAccounts {
    pub accounts: Vec<Account>,
    pub warnings: Vec<AccountsWarning>,
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

/// Why an `admin` command stopped. In every case the account file is as it
/// was before the command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdminFailure {
    NotInteractive,
    AccountExists(AccountName),
    AccountNotFound(AccountName),
    /// The sentence says which part of the rule the input breaks.
    PasswordInvalid(String),
    PasswordMismatch,
    InputCanceled,
    /// The Ctrl+C detector, the OS random source, or the hash memory is not
    /// available (`runtime_unavailable`).
    RuntimeUnavailable(String),
    Accounts(Vec<AccountsDiagnostic>),
}

impl AdminFailure {
    /// Calls `line(code, key, message)` for each diagnostic line. `key` is
    /// empty when the line has none.
    pub fn report(&self, line: &mut dyn FnMut(&str, &str, &str)) {
        let (code, message) = match self {
            Self::Accounts(diagnostics) => {
                for d in diagnostics {
                    line(d.error.code(), &d.key, &d.message);
                }
                return;
            }
            Self::NotInteractive => (
                "not_interactive",
                "표준 입력이 터미널이 아니라 입력을 받을 수 없다. 터미널에서 실행한다.".to_owned(),
            ),
            Self::AccountExists(name) => {
                ("account_exists", format!("계정 '{name}'이(가) 이미 있다."))
            }
            Self::AccountNotFound(name) => {
                ("account_not_found", format!("계정 '{name}'이(가) 없다."))
            }
            Self::PasswordInvalid(problem) => (
                "password_invalid",
                format!(
                    "{problem}. 비밀번호는 {}~{}자, UTF-8 {}바이트 이하이고 제어 문자가 없어야 한다.",
                    Password::MIN_CHARS,
                    Password::MAX_CHARS,
                    Password::MAX_BYTES
                ),
            ),
            Self::PasswordMismatch => (
                "password_mismatch",
                "두 번 입력한 비밀번호가 다르다.".to_owned(),
            ),
            Self::InputCanceled => (
                "input_canceled",
                "입력을 취소했다. 계정 파일은 바뀌지 않았다.".to_owned(),
            ),
            Self::RuntimeUnavailable(message) => ("runtime_unavailable", message.clone()),
        };
        line(code, "", &message);
    }
}

const PASSWORD_PROMPT: &str = "비밀번호: ";
const PASSWORD_CONFIRM_PROMPT: &str = "비밀번호 확인: ";
const NEW_PASSWORD_PROMPT: &str = "새 비밀번호: ";
const NEW_PASSWORD_CONFIRM_PROMPT: &str = "새 비밀번호 확인: ";

/// `admin list`: the account names in name order, or `None` when the file
/// does not exist.
pub fn list(
    store: &dyn AccountStore,
    warn: &mut dyn FnMut(AccountsWarning),
) -> Result<Option<Vec<AccountName>>, AdminFailure> {
    let Some(loaded) = store.read().map_err(AdminFailure::Accounts)? else {
        return Ok(None);
    };
    loaded.warnings.into_iter().for_each(&mut *warn);
    let mut names: Vec<AccountName> = loaded.accounts.into_iter().map(|a| a.name).collect();
    names.sort();
    Ok(Some(names))
}

/// The `admin` commands that change the account file (cli-v1 §1.5 order).
/// Warnings go to the `warn` function of each call as they happen.
pub struct AdminService<'a> {
    pub store: &'a dyn AccountStore,
    pub input: &'a dyn AccountInput,
    pub hasher: &'a dyn PasswordHasher,
    /// Parameters of new hashes (`admin.argon2`).
    pub params: Argon2Params,
}

impl AdminService<'_> {
    /// `admin add`. A missing file is created.
    pub fn add(
        &self,
        name: &AccountName,
        warn: &mut dyn FnMut(AccountsWarning),
    ) -> Result<(), AdminFailure> {
        let accounts = self.read(warn)?.unwrap_or_default();
        if position(&accounts, name).is_some() {
            return Err(AdminFailure::AccountExists(name.clone()));
        }
        let hash = self.new_hash(PASSWORD_PROMPT, PASSWORD_CONFIRM_PROMPT)?;
        self.save(warn, &mut |mut accounts| {
            if position(&accounts, name).is_some() {
                return Err(AdminFailure::AccountExists(name.clone()));
            }
            accounts.push(Account {
                name: name.clone(),
                password_hash: hash.clone(),
            });
            Ok(accounts)
        })
    }

    /// `admin passwd`. The current password is not asked.
    pub fn passwd(
        &self,
        name: &AccountName,
        warn: &mut dyn FnMut(AccountsWarning),
    ) -> Result<(), AdminFailure> {
        let accounts = self.read(warn)?.unwrap_or_default();
        if position(&accounts, name).is_none() {
            return Err(AdminFailure::AccountNotFound(name.clone()));
        }
        let hash = self.new_hash(NEW_PASSWORD_PROMPT, NEW_PASSWORD_CONFIRM_PROMPT)?;
        self.save(warn, &mut |mut accounts| {
            let index = position(&accounts, name)
                .ok_or_else(|| AdminFailure::AccountNotFound(name.clone()))?;
            accounts[index].password_hash = hash.clone();
            Ok(accounts)
        })
    }

    /// `admin remove`. `assume_yes` (`--yes`) skips the confirmation.
    pub fn remove(
        &self,
        name: &AccountName,
        assume_yes: bool,
        warn: &mut dyn FnMut(AccountsWarning),
    ) -> Result<(), AdminFailure> {
        let accounts = self.read(warn)?.unwrap_or_default();
        if position(&accounts, name).is_none() {
            return Err(AdminFailure::AccountNotFound(name.clone()));
        }
        if !assume_yes {
            let prompt = format!("계정 '{name}'을(를) 삭제할까요? [y/N] ");
            let approved = match self.input.line(&prompt) {
                Ok(Some(answer)) => {
                    let answer = answer.trim_end_matches(['\r', '\n']);
                    answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes")
                }
                Ok(None) | Err(InputError::Canceled | InputError::InvalidText) => false,
                Err(InputError::NotInteractive) => return Err(AdminFailure::NotInteractive),
            };
            if !approved {
                return Err(AdminFailure::InputCanceled);
            }
        }
        let mut remaining = 0;
        self.save(warn, &mut |mut accounts| {
            let index = position(&accounts, name)
                .ok_or_else(|| AdminFailure::AccountNotFound(name.clone()))?;
            accounts.remove(index);
            remaining = accounts.len();
            Ok(accounts)
        })?;
        if remaining == 0 {
            warn(AccountsWarning::LastAccountRemoved);
        }
        Ok(())
    }

    /// The accounts of the file, or `None` when it does not exist.
    fn read(
        &self,
        warn: &mut dyn FnMut(AccountsWarning),
    ) -> Result<Option<Vec<Account>>, AdminFailure> {
        let loaded = self.store.read().map_err(AdminFailure::Accounts)?;
        Ok(loaded.map(|loaded| {
            loaded.warnings.into_iter().for_each(&mut *warn);
            loaded.accounts
        }))
    }

    /// First entry, rule check, confirmation, comparison, then the hash.
    fn new_hash(&self, prompt: &str, confirm_prompt: &str) -> Result<PasswordHash, AdminFailure> {
        let password = Password::parse(self.hidden(prompt)?)
            .map_err(|e| AdminFailure::PasswordInvalid(e.to_string()))?;
        if self.hidden(confirm_prompt)?.as_bytes() != password.as_bytes() {
            return Err(AdminFailure::PasswordMismatch);
        }
        self.hasher
            .hash(&password, self.params)
            .map_err(|e| AdminFailure::RuntimeUnavailable(e.0))
    }

    fn hidden(&self, prompt: &str) -> Result<String, AdminFailure> {
        self.input.hidden(prompt).map_err(|error| match error {
            InputError::NotInteractive => AdminFailure::NotInteractive,
            InputError::Canceled => AdminFailure::InputCanceled,
            InputError::InvalidText => {
                AdminFailure::PasswordInvalid("입력한 비밀번호가 UTF-8 텍스트가 아니다".to_owned())
            }
        })
    }

    /// A Ctrl+C that came during hashing or confirmation still cancels.
    fn save(
        &self,
        warn: &mut dyn FnMut(AccountsWarning),
        change: &mut dyn FnMut(Vec<Account>) -> Result<Vec<Account>, AdminFailure>,
    ) -> Result<(), AdminFailure> {
        if self.input.canceled() {
            return Err(AdminFailure::InputCanceled);
        }
        self.store.update(change, warn)
    }
}

fn position(accounts: &[Account], name: &AccountName) -> Option<usize> {
    accounts.iter().position(|a| &a.name == name)
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;

    use super::*;
    use crate::ports::HashError;

    const HASH_A: &str = "$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    const HASH_B: &str = "$argon2id$v=19$m=19456,t=2,p=1$BBBBBBBBBBBBBBBBBBBBBA$BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBA";
    const PASSWORD: &str = "correct horse battery";

    type Answer = Result<Option<&'static str>, InputError>;
    type Seen = RefCell<Vec<&'static str>>;

    fn name(text: &str) -> AccountName {
        AccountName::parse(text).unwrap_or_else(|e| panic!("{e}"))
    }

    fn hash(text: &str) -> PasswordHash {
        PasswordHash::parse(text).unwrap_or_else(|e| panic!("{e}"))
    }

    fn account(text: &str, hash_text: &str) -> Account {
        Account {
            name: name(text),
            password_hash: hash(hash_text),
        }
    }

    /// An account file in memory. `on_lock` replaces the file content when
    /// `update` takes the lock, as another command would have.
    #[derive(Default)]
    struct FakeStore {
        file: RefCell<Option<Vec<Account>>>,
        on_lock: RefCell<Option<Vec<Account>>>,
        updates: Cell<u32>,
        cleanup: Vec<AccountsWarning>,
    }

    impl FakeStore {
        fn with(accounts: Vec<Account>) -> Self {
            Self {
                file: RefCell::new(Some(accounts)),
                ..Self::default()
            }
        }

        fn names(&self) -> Option<Vec<String>> {
            self.file
                .borrow()
                .as_ref()
                .map(|accounts| accounts.iter().map(|a| a.name.to_string()).collect())
        }
    }

    impl AccountStore for FakeStore {
        fn read(&self) -> Result<Option<LoadedAccounts>, Vec<AccountsDiagnostic>> {
            Ok(self.file.borrow().clone().map(|accounts| LoadedAccounts {
                accounts,
                warnings: vec![AccountsWarning::FilePermissive],
            }))
        }

        fn update(
            &self,
            change: &mut dyn FnMut(Vec<Account>) -> Result<Vec<Account>, AdminFailure>,
            warn: &mut dyn FnMut(AccountsWarning),
        ) -> Result<(), AdminFailure> {
            self.updates.set(self.updates.get() + 1);
            if let Some(changed) = self.on_lock.borrow_mut().take() {
                *self.file.borrow_mut() = Some(changed);
            }
            let current = self.file.borrow().clone().unwrap_or_default();
            let next = change(current)?;
            *self.file.borrow_mut() = Some(next);
            self.cleanup.iter().cloned().for_each(warn);
            Ok(())
        }
    }

    /// Scripted operator input. Each `hidden` or `line` call takes the next
    /// answer and records its prompt.
    #[derive(Default)]
    struct FakeInput {
        answers: RefCell<VecDeque<Result<Option<String>, InputError>>>,
        prompts: RefCell<Vec<String>>,
        canceled: Cell<bool>,
    }

    impl FakeInput {
        fn answering(answers: &[Result<Option<&str>, InputError>]) -> Self {
            Self {
                answers: RefCell::new(
                    answers
                        .iter()
                        .map(|a| a.map(|text| text.map(str::to_owned)))
                        .collect(),
                ),
                ..Self::default()
            }
        }

        fn next(&self, prompt: &str) -> Result<Option<String>, InputError> {
            self.prompts.borrow_mut().push(prompt.to_owned());
            self.answers
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| panic!("unexpected prompt {prompt:?}"))
        }
    }

    impl AccountInput for FakeInput {
        fn hidden(&self, prompt: &str) -> Result<String, InputError> {
            self.next(prompt)?.ok_or(InputError::Canceled)
        }

        fn line(&self, prompt: &str) -> Result<Option<String>, InputError> {
            self.next(prompt)
        }

        fn canceled(&self) -> bool {
            self.canceled.get()
        }
    }

    struct FakeHasher;

    impl PasswordHasher for FakeHasher {
        fn hash(
            &self,
            password: &Password,
            params: Argon2Params,
        ) -> Result<PasswordHash, HashError> {
            assert_eq!(password.as_bytes(), PASSWORD.as_bytes());
            assert_eq!(params.memory_kib, 65_536);
            Ok(hash(HASH_B))
        }
    }

    const PARAMS: Argon2Params = Argon2Params {
        memory_kib: 65_536,
        iterations: 3,
        parallelism: 1,
    };

    fn service<'a>(store: &'a FakeStore, input: &'a FakeInput) -> AdminService<'a> {
        AdminService {
            store,
            input,
            hasher: &FakeHasher,
            params: PARAMS,
        }
    }

    fn twice(password: &str) -> [Result<Option<&str>, InputError>; 2] {
        [Ok(Some(password)), Ok(Some(password))]
    }

    /// spec: operations.md:OPS-029 partial
    #[test]
    fn adds_an_account_after_two_matching_entries_and_creates_a_missing_file() {
        let store = FakeStore::default();
        let input = FakeInput::answering(&twice(PASSWORD));
        let seen: Seen = RefCell::new(Vec::new());
        service(&store, &input)
            .add(&name("alice"), &mut |w: AccountsWarning| {
                seen.borrow_mut().push(w.code())
            })
            .unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(store.names(), Some(vec!["alice".to_owned()]));
        assert_eq!(
            store
                .file
                .borrow()
                .as_ref()
                .map(|a| a[0].password_hash.as_str().to_owned()),
            Some(HASH_B.to_owned())
        );
        assert_eq!(*input.prompts.borrow(), ["비밀번호: ", "비밀번호 확인: "]);
        assert!(seen.borrow().is_empty());

        // A second account; the read warning is passed on once.
        let input = FakeInput::answering(&twice(PASSWORD));
        service(&store, &input)
            .add(&name("bob"), &mut |w: AccountsWarning| {
                seen.borrow_mut().push(w.code())
            })
            .unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(
            store.names(),
            Some(vec!["alice".to_owned(), "bob".to_owned()])
        );
        assert_eq!(*seen.borrow(), ["accounts_file_permissive"]);
    }

    /// spec: operations.md:OPS-029 partial
    #[test]
    fn checks_the_name_before_asking_and_again_under_the_lock() {
        let store = FakeStore::with(vec![account("alice", HASH_A)]);
        let input = FakeInput::default();
        let result = service(&store, &input).add(&name("alice"), &mut |_| {});
        assert_eq!(result, Err(AdminFailure::AccountExists(name("alice"))));
        assert!(input.prompts.borrow().is_empty());
        for missing in [
            service(&store, &input).passwd(&name("bob"), &mut |_| {}),
            service(&store, &input).remove(&name("bob"), true, &mut |_| {}),
        ] {
            assert_eq!(missing, Err(AdminFailure::AccountNotFound(name("bob"))));
        }
        let empty = FakeStore::default();
        assert_eq!(
            service(&empty, &input).passwd(&name("bob"), &mut |_| {}),
            Err(AdminFailure::AccountNotFound(name("bob")))
        );
        assert_eq!(store.updates.get(), 0);

        // Another command added the same name between the read and the lock.
        let store = FakeStore::default();
        *store.on_lock.borrow_mut() = Some(vec![account("carol", HASH_A)]);
        let input = FakeInput::answering(&twice(PASSWORD));
        assert_eq!(
            service(&store, &input).add(&name("carol"), &mut |_| {}),
            Err(AdminFailure::AccountExists(name("carol")))
        );
        assert_eq!(
            store
                .file
                .borrow()
                .as_ref()
                .map(|a| a[0].password_hash.as_str().to_owned()),
            Some(HASH_A.to_owned())
        );
    }

    /// spec: operations.md:OPS-030 partial
    #[test]
    fn stops_before_saving_on_invalid_mismatched_or_canceled_input() {
        let cases: [(&[Answer], AdminFailure, usize); 6] = [
            (
                &[Ok(Some("short"))],
                AdminFailure::PasswordInvalid("비밀번호가 12자보다 짧다".to_owned()),
                1,
            ),
            (
                &[Ok(Some(PASSWORD)), Ok(Some("correct horse battery!"))],
                AdminFailure::PasswordMismatch,
                2,
            ),
            (&[Err(InputError::Canceled)], AdminFailure::InputCanceled, 1),
            (
                &[Ok(Some(PASSWORD)), Err(InputError::Canceled)],
                AdminFailure::InputCanceled,
                2,
            ),
            (
                &[Err(InputError::NotInteractive)],
                AdminFailure::NotInteractive,
                1,
            ),
            (
                &[Err(InputError::InvalidText)],
                AdminFailure::PasswordInvalid("입력한 비밀번호가 UTF-8 텍스트가 아니다".to_owned()),
                1,
            ),
        ];
        for (answers, failure, prompts) in cases {
            let store = FakeStore::with(vec![account("alice", HASH_A)]);
            let input = FakeInput::answering(answers);
            assert_eq!(
                service(&store, &input).passwd(&name("alice"), &mut |_| {}),
                Err(failure.clone())
            );
            assert_eq!(input.prompts.borrow().len(), prompts, "{failure:?}");
            assert_eq!(store.updates.get(), 0, "{failure:?}");
        }

        // Ctrl+C during hashing: the entries were fine but nothing is saved.
        let store = FakeStore::with(vec![account("alice", HASH_A)]);
        let input = FakeInput::answering(&twice(PASSWORD));
        input.canceled.set(true);
        assert_eq!(
            service(&store, &input).passwd(&name("alice"), &mut |_| {}),
            Err(AdminFailure::InputCanceled)
        );
        assert_eq!(
            *input.prompts.borrow(),
            ["새 비밀번호: ", "새 비밀번호 확인: "]
        );
        assert_eq!(store.updates.get(), 0);
    }

    /// spec: operations.md:OPS-029 partial
    #[test]
    fn changes_only_the_hash_of_the_named_account() {
        let store = FakeStore::with(vec![account("alice", HASH_A), account("bob", HASH_A)]);
        let input = FakeInput::answering(&twice(PASSWORD));
        service(&store, &input)
            .passwd(&name("bob"), &mut |_| {})
            .unwrap_or_else(|e| panic!("{e:?}"));
        let file = store.file.borrow();
        let hashes: Vec<&str> = file
            .iter()
            .flatten()
            .map(|a| a.password_hash.as_str())
            .collect();
        assert_eq!(hashes, [HASH_A, HASH_B]);
    }

    /// spec: operations.md:OPS-030 partial
    #[test]
    fn removes_only_after_y_or_yes_and_warns_when_none_is_left() {
        for answer in ["y", "Y", "yes", "YES\r\n", "Yes\n"] {
            let store = FakeStore::with(vec![account("alice", HASH_A), account("bob", HASH_A)]);
            let input = FakeInput::answering(&[Ok(Some(answer))]);
            let seen: Seen = RefCell::new(Vec::new());
            service(&store, &input)
                .remove(&name("alice"), false, &mut |w: AccountsWarning| {
                    seen.borrow_mut().push(w.code())
                })
                .unwrap_or_else(|e| panic!("{answer:?}: {e:?}"));
            assert_eq!(store.names(), Some(vec!["bob".to_owned()]), "{answer:?}");
            assert_eq!(
                *input.prompts.borrow(),
                ["계정 'alice'을(를) 삭제할까요? [y/N] "]
            );
            assert_eq!(*seen.borrow(), ["accounts_file_permissive"]);
        }
        for answer in [
            Ok(Some("")),
            Ok(Some("n")),
            Ok(Some("ye")),
            Ok(Some(" y")),
            Ok(None),
            Err(InputError::Canceled),
        ] {
            let store = FakeStore::with(vec![account("alice", HASH_A)]);
            let input = FakeInput::answering(&[answer]);
            assert_eq!(
                service(&store, &input).remove(&name("alice"), false, &mut |_| {}),
                Err(AdminFailure::InputCanceled),
                "{answer:?}"
            );
            assert_eq!(store.updates.get(), 0);
        }
        let store = FakeStore::with(vec![account("alice", HASH_A)]);
        let input = FakeInput::answering(&[Err(InputError::NotInteractive)]);
        assert_eq!(
            service(&store, &input).remove(&name("alice"), false, &mut |_| {}),
            Err(AdminFailure::NotInteractive)
        );

        // `--yes` asks nothing; removing the last account warns.
        let input = FakeInput::default();
        let seen: Seen = RefCell::new(Vec::new());
        service(&store, &input)
            .remove(&name("alice"), true, &mut |w: AccountsWarning| {
                seen.borrow_mut().push(w.code())
            })
            .unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(store.names(), Some(Vec::new()));
        assert_eq!(
            *seen.borrow(),
            ["accounts_file_permissive", "last_account_removed"]
        );
    }

    #[test]
    fn passes_on_cleanup_warnings_after_saving() {
        let mut store = FakeStore::with(vec![account("alice", HASH_A)]);
        store.cleanup = vec![AccountsWarning::CleanupFailed(
            "잠금 파일을 지우지 못했다.".to_owned(),
        )];
        let input = FakeInput::answering(&twice(PASSWORD));
        let seen: Seen = RefCell::new(Vec::new());
        service(&store, &input)
            .passwd(&name("alice"), &mut |w: AccountsWarning| {
                seen.borrow_mut().push(w.code())
            })
            .unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(
            *seen.borrow(),
            ["accounts_file_permissive", "accounts_file_cleanup_failed"]
        );
    }

    #[test]
    fn lists_names_in_name_order_and_tells_a_missing_file() {
        let store = FakeStore::with(vec![
            account("zed", HASH_A),
            account("alice", HASH_A),
            account("bob", HASH_A),
        ]);
        let names = list(&store, &mut |_| {}).unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(
            names.map(|n| n.iter().map(ToString::to_string).collect::<Vec<_>>()),
            Some(vec!["alice".to_owned(), "bob".to_owned(), "zed".to_owned()])
        );
        assert_eq!(list(&FakeStore::default(), &mut |_| {}), Ok(None));
    }
}
