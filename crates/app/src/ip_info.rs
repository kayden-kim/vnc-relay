//! IP information databases (config-v4 §1.12): the two lookup slots that
//! event recording and the sources query read, and the service that reads
//! the files at start and again when their content changes.
//! Design: docs/architectures/source-stats.md §4.3, §4.4, §5.

#[cfg(test)]
mod tests;

use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock};
use std::time::SystemTime;

use vnc_relay_domain::ip_info::{self, Asn, CountryCode, Provider};

use crate::events::{Event, Phase};
use crate::ports::{Clock, EventSink, IpDbFileGateway, IpDbLookupGateway};

/// Longest `database_type` the query shows (management-api-v1 §1.17).
const DATABASE_TYPE_CHARS: usize = 128;

/// The two `[ip_info]` keys, in the order the query shows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IpDbKey {
    Country,
    Asn,
}

impl IpDbKey {
    pub const ALL: [Self; 2] = [Self::Country, Self::Asn];

    /// The full config key, which events write instead of the path.
    pub fn config_key(self) -> &'static str {
        match self {
            Self::Country => "ip_info.country_db",
            Self::Asn => "ip_info.asn_db",
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Country => 0,
            Self::Asn => 1,
        }
    }
}

/// Why a database cannot be used (config-v4 §1.12 원인 코드).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IpDbError {
    FileUnreadable,
    FileTooLarge,
    InvalidFormat,
    UnsupportedData,
}

impl IpDbError {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FileUnreadable => "file_unreadable",
            Self::FileTooLarge => "file_too_large",
            Self::InvalidFormat => "invalid_format",
            Self::UnsupportedData => "unsupported_data",
        }
    }
}

/// What the metadata of a database says (management-api-v1 §1.17).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpDbMetadata {
    /// The first 128 characters of the metadata `database_type`.
    pub database_type: String,
    pub build_time: SystemTime,
    /// Decided from the whole `database_type`.
    pub provider: Provider,
}

impl IpDbMetadata {
    pub fn new(database_type: &str, build_time: SystemTime) -> Self {
        Self {
            database_type: database_type.chars().take(DATABASE_TYPE_CHARS).collect(),
            build_time,
            provider: Provider::from_database_type(database_type),
        }
    }
}

/// What a file held when it was read. Equal fingerprints mean the same
/// content, so a read of content that already failed or is already in use
/// changes nothing (config-v4 §1.12 rule 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fingerprint {
    /// The file could not be opened or read.
    Unreadable,
    /// The file was over the limit; the value is its size.
    TooLarge(u64),
    /// A hash of the bytes read. It is only compared within one process.
    Content(u64),
}

/// A failed read: the cause and a sentence without path or file content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpDbReadError {
    pub failure: IpDbError,
    pub detail: Option<String>,
}

/// One read of one file by `IpDbFileGateway`.
pub struct IpDbRead {
    pub fingerprint: Fingerprint,
    pub result: Result<Arc<dyn IpDbLookupGateway>, IpDbReadError>,
}

/// The databases used now, one slot per key. Event recording and the
/// query read them; only `IpInfoService` replaces them.
pub struct IpLookup {
    slots: RwLock<[Option<Arc<dyn IpDbLookupGateway>>; 2]>,
}

impl IpLookup {
    /// Both slots start empty: no country and no ASN are found.
    pub fn new() -> Self {
        Self {
            slots: RwLock::new([None, None]),
        }
    }

    /// Country and AS number of `ip` in the databases used now. A local
    /// address is never looked up (management-api-v1 §1.17 `local`).
    pub fn find(&self, ip: IpAddr) -> (Option<CountryCode>, Option<Asn>) {
        if ip_info::is_local(ip) {
            return (None, None);
        }
        let slots = self.slots.read().unwrap_or_else(PoisonError::into_inner);
        let country = slots[IpDbKey::Country.index()]
            .as_ref()
            .and_then(|db| db.country(ip));
        let asn = slots[IpDbKey::Asn.index()]
            .as_ref()
            .and_then(|db| db.asn(ip));
        (country, asn)
    }

    /// The AS number and name of `ip` in the ASN database used now.
    pub fn as_name(&self, ip: IpAddr) -> Option<(Asn, String)> {
        if ip_info::is_local(ip) {
            return None;
        }
        let slots = self.slots.read().unwrap_or_else(PoisonError::into_inner);
        slots[IpDbKey::Asn.index()]
            .as_ref()
            .and_then(|db| db.as_name(ip))
    }

    pub(crate) fn set(&self, key: IpDbKey, db: Arc<dyn IpDbLookupGateway>) {
        let mut slots = self.slots.write().unwrap_or_else(PoisonError::into_inner);
        slots[key.index()] = Some(db);
    }
}

impl Default for IpLookup {
    fn default() -> Self {
        Self::new()
    }
}

/// `state` of one database in the query (management-api-v1 §1.17).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpDbState {
    NotConfigured,
    Loaded,
    Stale,
    Unavailable,
}

impl IpDbState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotConfigured => "not_configured",
            Self::Loaded => "loaded",
            Self::Stale => "stale",
            Self::Unavailable => "unavailable",
        }
    }
}

/// The database a key uses now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InUse {
    pub metadata: IpDbMetadata,
    pub loaded_at: SystemTime,
}

/// One key's status in the query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpDbStatus {
    pub state: IpDbState,
    /// The last failure while it lasts: present for `Stale` and for an
    /// `Unavailable` key whose file was read at least once.
    pub error: Option<IpDbError>,
    pub in_use: Option<InUse>,
}

/// Per-key state of the service.
#[derive(Default)]
struct KeyState {
    current: Option<Current>,
    /// The last failure and the content that failed. Cleared by a success.
    failure: Option<(IpDbError, Fingerprint)>,
}

