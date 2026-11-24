//! Automatic partial reload: the file watcher (docs/architectures/file-watch.md §2–§6, §9).
//!
//! `ReloadWatcher` owns the OS notification feed, the poll timer, and the processed state. A change
//! starts an episode. The episode waits for a quiet window, checks file stamps across a gap, reads a
//! candidate, and hands it to `ReloadService`, which decides the result.

mod feed;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::pin::pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use feed::{ChangeQueue, NotifyWatchGateway, WatchGateway, WatchTarget};
use tokio::sync::{mpsc, watch};
use vnc_relay_application::events::Event;
use vnc_relay_application::ip_info::IpInfoService;
use vnc_relay_application::ports::EventSink;
use vnc_relay_application::reload::ports::{ReloadClock, ReloadFileStore};
use vnc_relay_application::reload::service::ReloadOutcome;
use vnc_relay_application::reload::{ChangeSource, SharedReload, WatchError};

use crate::config_file;
use crate::ops_log::JsonLinesEventSink;

/// Quiet window Q before a candidate is read (file-watch.md §4 step 3 (a), §9 item 1).
/// Implementation constant. Q + G + read + verify + swap must stay below the apply limit D
/// (`settle_secs`). The poll interval Dp (`poll_secs`) sets detection, not this bound.
const QUIET: Duration = Duration::from_millis(300);

/// Gap G between the two stamp recordings (file-watch.md §4 step 3 (c), §9 item 1).
/// Implementation constant.
const GAP: Duration = Duration::from_millis(100);

/// Read size when hashing IP information database files, which can be large.
const HASH_CHUNK: usize = 64 * 1024;

/// Failure to start the file watcher. The caller reports `startup_failed` with `watch_unavailable`
/// and exits 1 (file-watch.md §6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReloadWatchError {
    detail: String,
}

impl ReloadWatchError {
    /// Creates the error from the backend detail text.
    pub fn new(detail: String) -> Self {
        Self { detail }
    }

    /// Backend detail text, without the message prefix.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for ReloadWatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "파일 감시를 할 수 없다. {}", self.detail)
    }
}

impl std::error::Error for ReloadWatchError {}

/// `ReloadFileStore` backed by the file system (file-watch.md §4 step 3).
#[derive(Debug, Clone, Copy, Default)]
pub struct FsReloadFileStore;

impl ReloadFileStore for FsReloadFileStore {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(path)
    }
}

/// `ReloadClock` backed by the tokio timer (file-watch.md §9 item 13).
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemReloadClock;

impl ReloadClock for SystemReloadClock {
    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + Send {
        tokio::time::sleep(duration)
    }
}

/// Starts the file watcher for the config file at `config_path` (file-watch.md §5, §9 item 11).
///
/// Registers the OS watches first, then reads the baseline content hashes. An error means the
/// watcher could not start.
pub fn start<A: Clone, C: ReloadClock>(
    config_path: &Path,
    service: SharedReload<A>,
    poll: Duration,
    clock: C,
    files: Arc<dyn ReloadFileStore>,
    log: Arc<dyn EventSink>,
    output: Arc<JsonLinesEventSink>,
) -> Result<ReloadWatcher<A, C>, ReloadWatchError> {
    let mut watcher = assemble(config_path, service, poll, clock, files, log, |queue| {
        let feed: Box<dyn WatchGateway> = Box::new(NotifyWatchGateway::new(queue)?);
        Ok(feed)
    })?;
    watcher.attach_log(output);
    Ok(watcher)
}

/// Builds the watcher around `make_feed`, so tests can replace the OS backend.
fn assemble<A: Clone, C: ReloadClock>(
    config_path: &Path,
    service: SharedReload<A>,
    poll: Duration,
    clock: C,
    files: Arc<dyn ReloadFileStore>,
    log: Arc<dyn EventSink>,
    make_feed: impl FnOnce(Arc<ChangeQueue>) -> Result<Box<dyn WatchGateway>, ReloadWatchError>,
) -> Result<ReloadWatcher<A, C>, ReloadWatchError> {
    let config_path = std::path::absolute(config_path)
        .map_err(|error| ReloadWatchError::new(format!("{}: {error}", config_path.display())))?;
    // Same rule as `config_file::load`: relative paths resolve against the config file's directory.
    let base_dir = config_path.parent().unwrap_or(Path::new("")).to_path_buf();
    let (queue, wake) = ChangeQueue::new();
    let feed = make_feed(Arc::clone(&queue))?;
    let mut watcher = ReloadWatcher {
        config_path,
        base_dir,
        service,
        feed,
        queue,
        wake,
        poll,
        clock: Arc::new(clock),
        files,
        log,
        watched: Vec::new(),
        processed: BTreeMap::new(),
        notify_failed: false,
        seen_generation: 0,
        log_output: None,
        ip_db: None,
        optional_missing: false,
    };
    watcher.attach()?;
    Ok(watcher)
}

/// Automatic partial reload watcher (file-watch.md §2–§6). Drive it with [`ReloadWatcher::run`].
pub struct ReloadWatcher<A, C> {
    config_path: PathBuf,
    base_dir: PathBuf,
    service: SharedReload<A>,
    feed: Box<dyn WatchGateway>,
    queue: Arc<ChangeQueue>,
    wake: mpsc::Receiver<()>,
    poll: Duration,
    clock: Arc<C>,
    files: Arc<dyn ReloadFileStore>,
    log: Arc<dyn EventSink>,
    log_output: Option<Arc<JsonLinesEventSink>>,
    /// Files watched now: the config file plus the service's TLS paths (file-watch.md §9 item 9).
    watched: Vec<PathBuf>,
    /// Content hash per watched file at the last processed state. `None` means the file is missing
    /// (file-watch.md §4 step 2).
    processed: BTreeMap<PathBuf, Option<u64>>,
    /// True while the last notify registration failed. Each poll tick retries it (file-watch.md §6).
    notify_failed: bool,
    /// Generation whose changes have already been handed to the service.
    seen_generation: u64,
    /// IP information database files, re-read by their own rules (config-v4 §1.12).
    ip_db: Option<IpDbWatch>,
    /// True while a directory registered only for the database files could not be registered.
    /// Each poll tick retries it without a warning (config-v4 §1.12 rule 2).
    optional_missing: bool,
}

/// The database files the watcher hashes and the service that re-reads them.
struct IpDbWatch {
    service: Arc<IpInfoService>,
    /// The configured paths, as the service names them.
    paths: Vec<PathBuf>,
    /// Content hash per path at the last read. `None` means missing or unreadable.
    processed: BTreeMap<PathBuf, Option<u64>>,
}

/// Outcome of one `run` loop turn that is not an episode step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flow {
    Continue,
    Stop,
}

/// Result of the quiet-window wait.
enum Quiet {
    Settled,
    Restarted,
    Stop,
}

/// Event that woke the `run` loop.
enum Step {
    Wake,
    Tick,
    Recheck,
    Stop,
}

/// Size and modification time of a watched file (file-watch.md §4 step 3 (b)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileStamp {
    len: u64,
    modified: Option<SystemTime>,
}

/// Config and content hashes read at the end of stabilization (file-watch.md §4 step 3 (e)).
struct Candidate {
    config: Option<Vec<u8>>,
    hashes: BTreeMap<PathBuf, Option<u64>>,
}

/// Targets and directories for a set of watched files.
struct WatchPlan {
    targets: Vec<WatchTarget>,
    dirs: BTreeSet<PathBuf>,
}

