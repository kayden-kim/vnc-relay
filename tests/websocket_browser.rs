//! Browser WebSocket clients against the real `vnc-relay` executable.
//!
//! WS-020 uses Chromium's own WebSocket implementation as the external client.
//! It shares no code with the product, which handles the upgrade with
//! tokio-tungstenite. The executable runs alone: the test starts no noVNC and
//! no websockify.
//!
//! WS-025 serves the pinned noVNC 1.7.0 archive to the same browser and
//! connects with `repeaterID`. A minimal RFB 3.8 server in this file is the TCP
//! Server. It is a test peer, not evidence of UltraVNC Server interop.
//!
//! Test procedure and prerequisites: docs/how-to/set-up-and-verify.md
//! ("noVNC 상호운용 시험 준비") and docs/conventions/testing.md section 7.

use std::fmt::Write as _;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

#[path = "common/cdp.rs"]
mod cdp;
use cdp::Browser;

const LIMIT: Duration = Duration::from_secs(20);
const GREETING: &[u8; 12] = b"RFB 000.000\n";
const RFB_VERSION: &[u8; 12] = b"RFB 003.008\n";
const NOVNC_ZIP: &str = "VNC_RELAY_NOVNC_ZIP";
const NOVNC_SHA256: &str = "d55e9cce9a611294df33c7200f32f35108652de2d86a468bebcfc6f2a1c9a196";
const BLANK_PAGE: &[u8] =
    b"<!doctype html><html><head><title>websocket client</title></head><body></body></html>";
static NEXT: AtomicU64 = AtomicU64::new(0);

/// A browser-side WebSocket client. `__URL__` and `__KEY__` are replaced
/// with JSON string literals. Received binary bytes collect in `bytes`.
const CLIENT_SCRIPT: &str = r"(() => {
  const state = { bytes: [], open: false, protocol: null, close: null, error: false, text: false };
  const socket = new WebSocket(__URL__);
  socket.binaryType = 'arraybuffer';
  socket.onopen = () => { state.open = true; state.protocol = socket.protocol; };
  socket.onmessage = (event) => {
    if (typeof event.data === 'string') { state.text = true; return; }
    for (const value of new Uint8Array(event.data)) state.bytes.push(value);
  };
  socket.onerror = () => { state.error = true; };
  socket.onclose = (event) => { state.close = event.code; };
  state.send = (values) => socket.send(new Uint8Array(values));
  state.finish = () => socket.close(1000);
  (window.relayClients ||= {})[__KEY__] = state;
  return true;
})()";

fn must<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("websocket browser test operation failed: {error:?}"))
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("crates/adapters/tests/fixtures/tls")
        .join(name)
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "vnc-relay-websocket-browser-{}-{}-{name}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    must(std::fs::create_dir_all(&dir));
    dir
}

fn connect(port: u16) -> TcpStream {
    let stream = must(TcpStream::connect_timeout(
        &SocketAddr::from(([127, 0, 0, 1], port)),
        LIMIT,
    ));
    must(stream.set_read_timeout(Some(LIMIT)));
    must(stream.set_write_timeout(Some(LIMIT)));
    must(stream.set_nodelay(true));
    stream
}

/// The 250-byte initial request: text, then NUL padding.
fn request(text: &str) -> [u8; 250] {
    let mut bytes = [0; 250];
    bytes[..text.len()].copy_from_slice(text.as_bytes());
    bytes
}

fn script_string(text: &str) -> String {
    must(serde_json::to_string(text))
}

fn bytes_of(value: &Value) -> Vec<u8> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("browser bytes are not an array: {value}"))
        .iter()
        .map(|byte| {
            byte.as_u64()
                .and_then(|byte| u8::try_from(byte).ok())
                .unwrap_or_else(|| panic!("browser value is not a byte: {byte}"))
        })
        .collect()
}

#[derive(Clone, Copy, Debug)]
enum Transport {
    Ws,
    Wss,
}

impl Transport {
    fn name(self) -> &'static str {
        match self {
            Self::Ws => "ws",
            Self::Wss => "wss",
        }
    }

    fn listener(self) -> &'static str {
        match self {
            Self::Ws => "ws",
            Self::Wss => "wss",
        }
    }
}

/// The real executable with TCP Server, WS and WSS Viewer listeners. Only the
/// test page origin may upgrade.
struct Relay {
    child: Child,
    dir: PathBuf,
    server_port: u16,
    ws_port: u16,
    wss_port: u16,
}

