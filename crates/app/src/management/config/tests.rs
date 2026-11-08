use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::Duration;

use vnc_relay_domain::allow_list::AllowList;
use vnc_relay_domain::target_policy::TargetPolicy;

use super::*;
use crate::reload::ports::{ReloadFileStore, ReloadTlsStore, TlsBuildError};
use crate::reload::service::ReloadService;
use crate::reload::snapshot::{ReloadCell, ReloadSnapshot};
use crate::reload::{ConfigFileState, ConfigPresence, ReloadResult};
use crate::settings::{
    AdminSessionSettings, AdminSettings, Argon2Settings, ConfigError, DashboardSettings,
    EntryPointSettings, KeepaliveSettings, LimitSettings, LogOutput, LogSettings,
    LoginLimitSettings, ManagementSettings, Mode1Settings, Mode2Settings, RecentClosedSettings,
    RelaySettings, ReloadSettings, Settings, StatusSettings, TimeoutSettings, TlsFiles,
};

type Trace = Arc<Mutex<Vec<String>>>;

fn record(trace: &Trace, call: impl Into<String>) {
    trace
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(call.into());
}

fn addr(port: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], port))
}

// The app reload fixture, deliberately not constructed by an adapter parser.
fn settings() -> Settings {
    Settings {
        admin: AdminSettings {
            accounts_file: PathBuf::from("accounts.json"),
            argon2: Argon2Settings {
                memory_kib: 19_456,
                iterations: 2,
                parallelism: 1,
            },
        },
        relay: RelaySettings {
            tcp_viewer: EntryPointSettings {
                enabled: true,
                listen: Some(addr(5901)),
                allow: Some(AllowList::default()),
                ..Default::default()
            },
            tcp_server: EntryPointSettings {
                enabled: true,
                listen: Some(addr(5500)),
                allow: Some(AllowList::default()),
                ..Default::default()
            },
            ws_viewer: EntryPointSettings::default(),
            wss_viewer: EntryPointSettings::default(),
        },
        mode1: Mode1Settings::default(),
        mode2: Mode2Settings::default(),
        limits: LimitSettings {
            relay_connections: 100,
            admin_connections: 10,
            waiting: 50,
            buffer_bytes: 65_536,
            ws_message_bytes: 1 << 20,
        },
        timeouts: TimeoutSettings {
            handshake: Duration::from_secs(10),
            tls_upgrade: Duration::from_secs(10),
            wait: Duration::from_secs(30),
            connect: Duration::from_secs(10),
            close: Duration::from_secs(5),
            shutdown_grace: Duration::from_secs(10),
        },
        keepalive: KeepaliveSettings {
            tcp_idle: Duration::from_secs(60),
            tcp_interval: Duration::from_secs(10),
            tcp_probes: 3,
            ws_ping: Duration::from_secs(30),
            ws_pong_timeout: Duration::from_secs(10),
        },
        management: ManagementSettings {
            listen: addr(8080),
            tls: None,
            config_edit: true,
            login_limit: LoginLimitSettings {
                per_account: 5,
                per_source: 20,
                window: Duration::from_secs(60),
                lockout: Duration::from_secs(300),
            },
            session: AdminSessionSettings {
                idle: Duration::from_secs(900),
                max_lifetime: Duration::from_secs(28_800),
            },
        },
        dashboard: DashboardSettings {
            poll: Duration::from_secs(2),
            query_timeout: Duration::from_secs(5),
        },
        recent_closed: RecentClosedSettings { max_entries: 100 },
        status: StatusSettings { listen: addr(9090) },
        log: LogSettings {
            output: LogOutput::Stdout,
        },
        reload: ReloadSettings {
            settle: Duration::from_secs(2),
            poll: Duration::from_secs(5),
        },
    }
}

fn transaction_settings() -> Settings {
    let mut value = settings();
    value.management.tls = Some(TlsFiles {
        cert_file: PathBuf::from("management.crt"),
        key_file: PathBuf::from("management.key"),
    });
    value
}

fn candidate(settings: Settings) -> ConfigCandidate {
    ConfigCandidate {
        settings,
        presence: ConfigPresence::default(),
    }
}

fn diagnostic(error: ConfigError, key: &str) -> Vec<ConfigDiagnostic> {
    vec![ConfigDiagnostic {
        error,
        key: key.to_owned(),
        message: "configuration rejected".to_owned(),
    }]
}

fn revision_of(bytes: &[u8]) -> String {
    let digit = if bytes == b"candidate" { 'b' } else { 'a' };
    format!("sha256:{}", digit.to_string().repeat(64))
}

struct FakeFiles {
    trace: Trace,
    bytes: Mutex<Vec<u8>>,
    unreadable: Mutex<bool>,
    replace_error: Mutex<Option<ConfigReplaceError>>,
    cell: Arc<ReloadCell<u32>>,
    snapshot_at_replace: Mutex<Option<Arc<ReloadSnapshot<u32>>>>,
}

impl ConfigFileStore for FakeFiles {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        assert_eq!(path, Path::new("config/relay.toml"));
        record(&self.trace, "read");
        if *self
            .unreadable
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
        {
            Err(io::Error::from(io::ErrorKind::PermissionDenied))
        } else {
            Ok(self
                .bytes
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone())
        }
    }

    fn revision(&self, bytes: &[u8]) -> String {
        record(
            &self.trace,
            format!("revision:{}", String::from_utf8_lossy(bytes)),
        );
        revision_of(bytes)
    }

    fn replace(&self, path: &Path, expected: &str, next: &[u8]) -> Result<(), ConfigReplaceError> {
        assert_eq!(path, Path::new("config/relay.toml"));
        assert_eq!(expected, revision_of(b"original"));
        assert_eq!(next, b"candidate");
        record(&self.trace, "replace");
        *self
            .snapshot_at_replace
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(self.cell.current());
        if let Some(error) = *self
            .replace_error
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
        {
            return Err(error);
        }
        *self
            .bytes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = next.to_vec();
        Ok(())
    }
}

