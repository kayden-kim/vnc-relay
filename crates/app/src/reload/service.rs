//! Verification and swap of a reloaded configuration (file-watch-v1 §4 steps 4–9).

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use vnc_relay_domain::entry_point::{EntryPoint, Listener, Role};

use super::{
    ConfigCandidate, ConfigFileState, ConfigPresence, FileClassification, LastReload,
    PrepareReloadError, ReloadResult, ReloadTrigger,
};
use crate::events::{Event, Phase};
use crate::ports::EventSink;
use crate::reload::ports::{ReloadFileStore, ReloadTicketKeyStore, ReloadTlsStore, TlsBuildError};
use crate::reload::snapshot::{ReloadCell, ReloadSnapshot};
use crate::settings::{
    ConfigDiagnostic, ConfigError, ConfigWarning, Settings, TicketSettings, TlsFiles,
};

/// `tls_file_invalid` detail for a certificate or key file that cannot be read.
const UNREADABLE_DETAIL: &str = "file unreadable";

/// `TlsBuildError::detail` for a malformed certificate. Other build causes are reported
/// on the key file key.
const CERTIFICATE_FORMAT_DETAIL: &str = "certificate format";

/// Result of one reload attempt (file-watch-v1 §4 step 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadOutcome {
    /// Changed reload targets were swapped in; `reload_applied` was emitted.
    Applied,
    /// The candidate was refused; the running values are kept.
    Rejected,
    /// A key outside the reload targets differs; nothing was swapped.
    RestartRequired,
    /// No reload target changed; nothing was swapped and no result event was emitted.
    Unchanged,
    /// The generation became stale or shutdown began; no event and no state change.
    Discarded,
}

/// Verifies a reloaded config candidate and swaps in the changed reload targets.
///
/// Follows file-watch-v1 §4: the TLS files of enabled listeners are built (step 6), the
/// non-target keys are compared (step 7), the generation is checked again (step 8), and
/// only then is the shared snapshot replaced (step 9). One call emits at most one result
/// event.
pub struct ReloadService<A> {
    /// Settings the process runs with. Replaced only by an applied candidate.
    running: Arc<Settings>,
    presence: Arc<ConfigPresence>,
    last_reload: Option<LastReload>,
    cell: Arc<ReloadCell<A>>,
    files: Arc<dyn ReloadFileStore>,
    tls: Arc<dyn ReloadTlsStore<A>>,
    /// Parses ticket public key files of enabled roles (config-v4 §1.11).
    ticket_keys: Arc<dyn ReloadTicketKeyStore>,
    events: Arc<dyn EventSink>,
    /// File paths of the last candidate that parsed (enabled TLS and ticket roles).
    candidate_paths: Vec<PathBuf>,
    /// Content hashes of the running WSS certificate and key. `None` when they could not be
    /// read at the last check, so the next check counts them as changed.
    wss_hashes: Option<FileHashes>,
    /// Content hashes of the running management certificate and key (same rule as WSS).
    management_hashes: Option<FileHashes>,
    /// Test seam (file-watch-v1 §4.3): runs once, right before the next candidate is validated.
    /// Only tests arm it; the product never does.
    before_validation: Option<Box<dyn FnOnce() + Send>>,
}

impl<A> ReloadService<A> {
    /// Starts from the running settings.
    ///
    /// Reads the running certificate and key bytes once to store baseline content hashes.
    /// Emits no event.
    pub fn new(
        running: ConfigCandidate,
        cell: Arc<ReloadCell<A>>,
        files: Arc<dyn ReloadFileStore>,
        tls: Arc<dyn ReloadTlsStore<A>>,
        ticket_keys: Arc<dyn ReloadTicketKeyStore>,
        events: Arc<dyn EventSink>,
    ) -> Self {
        let ConfigCandidate {
            settings: running,
            presence,
        } = running;
        let wss_hashes = baseline_hashes(files.as_ref(), TlsSlot::Wss.files(&running));
        let management_hashes =
            baseline_hashes(files.as_ref(), TlsSlot::Management.files(&running));
        Self {
            running: Arc::new(running),
            presence: Arc::new(presence),
            last_reload: None,
            cell,
            files,
            tls,
            ticket_keys,
            events,
            candidate_paths: Vec::new(),
            wss_hashes,
            management_hashes,
            before_validation: None,
        }
    }

    pub fn running(&self) -> &Settings {
        &self.running
    }

    pub fn running_presence(&self) -> &ConfigPresence {
        &self.presence
    }

    pub fn last_reload(&self) -> Option<&LastReload> {
        self.last_reload.as_ref()
    }
    /// Files of enabled TLS listeners and ticket roles, including the last parsed candidate
    /// (config-v4 §1.8), without duplicates.
    pub fn watched_paths(&self) -> Vec<PathBuf> {
        let mut paths = reload_paths(&self.running);
        for path in &self.candidate_paths {
            if !paths.contains(path) {
                paths.push(path.clone());
            }
        }
        paths
    }

    /// Test seam (file-watch-v1 §4.3): runs `action` once, right before the next candidate is
    /// validated. Only tests arm it; the product never does.
    pub fn arm_before_validation(&mut self, action: Box<dyn FnOnce() + Send>) {
        self.before_validation = Some(action);
    }
}

/// A fully validated candidate; commit does not read files or rebuild TLS.
pub struct PreparedReload<A> {
    snapshot: Arc<ReloadSnapshot<A>>,
    wss_hashes: Option<FileHashes>,
    management_hashes: Option<FileHashes>,
    applied: Vec<String>,
    event_keys: Vec<String>,
    history_keys: Vec<String>,
    warnings: Vec<ConfigWarning>,
}

impl<A> PreparedReload<A> {
    pub fn applied_keys(&self) -> &[String] {
        &self.applied
    }
}

impl<A: Clone> ReloadService<A> {
    pub fn prepare_candidate(
        &self,
        mut candidate: ConfigCandidate,
    ) -> Result<PreparedReload<A>, PrepareReloadError> {
        let tls = self.load_tls(&candidate.settings).map_err(|rejection| {
            PrepareReloadError::Invalid(vec![ConfigDiagnostic {
                error: ConfigError::TlsFileInvalid,
                key: rejection.config_key.to_owned(),
                message: rejection.detail.to_owned(),
            }])
        })?;
        super::ticket_keys::load(
            &mut candidate.settings,
            self.files.as_ref(),
            self.ticket_keys.as_ref(),
        )
        .map_err(PrepareReloadError::Invalid)?;
        let restart = restart_required_keys(&self.running, &candidate.settings);
        if !restart.is_empty() {
            return Err(PrepareReloadError::RestartRequired(restart));
        }
        let mut applied = dynamic_changed_keys(&self.running, &candidate.settings);
        for key in tls.changed {
            if !applied.iter().any(|existing| existing == key) {
                applied.push(key.to_owned());
            }
        }
        for role in [Role::Viewer, Role::Server] {
            let before = self.running.relay.ticket.role(role);
            let after = candidate.settings.relay.ticket.role(role);
            let key = super::ticket_keys::file_key(role);
            if before.enabled
                && after.enabled
                && before.public_keys_file == after.public_keys_file
                && before.keys != after.keys
                && !applied.contains(&key)
            {
                applied.push(key);
            }
        }
        applied.sort_by_key(|key| DYNAMIC_KEYS.iter().position(|known| known == key));
        let warnings = candidate
            .settings
            .warnings()
            .into_iter()
            .filter(|warning| match warning {
                ConfigWarning::AllowAllSource { entry_point, .. } => {
                    applied.iter().any(|key| key == allow_key(*entry_point))
                }
                ConfigWarning::TrustAllProxies { listener, .. } => applied
                    .iter()
                    .any(|key| key == &format!("relay.{}.trusted_proxies", listener.name())),
                ConfigWarning::AllowAllTarget { .. } => {
                    applied.iter().any(|key| key == "mode1.allow_targets")
                }
                ConfigWarning::TicketTcpBypass { role }
                | ConfigWarning::TicketPlaintextWs { role } => applied
                    .iter()
                    .any(|key| key == &format!("{}.enabled", TicketSettings::table(*role))),
                _ => false,
            })
            .collect();
        Ok(PreparedReload {
            snapshot: Arc::new(ReloadSnapshot::new(
                candidate,
                tls.wss_acceptor,
                tls.management_acceptor,
            )),
            wss_hashes: tls.wss_hashes,
            management_hashes: tls.management_hashes,
            event_keys: applied.clone(),
            history_keys: applied.clone(),
            applied,
            warnings,
        })
    }