impl Relay {
    fn start(name: &str, origin_port: u16) -> Self {
        let holders: Vec<_> = (0..5)
            .map(|_| must(TcpListener::bind("127.0.0.1:0")))
            .collect();
        let ports: Vec<u16> = holders
            .iter()
            .map(|holder| must(holder.local_addr()).port())
            .collect();
        let dir = scratch(name);
        std::fs::write(
            dir.join("admin-accounts.toml"),
            "version = 1\n[[account]]\nname = 'alice'\npassword_hash = '$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA'\n",
        )
        .unwrap_or_else(|error| panic!("write test account file: {error}"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            must(std::fs::set_permissions(
                dir.join("admin-accounts.toml"),
                std::fs::Permissions::from_mode(0o600),
            ));
        }
        let origin = format!("http://127.0.0.1:{origin_port}");
        let config = format!(
            r"[admin]
accounts_file = 'admin-accounts.toml'
[management]
listen = '127.0.0.1:{management}'
[status]
listen = '127.0.0.1:{status}'
[relay.tcp_server]
enabled = true
listen = '127.0.0.1:{server}'
allow = ['127.0.0.1']
[relay.ws]
listen = '127.0.0.1:{ws}'
[relay.ws.viewer]
enabled = true
allow = ['127.0.0.1']
allowed_origins = ['{origin}']
[relay.wss]
listen = '127.0.0.1:{wss}'
cert_file = '{cert}'
key_file = '{key}'
[relay.wss.viewer]
enabled = true
allow = ['127.0.0.1']
allowed_origins = ['{origin}']
[mode1]
enabled = true
allow_targets = ['127.0.0.1:1-65535']
[mode2]
enabled = true
[timeouts]
handshake_secs = 20
[log]
output = 'relay.log'
",
            management = ports[0],
            status = ports[1],
            server = ports[2],
            ws = ports[3],
            wss = ports[4],
            cert = fixture("server-chain.pem")
                .display()
                .to_string()
                .replace('\\', "/"),
            key = fixture("server-key-p256.pem")
                .display()
                .to_string()
                .replace('\\', "/"),
        );
        let config_path = dir.join("relay.toml");
        must(std::fs::write(&config_path, config));
        drop(holders);
        let child = must(
            Command::new(env!("CARGO_BIN_EXE_vnc-relay"))
                .args(["serve", "--config"])
                .arg(&config_path)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::from(must(std::fs::File::create(
                    dir.join("stderr.txt"),
                ))))
                .spawn(),
        );
        let mut relay = Self {
            child,
            dir,
            server_port: ports[2],
            ws_port: ports[3],
            wss_port: ports[4],
        };
        relay.wait_after(0, "server_ready", |event| event["event"] == "server_ready");
        relay
    }

    fn port(&self, transport: Transport) -> u16 {
        match transport {
            Transport::Ws => self.ws_port,
            Transport::Wss => self.wss_port,
        }
    }