struct FakeDocument {
    trace: Trace,
    original: Mutex<Result<ConfigCandidate, Vec<ConfigDiagnostic>>>,
    next: Mutex<Result<ConfigCandidate, Vec<ConfigDiagnostic>>>,
    edit_error: Mutex<Option<Vec<ConfigDiagnostic>>>,
    changed_keys: Vec<String>,
    noop: bool,
}

impl ConfigDocument for FakeDocument {
    fn parse(
        &self,
        bytes: &[u8],
        base_dir: &Path,
    ) -> Result<ConfigCandidate, Vec<ConfigDiagnostic>> {
        assert_eq!(base_dir, Path::new("config"));
        record(
            &self.trace,
            format!("parse:{}", String::from_utf8_lossy(bytes)),
        );
        match bytes {
            b"original" => self
                .original
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone(),
            b"candidate" => self
                .next
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone(),
            _ => panic!("unexpected symbolic document"),
        }
    }

    fn edit(
        &self,
        bytes: &[u8],
        changes: &[ConfigChange],
    ) -> Result<Vec<u8>, Vec<ConfigDiagnostic>> {
        assert_eq!(bytes, b"original");
        let keys = changes
            .iter()
            .map(|change| change.key.as_str())
            .collect::<Vec<_>>();
        record(&self.trace, format!("edit:{}", keys.join(",")));
        if let Some(diagnostics) = self
            .edit_error
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
        {
            return Err(diagnostics.clone());
        }
        let changes_file = changes
            .iter()
            .any(|change| self.changed_keys.contains(&change.key));
        Ok(if self.noop || !changes_file {
            b"original".to_vec()
        } else {
            b"candidate".to_vec()
        })
    }
}

struct FakeTls {
    trace: Trace,
    bad_original: Mutex<bool>,
}

impl ReloadFileStore for FakeTls {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        record(&self.trace, format!("tls-read:{}", path.display()));
        match path
            .to_str()
            .unwrap_or_else(|| panic!("fixture path must be UTF-8"))
        {
            "management.crt"
                if *self
                    .bad_original
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) =>
            {
                Ok(b"bad".to_vec())
            }
            "management.crt" => Ok(b"cert-v1".to_vec()),
            "management.key" => Ok(b"key-v1".to_vec()),
            "next.crt" => Ok(b"cert-v2".to_vec()),
            "bad.crt" => Ok(b"bad".to_vec()),
            _ => panic!("unexpected TLS file"),
        }
    }
}

impl ReloadTlsStore<u32> for FakeTls {
    fn build(&self, cert: &[u8], key: &[u8]) -> Result<u32, TlsBuildError> {
        assert_eq!(key, b"key-v1");
        record(&self.trace, "tls-build");
        if cert == b"bad" {
            Err(TlsBuildError {
                detail: "pair mismatch",
            })
        } else {
            Ok(2)
        }
    }
}

struct RecordingSink {
    trace: Trace,
    cell: Arc<ReloadCell<u32>>,
    observations: Mutex<Vec<(Event, Arc<ReloadSnapshot<u32>>)>>,
}

impl EventSink for RecordingSink {
    fn record(&self, event: &Event) {
        let name = match event {
            Event::ReloadApplied { .. } => "reload_applied".to_owned(),
            Event::ConfigEdited { .. } => "config_edited".to_owned(),
            Event::ConfigEditRejected { reason, .. } => format!("rejected:{reason}"),
            _ => panic!("unexpected transaction event: {event:?}"),
        };
        record(&self.trace, name);
        self.observations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((event.clone(), self.cell.current()));
    }
}

struct FixedClock(Trace);

impl Clock for FixedClock {
    fn monotonic(&self) -> Duration {
        Duration::from_secs(200)
    }
    fn wall(&self) -> SystemTime {
        record(&self.0, "wall");
        SystemTime::UNIX_EPOCH + Duration::from_secs(200)
    }
}

struct Harness {
    service: ConfigEditService<u32>,
    reload: SharedReload<u32>,
    files: Arc<FakeFiles>,
    document: Arc<FakeDocument>,
    tls: Arc<FakeTls>,
    sink: Arc<RecordingSink>,
    cell: Arc<ReloadCell<u32>>,
    trace: Trace,
}

impl Harness {
    fn new(
        running: Settings,
        original: Settings,
        next: Settings,
        changed_keys: &[&str],
        noop: bool,
    ) -> Self {
        let trace = Arc::new(Mutex::new(Vec::new()));
        let running = candidate(running);
        let management_tls = running.settings.management.tls.as_ref().map(|_| 1);
        let cell = Arc::new(ReloadCell::new(ReloadSnapshot::new(
            running.clone(),
            None,
            management_tls,
        )));
        let tls = Arc::new(FakeTls {
            trace: trace.clone(),
            bad_original: Mutex::new(false),
        });
        let sink = Arc::new(RecordingSink {
            trace: trace.clone(),
            cell: cell.clone(),
            observations: Mutex::new(Vec::new()),
        });
        let reload = Arc::new(Mutex::new(ReloadService::new(
            running,
            cell.clone(),
            tls.clone(),
            tls.clone(),
            sink.clone(),
        )));
        let files = Arc::new(FakeFiles {
            trace: trace.clone(),
            bytes: Mutex::new(b"original".to_vec()),
            unreadable: Mutex::new(false),
            replace_error: Mutex::new(None),
            cell: cell.clone(),
            snapshot_at_replace: Mutex::new(None),
        });
        let document = Arc::new(FakeDocument {
            trace: trace.clone(),
            original: Mutex::new(Ok(candidate(original))),
            next: Mutex::new(Ok(candidate(next))),
            edit_error: Mutex::new(None),
            changed_keys: changed_keys.iter().map(|key| (*key).to_owned()).collect(),
            noop,
        });
        let service = ConfigEditService::new(
            reload.clone(),
            files.clone(),
            document.clone(),
            PathBuf::from("config/relay.toml"),
            PathBuf::from("config"),
            Arc::new(FixedClock(trace.clone())),
            sink.clone(),
        );
        let h = Self {
            service,
            reload,
            files,
            document,
            tls,
            sink,
            cell,
            trace,
        };
        h.clear_observations();
        h
    }