    pub fn classify_file(
        &self,
        parsed: Option<Result<ConfigCandidate, Vec<ConfigDiagnostic>>>,
    ) -> FileClassification {
        let (state, config_keys, diagnostics) = match parsed {
            None => (ConfigFileState::Unreadable, Vec::new(), Vec::new()),
            Some(Err(diagnostics))
                if diagnostics
                    .iter()
                    .any(|d| d.error == ConfigError::FileUnreadable) =>
            {
                (ConfigFileState::Unreadable, Vec::new(), Vec::new())
            }
            Some(Err(diagnostics)) => (ConfigFileState::Invalid, Vec::new(), diagnostics),
            Some(Ok(candidate)) => match self.prepare_candidate(candidate) {
                Err(PrepareReloadError::Invalid(diagnostics)) => {
                    (ConfigFileState::Invalid, Vec::new(), diagnostics)
                }
                Err(PrepareReloadError::RestartRequired(keys)) => {
                    (ConfigFileState::RestartRequired, keys, Vec::new())
                }
                Ok(prepared) => {
                    // File state describes config values, not key-file-only rotations.
                    let keys = dynamic_changed_keys(&self.running, &prepared.snapshot.settings);
                    let applied: Vec<_> = prepared
                        .applied
                        .into_iter()
                        .filter(|key| !key.starts_with("relay.ticket.") || keys.contains(key))
                        .collect();
                    if applied.is_empty() {
                        (ConfigFileState::InEffect, Vec::new(), Vec::new())
                    } else {
                        (ConfigFileState::Pending, applied, Vec::new())
                    }
                }
            },
        };
        FileClassification {
            state,
            config_keys,
            diagnostics,
        }
    }

    pub fn commit_prepared(
        &mut self,
        prepared: PreparedReload<A>,
        trigger: ReloadTrigger,
        at: SystemTime,
    ) -> Vec<String> {
        if prepared.applied.is_empty() {
            return Vec::new();
        }
        let PreparedReload {
            snapshot,
            wss_hashes,
            management_hashes,
            applied,
            event_keys,
            history_keys,
            warnings,
        } = prepared;
        // All accepted settings and TLS handles are published in one generation.
        let settings = Arc::clone(&snapshot.settings);
        let presence = Arc::clone(&snapshot.presence);
        let event = Event::ReloadApplied {
            trigger,
            applied: event_keys,
        };
        let history = LastReload {
            at,
            result: ReloadResult::Applied,
            trigger,
            config_keys: history_keys,
        };
        self.cell.publish(snapshot);
        self.running = settings;
        self.presence = presence;
        self.wss_hashes = wss_hashes;
        self.management_hashes = management_hashes;
        for warning in warnings {
            self.events.record(&Event::ConfigWarning {
                warning,
                phase: Phase::Reload,
            });
        }
        self.events.record(&event);
        self.last_reload = Some(history);
        applied
    }

    /// Watcher-only transaction. Shutdown/generation veto does not affect PATCH commits.
    pub fn handle_config(
        &mut self,
        parsed: Result<ConfigCandidate, Vec<ConfigDiagnostic>>,
        is_current: &dyn Fn() -> bool,
        at: SystemTime,
    ) -> ReloadOutcome {
        if !is_current() {
            return ReloadOutcome::Discarded;
        }
        if let Some(action) = self.before_validation.take() {
            action();
        }
        let paths = parsed
            .as_ref()
            .ok()
            .map(|candidate| reload_paths(&candidate.settings));
        let prepared = match parsed {
            Ok(candidate) => self.prepare_candidate(candidate),
            Err(diagnostics) => Err(PrepareReloadError::Invalid(diagnostics)),
        };
        if !is_current() {
            return ReloadOutcome::Discarded;
        }
        if let Some(paths) = paths {
            self.candidate_paths = paths;
        }
        match prepared {
            Ok(mut prepared) if prepared.applied.is_empty() => {
                if *prepared.snapshot.presence != *self.presence {
                    let snapshot = Arc::make_mut(&mut prepared.snapshot);
                    snapshot.settings = Arc::clone(&self.running);
                    self.presence = Arc::clone(&snapshot.presence);
                    self.cell.publish(prepared.snapshot);
                }
                ReloadOutcome::Unchanged
            }
            Ok(prepared) => {
                self.commit_prepared(prepared, ReloadTrigger::FileWatch, at);
                ReloadOutcome::Applied
            }
            Err(PrepareReloadError::Invalid(diagnostics)) => {
                let (reason, config_key) = rejection_of(&diagnostics);
                let detail = diagnostics
                    .first()
                    .filter(|d| {
                        matches!(
                            d.error,
                            ConfigError::TlsFileInvalid | ConfigError::TicketKeysInvalid
                        )
                    })
                    .map(|d| d.message.clone());
                self.events.record(&Event::ReloadRejected {
                    reason: reason.to_owned(),
                    config_key,
                    detail,
                });
                self.last_reload = Some(LastReload {
                    at,
                    result: ReloadResult::Rejected,
                    trigger: ReloadTrigger::FileWatch,
                    config_keys: Vec::new(),
                });
                ReloadOutcome::Rejected
            }
            Err(PrepareReloadError::RestartRequired(keys)) => {
                self.events.record(&Event::ReloadRestartRequired {
                    config_keys: keys.clone(),
                });
                self.last_reload = Some(LastReload {
                    at,
                    result: ReloadResult::RestartRequired,
                    trigger: ReloadTrigger::FileWatch,
                    config_keys: keys,
                });
                ReloadOutcome::RestartRequired
            }
        }
    }

    pub fn handle_unreadable(
        &mut self,
        is_current: &dyn Fn() -> bool,
        at: SystemTime,
    ) -> ReloadOutcome {
        self.handle_config(
            Err(vec![ConfigDiagnostic {
                error: ConfigError::FileUnreadable,
                key: String::new(),
                message: "configuration file unreadable".to_owned(),
            }]),
            is_current,
            at,
        )
    }

    /// Loads both TLS listeners of the candidate. Rejects on the first failure.
    fn load_tls(&self, candidate: &Settings) -> Result<TlsLoad<A>, TlsRejection> {
        let current = self.cell.current();
        let wss = self.load_slot(TlsSlot::Wss, candidate, &current)?;
        let management = self.load_slot(TlsSlot::Management, candidate, &current)?;
        let mut changed = wss.changed;
        changed.extend(management.changed);
        Ok(TlsLoad {
            wss_acceptor: wss.acceptor,
            management_acceptor: management.acceptor,
            wss_hashes: wss.hashes,
            management_hashes: management.hashes,
            changed,
        })
    }

    /// Reads and checks the certificate and key of one listener.
    ///
    /// Each file is read on every call. A file is changed when its path or content hash
    /// differs from the running value. The acceptor is rebuilt only when a file changed
    /// or the running acceptor is missing. Otherwise the running acceptor is cloned.
    fn load_slot(
        &self,
        slot: TlsSlot,
        candidate: &Settings,
        current: &ReloadSnapshot<A>,
    ) -> Result<SlotLoad<A>, TlsRejection> {
        let Some(files) = slot.files(candidate) else {
            return Ok(SlotLoad {
                acceptor: None,
                hashes: None,
                changed: Vec::new(),
            });
        };
        let cert_pem = self
            .files
            .read(&files.cert_file)
            .map_err(|_| TlsRejection {
                config_key: slot.cert_key(),
                detail: UNREADABLE_DETAIL,
            })?;
        let key_pem = self.files.read(&files.key_file).map_err(|_| TlsRejection {
            config_key: slot.key_key(),
            detail: UNREADABLE_DETAIL,
        })?;
        let hashes = FileHashes {
            cert: content_hash(&cert_pem),
            key: content_hash(&key_pem),
        };

        let previous = slot.files(&self.running);
        let previous_hashes = self.running_hashes(slot);
        let cert_changed = previous.map(|tls| tls.cert_file.as_path())
            != Some(files.cert_file.as_path())
            || previous_hashes.map(|h| h.cert) != Some(hashes.cert);
        let key_changed = previous.map(|tls| tls.key_file.as_path())
            != Some(files.key_file.as_path())
            || previous_hashes.map(|h| h.key) != Some(hashes.key);

        let mut changed = Vec::new();
        if cert_changed {
            changed.push(slot.cert_key());
        }
        if key_changed {
            changed.push(slot.key_key());
        }

        let acceptor = match slot_acceptor(current, slot) {
            Some(existing) if changed.is_empty() => existing.clone(),
            _ => self
                .tls
                .build(&cert_pem, &key_pem)
                .map_err(|error| TlsRejection {
                    config_key: build_failure_key(slot, &error),
                    detail: error.detail,
                })?,
        };
        Ok(SlotLoad {
            acceptor: Some(acceptor),
            hashes: Some(hashes),
            changed,
        })
    }

