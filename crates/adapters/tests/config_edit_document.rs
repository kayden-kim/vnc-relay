//! Real-file and byte-preservation coverage for config-v4 §1.10.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use vnc_relay_adapters::config_edit::{FsConfigFileStore, TomlConfigDocument};
use vnc_relay_adapters::config_file::parse_candidate;
use vnc_relay_app::management::config::{
    ConfigChange, ConfigDocument, ConfigFileStore, ConfigReplaceError, ConfigValue,
};
use vnc_relay_app::settings::ConfigError;

const REQUIRED: &str = "[admin]\naccounts_file = 'accounts.toml'\n[management]\nlisten = '127.0.0.1:18080'\n[status]\nlisten = '127.0.0.1:18081'\n";

fn change(key: &str, value: ConfigValue) -> ConfigChange {
    ConfigChange {
        key: key.to_owned(),
        value,
    }
}

fn strings(values: &[&str]) -> ConfigValue {
    ConfigValue::Strings(values.iter().map(|value| (*value).to_owned()).collect())
}

fn edit(source: &str, changes: &[ConfigChange]) -> String {
    let bytes = TomlConfigDocument
        .edit(source.as_bytes(), changes)
        .unwrap_or_else(|diagnostics| panic!("edit failed: {diagnostics:?}"));
    String::from_utf8(bytes).unwrap_or_else(|error| panic!("non-UTF8 result: {error}"))
}

fn array_values(source: &str) -> Vec<String> {
    let table: toml::Table = source
        .parse()
        .unwrap_or_else(|error| panic!("invalid edited TOML: {error}"));
    table["relay"]["ws"]["viewer"]["allowed_origins"]
        .as_array()
        .unwrap_or_else(|| panic!("not an array"))
        .iter()
        .map(|value| {
            value
                .as_str()
                .unwrap_or_else(|| panic!("not a string"))
                .to_owned()
        })
        .collect()
}

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "vnc-relay-config-edit-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap_or_else(|error| panic!("create fixture: {error}"));
        Self(path)
    }

    fn path(&self) -> PathBuf {
        self.0.join("relay.toml")
    }

    fn entries(&self) -> Vec<PathBuf> {
        fs::read_dir(&self.0)
            .unwrap_or_else(|error| panic!("read fixture: {error}"))
            .map(|entry| {
                entry
                    .unwrap_or_else(|error| panic!("read entry: {error}"))
                    .path()
            })
            .collect()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap_or_else(|error| panic!("remove fixture: {error}"));
    }
}

#[test]
fn raw_revision_uses_sha256_and_includes_comments_bom_and_line_endings() {
    let store = FsConfigFileStore;
    assert_eq!(
        store.revision(b""),
        "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        store.revision(b"abc"),
        "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    for (left, right) in [
        (b"a\n".as_slice(), b"a\r\n".as_slice()),
        (b"a".as_slice(), b"a # comment".as_slice()),
        (b"a".as_slice(), b"\xef\xbb\xbfa".as_slice()),
        (b"a".as_slice(), b"\xff".as_slice()),
    ] {
        assert_ne!(store.revision(left), store.revision(right));
    }
}

#[test]
fn candidate_defaults_config_edit_off_and_collects_only_explicit_leaf_keys() {
    let base = Path::new("config-base");
    let minimal = parse_candidate(REQUIRED.as_bytes(), base)
        .unwrap_or_else(|diagnostics| panic!("parse: {diagnostics:?}"));
    assert!(!minimal.settings.management.config_edit);
    assert_eq!(
        minimal.settings.admin.accounts_file,
        base.join("accounts.toml")
    );
    assert_eq!(
        minimal.presence.keys.into_iter().collect::<Vec<_>>(),
        ["admin.accounts_file", "management.listen", "status.listen"]
    );
    let source = format!(
        "{}\n[relay.ws.viewer]\nenabled = false\nallowed_origins = []\nallow = ['127.0.0.1']\n[mode1]\nenabled = false\nallow_targets = []\n",
        REQUIRED.replace("[management]\n", "[management]\nconfig_edit = true\n")
    );
    let parsed = TomlConfigDocument
        .parse(source.as_bytes(), base)
        .unwrap_or_else(|diagnostics| panic!("parse: {diagnostics:?}"));
    assert!(parsed.settings.management.config_edit);
    assert!(parsed.presence.keys.contains("management.config_edit"));
    assert!(
        parsed
            .presence
            .keys
            .contains("relay.ws.viewer.allowed_origins")
    );
    assert!(parsed.presence.keys.contains("relay.ws.viewer.allow"));
    assert!(parsed.presence.keys.contains("mode1.allow_targets"));
    assert!(!parsed.presence.keys.contains("relay.ws.listen"));
    assert!(!parsed.presence.keys.contains("relay.ws.viewer"));
    assert!(parsed.presence.keys.iter().all(|key| !key.contains('[')));
}