    fn ordinary() -> Self {
        let running = transaction_settings();
        let mut next = running.clone();
        next.timeouts.close = Duration::from_secs(7);
        Self::new(
            running.clone(),
            running,
            next,
            &["timeouts.close_secs"],
            false,
        )
    }

    fn clear_observations(&self) {
        self.trace
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
        self.sink
            .observations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }

    fn calls(&self) -> Vec<String> {
        self.trace
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    // Establish a non-null old history without changing the fixture's final running values.
    fn seed_history(&self) {
        let mut reload = self
            .reload
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let original = reload.running().clone();
        let mut temporary = original.clone();
        temporary.timeouts.wait += Duration::from_secs(1);
        for value in [temporary, original] {
            let prepared = reload
                .prepare_candidate(candidate(value))
                .unwrap_or_else(|error| panic!("{error:?}"));
            reload.commit_prepared(
                prepared,
                ReloadTrigger::FileWatch,
                SystemTime::UNIX_EPOCH + Duration::from_secs(100),
            );
        }
        drop(reload);
        self.clear_observations();
    }

    fn assert_rejected_unchanged(
        &self,
        before: &Arc<ReloadSnapshot<u32>>,
        history: &Option<LastReload>,
        error: &ConfigEditError,
    ) {
        assert!(Arc::ptr_eq(before, &self.cell.current()));
        let reload = self
            .reload
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert_eq!(reload.running(), before.settings.as_ref());
        assert_eq!(&reload.last_reload().cloned(), history);
        assert_eq!(
            *self
                .files
                .bytes
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
            b"original"
        );
        let observations = self
            .sink
            .observations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert_eq!(observations.len(), 1);
        assert_eq!(
            observations[0].0,
            Event::ConfigEditRejected {
                admin_account: actor().admin_account,
                peer_ip: actor().peer_ip,
                reason: error.code().to_owned(),
                config_keys: Some(vec!["timeouts.close_secs".to_owned()]),
            }
        );
        assert!(Arc::ptr_eq(before, &observations[0].1));
    }
}

fn actor() -> ConfigActor {
    ConfigActor {
        admin_account: AccountName::parse("alice").unwrap_or_else(|error| panic!("{error:?}")),
        peer_ip: PeerIp::from(addr(51000)),
    }
}

fn change(key: &str, value: i64) -> ConfigChange {
    ConfigChange {
        key: key.to_owned(),
        value: ConfigValue::Integer(value),
    }
}

fn patch() -> ConfigPatch {
    ConfigPatch {
        revision: revision_of(b"original"),
        changes: vec![change("timeouts.close_secs", 7)],
    }
}

fn original_prepared_calls() -> Vec<&'static str> {
    vec![
        "read",
        "revision:original",
        "parse:original",
        "tls-read:management.crt",
        "tls-read:management.key",
    ]
}

fn candidate_prepared_calls() -> Vec<&'static str> {
    let mut calls = original_prepared_calls();
    calls.extend([
        "edit:timeouts.close_secs",
        "parse:candidate",
        "tls-read:management.crt",
        "tls-read:management.key",
    ]);
    calls
}

