//! Contract tests of the account file reader and writer
//! (docs/contracts/cli-v1.md §1.3, §1.4, §1.6; recorded in §5).

use std::path::{Path, PathBuf};

use vnc_relay_adapters::accounts_file::{AccountsFile, HashFloor, load, parse};
use vnc_relay_app::accounts::{AccountsWarning, AdminFailure};
use vnc_relay_app::ports::AccountStore;
use vnc_relay_domain::account::{Account, AccountName, PasswordHash};

/// 16 and 32 zero bytes in unpadded Base64. Test data, not a real hash.
const SALT: &str = "AAAAAAAAAAAAAAAAAAAAAA";
const HASH: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

fn phc(params: &str) -> String {
    format!("$argon2id$v=19${params}${SALT}${HASH}")
}

fn account(name: &str, hash: &str) -> String {
    format!("[[account]]\nname = \"{name}\"\npassword_hash = \"{hash}\"\n")
}

fn names(text: &str) -> Vec<String> {
    match parse(text, HashFloor::Reject) {
        Ok(accounts) => accounts
            .into_iter()
            .map(|a| a.name.as_str().to_owned())
            .collect(),
        Err(diagnostics) => panic!("unexpected diagnostics: {diagnostics:#?}"),
    }
}

/// `(code, key)` of every diagnostic, in order. Also checks that no message
/// quotes the hash text.
fn problems(text: &str) -> Vec<(&'static str, String)> {
    match parse(text, HashFloor::Reject) {
        Ok(accounts) => panic!("parsed unexpectedly: {accounts:#?}"),
        Err(diagnostics) => diagnostics
            .into_iter()
            .map(|d| {
                assert!(!d.message.contains(HASH), "hash in message: {}", d.message);
                assert!(!d.message.contains(SALT), "salt in message: {}", d.message);
                (d.error.code(), d.key)
            })
            .collect(),
    }
}

fn problem(code: &'static str, key: &str) -> (&'static str, String) {
    (code, key.to_owned())
}

fn temp_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("vnc-relay-accounts-{}-{name}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir)
            .unwrap_or_else(|error| panic!("clear {}: {error}", dir.display()));
    }
    std::fs::create_dir_all(&dir)
        .unwrap_or_else(|error| panic!("create {}: {error}", dir.display()));
    dir
}

#[test]
fn reads_accounts_in_file_order_and_accepts_an_empty_file() {
    let hash = phc("m=19456,t=2,p=1");
    let text = format!(
        "version = 1\n\n{}\n{}",
        account("zed", &hash),
        account("alice", &phc("m=65536,t=3,p=4"))
    );
    assert_eq!(names(&text), ["zed", "alice"]);
    assert_eq!(names("version = 1\n"), Vec::<String>::new());
}

#[test]
fn reports_every_problem_with_keys_inside_the_file() {
    let hash = phc("m=19456,t=2,p=1");
    let text = format!(
        "version = 2\nowner = \"ops\"\n\n{}\n{}\n{}\n{}",
        account("alice", &hash),
        account("alice", &hash),
        "[[account]]\nname = \"Bob\"\nhash = \"x\"\n",
        "[[account]]\nname = 7\npassword_hash = false\n",
    );
    assert_eq!(
        problems(&text),
        [
            problem("accounts_file_invalid", "version"),
            problem("accounts_file_invalid", "owner"),
            problem("accounts_file_invalid", "account[1].name"),
            problem("accounts_file_invalid", "account[2].hash"),
            problem("accounts_file_invalid", "account[2].name"),
            problem("accounts_file_invalid", "account[2].password_hash"),
            problem("accounts_file_invalid", "account[3].name"),
            problem("accounts_file_invalid", "account[3].password_hash"),
        ]
    );
    assert_eq!(
        problems("[[account]]\nname = \"alice\"\n"),
        [
            problem("accounts_file_invalid", "version"),
            problem("accounts_file_invalid", "account[0].password_hash"),
        ]
    );
    assert_eq!(
        problems("version = \"1\"\naccount = 1\n"),
        [
            problem("accounts_file_invalid", "version"),
            problem("accounts_file_invalid", "account"),
        ]
    );
}