    /// Complete log events in write order. An unfinished last line is not
    /// parsed.
    fn events(&self) -> Vec<Value> {
        let text = match std::fs::read_to_string(self.dir.join("relay.log")) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => return Vec::new(),
            Err(error) => panic!("read relay log: {error}"),
        };
        text.rsplit_once('\n')
            .map_or("", |(complete, _)| complete)
            .lines()
            .map(|line| must(serde_json::from_str(line)))
            .collect()
    }

    fn mark(&self) -> usize {
        self.events().len()
    }

    /// The first event at or after `mark` that matches.
    fn wait_after(&mut self, mark: usize, label: &str, matches: impl Fn(&Value) -> bool) -> Value {
        let deadline = Instant::now() + LIMIT;
        loop {
            let events = self.events();
            if let Some(found) = events.iter().skip(mark).find(|event| matches(event)) {
                return found.clone();
            }
            if let Some(status) = must(self.child.try_wait()) {
                let stderr =
                    std::fs::read_to_string(self.dir.join("stderr.txt")).unwrap_or_default();
                panic!("relay exited {status} before {label}: {stderr}; events: {events:?}");
            }
            assert!(
                Instant::now() < deadline,
                "relay event timeout: {label}; events: {events:?}"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for Relay {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Test-side static HTTP server. It serves `/blank.html` and, when a root is
/// given, the files below that root. It is the page origin for the browser.
struct Site {
    port: u16,
    stop: Arc<AtomicBool>,
    accept: Option<JoinHandle<()>>,
}

impl Site {
    fn start(root: Option<PathBuf>) -> Self {
        let listener = must(TcpListener::bind("127.0.0.1:0"));
        let port = must(listener.local_addr()).port();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let accept = thread::spawn(move || {
            for stream in listener.incoming() {
                if flag.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(stream) = stream else { continue };
                let root = root.clone();
                thread::spawn(move || {
                    // A browser may close a preconnected socket unused.
                    let _ = serve_file(stream, root.as_deref());
                });
            }
        });
        Self {
            port,
            stop,
            accept: Some(accept),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }
}

impl Drop for Site {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the blocking accept loop so it can see the stop flag.
        let _ = TcpStream::connect_timeout(&SocketAddr::from(([127, 0, 0, 1], self.port)), LIMIT);
        if let Some(accept) = self.accept.take() {
            let _ = accept.join();
        }
    }
}

fn serve_file(mut stream: TcpStream, root: Option<&Path>) -> std::io::Result<()> {
    // The listener blocks, so the accepted socket blocks too. A read timeout
    // bounds an idle browser preconnection.
    stream.set_read_timeout(Some(LIMIT))?;
    let mut head = Vec::new();
    let mut byte = [0; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if head.len() >= 16_384 || stream.read(&mut byte)? == 0 {
            return Ok(());
        }
        head.push(byte[0]);
    }
    let head = String::from_utf8_lossy(&head);
    let target = head.split_whitespace().nth(1).unwrap_or("/");
    let path = target.split(['?', '#']).next().unwrap_or("/");
    let (status, kind, body) = match resolve(path, root) {
        Some((kind, body)) => ("200 OK", kind, body),
        None => ("404 Not Found", "text/plain", b"not found".to_vec()),
    };
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(&body)?;
    stream.flush()
}

/// File bytes and media type for a request path. Empty, `.` and `..`
/// segments are refused, so no path leaves the root.
fn resolve(path: &str, root: Option<&Path>) -> Option<(&'static str, Vec<u8>)> {
    if path == "/blank.html" {
        return Some(("text/html; charset=utf-8", BLANK_PAGE.to_vec()));
    }
    let mut file = root?.to_path_buf();
    for segment in path.trim_start_matches('/').split('/') {
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || segment.contains(['\\', '%', ':'])
        {
            return None;
        }
        file.push(segment);
    }
    let body = std::fs::read(&file).ok()?;
    // Module scripts need a JavaScript media type.
    let kind = match file.extension().and_then(|extension| extension.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript",
        Some("css") => "text/css",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        _ => "application/octet-stream",
    };
    Some((kind, body))
}

/// Opens a browser WebSocket to the relay under `key` and checks the 12-byte
/// greeting that the relay sends first on a Viewer connection.
fn open_browser_client(browser: &mut Browser, key: &str, url: &str) -> String {
    let script = CLIENT_SCRIPT
        .replace("__URL__", &script_string(url))
        .replace("__KEY__", &script_string(key));
    assert_eq!(browser.eval(&script), json!(true));
    let client = format!("window.relayClients[{}]", script_string(key));
    browser.wait(&format!("{client}.bytes.length >= 12"));
    assert_eq!(
        bytes_of(&browser.eval(&format!("{client}.bytes"))),
        GREETING
    );
    // No subprotocol was offered, so the 101 response chose none.
    assert_eq!(browser.eval(&format!("{client}.protocol")), json!(""));
    client
}

fn browser_send(browser: &mut Browser, client: &str, bytes: &[u8]) {
    let values = must(serde_json::to_string(bytes));
    browser.eval(&format!("{client}.send({values})"));
}

fn pattern(length: usize, seed: u8) -> Vec<u8> {
    (0..length)
        .map(|index| {
            let low = u8::try_from(index % 251).unwrap_or(0);
            low.wrapping_mul(7).wrapping_add(seed)
        })
        .collect()
}

/// Accepts the Mode I target connection within the limit. The accepted socket
/// is set back to blocking: Windows gives it the listener's non-blocking mode.
fn accept_target(listener: &TcpListener) -> TcpStream {
    must(listener.set_nonblocking(true));
    let deadline = Instant::now() + LIMIT;
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                must(stream.set_nonblocking(false));
                must(stream.set_read_timeout(Some(LIMIT)));
                must(stream.set_write_timeout(Some(LIMIT)));
                return stream;
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                assert!(Instant::now() < deadline, "Mode I target was not connected");
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("accept Mode I target: {error}"),
        }
    }
}

/// One WS-020 combination: the browser is the Viewer, `peer` is the TCP Server
/// or the Mode I target. Bytes cross both ways, then the browser closes.
fn exercise_browser_relay(
    browser: &mut Browser,
    relay: &mut Relay,
    transport: Transport,
    mode2: Option<&str>,
    key: &str,
) {
    let url = format!(
        "{}://127.0.0.1:{}/relay/viewer",
        transport.name(),
        relay.port(transport)
    );
    let mark = relay.mark();
    let target = must(TcpListener::bind("127.0.0.1:0"));
    let (server, request_text) = if let Some(id) = mode2 {
        let mut server = connect(relay.server_port);
        must(server.write_all(&request(&format!("ID:{id}"))));
        relay.wait_after(mark, "Server registration", |event| {
            event["event"] == "waiting_registered" && event["role"] == "server"
        });
        (Some(server), format!("ID:{id}"))
    } else {
        (
            None,
            format!("127.0.0.1:{}", must(target.local_addr()).port()),
        )
    };
    let client = open_browser_client(browser, key, &url);
    browser_send(browser, &client, &request(&request_text));
    let mut peer = match server {
        Some(server) => server,
        None => accept_target(&target),
    };
    let started = relay.wait_after(mark, "session start", |event| {
        event["event"] == "session_started"
            && event["viewer_transport"] == transport.name()
            && event["mode"] == if mode2.is_some() { "mode2" } else { "mode1" }
    });

    let upstream = pattern(4096, 3);
    browser_send(browser, &client, &upstream);
    let mut received = vec![0; upstream.len()];
    must(peer.read_exact(&mut received));
    assert_eq!(received, upstream, "browser to TCP bytes");

    let downstream = pattern(4096, 101);
    must(peer.write_all(&downstream));
    let expected = GREETING.len() + downstream.len();
    browser.wait(&format!("{client}.bytes.length >= {expected}"));
    let all = bytes_of(&browser.eval(&format!("{client}.bytes")));
    assert_eq!(all.len(), expected, "no extra bytes reach the browser");
    assert_eq!(
        &all[GREETING.len()..],
        downstream.as_slice(),
        "TCP to browser bytes"
    );
    assert_eq!(
        browser.eval(&format!("{client}.text || {client}.error")),
        json!(false),
        "binary only, no WebSocket error"
    );

    browser.eval(&format!("{client}.finish()"));
    let mut rest = [0; 1];
    assert_eq!(
        must(peer.read(&mut rest)),
        0,
        "TCP peer gets EOF after the browser closes"
    );
    browser.wait(&format!("{client}.close === 1000"));
    let session = started["session_id"].clone();
    let closed = relay.wait_after(mark, "session close", |event| {
        event["event"] == "session_closed" && event["session_id"] == session
    });
    assert_eq!(closed["viewer_transport"], transport.name());
    assert_eq!(closed["bytes_viewer_to_target"], json!(upstream.len()));
    assert_eq!(closed["bytes_target_to_viewer"], json!(downstream.len()));
}

/// spec: websocket.md:WS-020
#[test]
fn browser_websocket_relays_mode_i_and_ii_over_ws_and_wss_without_novnc_or_websockify() {
    let site = Site::start(None);
    let mut relay = Relay::start("browser", site.port);
    let mut browser = Browser::start("websocket-browser", &["en-US"]);
    // The fixture chain is signed by a test root that the browser does not
    // trust. The relay log below still proves the WSS listener carried it.
    browser.command(
        "Security.setIgnoreCertificateErrors",
        json!({"ignore": true}),
    );
    browser.navigate(&site.url("/blank.html"));
    for (key, transport, mode2) in [
        ("ws-mode2", Transport::Ws, Some("20201")),
        ("ws-mode1", Transport::Ws, None),
        ("wss-mode2", Transport::Wss, Some("20202")),
        ("wss-mode1", Transport::Wss, None),
    ] {
        exercise_browser_relay(&mut browser, &mut relay, transport, mode2, key);
    }
}

/// What the RFB client sent after ServerInit.
#[derive(Debug, Default)]
struct RfbOutcome {
    client_version: Vec<u8>,
    security_type: u8,
    set_pixel_format: bool,
    set_encodings: bool,
    update_requests: usize,
}

/// Connects as the TCP Server, like `winvnc -id`: the 250-byte request and
/// the RFB version in one write, before any match.
fn rfb_server_connect(relay: &Relay, id: &str) -> TcpStream {
    let mut server = connect(relay.server_port);
    let mut first = request(&format!("ID:{id}")).to_vec();
    first.extend_from_slice(RFB_VERSION);
    must(server.write_all(&first));
    server
}

fn read_array<const N: usize>(stream: &mut TcpStream) -> [u8; N] {
    let mut bytes = [0; N];
    must(stream.read_exact(&mut bytes));
    bytes
}

/// Minimal RFB 3.8 server: security type None, a 4x4 true-colour desktop,
/// one raw update. Done when the client asks for the next update, which it
/// does only after it has read the first one.
fn complete_rfb(mut stream: TcpStream, name: &str) -> (RfbOutcome, TcpStream) {
    let mut outcome = RfbOutcome {
        client_version: read_array::<12>(&mut stream).to_vec(),
        ..RfbOutcome::default()
    };
    must(stream.write_all(&[1, 1]));
    outcome.security_type = read_array::<1>(&mut stream)[0];
    must(stream.write_all(&0_u32.to_be_bytes()));
    let _shared = read_array::<1>(&mut stream);
    let mut init = Vec::new();
    init.extend_from_slice(&4_u16.to_be_bytes());
    init.extend_from_slice(&4_u16.to_be_bytes());
    init.extend_from_slice(&[32, 24, 0, 1, 0, 255, 0, 255, 0, 255, 16, 8, 0, 0, 0, 0]);
    init.extend_from_slice(&must(u32::try_from(name.len())).to_be_bytes());
    init.extend_from_slice(name.as_bytes());
    must(stream.write_all(&init));
    let mut bytes_per_pixel = 4;
    while outcome.update_requests < 2 {
        match read_array::<1>(&mut stream)[0] {
            0 => {
                let rest = read_array::<19>(&mut stream);
                bytes_per_pixel = usize::from(rest[3] / 8);
                outcome.set_pixel_format = true;
            }
            2 => {
                let rest = read_array::<3>(&mut stream);
                let count = usize::from(u16::from_be_bytes([rest[1], rest[2]]));
                let mut list = vec![0; count * 4];
                must(stream.read_exact(&mut list));
                outcome.set_encodings = true;
            }
            3 => {
                let _request = read_array::<9>(&mut stream);
                outcome.update_requests += 1;
                if outcome.update_requests == 1 {
                    let mut update = vec![0, 0, 0, 1, 0, 0, 0, 0, 0, 4, 0, 4, 0, 0, 0, 0];
                    update.resize(update.len() + 16 * bytes_per_pixel, 0x80);
                    must(stream.write_all(&update));
                }
            }
            4 => {
                let _key = read_array::<7>(&mut stream);
            }
            5 => {
                let _pointer = read_array::<5>(&mut stream);
            }
            6 => {
                let rest = read_array::<7>(&mut stream);
                let length = u32::from_be_bytes([rest[3], rest[4], rest[5], rest[6]]);
                let mut text = vec![0; must(usize::try_from(length))];
                must(stream.read_exact(&mut text));
            }
            other => panic!("unexpected RFB client message type {other}"),
        }
    }
    (outcome, stream)
}

/// SHA-256 through the rustls aws-lc-rs provider, so the test adds no
/// dependency beyond the existing `rustls` dev-dependency.
fn sha256_hex(bytes: &[u8]) -> String {
    let rustls::SupportedCipherSuite::Tls13(suite) =
        rustls::crypto::aws_lc_rs::cipher_suite::TLS13_AES_128_GCM_SHA256
    else {
        panic!("TLS13_AES_128_GCM_SHA256 is not a TLS 1.3 suite");
    };
    let digest = suite.common.hash_provider.hash(bytes);
    let mut hex = String::new();
    for byte in digest.as_ref() {
        must(write!(hex, "{byte:02x}"));
    }
    hex
}

/// Checks the pinned archive and unpacks it below `dir`. Windows uses its
/// built-in bsdtar; Ubuntu uses the Python standard library.
fn novnc_root(dir: &Path) -> PathBuf {
    let archive = std::env::var_os(NOVNC_ZIP)
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("set {NOVNC_ZIP} to the pinned novnc-v1.7.0.zip"));
    let bytes = std::fs::read(&archive)
        .unwrap_or_else(|error| panic!("read {}: {error}", archive.display()));
    assert_eq!(
        sha256_hex(&bytes),
        NOVNC_SHA256,
        "{} is not the pinned noVNC 1.7.0 archive",
        archive.display()
    );
    #[cfg(windows)]
    let status = Command::new(
        Path::new(&std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into()))
            .join(r"System32\tar.exe"),
    )
    .arg("-xf")
    .arg(&archive)
    .arg("-C")
    .arg(dir)
    .status();
    #[cfg(not(windows))]
    let status = Command::new("python3")
        .args(["-m", "zipfile", "-e"])
        .arg(&archive)
        .arg(dir)
        .status();
    assert!(must(status).success(), "unpack the noVNC archive");
    let root = dir.join("noVNC-1.7.0");
    assert!(root.join("vnc.html").is_file(), "noVNC vnc.html missing");
    root
}