    /// Running content hashes of one listener.
    fn running_hashes(&self, slot: TlsSlot) -> Option<FileHashes> {
        match slot {
            TlsSlot::Wss => self.wss_hashes,
            TlsSlot::Management => self.management_hashes,
        }
    }
}

/// The two TLS listeners that can reload (config-v4 §1.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TlsSlot {
    Wss,
    Management,
}

impl TlsSlot {
    /// Certificate and key paths when the listener is enabled and uses TLS.
    fn files(self, settings: &Settings) -> Option<&TlsFiles> {
        match self {
            Self::Wss => {
                let wss = &settings.relay.wss;
                wss.tls.as_ref().filter(|_| wss.enabled())
            }
            Self::Management => settings.management.tls.as_ref(),
        }
    }

    /// Config key of the certificate file.
    fn cert_key(self) -> &'static str {
        match self {
            Self::Wss => "relay.wss.cert_file",
            Self::Management => "management.cert_file",
        }
    }

    /// Config key of the private key file.
    fn key_key(self) -> &'static str {
        match self {
            Self::Wss => "relay.wss.key_file",
            Self::Management => "management.key_file",
        }
    }
}

/// Content hashes of one listener's certificate and key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileHashes {
    cert: u64,
    key: u64,
}

/// Result of checking one listener.
struct SlotLoad<A> {
    acceptor: Option<A>,
    hashes: Option<FileHashes>,
    /// Config keys of the changed files, certificate before key.
    changed: Vec<&'static str>,
}

/// Result of checking both listeners.
struct TlsLoad<A> {
    wss_acceptor: Option<A>,
    management_acceptor: Option<A>,
    wss_hashes: Option<FileHashes>,
    management_hashes: Option<FileHashes>,
    /// Config keys of the changed TLS files, in file-watch-v1 §4 order.
    changed: Vec<&'static str>,
}

/// A certificate or key that could not be read or built.
struct TlsRejection {
    config_key: &'static str,
    detail: &'static str,
}

/// Maps the first diagnostic to the `reload_rejected` reason and key.
///
/// An empty list is not a parse result. It is reported as a syntax error without a key.
fn rejection_of(diagnostics: &[ConfigDiagnostic]) -> (&'static str, Option<String>) {
    match diagnostics.first() {
        Some(first) => (
            first.error.code(),
            Some(first.key.clone()).filter(|key| !key.is_empty()),
        ),
        None => (ConfigError::SyntaxError.code(), None),
    }
}

/// Config key of a TLS build failure: the certificate key for a malformed certificate,
/// otherwise the key file key.
fn build_failure_key(slot: TlsSlot, error: &TlsBuildError) -> &'static str {
    if error.detail == CERTIFICATE_FORMAT_DETAIL {
        slot.cert_key()
    } else {
        slot.key_key()
    }
}

/// Returns the acceptor currently serving `slot`.
fn slot_acceptor<A>(snapshot: &ReloadSnapshot<A>, slot: TlsSlot) -> Option<&A> {
    match slot {
        TlsSlot::Wss => snapshot.wss_tls.as_ref(),
        TlsSlot::Management => snapshot.management_tls.as_ref(),
    }
}

/// Hashes the certificate and key of a listener. `None` when either file cannot be read.
fn baseline_hashes(files: &dyn ReloadFileStore, tls: Option<&TlsFiles>) -> Option<FileHashes> {
    let tls = tls?;
    let cert = read_hash(files, &tls.cert_file)?;
    let key = read_hash(files, &tls.key_file)?;
    Some(FileHashes { cert, key })
}

fn read_hash(files: &dyn ReloadFileStore, path: &Path) -> Option<u64> {
    files.read(path).ok().map(|bytes| content_hash(&bytes))
}

/// Hash used to compare file contents within one process. It is not stable across builds.
fn content_hash(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// Certificate and key paths of the enabled TLS listeners.
fn tls_paths(settings: &Settings) -> Vec<PathBuf> {
    [TlsSlot::Wss, TlsSlot::Management]
        .into_iter()
        .filter_map(|slot| slot.files(settings))
        .flat_map(|files| [files.cert_file.clone(), files.key_file.clone()])
        .collect()
}

/// All files that participate in candidate verification (config-v4 §1.8).
fn reload_paths(settings: &Settings) -> Vec<PathBuf> {
    let mut paths = tls_paths(settings);
    for role in [Role::Viewer, Role::Server] {
        let ticket = settings.relay.ticket.role(role);
        if ticket.enabled
            && let Some(path) = &ticket.public_keys_file
            && !paths.contains(path)
        {
            paths.push(path.clone());
        }
    }
    paths
}

/// Config key of an allow list (config-v4 §1.8).
fn allow_key(entry_point: EntryPoint) -> &'static str {
    match entry_point {
        EntryPoint::TcpViewer => "relay.tcp_viewer.allow",
        EntryPoint::TcpServer => "relay.tcp_server.allow",
        EntryPoint::WsViewer => "relay.ws.viewer.allow",
        EntryPoint::WsServer => "relay.ws.server.allow",
        EntryPoint::WssViewer => "relay.wss.viewer.allow",
        EntryPoint::WssServer => "relay.wss.server.allow",
    }
}

fn push_if(keys: &mut Vec<String>, differs: bool, key: &str) {
    if differs {
        keys.push(key.to_owned());
    }
}

/// Canonical config-v4 key order, including TLS content changes.
const DYNAMIC_KEYS: [&str; 43] = [
    "relay.tcp_viewer.allow",
    "relay.tcp_server.allow",
    "relay.ws.trusted_proxies",
    "relay.ws.viewer.allow",
    "relay.ws.viewer.allowed_origins",
    "relay.ws.server.allow",
    "relay.ws.server.allowed_origins",
    "relay.wss.trusted_proxies",
    "relay.wss.cert_file",
    "relay.wss.key_file",
    "relay.wss.viewer.allow",
    "relay.wss.viewer.allowed_origins",
    "relay.wss.server.allow",
    "relay.wss.server.allowed_origins",
    "relay.ticket.viewer.enabled",
    "relay.ticket.viewer.issuer",
    "relay.ticket.viewer.relay_host",
    "relay.ticket.viewer.public_keys_file",
    "relay.ticket.viewer.max_lifetime_secs",
    "relay.ticket.server.enabled",
    "relay.ticket.server.issuer",
    "relay.ticket.server.relay_host",
    "relay.ticket.server.public_keys_file",
    "relay.ticket.server.max_lifetime_secs",
    "mode1.allow_targets",
    "limits.relay_connections",
    "limits.admin_connections",
    "limits.waiting",
    "limits.buffer_bytes",
    "limits.ws_message_bytes",
    "timeouts.handshake_secs",
    "timeouts.tls_upgrade_secs",
    "timeouts.wait_secs",
    "timeouts.connect_secs",
    "timeouts.close_secs",
    "timeouts.shutdown_grace_secs",
    "keepalive.tcp_idle_secs",
    "keepalive.tcp_interval_secs",
    "keepalive.tcp_probes",
    "keepalive.ws_ping_secs",
    "keepalive.ws_pong_timeout_secs",
    "management.cert_file",
    "management.key_file",
];

