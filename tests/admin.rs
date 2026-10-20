//! System tests of `vnc-relay admin add|passwd|remove|list`: the exit codes,
//! diagnostic lines, account file changes, and prompts of
//! docs/contracts/cli-v1.md §1 and §2 and docs/architectures/admin-accounts.md
//! §3 and §6. The terminal tests run the command under `script` (Linux only),
//! so the hidden prompts and Ctrl+C work as they do in a shell.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[cfg(target_os = "linux")]
use std::io::{Read, Write};
#[cfg(target_os = "linux")]
use std::process::{Child, ChildStdin};
#[cfg(target_os = "linux")]
use std::sync::{Arc, Mutex, PoisonError};
#[cfg(target_os = "linux")]
use std::thread::{self, JoinHandle};
#[cfg(target_os = "linux")]
use std::time::{Duration, Instant};

use vnc_relay_adapters::accounts_file::{self, HashFloor};

const BIN: &str = env!("CARGO_BIN_EXE_vnc-relay");

/// 16 and 32 zero bytes in unpadded Base64, as in tests/serve.rs. Test data,
/// not a real hash.
const SALT: &str = "AAAAAAAAAAAAAAAAAAAAAA";
const HASH: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

/// The default `admin.argon2` parameters.
const DEFAULT_PARAMS: &str = "m=19456,t=2,p=1";
const OTHER_PARAMS: &str = "m=19456,t=3,p=1";

const NOTICE: &str =
    "notice: 실행 중인 서버에는 반영되지 않는다. 서버를 정상 종료하고 다시 시작해야 한다.";

#[cfg(target_os = "linux")]
const PTY_TIMEOUT: Duration = Duration::from_secs(30);
#[cfg(target_os = "linux")]
const POLL: Duration = Duration::from_millis(20);
/// Wait before typed input, so that the command has switched the terminal
/// mode after its prompt appeared.
#[cfg(target_os = "linux")]
const SETTLE: Duration = Duration::from_millis(200);

/// An Argon2id PHC string with `params` and the test salt and hash.
fn phc(params: &str) -> String {
    format!("$argon2id$v=19${params}${SALT}${HASH}")
}

/// A fresh directory with a config file that points at `accounts.toml`.
struct Fixture {
    dir: PathBuf,
    config: PathBuf,
    accounts: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let dir = temp_dir(name);
        let config = dir.join("vnc-relay.toml");
        let accounts = dir.join("accounts.toml");
        write(&config, "[admin]\naccounts_file = \"accounts.toml\"\n");
        Self {
            dir,
            config,
            accounts,
        }
    }

    /// Writes the account file and returns `self`.
    fn with_accounts(self, text: &str) -> Self {
        write(&self.accounts, text);
        self
    }

    fn lock_path(&self) -> PathBuf {
        self.dir.join("accounts.toml.lock")
    }

    fn bytes(&self) -> Vec<u8> {
        fs::read(&self.accounts)
            .unwrap_or_else(|error| panic!("read {}: {error}", self.accounts.display()))
    }
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vnc-relay-admin-{}-{name}", std::process::id()));
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap_or_else(|error| panic!("clear {}: {error}", dir.display()));
    }
    fs::create_dir_all(&dir).unwrap_or_else(|error| panic!("create {}: {error}", dir.display()));
    dir
}

fn write(path: &Path, text: &str) {
    fs::write(path, text).unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .unwrap_or_else(|error| panic!("chmod {}: {error}", path.display()));
    }
}

/// The text of an account file with one `[[account]]` per entry, in order.
fn accounts_text(entries: &[(&str, String)]) -> String {
    let mut text = String::from("version = 1\n");
    for (name, hash) in entries {
        text.push_str(&format!(
            "\n[[account]]\nname = \"{name}\"\npassword_hash = \"{hash}\"\n"
        ));
    }
    text
}

/// `(name, hash)` of each account in the file, in file order.
fn loaded(path: &Path) -> Vec<(String, String)> {
    accounts_file::load(path, HashFloor::Reject)
        .unwrap_or_else(|errors| panic!("load {}: {} problem(s)", path.display(), errors.len()))
        .accounts
        .iter()
        .map(|account| {
            (
                account.name.as_str().to_owned(),
                account.password_hash.as_str().to_owned(),
            )
        })
        .collect()
}

struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Run {
    /// `(code, key)` of each `error[..]` line; an absent key is empty.
    fn errors(&self) -> Vec<(String, String)> {
        error_pairs(&self.stderr)
    }

    fn assert_no_secret(&self) {
        for text in [&self.stdout, &self.stderr] {
            assert!(
                !text.contains(HASH) && !text.contains(SALT),
                "secret in output: {text}"
            );
        }
    }
}

