//! Small synchronous CDP client. Only per-command deadlines apply; a browser
//! may stay open throughout the real 300-second management idle expiry.

use std::fs;
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::{Message, WebSocket, client};

use super::{LIMIT, NEXT, PASSWORD, Server, connect, must, read_reply};

pub struct Browser {
    child: Child,
    dir: PathBuf,
    socket: WebSocket<TcpStream>,
    next_id: u64,
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

impl Browser {
    pub fn start(name: &str, languages: &[&str]) -> Self {
        let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
        let mut stream = connect(port);
        must(std::io::Write::write_all(
            &mut stream,
            b"GET /json/list HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        ));
        let reply = read_reply(stream);
        assert_eq!(reply.status, 200);
        let targets: Value = must(serde_json::from_str(&reply.body));
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

    fn command(&mut self, method: &str, params: Value) -> Value {
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
                        continue;
                    }
                    let error = message["error"]["message"].as_str().unwrap_or("");
                    if method == "Runtime.evaluate"
                        && (error.contains("Execution context was destroyed")
                            || error.contains("Cannot find context"))
                    {
                        // A full-document login/logout navigation can replace
                        // the execution context between two bounded DOM reads.
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

    pub fn reload(&mut self) {
        let previous =
            self.command("Page.getFrameTree", json!({}))["frameTree"]["frame"]["loaderId"].clone();
        self.command("Page.reload", json!({"ignoreCache":true}));
        let deadline = Instant::now() + LIMIT;
        while self.command("Page.getFrameTree", json!({}))["frameTree"]["frame"]["loaderId"]
            == previous
        {
            assert!(Instant::now() < deadline, "reload loader did not commit");
            thread::sleep(Duration::from_millis(20));
        }
        self.wait("document.readyState === 'complete'");
    }

    pub fn wait(&mut self, expression: &str) {
        self.wait_limit(expression, LIMIT);
    }

    pub fn wait_limit(&mut self, expression: &str, limit: Duration) {
        let deadline = Instant::now() + limit;
        loop {
            if self.eval(expression).as_bool() == Some(true) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "browser predicate timeout: {expression}; text: {}",
                self.text()
            );
            thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn click(&mut self, selector: &str) {
        let selector = must(serde_json::to_string(selector));
        self.wait(&format!("(()=>{{const e=document.querySelector({selector});return !!e&&e.getBoundingClientRect().width>0}})()"));
        let point = self.eval(&format!("(()=>{{const e=document.querySelector({selector});e.scrollIntoView({{block:'center'}});const r=e.getBoundingClientRect();return {{x:r.x+r.width/2,y:r.y+r.height/2}}}})()"));
        let x = point["x"].as_f64().unwrap_or(0.0);
        let y = point["y"].as_f64().unwrap_or(0.0);
        self.command(
            "Input.dispatchMouseEvent",
            json!({"type":"mousePressed","x":x,"y":y,"button":"left","clickCount":1}),
        );
        self.command(
            "Input.dispatchMouseEvent",
            json!({"type":"mouseReleased","x":x,"y":y,"button":"left","clickCount":1}),
        );
    }

    pub fn fill(&mut self, selector: &str, value: &str) {
        let selector = must(serde_json::to_string(selector));
        self.wait(&format!("document.querySelector({selector}) !== null"));
        self.eval(&format!(
            "(()=>{{const e=document.querySelector({selector});e.focus();e.select()}})()"
        ));
        self.command("Input.insertText", json!({"text":value}));
    }

    pub fn key(&mut self, key: &str) {
        let code = match key {
            "Escape" => 27,
            "Enter" => 13,
            "Tab" => 9,
            _ => 0,
        };
        for kind in ["keyDown", "keyUp"] {
            self.command("Input.dispatchKeyEvent", json!({"type":kind,"key":key,"code":key,"windowsVirtualKeyCode":code,"nativeVirtualKeyCode":code}));
        }
    }

    pub fn text(&mut self) -> String {
        self.eval("document.body.innerText")
            .as_str()
            .unwrap_or("")
            .to_owned()
    }

    pub fn login(&mut self, server: &Server) {
        self.navigate(&server.url("/login/"));
        self.fill("#login-account", "alice");
        self.fill("#login-password", PASSWORD);
        self.click("#login-submit");
        self.wait(
            "location.pathname === '/' && document.querySelector('#operations-refresh') !== null",
        );
        self.wait("!document.body.innerText.includes('Loading') && !document.body.innerText.includes('조회 중')");
    }

    pub fn clear_cookies(&mut self) {
        self.command("Network.clearBrowserCookies", json!({}));
    }

    pub fn set_width(&mut self, width: u32) {
        self.command(
            "Emulation.setDeviceMetricsOverride",
            json!({"width":width,"height":900,"deviceScaleFactor":1,"mobile":false}),
        );
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
