//! Effective configuration views and serialized file-backed edits (ADR-0006).

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use vnc_relay_domain::account::AccountName;
use vnc_relay_domain::peer_ip::PeerIp;

use crate::events::Event;
use crate::ports::{Clock, EventSink};
use crate::reload::{
    ConfigCandidate, FileClassification, LastReload, PrepareReloadError, ReloadTrigger,
    SharedReload,
};
use crate::settings::ConfigDiagnostic;

mod items;
pub use items::{ConfigItem, ConfigReloadKind, config_items};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigValue {
    Bool(bool),
    Integer(i64),
    String(String),
    Strings(Vec<String>),
    Null,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigChange {
    pub key: String,
    pub value: ConfigValue,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigActor {
    pub admin_account: AccountName,
    pub peer_ip: PeerIp,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigPatch {
    pub revision: String,
    pub changes: Vec<ConfigChange>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigView {
    pub as_of: SystemTime,
    pub revision: Option<String>,
    pub edit_enabled: bool,
    pub file: FileClassification,
    pub last_reload: Option<LastReload>,
    pub items: Vec<ConfigItem>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigEditResult {
    pub as_of: SystemTime,
    pub revision: String,
    pub applied: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigGetError {
    Internal,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigEditError {
    InvalidRequest,
    ConfigInvalid { diagnostics: Vec<ConfigDiagnostic> },
    EditDisabled,
    Conflict,
    RestartPending { config_keys: Vec<String> },
    FileInvalid { diagnostics: Vec<ConfigDiagnostic> },
    WriteFailed,
    Internal,
}
impl ConfigEditError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidRequest => "invalid_request",
            Self::ConfigInvalid { .. } => "config_invalid",
            Self::EditDisabled => "config_edit_disabled",
            Self::Conflict => "config_conflict",
            Self::RestartPending { .. } => "config_restart_pending",
            Self::FileInvalid { .. } => "config_file_invalid",
            Self::WriteFailed => "config_write_failed",
            Self::Internal => "internal",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigReplaceError {
    Conflict,
    WriteFailed,
}
pub trait ConfigFileStore: Send + Sync {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
    fn revision(&self, bytes: &[u8]) -> String;
    fn replace(
        &self,
        path: &Path,
        expected_revision: &str,
        next: &[u8],
    ) -> Result<(), ConfigReplaceError>;
}
pub trait ConfigDocument: Send + Sync {
    fn parse(
        &self,
        bytes: &[u8],
        base_dir: &Path,
    ) -> Result<ConfigCandidate, Vec<ConfigDiagnostic>>;
    fn edit(
        &self,
        bytes: &[u8],
        changes: &[ConfigChange],
    ) -> Result<Vec<u8>, Vec<ConfigDiagnostic>>;
}
pub trait ConfigManagement: Send + Sync {
    fn get(&self) -> Result<ConfigView, ConfigGetError>;
    fn patch(
        &self,
        actor: ConfigActor,
        request: ConfigPatch,
    ) -> Result<ConfigEditResult, ConfigEditError>;
}

pub struct ConfigEditService<A> {
    reload: SharedReload<A>,
    files: Arc<dyn ConfigFileStore>,
    document: Arc<dyn ConfigDocument>,
    config_path: PathBuf,
    base_dir: PathBuf,
    clock: Arc<dyn Clock>,
    events: Arc<dyn EventSink>,
}
impl<A> ConfigEditService<A> {
    pub fn new(
        reload: SharedReload<A>,
        files: Arc<dyn ConfigFileStore>,
        document: Arc<dyn ConfigDocument>,
        config_path: PathBuf,
        base_dir: PathBuf,
        clock: Arc<dyn Clock>,
        events: Arc<dyn EventSink>,
    ) -> Self {
        Self {
            reload,
            files,
            document,
            config_path,
            base_dir,
            clock,
            events,
        }
    }
    fn reject(
        &self,
        actor: &ConfigActor,
        keys: Vec<String>,
        error: ConfigEditError,
    ) -> ConfigEditError {
        if !matches!(
            error,
            ConfigEditError::InvalidRequest | ConfigEditError::Internal
        ) {
            self.events.record(&Event::ConfigEditRejected {
                admin_account: actor.admin_account.clone(),
                peer_ip: actor.peer_ip,
                reason: error.code().to_owned(),
                config_keys: Some(keys),
            });
        }
        error
    }
}
impl<A: Clone + Send + Sync + 'static> ConfigManagement for ConfigEditService<A> {
    fn get(&self) -> Result<ConfigView, ConfigGetError> {
        let reload = self.reload.lock().map_err(|_| ConfigGetError::Internal)?;
        let bytes = self.files.read(&self.config_path).ok();
        let revision = bytes.as_deref().map(|bytes| self.files.revision(bytes));
        let parsed = bytes
            .as_deref()
            .map(|bytes| self.document.parse(bytes, &self.base_dir));
        let file = reload.classify_file(parsed);
        Ok(ConfigView {
            as_of: self.clock.wall(),
            revision,
            edit_enabled: reload.running().management.config_edit,
            file,
            last_reload: reload.last_reload().cloned(),
            items: config_items(reload.running(), reload.running_presence()),
        })
    }
    fn patch(
        &self,
        actor: ConfigActor,
        request: ConfigPatch,
    ) -> Result<ConfigEditResult, ConfigEditError> {
        let mut reload = self.reload.lock().map_err(|_| ConfigEditError::Internal)?;
        // Step 1 is checked before the edit switch or any file access. The same
        // guard then covers steps 2–10 and cannot race a watcher or another edit.
        if !valid_revision(&request.revision) || request.changes.is_empty() {
            return Err(ConfigEditError::InvalidRequest);
        }
        let mut seen = std::collections::HashSet::new();
        for change in &request.changes {
            if !seen.insert(&change.key) || !items::accepts_change(reload.running(), change) {
                return Err(ConfigEditError::InvalidRequest);
            }
        }
        let keys = || {
            request
                .changes
                .iter()
                .map(|change| change.key.clone())
                .collect::<Vec<_>>()
        };
        let reject = |error| self.reject(&actor, keys(), error);
        if !reload.running().management.config_edit {
            return Err(reject(ConfigEditError::EditDisabled));
        }
        let bytes = self.files.read(&self.config_path).map_err(|_| {
            reject(ConfigEditError::FileInvalid {
                diagnostics: vec![],
            })
        })?;
        if std::str::from_utf8(&bytes).is_err() {
            return Err(reject(ConfigEditError::FileInvalid {
                diagnostics: vec![],
            }));
        }
        let revision = self.files.revision(&bytes);
        if revision != request.revision {
            return Err(reject(ConfigEditError::Conflict));
        }
        let original = self
            .document
            .parse(&bytes, &self.base_dir)
            .map_err(|diagnostics| reject(ConfigEditError::FileInvalid { diagnostics }))?;
        match reload.prepare_candidate(original) {
            Ok(_) => {}
            Err(PrepareReloadError::Invalid(diagnostics)) => {
                return Err(reject(ConfigEditError::FileInvalid { diagnostics }));
            }
            Err(PrepareReloadError::RestartRequired(config_keys)) => {
                return Err(reject(ConfigEditError::RestartPending { config_keys }));
            }
        }
        // Compute requested file-value changes through the document port, not
        // running values: a pending file can already contain a requested value.
        let mut edited_keys = Vec::new();
        let mut next = None;
        for change in &request.changes {
            let before = next.as_deref().unwrap_or(bytes.as_slice());
            let edited = self
                .document
                .edit(before, std::slice::from_ref(change))
                .map_err(|diagnostics| reject(ConfigEditError::ConfigInvalid { diagnostics }))?;
            if edited != before {
                edited_keys.push(change.key.clone());
            }
            next = Some(edited);
        }
        // Step 1 guarantees at least one change, so an empty document is never
        // substituted for a missing edit.
        let next = next.ok_or(ConfigEditError::InvalidRequest)?;
        let candidate = self
            .document
            .parse(&next, &self.base_dir)
            .map_err(|diagnostics| reject(ConfigEditError::ConfigInvalid { diagnostics }))?;
        let prepared = reload.prepare_candidate(candidate).map_err(|error| {
            reject(match error {
                PrepareReloadError::Invalid(diagnostics) => {
                    ConfigEditError::ConfigInvalid { diagnostics }
                }
                PrepareReloadError::RestartRequired(config_keys) => {
                    ConfigEditError::RestartPending { config_keys }
                }
            })
        })?;
        if next == bytes {
            return Ok(ConfigEditResult {
                as_of: self.clock.wall(),
                revision,
                applied: vec![],
            });
        }
        // Allocate response/audit data before replacement. After replace there
        // is no fallible preparation, file read, shutdown veto or cancellation.
        let revision = self.files.revision(&next);
        let event = Event::ConfigEdited {
            admin_account: actor.admin_account.clone(),
            peer_ip: actor.peer_ip,
            config_keys: edited_keys,
            revision: revision.clone(),
        };
        self.files
            .replace(&self.config_path, &request.revision, &next)
            .map_err(|error| {
                reject(match error {
                    ConfigReplaceError::Conflict => ConfigEditError::Conflict,
                    ConfigReplaceError::WriteFailed => ConfigEditError::WriteFailed,
                })
            })?;
        let as_of = self.clock.wall();
        let applied = reload.commit_prepared(prepared, ReloadTrigger::ConfigEdit, as_of);
        self.events.record(&event);
        Ok(ConfigEditResult {
            as_of,
            revision,
            applied,
        })
    }
}

pub fn valid_revision(revision: &str) -> bool {
    revision.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

#[cfg(test)]
mod tests;