/// `(code, key)` of each `error[..]` line in `text`; an absent key is empty.
fn error_pairs(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| line.find("error[").map(|at| &line[at + "error[".len()..]))
        .map(|rest| {
            let (code, rest) = rest.split_once(']').unwrap_or((rest, ""));
            let key = rest
                .strip_prefix(" key=")
                .and_then(|rest| rest.split_once(':'))
                .map_or("", |(key, _)| key);
            (code.to_owned(), key.to_owned())
        })
        .collect()
}

fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
    items
        .iter()
        .map(|(code, key)| ((*code).to_owned(), (*key).to_owned()))
        .collect()
}

/// Runs `vnc-relay admin <args> --config <config>` with a stdin that is not a
/// terminal.
fn admin(args: &[&str], config: &Path) -> Run {
    let output = Command::new(BIN)
        .arg("admin")
        .args(args)
        .arg("--config")
        .arg(config)
        .stdin(Stdio::null())
        .output()
        .unwrap_or_else(|error| panic!("run admin: {error}"));
    Run {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// `list` shows names in name order, one per line, and the file's hashes
/// below the serve minimum do not stop it.
/// spec: operations.md:OPS-029 partial
#[test]
fn list_prints_account_names_in_name_order() {
    let fixture = Fixture::new("list-order").with_accounts(&accounts_text(&[
        ("zed", phc(DEFAULT_PARAMS)),
        ("alice", phc(DEFAULT_PARAMS)),
        ("bob", phc(OTHER_PARAMS)),
    ]));
    let run = admin(&["list"], &fixture.config);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(run.stdout, "alice\nbob\nzed\n");
    assert!(run.stderr.is_empty(), "{}", run.stderr);
    run.assert_no_secret();
}

/// spec: operations.md:OPS-029 partial
#[test]
fn list_without_an_account_file_prints_a_notice_and_exits_0() {
    let fixture = Fixture::new("list-missing");
    let run = admin(&["list"], &fixture.config);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.is_empty(), "{}", run.stdout);
    assert!(
        run.stderr.contains("계정 파일이 없어 계정이 없다: "),
        "{}",
        run.stderr
    );
    assert!(run.errors().is_empty(), "{}", run.stderr);
}

/// spec: operations.md:OPS-029 partial
#[test]
fn list_accepts_a_hash_below_the_serve_minimum() {
    let fixture = Fixture::new("list-weak")
        .with_accounts(&accounts_text(&[("alice", phc("m=1024,t=1,p=1"))]));
    let run = admin(&["list"], &fixture.config);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(run.stdout, "alice\n");
    assert!(run.stderr.is_empty(), "{}", run.stderr);
}

/// Ubuntu: a group or other permission on the account file is a warning only.
/// spec: operations.md:OPS-029 partial
#[cfg(unix)]
#[test]
fn list_warns_when_others_can_read_the_account_file() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new("list-permissive")
        .with_accounts(&accounts_text(&[("alice", phc(DEFAULT_PARAMS))]));
    fs::set_permissions(&fixture.accounts, fs::Permissions::from_mode(0o644))
        .unwrap_or_else(|error| panic!("chmod: {error}"));
    let run = admin(&["list"], &fixture.config);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(run.stdout, "alice\n");
    assert!(
        run.stderr
            .contains("warning[accounts_file_permissive] key=admin.accounts_file: "),
        "{}",
        run.stderr
    );
}

/// `remove --yes` deletes the named account and leaves the others' hashes as
/// they were.
/// spec: operations.md:OPS-029 partial
#[test]
fn remove_with_yes_deletes_one_account_and_keeps_the_others() {
    let fixture = Fixture::new("remove-yes").with_accounts(&accounts_text(&[
        ("alice", phc(DEFAULT_PARAMS)),
        ("bob", phc(OTHER_PARAMS)),
    ]));
    let run = admin(&["remove", "alice", "--yes"], &fixture.config);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(run.stdout, "계정 'alice'을(를) 삭제했다.\n");
    assert!(run.stderr.contains(NOTICE), "{}", run.stderr);
    run.assert_no_secret();
    assert_eq!(
        loaded(&fixture.accounts),
        vec![("bob".to_owned(), phc(OTHER_PARAMS))]
    );
}

/// Removing the last account is allowed, with a warning.
/// spec: operations.md:OPS-029 partial
#[test]
fn removing_the_last_account_warns_and_list_shows_none() {
    let fixture = Fixture::new("remove-last")
        .with_accounts(&accounts_text(&[("alice", phc(DEFAULT_PARAMS))]));
    let run = admin(&["remove", "alice", "--yes"], &fixture.config);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(
        run.stderr
            .contains("warning[last_account_removed] key=admin.accounts_file: "),
        "{}",
        run.stderr
    );
    assert!(loaded(&fixture.accounts).is_empty());

    let list = admin(&["list"], &fixture.config);
    assert_eq!(list.code, Some(0), "{}", list.stderr);
    assert!(list.stdout.is_empty(), "{}", list.stdout);
}