/// spec: management.md:MGT-030 partial
/// spec: management.md:MGT-033 partial
/// spec: management.md:MGT-034 partial
/// spec: management.md:MGT-035 partial
/// spec: management.md:MGT-036 partial
/// spec: management.md:MGT-038 partial
/// spec: management.md:MGT-039 partial
#[test]
fn patch_follows_all_ten_steps_and_stops_on_first_error() {
    for case in [
        "disabled",
        "unreadable",
        "non_utf8",
        "stale",
        "invalid_original",
        "original_tls",
        "restart",
        "edit",
        "candidate",
        "write",
        "replace_conflict",
        "success",
    ] {
        let h = Harness::ordinary();
        h.seed_history();
        let mut request = patch();
        let expected = match case {
            "disabled" => {
                // This is a startup switch, so recreate the fixture with it disabled.
                let mut running = transaction_settings();
                running.management.config_edit = false;
                let disabled = Harness::new(
                    running.clone(),
                    running.clone(),
                    running,
                    &["timeouts.close_secs"],
                    false,
                );
                disabled.seed_history();
                let before = disabled.cell.current();
                let history = disabled
                    .reload
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .last_reload()
                    .cloned();
                assert_eq!(
                    disabled.service.patch(actor(), request),
                    Err(ConfigEditError::EditDisabled)
                );
                assert_eq!(disabled.calls(), ["rejected:config_edit_disabled"]);
                disabled.assert_rejected_unchanged(
                    &before,
                    &history,
                    &ConfigEditError::EditDisabled,
                );
                continue;
            }
            "unreadable" => {
                *h.files
                    .unreadable
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = true;
                (
                    ConfigEditError::FileInvalid {
                        diagnostics: vec![],
                    },
                    vec!["read"],
                )
            }
            "non_utf8" => {
                *h.files
                    .bytes
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = vec![0xff];
                let before = h.cell.current();
                let history = h
                    .reload
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .last_reload()
                    .cloned();
                assert_eq!(
                    h.service.patch(actor(), request),
                    Err(ConfigEditError::FileInvalid {
                        diagnostics: vec![]
                    })
                );
                assert_eq!(h.calls(), ["read", "rejected:config_file_invalid"]);
                assert!(Arc::ptr_eq(&before, &h.cell.current()));
                assert_eq!(
                    h.reload
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .last_reload()
                        .cloned(),
                    history
                );
                assert_eq!(
                    *h.files
                        .bytes
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner),
                    [0xff]
                );
                assert!(
                    h.files
                        .snapshot_at_replace
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .is_none()
                );
                continue;
            }
            "stale" => {
                request.revision = revision_of(b"candidate");
                // Even malformed original syntax must not be parsed before revision comparison.
                *h.document
                    .original
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) =
                    Err(diagnostic(ConfigError::SyntaxError, ""));
                (ConfigEditError::Conflict, vec!["read", "revision:original"])
            }
            "invalid_original" => {
                let diagnostics = diagnostic(ConfigError::SyntaxError, "");
                *h.document
                    .original
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = Err(diagnostics.clone());
                (
                    ConfigEditError::FileInvalid { diagnostics },
                    vec!["read", "revision:original", "parse:original"],
                )
            }
            "original_tls" => {
                *h.tls
                    .bad_original
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = true;
                let mut calls = original_prepared_calls();
                calls.push("tls-build");
                (
                    ConfigEditError::FileInvalid {
                        diagnostics: vec![ConfigDiagnostic {
                            error: ConfigError::TlsFileInvalid,
                            key: "management.key_file".to_owned(),
                            message: "pair mismatch".to_owned(),
                        }],
                    },
                    calls,
                )
            }
            "restart" => {
                let mut original = transaction_settings();
                original.management.listen = addr(8081);
                original.timeouts.handshake = Duration::from_secs(11);
                *h.document
                    .original
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = Ok(candidate(original));
                (
                    ConfigEditError::RestartPending {
                        config_keys: vec!["management.listen".to_owned()],
                    },
                    original_prepared_calls(),
                )
            }
            "edit" => {
                let diagnostics = diagnostic(ConfigError::ValueOutOfRange, "timeouts.close_secs");
                *h.document
                    .edit_error
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(diagnostics.clone());
                let mut calls = original_prepared_calls();
                calls.push("edit:timeouts.close_secs");
                (ConfigEditError::ConfigInvalid { diagnostics }, calls)
            }
            "candidate" => {
                let diagnostics = diagnostic(ConfigError::RelationViolated, "limits.waiting");
                *h.document
                    .next
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = Err(diagnostics.clone());
                let mut calls = original_prepared_calls();
                calls.extend(["edit:timeouts.close_secs", "parse:candidate"]);
                (ConfigEditError::ConfigInvalid { diagnostics }, calls)
            }
            "write" | "replace_conflict" => {
                let (replace, error) = if case == "write" {
                    (
                        ConfigReplaceError::WriteFailed,
                        ConfigEditError::WriteFailed,
                    )
                } else {
                    (ConfigReplaceError::Conflict, ConfigEditError::Conflict)
                };
                *h.files
                    .replace_error
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(replace);
                let mut calls = candidate_prepared_calls();
                calls.extend(["revision:candidate", "replace"]);
                (error, calls)
            }
            "success" => {
                let before = h.cell.current();
                let result = h
                    .service
                    .patch(actor(), request)
                    .unwrap_or_else(|error| panic!("{error:?}"));
                let mut calls = candidate_prepared_calls();
                calls.extend([
                    "revision:candidate",
                    "replace",
                    "wall",
                    "reload_applied",
                    "config_edited",
                ]);
                assert_eq!(h.calls(), calls, "{case}");
                assert_eq!(result.applied, ["timeouts.close_secs"]);
                assert_eq!(result.revision, revision_of(b"candidate"));
                assert_eq!(
                    result.as_of,
                    SystemTime::UNIX_EPOCH + Duration::from_secs(200)
                );
                assert!(Arc::ptr_eq(
                    &before,
                    h.files
                        .snapshot_at_replace
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .as_ref()
                        .unwrap_or_else(|| panic!("replace must observe the old snapshot"))
                ));
                let after = h.cell.current();
                assert!(!Arc::ptr_eq(&before, &after));
                assert_eq!(after.settings.timeouts.close, Duration::from_secs(7));
                assert_eq!(
                    *h.files
                        .bytes
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner),
                    b"candidate"
                );
                let observations = h
                    .sink
                    .observations
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                assert_eq!(observations.len(), 2);
                assert_eq!(
                    observations[0].0,
                    Event::ReloadApplied {
                        trigger: ReloadTrigger::ConfigEdit,
                        applied: vec!["timeouts.close_secs".to_owned()],
                    }
                );
                assert_eq!(
                    observations[1].0,
                    Event::ConfigEdited {
                        admin_account: actor().admin_account,
                        peer_ip: actor().peer_ip,
                        config_keys: vec!["timeouts.close_secs".to_owned()],
                        revision: result.revision,
                    }
                );
                assert!(
                    observations
                        .iter()
                        .all(|(_, seen)| Arc::ptr_eq(&after, seen))
                );
                assert_eq!(
                    h.reload
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .last_reload(),
                    Some(&LastReload {
                        at: result.as_of,
                        result: ReloadResult::Applied,
                        trigger: ReloadTrigger::ConfigEdit,
                        config_keys: result.applied,
                    })
                );
                continue;
            }
            _ => unreachable!(),
        };
        let before = h.cell.current();
        let history = h
            .reload
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .last_reload()
            .cloned();
        let (error, calls) = expected;
        assert_eq!(
            h.service.patch(actor(), request),
            Err(error.clone()),
            "{case}"
        );
        let mut calls: Vec<String> = calls.into_iter().map(str::to_owned).collect();
        calls.push(format!("rejected:{}", error.code()));
        assert_eq!(h.calls(), calls, "{case}");
        h.assert_rejected_unchanged(&before, &history, &error);
    }
}