#[test]
fn rejects_hash_strings_outside_the_format() {
    let key = "account[0].password_hash";
    for hash in [
        "plain-text".to_owned(),
        format!("$argon2i$v=19$m=19456,t=2,p=1${SALT}${HASH}"),
        format!("$argon2id$v=16$m=19456,t=2,p=1${SALT}${HASH}"),
        phc("t=2,m=19456,p=1"),
        phc("m=19456,t=2,p=1,keyid=k"),
        format!("$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAA${HASH}"),
        format!("$argon2id$v=19$m=19456,t=2,p=1${SALT}$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"),
        format!("$argon2id$v=19$m=19456,t=2,p=1${SALT}${HASH}="),
    ] {
        let text = format!("version = 1\n{}", account("alice", &hash));
        assert_eq!(
            problems(&text),
            [problem("accounts_file_invalid", key)],
            "{hash}"
        );
    }
}

#[test]
fn tells_below_minimum_parameters_from_above_maximum_ones() {
    let key = "account[0].password_hash";
    for params in ["m=19455,t=2,p=1", "m=19456,t=1,p=1", "m=19456,t=2,p=0"] {
        let text = format!("version = 1\n{}", account("alice", &phc(params)));
        assert_eq!(
            problems(&text),
            [problem("accounts_hash_below_minimum", key)],
            "{params}"
        );
        // `admin` reads the same file so that `admin passwd` can fix it.
        assert_eq!(
            parse(&text, HashFloor::Accept).map(|a| a.len()),
            Ok(1),
            "{params}"
        );
    }
    for params in ["m=1048577,t=2,p=1", "m=19456,t=11,p=1", "m=19456,t=2,p=17"] {
        let text = format!("version = 1\n{}", account("alice", &phc(params)));
        assert_eq!(
            problems(&text),
            [problem("accounts_file_invalid", key)],
            "{params}"
        );
        assert!(parse(&text, HashFloor::Accept).is_err(), "{params}");
    }
    let upper = format!(
        "version = 1\n{}",
        account("alice", &phc("m=1048576,t=10,p=16"))
    );
    assert_eq!(names(&upper), ["alice"]);
}

#[test]
fn rejects_syntax_errors_and_duplicate_keys_without_a_key() {
    for text in ["version = 1\n[[account]\n", "version = 1\nversion = 1\n"] {
        assert_eq!(
            problems(text),
            [problem("accounts_file_invalid", "")],
            "{text}"
        );
    }
}

#[test]
fn reports_missing_unreadable_and_non_utf8_files() {
    let dir = temp_dir("files");
    let file_error = |path: &Path| match load(path, HashFloor::Reject) {
        Ok(loaded) => panic!("loaded unexpectedly: {loaded:#?}"),
        Err(diagnostics) => diagnostics
            .into_iter()
            .map(|d| (d.error.code(), d.key))
            .collect::<Vec<_>>(),
    };
    assert_eq!(
        file_error(&dir.join("missing.toml")),
        [problem("accounts_file_missing", "admin.accounts_file")]
    );
    assert_eq!(
        file_error(&dir),
        [problem("accounts_file_unreadable", "admin.accounts_file")]
    );
    let latin1 = dir.join("latin1.toml");
    std::fs::write(&latin1, b"version = 1\n# \xe9\n")
        .unwrap_or_else(|error| panic!("write: {error}"));
    assert_eq!(file_error(&latin1), [problem("accounts_file_invalid", "")]);
}

#[cfg(unix)]
#[test]
fn warns_when_group_or_others_have_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let dir = temp_dir("modes");
    let path = dir.join("admin-accounts.toml");
    let text = format!("version = 1\n{}", account("alice", &phc("m=19456,t=2,p=1")));
    std::fs::write(&path, text).unwrap_or_else(|error| panic!("write: {error}"));
    let warnings = |mode: u32| {
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode))
            .unwrap_or_else(|error| panic!("chmod: {error}"));
        match load(&path, HashFloor::Reject) {
            Ok(loaded) => loaded.warnings,
            Err(diagnostics) => panic!("unexpected diagnostics: {diagnostics:#?}"),
        }
    };
    assert_eq!(warnings(0o600), []);
    assert_eq!(warnings(0o400), []);
    assert_eq!(warnings(0o640), [AccountsWarning::FilePermissive]);
    assert_eq!(warnings(0o604), [AccountsWarning::FilePermissive]);
}

