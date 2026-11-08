//! Verification and swap of a reloaded configuration (file-watch-v1 §4 steps 4–9).

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use vnc_relay_domain::allow_list::AllowList;
use vnc_relay_domain::entry_point::EntryPoint;

use crate::events::{Event, Phase};
use crate::ports::EventSink;
use crate::reload::ports::{ReloadFileStore, ReloadTlsStore, TlsBuildError};
use crate::reload::snapshot::{RelayAllow, ReloadCell, ReloadSnapshot};
use crate::settings::{ConfigDiagnostic, ConfigError, ConfigWarning, Settings, TlsFiles};

/// Entry points in config order (config-v3 §1.8).
const ENTRY_POINTS: [EntryPoint; 4] = [
    EntryPoint::TcpViewer,
    EntryPoint::TcpServer,
    EntryPoint::WsViewer,
    EntryPoint::WssViewer,
];

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
    running: Settings,
    cell: Arc<ReloadCell<A>>,
    files: Arc<dyn ReloadFileStore>,
    tls: Arc<dyn ReloadTlsStore<A>>,
    events: Arc<dyn EventSink>,
    /// Certificate and key paths of the last candidate that parsed (enabled TLS listeners).
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
        running: Settings,
        cell: Arc<ReloadCell<A>>,
        files: Arc<dyn ReloadFileStore>,
        tls: Arc<dyn ReloadTlsStore<A>>,
        events: Arc<dyn EventSink>,
    ) -> Self {
        let wss_hashes = baseline_hashes(files.as_ref(), TlsSlot::Wss.files(&running));
        let management_hashes =
            baseline_hashes(files.as_ref(), TlsSlot::Management.files(&running));
        Self {
            running,
            cell,
            files,
            tls,
            events,
            candidate_paths: Vec::new(),
            wss_hashes,
            management_hashes,
            before_validation: None,
        }
    }

    /// Paths to watch besides the config file (file-watch-v1 §9 item 9): the running TLS
    /// files and those of the last candidate that parsed, without duplicates.
    pub fn watched_paths(&self) -> Vec<PathBuf> {
        let mut paths = tls_paths(&self.running);
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

impl<A: Clone> ReloadService<A> {
    /// Handles a parsed config file (file-watch-v1 §4 steps 4–9, §4.2 event order).
    ///
    /// `is_current` returns false when the generation is stale or shutdown began. It is
    /// checked before any work and again before each result event and the swap.
    pub fn handle_config(
        &mut self,
        parsed: Result<Settings, Vec<ConfigDiagnostic>>,
        is_current: &dyn Fn() -> bool,
    ) -> ReloadOutcome {
        if !is_current() {
            return ReloadOutcome::Discarded;
        }
        let candidate = match parsed {
            Ok(candidate) => candidate,
            Err(diagnostics) => {
                let (reason, config_key) = rejection_of(&diagnostics);
                self.events.record(&Event::ReloadRejected {
                    reason: reason.to_owned(),
                    config_key,
                    detail: None,
                });
                return ReloadOutcome::Rejected;
            }
        };
        let paths = tls_paths(&candidate);
        let outcome = self.apply_candidate(candidate, is_current);
        if outcome != ReloadOutcome::Discarded {
            self.candidate_paths = paths;
        }
        outcome
    }

    /// Handles a config file that could not be read during stabilization.
    ///
    /// Emits `reload_rejected` with `config_file_unreadable`. The running settings stay.
    pub fn handle_unreadable(&mut self, is_current: &dyn Fn() -> bool) -> ReloadOutcome {
        if !is_current() {
            return ReloadOutcome::Discarded;
        }
        self.events.record(&Event::ReloadRejected {
            reason: ConfigError::FileUnreadable.code().to_owned(),
            config_key: None,
            detail: None,
        });
        ReloadOutcome::Rejected
    }

    /// Runs file-watch-v1 §4 steps 6–9 for a parsed candidate.
    fn apply_candidate(
        &mut self,
        candidate: Settings,
        is_current: &dyn Fn() -> bool,
    ) -> ReloadOutcome {
        if let Some(action) = self.before_validation.take() {
            action();
        }

        // Step 6: build the TLS acceptors of enabled listeners.
        let tls = match self.load_tls(&candidate) {
            Ok(tls) => tls,
            Err(rejection) => {
                if !is_current() {
                    return ReloadOutcome::Discarded;
                }
                self.events.record(&Event::ReloadRejected {
                    reason: "tls_file_invalid".to_owned(),
                    config_key: Some(rejection.config_key.to_owned()),
                    detail: Some(rejection.detail.to_owned()),
                });
                return ReloadOutcome::Rejected;
            }
        };

        // Step 7: any key outside the reload targets must match the running value.
        let restart_keys = restart_required_keys(&self.running, &candidate);
        if !restart_keys.is_empty() {
            if !is_current() {
                return ReloadOutcome::Discarded;
            }
            self.events.record(&Event::ReloadRestartRequired {
                config_keys: restart_keys,
            });
            return ReloadOutcome::RestartRequired;
        }

        // Step 8: final generation check before the swap.
        if !is_current() {
            return ReloadOutcome::Discarded;
        }

        // Step 9: collect the changed reload targets in the fixed order of file-watch-v1 §4.
        let mut changed_allow = Vec::new();
        let mut applied: Vec<&'static str> = Vec::new();
        for entry_point in ENTRY_POINTS {
            if effective_allow(&self.running, entry_point)
                != effective_allow(&candidate, entry_point)
            {
                applied.push(allow_key(entry_point));
                changed_allow.push(entry_point);
            }
        }
        let TlsLoad {
            wss_acceptor,
            management_acceptor,
            wss_hashes,
            management_hashes,
            changed,
        } = tls;
        applied.extend(changed);
        if applied.is_empty() {
            return ReloadOutcome::Unchanged;
        }

        let warnings: Vec<ConfigWarning> = candidate
            .warnings()
            .into_iter()
            .filter(|warning| {
                matches!(
                    warning,
                    ConfigWarning::AllowAllSource { entry_point, .. }
                        if changed_allow.contains(entry_point)
                )
            })
            .collect();

        let relay_allow = relay_allow_of(&candidate);
        self.cell.replace(ReloadSnapshot::new(
            relay_allow,
            wss_acceptor,
            management_acceptor,
        ));
        self.running = candidate;
        self.wss_hashes = wss_hashes;
        self.management_hashes = management_hashes;

        for warning in warnings {
            self.events.record(&Event::ConfigWarning {
                warning,
                phase: Phase::Reload,
            });
        }
        self.events.record(&Event::ReloadApplied { applied });
        ReloadOutcome::Applied
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

/// The two TLS listeners that can reload (config-v3 §1.9).
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
                let wss = settings.relay.get(EntryPoint::WssViewer);
                wss.tls.as_ref().filter(|_| wss.enabled)
            }
            Self::Management => settings.management.tls.as_ref(),
        }
    }

    /// Config key of the certificate file.
    fn cert_key(self) -> &'static str {
        match self {
            Self::Wss => "relay.wss_viewer.cert_file",
            Self::Management => "management.cert_file",
        }
    }

    /// Config key of the private key file.
    fn key_key(self) -> &'static str {
        match self {
            Self::Wss => "relay.wss_viewer.key_file",
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

/// Allow list in effect: empty when the entry point is disabled or has no list.
fn effective_allow(settings: &Settings, entry_point: EntryPoint) -> AllowList {
    let entry = settings.relay.get(entry_point);
    if entry.enabled {
        entry.allow.clone().unwrap_or_default()
    } else {
        AllowList::default()
    }
}

fn relay_allow_of(settings: &Settings) -> RelayAllow {
    RelayAllow {
        tcp_viewer: effective_allow(settings, EntryPoint::TcpViewer),
        tcp_server: effective_allow(settings, EntryPoint::TcpServer),
        ws_viewer: effective_allow(settings, EntryPoint::WsViewer),
        wss_viewer: effective_allow(settings, EntryPoint::WssViewer),
    }
}

/// Config key of an allow list (config-v3 §1.8).
fn allow_key(entry_point: EntryPoint) -> &'static str {
    match entry_point {
        EntryPoint::TcpViewer => "relay.tcp_viewer.allow",
        EntryPoint::TcpServer => "relay.tcp_server.allow",
        EntryPoint::WsViewer => "relay.ws_viewer.allow",
        EntryPoint::WssViewer => "relay.wss_viewer.allow",
    }
}