/// spec: management.md:MGT-036 partial
#[test]
fn rejects_candidate_before_replace() {
    for invalid_tls in [false, true] {
        let h = Harness::ordinary();
        h.seed_history();
        let diagnostics = if invalid_tls {
            let mut next = transaction_settings();
            next.timeouts.close = Duration::from_secs(7);
            next.management
                .tls
                .as_mut()
                .unwrap_or_else(|| panic!("fixture requires management TLS"))
                .cert_file = PathBuf::from("bad.crt");
            *h.document
                .next
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Ok(candidate(next));
            vec![ConfigDiagnostic {
                error: ConfigError::TlsFileInvalid,
                key: "management.key_file".to_owned(),
                message: "pair mismatch".to_owned(),
            }]
        } else {
            let diagnostics = diagnostic(ConfigError::IpItemInvalid, "relay.tcp_viewer.allow[2]");
            *h.document
                .next
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Err(diagnostics.clone());
            diagnostics
        };
        let before = h.cell.current();
        let history = h
            .reload
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .last_reload()
            .cloned();
        let error = ConfigEditError::ConfigInvalid { diagnostics };
        assert_eq!(h.service.patch(actor(), patch()), Err(error.clone()));
        let mut calls = original_prepared_calls();
        calls.extend(["edit:timeouts.close_secs", "parse:candidate"]);
        if invalid_tls {
            calls.extend(["tls-read:bad.crt", "tls-read:management.key", "tls-build"]);
        }
        calls.push("rejected:config_invalid");
        assert_eq!(h.calls(), calls);
        assert!(
            h.files
                .snapshot_at_replace
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .is_none()
        );
        h.assert_rejected_unchanged(&before, &history, &error);
    }
}

/// spec: management.md:MGT-039 partial
#[test]
fn same_file_value_with_pending_runtime_delta_has_no_commit_or_history_or_event() {
    let running = transaction_settings();
    let mut pending = running.clone();
    pending.timeouts.close = Duration::from_secs(7);
    let h = Harness::new(running, pending.clone(), pending, &[], true);
    h.seed_history();
    let before = h.cell.current();
    let history = h
        .reload
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .last_reload()
        .cloned();
    let result = h
        .service
        .patch(actor(), patch())
        .unwrap_or_else(|error| panic!("{error:?}"));
    assert_eq!(result.applied, Vec::<String>::new());
    assert_eq!(result.revision, revision_of(b"original"));
    let mut calls = original_prepared_calls();
    calls.extend([
        "edit:timeouts.close_secs",
        "parse:original",
        "tls-read:management.crt",
        "tls-read:management.key",
        "wall",
    ]);
    assert_eq!(h.calls(), calls);
    assert!(Arc::ptr_eq(&before, &h.cell.current()));
    assert_eq!(
        h.cell.current().settings.timeouts.close,
        Duration::from_secs(5)
    );
    assert_eq!(
        h.reload
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .last_reload()
            .cloned(),
        history
    );
    assert!(
        h.sink
            .observations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_empty()
    );
    assert!(
        h.files
            .snapshot_at_replace
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_none()
    );
    assert_eq!(
        *h.files
            .bytes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
        b"original"
    );
}

/// spec: management.md:MGT-031 partial
/// spec: operations.md:OPS-059 partial
/// spec: operations.md:OPS-060 partial
#[test]
fn success_audits_only_requested_file_changes_but_applies_entire_prepared_candidate() {
    let running = transaction_settings();
    let mut pending = running.clone();
    pending.timeouts.handshake = Duration::from_secs(11);
    let mut next = pending.clone();
    next.timeouts.close = Duration::from_secs(7);
    next.management
        .tls
        .as_mut()
        .unwrap_or_else(|| panic!("fixture requires management TLS"))
        .cert_file = PathBuf::from("next.crt");
    let h = Harness::new(running, pending, next, &["timeouts.close_secs"], false);
    let result = h
        .service
        .patch(
            actor(),
            ConfigPatch {
                revision: revision_of(b"original"),
                changes: vec![
                    change("timeouts.handshake_secs", 11),
                    change("timeouts.close_secs", 7),
                ],
            },
        )
        .unwrap_or_else(|error| panic!("{error:?}"));
    let applied = vec![
        "timeouts.handshake_secs".to_owned(),
        "timeouts.close_secs".to_owned(),
        "management.cert_file".to_owned(),
    ];
    assert_eq!(result.applied, applied);
    assert_eq!(
        h.calls(),
        [
            "read",
            "revision:original",
            "parse:original",
            "tls-read:management.crt",
            "tls-read:management.key",
            "edit:timeouts.handshake_secs",
            "edit:timeouts.close_secs",
            "parse:candidate",
            "tls-read:next.crt",
            "tls-read:management.key",
            "tls-build",
            "revision:candidate",
            "replace",
            "wall",
            "reload_applied",
            "config_edited",
        ]
    );
    let after = h.cell.current();
    assert_eq!(after.settings.timeouts.handshake, Duration::from_secs(11));
    assert_eq!(after.settings.timeouts.close, Duration::from_secs(7));
    assert_eq!(after.management_tls, Some(2));
    let observations = h
        .sink
        .observations
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(
        observations
            .iter()
            .map(|(event, _)| event.clone())
            .collect::<Vec<_>>(),
        vec![
            Event::ReloadApplied {
                trigger: ReloadTrigger::ConfigEdit,
                applied: applied.clone()
            },
            Event::ConfigEdited {
                admin_account: actor().admin_account,
                peer_ip: actor().peer_ip,
                config_keys: vec!["timeouts.close_secs".to_owned()],
                revision: revision_of(b"candidate"),
            },
        ]
    );
    assert!(
        observations
            .iter()
            .all(|(_, seen)| Arc::ptr_eq(&after, seen))
    );
    assert_eq!(
        h.reload
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .last_reload()
            .unwrap_or_else(|| panic!("successful commit must record history"))
            .config_keys,
        applied
    );
}