/// Without a terminal, every command that needs input exits 2 before it asks
/// for anything, and it does not print the usage text.
/// spec: operations.md:OPS-029 partial
#[test]
fn account_commands_without_a_terminal_exit_2_without_usage_text() {
    let fixture = Fixture::new("not-interactive")
        .with_accounts(&accounts_text(&[("alice", phc(DEFAULT_PARAMS))]));
    let original = fixture.bytes();
    let commands: [&[&str]; 3] = [&["add", "bob"], &["passwd", "alice"], &["remove", "alice"]];
    for args in commands {
        let run = admin(args, &fixture.config);
        assert_eq!(run.code, Some(2), "{args:?}: {}", run.stderr);
        assert_eq!(run.errors(), pairs(&[("not_interactive", "")]), "{args:?}");
        assert!(
            !run.stderr.contains("사용법"),
            "usage text for a terminal error:\n{}",
            run.stderr
        );
        assert!(run.stdout.is_empty(), "{args:?}: {}", run.stdout);
        run.assert_no_secret();
    }
    assert_eq!(fixture.bytes(), original);
    assert!(!fixture.lock_path().exists());
}

/// `add` checks the name before it asks for a password; `passwd` and `remove`
/// check that the name exists before they ask. Stdin is not a terminal here,
/// so a prompt that ran first would show `not_interactive` instead.
/// spec: operations.md:OPS-029 partial
#[test]
fn existing_and_unknown_names_fail_before_any_password_prompt() {
    let fixture = Fixture::new("names-check")
        .with_accounts(&accounts_text(&[("alice", phc(DEFAULT_PARAMS))]));
    let original = fixture.bytes();

    let run = admin(&["add", "alice"], &fixture.config);
    assert_eq!(run.code, Some(1), "{}", run.stderr);
    assert_eq!(run.errors(), pairs(&[("account_exists", "")]));

    let run = admin(&["passwd", "bob"], &fixture.config);
    assert_eq!(run.code, Some(1), "{}", run.stderr);
    assert_eq!(run.errors(), pairs(&[("account_not_found", "")]));

    let run = admin(&["remove", "bob"], &fixture.config);
    assert_eq!(run.code, Some(1), "{}", run.stderr);
    assert_eq!(run.errors(), pairs(&[("account_not_found", "")]));

    assert_eq!(fixture.bytes(), original);
}

/// Names that break the rule exit 2 with `account_name_invalid` and no usage
/// text, and no account file is created.
/// spec: operations.md:OPS-030 partial
#[test]
fn invalid_account_names_exit_2_and_create_no_file() {
    let fixture = Fixture::new("names-invalid");
    let long = "a".repeat(33);
    for name in [
        "",
        "Alice",
        "with space",
        "slash/name",
        "가나다",
        long.as_str(),
    ] {
        let run = admin(&["add", name], &fixture.config);
        assert_eq!(run.code, Some(2), "{name:?}: {}", run.stderr);
        assert_eq!(
            run.errors(),
            pairs(&[("account_name_invalid", "")]),
            "{name:?}"
        );
        assert!(
            !run.stderr.contains("사용법"),
            "usage text for a name error:\n{}",
            run.stderr
        );
        assert!(run.stdout.is_empty(), "{name:?}: {}", run.stdout);
    }
    assert!(!fixture.accounts.exists());
}

/// Config problems exit 3 with the `config_*` code and the key the contract
/// names.
/// spec: operations.md:OPS-030 partial
#[test]
fn config_problems_exit_3_with_their_codes() {
    let dir = temp_dir("config-errors");
    let missing = dir.join("missing.toml");
    let syntax = dir.join("syntax.toml");
    write(&syntax, "[admin\naccounts_file = ");
    let no_key = dir.join("no-key.toml");
    write(&no_key, "[admin]\n");
    let cases: [(&Path, &str, &str); 3] = [
        (&missing, "config_file_unreadable", ""),
        (&syntax, "config_syntax_error", ""),
        (&no_key, "config_missing_key", "admin.accounts_file"),
    ];
    for (config, code, key) in cases {
        let run = admin(&["list"], config);
        assert_eq!(run.code, Some(3), "{}: {}", config.display(), run.stderr);
        assert_eq!(run.errors(), pairs(&[(code, key)]), "{}", config.display());
        assert!(run.stdout.is_empty(), "{}", run.stdout);
    }
}

