use super::*;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::oneshot;
use vnc_relay_app::reload::ports::{ReloadTlsStore, TlsBuildError};
use vnc_relay_app::reload::service::ReloadService;
use vnc_relay_app::reload::snapshot::{ReloadCell, ReloadSnapshot};
use vnc_relay_app::settings::LogOutput;

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "vr-reopen-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap_or_else(|e| panic!("create temp dir: {e}"));
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

type Request = (Duration, oneshot::Sender<()>);
struct Clock(mpsc::UnboundedSender<Request>);
impl ReloadClock for Clock {
    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + Send {
        let (tx, rx) = oneshot::channel();
        self.0
            .send((duration, tx))
            .unwrap_or_else(|e| panic!("clock request: {e}"));
        async move {
            let _ = rx.await;
        }
    }
}
async fn tick(requests: &mut mpsc::UnboundedReceiver<Request>) -> oneshot::Sender<()> {
    let (duration, tx) = tokio::time::timeout(Duration::from_secs(5), requests.recv())
        .await
        .unwrap_or_else(|e| panic!("clock deadline: {e}"))
        .unwrap_or_else(|| panic!("clock closed"));
    assert_eq!(duration, Duration::from_secs(30));
    tx
}
async fn wait_for(mut ready: impl FnMut() -> bool) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !ready() {
        assert!(tokio::time::Instant::now() < deadline, "condition deadline");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
fn text(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read log: {e}"))
}
fn marker(log: &JsonLinesEventSink, value: &str) {
    log.record(&Event::ReloadRestartRequired {
        config_keys: vec![value.to_owned()],
    });
}
fn notify(queue: &ChangeQueue, path: &Path, kind: notify::EventKind) {
    queue.offer_event(&notify::Event::new(kind).add_path(path.to_path_buf()));
}

/// spec: operations.md:OPS-045
#[tokio::test]
async fn reopens_on_notifications_and_poll_and_reports_each_failure_streak_once() {
    let dir = Directory::new();
    let path = dir.0.join("relay.log");
    let old = dir.0.join("old.log");
    let polled = dir.0.join("polled.log");
    let notices = Arc::new(Mutex::new(Vec::new()));
    let notice_copy = notices.clone();
    let log = Arc::new(
        JsonLinesEventSink::open(
            &LogOutput::File(path.clone()),
            |e| panic!("write: {e}"),
            move |e| {
                notice_copy
                    .lock()
                    .unwrap_or_else(|e| panic!("notice lock: {e}"))
                    .push(e.to_string());
            },
        )
        .unwrap_or_else(|e| panic!("open log: {e}")),
    );
    let (queue, mut wake) = ChangeQueue::new();
    queue.set_log_targets(vec![
        WatchTarget::for_path(&path).unwrap_or_else(|| panic!("log target")),
    ]);
    let (requests, mut sleeps) = mpsc::unbounded_channel();
    let (stop, stopped) = watch::channel(false);
    let driver = async {
        let first_tick = tick(&mut sleeps).await;
        marker(&log, "before_notify");
        std::fs::rename(&path, &old).unwrap_or_else(|e| panic!("rotate: {e}"));
        notify(
            &queue,
            &path,
            notify::EventKind::Modify(notify::event::ModifyKind::Name(
                notify::event::RenameMode::From,
            )),
        );
        wait_for(|| path.is_file()).await;
        marker(&log, "after_notify");
        assert!(text(&old).contains("before_notify"));
        assert!(!text(&old).contains("after_notify"));
        // Our writes and close-after-write events must neither reopen nor start reload episodes.
        std::fs::rename(&path, &polled).unwrap_or_else(|e| panic!("rotate without notify: {e}"));
        notify(
            &queue,
            &path,
            notify::EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Any,
            )),
        );
        notify(
            &queue,
            &path,
            notify::EventKind::Access(notify::event::AccessKind::Close(
                notify::event::AccessMode::Write,
            )),
        );
        assert_eq!(queue.generation(), 0);
        assert!(wake.try_recv().is_err());
        assert!(!path.exists());
        first_tick
            .send(())
            .unwrap_or_else(|()| panic!("release first tick"));
        let next = tick(&mut sleeps).await;
        assert!(path.is_file());
        marker(&log, "after_poll");
        // Replace the path by a directory. Repeated failures keep the old handle and report once.
        let kept = dir.0.join("kept.log");
        std::fs::rename(&path, &kept).unwrap_or_else(|e| panic!("keep handle: {e}"));
        std::fs::create_dir(&path).unwrap_or_else(|e| panic!("block path: {e}"));
        next.send(())
            .unwrap_or_else(|()| panic!("release failed tick"));
        let next = tick(&mut sleeps).await;
        marker(&log, "during_failure");
        next.send(())
            .unwrap_or_else(|()| panic!("repeat failed tick"));
        let next = tick(&mut sleeps).await;
        assert_eq!(
            notices
                .lock()
                .unwrap_or_else(|e| panic!("notice lock: {e}"))
                .len(),
            1
        );
        let failure_lines: Vec<serde_json::Value> = text(&kept)
            .lines()
            .map(|line| serde_json::from_str(line).unwrap_or_else(|e| panic!("JSON: {e}")))
            .filter(|line: &serde_json::Value| line["event"] == "log_reopen_failed")
            .collect();
        assert_eq!(failure_lines.len(), 1);
        assert_eq!(failure_lines[0]["level"], "warn");
        assert_eq!(failure_lines[0]["reason"], "log_file_unavailable");
        assert_eq!(failure_lines[0]["config_key"], "log.output");
        assert!(
            !failure_lines[0]["detail"]
                .as_str()
                .unwrap_or_else(|| panic!("detail"))
                .contains("relay.log")
        );
        assert!(text(&kept).contains("during_failure"));
        std::fs::remove_dir(&path).unwrap_or_else(|e| panic!("restore path: {e}"));
        next.send(()).unwrap_or_else(|()| panic!("recovery tick"));
        let next = tick(&mut sleeps).await;
        marker(&log, "after_recovery");
        let recovered = dir.0.join("recovered.log");
        std::fs::rename(&path, &recovered).unwrap_or_else(|e| panic!("rotate recovered: {e}"));
        std::fs::create_dir(&path).unwrap_or_else(|e| panic!("block again: {e}"));
        // A remove/create notification also retries immediately, without releasing Dp.
        notify(
            &queue,
            &path,
            notify::EventKind::Remove(notify::event::RemoveKind::File),
        );
        wait_for(|| {
            notices
                .lock()
                .unwrap_or_else(|e| panic!("notice lock: {e}"))
                .len()
                == 2
        })
        .await;
        std::fs::remove_dir(&path).unwrap_or_else(|e| panic!("unblock again: {e}"));
        notify(
            &queue,
            &path,
            notify::EventKind::Create(notify::event::CreateKind::File),
        );
        wait_for(|| path.is_file()).await;
        marker(&log, "after_create");
        let all = [&old, &polled, &kept, &recovered, &path]
            .into_iter()
            .map(|p| text(p))
            .collect::<String>();
        for value in [
            "before_notify",
            "after_notify",
            "after_poll",
            "during_failure",
            "after_recovery",
            "after_create",
        ] {
            assert_eq!(all.matches(value).count(), 1, "marker {value}");
        }
        assert_eq!(all.matches("\"event\":\"log_reopen_failed\"").count(), 2);
        assert_eq!(queue.generation(), 0);
        assert!(wake.try_recv().is_err());
        stop.send(true).unwrap_or_else(|e| panic!("stop: {e}"));
        next
    };
    let ((), _held_tick) = tokio::join!(
        reopen_log(
            log.clone(),
            queue.clone(),
            Arc::new(Clock(requests)),
            Duration::from_secs(30),
            stopped
        ),
        driver
    );
}