struct Current {
    in_use: InUse,
    fingerprint: Fingerprint,
}

/// Reads the configured database files and keeps each key's state
/// (source-stats.md §4.3, §4.4). Reads never block the relay: a failure
/// keeps the previous database and only writes a warning.
pub struct IpInfoService {
    paths: [Option<PathBuf>; 2],
    loader: Arc<dyn IpDbFileGateway>,
    lookup: Arc<IpLookup>,
    clock: Arc<dyn Clock>,
    log: Arc<dyn EventSink>,
    states: Mutex<[KeyState; 2]>,
}

impl IpInfoService {
    /// `country_db` and `asn_db` are the `[ip_info]` paths, already
    /// resolved against the config file directory.
    pub fn new(
        country_db: Option<PathBuf>,
        asn_db: Option<PathBuf>,
        loader: Arc<dyn IpDbFileGateway>,
        lookup: Arc<IpLookup>,
        clock: Arc<dyn Clock>,
        log: Arc<dyn EventSink>,
    ) -> Self {
        Self {
            paths: [country_db, asn_db],
            loader,
            lookup,
            clock,
            log,
            states: Mutex::new(Default::default()),
        }
    }

    /// The distinct configured paths, for the file watcher.
    pub fn paths(&self) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = Vec::new();
        for path in self.paths.iter().flatten() {
            if !paths.contains(path) {
                paths.push(path.clone());
            }
        }
        paths
    }

    /// The first read of every configured key (config-v4 §1.12 rule 1).
    pub fn load_startup(&self) {
        self.read_keys(|_| true, Phase::Startup);
    }

    /// Reads again the keys whose file content changed (rule 2, 3). The
    /// watcher passes the paths whose content hash differs.
    pub fn refresh(&self, changed: &[PathBuf]) {
        self.read_keys(
            |path| changed.iter().any(|item| item == path),
            Phase::Reload,
        );
    }

    /// Both keys' status, country first.
    pub fn status(&self) -> [IpDbStatus; 2] {
        let states = lock(&self.states);
        IpDbKey::ALL.map(|key| {
            let state = &states[key.index()];
            let in_use = state.current.as_ref().map(|current| current.in_use.clone());
            let error = state.failure.map(|(failure, _)| failure);
            let kind = match (&self.paths[key.index()], &state.current, error) {
                (None, _, _) => IpDbState::NotConfigured,
                (Some(_), Some(_), None) => IpDbState::Loaded,
                (Some(_), Some(_), Some(_)) => IpDbState::Stale,
                (Some(_), None, _) => IpDbState::Unavailable,
            };
            IpDbStatus {
                state: kind,
                error,
                in_use,
            }
        })
    }

    /// Reads each wanted path once, outside the state lock, then settles
    /// every key that uses it. Events are written after the lock is let go.
    fn read_keys(&self, wanted: impl Fn(&Path) -> bool, phase: Phase) {
        let mut reads: Vec<(&Path, IpDbRead)> = Vec::new();
        for path in self.paths.iter().flatten() {
            if wanted(path) && !reads.iter().any(|(read, _)| *read == path.as_path()) {
                reads.push((path, self.loader.read(path)));
            }
        }
        if reads.is_empty() {
            return;
        }
        let mut events = Vec::new();
        {
            let mut states = lock(&self.states);
            for key in IpDbKey::ALL {
                let Some(path) = &self.paths[key.index()] else {
                    continue;
                };
                let Some((_, read)) = reads.iter().find(|(read, _)| *read == path.as_path()) else {
                    continue;
                };
                if let Some(event) = self.settle(&mut states[key.index()], key, read, phase) {
                    events.push(event);
                }
            }
        }
        for event in &events {
            self.log.record(event);
        }
    }

    /// The outcome of one read for one key (source-stats.md §4.4 step 3).
    fn settle(
        &self,
        state: &mut KeyState,
        key: IpDbKey,
        read: &IpDbRead,
        phase: Phase,
    ) -> Option<Event> {
        let error = match &read.result {
            Ok(db) if db.supports(key) => {
                let unchanged = state
                    .current
                    .as_ref()
                    .is_some_and(|current| current.fingerprint == read.fingerprint);
                if unchanged && state.failure.is_none() {
                    return None;
                }
                self.lookup.set(key, Arc::clone(db));
                let metadata = db.metadata().clone();
                let event = Event::IpDbLoaded {
                    key,
                    database_type: metadata.database_type.clone(),
                    build_time: metadata.build_time,
                    phase,
                };
                state.current = Some(Current {
                    in_use: InUse {
                        metadata,
                        loaded_at: self.clock.wall(),
                    },
                    fingerprint: read.fingerprint,
                });
                state.failure = None;
                return Some(event);
            }
            Ok(_) => IpDbReadError {
                failure: IpDbError::UnsupportedData,
                detail: Some(missing_field(key).to_owned()),
            },
            Err(error) => error.clone(),
        };
        if state.failure == Some((error.failure, read.fingerprint)) {
            return None;
        }
        state.failure = Some((error.failure, read.fingerprint));
        Some(Event::IpDbUnavailable {
            key,
            reason: error.failure,
            phase,
            detail: error.detail,
        })
    }
}

/// `detail` of `unsupported_data`: the fields that were looked for.
fn missing_field(key: IpDbKey) -> &'static str {
    match key {
        IpDbKey::Country => "국가 필드(country_code 또는 country.iso_code)를 찾지 못했다.",
        IpDbKey::Asn => "ASN 필드(asn 또는 autonomous_system_number)를 찾지 못했다.",
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    // Nothing under this lock panics; a poisoned value is still whole.
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