#[test]
fn candidate_presence_handles_inline_and_dotted_tables_and_checks_new_flag_type() {
    let source = "admin.accounts_file='accounts.toml'\nmanagement={listen='127.0.0.1:18080', config_edit=true}\nstatus.listen='127.0.0.1:18081'\nrelay.ws.viewer.allowed_origins=[]\n";
    let parsed = parse_candidate(source.as_bytes(), Path::new("."))
        .unwrap_or_else(|diagnostics| panic!("parse: {diagnostics:?}"));
    assert_eq!(parsed.presence.keys.len(), 5);
    assert!(
        parsed
            .presence
            .keys
            .contains("relay.ws.viewer.allowed_origins")
    );
    let invalid = source.replace("config_edit=true", "config_edit='true'");
    let diagnostics = parse_candidate(invalid.as_bytes(), Path::new("."))
        .err()
        .unwrap_or_else(|| panic!("accepted non-boolean flag"));
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.error == ConfigError::TypeMismatch
                && diagnostic.key == "management.config_edit")
    );
}

/// spec: management.md:MGT-035 partial
#[test]
fn non_utf8_is_unreadable_without_echoing_file_content() {
    let bytes = b"private sentinel\xff";
    for diagnostics in [
        TomlConfigDocument
            .parse(bytes, Path::new("."))
            .err()
            .unwrap_or_else(|| panic!("accepted invalid UTF8")),
        TomlConfigDocument
            .edit(bytes, &[])
            .err()
            .unwrap_or_else(|| panic!("edited invalid UTF8")),
    ] {
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].error, ConfigError::FileUnreadable);
        assert_eq!(diagnostics[0].key, "");
        assert!(!diagnostics[0].message.contains("sentinel"));
    }
    let diagnostics = TomlConfigDocument
        .edit(b"secret = [ private sentinel", &[])
        .err();
    // Empty changes must still validate syntax, not hide a corrupt document.
    assert!(diagnostics.is_some());
    let diagnostics = diagnostics.unwrap_or_else(|| panic!("missing syntax diagnostic"));
    assert_eq!(diagnostics[0].error, ConfigError::SyntaxError);
    assert!(!diagnostics[0].message.contains("sentinel"));
}

/// spec: management.md:MGT-039 partial
#[test]
fn semantically_equal_values_keep_every_original_byte_and_file_metadata() {
    let source = "\u{feff}# header\r\n[limits]\r\nrelay_connections = 0x3e8 # hex\n[relay.ws.viewer]\r\nenabled=false\nallowed_origins = [\r\n  'https://example.test', # retained, comma\n]\r\n";
    let changes = [
        change("limits.relay_connections", ConfigValue::Integer(1000)),
        change("relay.ws.viewer.enabled", ConfigValue::Bool(false)),
        change(
            "relay.ws.viewer.allowed_origins",
            strings(&["https://example.test"]),
        ),
    ];
    let fixture = Fixture::new();
    let path = fixture.path();
    fs::write(&path, source).unwrap_or_else(|error| panic!("write fixture: {error}"));
    let before = fs::metadata(&path).unwrap_or_else(|error| panic!("metadata: {error}"));
    let result = TomlConfigDocument
        .edit(
            &FsConfigFileStore
                .read(&path)
                .unwrap_or_else(|error| panic!("read: {error}")),
            &changes,
        )
        .unwrap_or_else(|diagnostics| panic!("edit: {diagnostics:?}"));
    assert_eq!(result, source.as_bytes());
    let after = fs::metadata(&path).unwrap_or_else(|error| panic!("metadata: {error}"));
    assert_eq!(before.modified().ok(), after.modified().ok());
    assert_eq!(before.len(), after.len());
    assert_eq!(before.permissions(), after.permissions());
    assert_eq!(fixture.entries(), [path]);
}