/// Only the TOML syntax and the `[admin]` table are checked; other tables with
/// bad values do not stop the command.
/// spec: operations.md:OPS-029 partial
#[test]
fn tables_other_than_admin_are_not_checked() {
    let fixture = Fixture::new("other-tables")
        .with_accounts(&accounts_text(&[("alice", phc(DEFAULT_PARAMS))]));
    write(
        &fixture.config,
        "[admin]\naccounts_file = \"accounts.toml\"\n\n[management]\nlisten = \"not an address\"\n\n[relay.tcp_viewer]\nlisten = 42\n",
    );
    let run = admin(&["list"], &fixture.config);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(run.stdout, "alice\n");
    assert!(run.errors().is_empty(), "{}", run.stderr);
}

/// Each broken account file exits 5 with the file's key, prints no hash, and
/// stays as it was.
/// spec: operations.md:OPS-030 partial
#[test]
fn broken_account_files_exit_5_and_are_not_changed() {
    let algorithm = format!(
        "version = 1\n\n[[account]]\nname = \"alice\"\npassword_hash = \"$argon2i$v=19${DEFAULT_PARAMS}${SALT}${HASH}\"\n"
    );
    let cases: [(&str, String, &str, &str); 4] = [
        (
            "syntax",
            "version = 1\n[[account\n".to_owned(),
            "accounts_file_invalid",
            "",
        ),
        (
            "version",
            "version = 2\n".to_owned(),
            "accounts_file_invalid",
            "version",
        ),
        (
            "name",
            accounts_text(&[("Alice", phc(DEFAULT_PARAMS))]),
            "accounts_file_invalid",
            "account[0].name",
        ),
        (
            "algorithm",
            algorithm,
            "accounts_file_invalid",
            "account[0].password_hash",
        ),
    ];
    for (case, text, code, key) in cases {
        let fixture = Fixture::new(&format!("broken-{case}")).with_accounts(&text);
        let original = fixture.bytes();
        let run = admin(&["list"], &fixture.config);
        assert_eq!(run.code, Some(5), "{case}: {}", run.stderr);
        assert_eq!(run.errors(), pairs(&[(code, key)]), "{case}");
        assert!(run.stdout.is_empty(), "{case}: {}", run.stdout);
        run.assert_no_secret();
        assert_eq!(fixture.bytes(), original, "{case}");
    }
}

/// An account path that is a directory is an unreadable file.
/// spec: operations.md:OPS-030 partial
#[test]
fn an_account_path_that_is_a_directory_exits_5() {
    let fixture = Fixture::new("dir-accounts");
    fs::create_dir(&fixture.accounts).unwrap_or_else(|error| panic!("mkdir: {error}"));
    let run = admin(&["list"], &fixture.config);
    assert_eq!(run.code, Some(5), "{}", run.stderr);
    assert_eq!(
        run.errors(),
        pairs(&[("accounts_file_unreadable", "admin.accounts_file")])
    );
    assert!(run.stdout.is_empty(), "{}", run.stdout);
}

/// A lock file from another `admin` command stops the change. The command
/// does not remove the lock file, because it did not create it.
/// spec: operations.md:OPS-030 partial
#[test]
fn a_lock_file_stops_account_changes_and_is_left_in_place() {
    let fixture =
        Fixture::new("lock").with_accounts(&accounts_text(&[("alice", phc(DEFAULT_PARAMS))]));
    write(&fixture.lock_path(), "");
    let original = fixture.bytes();
    let run = admin(&["remove", "alice", "--yes"], &fixture.config);
    assert_eq!(run.code, Some(5), "{}", run.stderr);
    assert_eq!(
        run.errors(),
        pairs(&[("accounts_file_locked", "admin.accounts_file")])
    );
    assert!(
        run.stderr
            .contains(&fixture.lock_path().display().to_string()),
        "{}",
        run.stderr
    );
    assert_eq!(fixture.bytes(), original);
    assert!(fixture.lock_path().exists());
}

/// Makes a directory read-only until dropped. The drop also runs on a panic,
/// so a failing assert cannot leave the test directory unwritable.
#[cfg(unix)]
struct DirLocked(PathBuf);

#[cfg(unix)]
impl DirLocked {
    fn new(dir: &Path) -> Self {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o555))
            .unwrap_or_else(|error| panic!("chmod {}: {error}", dir.display()));
        Self(dir.to_path_buf())
    }
}

#[cfg(unix)]
impl Drop for DirLocked {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o755));
    }
}

/// Makes a file read-only until dropped, as `DirLocked` does for a directory.
#[cfg(windows)]
struct ReadOnlyFile(PathBuf);

#[cfg(windows)]
impl ReadOnlyFile {
    fn new(path: &Path) -> Self {
        let mut permissions = fs::metadata(path)
            .unwrap_or_else(|error| panic!("stat {}: {error}", path.display()))
            .permissions();
        permissions.set_readonly(true);
        fs::set_permissions(path, permissions)
            .unwrap_or_else(|error| panic!("set {} read-only: {error}", path.display()));
        Self(path.to_path_buf())
    }
}

