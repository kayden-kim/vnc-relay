//! Small synchronous CDP (Chrome DevTools Protocol) client for headless Chrome
//! or Edge. Only per-command deadlines apply; a browser may stay open for a
//! long real-time wait. Screen-specific actions belong to the test that uses
//! them. No scenario markers belong in this module.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::{Message, WebSocket, client};

const LIMIT: Duration = Duration::from_secs(20);
static NEXT: AtomicU64 = AtomicU64::new(0);

fn must<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("browser operation failed: {error:?}"))
}

fn connect(port: u16) -> TcpStream {
    let stream = must(TcpStream::connect_timeout(
        &SocketAddr::from(([127, 0, 0, 1], port)),
        LIMIT,
    ));
    must(stream.set_read_timeout(Some(LIMIT)));
    must(stream.set_write_timeout(Some(LIMIT)));
    stream
}

/// Page targets from the DevTools HTTP endpoint. The reply has a
/// `Content-Length` body.
fn page_targets(port: u16) -> Value {
    let mut stream = connect(port);
    must(
        stream
            .write_all(b"GET /json/list HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"),
    );
    let mut reader = BufReader::new(stream);
    let mut head = String::new();
    while !head.ends_with("\r\n\r\n") {
        assert!(head.len() < 65_536, "DevTools header too large");
        assert!(
            must(reader.read_line(&mut head)) != 0,
            "DevTools reply has no header boundary"
        );
    }
    assert!(
        head.starts_with("HTTP/1.1 200"),
        "DevTools target list status: {head}"
    );
    let length = head
        .lines()
        .find_map(|line| {
            line.split_once(':').and_then(|(name, value)| {
                name.eq_ignore_ascii_case("content-length")
                    .then(|| must(value.trim().parse::<usize>()))
            })
        })
        .unwrap_or_else(|| panic!("DevTools target list without Content-Length"));
    let mut body = vec![0; length];
    must(reader.read_exact(&mut body));
    must(serde_json::from_slice(&body))
}

fn executable() -> PathBuf {
    if let Some(value) = std::env::var_os("DASHBOARD_BROWSER") {
        let path = PathBuf::from(value);
        assert!(
            path.is_file(),
            "DASHBOARD_BROWSER is not an executable file"
        );
        return path;
    }
    for candidate in [
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    ] {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return path;
        }
    }
    if let Some(paths) = std::env::var_os("PATH") {
        for name in [
            "google-chrome",
            "chromium",
            "chromium-browser",
            "msedge",
            "microsoft-edge",
        ] {
            for dir in std::env::split_paths(&paths) {
                for path in [dir.join(name), dir.join(format!("{name}.exe"))] {
                    if path.is_file() {
                        return path;
                    }
                }
            }
        }
    }
    panic!("Chromium is required. Install Chrome/Edge or set DASHBOARD_BROWSER.");
}

pub struct Browser {
    child: Child,
    dir: PathBuf,
    socket: WebSocket<TcpStream>,
    next_id: u64,
    network: Vec<Value>,
}

