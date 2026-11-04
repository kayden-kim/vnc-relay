//! Real management process, accounts and relay sockets shared by CDP tests.
//! No scenario markers belong in this module.

use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use vnc_relay_adapters::accounts_file::Argon2Hasher;
use vnc_relay_app::ports::PasswordHasher;
use vnc_relay_domain::account::{Argon2Params, Password};

#[path = "dashboard_cdp.rs"]
mod cdp;
#[path = "dashboard_proxy.rs"]
mod proxy;
pub use cdp::Browser;
pub use proxy::{Fault, Proxy};

pub const LIMIT: Duration = Duration::from_secs(20);
pub const PASSWORD: &str = "dashboard-real-password";
static NEXT: AtomicU64 = AtomicU64::new(0);
static HASH: LazyLock<String> = LazyLock::new(|| {
    let password = must(Password::parse(PASSWORD.to_owned()));
    must(Argon2Hasher.hash(
        &password,
        Argon2Params {
            memory_kib: 19_456,
            iterations: 2,
            parallelism: 1,
        },
    ))
    .as_str()
    .to_owned()
});

pub fn must<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("dashboard operation failed: {error:?}"))
}

pub fn wait_for(label: &str, limit: Duration, mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + limit;
    while !predicate() {
        assert!(
            Instant::now() < deadline,
            "dashboard wait timed out: {label}"
        );
        thread::sleep(Duration::from_millis(20));
    }
}

pub fn connect(port: u16) -> TcpStream {
    let stream = must(TcpStream::connect_timeout(
        &SocketAddr::from(([127, 0, 0, 1], port)),
        LIMIT,
    ));
    must(stream.set_read_timeout(Some(LIMIT)));
    must(stream.set_write_timeout(Some(LIMIT)));
    stream
}

pub struct Reply {
    pub status: u16,
    pub headers: String,
    pub body: String,
}

pub fn read_reply(stream: TcpStream) -> Reply {
    let mut reader = BufReader::new(stream);
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        assert!(head.len() < 65_536, "HTTP response header too large");
        let left = (65_536 - head.len()) as u64;
        let count = must((&mut reader).take(left).read_until(b'\n', &mut head));
        assert!(count != 0, "HTTP response has no header boundary");
    }
    let mut headers = must(String::from_utf8(head));
    headers.truncate(headers.len() - 4);
    let status = must(
        headers
            .split_whitespace()
            .nth(1)
            .unwrap_or("missing")
            .parse(),
    );
    let body = if headers.lines().any(|line| {
        line.split_once(':').is_some_and(|(name, value)| {
            name.eq_ignore_ascii_case("transfer-encoding")
                && value.trim().eq_ignore_ascii_case("chunked")
        })
    }) {
        let mut decoded = Vec::new();
        let mut line = Vec::new();
        loop {
            line.clear();
            assert!(
                must((&mut reader).take(65_536).read_until(b'\n', &mut line)) != 0
                    && line.ends_with(b"\r\n"),
                "incomplete HTTP chunk size"
            );
            let size_text = must(std::str::from_utf8(&line));
            let size = must(usize::from_str_radix(
                size_text.trim().split(';').next().unwrap_or(""),
                16,
            ));
            if size == 0 {
                break;
            }
            assert!(
                size <= 8 * 1024 * 1024 - decoded.len(),
                "HTTP body too large"
            );
            let start = decoded.len();
            decoded.resize(start + size, 0);
            must(reader.read_exact(&mut decoded[start..]));
            let mut end = [0; 2];
            must(reader.read_exact(&mut end));
            assert_eq!(&end, b"\r\n", "HTTP chunk terminator");
        }
        must(String::from_utf8(decoded))
    } else {
        let mut body = Vec::new();
        let content_length = headers.lines().find_map(|line| {
            line.split_once(':').and_then(|(name, value)| {
                name.eq_ignore_ascii_case("content-length")
                    .then(|| must(value.trim().parse::<usize>()))
            })
        });
        if let Some(length) = content_length {
            assert!(length <= 8 * 1024 * 1024, "HTTP body too large");
            body.resize(length, 0);
            must(reader.read_exact(&mut body));
        } else {
            must(reader.take(8 * 1024 * 1024).read_to_end(&mut body));
        }
        must(String::from_utf8(body))
    };
    Reply {
        status,
        headers,
        body,
    }
}