#[cfg(windows)]
impl Drop for ReadOnlyFile {
    #[allow(
        clippy::permissions_set_readonly_false,
        reason = "clears only the read-only attribute of a test temp file on Windows"
    )]
    fn drop(&mut self) {
        if let Ok(mut permissions) = fs::metadata(&self.0).map(|meta| meta.permissions()) {
            permissions.set_readonly(false);
            let _ = fs::set_permissions(&self.0, permissions);
        }
    }
}

/// A save that cannot finish changes nothing: exit 5 with
/// `accounts_file_write_failed`, the account file is as it was, no lock or
/// temporary file is left, and no secret is printed. Unix makes the directory
/// read-only, so the lock file cannot be made. Windows makes the account file
/// read-only, so the lock and temporary file are made and the replacement is
/// refused.
/// spec: operations.md:OPS-030 partial
#[test]
fn a_failed_save_exits_5_and_keeps_the_account_file() {
    let fixture = Fixture::new("save-fail").with_accounts(&accounts_text(&[
        ("alice", phc(DEFAULT_PARAMS)),
        ("bob", phc(OTHER_PARAMS)),
    ]));
    let original = fixture.bytes();

    #[cfg(unix)]
    let blocked = DirLocked::new(&fixture.dir);
    #[cfg(windows)]
    let blocked = ReadOnlyFile::new(&fixture.accounts);
    let run = admin(&["remove", "bob", "--yes"], &fixture.config);
    drop(blocked);

    assert_eq!(run.code, Some(5), "{}", run.stderr);
    assert_eq!(
        run.errors(),
        pairs(&[("accounts_file_write_failed", "admin.accounts_file")]),
        "{}",
        run.stderr
    );
    #[cfg(windows)]
    assert!(
        run.stderr.contains("계정 파일을 교체할 수 없다"),
        "the replacement did not fail: {}",
        run.stderr
    );
    run.assert_no_secret();
    for text in [&run.stdout, &run.stderr] {
        assert!(!text.contains("$argon2id"), "{text}");
    }
    assert_eq!(fixture.bytes(), original);
    let mut names: Vec<String> = fs::read_dir(&fixture.dir)
        .unwrap_or_else(|error| panic!("read {}: {error}", fixture.dir.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|error| panic!("read entry: {error}"))
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    assert_eq!(names, ["accounts.toml", "vnc-relay.toml"], "left behind");
}

/// A terminal session for one `vnc-relay admin` command, run under `script` so
/// the child has a pseudo-terminal. Output is collected on a thread.
#[cfg(target_os = "linux")]
struct PtySession {
    child: Child,
    stdin: Option<ChildStdin>,
    transcript: Arc<Mutex<Vec<u8>>>,
    reader: Option<JoinHandle<()>>,
}

#[cfg(target_os = "linux")]
impl PtySession {
    fn start(command: &str) -> Self {
        let mut child = Command::new("script")
            .args(["-q", "-E", "never", "-c", command, "/dev/null"])
            .env("SHELL", "/bin/sh")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|error| panic!("start script: {error}"));
        let stdin = child.stdin.take();
        let mut stdout = child
            .stdout
            .take()
            .unwrap_or_else(|| panic!("script has no stdout"));
        let transcript = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&transcript);
        let reader = thread::spawn(move || {
            let mut chunk = [0_u8; 4096];
            while let Ok(n) = stdout.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                sink.lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .extend_from_slice(&chunk[..n]);
            }
        });
        Self {
            child,
            stdin,
            transcript,
            reader: Some(reader),
        }
    }

    /// The transcript so far, with CRLF line ends as LF.
    fn text(&self) -> String {
        let bytes = self
            .transcript
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        String::from_utf8_lossy(&bytes).replace("\r\n", "\n")
    }

    /// Waits until the transcript contains `marker`.
    fn wait_for(&self, marker: &str) {
        let deadline = Instant::now() + PTY_TIMEOUT;
        loop {
            let text = self.text();
            if text.contains(marker) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "no {marker:?} in the transcript:\n{text}"
            );
            thread::sleep(POLL);
        }
    }

    fn send(&mut self, input: &str) {
        thread::sleep(SETTLE);
        let stdin = self
            .stdin
            .as_mut()
            .unwrap_or_else(|| panic!("input is closed"));
        stdin
            .write_all(input.as_bytes())
            .unwrap_or_else(|error| panic!("send input: {error}"));
        stdin
            .flush()
            .unwrap_or_else(|error| panic!("flush input: {error}"));
    }

    /// Waits for `end` in the transcript, closes the input so `script` ends,
    /// and returns the whole transcript.
    fn finish(mut self, end: &str) -> String {
        self.wait_for(end);
        self.stdin = None;
        let deadline = Instant::now() + PTY_TIMEOUT;
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) => {
                    assert!(
                        Instant::now() < deadline,
                        "script did not exit:\n{}",
                        self.text()
                    );
                    thread::sleep(POLL);
                }
                Err(error) => panic!("wait for script: {error}"),
            }
        }
        if let Some(reader) = self.reader.take() {
            reader
                .join()
                .unwrap_or_else(|_| panic!("transcript reader panicked"));
        }
        self.text()
    }
}