/// spec: management.md:MGT-031 partial
/// spec: operations.md:OPS-060 partial
#[test]
fn scalar_edits_preserve_bom_comments_decor_and_mixed_newlines_exactly() {
    let source = "\u{feff}# header\r\n[limits] # section\n relay_connections\t=\t1_000  # keep\r\nwaiting = 2\n\n[mode2]\r\nenabled = true # untouched\n";
    assert_eq!(
        edit(
            source,
            &[change(
                "limits.relay_connections",
                ConfigValue::Integer(900)
            )]
        ),
        source.replace("1_000", "900")
    );
    assert_eq!(
        edit(source, &[change("mode2.enabled", ConfigValue::Bool(false))]),
        source.replace("enabled = true", "enabled = false")
    );
}

/// spec: management.md:MGT-031 partial
/// spec: operations.md:OPS-060 partial
#[test]
fn array_value_edits_keep_internal_comments_spacing_and_each_newline() {
    let source = "[relay.ws.viewer]\r\nallowed_origins = [\r\n  'https://one.test', # first, comma\n\t\"https://two.test\", # second\r\n  # tail\n]  # after\r\n";
    let result = edit(
        source,
        &[change(
            "relay.ws.viewer.allowed_origins",
            strings(&["https://one.test", "https://new.test"]),
        )],
    );
    assert_eq!(
        result,
        source.replace("\"https://two.test\"", "\"https://new.test\"")
    );
    assert_eq!(
        array_values(&result),
        ["https://one.test", "https://new.test"]
    );
}

/// spec: management.md:MGT-031 partial
#[test]
fn array_shrink_empty_and_grow_keep_all_comments_and_parse_as_requested() {
    let source = "[relay.ws.viewer]\nallowed_origins = [\n  'https://one.test', # first, comma\n  'https://two.test', # second, comma\n  # tail, comma\n] # outside\n";
    for values in [
        vec!["https://new.test"],
        vec![],
        vec![
            "https://one.test",
            "https://two.test",
            "https://three.test",
            "https://four.test",
        ],
    ] {
        let result = edit(
            source,
            &[change("relay.ws.viewer.allowed_origins", strings(&values))],
        );
        assert_eq!(array_values(&result), values);
        for comment in [
            "# first, comma",
            "# second, comma",
            "# tail, comma",
            "# outside",
        ] {
            assert!(result.contains(comment), "missing {comment}: {result}");
        }
    }
    for source in [
        "[relay.ws.viewer]\nallowed_origins=[] # empty\n",
        "[relay.ws.viewer]\r\nallowed_origins=[\r\n # retained\r\n]\r\n",
        "[relay.ws.viewer]\nallowed_origins=['a', 'b'] # no trailing comma\n",
    ] {
        let result = edit(
            source,
            &[change(
                "relay.ws.viewer.allowed_origins",
                strings(&["one", "two", "three"]),
            )],
        );
        assert_eq!(array_values(&result), ["one", "two", "three"]);
        if source.contains("# retained") {
            assert!(result.contains("# retained"));
        }
    }
}

/// spec: management.md:MGT-031 partial
#[test]
fn inline_and_dotted_table_edits_change_only_the_value_token() {
    for source in [
        "relay = { ws = { viewer = { enabled = false, allowed_origins = ['old'] } } } # inline\r\n",
        "relay.ws.viewer.enabled=false\r\nrelay.ws.viewer.allowed_origins=['old'] # dotted\n",
        "[relay]\nws.viewer.enabled=false\nws.viewer.allowed_origins=['old']\n",
        "relay = { ws.viewer.allowed_origins = ['old'] } # dotted inline\n",
    ] {
        let result = edit(
            source,
            &[change("relay.ws.viewer.allowed_origins", strings(&["new"]))],
        );
        assert_eq!(result, source.replace("'old'", "\"new\""));
        assert_eq!(array_values(&result), ["new"]);
    }
}

