//! OS notification side of the reload file watcher (docs/architectures/file-watch.md §4 step 1, §6).
//!
//! The notify callback runs on the backend thread. It only updates atomics and queues, then
//! nudges the async loop through a bounded channel. It never blocks.

use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use notify::event::{AccessKind, AccessMode, EventKind, ModifyKind};
use notify::{RecursiveMode, Watcher};
use tokio::sync::{Notify, mpsc};

use super::ReloadWatchError;

/// Capacity of the wake channel. A full channel loses a wake, which forces a full recheck.
const WAKE_CAPACITY: usize = 64;

/// Shared state between the notify callback and the watcher loop.
pub(super) struct ChangeQueue {
    /// Bumped once per event that names a watched file (file-watch.md §4 step 1).
    generation: AtomicU64,
    /// Set when a wake was lost. The next wake runs a full recheck (file-watch.md §6).
    overflow: AtomicBool,
    failures: Mutex<Vec<String>>,
    targets: Mutex<Vec<WatchTarget>>,
    wake: mpsc::Sender<()>,
    log_targets: Mutex<Vec<WatchTarget>>,
    pub(super) log_changed: Notify,
}

impl ChangeQueue {
    /// Creates the queue and the receiver that the watcher loop waits on.
    pub(super) fn new() -> (Arc<Self>, mpsc::Receiver<()>) {
        let (wake, receiver) = mpsc::channel(WAKE_CAPACITY);
        let queue = Self {
            generation: AtomicU64::new(0),
            overflow: AtomicBool::new(false),
            failures: Mutex::new(Vec::new()),
            targets: Mutex::new(Vec::new()),
            wake,
            log_targets: Mutex::new(Vec::new()),
            log_changed: Notify::new(),
        };
        (Arc::new(queue), receiver)
    }

    /// Current generation number (file-watch.md §3, §4 step 4).
    pub(super) fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    /// Returns true once per overflow and clears it.
    pub(super) fn take_overflow(&self) -> bool {
        self.overflow.swap(false, Ordering::SeqCst)
    }

    /// Returns the notify errors reported since the last call.
    pub(super) fn take_failures(&self) -> Vec<String> {
        std::mem::take(&mut *lock(&self.failures))
    }

    /// Replaces the files whose events count as changes.
    pub(super) fn set_targets(&self, targets: Vec<WatchTarget>) {
        *lock(&self.targets) = targets;
    }

    pub(super) fn set_log_targets(&self, targets: Vec<WatchTarget>) {
        *lock(&self.log_targets) = targets;
    }

    /// Handles one OS event (file-watch.md §4 step 1).
    pub(super) fn offer_event(&self, event: &notify::Event) {
        if event.need_rescan() {
            self.overflow.store(true, Ordering::SeqCst);
            self.log_changed.notify_one();
            self.nudge();
            return;
        }
        if matches!(
            event.kind,
            EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(ModifyKind::Name(_))
        ) && event.paths.iter().any(|path| {
            lock(&self.log_targets)
                .iter()
                .any(|target| target.matches(path))
        }) {
            self.log_changed.notify_one();
        }
        // Reads made by this process show up as Access events. Only a close after a write is a
        // change, so ignoring the rest keeps the watcher's own reads from starting an episode.
        let read_only = matches!(
            &event.kind,
            EventKind::Access(kind) if !matches!(kind, AccessKind::Close(AccessMode::Write))
        );
        if read_only {
            return;
        }
        for path in &event.paths {
            self.offer_path(path);
        }
    }

    /// Bumps the generation when `path` names a watched file (file-watch.md §4 step 1).
    /// Temporary file names match no target and are ignored.
    pub(super) fn offer_path(&self, path: &Path) {
        if lock(&self.targets)
            .iter()
            .any(|target| target.matches(path))
        {
            self.generation.fetch_add(1, Ordering::SeqCst);
            self.nudge();
        }
    }

    pub(super) fn offer_failure(&self, detail: String) {
        lock(&self.failures).push(detail);
        self.nudge();
    }

    /// Wakes the loop. A full channel loses the wake, so it is recorded as an overflow.
    fn nudge(&self) {
        if self.wake.try_send(()).is_err() {
            self.overflow.store(true, Ordering::SeqCst);
        }
    }
}

/// One watched file, identified by its directory and file name (file-watch.md §4 step 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WatchTarget {
    dir: PathBuf,
    name: OsString,
}

impl WatchTarget {
    /// Target for `path`, or `None` when `path` has no parent directory or no file name.
    pub(super) fn for_path(path: &Path) -> Option<Self> {
        let dir = path.parent()?;
        let name = path.file_name()?;
        Some(Self {
            dir: dir.to_path_buf(),
            name: name.to_os_string(),
        })
    }

    /// Directory that must be registered with the OS to see this file.
    pub(super) fn dir(&self) -> &Path {
        &self.dir
    }

    fn matches(&self, path: &Path) -> bool {
        path.parent() == Some(self.dir.as_path())
            && path
                .file_name()
                .is_some_and(|name| same_name(name, &self.name))
    }
}

/// Compares file names. Windows file names are case-insensitive.
fn same_name(left: &OsStr, right: &OsStr) -> bool {
    if cfg!(windows) {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    } else {
        left == right
    }
}

/// Registers directories with the OS notification API (file-watch.md §4 step 1, §6).
///
/// The trait lets tests replace the OS backend with a silent feed. The watcher otherwise depends
/// only on [`ChangeQueue`].
pub(super) trait ChangeFeed: Send {
    /// Makes `dirs` the registered set. Directories already registered stay registered.
    fn watch_dirs(&mut self, dirs: &BTreeSet<PathBuf>) -> Result<(), ReloadWatchError>;
}

/// Feed backed by the platform notification API through `notify::recommended_watcher`.
pub(super) struct NotifyFeed {
    watcher: notify::RecommendedWatcher,
    registered: BTreeSet<PathBuf>,
}

impl NotifyFeed {
    /// Creates the backend. Its callback feeds `queue`. No directory is registered yet.
    pub(super) fn new(queue: Arc<ChangeQueue>) -> Result<Self, ReloadWatchError> {
        let watcher =
            notify::recommended_watcher(
                move |result: notify::Result<notify::Event>| match result {
                    Ok(event) => queue.offer_event(&event),
                    Err(error) => queue.offer_failure(error.to_string()),
                },
            )
            .map_err(|error| ReloadWatchError::new(error.to_string()))?;
        Ok(Self {
            watcher,
            registered: BTreeSet::new(),
        })
    }
}

impl ChangeFeed for NotifyFeed {
    fn watch_dirs(&mut self, dirs: &BTreeSet<PathBuf>) -> Result<(), ReloadWatchError> {
        let stale: Vec<PathBuf> = self.registered.difference(dirs).cloned().collect();
        for dir in stale {
            // An unwatch error is ignored. The directory has left the watch set, and a leftover
            // OS registration only produces events that no target matches.
            let _ = self.watcher.unwatch(&dir);
            self.registered.remove(&dir);
        }
        for dir in dirs {
            if self.registered.contains(dir) {
                continue;
            }
            self.watcher
                .watch(dir, RecursiveMode::NonRecursive)
                .map_err(|error| ReloadWatchError::new(format!("{}: {error}", dir.display())))?;
            self.registered.insert(dir.clone());
        }
        Ok(())
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