pub struct Admin {
    pub cookie: String,
}

pub struct Server {
    dir: PathBuf,
    child: Child,
    pub ports: [u16; 4],
    pub ws_port: u16,
    pub proxy: Proxy,
}

impl Server {
    pub fn start(name: &str, poll_secs: Option<u32>) -> Self {
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "vnc-relay-dashboard-live-{}-{serial}-{name}",
            std::process::id()
        ));
        must(fs::create_dir_all(&dir));
        let hash = &*HASH;
        must(fs::write(
            dir.join("accounts.toml"),
            format!(
                "version = 1\n[[account]]\nname = \"alice\"\npassword_hash = \"{hash}\"\n[[account]]\nname = \"bob\"\npassword_hash = \"{hash}\"\n"
            ),
        ));
        let holders: Vec<_> = (0..5)
            .map(|_| must(TcpListener::bind("127.0.0.1:0")))
            .collect();
        let port_values: Vec<_> = holders
            .iter()
            .map(|listener| must(listener.local_addr()).port())
            .collect();
        let ports = [
            port_values[0],
            port_values[1],
            port_values[2],
            port_values[3],
        ];
        let [management, status, viewer, target] = ports;
        let ws_port = port_values[4];
        let dashboard = poll_secs.map_or_else(String::new, |poll| {
            format!("[dashboard]\npoll_secs = {poll}\nquery_timeout_secs = 5\n")
        });
        let config = format!(
            r#"
[admin]
accounts_file = "accounts.toml"
[management]
listen = "127.0.0.1:{management}"
[management.session]
idle_secs = 300
max_lifetime_secs = 3600
[management.login_limit]
per_account = 2
per_source = 20
window_secs = 900
lockout_secs = 900
[status]
listen = "127.0.0.1:{status}"
[log]
output = "events.jsonl"
[limits]
relay_connections = 16
admin_connections = 16
waiting = 8
[timeouts]
handshake_secs = 30
connect_secs = 30
close_secs = 2
{dashboard}
[relay.tcp_viewer]
enabled = true
listen = "127.0.0.1:{viewer}"
allow = ["127.0.0.1"]
[relay.tcp_server]
enabled = true
listen = "127.0.0.1:{target}"
allow = ["127.0.0.1"]
[relay.ws_viewer]
enabled = true
listen = "127.0.0.1:{ws_port}"
allow = ["127.0.0.1"]
allowed_origins = ["http://127.0.0.1"]
[mode1]
enabled = true
allow_targets = ["127.0.0.1:1-65535"]
[mode2]
enabled = true
"#
        );
        must(fs::write(dir.join("relay.toml"), config));
        drop(holders);
        let proxy = Proxy::start(management);
        let child = must(
            Command::new(env!("CARGO_BIN_EXE_vnc-relay"))
                .args(["serve", "--config"])
                .arg(dir.join("relay.toml"))
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::from(must(fs::File::create(dir.join("stderr.txt")))))
                .spawn(),
        );
        let mut server = Self {
            dir,
            child,
            ports,
            ws_port,
            proxy,
        };
        let deadline = Instant::now() + LIMIT;
        loop {
            if let Some(status) = must(server.child.try_wait()) {
                panic!(
                    "serve exited {status}: {}",
                    fs::read_to_string(server.dir.join("stderr.txt")).unwrap_or_default()
                );
            }
            if let Ok(mut stream) = TcpStream::connect_timeout(
                &SocketAddr::from(([127, 0, 0, 1], status)),
                Duration::from_millis(100),
            ) {
                must(stream.set_read_timeout(Some(LIMIT)));
                must(stream.set_write_timeout(Some(LIMIT)));
                must(stream.write_all(
                    b"GET /readyz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                ));
                if read_reply(stream).status == 200 {
                    break;
                }
            }
            assert!(Instant::now() < deadline, "serve readiness timeout");
            thread::sleep(Duration::from_millis(20));
        }
        server
    }

    pub fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.proxy.port)
    }

    pub fn diagnostics(&self) -> Value {
        let events = fs::read_to_string(self.dir.join("events.jsonl")).unwrap_or_default();
        let mut sanitized = Vec::new();
        for line in events.lines() {
            if let Ok(event) = serde_json::from_str::<Value>(line) {
                let mut record = serde_json::Map::new();
                for key in ["event", "level", "reason", "listener", "stage", "code"] {
                    if let Some(value) = event[key].as_str()
                        && value.len() <= 64
                        && value
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
                    {
                        record.insert(key.to_owned(), json!(value));
                    }
                }
                if !record.is_empty() {
                    sanitized.push(Value::Object(record));
                }
            }
        }
        if sanitized.len() > 80 {
            sanitized.drain(..sanitized.len() - 80);
        }
        let stderr = fs::read_to_string(self.dir.join("stderr.txt")).unwrap_or_default();
        let codes: Vec<_> = stderr
            .lines()
            .filter_map(|line| {
                line.strip_prefix("error[")
                    .and_then(|line| line.split_once(']'))
                    .map(|(code, _)| code)
                    .filter(|code| {
                        code.bytes()
                            .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
                    })
            })
            .collect();
        json!({"proxy":self.proxy.diagnostics(),"backend_events":sanitized,
            "stderr_bytes":stderr.len(),"stderr_error_codes":codes})
    }

    pub fn request(
        &self,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> Reply {
        // Oracle queries bypass fault injection so browser failures cannot be
        // mistaken for real management errors or fabricated successful data.
        let mut stream = connect(self.ports[0]);
        let mut request = format!(
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: {}\r\n",
            body.len()
        );
        for (name, value) in headers {
            request.push_str(&format!("{name}: {value}\r\n"));
        }
        request.push_str("\r\n");
        must(stream.write_all(request.as_bytes()));
        must(stream.write_all(body));
        read_reply(stream)
    }

    pub fn login(&self) -> Admin {
        let body = json!({"account":"bob","password":PASSWORD}).to_string();
        let reply = self.request(
            "POST",
            "/api/v1/session",
            &[("Content-Type", "application/json")],
            body.as_bytes(),
        );
        assert_eq!(reply.status, 200, "oracle login refused");
        let cookie = reply
            .headers
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.eq_ignore_ascii_case("set-cookie"))
            .and_then(|(_, value)| value.trim().split(';').next())
            .unwrap_or_else(|| panic!("oracle login cookie missing"))
            .to_owned();
        let body: Value = must(serde_json::from_str(&reply.body));
        assert!(
            body["csrf_token"]
                .as_str()
                .is_some_and(|token| !token.is_empty()),
            "oracle CSRF missing"
        );
        Admin { cookie }
    }

    pub fn query(&self, admin: &Admin, path: &str) -> Value {
        let reply = self.request("GET", path, &[("Cookie", &admin.cookie)], b"");
        assert_eq!(reply.status, 200, "oracle query {path}: {}", reply.body);
        must(serde_json::from_str(&reply.body))
    }

    pub fn handshaking(&self) -> TcpStream {
        let mut viewer = connect(self.ports[2]);
        let mut greeting = [0; 12];
        must(viewer.read_exact(&mut greeting));
        assert_eq!(&greeting, b"RFB 000.000\n");
        viewer
    }

    pub fn waiting(&self, id: &str) -> TcpStream {
        let mut viewer = self.handshaking();
        must(viewer.write_all(&initial(&format!("ID:{id}"))));
        viewer
    }

    pub fn pair(&self, admin: &Admin, mode: &str, id: &str) -> Pair {
        let old = self.query(admin, "/api/v1/sessions");
        let mut viewer = self.handshaking();
        let target = match mode {
            "II" => {
                let mut target = connect(self.ports[3]);
                must(target.write_all(&initial(&format!("ID:{id}"))));
                wait_for("Mode II target waiting", LIMIT, || {
                    self.query(admin, "/api/v1/waiting")["items"]
                        .as_array()
                        .is_some_and(|items| items.iter().any(|item| item["relay_id"] == id))
                });
                must(viewer.write_all(&initial(&format!("ID:{id}"))));
                target
            }
            "I" => {
                let listener = must(TcpListener::bind("127.0.0.1:0"));
                must(listener.set_nonblocking(true));
                must(viewer.write_all(&initial(&must(listener.local_addr()).to_string())));
                let deadline = Instant::now() + LIMIT;
                loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                            assert!(Instant::now() < deadline, "Mode I target accept timeout");
                            thread::sleep(Duration::from_millis(20));
                        }
                        Err(error) => panic!("Mode I target accept: {error}"),
                    }
                }
            }
            _ => panic!("invalid fixture mode {mode}"),
        };
        must(target.set_nonblocking(false));
        must(target.set_read_timeout(Some(LIMIT)));
        must(target.set_write_timeout(Some(LIMIT)));
        let mut sid = None;
        wait_for("new active session", LIMIT, || {
            let current = self.query(admin, "/api/v1/sessions");
            sid = current["items"]
                .as_array()
                .and_then(|items| {
                    items.iter().find(|item| {
                        !old["items"].as_array().is_some_and(|previous| {
                            previous
                                .iter()
                                .any(|old_item| old_item["session_id"] == item["session_id"])
                        })
                    })
                })
                .and_then(|item| item["session_id"].as_str())
                .map(str::to_owned);
            sid.is_some()
        });
        Pair {
            viewer,
            target,
            sid: sid.unwrap_or_else(|| panic!("new session id missing")),
        }
    }
}