/// spec: management.md:MGT-031 partial
/// spec: operations.md:OPS-060 partial
#[test]
fn absent_keys_use_existing_tables_and_absent_tables_append_with_first_newline() {
    let source = "# heading\r\n[limits] # keep\nrelay_connections=1000\r\n# status comment\n[status]\nlisten='127.0.0.1:18081'";
    let result = edit(
        source,
        &[
            change("limits.waiting", ConfigValue::Integer(100)),
            change("timeouts.close_secs", ConfigValue::Integer(5)),
            change("timeouts.wait_secs", ConfigValue::Integer(10)),
        ],
    );
    let expected = source.replace(
        "relay_connections=1000\r\n",
        "relay_connections=1000\r\nwaiting = 100\r\n",
    );
    assert_eq!(
        result,
        format!("{expected}\r\n[timeouts]\r\nclose_secs = 5\r\nwait_secs = 10\r\n")
    );
    let parsed: toml::Table = result
        .parse()
        .unwrap_or_else(|error| panic!("parse: {error}"));
    assert_eq!(parsed["limits"]["waiting"].as_integer(), Some(100));
    assert_eq!(parsed["timeouts"]["close_secs"].as_integer(), Some(5));
    assert_eq!(
        edit(
            "",
            &[change("timeouts.close_secs", ConfigValue::Integer(5))]
        ),
        "[timeouts]\nclose_secs = 5\n"
    );
    assert_eq!(
        edit(
            "[limits]",
            &[change("limits.waiting", ConfigValue::Integer(1))]
        ),
        "[limits]\nwaiting = 1\n"
    );
}

#[test]
fn absent_dotted_inline_and_implicit_parent_keys_stay_in_the_right_scope() {
    for source in [
        "management.listen='127.0.0.1:18080'\n[status]\nlisten='127.0.0.1:18081'\n",
        "management={listen='127.0.0.1:18080'}\nstatus.listen='127.0.0.1:18081'\n",
        "[management.session]\nidle_secs=1800\n[status]\nlisten='127.0.0.1:18081'\n",
        "management={session.idle_secs=1800}\nstatus.listen='127.0.0.1:18081'\n",
    ] {
        let result = edit(
            source,
            &[
                change("management.config_edit", ConfigValue::Bool(true)),
                change(
                    "management.session.max_lifetime_secs",
                    ConfigValue::Integer(7200),
                ),
            ],
        );
        let table: toml::Table = result
            .parse()
            .unwrap_or_else(|error| panic!("parse {result}: {error}"));
        assert_eq!(table["management"]["config_edit"].as_bool(), Some(true));
        assert_eq!(
            table["management"]["session"]["max_lifetime_secs"].as_integer(),
            Some(7200)
        );
        assert_eq!(table["status"]["listen"].as_str(), Some("127.0.0.1:18081"));
    }
}

#[test]
fn inserted_strings_are_toml_encoded_without_changing_unrelated_text() {
    let value = "한글 \"quotes\" \\ slash\nnew line";
    let source = "[relay.ws.viewer]\nallowed_origins = [] # keep\n";
    let result = edit(
        source,
        &[change("relay.ws.viewer.allowed_origins", strings(&[value]))],
    );
    assert_eq!(array_values(&result), [value]);
    assert!(result.ends_with(" # keep\n"));
    let result = edit(
        "[admin]\naccounts_file='old' # keep\n",
        &[change(
            "admin.accounts_file",
            ConfigValue::String(value.to_owned()),
        )],
    );
    let table: toml::Table = result
        .parse()
        .unwrap_or_else(|error| panic!("parse: {error}"));
    assert_eq!(table["admin"]["accounts_file"].as_str(), Some(value));
    assert!(result.ends_with(" # keep\n"));
}

