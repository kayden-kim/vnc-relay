//! Automatic partial reload: the file watcher (docs/architectures/file-watch.md §2–§6, §9).
//!
//! `ReloadWatcher` owns the OS notification feed, the poll timer, and the processed state. A change
//! starts an episode. The episode waits for a quiet window, checks file stamps across a gap, reads a
//! candidate, and hands it to `ReloadService`, which decides the result.

mod feed;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};
use std::pin::pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use feed::{ChangeFeed, ChangeQueue, NotifyFeed, WatchTarget};
use tokio::sync::{mpsc, watch};
use vnc_relay_app::events::Event;
use vnc_relay_app::ports::EventSink;
use vnc_relay_app::reload::ports::{ReloadClock, ReloadFileStore};
use vnc_relay_app::reload::service::{ReloadOutcome, ReloadService};
use vnc_relay_app::reload::{ChangeSource, WatchFailure};

use crate::config_file;

/// Quiet window Q before a candidate is read (file-watch.md §4 step 3 (a), §9 item 1).
/// Implementation constant. Q + G + read + verify + swap must stay below the poll interval D.
const QUIET: Duration = Duration::from_millis(300);

/// Gap G between the two stamp recordings (file-watch.md §4 step 3 (c), §9 item 1).
/// Implementation constant.
const GAP: Duration = Duration::from_millis(100);

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
    service: ReloadService<A>,
    poll: Duration,
    clock: C,
    files: Arc<dyn ReloadFileStore>,
    log: Arc<dyn EventSink>,
) -> Result<ReloadWatcher<A, C>, ReloadWatchError> {
    assemble(config_path, service, poll, clock, files, log, |queue| {
        let feed: Box<dyn ChangeFeed> = Box::new(NotifyFeed::new(queue)?);
        Ok(feed)
    })
}

/// Builds the watcher around `make_feed`, so tests can replace the OS backend.
fn assemble<A: Clone, C: ReloadClock>(
    config_path: &Path,
    service: ReloadService<A>,
    poll: Duration,
    clock: C,
    files: Arc<dyn ReloadFileStore>,
    log: Arc<dyn EventSink>,
    make_feed: impl FnOnce(Arc<ChangeQueue>) -> Result<Box<dyn ChangeFeed>, ReloadWatchError>,
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
    };
    watcher.attach()?;
    Ok(watcher)
}

/// Automatic partial reload watcher (file-watch.md §2–§6). Drive it with [`ReloadWatcher::run`].
pub struct ReloadWatcher<A, C> {
    config_path: PathBuf,
    base_dir: PathBuf,
    service: ReloadService<A>,
    feed: Box<dyn ChangeFeed>,
    queue: Arc<ChangeQueue>,
    wake: mpsc::Receiver<()>,
    poll: Duration,
    clock: Arc<C>,
    files: Arc<dyn ReloadFileStore>,
    log: Arc<dyn EventSink>,
    /// Files watched now: the config file plus the service's TLS paths (file-watch.md §9 item 9).
    watched: Vec<PathBuf>,
    /// Content hash per watched file at the last processed state. `None` means the file is missing
    /// (file-watch.md §4 step 2).
    processed: BTreeMap<PathBuf, Option<u64>>,
    /// True while the last notify registration failed. Each poll tick retries it (file-watch.md §6).
    notify_failed: bool,
    /// Generation whose changes have already been handed to the service.
    seen_generation: u64,
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
        for path in self.service.watched_paths() {
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
        paths
    }