/// spec: management.md:MGT-037 partial
#[test]
fn request_shape_and_editability_are_checked_before_disabled_switch_and_io() {
    let mut running = transaction_settings();
    running.management.config_edit = false;
    let h = Harness::new(running.clone(), running.clone(), running, &[], false);
    let mut invalid = vec![
        ConfigPatch {
            revision: "bad".to_owned(),
            changes: patch().changes,
        },
        ConfigPatch {
            revision: revision_of(b"original"),
            changes: vec![],
        },
    ];
    for changes in [
        vec![change("unknown.key", 1)],
        vec![change("management.config_edit", 1)],
        vec![ConfigChange {
            key: "management.cert_file".to_owned(),
            value: ConfigValue::String("secret".to_owned()),
        }],
        vec![ConfigChange {
            key: "timeouts.close_secs".to_owned(),
            value: ConfigValue::String("7".to_owned()),
        }],
        vec![
            change("timeouts.close_secs", 7),
            change("timeouts.close_secs", 8),
        ],
        vec![ConfigChange {
            key: "relay.ws_viewer.allowed_origins".to_owned(),
            value: ConfigValue::Strings(vec![]),
        }],
    ] {
        invalid.push(ConfigPatch {
            revision: revision_of(b"original"),
            changes,
        });
    }
    for request in invalid {
        assert_eq!(
            h.service.patch(actor(), request),
            Err(ConfigEditError::InvalidRequest)
        );
        assert!(h.calls().is_empty());
        assert!(
            h.sink
                .observations
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .is_empty()
        );
        assert!(
            h.reload
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .last_reload()
                .is_none()
        );
    }
}

/// spec: management.md:MGT-028 partial
#[test]
fn get_classifies_unreadable_and_invalid_files_without_changing_runtime_or_history() {
    for state in [
        ConfigFileState::InEffect,
        ConfigFileState::Pending,
        ConfigFileState::RestartRequired,
        ConfigFileState::Invalid,
        ConfigFileState::Unreadable,
    ] {
        let h = Harness::ordinary();
        h.seed_history();
        let mut original = transaction_settings();
        let (keys, diagnostics) = match state {
            ConfigFileState::InEffect => (vec![], vec![]),
            ConfigFileState::Pending => {
                original.timeouts.close = Duration::from_secs(9);
                (vec!["timeouts.close_secs".to_owned()], vec![])
            }
            ConfigFileState::RestartRequired => {
                original.management.listen = addr(8081);
                original.timeouts.close = Duration::from_secs(9);
                (vec!["management.listen".to_owned()], vec![])
            }
            ConfigFileState::Invalid => (vec![], diagnostic(ConfigError::SyntaxError, "")),
            ConfigFileState::Unreadable => {
                *h.files
                    .unreadable
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = true;
                (vec![], vec![])
            }
        };
        *h.document
            .original
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) =
            if state == ConfigFileState::Invalid {
                Err(diagnostics.clone())
            } else {
                Ok(candidate(original))
            };
        let before = h.cell.current();
        let history = h
            .reload
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .last_reload()
            .cloned();
        let view = h.service.get().unwrap_or_else(|error| panic!("{error:?}"));
        assert_eq!(view.file.state, state);
        assert_eq!(view.file.config_keys, keys);
        assert_eq!(view.file.diagnostics, diagnostics);
        assert_eq!(
            view.revision,
            if state == ConfigFileState::Unreadable {
                None
            } else {
                Some(revision_of(b"original"))
            }
        );
        assert_eq!(view.last_reload, history);
        assert!(view.edit_enabled);
        assert_eq!(view.items, config_items(&before.settings, &before.presence));
        assert!(Arc::ptr_eq(&before, &h.cell.current()));
        assert_eq!(
            h.reload
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .last_reload()
                .cloned(),
            history
        );
        assert!(
            h.sink
                .observations
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .is_empty()
        );
    }
    // A successful byte read whose parser reports unreadable still has a revision.
    let h = Harness::ordinary();
    *h.document
        .original
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) =
        Err(diagnostic(ConfigError::FileUnreadable, ""));
    let view = h.service.get().unwrap_or_else(|error| panic!("{error:?}"));
    assert_eq!(view.file.state, ConfigFileState::Unreadable);
    assert!(view.file.diagnostics.is_empty());
    assert_eq!(view.revision, Some(revision_of(b"original")));
    assert_eq!(view.last_reload, None);
}