/// spec: management.md:MGT-031 partial
/// spec: operations.md:OPS-060 partial
#[test]
fn real_file_replacement_saves_exact_bytes_preserves_permissions_and_leaves_no_temp() {
    let fixture = Fixture::new();
    let path = fixture.path();
    let original = format!("\u{feff}{}", REQUIRED.replace('\n', "\r\n"));
    fs::write(&path, original.as_bytes()).unwrap_or_else(|error| panic!("write: {error}"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .unwrap_or_else(|error| panic!("chmod: {error}"));
    }
    let before = fs::metadata(&path).unwrap_or_else(|error| panic!("metadata: {error}"));
    let next = edit(
        &original,
        &[change("management.config_edit", ConfigValue::Bool(true))],
    );
    let store = FsConfigFileStore;
    store
        .replace(&path, &store.revision(original.as_bytes()), next.as_bytes())
        .unwrap_or_else(|error| panic!("replace: {error:?}"));
    let bytes = store
        .read(&path)
        .unwrap_or_else(|error| panic!("read: {error}"));
    assert_eq!(bytes, next.as_bytes());
    assert_ne!(store.revision(&bytes), store.revision(original.as_bytes()));
    let parsed = TomlConfigDocument
        .parse(&bytes, &fixture.0)
        .unwrap_or_else(|diagnostics| panic!("parse: {diagnostics:?}"));
    assert!(parsed.settings.management.config_edit);
    let after = fs::metadata(&path).unwrap_or_else(|error| panic!("metadata: {error}"));
    assert_eq!(after.len(), next.len() as u64);
    assert_eq!(after.permissions(), before.permissions());
    assert_eq!(fixture.entries(), [path]);
}

/// spec: management.md:MGT-033 partial
#[test]
fn stale_revision_keeps_external_bytes_and_metadata_and_removes_own_temp() {
    let fixture = Fixture::new();
    let path = fixture.path();
    let external = b"# external save\r\n";
    fs::write(&path, external).unwrap_or_else(|error| panic!("write: {error}"));
    let before = fs::metadata(&path).unwrap_or_else(|error| panic!("metadata: {error}"));
    let store = FsConfigFileStore;
    let result = store.replace(
        &path,
        &store.revision(REQUIRED.as_bytes()),
        b"request must not win",
    );
    assert_eq!(result, Err(ConfigReplaceError::Conflict));
    assert_eq!(
        fs::read(&path).unwrap_or_else(|error| panic!("read: {error}")),
        external
    );
    let after = fs::metadata(&path).unwrap_or_else(|error| panic!("metadata: {error}"));
    assert_eq!(before.modified().ok(), after.modified().ok());
    assert_eq!(before.len(), after.len());
    assert_eq!(fixture.entries(), [path]);
}

/// spec: management.md:MGT-038 partial
/// spec: operations.md:OPS-061 partial
#[test]
fn missing_parent_fails_before_touching_any_existing_file() {
    let fixture = Fixture::new();
    let path = fixture.path();
    fs::write(&path, REQUIRED).unwrap_or_else(|error| panic!("write: {error}"));
    let store = FsConfigFileStore;
    let result = store.replace(
        &fixture.0.join("absent").join("relay.toml"),
        &store.revision(REQUIRED.as_bytes()),
        b"candidate",
    );
    assert_eq!(result, Err(ConfigReplaceError::WriteFailed));
    assert_eq!(
        fs::read(&path).unwrap_or_else(|error| panic!("read: {error}")),
        REQUIRED.as_bytes()
    );
    assert_eq!(fixture.entries(), [path]);
}

/// spec: management.md:MGT-038 partial
/// spec: operations.md:OPS-061 partial
#[cfg(windows)]
#[test]
fn windows_open_handle_without_share_delete_prevents_rename_and_cleans_temp() {
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;
    let fixture = Fixture::new();
    let path = fixture.path();
    fs::write(&path, REQUIRED).unwrap_or_else(|error| panic!("write: {error}"));
    let handle = OpenOptions::new()
        .read(true)
        .share_mode(0x1 | 0x2)
        .open(&path)
        .unwrap_or_else(|error| panic!("open held file: {error}"));
    let store = FsConfigFileStore;
    assert_eq!(
        store.replace(&path, &store.revision(REQUIRED.as_bytes()), b"candidate"),
        Err(ConfigReplaceError::WriteFailed)
    );
    assert_eq!(
        fs::read(&path).unwrap_or_else(|error| panic!("read: {error}")),
        REQUIRED.as_bytes()
    );
    assert_eq!(fixture.entries(), [path]);
    drop(handle);
}

#[test]
fn disabled_lone_tls_paths_survive_candidate_presence_with_resolved_paths() {
    let base = Path::new("config-base");
    for key in ["cert_file", "key_file"] {
        let source = format!(
            "{REQUIRED}\n[relay.wss]\n{key} = 'tls/disabled.pem'\n[relay.wss.viewer]\nenabled = false\n[relay.wss.server]\nenabled = false\n"
        );
        let candidate = parse_candidate(source.as_bytes(), base)
            .unwrap_or_else(|diagnostics| panic!("parse: {diagnostics:?}"));
        assert!(candidate.settings.relay.wss.tls.is_none());
        assert_eq!(candidate.presence.inactive_paths.len(), 1);
        assert_eq!(
            candidate
                .presence
                .inactive_paths
                .get(&format!("relay.wss.{key}")),
            Some(&base.join("tls/disabled.pem"))
        );
        assert!(
            candidate
                .presence
                .keys
                .contains(&format!("relay.wss.{key}"))
        );
    }
    let source = format!("{REQUIRED}\n[relay.wss]\ncert_file='cert.pem'\nkey_file='key.pem'\n");
    let candidate = parse_candidate(source.as_bytes(), base)
        .unwrap_or_else(|diagnostics| panic!("parse: {diagnostics:?}"));
    assert!(candidate.presence.inactive_paths.is_empty());
    assert_eq!(
        candidate
            .settings
            .relay
            .wss
            .tls
            .as_ref()
            .map(|tls| tls.cert_file.as_path()),
        Some(base.join("cert.pem").as_path())
    );
    let candidate = parse_candidate(REQUIRED.as_bytes(), base)
        .unwrap_or_else(|diagnostics| panic!("parse: {diagnostics:?}"));
    assert!(candidate.presence.inactive_paths.is_empty());
}

/// spec: management.md:MGT-031 partial
/// spec: operations.md:OPS-060 partial
#[test]
fn missing_nested_role_tables_append_without_rewriting_listener_or_sibling_bytes() {
    for source in [
        "# untouched\r\n[status]\nlisten='127.0.0.1:18081' # keep",
        "# untouched\r\n[relay.ws] # listener\nlisten='127.0.0.1:18090'\n[status]\nlisten='127.0.0.1:18081' # keep",
        "# untouched\r\n[relay.ws.viewer]\nallow=['192.0.2.1'] # sibling\n[status]\nlisten='127.0.0.1:18081' # keep",
    ] {
        let result = edit(
            source,
            &[
                change("relay.ws.server.allow", strings(&["192.0.2.2"])),
                change(
                    "relay.ws.server.allowed_origins",
                    strings(&["https://server.example"]),
                ),
            ],
        );
        assert_eq!(
            result,
            format!(
                "{source}\r\n[relay.ws.server]\r\nallow = [\"192.0.2.2\"]\r\n\
                 allowed_origins = [\"https://server.example\"]\r\n"
            )
        );
        let table: toml::Table = result.parse().expect("valid nested table");
        assert_eq!(
            table["relay"]["ws"]["server"]["allow"][0].as_str(),
            Some("192.0.2.2")
        );
        assert_eq!(
            table["relay"]["ws"]["server"]["allowed_origins"][0].as_str(),
            Some("https://server.example")
        );
    }
}

/// spec: management.md:MGT-031 partial
/// spec: operations.md:OPS-060 partial
#[test]
fn missing_shared_listener_keys_append_at_parent_not_under_existing_role() {
    let source = "# heading\r\n[relay.wss.server]\nenabled=false\nallow=[] # role\n[status]\nlisten='127.0.0.1:18081'\n";
    let result = edit(
        source,
        &[
            change("relay.wss.trusted_proxies", strings(&["192.0.2.10"])),
            change(
                "relay.wss.listen",
                ConfigValue::String("127.0.0.1:18090".to_owned()),
            ),
        ],
    );
    assert_eq!(
        result,
        format!(
            "{source}[relay.wss]\r\ntrusted_proxies = [\"192.0.2.10\"]\r\n\
             listen = \"127.0.0.1:18090\"\r\n"
        )
    );
    let table: toml::Table = result.parse().expect("valid shared listener table");
    assert_eq!(
        table["relay"]["wss"]["trusted_proxies"][0].as_str(),
        Some("192.0.2.10")
    );
    assert!(
        table["relay"]["wss"]["server"]
            .get("trusted_proxies")
            .is_none()
    );
}

/// spec: management.md:MGT-031 partial
#[test]
fn missing_nested_keys_respect_dotted_and_inline_listener_scopes() {
    for source in [
        "relay.ws.viewer.allow=['192.0.2.1'] # keep\r\n[status]\nlisten='127.0.0.1:18081'\n",
        "[relay]\nws.viewer.allow=['192.0.2.1'] # keep\r\n[status]\nlisten='127.0.0.1:18081'\n",
        "relay={ws={viewer={allow=['192.0.2.1']}}} # keep\r\nstatus.listen='127.0.0.1:18081'\n",
        "relay={ws.viewer.allow=['192.0.2.1']} # keep\r\nstatus.listen='127.0.0.1:18081'\n",
    ] {
        let result = edit(
            source,
            &[
                change(
                    "relay.ws.viewer.allowed_origins",
                    strings(&["https://viewer.example"]),
                ),
                change("relay.ws.server.allow", strings(&["192.0.2.2"])),
                change("relay.ws.trusted_proxies", strings(&["192.0.2.10"])),
            ],
        );
        let table: toml::Table = result.parse().expect("valid dotted or inline edit");
        assert_eq!(
            table["relay"]["ws"]["viewer"]["allow"][0].as_str(),
            Some("192.0.2.1")
        );
        assert_eq!(
            table["relay"]["ws"]["viewer"]["allowed_origins"][0].as_str(),
            Some("https://viewer.example")
        );
        assert_eq!(
            table["relay"]["ws"]["server"]["allow"][0].as_str(),
            Some("192.0.2.2")
        );
        assert_eq!(
            table["relay"]["ws"]["trusted_proxies"][0].as_str(),
            Some("192.0.2.10")
        );
        assert_eq!(table["status"]["listen"].as_str(), Some("127.0.0.1:18081"));
        assert!(result.contains("# keep\r\n"));
        assert!(result.contains("['192.0.2.1']"));
    }
}

/// spec: operations.md:OPS-065 partial
#[test]
fn nested_role_edits_and_trusted_proxy_edits_parse_through_the_real_loader() {
    let source = format!(
        "{REQUIRED}[relay.ws]\nlisten='127.0.0.1:18090'\n\
         [relay.ws.viewer]\nenabled=true\nallow=[] # viewer\n\
         [relay.ws.server]\nenabled=true\nallow=[] # server\n"
    );
    let result = edit(
        &source,
        &[
            change("relay.ws.trusted_proxies", strings(&["192.0.2.10"])),
            change("relay.ws.viewer.allow", strings(&["192.0.2.1"])),
            change("relay.ws.server.allow", strings(&["192.0.2.2"])),
            change(
                "relay.ws.server.allowed_origins",
                strings(&["https://server.example"]),
            ),
        ],
    );
    let candidate = parse_candidate(result.as_bytes(), Path::new(".")).expect("v4 loader");
    assert!(candidate.settings.relay.ws.viewer.enabled);
    assert!(candidate.settings.relay.ws.server.enabled);
    assert_eq!(candidate.settings.relay.ws.trusted_proxies.items().len(), 1);
    assert_eq!(candidate.settings.relay.ws.server.allowed_origins.len(), 1);
    assert!(result.contains("# viewer\n"));
    assert!(result.contains("# server\n"));
    assert!(
        !candidate
            .presence
            .keys
            .contains("relay.ws.viewer.trusted_proxies")
    );
}

/// spec: operations.md:OPS-065 partial
#[test]
fn server_only_wss_uses_parent_tls_and_never_records_inactive_paths() {
    let base = Path::new("config-base");
    let source = format!(
        "{REQUIRED}[relay.wss]\nlisten='127.0.0.1:18090'\n\
         cert_file='tls/cert.pem'\nkey_file='tls/key.pem'\n\
         [relay.wss.viewer]\nenabled=false\n\
         [relay.wss.server]\nenabled=true\nallow=[]\n"
    );
    let candidate = parse_candidate(source.as_bytes(), base).expect("server-only WSS");
    assert!(!candidate.settings.relay.wss.viewer.enabled);
    assert!(candidate.settings.relay.wss.server.enabled);
    assert!(candidate.presence.inactive_paths.is_empty());
    let tls = candidate.settings.relay.wss.tls.expect("parent TLS pair");
    assert_eq!(tls.cert_file, base.join("tls/cert.pem"));
    assert_eq!(tls.key_file, base.join("tls/key.pem"));
    assert!(candidate.presence.keys.contains("relay.wss.cert_file"));
    assert!(candidate.presence.keys.contains("relay.wss.key_file"));
    assert!(
        !candidate
            .presence
            .keys
            .contains("relay.wss.server.cert_file")
    );
}