/// Runtime value differences; TLS content differences are added by prepare.
pub fn dynamic_changed_keys(a: &Settings, b: &Settings) -> Vec<String> {
    let mut keys = Vec::new();
    for entry_point in EntryPoint::ALL {
        push_if(
            &mut keys,
            a.relay
                .allow(entry_point)
                .filter(|_| a.relay.enabled(entry_point))
                != b.relay
                    .allow(entry_point)
                    .filter(|_| b.relay.enabled(entry_point)),
            allow_key(entry_point),
        );
        if let (Some(before), Some(after)) =
            (a.relay.ws_role(entry_point), b.relay.ws_role(entry_point))
        {
            push_if(
                &mut keys,
                before.enabled && after.enabled && before.allowed_origins != after.allowed_origins,
                &format!("{}.allowed_origins", entry_point.config_table()),
            );
        }
    }
    for (listener, before, after) in [
        (Listener::Ws, &a.relay.ws, &b.relay.ws),
        (Listener::Wss, &a.relay.wss, &b.relay.wss),
    ] {
        push_if(
            &mut keys,
            before.enabled() && after.enabled() && before.trusted_proxies != after.trusted_proxies,
            &format!("relay.{}.trusted_proxies", listener.name()),
        );
    }
    for role in [Role::Viewer, Role::Server] {
        let before = a.relay.ticket.role(role);
        let after = b.relay.ticket.role(role);
        let table = TicketSettings::table(role);
        for (name, differs) in [
            ("enabled", before.enabled != after.enabled),
            ("issuer", before.issuer != after.issuer),
            ("relay_host", before.relay_host != after.relay_host),
            (
                "public_keys_file",
                before.public_keys_file != after.public_keys_file,
            ),
            (
                "max_lifetime_secs",
                before.max_lifetime != after.max_lifetime,
            ),
        ] {
            push_if(&mut keys, differs, &format!("{table}.{name}"));
        }
    }
    push_if(
        &mut keys,
        a.mode1.allow_targets.as_ref().filter(|_| a.mode1.enabled)
            != b.mode1.allow_targets.as_ref().filter(|_| b.mode1.enabled),
        "mode1.allow_targets",
    );
    macro_rules! changed {
        ($key:literal, $before:expr, $after:expr) => {
            push_if(&mut keys, $before != $after, $key);
        };
    }
    changed!(
        "limits.relay_connections",
        a.limits.relay_connections,
        b.limits.relay_connections
    );
    changed!(
        "limits.admin_connections",
        a.limits.admin_connections,
        b.limits.admin_connections
    );
    changed!("limits.waiting", a.limits.waiting, b.limits.waiting);
    changed!(
        "limits.buffer_bytes",
        a.limits.buffer_bytes,
        b.limits.buffer_bytes
    );
    changed!(
        "limits.ws_message_bytes",
        a.limits.ws_message_bytes,
        b.limits.ws_message_bytes
    );
    changed!(
        "timeouts.handshake_secs",
        a.timeouts.handshake,
        b.timeouts.handshake
    );
    changed!(
        "timeouts.tls_upgrade_secs",
        a.timeouts.tls_upgrade,
        b.timeouts.tls_upgrade
    );
    changed!("timeouts.wait_secs", a.timeouts.wait, b.timeouts.wait);
    changed!(
        "timeouts.connect_secs",
        a.timeouts.connect,
        b.timeouts.connect
    );
    changed!("timeouts.close_secs", a.timeouts.close, b.timeouts.close);
    changed!(
        "timeouts.shutdown_grace_secs",
        a.timeouts.shutdown_grace,
        b.timeouts.shutdown_grace
    );
    changed!(
        "keepalive.tcp_idle_secs",
        a.keepalive.tcp_idle,
        b.keepalive.tcp_idle
    );
    changed!(
        "keepalive.tcp_interval_secs",
        a.keepalive.tcp_interval,
        b.keepalive.tcp_interval
    );
    changed!(
        "keepalive.tcp_probes",
        a.keepalive.tcp_probes,
        b.keepalive.tcp_probes
    );
    changed!(
        "keepalive.ws_ping_secs",
        a.keepalive.ws_ping,
        b.keepalive.ws_ping
    );
    changed!(
        "keepalive.ws_pong_timeout_secs",
        a.keepalive.ws_pong_timeout,
        b.keepalive.ws_pong_timeout
    );
    for slot in [TlsSlot::Wss, TlsSlot::Management] {
        if let (Some(before), Some(after)) = (slot.files(a), slot.files(b)) {
            push_if(
                &mut keys,
                before.cert_file != after.cert_file,
                slot.cert_key(),
            );
            push_if(&mut keys, before.key_file != after.key_file, slot.key_key());
        }
    }
    keys.sort_by_key(|key| DYNAMIC_KEYS.iter().position(|known| known == key));
    keys
}