fn stored(name: &str) -> Account {
    Account {
        name: AccountName::parse(name).unwrap_or_else(|e| panic!("{e}")),
        password_hash: PasswordHash::parse(&phc("m=19456,t=2,p=1"))
            .unwrap_or_else(|e| panic!("{e}")),
    }
}

/// Runs `update` and returns its result and the warnings it gave.
fn update(
    file: &AccountsFile,
    mut change: impl FnMut(Vec<Account>) -> Result<Vec<Account>, AdminFailure>,
) -> (Result<(), AdminFailure>, Vec<AccountsWarning>) {
    let mut warnings = Vec::new();
    let result = file.update(&mut change, &mut |w| warnings.push(w));
    (result, warnings)
}

/// File names in `dir`, sorted.
fn entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read_dir: {e}"))
        .map(|entry| {
            entry
                .unwrap_or_else(|e| panic!("entry: {e}"))
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

fn failure_codes(result: Result<(), AdminFailure>) -> Vec<&'static str> {
    match result {
        Err(AdminFailure::Accounts(diagnostics)) => {
            diagnostics.iter().map(|d| d.error.code()).collect()
        }
        other => panic!("expected an account file failure: {other:?}"),
    }
}

#[test]
fn creates_a_missing_file_in_name_order_and_leaves_no_lock_or_temporary_file() {
    let dir = temp_dir("write");
    let path = dir.join("admin-accounts.toml");
    let file = AccountsFile::new(path.clone());
    assert_eq!(file.read(), Ok(None));
    let (result, warnings) = update(&file, |mut accounts| {
        assert!(accounts.is_empty());
        accounts.extend([stored("zed"), stored("alice")]);
        Ok(accounts)
    });
    assert_eq!(result, Ok(()));
    assert_eq!(warnings, []);
    let hash = phc("m=19456,t=2,p=1");
    assert_eq!(
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read: {e}")),
        format!(
            "version = 1\n\n{}\n{}",
            account("alice", &hash),
            account("zed", &hash)
        )
    );
    assert_eq!(entries(&dir), ["admin-accounts.toml"]);
    // `serve` reads what `admin` wrote.
    let loaded = load(&path, HashFloor::Reject).unwrap_or_else(|d| panic!("{d:#?}"));
    assert_eq!(loaded.accounts, [stored("alice"), stored("zed")]);
    assert_eq!(loaded.warnings, []);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path)
            .unwrap_or_else(|e| panic!("metadata: {e}"))
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    // The change sees the file as it is under the lock.
    let (result, _) = update(&file, |accounts| {
        assert_eq!(accounts, [stored("alice"), stored("zed")]);
        Ok(accounts
            .into_iter()
            .filter(|a| a.name.as_str() != "zed")
            .collect())
    });
    assert_eq!(result, Ok(()));
    assert_eq!(
        names(&std::fs::read_to_string(&path).unwrap_or_default()),
        ["alice"]
    );
}