#[cfg(target_os = "linux")]
impl Drop for PtySession {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A shell line that runs `vnc-relay admin <args>` and prints its exit
/// status as `__RC=<n>`. `trap : INT` keeps the shell alive when a terminal
/// Ctrl+C sends SIGINT to the foreground group, so the status still prints.
#[cfg(target_os = "linux")]
fn pty_command(args: &str, config: &Path) -> String {
    format!(
        "trap : INT; '{BIN}' admin {args} --config '{}'; echo __RC=$?",
        config.display()
    )
}

/// `pty_command` between two `stty -g` dumps, so a test can compare the
/// terminal settings before and after. `__END` marks the second dump.
#[cfg(target_os = "linux")]
fn pty_command_with_modes(args: &str, config: &Path, before: &Path, after: &Path) -> String {
    format!(
        "stty -g > '{}'; {}; stty -g > '{}'; echo __END",
        before.display(),
        pty_command(args, config),
        after.display()
    )
}

/// `add` twice on a terminal: the two entries are hidden, each account gets an
/// Argon2id hash with its own salt, and the typed text is not in the output.
/// spec: operations.md:OPS-029 partial
#[cfg(target_os = "linux")]
#[test]
fn add_saves_two_accounts_with_argon2id_hashes_from_hidden_input() {
    let fixture = Fixture::new("pty-add");
    for (name, secret) in [
        ("alice", "correct horse battery"),
        ("bob", "second horse battery"),
    ] {
        let mut session = PtySession::start(&pty_command(&format!("add {name}"), &fixture.config));
        session.wait_for("비밀번호: ");
        session.send(&format!("{secret}\n"));
        session.wait_for("비밀번호 확인: ");
        session.send(&format!("{secret}\n"));
        let transcript = session.finish("__RC=");
        assert!(
            transcript.contains(&format!("계정 '{name}'을(를) 추가했다.\n")),
            "{transcript}"
        );
        assert!(transcript.contains(NOTICE), "{transcript}");
        assert!(transcript.contains("__RC=0"), "{transcript}");
        assert!(
            !transcript.contains(secret),
            "typed password in the output:\n{transcript}"
        );
    }

    let accounts = loaded(&fixture.accounts);
    let names: Vec<&str> = accounts.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["alice", "bob"]);
    let prefix = format!("$argon2id$v=19${DEFAULT_PARAMS}$");
    for (name, hash) in &accounts {
        assert!(hash.starts_with(&prefix), "{name}: {hash}");
    }
    assert_ne!(accounts[0].1, accounts[1].1, "two accounts share one hash");
}

/// `passwd` replaces only the named account's hash.
/// spec: operations.md:OPS-029 partial
#[cfg(target_os = "linux")]
#[test]
fn passwd_replaces_only_the_named_hash() {
    let fixture = Fixture::new("pty-passwd").with_accounts(&accounts_text(&[
        ("alice", phc(DEFAULT_PARAMS)),
        ("bob", phc(OTHER_PARAMS)),
    ]));
    let mut session = PtySession::start(&pty_command("passwd alice", &fixture.config));
    session.wait_for("새 비밀번호: ");
    session.send("replacement horse battery\n");
    session.wait_for("새 비밀번호 확인: ");
    session.send("replacement horse battery\n");
    let transcript = session.finish("__RC=");
    assert!(
        transcript.contains("계정 'alice'의 비밀번호를 바꿨다.\n"),
        "{transcript}"
    );
    assert!(transcript.contains("__RC=0"), "{transcript}");
    assert!(
        !transcript.contains("replacement horse battery"),
        "typed password in the output:\n{transcript}"
    );

    let accounts = loaded(&fixture.accounts);
    assert_eq!(accounts.len(), 2);
    let (alice, alice_hash) = &accounts[0];
    assert_eq!(alice, "alice");
    assert_ne!(alice_hash, &phc(DEFAULT_PARAMS));
    assert!(alice_hash.starts_with(&format!("$argon2id$v=19${DEFAULT_PARAMS}$")));
    assert_eq!(accounts[1], ("bob".to_owned(), phc(OTHER_PARAMS)));
}