/// Content hash of file bytes (file-watch.md §9 item 3). Used only within one process.
fn content_hash(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// Hash of a file's current content, or `None` when it is missing or unreadable (file-watch.md §4 step 2).
fn hash_of(files: &dyn ReloadFileStore, path: &Path) -> Option<u64> {
    files.read(path).ok().map(|bytes| content_hash(&bytes))
}

/// Hash of a file's content read in chunks, or `None` when it cannot be read. Database files can
/// be hundreds of megabytes, so they are never held whole here (source-stats.md §7).
fn stream_hash(path: &Path) -> Option<u64> {
    let mut file = File::open(path).ok()?;
    let mut hasher = DefaultHasher::new();
    let mut chunk = vec![0; HASH_CHUNK];
    loop {
        match file.read(&mut chunk) {
            Ok(0) => return Some(hasher.finish()),
            Ok(read) => hasher.write(chunk.get(..read)?),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => return None,
        }
    }
}

fn stream_hashes(paths: Vec<PathBuf>) -> BTreeMap<PathBuf, Option<u64>> {
    paths
        .into_iter()
        .map(|path| {
            let hash = stream_hash(&path);
            (path, hash)
        })
        .collect()
}

fn stamp_of(path: &Path) -> Option<FileStamp> {
    let meta = std::fs::metadata(path).ok()?;
    Some(FileStamp {
        len: meta.len(),
        modified: meta.modified().ok(),
    })
}

/// Computes the watch targets for `paths`. A symlink also registers the directory of its resolved
/// target (file-watch.md §9 item 6).
fn watch_plan(paths: &[PathBuf]) -> Result<WatchPlan, ReloadWatchError> {
    let mut plan = WatchPlan {
        targets: Vec::new(),
        dirs: BTreeSet::new(),
    };
    for path in paths {
        let absolute = std::path::absolute(path)
            .map_err(|error| ReloadWatchError::new(format!("{}: {error}", path.display())))?;
        add_target(&mut plan, &absolute);
        let is_link = std::fs::symlink_metadata(&absolute).is_ok_and(|meta| meta.is_symlink());
        if is_link && let Ok(resolved) = std::fs::canonicalize(&absolute) {
            add_target(&mut plan, &resolved);
        }
    }
    Ok(plan)
}

fn add_target(plan: &mut WatchPlan, path: &Path) {
    let Some(target) = WatchTarget::for_path(path) else {
        return;
    };
    if plan.targets.contains(&target) {
        return;
    }
    plan.dirs.insert(target.dir().to_path_buf());
    plan.targets.push(target);
}

impl<A: Clone, C: ReloadClock> ReloadWatcher<A, C> {
    /// Registers the watches and records the baseline hashes (file-watch.md §9 item 11).
    fn attach(&mut self) -> Result<(), ReloadWatchError> {
        self.watched = self.current_watched();
        self.sync_notify()?;
        self.processed = self
            .watched
            .iter()
            .map(|path| (path.clone(), hash_of(self.files.as_ref(), path)))
            .collect();
        Ok(())
    }

    /// The config file plus every path the service watches (file-watch.md §1.8, §9 item 9).
    fn current_watched(&self) -> Vec<PathBuf> {
        let mut paths = vec![self.config_path.clone()];
        for path in self
            .service
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .watched_paths()
        {
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
        paths
    }

    fn sync_notify(&mut self) -> Result<(), ReloadWatchError> {
        let plan = watch_plan(&self.watched)?;
        let mut targets = plan.targets;
        let mut db_dirs = BTreeSet::new();
        let mut hidden = Vec::new();
        if let Some(ip_db) = &self.ip_db {
            hidden.extend(ip_db.paths.iter().cloned());
            // A database path that cannot be resolved is only polled; its error names the path.
            if let Ok(db_plan) = watch_plan(&ip_db.paths) {
                for target in db_plan.targets {
                    hidden.push(target.path());
                    if !targets.contains(&target) {
                        targets.push(target);
                    }
                }
                db_dirs = db_plan.dirs;
            }
        }
        self.queue.set_targets(targets);
        // `assemble` first registers config/TLS alone; `attach_log` then adds
        // optional log directories without making their failure fatal (Q111).
        self.sync_log_notify(plan.dirs, db_dirs, hidden)
    }

    fn attach_log(&mut self, output: Arc<JsonLinesEventSink>) {
        if output.file_path().is_none() {
            return;
        }
        self.log_output = Some(output);
        self.resync_notify();
    }

    /// Registers `dirs` and the log directory as required, and the directories needed only by
    /// the database files as optional.
    fn sync_log_notify(
        &mut self,
        mut dirs: BTreeSet<PathBuf>,
        db_dirs: BTreeSet<PathBuf>,
        mut hidden: Vec<PathBuf>,
    ) -> Result<(), ReloadWatchError> {
        if let Some(path) = self.log_output.as_ref().and_then(|log| log.file_path()) {
            let plan = watch_plan(&[path.to_path_buf()])?;
            self.queue.set_log_targets(plan.targets);
            dirs.extend(plan.dirs);
        }
        let optional: BTreeSet<PathBuf> = db_dirs.difference(&dirs).cloned().collect();
        hidden.extend(optional.iter().cloned());
        self.queue.set_hidden(hidden);
        let complete = self.feed.watch_dirs(&dirs, &optional)?;
        self.optional_missing = !complete;
        Ok(())
    }

    /// Re-registers the watches. A failure is reported again on every attempt (file-watch.md §6, §9 item 8).
    fn resync_notify(&mut self) {
        match self.sync_notify() {
            Ok(()) => self.notify_failed = false,
            Err(error) => self.report_notify_error(error.detail()),
        }
    }

    /// Records `reload_watch_failed` with `notify_error`. The next poll tick retries registration
    /// (file-watch.md §6, §9 item 5).
    fn report_notify_error(&mut self, detail: &str) {
        self.notify_failed = true;
        self.log.record(&Event::ReloadWatchFailed {
            reason: WatchError::NotifyError,
            detail: Some(detail.to_owned()),
        });
    }

    fn report_notify_failures(&mut self) {
        for detail in self.queue.take_failures() {
            self.report_notify_error(&detail);
        }
    }

    /// Reads the config file and every watched file for a candidate (file-watch.md §4 step 3 (e)).
    /// Unreadable files become `None`.
    fn read_candidate(&self) -> Candidate {
        let mut config = None;
        let mut hashes = BTreeMap::new();
        for path in &self.watched {
            let bytes = self.files.read(path).ok();
            hashes.insert(path.clone(), bytes.as_deref().map(content_hash));
            if *path == self.config_path {
                config = bytes;
            }
        }
        Candidate { config, hashes }
    }

    /// Hashes every watched file now. An unexpected I/O error is returned as its detail text
    /// (file-watch.md §4 step 2, §6).
    fn read_poll(&self) -> Result<BTreeMap<PathBuf, Option<u64>>, String> {
        let mut hashes = BTreeMap::new();
        for path in &self.watched {
            let hash = match self.files.read(path) {
                Ok(bytes) => Some(content_hash(&bytes)),
                Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                Err(error) => return Err(format!("{}: {error}", path.display())),
            };
            hashes.insert(path.clone(), hash);
        }
        Ok(hashes)
    }

    /// Stamps of the watched files and the database files, so a database copy in progress also
    /// holds the episode back (file-watch.md §4 step 3).
    fn stamps(&self) -> Vec<Option<FileStamp>> {
        let db_paths = self.ip_db.iter().flat_map(|ip_db| ip_db.paths.iter());
        self.watched
            .iter()
            .chain(db_paths)
            .map(|path| stamp_of(path))
            .collect()
    }

    /// Makes the candidate's hashes the processed state and re-derives the watch set (file-watch.md §4 step 5, §9 item 9).
    fn refresh_watch_set(&mut self, mut hashes: BTreeMap<PathBuf, Option<u64>>) {
        self.watched = self.current_watched();
        self.processed = self
            .watched
            .iter()
            .map(|path| {
                let hash = match hashes.remove(path) {
                    Some(hash) => hash,
                    // A path that joined the watch set after the candidate was read is read now.
                    None => hash_of(self.files.as_ref(), path),
                };
                (path.clone(), hash)
            })
            .collect();
        self.resync_notify();
    }

    /// The final read, parse, prepare and commit share the same lock as PATCH. `None` means the
    /// files moved and stabilization starts again; otherwise whether the candidate was handled.
    fn apply(
        &mut self,
        window: u64,
        before: &[Option<FileStamp>],
        stop: &watch::Receiver<bool>,
    ) -> Option<bool> {
        let shared = Arc::clone(&self.service);
        let mut service = shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let candidate = self.read_candidate();
        if self.stamps() != before || self.queue.generation() != window {
            return None;
        }
        let queue = Arc::clone(&self.queue);
        let is_current = || queue.generation() == window && !*stop.borrow();
        let at = SystemTime::now();
        let outcome = match candidate.config.as_deref().map(std::str::from_utf8) {
            Some(Ok(text)) => service.handle_config(
                config_file::parse_candidate(text.as_bytes(), &self.base_dir),
                &is_current,
                at,
            ),
            _ => service.handle_unreadable(&is_current, at),
        };
        drop(service);
        self.seen_generation = window;
        let handled = !matches!(outcome, ReloadOutcome::Discarded);
        if handled {
            self.refresh_watch_set(candidate.hashes);
        }
        Some(handled)
    }

    /// Content hashes of the database files, computed off the async threads. Takes `&mut self`
    /// so the future stays `Send`: the change feed is not `Sync`.
    async fn db_hashes(&mut self) -> Option<BTreeMap<PathBuf, Option<u64>>> {
        let paths = self.ip_db.as_ref()?.paths.clone();
        tokio::task::spawn_blocking(move || stream_hashes(paths))
            .await
            .ok()
    }

    /// True when a database file differs from the content last read.
    async fn ip_db_changed(&mut self) -> bool {
        match (self.db_hashes().await, &self.ip_db) {
            (Some(hashes), Some(ip_db)) => hashes != ip_db.processed,
            _ => false,
        }
    }

    /// Re-reads the database files whose content changed (config-v4 §1.12 rule 2, 3). Runs after
    /// the config candidate, outside the reload lock, so a database never blocks a config reload
    /// and the result never appears in `reload_applied`. Awaited, so its events come before
    /// `server_stopped`.
    async fn refresh_ip_db(&mut self, stop: &watch::Receiver<bool>) {
        let Some(hashes) = self.db_hashes().await else {
            return;
        };
        let Some(ip_db) = self.ip_db.as_mut() else {
            return;
        };
        let changed: Vec<PathBuf> = hashes
            .iter()
            .filter(|(path, hash)| ip_db.processed.get(*path) != Some(*hash))
            .map(|(path, _)| path.clone())
            .collect();
        ip_db.processed = hashes;
        if changed.is_empty() || *stop.borrow() {
            return;
        }
        let service = Arc::clone(&ip_db.service);
        // A panic inside the read is not expected; the previous databases stay in use.
        let _ = tokio::task::spawn_blocking(move || service.refresh(&changed)).await;
    }

    /// Waits for the quiet window Q. A notify event or a stop signal ends the wait early.
    async fn wait_quiet(&mut self, stop: &mut watch::Receiver<bool>) -> Quiet {
        if *stop.borrow() {
            return Quiet::Stop;
        }
        tokio::select! {
            () = self.clock.sleep(QUIET) => Quiet::Settled,
            message = self.wake.recv() => match message {
                Some(()) => Quiet::Restarted,
                None => Quiet::Stop,
            },
            changed = stop.changed() => match changed {
                Ok(()) => Quiet::Restarted,
                Err(_) => Quiet::Stop,
            },
        }
    }

    /// One change episode (file-watch.md §4 steps 1–5). Records `reload_change_detected` once, then
    /// stabilizes the files and applies the candidate. An event during stabilization restarts it.
    async fn episode(&mut self, source: ChangeSource, stop: &mut watch::Receiver<bool>) -> Flow {
        self.log.record(&Event::ReloadChangeDetected { source });
        loop {
            let window = self.queue.generation();
            match self.wait_quiet(stop).await {
                Quiet::Stop => return Flow::Stop,
                Quiet::Restarted => {
                    self.report_notify_failures();
                    continue;
                }
                Quiet::Settled => {}
            }
            if self.queue.generation() != window {
                continue;
            }
            // (b) stamp, (c) gap, (d) stamp compared with (b), (e) read, (f) stamp compared with (b).
            let before = self.stamps();
            self.clock.sleep(GAP).await;
            if *stop.borrow() {
                return Flow::Stop;
            }
            if self.stamps() != before || self.queue.generation() != window {
                continue;
            }
            let Some(handled) = self.apply(window, &before, stop) else {
                continue;
            };
            if handled {
                self.refresh_ip_db(stop).await;
            }
            return Flow::Continue;
        }
    }

    /// A notify wake. A lost wake (overflow) forces a full recheck. Otherwise a new generation
    /// starts an episode (file-watch.md §4 step 1, §6).
    async fn on_wake(&mut self, stop: &mut watch::Receiver<bool>) -> Flow {
        self.report_notify_failures();
        if self.queue.take_overflow() {
            return self.poll_check(stop).await;
        }
        if self.queue.generation() != self.seen_generation {
            return self.episode(ChangeSource::OsNotify, stop).await;
        }
        Flow::Continue
    }

    /// A poll tick. Retries registration when it failed, then compares the files with the processed
    /// state (file-watch.md §4 step 2, §6).
    async fn on_tick(&mut self, stop: &mut watch::Receiver<bool>) -> Flow {
        self.report_notify_failures();
        if self.notify_failed || self.optional_missing {
            self.resync_notify();
        }
        self.poll_check(stop).await
    }

    /// Compares the files with the processed state. A difference starts a poll episode. A config
    /// or TLS read error is recorded as `reload_watch_failed` with `poll_error` on every tick
    /// (file-watch.md §6, §9 item 8). A database file that cannot be read is not a watch failure:
    /// its read reports `file_unreadable` (config-v4 §1.12 rule 2).
    async fn poll_check(&mut self, stop: &mut watch::Receiver<bool>) -> Flow {
        let config = self.read_poll();
        let db_changed = self.ip_db_changed().await;
        match config {
            Ok(hashes) if hashes == self.processed && !db_changed => Flow::Continue,
            Ok(_) => self.episode(ChangeSource::Poll, stop).await,
            Err(detail) => {
                self.log.record(&Event::ReloadWatchFailed {
                    reason: WatchError::PollError,
                    detail: Some(detail),
                });
                if db_changed {
                    self.episode(ChangeSource::Poll, stop).await
                } else {
                    Flow::Continue
                }
            }
        }
    }
}

impl<A: Clone, C: ReloadClock> ReloadWatcher<A, C> {
    /// Watches the IP information database files of `service` too (config-v4 §1.12 rule 2).
    /// Their content is hashed now as the baseline, so call this before the first read. A
    /// directory that cannot be registered is polled only and retried on each tick, without a
    /// warning; no error names a database path.
    pub fn attach_ip_info(&mut self, service: Arc<IpInfoService>) {
        let paths = service.paths();
        if paths.is_empty() {
            return;
        }
        let processed = stream_hashes(paths.clone());
        self.ip_db = Some(IpDbWatch {
            service,
            paths,
            processed,
        });
        self.resync_notify();
    }

    /// Runs the watcher until `stop` becomes true or its sender is dropped (file-watch.md §4, §5).
    pub async fn run(mut self, stop: watch::Receiver<bool>) {
        let Some(output) = self.log_output.take() else {
            self.run_reload(stop).await;
            return;
        };
        // Keep the output available to notification registration refreshes.
        self.log_output = Some(output.clone());
        let queue = Arc::clone(&self.queue);
        let clock = Arc::clone(&self.clock);
        let poll = self.poll;
        tokio::select! {
            () = self.run_reload(stop.clone()) => {},
            () = reopen_log(output, queue, clock, poll, stop) => {},
        }
    }

    async fn run_reload(mut self, mut stop: watch::Receiver<bool>) {
        let clock = Arc::clone(&self.clock);
        let mut tick = pin!(clock.sleep(self.poll));
        loop {
            if *stop.borrow() {
                break;
            }
            let step = tokio::select! {
                message = self.wake.recv() => match message {
                    Some(()) => Step::Wake,
                    None => Step::Stop,
                },
                () = tick.as_mut() => Step::Tick,
                changed = stop.changed() => match changed {
                    Ok(()) => Step::Recheck,
                    Err(_) => Step::Stop,
                },
            };
            let flow = match step {
                Step::Wake => self.on_wake(&mut stop).await,
                Step::Tick => {
                    let flow = self.on_tick(&mut stop).await;
                    tick.set(clock.sleep(self.poll));
                    flow
                }
                Step::Recheck => Flow::Continue,
                Step::Stop => Flow::Stop,
            };
            if flow == Flow::Stop {
                break;
            }
        }
    }
}

/// Independent timer: long-running reload episodes cannot delay log reopen.
async fn reopen_log<C: ReloadClock>(
    output: Arc<JsonLinesEventSink>,
    queue: Arc<ChangeQueue>,
    clock: Arc<C>,
    poll: Duration,
    mut stop: watch::Receiver<bool>,
) {
    let mut tick = pin!(clock.sleep(poll));
    loop {
        if *stop.borrow() {
            return;
        }
        tokio::select! {
            () = queue.log_changed.notified() => output.reopen(),
            () = tick.as_mut() => {
                output.reopen();
                tick.set(clock.sleep(poll));
            },
            changed = stop.changed() => {
                if changed.is_err() || *stop.borrow() {
                    return;
                }
            },
        }
    }
}

#[cfg(test)]
mod reopen_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::sync::oneshot;
    use vnc_relay_application::reload::ports::{ReloadTlsStore, TlsBuildError};
    use vnc_relay_application::reload::service::ReloadService;
    use vnc_relay_application::reload::snapshot::{self, ReloadSnapshot};
    use vnc_relay_domain::allow_list::{AllowList, IpItem};
    use vnc_relay_domain::peer_ip::PeerIp;

    const CONFIG: &str = "[admin]\naccounts_file = \"admin-accounts.toml\"\n\n[management]\nlisten = \"127.0.0.1:18080\"\n\n[status]\nlisten = \"127.0.0.1:18081\"\n";
    const INVALID_CONFIG: &str = "not = [valid toml\n";
    const POLL: Duration = Duration::from_secs(30);

    const RELAY_CONFIG: &str = "[admin]\naccounts_file = \"admin-accounts.toml\"\n\n[management]\nlisten = \"127.0.0.1:18080\"\n\n[status]\nlisten = \"127.0.0.1:18081\"\n\n[relay.tcp_viewer]\nenabled = true\nlisten = \"127.0.0.1:5901\"\nallow = [\"127.0.0.1\"]\n";
    const RELAY_CANDIDATE: &str = "[admin]\naccounts_file = \"admin-accounts.toml\"\n\n[management]\nlisten = \"127.0.0.1:18080\"\n\n[status]\nlisten = \"127.0.0.1:18081\"\n\n[relay.tcp_viewer]\nenabled = true\nlisten = \"127.0.0.1:5901\"\nallow = [\"127.0.0.2\"]\n";

    type SleepRequest = (Duration, oneshot::Sender<()>);

    /// Clock whose sleeps finish only when the test releases them.
    struct FakeReloadClock {
        requests: mpsc::UnboundedSender<SleepRequest>,
    }

    impl ReloadClock for FakeReloadClock {
        fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + Send {
            let (release, released) = oneshot::channel();
            let requested = self.requests.send((duration, release)).is_ok();
            async move {
                if requested {
                    let _ = released.await;
                } else {
                    std::future::pending::<()>().await;
                }
            }
        }
    }

    async fn next_sleep(
        sleeps: &mut mpsc::UnboundedReceiver<SleepRequest>,
        expected: Duration,
    ) -> oneshot::Sender<()> {
        let (duration, release) = sleeps.recv().await.expect("the watcher requests a sleep");
        assert_eq!(duration, expected);
        release
    }

    /// Feed that refuses every registration while `refuse` is set. It never delivers OS events, so
    /// the poll tick is the only detector, as when the notify backend is blocked (OPS-041).
    struct FakeWatchGateway {
        refuse: Arc<AtomicBool>,
    }

    impl WatchGateway for FakeWatchGateway {
        fn watch_dirs(
            &mut self,
            _required: &BTreeSet<PathBuf>,
            _optional: &BTreeSet<PathBuf>,
        ) -> Result<bool, ReloadWatchError> {
            if self.refuse.load(Ordering::SeqCst) {
                Err(ReloadWatchError::new("registration refused".to_owned()))
            } else {
                Ok(true)
            }
        }
    }

    struct StubReloadTlsStore;

    impl ReloadTlsStore<()> for StubReloadTlsStore {
        fn build(&self, _cert_pem: &[u8], _key_pem: &[u8]) -> Result<(), TlsBuildError> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct SpyEventSink {
        events: Mutex<Vec<Event>>,
    }

    impl SpyEventSink {
        fn names(&self) -> Vec<&'static str> {
            self.events
                .lock()
                .expect("sink lock")
                .iter()
                .map(Event::name)
                .collect()
        }

        fn snapshot(&self) -> Vec<Event> {
            self.events.lock().expect("sink lock").clone()
        }
    }

    impl EventSink for SpyEventSink {
        fn record(&self, event: &Event) {
            self.events.lock().expect("sink lock").push(event.clone());
        }
    }

    #[derive(Default)]
    struct FakeReloadFileStore {
        files: Mutex<BTreeMap<PathBuf, Vec<u8>>>,
        denied: Mutex<BTreeSet<PathBuf>>,
    }

    impl FakeReloadFileStore {
        fn set(&self, path: &Path, bytes: &[u8]) {
            self.files
                .lock()
                .expect("files lock")
                .insert(path.to_path_buf(), bytes.to_vec());
        }

        fn deny(&self, path: &Path) {
            self.denied
                .lock()
                .expect("denied lock")
                .insert(path.to_path_buf());
        }
    }

    impl ReloadFileStore for FakeReloadFileStore {
        fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
            if self.denied.lock().expect("denied lock").contains(path) {
                return Err(io::Error::from(io::ErrorKind::PermissionDenied));
            }
            self.files
                .lock()
                .expect("files lock")
                .get(path)
                .cloned()
                .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
        }
    }

    struct Fixture {
        watcher: ReloadWatcher<(), FakeReloadClock>,
        sleeps: mpsc::UnboundedReceiver<SleepRequest>,
        files: Arc<FakeReloadFileStore>,
        sink: Arc<SpyEventSink>,
        config_path: PathBuf,
        cell: Arc<snapshot::Cell<()>>,
        stop: watch::Receiver<bool>,
        _stop_sender: watch::Sender<bool>,
    }

    fn fixture() -> Fixture {
        fixture_running(CONFIG)
    }

    /// Starts a watcher whose running configuration and file on disk are `running_text`.
    fn fixture_running(running_text: &str) -> Fixture {
        fixture_with_feed(running_text, Arc::new(AtomicBool::new(false)))
            .expect("watcher assembles")
    }

    /// Like `fixture_running`, with a feed that refuses registration while `refuse` is set. The
    /// result is returned so that a test can check a refused start.
    fn fixture_with_feed(
        running_text: &str,
        refuse: Arc<AtomicBool>,
    ) -> Result<Fixture, ReloadWatchError> {
        let (requests, sleeps) = mpsc::unbounded_channel();
        let files = Arc::new(FakeReloadFileStore::default());
        let sink = Arc::new(SpyEventSink::default());
        let config_path = std::path::absolute("vnc-relay.toml").expect("current directory");
        files.set(&config_path, running_text.as_bytes());
        // The watcher parses candidates against the config directory, so the running settings must
        // use the same base; otherwise relative paths differ and read as restart-required changes.
        let base_dir = config_path.parent().expect("config directory");
        let running = config_file::parse_candidate(running_text.as_bytes(), base_dir)
            .expect("valid running config");
        let cell = Arc::new(snapshot::Cell::<()>::new(ReloadSnapshot::new(
            running.clone(),
            None,
            None,
        )));
        let service = ReloadService::new(
            running,
            Arc::clone(&cell),
            files.clone(),
            Arc::new(StubReloadTlsStore),
            Arc::new(crate::ticket::TicketJwksStore),
            sink.clone(),
        );
        let watcher = assemble(
            &config_path,
            Arc::new(Mutex::new(service)),
            POLL,
            FakeReloadClock { requests },
            files.clone(),
            sink.clone(),
            |_queue| {
                let feed: Box<dyn WatchGateway> = Box::new(FakeWatchGateway { refuse });
                Ok(feed)
            },
        )?;
        let (stop_sender, stop) = watch::channel(false);
        Ok(Fixture {
            watcher,
            sleeps,
            files,
            sink,
            config_path,
            cell,
            stop,
            _stop_sender: stop_sender,
        })
    }

    /// spec: operations.md:OPS-027 partial
    #[tokio::test]
    async fn notify_during_quiet_window_restarts_the_episode() {
        let Fixture {
            mut watcher,
            mut sleeps,
            files,
            sink,
            config_path,
            mut stop,
            _stop_sender,
            ..
        } = fixture();
        files.set(&config_path, INVALID_CONFIG.as_bytes());
        let queue = Arc::clone(&watcher.queue);
        queue.offer_path(&config_path);
        // The run loop consumes the wake before it calls the handler.
        watcher.wake.recv().await.expect("wake sent");
        let driver = async {
            let _interrupted = next_sleep(&mut sleeps, QUIET).await;
            queue.offer_path(&config_path);
            let quiet = next_sleep(&mut sleeps, QUIET).await;
            let _ = quiet.send(());
            let gap = next_sleep(&mut sleeps, GAP).await;
            let _ = gap.send(());
        };
        let (flow, ()) = tokio::join!(watcher.on_wake(&mut stop), driver);
        assert_eq!(flow, Flow::Continue);
        assert_eq!(sink.names(), ["reload_change_detected", "reload_rejected"]);
        assert!(matches!(
            sink.snapshot()[0],
            Event::ReloadChangeDetected {
                source: ChangeSource::OsNotify
            }
        ));
    }

    /// spec: operations.md:OPS-026 partial
    #[tokio::test]
    async fn poll_change_settles_then_applies_once() {
        let Fixture {
            mut watcher,
            mut sleeps,
            files,
            sink,
            config_path,
            mut stop,
            _stop_sender,
            ..
        } = fixture();
        files.set(&config_path, INVALID_CONFIG.as_bytes());
        let driver = async {
            let quiet = next_sleep(&mut sleeps, QUIET).await;
            let _ = quiet.send(());
            let gap = next_sleep(&mut sleeps, GAP).await;
            let _ = gap.send(());
        };
        let (flow, ()) = tokio::join!(watcher.on_tick(&mut stop), driver);
        assert_eq!(flow, Flow::Continue);
        assert_eq!(sink.names(), ["reload_change_detected", "reload_rejected"]);
        // The processed state now holds the rejected content, so the next poll is silent.
        assert_eq!(watcher.on_tick(&mut stop).await, Flow::Continue);
        assert!(sleeps.try_recv().is_err());
        assert_eq!(sink.names().len(), 2);
    }

    /// spec: operations.md:OPS-028 partial
    #[tokio::test]
    async fn poll_read_error_is_recorded_without_an_episode() {
        let Fixture {
            mut watcher,
            mut sleeps,
            files,
            sink,
            config_path,
            cell,
            mut stop,
            _stop_sender,
        } = fixture_running(RELAY_CONFIG);
        let before = cell.current().settings.relay.tcp_viewer.allow.clone();
        files.deny(&config_path);
        assert_eq!(watcher.on_tick(&mut stop).await, Flow::Continue);
        assert!(sleeps.try_recv().is_err());
        assert_eq!(sink.names(), ["reload_watch_failed"]);
        // The failed read starts no episode, and the running list stays in force.
        assert_eq!(cell.current().settings.relay.tcp_viewer.allow, before);
    }

    /// spec: operations.md:OPS-041
    #[tokio::test]
    async fn allow_change_is_applied_by_poll_when_the_notify_is_blocked() {
        let Fixture {
            watcher,
            mut sleeps,
            files,
            sink,
            config_path,
            cell,
            stop,
            _stop_sender: stop_sender,
        } = fixture_running(RELAY_CONFIG);
        files.set(&config_path, RELAY_CANDIDATE.as_bytes());
        let driver = async {
            // The run loop first sleeps for the poll interval Dp. No notify event is delivered, so
            // the poll tick is the only path that can see the change.
            let tick = next_sleep(&mut sleeps, POLL).await;
            let _ = tick.send(());
            // The episode waits Q, then G, then applies. Q + G stays below the smallest settle_secs.
            let quiet = next_sleep(&mut sleeps, QUIET).await;
            let _ = quiet.send(());
            let gap = next_sleep(&mut sleeps, GAP).await;
            let _ = gap.send(());
            // The next tick is requested one Dp after the detecting tick. The sender is returned, not
            // dropped, so the run loop keeps waiting on that tick until the stop request arrives.
            let next_tick = next_sleep(&mut sleeps, POLL).await;
            stop_sender
                .send(true)
                .expect("the watcher holds the stop receiver");
            next_tick
        };
        let ((), _next_tick) = tokio::join!(watcher.run(stop), driver);
        assert_eq!(sink.names(), ["reload_change_detected", "reload_applied"]);
        assert!(matches!(
            sink.snapshot()[0],
            Event::ReloadChangeDetected {
                source: ChangeSource::Poll
            }
        ));
        assert!(
            QUIET + GAP < Duration::from_secs(1),
            "Q + G must stay below the smallest settle_secs (1 s)"
        );
        // New decisions use the new list: 127.0.0.2 is permitted and the removed 127.0.0.1 is not.
        let allow = cell
            .current()
            .settings
            .relay
            .tcp_viewer
            .allow
            .clone()
            .expect("enabled allow");
        assert!(allow.permits(PeerIp::new(IpAddr::from([127, 0, 0, 2]))));
        assert!(!allow.permits(PeerIp::new(IpAddr::from([127, 0, 0, 1]))));
        assert_eq!(
            allow,
            AllowList::new(vec![IpItem::Single(IpAddr::from([127, 0, 0, 2]))])
        );
    }

    /// spec: operations.md:OPS-028 partial
    #[tokio::test]
    async fn poll_still_applies_a_change_while_notify_registration_is_refused() {
        let refuse = Arc::new(AtomicBool::new(false));
        let Fixture {
            mut watcher,
            mut sleeps,
            files,
            sink,
            config_path,
            cell,
            mut stop,
            _stop_sender,
        } = fixture_with_feed(RELAY_CONFIG, Arc::clone(&refuse)).expect("watcher assembles");
        refuse.store(true, Ordering::SeqCst);
        watcher.queue.offer_failure("OS notify error".to_owned());
        watcher.wake.recv().await.expect("wake sent");
        files.set(&config_path, RELAY_CANDIDATE.as_bytes());
        let driver = async {
            let quiet = next_sleep(&mut sleeps, QUIET).await;
            let _ = quiet.send(());
            let gap = next_sleep(&mut sleeps, GAP).await;
            let _ = gap.send(());
        };
        let (flow, ()) = tokio::join!(watcher.on_tick(&mut stop), driver);
        assert_eq!(flow, Flow::Continue);
        // Applying the candidate retries registration too, and the refusal is recorded again.
        assert_eq!(
            sink.names(),
            [
                "reload_watch_failed",
                "reload_watch_failed",
                "reload_change_detected",
                "reload_applied",
                "reload_watch_failed",
            ]
        );
        let allow = AllowList::new(vec![IpItem::Single(IpAddr::from([127, 0, 0, 2]))]);
        assert_eq!(
            cell.current().settings.relay.tcp_viewer.allow.as_ref(),
            Some(&allow)
        );
    }

    /// spec: operations.md:OPS-027 partial
    #[tokio::test]
    async fn latest_valid_candidate_is_applied_when_a_newer_write_lands_in_the_quiet_window() {
        let Fixture {
            mut watcher,
            mut sleeps,
            files,
            sink,
            config_path,
            cell,
            mut stop,
            _stop_sender,
        } = fixture_running(RELAY_CONFIG);
        let queue = Arc::clone(&watcher.queue);
        let newest = RELAY_CANDIDATE.replace("127.0.0.2", "127.0.0.3");
        files.set(&config_path, RELAY_CANDIDATE.as_bytes());
        let driver = async {
            let _first_quiet = next_sleep(&mut sleeps, QUIET).await;
            // A newer valid write lands inside the quiet window, so the first candidate never applies.
            files.set(&config_path, newest.as_bytes());
            queue.offer_path(&config_path);
            let quiet = next_sleep(&mut sleeps, QUIET).await;
            let _ = quiet.send(());
            let gap = next_sleep(&mut sleeps, GAP).await;
            let _ = gap.send(());
        };
        let (flow, ()) = tokio::join!(watcher.on_tick(&mut stop), driver);
        assert_eq!(flow, Flow::Continue);
        assert_eq!(sink.names(), ["reload_change_detected", "reload_applied"]);
        let allow = AllowList::new(vec![IpItem::Single(IpAddr::from([127, 0, 0, 3]))]);
        assert_eq!(
            cell.current().settings.relay.tcp_viewer.allow.as_ref(),
            Some(&allow)
        );
    }

    /// spec: operations.md:OPS-027 partial
    #[tokio::test]
    async fn invalid_latest_candidate_keeps_the_running_allow_list_when_it_lands_in_the_gap() {
        let Fixture {
            mut watcher,
            mut sleeps,
            files,
            sink,
            config_path,
            cell,
            mut stop,
            _stop_sender,
        } = fixture_running(RELAY_CONFIG);
        let queue = Arc::clone(&watcher.queue);
        let before = cell.current().settings.relay.tcp_viewer.allow.clone();
        files.set(&config_path, RELAY_CANDIDATE.as_bytes());
        let driver = async {
            let quiet = next_sleep(&mut sleeps, QUIET).await;
            let _ = quiet.send(());
            let gap = next_sleep(&mut sleeps, GAP).await;
            // A newer invalid write lands after the stability check, so the episode starts again.
            files.set(&config_path, INVALID_CONFIG.as_bytes());
            queue.offer_path(&config_path);
            let _ = gap.send(());
            // That write left a wake, so the restarted quiet window ends at once. The next one settles.
            let _interrupted = next_sleep(&mut sleeps, QUIET).await;
            let settled = next_sleep(&mut sleeps, QUIET).await;
            let _ = settled.send(());
            let gap_again = next_sleep(&mut sleeps, GAP).await;
            let _ = gap_again.send(());
        };
        let (flow, ()) = tokio::join!(watcher.on_tick(&mut stop), driver);
        assert_eq!(flow, Flow::Continue);
        assert_eq!(sink.names(), ["reload_change_detected", "reload_rejected"]);
        assert_eq!(cell.current().settings.relay.tcp_viewer.allow, before);
    }

    /// Arms the §4.3 validation hook. While the candidate is being validated, `text` is written and
    /// queued as a newer change, the way a second save would arrive.
    fn arm_newer_write(
        watcher: &mut ReloadWatcher<(), FakeReloadClock>,
        files: &Arc<FakeReloadFileStore>,
        config_path: &Path,
        text: &str,
    ) {
        let queue = Arc::clone(&watcher.queue);
        let files = Arc::clone(files);
        let path = config_path.to_path_buf();
        let text = text.to_owned();
        watcher
            .service
            .lock()
            .expect("reload lock")
            .arm_before_validation(Box::new(move || {
                files.set(&path, text.as_bytes());
                queue.offer_path(&path);
            }));
    }

    /// spec: operations.md:OPS-028 partial
    #[tokio::test]
    async fn stop_during_validation_discards_the_candidate_being_validated() {
        let Fixture {
            mut watcher,
            mut sleeps,
            files,
            sink,
            config_path,
            cell,
            mut stop,
            _stop_sender: stop_sender,
        } = fixture_running(RELAY_CONFIG);
        let before = cell.current().settings.relay.tcp_viewer.allow.clone();
        files.set(&config_path, RELAY_CANDIDATE.as_bytes());
        watcher
            .service
            .lock()
            .expect("reload lock")
            .arm_before_validation(Box::new(move || {
                // The stop request arrives after the candidate was read and before it is validated.
                stop_sender
                    .send(true)
                    .expect("the watcher holds the stop receiver");
            }));
        let driver = async {
            let quiet = next_sleep(&mut sleeps, QUIET).await;
            let _ = quiet.send(());
            let gap = next_sleep(&mut sleeps, GAP).await;
            let _ = gap.send(());
        };
        let (flow, ()) = tokio::join!(watcher.on_tick(&mut stop), driver);
        assert_eq!(flow, Flow::Continue);
        assert!(*stop.borrow());
        assert_eq!(sink.names(), ["reload_change_detected"]);
        assert_eq!(cell.current().settings.relay.tcp_viewer.allow, before);
    }

    /// spec: operations.md:OPS-027
    #[tokio::test]
    async fn overtaken_candidate_is_never_applied_when_the_newest_write_is_valid_or_invalid() {
        // Newest valid: a valid newer write lands while the first candidate is validated. The first
        // candidate is discarded without an event or a swap, and only the newest candidate applies.
        {
            let Fixture {
                mut watcher,
                mut sleeps,
                files,
                sink,
                config_path,
                cell,
                mut stop,
                _stop_sender,
            } = fixture_running(RELAY_CONFIG);
            let newest = RELAY_CANDIDATE.replace("127.0.0.2", "127.0.0.3");
            let before = cell.current().settings.relay.tcp_viewer.allow.clone();
            files.set(&config_path, RELAY_CANDIDATE.as_bytes());
            arm_newer_write(&mut watcher, &files, &config_path, &newest);
            let driver = async {
                let quiet = next_sleep(&mut sleeps, QUIET).await;
                let _ = quiet.send(());
                let gap = next_sleep(&mut sleeps, GAP).await;
                let _ = gap.send(());
            };
            let (flow, ()) = tokio::join!(watcher.on_tick(&mut stop), driver);
            assert_eq!(flow, Flow::Continue);
            assert_eq!(sink.names(), ["reload_change_detected"]);
            assert_eq!(cell.current().settings.relay.tcp_viewer.allow, before);
            // The newer write left a wake, which starts the episode for the newest candidate.
            watcher.wake.recv().await.expect("wake sent");
            let driver = async {
                let quiet = next_sleep(&mut sleeps, QUIET).await;
                let _ = quiet.send(());
                let gap = next_sleep(&mut sleeps, GAP).await;
                let _ = gap.send(());
            };
            let (flow, ()) = tokio::join!(watcher.on_wake(&mut stop), driver);
            assert_eq!(flow, Flow::Continue);
            assert_eq!(
                sink.names(),
                [
                    "reload_change_detected",
                    "reload_change_detected",
                    "reload_applied",
                ]
            );
            let allow = AllowList::new(vec![IpItem::Single(IpAddr::from([127, 0, 0, 3]))]);
            assert_eq!(
                cell.current().settings.relay.tcp_viewer.allow.as_ref(),
                Some(&allow)
            );
        }
        // Newest invalid: an invalid newer write lands while the first candidate is validated. The
        // first candidate is still never applied, and the running allow list stays in force.
        {
            let Fixture {
                mut watcher,
                mut sleeps,
                files,
                sink,
                config_path,
                cell,
                mut stop,
                _stop_sender,
            } = fixture_running(RELAY_CONFIG);
            let before = cell.current().settings.relay.tcp_viewer.allow.clone();
            files.set(&config_path, RELAY_CANDIDATE.as_bytes());
            arm_newer_write(&mut watcher, &files, &config_path, INVALID_CONFIG);
            let driver = async {
                let quiet = next_sleep(&mut sleeps, QUIET).await;
                let _ = quiet.send(());
                let gap = next_sleep(&mut sleeps, GAP).await;
                let _ = gap.send(());
            };
            let (flow, ()) = tokio::join!(watcher.on_tick(&mut stop), driver);
            assert_eq!(flow, Flow::Continue);
            assert_eq!(sink.names(), ["reload_change_detected"]);
            assert_eq!(cell.current().settings.relay.tcp_viewer.allow, before);
            watcher.wake.recv().await.expect("wake sent");
            let driver = async {
                let quiet = next_sleep(&mut sleeps, QUIET).await;
                let _ = quiet.send(());
                let gap = next_sleep(&mut sleeps, GAP).await;
                let _ = gap.send(());
            };
            let (flow, ()) = tokio::join!(watcher.on_wake(&mut stop), driver);
            assert_eq!(flow, Flow::Continue);
            assert_eq!(
                sink.names(),
                [
                    "reload_change_detected",
                    "reload_change_detected",
                    "reload_rejected",
                ]
            );
            assert_eq!(cell.current().settings.relay.tcp_viewer.allow, before);
        }
    }

    /// spec: operations.md:OPS-028
    #[tokio::test]
    async fn watch_faults_at_start_running_and_shutdown_are_recorded_and_never_applied() {
        // Start: a refused registration is an error, so no watcher starts without notify.
        let refused = fixture_with_feed(CONFIG, Arc::new(AtomicBool::new(true)));
        assert_eq!(
            refused.err().expect("the start is refused").detail(),
            "registration refused"
        );
        // Running: the failure is recorded on the tick and the running allow list stays in force.
        let refuse = Arc::new(AtomicBool::new(false));
        let Fixture {
            mut watcher,
            sink,
            cell,
            mut stop,
            _stop_sender,
            ..
        } = fixture_with_feed(RELAY_CONFIG, Arc::clone(&refuse)).expect("watcher assembles");
        let before = cell.current().settings.relay.tcp_viewer.allow.clone();
        refuse.store(true, Ordering::SeqCst);
        watcher.queue.offer_failure("OS notify error".to_owned());
        // The run loop consumes the wake before it calls the handler.
        watcher.wake.recv().await.expect("wake sent");
        assert_eq!(watcher.on_tick(&mut stop).await, Flow::Continue);
        assert_eq!(sink.names(), ["reload_watch_failed", "reload_watch_failed"]);
        assert_eq!(cell.current().settings.relay.tcp_viewer.allow, before);
        assert!(matches!(
            sink.snapshot()[0],
            Event::ReloadWatchFailed {
                reason: WatchError::NotifyError,
                ..
            }
        ));
        // Every later tick retries registration and records the refusal again.
        assert_eq!(watcher.on_tick(&mut stop).await, Flow::Continue);
        assert_eq!(sink.names().len(), 3);
        // Registration succeeds again. Recovery has no event, so the tick records nothing.
        refuse.store(false, Ordering::SeqCst);
        assert_eq!(watcher.on_tick(&mut stop).await, Flow::Continue);
        assert_eq!(sink.names().len(), 3);
        // Shutdown: a change seen while the quiet window is open is not applied.
        let Fixture {
            mut watcher,
            mut sleeps,
            files,
            sink,
            config_path,
            cell,
            mut stop,
            _stop_sender: stop_sender,
        } = fixture_running(RELAY_CONFIG);
        let before = cell.current().settings.relay.tcp_viewer.allow.clone();
        files.set(&config_path, RELAY_CANDIDATE.as_bytes());
        let driver = async {
            let quiet = next_sleep(&mut sleeps, QUIET).await;
            stop_sender
                .send(true)
                .expect("the watcher holds the stop receiver");
            quiet
        };
        let (flow, _quiet) = tokio::join!(watcher.on_tick(&mut stop), driver);
        assert_eq!(flow, Flow::Stop);
        assert_eq!(sink.names(), ["reload_change_detected"]);
        assert_eq!(cell.current().settings.relay.tcp_viewer.allow, before);
    }
    #[test]
    fn final_candidate_read_holds_the_same_mutex_as_config_edit() {
        use std::sync::atomic::AtomicUsize;
        struct SpyReloadFileStore {
            inner: Arc<FakeReloadFileStore>,
            reload: std::sync::Weak<Mutex<ReloadService<()>>>,
            reads: AtomicUsize,
        }
        impl ReloadFileStore for SpyReloadFileStore {
            fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
                let reload = self.reload.upgrade().expect("shared transaction alive");
                assert!(
                    matches!(reload.try_lock(), Err(std::sync::TryLockError::WouldBlock)),
                    "the watcher must lock before its final file read"
                );
                self.reads.fetch_add(1, Ordering::SeqCst);
                self.inner.read(path)
            }
        }
        let Fixture {
            mut watcher,
            files,
            config_path,
            sink,
            stop,
            ..
        } = fixture_running(RELAY_CONFIG);
        let checked = Arc::new(SpyReloadFileStore {
            inner: files.clone(),
            reload: Arc::downgrade(&watcher.service),
            reads: AtomicUsize::new(0),
        });
        watcher.files = checked.clone();
        files.set(&config_path, RELAY_CANDIDATE.as_bytes());
        let before = watcher.stamps();
        assert_eq!(
            watcher.apply(watcher.queue.generation(), &before, &stop),
            Some(true)
        );
        assert!(checked.reads.load(Ordering::SeqCst) > 0);
        assert_eq!(sink.names(), ["reload_applied"]);
    }

    /// Reads database files from disk. `db:<type>` is a usable database of that type; any other
    /// content is an invalid file. Lookups find nothing.
    struct FakeIpDbFileGateway;

    struct StubIpDbLookupGateway(vnc_relay_application::ip_info::IpDbMetadata);

    impl vnc_relay_application::ports::IpDbLookupGateway for StubIpDbLookupGateway {
        fn metadata(&self) -> &vnc_relay_application::ip_info::IpDbMetadata {
            &self.0
        }
        fn supports(&self, _key: vnc_relay_application::ip_info::IpDbKey) -> bool {
            true
        }
        fn country(&self, _ip: IpAddr) -> Option<vnc_relay_domain::ip_info::CountryCode> {
            None
        }
        fn asn(&self, _ip: IpAddr) -> Option<vnc_relay_domain::ip_info::Asn> {
            None
        }
        fn as_name(&self, _ip: IpAddr) -> Option<(vnc_relay_domain::ip_info::Asn, String)> {
            None
        }
    }

    impl vnc_relay_application::ports::IpDbFileGateway for FakeIpDbFileGateway {
        fn read(&self, path: &Path) -> vnc_relay_application::ip_info::IpDbRead {
            use vnc_relay_application::ip_info::{
                Fingerprint, IpDbError, IpDbMetadata, IpDbRead, IpDbReadError,
            };
            let Ok(bytes) = std::fs::read(path) else {
                return IpDbRead {
                    fingerprint: Fingerprint::Unreadable,
                    result: Err(IpDbReadError {
                        failure: IpDbError::FileUnreadable,
                        detail: None,
                    }),
                };
            };
            let fingerprint = Fingerprint::Content(content_hash(&bytes));
            let result = match bytes.strip_prefix(b"db:") {
                Some(kind) => {
                    let kind = String::from_utf8_lossy(kind);
                    let db: Arc<dyn vnc_relay_application::ports::IpDbLookupGateway> = Arc::new(
                        StubIpDbLookupGateway(IpDbMetadata::new(&kind, SystemTime::UNIX_EPOCH)),
                    );
                    Ok(db)
                }
                None => Err(IpDbReadError {
                    failure: IpDbError::InvalidFormat,
                    detail: None,
                }),
            };
            IpDbRead {
                fingerprint,
                result,
            }
        }
    }

    fn db_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "vnc-relay-watch-ipdb-{}-{name}",
            std::process::id()
        ));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("clear database directory");
        }
        std::fs::create_dir_all(&dir).expect("create database directory");
        dir
    }

    /// Attaches a country database at `path` and runs the startup read.
    fn attach_country_db(
        watcher: &mut ReloadWatcher<(), FakeReloadClock>,
        sink: &Arc<SpyEventSink>,
        path: &Path,
    ) -> Arc<IpInfoService> {
        let service = Arc::new(IpInfoService::new(
            Some(path.to_path_buf()),
            None,
            Arc::new(FakeIpDbFileGateway),
            Arc::new(vnc_relay_application::ip_info::IpLookup::new()),
            Arc::new(crate::clock::SystemClock::new()),
            sink.clone(),
        ));
        watcher.attach_ip_info(Arc::clone(&service));
        service.load_startup();
        service
    }

    /// Runs one poll tick and releases the quiet window and the gap if an episode starts.
    async fn tick_through_episode(
        watcher: &mut ReloadWatcher<(), FakeReloadClock>,
        sleeps: &mut mpsc::UnboundedReceiver<SleepRequest>,
        stop: &mut watch::Receiver<bool>,
    ) {
        let driver = async {
            let quiet = next_sleep(sleeps, QUIET).await;
            let _ = quiet.send(());
            let gap = next_sleep(sleeps, GAP).await;
            let _ = gap.send(());
        };
        let (flow, ()) = tokio::join!(watcher.on_tick(stop), driver);
        assert_eq!(flow, Flow::Continue);
    }

    /// `(event name, database_type or reason, phase)` of the database events.
    fn db_events(sink: &SpyEventSink) -> Vec<(&'static str, String, &'static str)> {
        sink.snapshot()
            .into_iter()
            .filter_map(|event| match event {
                Event::IpDbLoaded {
                    database_type,
                    phase,
                    ..
                } => Some(("ip_db_loaded", database_type, phase.as_str())),
                Event::IpDbUnavailable { reason, phase, .. } => Some((
                    "ip_db_unavailable",
                    reason.as_str().to_owned(),
                    phase.as_str(),
                )),
                _ => None,
            })
            .collect()
    }

    /// spec: operations.md:OPS-097 partial
    #[tokio::test]
    async fn database_content_changes_are_reread_and_never_join_reload_applied() {
        let Fixture {
            mut watcher,
            mut sleeps,
            sink,
            mut stop,
            _stop_sender,
            ..
        } = fixture();
        let dir = db_dir("replace");
        let path = dir.join("country.mmdb");
        std::fs::write(&path, b"db:A").expect("write database");
        let service = attach_country_db(&mut watcher, &sink, &path);

        // Another valid database.
        std::fs::write(&path, b"db:B").expect("write database");
        tick_through_episode(&mut watcher, &mut sleeps, &mut stop).await;
        // A broken file keeps B in use.
        std::fs::write(&path, b"broken").expect("write database");
        tick_through_episode(&mut watcher, &mut sleeps, &mut stop).await;
        assert_eq!(
            service.status()[0].state,
            vnc_relay_application::ip_info::IpDbState::Stale
        );
        // The same broken content saved again starts nothing and warns nothing.
        std::fs::write(&path, b"broken").expect("write database");
        assert_eq!(watcher.on_tick(&mut stop).await, Flow::Continue);
        assert!(sleeps.try_recv().is_err());
        // A valid file again.
        std::fs::write(&path, b"db:A").expect("write database");
        tick_through_episode(&mut watcher, &mut sleeps, &mut stop).await;

        assert_eq!(
            db_events(&sink),
            [
                ("ip_db_loaded", "A".to_owned(), "startup"),
                ("ip_db_loaded", "B".to_owned(), "reload"),
                ("ip_db_unavailable", "invalid_format".to_owned(), "reload"),
                ("ip_db_loaded", "A".to_owned(), "reload"),
            ]
        );
        assert_eq!(
            service.status()[0].state,
            vnc_relay_application::ip_info::IpDbState::Loaded
        );
        // The config file did not change: no reload result at all.
        assert!(
            !sink
                .names()
                .iter()
                .any(|name| name.starts_with("reload_") && *name != "reload_change_detected"),
            "{:?}",
            sink.names()
        );
    }

    /// spec: operations.md:OPS-097 partial
    #[tokio::test]
    async fn database_is_reread_while_a_restart_required_difference_stays() {
        let Fixture {
            mut watcher,
            mut sleeps,
            files,
            sink,
            config_path,
            mut stop,
            _stop_sender,
            ..
        } = fixture();
        let dir = db_dir("restart");
        let path = dir.join("country.mmdb");
        std::fs::write(&path, b"db:A").expect("write database");
        attach_country_db(&mut watcher, &sink, &path);
        files.set(
            &config_path,
            CONFIG
                .replace("127.0.0.1:18081", "127.0.0.1:18082")
                .as_bytes(),
        );
        tick_through_episode(&mut watcher, &mut sleeps, &mut stop).await;
        assert_eq!(
            sink.names(),
            [
                "ip_db_loaded",
                "reload_change_detected",
                "reload_restart_required"
            ]
        );
        std::fs::write(&path, b"db:B").expect("write database");
        tick_through_episode(&mut watcher, &mut sleeps, &mut stop).await;
        assert_eq!(
            db_events(&sink).last(),
            Some(&("ip_db_loaded", "B".to_owned(), "reload"))
        );
    }

    /// spec: operations.md:OPS-097 partial
    #[tokio::test]
    async fn a_database_in_a_missing_directory_is_polled_without_a_watch_failure() {
        let Fixture {
            mut watcher,
            mut sleeps,
            sink,
            mut stop,
            _stop_sender,
            ..
        } = fixture();
        let dir = db_dir("missing").join("later");
        let path = dir.join("country.mmdb");
        attach_country_db(&mut watcher, &sink, &path);
        assert_eq!(watcher.on_tick(&mut stop).await, Flow::Continue);
        assert!(sleeps.try_recv().is_err());
        std::fs::create_dir_all(&dir).expect("create database directory");
        std::fs::write(&path, b"db:A").expect("write database");
        tick_through_episode(&mut watcher, &mut sleeps, &mut stop).await;
        assert_eq!(
            db_events(&sink),
            [
                ("ip_db_unavailable", "file_unreadable".to_owned(), "startup"),
                ("ip_db_loaded", "A".to_owned(), "reload"),
            ]
        );
        assert!(!sink.names().contains(&"reload_watch_failed"));
    }
}
