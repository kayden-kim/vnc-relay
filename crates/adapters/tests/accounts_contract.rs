//! Contract tests of the account file reader (docs/contracts/cli-v1.md §1.3,
//! §1.4, §1.6; recorded in §5).

use std::path::PathBuf;

use vnc_relay_adapters::accounts_file::{load, parse};

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
    match parse(text) {
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
    match parse(text) {
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
    }
    for params in ["m=1048577,t=2,p=1", "m=19456,t=11,p=1", "m=19456,t=2,p=17"] {
        let text = format!("version = 1\n{}", account("alice", &phc(params)));
        assert_eq!(
            problems(&text),
            [problem("accounts_file_invalid", key)],
            "{params}"
        );
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
    let file_error = |path: &std::path::Path| match load(path) {
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
    use vnc_relay_app::accounts::AccountsWarning;

    let dir = temp_dir("modes");
    let path = dir.join("admin-accounts.toml");
    let text = format!("version = 1\n{}", account("alice", &phc("m=19456,t=2,p=1")));
    std::fs::write(&path, text).unwrap_or_else(|error| panic!("write: {error}"));
    let warnings = |mode: u32| {
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode))
            .unwrap_or_else(|error| panic!("chmod: {error}"));
        match load(&path) {
            Ok(loaded) => loaded.warnings,
            Err(diagnostics) => panic!("unexpected diagnostics: {diagnostics:#?}"),
        }
    };
    assert_eq!(warnings(0o600), []);
    assert_eq!(warnings(0o400), []);
    assert_eq!(warnings(0o640), [AccountsWarning::FilePermissive]);
    assert_eq!(warnings(0o604), [AccountsWarning::FilePermissive]);
}