#[test]
fn keeps_the_file_when_the_change_fails_the_lock_exists_or_writing_fails() {
    let dir = temp_dir("keep");
    let path = dir.join("admin-accounts.toml");
    let original = format!(
        "# kept\nversion = 1\n{}",
        account("alice", &phc("m=19456,t=2,p=1"))
    );
    std::fs::write(&path, &original).unwrap_or_else(|e| panic!("write: {e}"));
    let file = AccountsFile::new(path.clone());
    let contents = || std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read: {e}"));
    let name = AccountName::parse("alice").unwrap_or_else(|e| panic!("{e}"));

    let (result, _) = update(&file, |_| Err(AdminFailure::AccountExists(name.clone())));
    assert_eq!(result, Err(AdminFailure::AccountExists(name.clone())));
    assert_eq!(contents(), original);
    assert_eq!(entries(&dir), ["admin-accounts.toml"]);

    // Another command holds the lock: no wait, no change, its lock stays.
    let lock = dir.join("admin-accounts.toml.lock");
    std::fs::write(&lock, "").unwrap_or_else(|e| panic!("write: {e}"));
    let (result, _) = update(&file, |_| panic!("changed under someone else's lock"));
    match &result {
        Err(AdminFailure::Accounts(d)) => {
            assert_eq!(d[0].key, "admin.accounts_file");
            assert!(
                d[0].message.contains(&lock.display().to_string()),
                "{}",
                d[0].message
            );
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(failure_codes(result), ["accounts_file_locked"]);
    assert_eq!(contents(), original);
    assert!(lock.exists());
    std::fs::remove_file(&lock).unwrap_or_else(|e| panic!("remove: {e}"));

    // The temporary file cannot be made: the stranger at its path stays too.
    let stranger = dir.join(format!("admin-accounts.toml.{}.tmp", std::process::id()));
    std::fs::write(&stranger, "x").unwrap_or_else(|e| panic!("write: {e}"));
    let (result, _) = update(&file, Ok);
    assert_eq!(failure_codes(result), ["accounts_file_write_failed"]);
    assert_eq!(contents(), original);
    assert_eq!(std::fs::read_to_string(&stranger).unwrap_or_default(), "x");
    std::fs::remove_file(&stranger).unwrap_or_else(|e| panic!("remove: {e}"));
    assert_eq!(entries(&dir), ["admin-accounts.toml"]);

    // An invalid file is not rewritten.
    std::fs::write(&path, "version = 2\n").unwrap_or_else(|e| panic!("write: {e}"));
    let (result, _) = update(&file, |_| panic!("changed an invalid file"));
    assert_eq!(failure_codes(result), ["accounts_file_invalid"]);
    assert_eq!(contents(), "version = 2\n");
    assert_eq!(entries(&dir), ["admin-accounts.toml"]);
}

#[test]
fn fails_to_write_when_the_directory_does_not_exist() {
    let dir = temp_dir("nodir");
    let file = AccountsFile::new(dir.join("missing").join("admin-accounts.toml"));
    let (result, _) = update(&file, |_| panic!("changed without a lock"));
    assert_eq!(failure_codes(result), ["accounts_file_write_failed"]);
    assert_eq!(entries(&dir), Vec::<String>::new());
}

#[test]
fn admin_reads_hashes_below_the_minimum() {
    let dir = temp_dir("floor");
    let path = dir.join("admin-accounts.toml");
    let text = format!("version = 1\n{}", account("alice", &phc("m=8192,t=1,p=1")));
    std::fs::write(&path, &text).unwrap_or_else(|e| panic!("write: {e}"));
    let file = AccountsFile::new(path.clone());
    let loaded = file
        .read()
        .unwrap_or_else(|d| panic!("{d:#?}"))
        .unwrap_or_else(|| panic!("file missing"));
    assert_eq!(loaded.accounts.len(), 1);
    // Rewriting keeps the old hash until `admin passwd` replaces it.
    let (result, _) = update(&file, Ok);
    assert_eq!(result, Ok(()));
    assert_eq!(
        load(&path, HashFloor::Reject)
            .map(|_| ())
            .map_err(|d| d[0].error.code()),
        Err("accounts_hash_below_minimum")
    );
}

#[test]
fn admin_refuses_an_above_maximum_parameter_even_with_a_below_minimum_one() {
    let key = "account[0].password_hash";
    // `serve` reports the minimum first. `admin` still refuses the maximum,
    // so `admin passwd` never keeps a hash that is outside the range.
    let text = format!(
        "version = 1\n{}",
        account("alice", &phc("m=19455,t=11,p=1"))
    );
    assert_eq!(
        problems(&text),
        [problem("accounts_hash_below_minimum", key)]
    );
    assert_eq!(
        parse(&text, HashFloor::Accept)
            .map(|a| a.len())
            .map_err(|d| (d[0].error.code(), d[0].key.clone())),
        Err(("accounts_file_invalid", key.to_owned()))
    );
}