const ORDERED_KEYS: &str = "
admin.accounts_file admin.argon2.memory_kib admin.argon2.iterations admin.argon2.parallelism
relay.tcp_viewer.enabled relay.tcp_viewer.listen relay.tcp_viewer.allow
relay.tcp_server.enabled relay.tcp_server.listen relay.tcp_server.allow
relay.ws_viewer.enabled relay.ws_viewer.listen relay.ws_viewer.allow relay.ws_viewer.allowed_origins
relay.wss_viewer.enabled relay.wss_viewer.listen relay.wss_viewer.allow relay.wss_viewer.allowed_origins relay.wss_viewer.cert_file relay.wss_viewer.key_file
mode1.enabled mode1.allow_targets mode2.enabled
limits.relay_connections limits.admin_connections limits.waiting limits.buffer_bytes limits.ws_message_bytes
timeouts.handshake_secs timeouts.tls_upgrade_secs timeouts.wait_secs timeouts.connect_secs timeouts.close_secs timeouts.shutdown_grace_secs
keepalive.tcp_idle_secs keepalive.tcp_interval_secs keepalive.tcp_probes keepalive.ws_ping_secs keepalive.ws_pong_timeout_secs
management.listen management.cert_file management.key_file management.config_edit
management.login_limit.per_account management.login_limit.per_source management.login_limit.window_secs management.login_limit.lockout_secs
management.session.idle_secs management.session.max_lifetime_secs dashboard.poll_secs dashboard.query_timeout_secs recent_closed.max_entries
status.listen log.output reload.settle_secs reload.poll_secs";

const DYNAMIC_KEYS: &str = "
relay.tcp_viewer.allow relay.tcp_server.allow relay.ws_viewer.allow relay.ws_viewer.allowed_origins
relay.wss_viewer.allow relay.wss_viewer.allowed_origins relay.wss_viewer.cert_file relay.wss_viewer.key_file mode1.allow_targets
limits.relay_connections limits.admin_connections limits.waiting limits.buffer_bytes limits.ws_message_bytes
timeouts.handshake_secs timeouts.tls_upgrade_secs timeouts.wait_secs timeouts.connect_secs timeouts.close_secs timeouts.shutdown_grace_secs
keepalive.tcp_idle_secs keepalive.tcp_interval_secs keepalive.tcp_probes keepalive.ws_ping_secs keepalive.ws_pong_timeout_secs management.cert_file management.key_file";

fn item_named<'a>(items: &'a [ConfigItem], key: &str) -> &'a ConfigItem {
    items
        .iter()
        .find(|item| item.key == key)
        .unwrap_or_else(|| panic!("missing descriptor: {key}"))
}

/// spec: management.md:MGT-028 partial
/// spec: management.md:MGT-037 partial
#[test]
fn descriptors_follow_contract_order_with_27_dynamic_and_23_enabled_editable_keys() {
    let mut all = transaction_settings();
    all.relay.ws_viewer = EntryPointSettings {
        enabled: true,
        listen: Some(addr(8000)),
        allow: Some(AllowList::default()),
        ..Default::default()
    };
    all.relay.wss_viewer = EntryPointSettings {
        enabled: true,
        listen: Some(addr(8443)),
        allow: Some(AllowList::default()),
        tls: Some(TlsFiles {
            cert_file: "wss.crt".into(),
            key_file: "wss.key".into(),
        }),
        ..Default::default()
    };
    all.mode1 = Mode1Settings {
        enabled: true,
        allow_targets: Some(TargetPolicy::default()),
    };
    let tls_paths = [
        "relay.wss_viewer.cert_file",
        "relay.wss_viewer.key_file",
        "management.cert_file",
        "management.key_file",
    ];
    for edit_switch in [true, false] {
        all.management.config_edit = edit_switch;
        let items = config_items(&all, &ConfigPresence::default());
        assert_eq!(
            items
                .iter()
                .map(|item| item.key.as_str())
                .collect::<Vec<_>>(),
            ORDERED_KEYS.split_whitespace().collect::<Vec<_>>()
        );
        assert_eq!(items.len(), 56);
        let dynamic = items
            .iter()
            .filter(|item| item.reload == ConfigReloadKind::Dynamic)
            .map(|item| item.key.as_str())
            .collect::<Vec<_>>();
        assert_eq!(dynamic, DYNAMIC_KEYS.split_whitespace().collect::<Vec<_>>());
        assert_eq!(dynamic.len(), 27);
        let editable = items
            .iter()
            .filter(|item| item.editable)
            .map(|item| item.key.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            editable,
            dynamic
                .iter()
                .copied()
                .filter(|key| !tls_paths.contains(key))
                .collect::<Vec<_>>()
        );
        assert_eq!(editable.len(), 23);
        assert!(items.iter().all(|item| item.editable
            == (item.reload == ConfigReloadKind::Dynamic
                && !tls_paths.contains(&item.key.as_str()))));
    }
    all.relay.tcp_viewer.enabled = false;
    all.relay.tcp_server.enabled = false;
    all.relay.ws_viewer.enabled = false;
    all.relay.wss_viewer.enabled = false;
    all.mode1.enabled = false;
    let items = config_items(&all, &ConfigPresence::default());
    assert_eq!(
        items
            .iter()
            .filter(|item| item.reload == ConfigReloadKind::Dynamic)
            .count(),
        27
    );
    assert_eq!(items.iter().filter(|item| item.editable).count(), 16);
    for key in [
        "relay.tcp_viewer.allow",
        "relay.tcp_server.allow",
        "relay.ws_viewer.allow",
        "relay.ws_viewer.allowed_origins",
        "relay.wss_viewer.allow",
        "relay.wss_viewer.allowed_origins",
        "mode1.allow_targets",
    ] {
        assert!(!item_named(&items, key).editable, "{key}");
    }
}