struct NoTls;
impl ReloadTlsStore<()> for NoTls {
    fn build(&self, _: &[u8], _: &[u8]) -> Result<(), TlsBuildError> {
        Ok(())
    }
}
struct RefuseLog {
    log_dir: PathBuf,
    refuse: Arc<AtomicBool>,
    attempts: Arc<AtomicU64>,
}
impl ChangeFeed for RefuseLog {
    fn watch_dirs(&mut self, dirs: &BTreeSet<PathBuf>) -> Result<(), ReloadWatchError> {
        if dirs.contains(&self.log_dir) {
            self.attempts.fetch_add(1, Ordering::SeqCst);
            if self.refuse.load(Ordering::SeqCst) {
                return Err(ReloadWatchError::new("log registration refused".to_owned()));
            }
        }
        Ok(())
    }
}

/// spec: operations.md:OPS-045 partial
#[tokio::test]
async fn log_directory_registration_failure_keeps_poll_and_retries() {
    let dir = Directory::new();
    let log_dir = dir.0.join("logs");
    std::fs::create_dir(&log_dir).unwrap_or_else(|e| panic!("log dir: {e}"));
    let config = dir.0.join("relay.toml");
    let source = "[admin]\naccounts_file = 'accounts.toml'\n[management]\nlisten = '127.0.0.1:18080'\n[status]\nlisten = '127.0.0.1:18081'\n";
    std::fs::write(&config, source).unwrap_or_else(|e| panic!("config: {e}"));
    let settings = config_file::parse_candidate(source.as_bytes(), &dir.0)
        .unwrap_or_else(|e| panic!("parse: {e:?}"));
    let path = log_dir.join("relay.log");
    let log = Arc::new(
        JsonLinesEventSink::open(
            &LogOutput::File(path.clone()),
            |e| panic!("write: {e}"),
            |e| panic!("reopen: {e}"),
        )
        .unwrap_or_else(|e| panic!("open: {e}")),
    );
    let files = Arc::new(FsReloadFileStore);
    let service = ReloadService::new(
        settings.clone(),
        Arc::new(ReloadCell::new(ReloadSnapshot::new(settings, None, None))),
        files.clone(),
        Arc::new(NoTls),
        Arc::new(crate::ticket::TicketJwksStore),
        log.clone(),
    );
    let refuse = Arc::new(AtomicBool::new(true));
    let attempts = Arc::new(AtomicU64::new(0));
    let (requests, _sleeps) = mpsc::unbounded_channel();
    let mut watcher = assemble(
        &config,
        Arc::new(Mutex::new(service)),
        Duration::from_secs(30),
        Clock(requests),
        files,
        log.clone(),
        |_| {
            Ok(Box::new(RefuseLog {
                log_dir,
                refuse: refuse.clone(),
                attempts: attempts.clone(),
            }))
        },
    )
    .unwrap_or_else(|e| panic!("config registration must succeed: {e}"));
    watcher.attach_log(log.clone());
    assert!(watcher.notify_failed);
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
    assert_eq!(
        text(&path)
            .matches("\"event\":\"reload_watch_failed\"")
            .count(),
        1
    );
    assert!(!text(&path).contains("startup_failed"));
    let (_stop, mut stopped) = watch::channel(false);
    assert_eq!(watcher.on_tick(&mut stopped).await, Flow::Continue);
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    refuse.store(false, Ordering::SeqCst);
    assert_eq!(watcher.on_tick(&mut stopped).await, Flow::Continue);
    assert!(!watcher.notify_failed);
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
    assert_eq!(watcher.processed.len(), 1);
    assert!(!watcher.watched.contains(&path));
}

#[test]
fn stdout_has_no_reopen_target_and_emits_no_failure() {
    let log = JsonLinesEventSink::open(
        &LogOutput::Stdout,
        |e| panic!("write: {e}"),
        |e| panic!("reopen: {e}"),
    )
    .unwrap_or_else(|e| panic!("stdout: {e}"));
    assert!(log.file_path().is_none());
    log.reopen();
}