/// spec: websocket.md:WS-025
#[test]
#[ignore = "pinned noVNC 1.7.0 zip in VNC_RELAY_NOVNC_ZIP; docs/how-to/set-up-and-verify.md#novnc-상호운용-시험-준비"]
fn novnc_repeater_id_completes_rfb_through_ws_and_wss_in_both_server_orders() {
    let dir = scratch("novnc");
    let site = Site::start(Some(novnc_root(&dir)));
    let mut relay = Relay::start("novnc", site.port);
    let mut browser = Browser::start("novnc", &["en-US"]);
    browser.command(
        "Security.setIgnoreCertificateErrors",
        json!({"ignore": true}),
    );
    for (index, (transport, server_first)) in [
        (Transport::Ws, true),
        (Transport::Ws, false),
        (Transport::Wss, true),
        (Transport::Wss, false),
    ]
    .into_iter()
    .enumerate()
    {
        let id = format!("2502{index}");
        let order = if server_first {
            "server-first"
        } else {
            "viewer-first"
        };
        let name = format!("ws-025 {} {order}", transport.name());
        // Separate host, port, encrypt and path: an `http:` page with a
        // `wss:` path connected in plain text in the recorded observation.
        let url = site.url(&format!(
            "/vnc.html?autoconnect=true&reconnect=false&repeaterID={id}&host=127.0.0.1&port={}&encrypt={}&path=relay/viewer",
            relay.port(transport),
            matches!(transport, Transport::Wss)
        ));
        let mark = relay.mark();
        let server = if server_first {
            let server = rfb_server_connect(&relay, &id);
            relay.wait_after(mark, "Server registration", |event| {
                event["event"] == "waiting_registered" && event["role"] == "server"
            });
            browser.navigate(&url);
            server
        } else {
            browser.navigate(&url);
            relay.wait_after(mark, "noVNC registration", |event| {
                event["event"] == "waiting_registered"
                    && event["role"] == "viewer"
                    && event["listener"] == transport.listener()
            });
            rfb_server_connect(&relay, &id)
        };
        let desktop = name.clone();
        let rfb = thread::spawn(move || complete_rfb(server, &desktop));
        let (outcome, server) = rfb
            .join()
            .unwrap_or_else(|_| panic!("{name}: RFB server thread failed"));
        assert_eq!(outcome.client_version, RFB_VERSION, "{name}");
        assert_eq!(outcome.security_type, 1, "{name}: security None");
        assert!(outcome.set_pixel_format, "{name}: SetPixelFormat");
        assert!(outcome.set_encodings, "{name}: SetEncodings");
        assert_eq!(
            outcome.update_requests, 2,
            "{name}: update read, next requested"
        );
        browser.wait(&format!(
            "document.title === {}",
            script_string(&format!("{name} - noVNC"))
        ));
        let started = relay.wait_after(mark, "session start", |event| {
            event["event"] == "session_started"
                && event["mode"] == "mode2"
                && event["viewer_transport"] == transport.name()
        });
        let registered = relay.wait_after(mark, "registration", |event| {
            event["event"] == "waiting_registered"
        });
        assert_eq!(
            started["relay_id_ref"], registered["relay_id_ref"],
            "{name}: same ID"
        );

        browser.navigate("about:blank");
        let session = started["session_id"].clone();
        relay.wait_after(mark, "session close", |event| {
            event["event"] == "session_closed" && event["session_id"] == session
        });
        drop(server);
    }
    drop(site);
    let _ = std::fs::remove_dir_all(&dir);
}
