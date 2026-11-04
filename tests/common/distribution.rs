//! Exact distribution example exercised through the real binary on each OS.

use std::fs;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

const LIMIT: Duration = Duration::from_secs(45);
static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

pub trait DistributionProcess: Sized {
    fn start(config: &Path) -> Self;
    fn request_stop(&mut self);
    fn try_wait(&mut self) -> Option<i32>;
}

struct Fixture {
    dir: PathBuf,
    config: PathBuf,
    ports: [u16; 4],
}

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "vnc-relay-distribution-{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("create distribution fixture: {e}"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))
                .unwrap_or_else(|e| panic!("protect distribution fixture: {e}"));
        }
        let holders: Vec<_> = (0..4)
            .map(|_| {
                TcpListener::bind("127.0.0.1:0")
                    .unwrap_or_else(|e| panic!("reserve distribution port: {e}"))
            })
            .collect();
        let mut ports = [0; 4];
        for (port, holder) in ports.iter_mut().zip(&holders) {
            *port = holder
                .local_addr()
                .unwrap_or_else(|e| panic!("distribution port address: {e}"))
                .port();
        }
        let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/relay.toml");
        let mut text = fs::read_to_string(example)
            .unwrap_or_else(|e| panic!("read distribution example: {e}"));
        // Only listener ports change; account and log paths remain relative.
        for (original, port) in [5901, 5500, 18080, 18081].into_iter().zip(ports) {
            let old = format!("127.0.0.1:{original}\"");
            assert_eq!(text.matches(&old).count(), 1, "example listener changed");
            text = text.replace(&old, &format!("127.0.0.1:{port}\""));
        }
        let config = dir.join("relay.toml");
        fs::write(&config, text).unwrap_or_else(|e| panic!("copy distribution example: {e}"));
        Self { dir, config, ports }
    }

    fn log(&self) -> String {
        match fs::read_to_string(self.dir.join("relay.log")) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => panic!("read distribution log: {e}"),
        }
    }

    fn wait_event<P: DistributionProcess>(&self, process: &mut P, event: &str) {
        let marker = format!("\"event\":\"{event}\"");
        let deadline = Instant::now() + LIMIT;
        loop {
            let log = self.log();
            if log.contains(&marker) {
                return;
            }
            assert_eq!(
                process.try_wait(),
                None,
                "server exited before {event}: {log}"
            );
            assert!(Instant::now() < deadline, "missing {event}: {log}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn connect(port: u16) -> TcpStream {
    let address = ([127, 0, 0, 1], port).into();
    let stream = TcpStream::connect_timeout(&address, LIMIT)
        .unwrap_or_else(|e| panic!("distribution connection: {e}"));
    stream
        .set_read_timeout(Some(LIMIT))
        .unwrap_or_else(|e| panic!("distribution read timeout: {e}"));
    stream
        .set_write_timeout(Some(LIMIT))
        .unwrap_or_else(|e| panic!("distribution write timeout: {e}"));
    stream
}

struct RelayPair {
    viewer: TcpStream,
    server: TcpStream,
}

impl RelayPair {
    fn start(fixture: &Fixture, id: &str) -> Self {
        let mut server = connect(fixture.ports[1]);
        let mut viewer = connect(fixture.ports[0]);
        let mut greeting = [0; 12];
        viewer
            .read_exact(&mut greeting)
            .unwrap_or_else(|e| panic!("distribution Viewer greeting: {e}"));
        assert_eq!(&greeting, b"RFB 000.000\n");
        let request = format!("ID:{id}");
        let mut frame = [0; 250];
        frame[..request.len()].copy_from_slice(request.as_bytes());
        server
            .write_all(&frame)
            .unwrap_or_else(|e| panic!("distribution Server handshake: {e}"));
        viewer
            .write_all(&frame)
            .unwrap_or_else(|e| panic!("distribution Viewer handshake: {e}"));
        Self { viewer, server }
    }

    fn transfer(&mut self) {
        fn transfer(source: &mut TcpStream, target: &mut TcpStream, payload: &[u8; 16]) {
            source
                .write_all(payload)
                .unwrap_or_else(|e| panic!("distribution relay write: {e}"));
            let mut received = [0; 16];
            target
                .read_exact(&mut received)
                .unwrap_or_else(|e| panic!("distribution relay read: {e}"));
            assert_eq!(&received, payload);
        }
        transfer(&mut self.viewer, &mut self.server, b"viewer-to-server");
        transfer(&mut self.server, &mut self.viewer, b"server-to-viewer");
    }

    fn finish(self) {
        for peer in [self.viewer, self.server] {
            peer.shutdown(Shutdown::Both)
                .unwrap_or_else(|e| panic!("distribution peer shutdown: {e}"));
        }
    }
}

pub fn exercise_example<P: DistributionProcess>(prepare_account: impl FnOnce(&Path, &str)) {
    let fixture = Fixture::new();
    let password = format!("distribution-only-{}", std::process::id());
    prepare_account(&fixture.config, &password);
    let accounts_path = fixture.dir.join("admin-accounts.toml");
    let accounts = fs::read_to_string(&accounts_path)
        .unwrap_or_else(|e| panic!("read CLI-prepared distribution account: {e}"));
    assert!(
        accounts.contains("$argon2id$"),
        "CLI did not store an Argon2id hash"
    );
    assert!(!accounts.contains(&password), "plaintext credential stored");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&accounts_path)
            .unwrap_or_else(|e| panic!("distribution account metadata: {e}"))
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }
    let mut process = P::start(&fixture.config);
    fixture.wait_event(&mut process, "server_ready");
    let mut existing = RelayPair::start(&fixture, "560031");
    existing.transfer();
    let original = fs::read_to_string(&fixture.config)
        .unwrap_or_else(|e| panic!("read running distribution config: {e}"));
    let before = "[relay.tcp_viewer]\nenabled = true\n";
    assert!(original.contains(before), "example Viewer section changed");
    // The only running configuration edit is a reloadable IP allow list.
    let changed = original.replacen(
        "allow = [\"127.0.0.1\"]",
        "allow = [\"127.0.0.1\", \"::1\"]",
        1,
    );
    assert_ne!(changed, original);
    let began = Instant::now();
    fs::write(&fixture.config, changed)
        .unwrap_or_else(|e| panic!("save distribution IP reload: {e}"));
    fixture.wait_event(&mut process, "reload_applied");
    assert!(
        began.elapsed() < Duration::from_secs(35),
        "reload exceeded Dp + D"
    );
    existing.transfer();
    let mut new_pair = RelayPair::start(&fixture, "560032");
    new_pair.transfer();
    existing.finish();
    new_pair.finish();
    let deadline = Instant::now() + LIMIT;
    while fixture
        .log()
        .matches("\"event\":\"session_closed\"")
        .count()
        < 2
    {
        assert!(
            Instant::now() < deadline,
            "distribution sessions did not close"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    process.request_stop();
    let deadline = Instant::now() + LIMIT;
    loop {
        if let Some(code) = process.try_wait() {
            assert_eq!(code, 0, "distribution server normal exit");
            break;
        }
        assert!(
            Instant::now() < deadline,
            "distribution server did not stop"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let log = fixture.log();
    for event in [
        "server_ready",
        "session_started",
        "reload_applied",
        "shutdown_started",
        "server_stopped",
    ] {
        assert!(
            log.contains(&format!("\"event\":\"{event}\"")),
            "missing {event}: {log}"
        );
    }
    assert!(
        !log.contains("\"event\":\"shutdown_forced\""),
        "unexpected forced cleanup: {log}"
    );
    assert!(!log.contains(&password), "credential in distribution log");
    assert!(!log.contains("$argon2"), "hash in distribution log");
}