/// Password entries that break the rule exit 1 and leave the file as it was:
/// a short password stops at the first entry, a mismatch stops after the
/// second, and Ctrl+D at an empty entry is end of input.
/// spec: operations.md:OPS-030 partial
#[cfg(target_os = "linux")]
#[test]
fn rejected_password_entries_exit_1_and_keep_the_account_file() {
    let fixture =
        Fixture::new("pty-reject").with_accounts(&accounts_text(&[("alice", phc(DEFAULT_PARAMS))]));
    let original = fixture.bytes();

    let mut session = PtySession::start(&pty_command("add bob", &fixture.config));
    session.wait_for("비밀번호: ");
    session.send("too short\n");
    let transcript = session.finish("__RC=");
    assert_eq!(
        error_pairs(&transcript),
        pairs(&[("password_invalid", "")]),
        "{transcript}"
    );
    assert!(!transcript.contains("비밀번호 확인: "), "{transcript}");
    assert!(transcript.contains("__RC=1"), "{transcript}");
    assert_eq!(fixture.bytes(), original);

    let mut session = PtySession::start(&pty_command("add bob", &fixture.config));
    session.wait_for("비밀번호: ");
    session.send("correct horse battery\n");
    session.wait_for("비밀번호 확인: ");
    session.send("correct horse batterz\n");
    let transcript = session.finish("__RC=");
    assert_eq!(
        error_pairs(&transcript),
        pairs(&[("password_mismatch", "")]),
        "{transcript}"
    );
    assert!(transcript.contains("__RC=1"), "{transcript}");
    assert_eq!(fixture.bytes(), original);

    let mut session = PtySession::start(&pty_command("add bob", &fixture.config));
    session.wait_for("비밀번호: ");
    session.send("\u{4}");
    let transcript = session.finish("__RC=");
    assert_eq!(
        error_pairs(&transcript),
        pairs(&[("input_canceled", "")]),
        "{transcript}"
    );
    assert!(transcript.contains("__RC=1"), "{transcript}");
    assert_eq!(fixture.bytes(), original);
}

/// Ctrl+C at the hidden password prompt cancels, leaves the file as it was,
/// and restores the terminal settings.
/// spec: operations.md:OPS-030 partial
#[cfg(target_os = "linux")]
#[test]
fn ctrl_c_at_the_password_prompt_cancels_and_restores_the_terminal() {
    let fixture =
        Fixture::new("pty-ctrl-c").with_accounts(&accounts_text(&[("alice", phc(DEFAULT_PARAMS))]));
    let original = fixture.bytes();
    let before = fixture.dir.join("mode-before");
    let after = fixture.dir.join("mode-after");
    let mut session = PtySession::start(&pty_command_with_modes(
        "passwd alice",
        &fixture.config,
        &before,
        &after,
    ));
    session.wait_for("새 비밀번호: ");
    session.send("\u{3}");
    let transcript = session.finish("__END");
    assert_eq!(
        error_pairs(&transcript),
        pairs(&[("input_canceled", "")]),
        "{transcript}"
    );
    assert!(transcript.contains("__RC=1"), "{transcript}");

    let before_mode = fs::read_to_string(&before)
        .unwrap_or_else(|error| panic!("read {}: {error}", before.display()));
    let after_mode = fs::read_to_string(&after)
        .unwrap_or_else(|error| panic!("read {}: {error}", after.display()));
    assert!(!before_mode.trim().is_empty(), "no terminal settings read");
    assert_eq!(
        after_mode, before_mode,
        "terminal settings were not restored"
    );
    assert_eq!(fixture.bytes(), original);
}

/// The removal confirmation accepts `y` or `yes` in any case. Any other line
/// cancels, and so does Ctrl+C at the prompt. Only the accepted removal
/// changes the file.
/// spec: operations.md:OPS-030 partial
#[cfg(target_os = "linux")]
#[test]
fn remove_confirmation_accepts_only_yes_and_ctrl_c_cancels() {
    let fixture = Fixture::new("pty-remove").with_accounts(&accounts_text(&[
        ("alice", phc(DEFAULT_PARAMS)),
        ("bob", phc(OTHER_PARAMS)),
    ]));
    let original = fixture.bytes();
    let prompt = "계정 'alice'을(를) 삭제할까요? [y/N] ";

    let mut session = PtySession::start(&pty_command("remove alice", &fixture.config));
    session.wait_for(prompt);
    session.send("n\n");
    let transcript = session.finish("__RC=");
    assert_eq!(
        error_pairs(&transcript),
        pairs(&[("input_canceled", "")]),
        "{transcript}"
    );
    assert!(transcript.contains("__RC=1"), "{transcript}");
    assert_eq!(fixture.bytes(), original);

    let mut session = PtySession::start(&pty_command("remove alice", &fixture.config));
    session.wait_for(prompt);
    session.send("\u{3}");
    let transcript = session.finish("__RC=");
    assert_eq!(
        error_pairs(&transcript),
        pairs(&[("input_canceled", "")]),
        "{transcript}"
    );
    assert!(transcript.contains("__RC=1"), "{transcript}");
    assert_eq!(fixture.bytes(), original);

    let mut session = PtySession::start(&pty_command("remove alice", &fixture.config));
    session.wait_for(prompt);
    session.send("YES\n");
    let transcript = session.finish("__RC=");
    assert!(
        transcript.contains("계정 'alice'을(를) 삭제했다.\n"),
        "{transcript}"
    );
    assert!(transcript.contains("__RC=0"), "{transcript}");
    let names: Vec<String> = loaded(&fixture.accounts)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(names, ["bob"]);
}