/// Config keys whose candidate value differs from the running value, excluding reload
/// targets (config-v4 §1.8, file-watch-v1 §4 step 7).
///
/// The management certificate is checked by presence only: HTTP and HTTPS switch need a
/// restart. WSS TLS presence follows whether either WSS role is enabled.
pub fn restart_required_keys(running: &Settings, candidate: &Settings) -> Vec<String> {
    let mut keys = Vec::new();
    for listener in Listener::ALL {
        if let Some(role) = listener.fixed_role()
            && let Some(entry) = EntryPoint::from_parts(listener, role)
        {
            push_if(
                &mut keys,
                running.relay.enabled(entry) != candidate.relay.enabled(entry),
                &format!("{}.enabled", entry.config_table()),
            );
        }
        push_if(
            &mut keys,
            running.relay.listen(listener) != candidate.relay.listen(listener),
            &format!("relay.{}.listen", listener.name()),
        );
        for entry in EntryPoint::ALL {
            if entry.listener() == listener && listener.fixed_role().is_none() {
                push_if(
                    &mut keys,
                    running.relay.enabled(entry) != candidate.relay.enabled(entry),
                    &format!("{}.enabled", entry.config_table()),
                );
            }
        }
    }

    let (a, b) = (running, candidate);
    push_if(
        &mut keys,
        a.mode1.enabled != b.mode1.enabled,
        "mode1.enabled",
    );
    push_if(
        &mut keys,
        a.mode2.enabled != b.mode2.enabled,
        "mode2.enabled",
    );

    push_if(
        &mut keys,
        a.admin.accounts_file != b.admin.accounts_file,
        "admin.accounts_file",
    );
    push_if(
        &mut keys,
        a.admin.argon2.memory_kib != b.admin.argon2.memory_kib,
        "admin.argon2.memory_kib",
    );
    push_if(
        &mut keys,
        a.admin.argon2.iterations != b.admin.argon2.iterations,
        "admin.argon2.iterations",
    );
    push_if(
        &mut keys,
        a.admin.argon2.parallelism != b.admin.argon2.parallelism,
        "admin.argon2.parallelism",
    );

    push_if(
        &mut keys,
        a.management.listen != b.management.listen,
        "management.listen",
    );
    push_if(
        &mut keys,
        a.management.tls.is_some() != b.management.tls.is_some(),
        "management.cert_file",
    );
    push_if(
        &mut keys,
        a.management.config_edit != b.management.config_edit,
        "management.config_edit",
    );
    push_if(
        &mut keys,
        a.management.login_limit.per_account != b.management.login_limit.per_account,
        "management.login_limit.per_account",
    );
    push_if(
        &mut keys,
        a.management.login_limit.per_source != b.management.login_limit.per_source,
        "management.login_limit.per_source",
    );
    push_if(
        &mut keys,
        a.management.login_limit.window != b.management.login_limit.window,
        "management.login_limit.window_secs",
    );
    push_if(
        &mut keys,
        a.management.login_limit.lockout != b.management.login_limit.lockout,
        "management.login_limit.lockout_secs",
    );
    push_if(
        &mut keys,
        a.management.session.idle != b.management.session.idle,
        "management.session.idle_secs",
    );
    push_if(
        &mut keys,
        a.management.session.max_lifetime != b.management.session.max_lifetime,
        "management.session.max_lifetime_secs",
    );

    push_if(
        &mut keys,
        a.dashboard.poll != b.dashboard.poll,
        "dashboard.poll_secs",
    );
    push_if(
        &mut keys,
        a.dashboard.query_timeout != b.dashboard.query_timeout,
        "dashboard.query_timeout_secs",
    );
    push_if(
        &mut keys,
        a.recent_closed.max_entries != b.recent_closed.max_entries,
        "recent_closed.max_entries",
    );
    push_if(
        &mut keys,
        a.recent_logs.max_entries != b.recent_logs.max_entries,
        "recent_logs.max_entries",
    );
    push_if(
        &mut keys,
        a.status.listen != b.status.listen,
        "status.listen",
    );
    push_if(&mut keys, a.log.output != b.log.output, "log.output");
    push_if(
        &mut keys,
        a.reload.settle != b.reload.settle,
        "reload.settle_secs",
    );
    push_if(
        &mut keys,
        a.reload.poll != b.reload.poll,
        "reload.poll_secs",
    );
    keys
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::io;
    use std::net::{IpAddr, SocketAddr};
    use std::sync::{Mutex, PoisonError};
    use std::time::Duration;

    use vnc_relay_domain::allow_list::AllowList;
    use vnc_relay_domain::allow_list::IpItem;

    use super::*;
    use crate::settings::{
        AdminSessionSettings, AdminSettings, Argon2Settings, DashboardSettings, EntryPointSettings,
        KeepaliveSettings, LimitSettings, LogOutput, LogSettings, LoginLimitSettings,
        Mode1Settings, Mode2Settings, RecentClosedSettings, RecentLogsSettings, RelaySettings,
        ReloadSettings, StatusSettings, TicketSettings, TimeoutSettings, WsListenerSettings,
        WsRoleSettings,
    };

    /// In-memory file store. Tests change file contents through `put`.
    #[derive(Default)]
    struct FakeFiles {
        files: Mutex<HashMap<PathBuf, Vec<u8>>>,
    }

    impl FakeFiles {
        fn put(&self, path: &str, bytes: &[u8]) {
            self.files
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(PathBuf::from(path), bytes.to_vec());
        }
    }

    impl ReloadFileStore for FakeFiles {
        fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
            self.files
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .get(path)
                .cloned()
                .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
        }
    }

    /// Builds `1` for any certificate except `bad`, which models a pair mismatch.
    struct FakeTls;

    impl ReloadTlsStore<u32> for FakeTls {
        fn build(&self, cert_pem: &[u8], _key_pem: &[u8]) -> Result<u32, TlsBuildError> {
            if cert_pem == &b"bad"[..] {
                Err(TlsBuildError {
                    detail: "pair mismatch",
                })
            } else {
                Ok(1)
            }
        }
    }

    #[derive(Default)]
    struct RecordingSink {
        events: Mutex<Vec<Event>>,
    }

    impl RecordingSink {
        fn events(&self) -> Vec<Event> {
            self.events
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }
    }

    impl EventSink for RecordingSink {
        fn record(&self, event: &Event) {
            self.events
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(event.clone());
        }
    }

    struct Harness {
        files: Arc<FakeFiles>,
        sink: Arc<RecordingSink>,
        cell: Arc<ReloadCell<u32>>,
        service: ReloadService<u32>,
    }

    fn candidate(settings: Settings) -> ConfigCandidate {
        ConfigCandidate {
            settings,
            presence: ConfigPresence::default(),
        }
    }

    fn harness(running: Settings, wss_acceptor: Option<u32>) -> Harness {
        let files = Arc::new(FakeFiles::default());
        files.put("wss.crt", b"cert-v1");
        files.put("wss.key", b"key-v1");
        let cell = Arc::new(ReloadCell::new(ReloadSnapshot::new(
            candidate(running.clone()),
            wss_acceptor,
            None,
        )));
        let sink = Arc::new(RecordingSink::default());
        let service = ReloadService::new(
            candidate(running),
            Arc::clone(&cell),
            files.clone(),
            Arc::new(FakeTls),
            Arc::new(crate::reload::ticket_keys::FakeTicketKeys),
            sink.clone(),
        );
        Harness {
            files,
            sink,
            cell,
            service,
        }
    }

    fn addr(port: u16) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], port))
    }

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
                },
                tcp_server: EntryPointSettings {
                    enabled: true,
                    listen: Some(addr(5500)),
                    allow: Some(AllowList::default()),
                },
                ws: WsListenerSettings::default(),
                wss: WsListenerSettings::default(),
                ticket: TicketSettings::default(),
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
            management: crate::settings::ManagementSettings {
                listen: addr(8080),
                tls: None,
                config_edit: false,
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
            recent_logs: RecentLogsSettings { max_entries: 1000 },
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

    /// `settings()` with an enabled WSS listener that uses `wss.crt` and `wss.key`.
    fn with_wss(mut settings: Settings) -> Settings {
        settings.relay.wss = WsListenerSettings {
            listen: Some(addr(8443)),
            viewer: WsRoleSettings {
                enabled: true,
                allow: Some(AllowList::default()),
                allowed_origins: Vec::new(),
            },
            tls: Some(TlsFiles {
                cert_file: PathBuf::from("wss.crt"),
                key_file: PathBuf::from("wss.key"),
            }),
            ..Default::default()
        };
        settings
    }

    /// spec: operations.md:OPS-035 partial
    #[test]
    fn restart_required_key_is_reported_and_nothing_is_swapped() {
        let running = settings();
        let mut h = harness(running.clone(), None);
        let before = h.cell.current();
        let mut candidate = running;
        candidate.relay.tcp_viewer.listen = Some(addr(5902));

        let outcome = h.service.handle_config(
            Ok(self::candidate(candidate)),
            &|| true,
            SystemTime::UNIX_EPOCH,
        );

        assert_eq!(outcome, ReloadOutcome::RestartRequired);
        assert_eq!(
            h.sink.events(),
            vec![Event::ReloadRestartRequired {
                config_keys: vec!["relay.tcp_viewer.listen".to_owned()],
            }]
        );
        assert!(Arc::ptr_eq(&before, &h.cell.current()));
    }

    /// spec: operations.md:OPS-025 partial
    #[test]
    fn allow_list_change_is_applied_and_visible_through_cell() {
        let running = settings();
        let mut h = harness(running.clone(), None);
        let allow = AllowList::new(vec![IpItem::Single(IpAddr::from([10, 0, 0, 1]))]);
        let mut candidate = running;
        candidate.relay.tcp_viewer.allow = Some(allow.clone());

        let outcome = h.service.handle_config(
            Ok(self::candidate(candidate)),
            &|| true,
            SystemTime::UNIX_EPOCH,
        );

        assert_eq!(outcome, ReloadOutcome::Applied);
        assert_eq!(
            h.cell.current().settings.relay.tcp_viewer.allow.as_ref(),
            Some(&allow)
        );
        assert_eq!(
            h.sink.events(),
            vec![Event::ReloadApplied {
                trigger: ReloadTrigger::FileWatch,
                applied: vec!["relay.tcp_viewer.allow".to_owned()],
            }]
        );
    }

    /// spec: operations.md:OPS-026 partial
    #[test]
    fn tls_build_failure_keeps_old_acceptor_and_emits_tls_file_invalid() {
        let running = with_wss(settings());
        let mut h = harness(running.clone(), Some(5));
        h.files.put("wss.crt", b"bad");

        let outcome =
            h.service
                .handle_config(Ok(candidate(running)), &|| true, SystemTime::UNIX_EPOCH);

        assert_eq!(outcome, ReloadOutcome::Rejected);
        assert_eq!(h.cell.current().wss_tls, Some(5));
        assert_eq!(
            h.sink.events(),
            vec![Event::ReloadRejected {
                reason: "tls_file_invalid".to_owned(),
                config_key: Some("relay.wss.key_file".to_owned()),
                detail: Some("pair mismatch".to_owned()),
            }]
        );
    }

    #[test]
    fn unchanged_candidate_emits_no_result_event() {
        let running = settings();
        let mut h = harness(running.clone(), None);

        let outcome =
            h.service
                .handle_config(Ok(candidate(running)), &|| true, SystemTime::UNIX_EPOCH);

        assert_eq!(outcome, ReloadOutcome::Unchanged);
        assert!(h.sink.events().is_empty());
    }

    /// spec: operations.md:OPS-027 partial
    #[test]
    fn stale_generation_discards_without_event_or_swap() {
        let running = settings();
        let mut h = harness(running.clone(), None);
        let before = h.cell.current();
        let mut candidate = running;
        candidate.relay.tcp_viewer.allow =
            Some(AllowList::new(vec![IpItem::Single(IpAddr::from([
                10, 0, 0, 1,
            ]))]));

        let outcome = h.service.handle_config(
            Ok(self::candidate(candidate.clone())),
            &|| false,
            SystemTime::UNIX_EPOCH,
        );

        assert_eq!(outcome, ReloadOutcome::Discarded);
        assert!(h.sink.events().is_empty());
        assert!(Arc::ptr_eq(&before, &h.cell.current()));

        // The discarded attempt left no state behind, so a current attempt applies.
        let outcome = h.service.handle_config(
            Ok(self::candidate(candidate)),
            &|| true,
            SystemTime::UNIX_EPOCH,
        );
        assert_eq!(outcome, ReloadOutcome::Applied);
    }

    /// spec: operations.md:OPS-026 partial
    #[test]
    fn parse_error_emits_reload_rejected_with_diagnostic_code() {
        let mut h = harness(settings(), None);
        let diagnostic = ConfigDiagnostic {
            error: ConfigError::UnknownKey,
            key: "relay.tcp_viewer.bogus".to_owned(),
            message: "unknown key".to_owned(),
        };

        let outcome =
            h.service
                .handle_config(Err(vec![diagnostic]), &|| true, SystemTime::UNIX_EPOCH);

        assert_eq!(outcome, ReloadOutcome::Rejected);
        assert_eq!(
            h.sink.events(),
            vec![Event::ReloadRejected {
                reason: ConfigError::UnknownKey.code().to_owned(),
                config_key: Some("relay.tcp_viewer.bogus".to_owned()),
                detail: None,
            }]
        );
    }
    /// spec: operations.md:OPS-050 partial
    #[test]
    fn prepared_snapshot_publishes_all_runtime_groups_once() {
        let running = settings();
        let mut h = harness(running.clone(), None);
        let before = h.cell.current();
        let mut next = running.clone();
        next.limits.relay_connections = 80;
        next.limits.admin_connections = 8;
        next.limits.waiting = 40;
        next.limits.buffer_bytes = 32_768;
        next.limits.ws_message_bytes = 131_072;
        next.timeouts.handshake = Duration::from_secs(11);
        next.timeouts.tls_upgrade = Duration::from_secs(12);
        next.timeouts.wait = Duration::from_secs(40);
        next.timeouts.connect = Duration::from_secs(13);
        next.timeouts.close = Duration::from_secs(14);
        next.timeouts.shutdown_grace = Duration::from_secs(15);
        next.keepalive.tcp_idle = Duration::from_secs(70);
        next.keepalive.tcp_interval = Duration::from_secs(11);
        next.keepalive.tcp_probes = 4;
        next.keepalive.ws_ping = Duration::from_secs(35);
        next.keepalive.ws_pong_timeout = Duration::from_secs(15);
        let prepared = h
            .service
            .prepare_candidate(candidate(next.clone()))
            .unwrap_or_else(|error| panic!("valid preparation: {error:?}"));
        assert_eq!(prepared.applied_keys().len(), 16);
        assert!(Arc::ptr_eq(&before, &h.cell.current()));
        assert_eq!(h.service.running(), &running);
        assert!(h.service.last_reload().is_none());
        assert!(h.sink.events().is_empty());
        let applied =
            h.service
                .commit_prepared(prepared, ReloadTrigger::ConfigEdit, SystemTime::UNIX_EPOCH);
        assert_eq!(*h.cell.current().settings, next);
        assert_eq!(*before.settings, running);
        assert_eq!(h.service.running(), &next);
        assert_eq!(
            h.sink.events(),
            vec![Event::ReloadApplied {
                trigger: ReloadTrigger::ConfigEdit,
                applied: applied.clone(),
            }]
        );
        let history = h.service.last_reload().cloned();
        assert_eq!(
            history.as_ref().map(|result| &result.config_keys),
            Some(&applied)
        );
        let unchanged = h
            .service
            .prepare_candidate(candidate(next.clone()))
            .unwrap_or_else(|error| panic!("unchanged preparation: {error:?}"));
        assert!(
            h.service
                .commit_prepared(unchanged, ReloadTrigger::ConfigEdit, SystemTime::UNIX_EPOCH)
                .is_empty()
        );
        assert_eq!(h.service.last_reload(), history.as_ref());
        assert_eq!(
            h.service.classify_file(Some(Ok(candidate(next)))).state,
            ConfigFileState::InEffect
        );
        assert_eq!(h.service.last_reload(), history.as_ref());
        assert_eq!(h.sink.events().len(), 1);
    }

    /// spec: operations.md:OPS-063 partial
    #[test]
    fn restart_key_blocks_all_new_dynamic_fields() {
        let running = settings();
        let h = harness(running.clone(), None);
        let mut next = running.clone();
        next.relay.tcp_viewer.listen = Some(addr(5902));
        next.limits.relay_connections = 200;
        next.timeouts.close = Duration::from_secs(20);
        next.keepalive.tcp_probes = 10;
        assert!(matches!(h.service.prepare_candidate(candidate(next)),
            Err(PrepareReloadError::RestartRequired(keys))
                if keys == ["relay.tcp_viewer.listen"]));
        assert_eq!(h.service.running(), &running);
        assert!(h.sink.events().is_empty());
        assert!(h.service.last_reload().is_none());
    }

    #[test]
    fn classification_is_side_effect_free_and_tls_invalid_precedes_restart() {
        let running = with_wss(settings());
        let h = harness(running.clone(), Some(5));
        let before = h.cell.current();
        assert_eq!(
            h.service.classify_file(None).state,
            ConfigFileState::Unreadable
        );
        let unreadable = h.service.classify_file(Some(Err(vec![ConfigDiagnostic {
            error: ConfigError::FileUnreadable,
            key: String::new(),
            message: "unreadable".to_owned(),
        }])));
        assert_eq!(unreadable.state, ConfigFileState::Unreadable);
        assert!(unreadable.diagnostics.is_empty());
        let mut next = running.clone();
        next.timeouts.close = Duration::from_secs(20);
        let pending = h.service.classify_file(Some(Ok(candidate(next.clone()))));
        assert_eq!(pending.state, ConfigFileState::Pending);
        assert_eq!(pending.config_keys, ["timeouts.close_secs"]);
        next.management.listen = addr(8081);
        assert_eq!(
            h.service
                .classify_file(Some(Ok(candidate(next.clone()))))
                .state,
            ConfigFileState::RestartRequired
        );
        h.files.put("wss.crt", b"bad");
        let invalid = h.service.classify_file(Some(Ok(candidate(next))));
        assert_eq!(invalid.state, ConfigFileState::Invalid);
        assert_eq!(invalid.diagnostics[0].error, ConfigError::TlsFileInvalid);
        assert!(Arc::ptr_eq(&before, &h.cell.current()));
        assert!(h.sink.events().is_empty());
        assert!(h.service.last_reload().is_none());
    }

    #[test]
    fn config_edit_commit_is_not_vetoed_by_watcher_shutdown() {
        let running = settings();
        let mut h = harness(running.clone(), None);
        let mut next = running;
        next.timeouts.close = Duration::from_secs(20);
        let prepared = h
            .service
            .prepare_candidate(candidate(next.clone()))
            .unwrap_or_else(|error| panic!("preparation: {error:?}"));
        assert_eq!(
            h.service.handle_config(
                Ok(candidate(next.clone())),
                &|| false,
                SystemTime::UNIX_EPOCH
            ),
            ReloadOutcome::Discarded
        );
        assert_eq!(
            h.service
                .commit_prepared(prepared, ReloadTrigger::ConfigEdit, SystemTime::UNIX_EPOCH),
            ["timeouts.close_secs"]
        );
        let history = h.service.last_reload().cloned();
        assert_eq!(
            h.service
                .handle_config(Ok(candidate(next)), &|| true, SystemTime::UNIX_EPOCH),
            ReloadOutcome::Unchanged
        );
        assert_eq!(h.service.last_reload(), history.as_ref());
    }

    #[test]
    fn management_transport_presence_and_edit_permission_require_restart() {
        let running = settings();
        let mut next = running.clone();
        next.management.config_edit = true;
        next.management.tls = Some(TlsFiles {
            cert_file: "m.crt".into(),
            key_file: "m.key".into(),
        });
        assert_eq!(
            restart_required_keys(&running, &next),
            ["management.cert_file", "management.config_edit"]
        );
        next = running.clone();
        next.relay.ws.viewer.allowed_origins = Vec::new();
        let mut presence = ConfigPresence::default();
        presence
            .keys
            .insert("relay.ws.viewer.allowed_origins".to_owned());
        let h = harness(running, None);
        assert_eq!(
            h.service
                .classify_file(Some(Ok(ConfigCandidate {
                    settings: next,
                    presence
                })))
                .state,
            ConfigFileState::InEffect
        );
    }
    #[test]
    fn watcher_refreshes_presence_without_runtime_change_event_or_history() {
        let mut h = harness(settings(), None);
        let mut next = h.service.running().clone();
        next.timeouts.close = Duration::from_secs(20);
        h.service.handle_config(
            Ok(candidate(next.clone())),
            &|| true,
            SystemTime::UNIX_EPOCH,
        );
        let history = h.service.last_reload().cloned();
        let events = h.sink.events();
        let before = h.cell.current();
        let mut presence = ConfigPresence::default();
        presence
            .keys
            .insert("relay.ws.viewer.allowed_origins".to_owned());
        presence.keys.insert("relay.wss.cert_file".to_owned());
        presence.inactive_paths.insert(
            "relay.wss.cert_file".to_owned(),
            PathBuf::from("inactive.crt"),
        );
        let next = ConfigCandidate {
            settings: next,
            presence: presence.clone(),
        };
        assert_eq!(
            h.service
                .handle_config(Ok(next), &|| true, SystemTime::UNIX_EPOCH),
            ReloadOutcome::Unchanged
        );
        let after = h.cell.current();
        assert!(Arc::ptr_eq(&before.settings, &after.settings));
        assert_eq!(h.service.running_presence(), &presence);
        assert_eq!(*after.presence, presence);
        assert_eq!(h.service.last_reload(), history.as_ref());
        assert_eq!(h.sink.events(), events);
    }
    /// spec: operations.md:OPS-064 partial
    #[test]
    fn every_new_runtime_key_is_dynamic_and_not_restart_only() {
        use vnc_relay_domain::origin::Origin;
        use vnc_relay_domain::target_policy::{TargetItem, TargetPolicy};
        let mut running = with_wss(settings());
        running.relay.ws.viewer.enabled = true;
        running.relay.ws.server.enabled = true;
        running.relay.wss.server.enabled = true;
        running.relay.ws.listen = Some(addr(8081));
        running.relay.ws.viewer.allow = Some(AllowList::default());
        running.relay.ws.server.allow = Some(AllowList::default());
        running.relay.wss.server.allow = Some(AllowList::default());
        running.mode1.enabled = true;
        running.mode1.allow_targets = Some(TargetPolicy::default());
        type Change = (&'static str, fn(&mut Settings));
        let changes: [Change; 25] = [
            ("relay.ws.trusted_proxies", |s| {
                s.relay.ws.trusted_proxies =
                    AllowList::new(vec![IpItem::Single(IpAddr::from([10, 0, 0, 1]))]);
            }),
            ("relay.wss.trusted_proxies", |s| {
                s.relay.wss.trusted_proxies =
                    AllowList::new(vec![IpItem::Single(IpAddr::from([10, 0, 0, 1]))]);
            }),
            ("relay.ws.server.allow", |s| {
                s.relay.ws.server.allow =
                    Some(AllowList::new(vec![IpItem::Single(IpAddr::from([
                        10, 0, 0, 1,
                    ]))]));
            }),
            ("relay.wss.server.allow", |s| {
                s.relay.wss.server.allow =
                    Some(AllowList::new(vec![IpItem::Single(IpAddr::from([
                        10, 0, 0, 1,
                    ]))]));
            }),
            ("relay.ws.server.allowed_origins", |s| {
                s.relay.ws.server.allowed_origins.push(
                    Origin::parse("https://server.example")
                        .unwrap_or_else(|error| panic!("origin: {error:?}")),
                );
            }),
            ("relay.wss.server.allowed_origins", |s| {
                s.relay.wss.server.allowed_origins.push(
                    Origin::parse("https://server.example")
                        .unwrap_or_else(|error| panic!("origin: {error:?}")),
                );
            }),
            ("relay.ws.viewer.allowed_origins", |s| {
                s.relay.ws.viewer.allowed_origins.push(
                    Origin::parse("https://viewer.example")
                        .unwrap_or_else(|error| panic!("origin: {error:?}")),
                )
            }),
            ("relay.wss.viewer.allowed_origins", |s| {
                s.relay.wss.viewer.allowed_origins.push(
                    Origin::parse("https://viewer.example")
                        .unwrap_or_else(|error| panic!("origin: {error:?}")),
                )
            }),
            ("mode1.allow_targets", |s| {
                s.mode1.allow_targets = Some(TargetPolicy::new(vec![
                    TargetItem::parse("127.0.0.1:5900")
                        .unwrap_or_else(|error| panic!("target: {error:?}")),
                ]))
            }),
            ("limits.relay_connections", |s| {
                s.limits.relay_connections += 1
            }),
            ("limits.admin_connections", |s| {
                s.limits.admin_connections += 1
            }),
            ("limits.waiting", |s| s.limits.waiting += 1),
            ("limits.buffer_bytes", |s| s.limits.buffer_bytes += 1),
            ("limits.ws_message_bytes", |s| {
                s.limits.ws_message_bytes += 1
            }),
            ("timeouts.handshake_secs", |s| {
                s.timeouts.handshake += Duration::from_secs(1)
            }),
            ("timeouts.tls_upgrade_secs", |s| {
                s.timeouts.tls_upgrade += Duration::from_secs(1)
            }),
            ("timeouts.wait_secs", |s| {
                s.timeouts.wait += Duration::from_secs(1)
            }),
            ("timeouts.connect_secs", |s| {
                s.timeouts.connect += Duration::from_secs(1)
            }),
            ("timeouts.close_secs", |s| {
                s.timeouts.close += Duration::from_secs(1)
            }),
            ("timeouts.shutdown_grace_secs", |s| {
                s.timeouts.shutdown_grace += Duration::from_secs(1)
            }),
            ("keepalive.tcp_idle_secs", |s| {
                s.keepalive.tcp_idle += Duration::from_secs(1)
            }),
            ("keepalive.tcp_interval_secs", |s| {
                s.keepalive.tcp_interval += Duration::from_secs(1)
            }),
            ("keepalive.tcp_probes", |s| s.keepalive.tcp_probes += 1),
            ("keepalive.ws_ping_secs", |s| {
                s.keepalive.ws_ping += Duration::from_secs(1)
            }),
            ("keepalive.ws_pong_timeout_secs", |s| {
                s.keepalive.ws_pong_timeout += Duration::from_secs(1)
            }),
        ];
        let h = harness(running.clone(), Some(5));
        for (key, change) in changes {
            let mut next = running.clone();
            change(&mut next);
            assert_eq!(dynamic_changed_keys(&running, &next), [key], "{key}");
            assert!(restart_required_keys(&running, &next).is_empty(), "{key}");
            let prepared = h
                .service
                .prepare_candidate(candidate(next))
                .unwrap_or_else(|error| panic!("{key}: {error:?}"));
            assert_eq!(prepared.applied_keys(), [key], "{key}");
        }
        assert!(h.sink.events().is_empty());
        assert!(h.service.last_reload().is_none());
    }
    /// spec: operations.md:OPS-067 partial
    #[test]
    fn role_policies_and_trusted_proxies_publish_in_one_snapshot() {
        use vnc_relay_domain::origin::Origin;

        let mut running = with_wss(settings());
        running.relay.wss.viewer.enabled = false;
        running.relay.wss.server = WsRoleSettings {
            enabled: true,
            allow: Some(AllowList::default()),
            ..Default::default()
        };
        let mut h = harness(running.clone(), Some(5));
        assert_eq!(
            h.service.watched_paths(),
            [PathBuf::from("wss.crt"), PathBuf::from("wss.key")]
        );
        let before = h.cell.current();
        let mut next = running;
        let allow = AllowList::new(vec![IpItem::Single(IpAddr::from([10, 0, 0, 1]))]);
        next.relay.wss.server.allow = Some(allow.clone());
        next.relay.wss.server.allowed_origins = vec![
            Origin::parse("https://server.example")
                .unwrap_or_else(|error| panic!("origin: {error:?}")),
        ];
        next.relay.wss.trusted_proxies = allow;
        let prepared = h
            .service
            .prepare_candidate(candidate(next.clone()))
            .unwrap_or_else(|error| panic!("preparation: {error:?}"));
        assert_eq!(
            prepared.applied_keys(),
            [
                "relay.wss.trusted_proxies",
                "relay.wss.server.allow",
                "relay.wss.server.allowed_origins",
            ]
        );
        assert!(Arc::ptr_eq(&before, &h.cell.current()));
        h.service
            .commit_prepared(prepared, ReloadTrigger::FileWatch, SystemTime::UNIX_EPOCH);
        assert_eq!(*h.cell.current().settings, next);
        assert!(before.settings.relay.wss.trusted_proxies.items().is_empty());
        assert!(before.settings.relay.wss.server.allowed_origins.is_empty());
        assert_eq!(h.cell.current().wss_tls, Some(5));
    }

    /// spec: operations.md:OPS-065 partial
    #[test]
    fn shared_listener_addresses_role_switches_and_recent_logs_require_restart() {
        let running = settings();
        let mut next = running.clone();
        next.relay.ws.listen = Some(addr(8081));
        next.relay.ws.server.enabled = true;
        next.relay.wss.listen = Some(addr(8443));
        next.relay.wss.viewer.enabled = true;
        next.relay.wss.server.enabled = true;
        next.recent_logs.max_entries = 0;
        assert_eq!(
            restart_required_keys(&running, &next),
            [
                "relay.ws.listen",
                "relay.ws.server.enabled",
                "relay.wss.listen",
                "relay.wss.viewer.enabled",
                "relay.wss.server.enabled",
                "recent_logs.max_entries",
            ]
        );
    }

    #[test]
    fn disabled_role_and_listener_lists_do_not_count_as_runtime_changes() {
        use vnc_relay_domain::origin::Origin;

        let running = settings();
        let mut next = running.clone();
        next.relay.ws.server.allow = Some(AllowList::default());
        next.relay.ws.server.allowed_origins = vec![
            Origin::parse("https://server.example")
                .unwrap_or_else(|error| panic!("origin: {error:?}")),
        ];
        next.relay.ws.trusted_proxies =
            AllowList::new(vec![IpItem::Single(IpAddr::from([10, 0, 0, 1]))]);
        assert!(dynamic_changed_keys(&running, &next).is_empty());
        assert!(restart_required_keys(&running, &next).is_empty());
    }
    fn enable_ticket(settings: &mut Settings, role: Role, path: &str) {
        let ticket = settings.relay.ticket.role_mut(role);
        ticket.enabled = true;
        ticket.issuer = Some("issuer.example".to_owned());
        ticket.relay_host = Some("relay.example".to_owned());
        ticket.public_keys_file = Some(PathBuf::from(path));
    }

    /// spec: operations.md:OPS-072 partial
    #[test]
    fn ticket_rotation_rejects_invalid_keys_and_publishes_valid_keys_atomically() {
        let mut running = settings();
        enable_ticket(&mut running, Role::Viewer, "viewer.jwks");
        running.relay.ticket.viewer.keys = crate::reload::ticket_keys::FakeTicketKeys
            .parse(b"old 1")
            .unwrap();
        let mut h = harness(running.clone(), None);
        assert_eq!(h.service.watched_paths(), [PathBuf::from("viewer.jwks")]);
        let before = h.cell.current();
        h.files.put("viewer.jwks", b"invalid");
        assert_eq!(
            h.service.handle_config(
                Ok(candidate(running.clone())),
                &|| true,
                SystemTime::UNIX_EPOCH
            ),
            ReloadOutcome::Rejected
        );
        assert!(Arc::ptr_eq(&before, &h.cell.current()));
        assert!(matches!(&h.sink.events()[0], Event::ReloadRejected {
            reason, config_key: Some(key), detail: Some(detail)
        } if reason == "ticket_keys_invalid"
            && key == "relay.ticket.viewer.public_keys_file"
            && detail == vnc_relay_domain::ticket::TicketKeyError::Format.detail()));
        h.files.put("viewer.jwks", b"new 2");
        assert_eq!(
            h.service
                .classify_file(Some(Ok(candidate(running.clone()))))
                .state,
            ConfigFileState::InEffect
        );
        let prepared = h
            .service
            .prepare_candidate(candidate(running.clone()))
            .unwrap();
        assert_eq!(
            prepared.applied_keys(),
            ["relay.ticket.viewer.public_keys_file"]
        );
        h.service
            .commit_prepared(prepared, ReloadTrigger::ConfigEdit, SystemTime::UNIX_EPOCH);
        assert!(
            h.service
                .running()
                .relay
                .ticket
                .viewer
                .keys
                .find("new")
                .is_some()
        );
        assert!(
            before
                .settings
                .relay
                .ticket
                .viewer
                .keys
                .find("old")
                .is_some()
        );
        assert_eq!(
            h.service
                .handle_config(Ok(candidate(running)), &|| true, SystemTime::UNIX_EPOCH),
            ReloadOutcome::Unchanged
        );
        assert!(
            h.cell
                .current()
                .settings
                .relay
                .ticket
                .viewer
                .keys
                .find("new")
                .is_some()
        );
    }

    /// spec: operations.md:OPS-072 partial
    #[test]
    fn all_ten_ticket_values_are_dynamic_and_failed_candidate_files_remain_watched() {
        let running = settings();
        let mut next = running.clone();
        enable_ticket(&mut next, Role::Viewer, "viewer.jwks");
        enable_ticket(&mut next, Role::Server, "server.jwks");
        next.relay.ticket.viewer.max_lifetime = Duration::from_secs(10);
        next.relay.ticket.server.max_lifetime = Duration::from_secs(600);
        let expected = &DYNAMIC_KEYS[14..24];
        assert_eq!(DYNAMIC_KEYS.len(), 43);
        assert_eq!(dynamic_changed_keys(&running, &next), expected);
        assert!(restart_required_keys(&running, &next).is_empty());
        let mut h = harness(running, None);
        assert_eq!(
            h.service.handle_config(
                Ok(candidate(next.clone())),
                &|| true,
                SystemTime::UNIX_EPOCH
            ),
            ReloadOutcome::Rejected
        );
        assert_eq!(
            h.service.watched_paths(),
            [PathBuf::from("viewer.jwks"), PathBuf::from("server.jwks")]
        );
        h.files.put("viewer.jwks", b"viewer 1");
        h.files.put("server.jwks", b"server 2");
        let prepared = h.service.prepare_candidate(candidate(next)).unwrap();
        assert_eq!(prepared.applied_keys(), expected);
        h.service
            .commit_prepared(prepared, ReloadTrigger::FileWatch, SystemTime::UNIX_EPOCH);
        assert_eq!(h.cell.current().settings.relay.ticket.viewer.keys.len(), 1);
        assert_eq!(h.cell.current().settings.relay.ticket.server.keys.len(), 1);
    }

    /// spec: operations.md:OPS-071 partial
    #[test]
    fn ticket_reload_warnings_only_repeat_for_the_role_being_enabled() {
        let mut running = settings();
        running.relay.ws.viewer.enabled = true;
        running.relay.ws.server.enabled = true;
        let mut h = harness(running.clone(), None);
        h.files.put("viewer.jwks", b"viewer 1");
        h.files.put("server.jwks", b"server 2");
        for role in [Role::Viewer, Role::Server] {
            enable_ticket(
                &mut running,
                role,
                if role == Role::Viewer {
                    "viewer.jwks"
                } else {
                    "server.jwks"
                },
            );
            let prepared = h
                .service
                .prepare_candidate(candidate(running.clone()))
                .unwrap();
            h.service
                .commit_prepared(prepared, ReloadTrigger::FileWatch, SystemTime::UNIX_EPOCH);
            let warnings: Vec<_> = h
                .sink
                .events()
                .into_iter()
                .filter_map(|event| match event {
                    Event::ConfigWarning {
                        warning,
                        phase: Phase::Reload,
                    } => Some(warning),
                    _ => None,
                })
                .collect();
            assert!(warnings.contains(&ConfigWarning::TicketTcpBypass { role }));
            assert!(warnings.contains(&ConfigWarning::TicketPlaintextWs { role }));
        }
        let warning_count = h
            .sink
            .events()
            .iter()
            .filter(|event| matches!(event, Event::ConfigWarning { .. }))
            .count();
        running.relay.ticket.viewer.issuer = Some("other-issuer.example".to_owned());
        let prepared = h.service.prepare_candidate(candidate(running)).unwrap();
        h.service
            .commit_prepared(prepared, ReloadTrigger::FileWatch, SystemTime::UNIX_EPOCH);
        assert_eq!(
            h.sink
                .events()
                .iter()
                .filter(|event| matches!(event, Event::ConfigWarning { .. }))
                .count(),
            warning_count
        );
    }
}