fn push_if(keys: &mut Vec<String>, differs: bool, key: &str) {
    if differs {
        keys.push(key.to_owned());
    }
}

/// Config keys whose candidate value differs from the running value, excluding reload
/// targets (config-v3 §1.8, file-watch-v1 §4 step 7).
///
/// The management certificate is checked by presence only: HTTP and HTTPS switch need a
/// restart. Changes to a WSS listener's TLS presence follow `relay.wss_viewer.enabled`.
fn restart_required_keys(running: &Settings, candidate: &Settings) -> Vec<String> {
    let mut keys = Vec::new();
    for entry_point in ENTRY_POINTS {
        let before = running.relay.get(entry_point);
        let after = candidate.relay.get(entry_point);
        let name = entry_point.name();
        push_if(
            &mut keys,
            before.enabled != after.enabled,
            &format!("relay.{name}.enabled"),
        );
        push_if(
            &mut keys,
            before.listen != after.listen,
            &format!("relay.{name}.listen"),
        );
        push_if(
            &mut keys,
            before.allowed_origins != after.allowed_origins,
            &format!("relay.{name}.allowed_origins"),
        );
    }

    let (a, b) = (running, candidate);
    push_if(
        &mut keys,
        a.mode1.enabled != b.mode1.enabled,
        "mode1.enabled",
    );
    push_if(
        &mut keys,
        a.mode1.allow_targets != b.mode1.allow_targets,
        "mode1.allow_targets",
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
        a.limits.relay_connections != b.limits.relay_connections,
        "limits.relay_connections",
    );
    push_if(
        &mut keys,
        a.limits.admin_connections != b.limits.admin_connections,
        "limits.admin_connections",
    );
    push_if(
        &mut keys,
        a.limits.waiting != b.limits.waiting,
        "limits.waiting",
    );
    push_if(
        &mut keys,
        a.limits.buffer_bytes != b.limits.buffer_bytes,
        "limits.buffer_bytes",
    );
    push_if(
        &mut keys,
        a.limits.ws_message_bytes != b.limits.ws_message_bytes,
        "limits.ws_message_bytes",
    );

    push_if(
        &mut keys,
        a.timeouts.handshake != b.timeouts.handshake,
        "timeouts.handshake_secs",
    );
    push_if(
        &mut keys,
        a.timeouts.tls_upgrade != b.timeouts.tls_upgrade,
        "timeouts.tls_upgrade_secs",
    );
    push_if(
        &mut keys,
        a.timeouts.wait != b.timeouts.wait,
        "timeouts.wait_secs",
    );
    push_if(
        &mut keys,
        a.timeouts.connect != b.timeouts.connect,
        "timeouts.connect_secs",
    );
    push_if(
        &mut keys,
        a.timeouts.close != b.timeouts.close,
        "timeouts.close_secs",
    );
    push_if(
        &mut keys,
        a.timeouts.shutdown_grace != b.timeouts.shutdown_grace,
        "timeouts.shutdown_grace_secs",
    );

    push_if(
        &mut keys,
        a.keepalive.tcp_idle != b.keepalive.tcp_idle,
        "keepalive.tcp_idle_secs",
    );
    push_if(
        &mut keys,
        a.keepalive.tcp_interval != b.keepalive.tcp_interval,
        "keepalive.tcp_interval_secs",
    );
    push_if(
        &mut keys,
        a.keepalive.tcp_probes != b.keepalive.tcp_probes,
        "keepalive.tcp_probes",
    );
    push_if(
        &mut keys,
        a.keepalive.ws_ping != b.keepalive.ws_ping,
        "keepalive.ws_ping_secs",
    );
    push_if(
        &mut keys,
        a.keepalive.ws_pong_timeout != b.keepalive.ws_pong_timeout,
        "keepalive.ws_pong_timeout_secs",
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

    use vnc_relay_domain::allow_list::IpItem;

    use super::*;
    use crate::settings::{
        AdminSessionSettings, AdminSettings, Argon2Settings, DashboardSettings, EntryPointSettings,
        KeepaliveSettings, LimitSettings, LogOutput, LogSettings, LoginLimitSettings,
        Mode1Settings, Mode2Settings, RecentClosedSettings, RelaySettings, ReloadSettings,
        StatusSettings, TimeoutSettings,
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

    fn harness(running: Settings, wss_acceptor: Option<u32>) -> Harness {
        let files = Arc::new(FakeFiles::default());
        files.put("wss.crt", b"cert-v1");
        files.put("wss.key", b"key-v1");
        let cell = Arc::new(ReloadCell::new(ReloadSnapshot::new(
            relay_allow_of(&running),
            wss_acceptor,
            None,
        )));
        let sink = Arc::new(RecordingSink::default());
        let service = ReloadService::new(
            running,
            Arc::clone(&cell),
            files.clone(),
            Arc::new(FakeTls),
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
            management: crate::settings::ManagementSettings {
                listen: addr(8080),
                tls: None,
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

    /// `settings()` with an enabled WSS listener that uses `wss.crt` and `wss.key`.
    fn with_wss(mut settings: Settings) -> Settings {
        settings.relay.wss_viewer = EntryPointSettings {
            enabled: true,
            listen: Some(addr(8443)),
            allow: Some(AllowList::default()),
            allowed_origins: Vec::new(),
            tls: Some(TlsFiles {
                cert_file: PathBuf::from("wss.crt"),
                key_file: PathBuf::from("wss.key"),
            }),
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
        candidate.limits.relay_connections = 200;

        let outcome = h.service.handle_config(Ok(candidate), &|| true);

        assert_eq!(outcome, ReloadOutcome::RestartRequired);
        assert_eq!(
            h.sink.events(),
            vec![Event::ReloadRestartRequired {
                config_keys: vec!["limits.relay_connections".to_owned()],
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

        let outcome = h.service.handle_config(Ok(candidate), &|| true);

        assert_eq!(outcome, ReloadOutcome::Applied);
        assert_eq!(h.cell.current().relay_allow.tcp_viewer, allow);
        assert_eq!(
            h.sink.events(),
            vec![Event::ReloadApplied {
                applied: vec!["relay.tcp_viewer.allow"],
            }]
        );
    }

    /// spec: operations.md:OPS-026 partial
    #[test]
    fn tls_build_failure_keeps_old_acceptor_and_emits_tls_file_invalid() {
        let running = with_wss(settings());
        let mut h = harness(running.clone(), Some(5));
        h.files.put("wss.crt", b"bad");

        let outcome = h.service.handle_config(Ok(running), &|| true);

        assert_eq!(outcome, ReloadOutcome::Rejected);
        assert_eq!(h.cell.current().wss_tls, Some(5));
        assert_eq!(
            h.sink.events(),
            vec![Event::ReloadRejected {
                reason: "tls_file_invalid".to_owned(),
                config_key: Some("relay.wss_viewer.key_file".to_owned()),
                detail: Some("pair mismatch".to_owned()),
            }]
        );
    }

    #[test]
    fn unchanged_candidate_emits_no_result_event() {
        let running = settings();
        let mut h = harness(running.clone(), None);

        let outcome = h.service.handle_config(Ok(running), &|| true);

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

        let outcome = h.service.handle_config(Ok(candidate.clone()), &|| false);

        assert_eq!(outcome, ReloadOutcome::Discarded);
        assert!(h.sink.events().is_empty());
        assert!(Arc::ptr_eq(&before, &h.cell.current()));

        // The discarded attempt left no state behind, so a current attempt applies.
        let outcome = h.service.handle_config(Ok(candidate), &|| true);
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

        let outcome = h.service.handle_config(Err(vec![diagnostic]), &|| true);

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
}