pub fn initial(request: &str) -> [u8; 250] {
    assert!(request.len() < 250, "test handshake fits wire format");
    let mut bytes = [0; 250];
    bytes[..request.len()].copy_from_slice(request.as_bytes());
    bytes
}

pub struct Pair {
    pub sid: String,
    pub viewer: TcpStream,
    pub target: TcpStream,
}

impl Pair {
    pub fn transfer(&mut self, viewer_bytes: usize, target_bytes: usize) {
        fn direction(source: &mut TcpStream, destination: &mut TcpStream, length: usize, byte: u8) {
            let buffer = [byte; 8192];
            let mut read = [0; 8192];
            let mut remaining = length;
            while remaining > 0 {
                let length = remaining.min(buffer.len());
                must(source.write_all(&buffer[..length]));
                must(destination.read_exact(&mut read[..length]));
                assert_eq!(&read[..length], &buffer[..length]);
                remaining -= length;
            }
        }
        direction(&mut self.viewer, &mut self.target, viewer_bytes, 0x51);
        direction(&mut self.target, &mut self.viewer, target_bytes, 0x72);
    }

    pub fn close(mut self) {
        must(self.viewer.shutdown(Shutdown::Write));
        must(self.target.shutdown(Shutdown::Write));
        let mut bytes = Vec::new();
        must((&mut self.viewer).take(1024).read_to_end(&mut bytes));
        must((&mut self.target).take(1024).read_to_end(&mut bytes));
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.proxy.shutdown();
        let _ = self.child.kill();
        let deadline = Instant::now() + LIMIT;
        while matches!(self.child.try_wait(), Ok(None)) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(20));
        }
        let _ = fs::remove_dir_all(&self.dir);
    }
}