/// Asserts that a terminal transcript shows none of `typed`, the Argon2id
/// marker, or the test salt and hash.
#[cfg(target_os = "linux")]
fn assert_no_pty_secret(transcript: &str, typed: &[&str]) {
    for secret in typed.iter().copied().chain(["$argon2id", SALT, HASH]) {
        assert!(
            !transcript.contains(secret),
            "secret in output: {transcript}"
        );
    }
}

/// Four failed preparations, each on its own run: Ctrl+C at the first prompt
/// (OPS-030 input canceled), a short password (invalid input), two different
/// passwords (invalid input at the confirmation), and a read-only directory
/// with a valid password entered twice (save failure). Each run keeps the
/// account file and prints no typed password, Argon2id marker, salt, or hash.
/// spec: operations.md:OPS-030
#[cfg(target_os = "linux")]
#[test]
fn every_failed_preparation_keeps_the_account_file_and_prints_no_secret() {
    let fixture = Fixture::new("pty-every-failure").with_accounts(&accounts_text(&[
        ("alice", phc(DEFAULT_PARAMS)),
        ("bob", phc(OTHER_PARAMS)),
    ]));
    let original = fixture.bytes();
    let typed = [
        "correct horse battery",
        "correct horse batterz",
        "too short",
    ];
    let new_prompt = "새 비밀번호: ";
    let confirm_prompt = "비밀번호 확인: ";

    // (a) Ctrl+C at the first prompt cancels the input.
    let mut session = PtySession::start(&pty_command("passwd alice", &fixture.config));
    session.wait_for(new_prompt);
    session.send("\u{3}");
    let transcript = session.finish("__RC=");
    assert_eq!(
        error_pairs(&transcript),
        pairs(&[("input_canceled", "")]),
        "{transcript}"
    );
    assert!(transcript.contains("__RC=1"), "{transcript}");
    assert_no_pty_secret(&transcript, &typed);
    assert_eq!(fixture.bytes(), original);

    // (b) A password under the rule stops at the first entry.
    let mut session = PtySession::start(&pty_command("passwd alice", &fixture.config));
    session.wait_for(new_prompt);
    session.send("too short\n");
    let transcript = session.finish("__RC=");
    assert_eq!(
        error_pairs(&transcript),
        pairs(&[("password_invalid", "")]),
        "{transcript}"
    );
    assert!(transcript.contains("__RC=1"), "{transcript}");
    assert_no_pty_secret(&transcript, &typed);
    assert_eq!(fixture.bytes(), original);

    // (c) Two different valid passwords stop at the confirmation.
    let mut session = PtySession::start(&pty_command("passwd alice", &fixture.config));
    session.wait_for(new_prompt);
    session.send("correct horse battery\n");
    session.wait_for(confirm_prompt);
    session.send("correct horse batterz\n");
    let transcript = session.finish("__RC=");
    assert_eq!(
        error_pairs(&transcript),
        pairs(&[("password_mismatch", "")]),
        "{transcript}"
    );
    assert!(transcript.contains("__RC=1"), "{transcript}");
    assert_no_pty_secret(&transcript, &typed);
    assert_eq!(fixture.bytes(), original);

    // (d) A valid password twice, with the directory read-only: the lock file
    // cannot be made, so the save fails with exit 5.
    let blocked = DirLocked::new(&fixture.dir);
    let mut session = PtySession::start(&pty_command("passwd alice", &fixture.config));
    session.wait_for(new_prompt);
    session.send("correct horse battery\n");
    session.wait_for(confirm_prompt);
    session.send("correct horse battery\n");
    let transcript = session.finish("__RC=");
    drop(blocked);
    assert_eq!(
        error_pairs(&transcript),
        pairs(&[("accounts_file_write_failed", "admin.accounts_file")]),
        "{transcript}"
    );
    assert!(transcript.contains("__RC=5"), "{transcript}");
    assert_no_pty_secret(&transcript, &typed);
    assert_eq!(fixture.bytes(), original);
}