/// spec: management.md:MGT-028 partial
#[test]
fn descriptors_distinguish_absent_disabled_values_from_explicit_empty_lists_and_defaults() {
    let mut disabled = settings();
    disabled.relay.tcp_viewer = EntryPointSettings::default();
    disabled.relay.tcp_server = EntryPointSettings::default();
    let absent = config_items(&disabled, &ConfigPresence::default());
    for key in [
        "relay.tcp_viewer.listen",
        "relay.tcp_viewer.allow",
        "relay.tcp_server.listen",
        "relay.tcp_server.allow",
        "relay.ws_viewer.listen",
        "relay.ws_viewer.allow",
        "relay.ws_viewer.allowed_origins",
        "relay.wss_viewer.listen",
        "relay.wss_viewer.allow",
        "relay.wss_viewer.allowed_origins",
        "relay.wss_viewer.cert_file",
        "relay.wss_viewer.key_file",
        "mode1.allow_targets",
        "management.cert_file",
        "management.key_file",
    ] {
        assert_eq!(item_named(&absent, key).value, ConfigValue::Null, "{key}");
    }
    assert_eq!(
        item_named(&absent, "limits.relay_connections").value,
        ConfigValue::Integer(100)
    );
    assert_eq!(
        item_named(&absent, "relay.ws_viewer.enabled").value,
        ConfigValue::Bool(false)
    );
    assert_eq!(
        item_named(&absent, "management.config_edit").value,
        ConfigValue::Bool(true)
    );
    let presence = ConfigPresence {
        keys: [
            "relay.ws_viewer.allowed_origins",
            "relay.wss_viewer.allowed_origins",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>(),
        ..ConfigPresence::default()
    };
    disabled.relay.ws_viewer.allow = Some(AllowList::default());
    disabled.relay.wss_viewer.allow = Some(AllowList::default());
    disabled.mode1.allow_targets = Some(TargetPolicy::default());
    let explicit = config_items(&disabled, &presence);
    for key in [
        "relay.ws_viewer.allow",
        "relay.ws_viewer.allowed_origins",
        "relay.wss_viewer.allow",
        "relay.wss_viewer.allowed_origins",
        "mode1.allow_targets",
    ] {
        let item = item_named(&explicit, key);
        assert_eq!(item.value, ConfigValue::Strings(vec![]), "{key}");
        assert!(!item.editable);
    }
}

/// spec: management.md:MGT-028 partial
/// spec: management.md:MGT-036 partial
#[test]
fn descriptors_expose_all_integer_ranges_and_no_bounds_for_other_types() {
    let items = config_items(&settings(), &ConfigPresence::default());
    let expected = [
        ("admin.argon2.memory_kib", 19_456, 1_048_576),
        ("admin.argon2.iterations", 2, 10),
        ("admin.argon2.parallelism", 1, 16),
        ("limits.relay_connections", 2, 65_535),
        ("limits.admin_connections", 1, 1024),
        ("limits.waiting", 1, 65_535),
        ("limits.buffer_bytes", 16_384, 4_194_304),
        ("limits.ws_message_bytes", 65_536, 16_777_216),
        ("timeouts.handshake_secs", 1, 120),
        ("timeouts.tls_upgrade_secs", 1, 120),
        ("timeouts.wait_secs", 10, 86_400),
        ("timeouts.connect_secs", 1, 120),
        ("timeouts.close_secs", 1, 120),
        ("timeouts.shutdown_grace_secs", 0, 600),
        ("keepalive.tcp_idle_secs", 10, 3600),
        ("keepalive.tcp_interval_secs", 1, 300),
        ("keepalive.tcp_probes", 1, 20),
        ("keepalive.ws_ping_secs", 5, 300),
        ("keepalive.ws_pong_timeout_secs", 5, 300),
        ("management.login_limit.per_account", 1, 100),
        ("management.login_limit.per_source", 1, 100),
        ("management.login_limit.window_secs", 60, 86_400),
        ("management.login_limit.lockout_secs", 60, 86_400),
        ("management.session.idle_secs", 300, 28_800),
        ("management.session.max_lifetime_secs", 3600, 259_200),
        ("dashboard.poll_secs", 1, 60),
        ("dashboard.query_timeout_secs", 1, 60),
        ("recent_closed.max_entries", 0, 10_000),
        ("reload.settle_secs", 1, 60),
        ("reload.poll_secs", 5, 600),
    ];
    assert_eq!(
        items
            .iter()
            .filter(|item| matches!(item.value, ConfigValue::Integer(_)))
            .count(),
        expected.len()
    );
    for (key, min, max) in expected {
        let item = item_named(&items, key);
        assert_eq!((item.min, item.max), (Some(min), Some(max)), "{key}");
    }
    for item in items
        .iter()
        .filter(|item| !matches!(item.value, ConfigValue::Integer(_)))
    {
        assert_eq!((item.min, item.max), (None, None), "{}", item.key);
    }
}

/// spec: management.md:MGT-028 partial
#[test]
fn descriptors_keep_lone_disabled_tls_paths_without_inventing_omitted_paths() {
    let running = settings();
    for key in ["relay.wss_viewer.cert_file", "relay.wss_viewer.key_file"] {
        let mut presence = ConfigPresence::default();
        let path = PathBuf::from("config/inactive.pem");
        presence.keys.insert(key.to_owned());
        presence.inactive_paths.insert(key.to_owned(), path.clone());
        let items = config_items(&running, &presence);
        let present = item_named(&items, key);
        assert_eq!(
            present.value,
            ConfigValue::String(path.display().to_string())
        );
        assert_eq!(present.reload, ConfigReloadKind::Dynamic);
        assert!(!present.editable);
        let omitted = if key.ends_with(".cert_file") {
            "relay.wss_viewer.key_file"
        } else {
            "relay.wss_viewer.cert_file"
        };
        assert_eq!(item_named(&items, omitted).value, ConfigValue::Null);
        assert_eq!(
            item_named(&items, "relay.wss_viewer.enabled").value,
            ConfigValue::Bool(false)
        );
    }
}