impl Browser {
    pub fn start(name: &str, languages: &[&str]) -> Self {
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "vnc-relay-cdp-{}-{serial}-{name}",
            std::process::id()
        ));
        must(fs::create_dir_all(&dir));
        // Chromium selects and writes its own free debug port. The port is not
        // guessed or raced against another test process.
        let child = must(
            Command::new(executable())
                .args([
                    "--headless",
                    "--disable-gpu",
                    "--no-first-run",
                    "--no-default-browser-check",
                    "--disable-background-networking",
                    "--disable-background-timer-throttling",
                    "--disable-renderer-backgrounding",
                    "--remote-debugging-address=127.0.0.1",
                    "--remote-debugging-port=0",
                    "--window-size=1280,900",
                ])
                .arg(format!("--user-data-dir={}", dir.display()))
                .arg(format!(
                    "--lang={}",
                    languages.first().copied().unwrap_or("en-US")
                ))
                .arg("about:blank")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn(),
        );
        let mut process = StartingBrowser {
            child: Some(child),
            dir,
        };
        let deadline = Instant::now() + LIMIT;
        let port = loop {
            assert!(
                must(
                    process
                        .child
                        .as_mut()
                        .unwrap_or_else(|| panic!("browser process missing"))
                        .try_wait()
                )
                .is_none(),
                "browser exited before CDP became ready"
            );
            if let Ok(contents) = fs::read_to_string(process.dir.join("DevToolsActivePort"))
                && let Some(port) = contents.lines().next().and_then(|s| s.parse::<u16>().ok())
            {
                break port;
            }
            assert!(
                Instant::now() < deadline,
                "browser CDP port readiness timeout"
            );
            thread::sleep(Duration::from_millis(20));
        };
        let targets = page_targets(port);
        let url = targets
            .as_array()
            .and_then(|items| items.iter().find(|item| item["type"] == "page"))
            .and_then(|item| item["webSocketDebuggerUrl"].as_str())
            .unwrap_or_else(|| panic!("CDP page target missing"));
        let (socket, _) = must(client(url, connect(port)));
        let (child, dir) = process.take();
        let mut browser = Self {
            child,
            dir,
            socket,
            next_id: 0,
            network: Vec::new(),
        };
        browser.command("Page.enable", json!({}));
        browser.command("Runtime.enable", json!({}));
        browser.command("Network.enable", json!({}));
        // Both Accept-Language and navigator.languages are genuine browser
        // overrides. Application fetch, clocks and data remain untouched.
        let user_agent = browser.eval("navigator.userAgent");
        browser.command(
            "Network.setUserAgentOverride",
            json!({
                "userAgent": user_agent.as_str().unwrap_or(""),
                "acceptLanguage": languages.join(","),
            }),
        );
        let language_values = must(serde_json::to_string(languages));
        browser.command("Page.addScriptToEvaluateOnNewDocument", json!({
            "source": format!(
                "Object.defineProperty(navigator,'languages',{{get:()=>{language_values}}});Object.defineProperty(navigator,'language',{{get:()=>({language_values})[0]}});"
            ),
        }));
        browser
    }

    pub fn command(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let mut id = self.next_id;
        must(
            self.socket.send(Message::Text(
                json!({"id":id,"method":method,"params":params})
                    .to_string()
                    .into(),
            )),
        );
        let deadline = Instant::now() + LIMIT;
        loop {
            assert!(Instant::now() < deadline, "CDP command timeout: {method}");
            must(
                self.socket
                    .get_mut()
                    .set_read_timeout(Some(deadline.saturating_duration_since(Instant::now()))),
            );
            match must(self.socket.read()) {
                Message::Text(text) => {
                    let message: Value = must(serde_json::from_str(text.as_str()));
                    if message["id"].as_u64() != Some(id) {
                        if let Some(event) = message["method"].as_str() {
                            let params = &message["params"];
                            let record = match event {
                                "Network.loadingFailed" => Some(json!({
                                    "event":event,"request":params["requestId"],
                                    "error":params["errorText"],"canceled":params["canceled"],
                                    "blocked":params["blockedReason"],
                                })),
                                "Network.responseReceived" => Some(json!({
                                    "event":event,"request":params["requestId"],
                                    "status":params["response"]["status"],
                                })),
                                "Network.requestWillBeSent" => Some(json!({
                                    "event":event,"request":params["requestId"],
                                    "method":params["request"]["method"],
                                    "body_bytes":params["request"]["postData"].as_str().map(str::len),
                                })),
                                _ => None,
                            };
                            if let Some(record) = record {
                                if self.network.len() >= 200 {
                                    self.network.remove(0);
                                }
                                self.network.push(record);
                            }
                        }
                        continue;
                    }
                    let error = message["error"]["message"].as_str().unwrap_or("");
                    if method == "Runtime.evaluate"
                        && (error.contains("Execution context was destroyed")
                            || error.contains("Cannot find context"))
                    {
                        // A full-document navigation can replace the execution
                        // context between two bounded DOM reads.
                        thread::sleep(Duration::from_millis(20));
                        self.next_id += 1;
                        id = self.next_id;
                        must(
                            self.socket.send(Message::Text(
                                json!({"id":id,"method":method,"params":params})
                                    .to_string()
                                    .into(),
                            )),
                        );
                        continue;
                    }
                    assert!(
                        message.get("error").is_none(),
                        "CDP {method} failed: {}",
                        message["error"]
                    );
                    return message["result"].clone();
                }
                Message::Ping(_) | Message::Pong(_) | Message::Binary(_) | Message::Frame(_) => {}
                Message::Close(_) => panic!("CDP closed during {method}"),
            }
        }
    }

    /// Recent network events of the page, newest last, for failure messages.
    pub fn network(&self) -> Value {
        json!(self.network)
    }

    pub fn eval(&mut self, expression: &str) -> Value {
        let result = self.command(
            "Runtime.evaluate",
            json!({
                "expression":expression,"awaitPromise":true,"returnByValue":true,
            }),
        );
        assert!(
            result.get("exceptionDetails").is_none(),
            "browser script failed: {}",
            result["exceptionDetails"]
        );
        result["result"]["value"].clone()
    }

    pub fn navigate(&mut self, url: &str) {
        let result = self.command("Page.navigate", json!({"url":url}));
        assert!(
            result.get("errorText").is_none(),
            "browser navigation failed: {}",
            result["errorText"]
        );
        if let Some(loader) = result.get("loaderId") {
            let deadline = Instant::now() + LIMIT;
            while self.command("Page.getFrameTree", json!({}))["frameTree"]["frame"]["loaderId"]
                != *loader
            {
                assert!(
                    Instant::now() < deadline,
                    "navigation loader did not commit"
                );
                thread::sleep(Duration::from_millis(20));
            }
        }
        self.wait("document.readyState === 'complete'");
    }

    pub fn wait(&mut self, expression: &str) {
        let deadline = Instant::now() + LIMIT;
        loop {
            if self.eval(expression).as_bool() == Some(true) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "browser predicate timeout: {expression}; text: {}; network: {}",
                self.text(),
                self.network()
            );
            thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn text(&mut self) -> String {
        self.eval("document.body ? document.body.innerText : ''")
            .as_str()
            .unwrap_or("")
            .to_owned()
    }
}

// Owns a launched browser before the CDP handshake succeeds. A setup panic must
// not leave an orphan Chromium process behind.
struct StartingBrowser {
    child: Option<Child>,
    dir: PathBuf,
}

impl StartingBrowser {
    fn take(mut self) -> (Child, PathBuf) {
        (
            self.child
                .take()
                .unwrap_or_else(|| panic!("browser process missing")),
            self.dir.clone(),
        )
    }
}

impl Drop for StartingBrowser {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let deadline = Instant::now() + LIMIT;
            while matches!(child.try_wait(), Ok(None)) && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(20));
            }
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        let _ = self.socket.close(None);
        let _ = self.child.kill();
        let deadline = Instant::now() + LIMIT;
        while matches!(self.child.try_wait(), Ok(None)) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(20));
        }
        let _ = fs::remove_dir_all(&self.dir);
    }
}