    fn sync_notify(&mut self) -> Result<(), ReloadWatchError> {
        let plan = watch_plan(&self.watched)?;
        self.queue.set_targets(plan.targets);
        self.feed.watch_dirs(&plan.dirs)
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
            reason: WatchFailure::NotifyError,
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

    fn stamps(&self) -> Vec<Option<FileStamp>> {
        self.watched.iter().map(|path| stamp_of(path)).collect()
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

    /// Hands the candidate to the service. The processed state is refreshed unless the result is
    /// discarded, so a discarded candidate is seen again on the next change (file-watch.md §4 step 5, §9 item 10).
    fn apply(&mut self, window: u64, candidate: Candidate, stop: &watch::Receiver<bool>) {
        let queue = Arc::clone(&self.queue);
        let is_current = || queue.generation() == window && !*stop.borrow();
        let outcome = match candidate.config.as_deref().map(std::str::from_utf8) {
            Some(Ok(text)) => self
                .service
                .handle_config(config_file::parse(text, &self.base_dir), &is_current),
            // A missing or non-UTF-8 config file is unreadable (config-v1 §1.9, `config_file_unreadable`).
            _ => self.service.handle_unreadable(&is_current),
        };
        if !matches!(outcome, ReloadOutcome::Discarded) {
            self.refresh_watch_set(candidate.hashes);
        }
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
            let candidate = self.read_candidate();
            if self.stamps() != before || self.queue.generation() != window {
                continue;
            }
            self.seen_generation = window;
            self.apply(window, candidate, stop);
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
        if self.notify_failed {
            self.resync_notify();
        }
        self.poll_check(stop).await
    }

    /// Compares the files with the processed state. A difference starts a poll episode. A read error
    /// is recorded as `reload_watch_failed` with `poll_error` on every tick (file-watch.md §6, §9 item 8).
    async fn poll_check(&mut self, stop: &mut watch::Receiver<bool>) -> Flow {
        match self.read_poll() {
            Ok(hashes) if hashes == self.processed => Flow::Continue,
            Ok(_) => self.episode(ChangeSource::Poll, stop).await,
            Err(detail) => {
                self.log.record(&Event::ReloadWatchFailed {
                    reason: WatchFailure::PollError,
                    detail: Some(detail),
                });
                Flow::Continue
            }
        }
    }
}

impl<A: Clone, C: ReloadClock> ReloadWatcher<A, C> {
    /// Runs the watcher until `stop` becomes true or its sender is dropped (file-watch.md §4, §5).
    pub async fn run(mut self, mut stop: watch::Receiver<bool>) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::sync::oneshot;
    use vnc_relay_app::reload::ports::{ReloadTlsStore, TlsBuildError};
    use vnc_relay_app::reload::snapshot::{RelayAllow, ReloadCell, ReloadSnapshot};
    use vnc_relay_domain::allow_list::{AllowList, IpItem};
    use vnc_relay_domain::peer_ip::PeerIp;

    const CONFIG: &str = "[admin]\naccounts_file = \"admin-accounts.toml\"\n\n[management]\nlisten = \"127.0.0.1:18080\"\n\n[status]\nlisten = \"127.0.0.1:18081\"\n";
    const INVALID_CONFIG: &str = "not = [valid toml\n";
    const POLL: Duration = Duration::from_secs(30);

    const RELAY_CONFIG: &str = "[admin]\naccounts_file = \"admin-accounts.toml\"\n\n[management]\nlisten = \"127.0.0.1:18080\"\n\n[status]\nlisten = \"127.0.0.1:18081\"\n\n[relay.tcp_viewer]\nenabled = true\nlisten = \"127.0.0.1:5901\"\nallow = [\"127.0.0.1\"]\n";
    const RELAY_CANDIDATE: &str = "[admin]\naccounts_file = \"admin-accounts.toml\"\n\n[management]\nlisten = \"127.0.0.1:18080\"\n\n[status]\nlisten = \"127.0.0.1:18081\"\n\n[relay.tcp_viewer]\nenabled = true\nlisten = \"127.0.0.1:5901\"\nallow = [\"127.0.0.2\"]\n";

    type SleepRequest = (Duration, oneshot::Sender<()>);

    /// Clock whose sleeps finish only when the test releases them.
    struct ManualClock {
        requests: mpsc::UnboundedSender<SleepRequest>,
    }

    impl ReloadClock for ManualClock {
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
    struct SwitchFeed {
        refuse: Arc<AtomicBool>,
    }

    impl ChangeFeed for SwitchFeed {
        fn watch_dirs(&mut self, _dirs: &BTreeSet<PathBuf>) -> Result<(), ReloadWatchError> {
            if self.refuse.load(Ordering::SeqCst) {
                Err(ReloadWatchError::new("registration refused".to_owned()))
            } else {
                Ok(())
            }
        }
    }

    struct NoTls;

    impl ReloadTlsStore<()> for NoTls {
        fn build(&self, _cert_pem: &[u8], _key_pem: &[u8]) -> Result<(), TlsBuildError> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct RecordingSink {
        events: Mutex<Vec<Event>>,
    }

    impl RecordingSink {
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

    impl EventSink for RecordingSink {
        fn record(&self, event: &Event) {
            self.events.lock().expect("sink lock").push(event.clone());
        }
    }

    #[derive(Default)]
    struct MemoryFiles {
        files: Mutex<BTreeMap<PathBuf, Vec<u8>>>,
        denied: Mutex<BTreeSet<PathBuf>>,
    }

    impl MemoryFiles {
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

    impl ReloadFileStore for MemoryFiles {
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
        watcher: ReloadWatcher<(), ManualClock>,
        sleeps: mpsc::UnboundedReceiver<SleepRequest>,
        files: Arc<MemoryFiles>,
        sink: Arc<RecordingSink>,
        config_path: PathBuf,
        cell: Arc<ReloadCell<()>>,
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
        let files = Arc::new(MemoryFiles::default());
        let sink = Arc::new(RecordingSink::default());
        let config_path = std::path::absolute("vnc-relay.toml").expect("current directory");
        files.set(&config_path, running_text.as_bytes());
        // The watcher parses candidates against the config directory, so the running settings must
        // use the same base; otherwise relative paths differ and read as restart-required changes.
        let base_dir = config_path.parent().expect("config directory");
        let running = config_file::parse(running_text, base_dir).expect("valid running config");
        let cell = Arc::new(ReloadCell::<()>::new(ReloadSnapshot::new(
            RelayAllow::default(),
            None,
            None,
        )));
        let service = ReloadService::new(
            running,
            Arc::clone(&cell),
            files.clone(),
            Arc::new(NoTls),
            sink.clone(),
        );
        let watcher = assemble(
            &config_path,
            service,
            POLL,
            ManualClock { requests },
            files.clone(),
            sink.clone(),
            |_queue| {
                let feed: Box<dyn ChangeFeed> = Box::new(SwitchFeed { refuse });
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

    /// spec: operations.md:OPS-027
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

    #[tokio::test]
    async fn unchanged_poll_is_silent() {
        let Fixture {
            mut watcher,
            mut sleeps,
            sink,
            mut stop,
            _stop_sender,
            ..
        } = fixture();
        assert_eq!(watcher.on_tick(&mut stop).await, Flow::Continue);
        assert!(sleeps.try_recv().is_err());
        assert!(sink.names().is_empty());
    }

    /// spec: operations.md:OPS-028
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
        let before = cell.current().relay_allow.tcp_viewer.clone();
        files.deny(&config_path);
        assert_eq!(watcher.on_tick(&mut stop).await, Flow::Continue);
        assert!(sleeps.try_recv().is_err());
        assert_eq!(sink.names(), ["reload_watch_failed"]);
        // The failed read starts no episode, and the running list stays in force.
        assert_eq!(cell.current().relay_allow.tcp_viewer, before);
    }

    /// spec: operations.md:OPS-041
    #[tokio::test]
    async fn allow_change_is_applied_by_poll_when_the_notify_is_blocked() {
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
        files.set(&config_path, RELAY_CANDIDATE.as_bytes());
        let driver = async {
            let quiet = next_sleep(&mut sleeps, QUIET).await;
            let _ = quiet.send(());
            let gap = next_sleep(&mut sleeps, GAP).await;
            let _ = gap.send(());
        };
        let (flow, ()) = tokio::join!(watcher.on_tick(&mut stop), driver);
        assert_eq!(flow, Flow::Continue);
        // No notify event reaches the queue: the poll tick is the only path that found the change.
        assert!(
            watcher.wake.try_recv().is_err(),
            "no notify event was delivered"
        );
        assert_eq!(sink.names(), ["reload_change_detected", "reload_applied"]);
        assert!(matches!(
            sink.snapshot()[0],
            Event::ReloadChangeDetected {
                source: ChangeSource::Poll
            }
        ));
        // New decisions use the new list: 127.0.0.2 is permitted and the removed 127.0.0.1 is not.
        let allow = cell.current().relay_allow.tcp_viewer.clone();
        assert!(allow.permits(PeerIp::new(IpAddr::from([127, 0, 0, 2]))));
        assert!(!allow.permits(PeerIp::new(IpAddr::from([127, 0, 0, 1]))));
        assert_eq!(
            allow,
            AllowList::new(vec![IpItem::Single(IpAddr::from([127, 0, 0, 2]))])
        );
    }

    /// spec: operations.md:OPS-028
    #[test]
    fn refused_registration_at_start_is_an_error_not_a_silent_start() {
        let error = fixture_with_feed(CONFIG, Arc::new(AtomicBool::new(true)))
            .err()
            .expect("the start is refused");
        assert_eq!(error.detail(), "registration refused");
    }

    /// spec: operations.md:OPS-028
    #[tokio::test]
    async fn notify_failure_is_recorded_on_every_tick_and_keeps_the_running_allow_list() {
        let refuse = Arc::new(AtomicBool::new(false));
        let Fixture {
            mut watcher,
            sink,
            cell,
            mut stop,
            _stop_sender,
            ..
        } = fixture_with_feed(RELAY_CONFIG, Arc::clone(&refuse)).expect("watcher assembles");
        let before = cell.current().relay_allow.tcp_viewer.clone();
        refuse.store(true, Ordering::SeqCst);
        watcher.queue.offer_failure("OS notify error".to_owned());
        // The run loop consumes the wake before it calls the handler.
        watcher.wake.recv().await.expect("wake sent");
        // The OS error is recorded, and the retry in the same tick is refused too. Every later tick
        // retries registration and records the refusal again.
        assert_eq!(watcher.on_tick(&mut stop).await, Flow::Continue);
        assert_eq!(watcher.on_tick(&mut stop).await, Flow::Continue);
        assert_eq!(
            sink.names(),
            [
                "reload_watch_failed",
                "reload_watch_failed",
                "reload_watch_failed",
            ]
        );
        assert!(matches!(
            sink.snapshot()[0],
            Event::ReloadWatchFailed {
                reason: WatchFailure::NotifyError,
                ..
            }
        ));
        assert_eq!(cell.current().relay_allow.tcp_viewer, before);
        // Registration succeeds again. Recovery has no event, so the tick records nothing.
        refuse.store(false, Ordering::SeqCst);
        assert_eq!(watcher.on_tick(&mut stop).await, Flow::Continue);
        assert_eq!(sink.names().len(), 3);
    }

    /// spec: operations.md:OPS-028
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
        assert_eq!(cell.current().relay_allow.tcp_viewer, allow);
    }

    /// spec: operations.md:OPS-028
    #[tokio::test]
    async fn change_seen_during_shutdown_is_not_applied() {
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
        let before = cell.current().relay_allow.tcp_viewer.clone();
        files.set(&config_path, RELAY_CANDIDATE.as_bytes());
        let driver = async {
            let quiet = next_sleep(&mut sleeps, QUIET).await;
            // The shutdown request arrives while the episode waits for the quiet window. The sender is
            // returned, not dropped, so the window stays open until the watcher sees the stop.
            stop_sender
                .send(true)
                .expect("the watcher holds the stop receiver");
            quiet
        };
        let (flow, _quiet) = tokio::join!(watcher.on_tick(&mut stop), driver);
        assert_eq!(flow, Flow::Stop);
        assert_eq!(sink.names(), ["reload_change_detected"]);
        assert_eq!(cell.current().relay_allow.tcp_viewer, before);
    }

    /// spec: operations.md:OPS-027
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
        assert_eq!(cell.current().relay_allow.tcp_viewer, allow);
    }

    /// spec: operations.md:OPS-027
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
        let before = cell.current().relay_allow.tcp_viewer.clone();
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
        assert_eq!(cell.current().relay_allow.tcp_viewer, before);
    }

    /// Arms the §4.3 validation hook. While the candidate is being validated, `text` is written and
    /// queued as a newer change, the way a second save would arrive.
    fn arm_newer_write(
        watcher: &mut ReloadWatcher<(), ManualClock>,
        files: &Arc<MemoryFiles>,
        config_path: &Path,
        text: &str,
    ) {
        let queue = Arc::clone(&watcher.queue);
        let files = Arc::clone(files);
        let path = config_path.to_path_buf();
        let text = text.to_owned();
        watcher.service.arm_before_validation(Box::new(move || {
            files.set(&path, text.as_bytes());
            queue.offer_path(&path);
        }));
    }

    /// spec: operations.md:OPS-027
    #[tokio::test]
    async fn newer_valid_write_during_validation_discards_the_candidate_being_validated() {
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
        let before = cell.current().relay_allow.tcp_viewer.clone();
        let newest = RELAY_CANDIDATE.replace("127.0.0.2", "127.0.0.3");
        files.set(&config_path, RELAY_CANDIDATE.as_bytes());
        arm_newer_write(&mut watcher, &files, &config_path, &newest);
        let driver = async {
            let quiet = next_sleep(&mut sleeps, QUIET).await;
            let _ = quiet.send(());
            let gap = next_sleep(&mut sleeps, GAP).await;
            let _ = gap.send(());
        };
        let (flow, ()) = tokio::join!(watcher.on_tick(&mut stop), driver);
        // The hook runs after the first candidate was read. Its generation is stale at step 8, so it
        // is discarded without an event or a swap.
        assert_eq!(flow, Flow::Continue);
        assert_eq!(sink.names(), ["reload_change_detected"]);
        assert_eq!(cell.current().relay_allow.tcp_viewer, before);
        // The newer write left a wake. The run loop takes it and starts an episode for that write.
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
        assert!(matches!(
            sink.snapshot()[1],
            Event::ReloadChangeDetected {
                source: ChangeSource::OsNotify
            }
        ));
        let allow = AllowList::new(vec![IpItem::Single(IpAddr::from([127, 0, 0, 3]))]);
        assert_eq!(cell.current().relay_allow.tcp_viewer, allow);
    }

    /// spec: operations.md:OPS-027
    #[tokio::test]
    async fn newer_invalid_write_during_validation_keeps_the_running_allow_list() {
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
        let before = cell.current().relay_allow.tcp_viewer.clone();
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
        assert_eq!(cell.current().relay_allow.tcp_viewer, before);
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
        assert_eq!(cell.current().relay_allow.tcp_viewer, before);
    }

    /// spec: operations.md:OPS-028
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
        let before = cell.current().relay_allow.tcp_viewer.clone();
        files.set(&config_path, RELAY_CANDIDATE.as_bytes());
        watcher.service.arm_before_validation(Box::new(move || {
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
        assert_eq!(cell.current().relay_allow.tcp_viewer, before);
    }
}
