//! System tests of `vnc-relay serve`: the start order, exit codes,
//! diagnostic lines, and operation log lines of docs/contracts/cli-v1.md
//! §1.5, §1.8, §2 and docs/contracts/ops-info-v1.md §1.1, §1.5, §1.6, the
//! source IP check of the TCP entry points (relay spec §3.1 step 1), and
//! the relay connection limit L and the initial exchange time H.

use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::version::{TLS12, TLS13};
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};

const BIN: &str = env!("CARGO_BIN_EXE_vnc-relay");

/// 16 and 32 zero bytes in unpadded Base64. Test data, not a real hash.
const SALT: &str = "AAAAAAAAAAAAAAAAAAAAAA";
const HASH: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

const EXIT_LIMIT: Duration = Duration::from_secs(20);

/// Requests that carry no login. Each one gets `401 unauthenticated` with no data.
const LOGIN_LESS_REQUESTS: [(&str, &str); 7] = [
    ("GET", "/api/v1/session"),
    ("GET", "/api/v1/summary"),
    ("GET", "/api/v1/waiting"),
    ("GET", "/api/v1/sessions"),
    ("GET", "/api/v1/sessions/ses_3k9d2m7q4x8v1b6n5c0z2h4w7y"),
    ("GET", "/api/v1/closed-sessions"),
    (
        "POST",
        "/api/v1/sessions/ses_3k9d2m7q4x8v1b6n5c0z2h4w7y/terminate",
    ),
];

/// Tests pick free ports by binding port 0 and releasing it. Two tests that
/// run at the same time can get the same port, so tests that bind take this
/// lock first.
static PORTS_LOCK: Mutex<()> = Mutex::new(());

fn lock_ports() -> MutexGuard<'static, ()> {
    PORTS_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Listener ports of one test config, in bind order.
#[derive(Clone, Copy)]
struct Ports {
    status: u16,
    management: u16,
    tcp_viewer: u16,
    tcp_server: u16,
}

impl Ports {
    fn free() -> Self {
        Self::free_on("127.0.0.1:0")
    }

    /// Ports that `probe` (an address with port 0) can bind. A listener on
    /// `[::]` also needs the port free on every 127.0.0.x, where earlier
    /// tests leave client sockets, so such a test probes `[::]:0`.
    fn free_on(probe: &str) -> Self {
        // Hold all four until every port is chosen so that they differ.
        let holders: Vec<TcpListener> = (0..4)
            .map(|_| TcpListener::bind(probe).unwrap_or_else(|e| panic!("free port: {e}")))
            .collect();
        let port = |i: usize| {
            holders[i]
                .local_addr()
                .unwrap_or_else(|e| panic!("local addr: {e}"))
                .port()
        };
        Self {
            status: port(0),
            management: port(1),
            tcp_viewer: port(2),
            tcp_server: port(3),
        }
    }

    fn all(self) -> [u16; 4] {
        [
            self.status,
            self.management,
            self.tcp_viewer,
            self.tcp_server,
        ]
    }
}

struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Run {
    /// `(code, key)` of every `error[..]` line; an absent key is empty.
    fn errors(&self) -> Vec<(String, String)> {
        self.stderr
            .lines()
            .filter_map(|line| line.strip_prefix("error["))
            .map(|rest| {
                let (code, rest) = rest.split_once(']').unwrap_or((rest, ""));
                let key = rest
                    .strip_prefix(" key=")
                    .and_then(|rest| rest.split_once(':'))
                    .map_or("", |(key, _)| key);
                (code.to_owned(), key.to_owned())
            })
            .collect()
    }

    fn assert_no_secret(&self) {
        for text in [&self.stdout, &self.stderr] {
            assert!(
                !text.contains(HASH) && !text.contains(SALT),
                "secret in output: {text}"
            );
        }
    }
}

fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
    items
        .iter()
        .map(|(code, key)| ((*code).to_owned(), (*key).to_owned()))
        .collect()
}

/// The string value of `"name":"value"` in one log line. Values in these
/// tests hold no quote, so a plain search is enough. The ops-info contract
/// tests parse lines as JSON.
fn field<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let start = line.find(&format!("\"{name}\":\""))? + name.len() + 4;
    let rest = &line[start..];
    rest.find('"').map(|end| &rest[..end])
}

/// The `event` of each log line, in order.
fn events(log: &str) -> Vec<&str> {
    log.lines()
        .map(|line| field(line, "event").unwrap_or_else(|| panic!("no event: {line}")))
        .collect()
}

/// `events` without connection events. `wait_listening` connects to the
/// TCP entry points from 127.0.0.1, so a running server logs those probes.
/// Only the Unix signal tests stop a running server and read its whole log.
#[cfg(unix)]
fn lifecycle_events(log: &str) -> Vec<&str> {
    events(log)
        .into_iter()
        .filter(|event| !event.starts_with("connection_"))
        .collect()
}

/// `(reason, config_key)` of each `startup_failed` line; an absent key is
/// empty.
fn startup_failures(log: &str) -> Vec<(String, String)> {
    log.lines()
        .filter(|line| field(line, "event") == Some("startup_failed"))
        .map(|line| {
            let reason = field(line, "reason").unwrap_or_else(|| panic!("no reason: {line}"));
            let key = field(line, "config_key").unwrap_or("");
            (reason.to_owned(), key.to_owned())
        })
        .collect()
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vnc-relay-serve-{}-{name}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir)
            .unwrap_or_else(|error| panic!("clear {}: {error}", dir.display()));
    }
    std::fs::create_dir_all(&dir)
        .unwrap_or_else(|error| panic!("create {}: {error}", dir.display()));
    dir
}

fn write(path: &Path, text: &str) {
    std::fs::write(path, text).unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .unwrap_or_else(|error| panic!("chmod {}: {error}", path.display()));
    }
}

fn config_text(ports: Ports) -> String {
    format!(
        r#"
[admin]
accounts_file = "admin-accounts.toml"

[management]
listen = "127.0.0.1:{}"

[status]
listen = "127.0.0.1:{}"

[relay.tcp_viewer]
enabled = true
listen = "127.0.0.1:{}"
allow = ["127.0.0.1"]

[relay.tcp_server]
enabled = true
listen = "127.0.0.1:{}"
allow = ["127.0.0.1"]
"#,
        ports.management, ports.status, ports.tcp_viewer, ports.tcp_server
    )
}

fn accounts_text(params: &str) -> String {
    format!(
        "version = 1\n\n[[account]]\nname = \"alice\"\npassword_hash = \"$argon2id$v=19${params}${SALT}${HASH}\"\n"
    )
}

/// Writes a valid config and account file into a new directory and returns
/// the config path.
fn valid_setup(name: &str, ports: Ports) -> PathBuf {
    setup_with(name, &config_text(ports))
}

/// Writes `config` and a valid account file into a new directory and returns
/// the config path.
fn setup_with(name: &str, config: &str) -> PathBuf {
    let dir = temp_dir(name);
    write(
        &dir.join("admin-accounts.toml"),
        &accounts_text("m=19456,t=2,p=1"),
    );
    let config_path = dir.join("relay.toml");
    write(&config_path, config);
    config_path
}

fn spawn(config: &Path) -> Child {
    Command::new(BIN)
        .arg("serve")
        .arg("--config")
        .arg(config)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("spawn {BIN}: {error}"))
}

fn wait_exit(child: &mut Child) -> ExitStatus {
    let deadline = Instant::now() + EXIT_LIMIT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            Ok(None) => {
                let _ = child.kill();
                panic!("vnc-relay did not exit in {EXIT_LIMIT:?}");
            }
            Err(error) => panic!("wait: {error}"),
        }
    }
}

/// Waits for exit and collects the output. stdout and stderr stay small,
/// so the child never blocks on a full pipe.
fn finish(mut child: Child) -> Run {
    let status = wait_exit(&mut child);
    Run {
        code: status.code(),
        stdout: read_all(child.stdout.as_mut()),
        stderr: read_all(child.stderr.as_mut()),
    }
}

fn read_all(stream: Option<&mut impl Read>) -> String {
    let mut text = String::new();
    if let Some(stream) = stream {
        stream
            .read_to_string(&mut text)
            .unwrap_or_else(|error| panic!("read output: {error}"));
    }
    text
}

fn serve(config: &Path) -> Run {
    finish(spawn(config))
}

fn connects(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok()
}

/// Every port can be bound again: the process released its listeners.
fn assert_released(ports: Ports) {
    for port in ports.all() {
        if let Err(error) = TcpListener::bind(("127.0.0.1", port)) {
            panic!("port {port} not released: {error}");
        }
    }
}

/// Waits until every port accepts a TCP connection. Fails early with the
/// output when the child exits.
fn wait_listening(child: &mut Child, ports: Ports) {
    let deadline = Instant::now() + EXIT_LIMIT;
    while !ports.all().into_iter().all(connects) {
        if let Ok(Some(status)) = child.try_wait() {
            let mut stderr = String::new();
            if let Some(stream) = child.stderr.as_mut() {
                let _ = stream.read_to_string(&mut stderr);
            }
            panic!("vnc-relay exited early ({status}): {stderr}");
        }
        assert!(
            Instant::now() < deadline,
            "listeners not ready in {EXIT_LIMIT:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Kills the child if a test fails before it stops the child itself.
struct Guard(Option<Child>);

impl Guard {
    fn child(&mut self) -> &mut Child {
        self.0
            .as_mut()
            .unwrap_or_else(|| panic!("child already taken"))
    }

    /// Hands the child over for a signal test (Unix only).
    #[cfg(unix)]
    fn take(&mut self) -> Child {
        self.0
            .take()
            .unwrap_or_else(|| panic!("child already taken"))
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[cfg(unix)]
fn send_signal(child: &Child, name: &str) {
    let status = Command::new("kill")
        .arg(format!("-{name}"))
        .arg(child.id().to_string())
        .status()
        .unwrap_or_else(|error| panic!("kill -{name}: {error}"));
    assert!(status.success(), "kill -{name} failed: {status}");
}

/// spec: operations.md:OPS-001
/// spec: operations.md:OPS-012 partial
#[test]
fn rejects_invalid_config_with_exit_code_3_and_every_cause() {
    let dir = temp_dir("bad-config");
    let config = dir.join("relay.toml");
    write(
        &config,
        r#"
[admin]

[management]
listen = "127.0.0.1:99999"

[status]
listen = "127.0.0.1:18081"

[relay.tcp_viewer]
enabled = true
listen = "127.0.0.1:18082"
allow = ["198.51.100.300"]

[limits]
admin_connections = 0

[timeouts]
handshake_secs = 121
"#,
    );
    let run = serve(&config);
    assert_eq!(run.code, Some(3), "{}", run.stderr);
    let mut errors = run.errors();
    errors.sort();
    assert_eq!(
        errors,
        pairs(&[
            ("config_ip_item_invalid", "relay.tcp_viewer.allow[0]"),
            ("config_listen_invalid", "management.listen"),
            ("config_missing_key", "admin.accounts_file"),
            ("config_value_out_of_range", "limits.admin_connections"),
            ("config_value_out_of_range", "timeouts.handshake_secs"),
        ])
    );
    // The log output is not known yet, so stdout gets one line per
    // diagnostic. The lines name keys, never the rejected values.
    assert!(
        events(&run.stdout)
            .iter()
            .all(|event| *event == "startup_failed"),
        "{}",
        run.stdout
    );
    let mut logged = startup_failures(&run.stdout);
    logged.sort();
    assert_eq!(logged, errors);
    for value in ["198.51.100.300", "99999", "18082"] {
        assert!(!run.stdout.contains(value), "{value} in {}", run.stdout);
    }
}

/// spec: operations.md:OPS-001
#[test]
fn rejects_toml_syntax_error_and_missing_config_file_with_exit_code_3() {
    let dir = temp_dir("syntax");
    let config = dir.join("relay.toml");
    write(&config, "[admin\naccounts_file = \"a.toml\"\n");
    let run = serve(&config);
    assert_eq!(run.code, Some(3), "{}", run.stderr);
    assert_eq!(events(&run.stdout), ["startup_failed"]);
    assert_eq!(
        startup_failures(&run.stdout),
        pairs(&[("config_syntax_error", "")])
    );
    assert_eq!(run.errors(), pairs(&[("config_syntax_error", "")]));
    assert!(
        run.stderr.starts_with("error[config_syntax_error]: "),
        "{}",
        run.stderr
    );

    let run = serve(&dir.join("missing.toml"));
    assert_eq!(run.code, Some(3), "{}", run.stderr);
    assert_eq!(run.errors(), pairs(&[("config_file_unreadable", "")]));
    assert_eq!(
        startup_failures(&run.stdout),
        pairs(&[("config_file_unreadable", "")])
    );
}

/// spec: operations.md:OPS-001
/// spec: operations.md:OPS-012 partial
#[test]
fn refuses_to_start_without_a_usable_account_with_exit_code_5() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = valid_setup("accounts", ports);
    let accounts = config.with_file_name("admin-accounts.toml");
    // (account file text or none, expected code, expected key)
    let cases = [
        (None, "accounts_file_missing", "admin.accounts_file"),
        (
            Some("version = 1\n".to_owned()),
            "accounts_none",
            "admin.accounts_file",
        ),
        (
            Some(accounts_text("m=19456,t=2,p=1").replace("argon2id", "argon2i")),
            "accounts_file_invalid",
            "account[0].password_hash",
        ),
        (
            Some(accounts_text("m=4096,t=2,p=1")),
            "accounts_hash_below_minimum",
            "account[0].password_hash",
        ),
    ];
    for (text, code, key) in cases {
        match &text {
            Some(text) => write(&accounts, text),
            None => {
                let _ = std::fs::remove_file(&accounts);
            }
        }
        let run = serve(&config);
        assert_eq!(run.code, Some(5), "{}", run.stderr);
        // The log names the account file key; keys inside the file stay on
        // stderr.
        assert_eq!(events(&run.stdout), ["server_starting", "startup_failed"]);
        assert_eq!(
            startup_failures(&run.stdout),
            pairs(&[(code, "admin.accounts_file")])
        );
        assert_eq!(run.errors(), pairs(&[(code, key)]));
        run.assert_no_secret();
        // The account stage comes before any listener.
        assert_released(ports);
    }
}

/// Plaintext management on a public address stops at the config check (exit
/// code 3), before any bind.
fn refuses_plaintext_management_on_a_public_address_with_exit_code_3() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let public = with_public_management(ports, &config_text(ports));
    let config = setup_with("plaintext-public", &public);
    let run = serve(&config);
    assert_eq!(run.code, Some(3), "{}", run.stderr);
    let expected = pairs(&[(
        "config_plaintext_management_not_loopback",
        "management.listen",
    )]);
    assert_eq!(run.errors(), expected);
    assert_eq!(startup_failures(&run.stdout), expected);
    assert_eq!(events(&run.stdout), ["startup_failed"]);
    // One stderr line per error, in the `error[code] key=key: message` form.
    let prefix = "error[config_plaintext_management_not_loopback] key=management.listen: ";
    assert!(run.stderr.starts_with(prefix), "{}", run.stderr);
    assert_eq!(run.stderr.lines().count(), 1, "{}", run.stderr);
    // Config checks run before any bind, so every port is still free.
    assert_released(ports);
}

/// spec: operations.md:OPS-034
#[test]
fn refuses_public_plaintext_and_answers_login_less_queries_with_401_on_loopback() {
    // OPS-034 has two conditions; this test runs both.
    refuses_plaintext_management_on_a_public_address_with_exit_code_3();
    answers_api_queries_with_401_without_login_on_loopback_plaintext();
}

/// spec: operations.md:OPS-014
#[test]
fn public_management_never_starts_on_plaintext_or_without_tls_or_an_account() {
    // OPS-014 has four conditions, and this test runs all of them. A plaintext
    // config on a public address does not start. A missing certificate or
    // account does not start it either. The running public HTTPS port gives no
    // HTTP reply to plaintext and answers login-less requests with 401.
    refuses_plaintext_management_on_a_public_address_with_exit_code_3();
    refuses_public_management_without_its_certificate_with_exit_code_3();
    refuses_public_management_without_an_account_with_exit_code_5();
    public_https_refuses_plaintext_and_answers_login_less_requests_with_401();
}

/// Public management with a built-in certificate and an account starts on
/// `0.0.0.0`. A plain HTTP request to its port gets no HTTP reply, and every
/// login-less request gets `401 unauthenticated` with no data over HTTPS.
fn public_https_refuses_plaintext_and_answers_login_less_requests_with_401() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let public = with_public_management(
        ports,
        &management_config(ports, "server-chain.pem", "server-key-p256.pem"),
    );
    let config = setup_with("public-https", &public);
    let mut guard = Guard(Some(spawn(&config)));
    // A listener bound on 0.0.0.0 also answers on 127.0.0.1, so every port connects.
    wait_listening(guard.child(), ports);
    plaintext_request_gets_no_http_reply(ports.management);
    for (method, path) in LOGIN_LESS_REQUESTS {
        let (status, body) = https_request(ports.management, method, path);
        assert_eq!(status, 401, "{method} {path}: {body}");
        assert!(
            body.contains(r#""code":"unauthenticated""#),
            "{method} {path}: {body}"
        );
    }
}

/// `method path` over TLS to the loopback `port`. The client trusts only the
/// test root CA. Returns the status code and the body.
fn https_request(port: u16, method: &str, path: &str) -> (u16, String) {
    let mut roots = RootCertStore::empty();
    let certs = CertificateDer::pem_file_iter(tls_file("root-ca.pem"))
        .unwrap_or_else(|error| panic!("root CA opens: {error}"));
    for cert in certs {
        let cert = cert.unwrap_or_else(|error| panic!("root CA is PEM: {error}"));
        roots
            .add(cert)
            .unwrap_or_else(|error| panic!("root CA is a certificate: {error}"));
    }
    let config = ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_protocol_versions(&[&TLS13, &TLS12])
    .unwrap_or_else(|error| panic!("client protocol versions: {error}"))
    .with_root_certificates(roots)
    .with_no_client_auth();
    let name = ServerName::try_from("localhost")
        .unwrap_or_else(|error| panic!("server name is valid: {error}"));
    let connection = ClientConnection::new(Arc::new(config), name)
        .unwrap_or_else(|error| panic!("client connection: {error}"));
    let tcp = TcpStream::connect(("127.0.0.1", port))
        .unwrap_or_else(|error| panic!("connect {port}: {error}"));
    tcp.set_read_timeout(Some(EXIT_LIMIT))
        .unwrap_or_else(|error| panic!("read timeout: {error}"));
    let mut stream = StreamOwned::new(connection, tcp);
    let request =
        format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .unwrap_or_else(|error| panic!("request is sent: {error}"));
    stream
        .flush()
        .unwrap_or_else(|error| panic!("request is flushed: {error}"));
    let mut raw = Vec::new();
    // A TLS peer may close without close_notify. The bytes read so far are the reply.
    let _ = stream.read_to_end(&mut raw);
    let raw = String::from_utf8_lossy(&raw).into_owned();
    let (head, body) = raw
        .split_once("\r\n\r\n")
        .unwrap_or_else(|| panic!("no header end: {raw:?}"));
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status line: {raw:?}"));
    (status, body.to_owned())
}

/// Plain HTTP sent to the TLS port. The server must not answer it as HTTP. It
/// may send a TLS alert or nothing, and it may close the connection.
fn plaintext_request_gets_no_http_reply(port: u16) {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .unwrap_or_else(|error| panic!("connect {port}: {error}"));
    stream
        .set_read_timeout(Some(EXIT_LIMIT))
        .unwrap_or_else(|error| panic!("read timeout: {error}"));
    // The server may close before the request is written. That is a refusal too.
    let _ = stream.write_all(b"GET /api/v1/summary HTTP/1.1\r\nHost: x\r\n\r\n");
    let mut reply = Vec::new();
    // A reset or the read timeout ends the reply. The bytes read so far are kept.
    let _ = stream.read_to_end(&mut reply);
    assert!(
        !String::from_utf8_lossy(&reply).contains("HTTP/"),
        "plaintext got an HTTP reply: {reply:?}"
    );
}

/// Public management with the built-in certificate file missing: exit code 3
/// before any bind.
fn refuses_public_management_without_its_certificate_with_exit_code_3() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let public = with_public_management(
        ports,
        &management_config(ports, "no-such-file.pem", "server-key-p256.pem"),
    );
    let config = setup_with("public-cert-missing", &public);
    let run = serve(&config);
    assert_eq!(run.code, Some(3), "{}", run.stderr);
    assert_eq!(events(&run.stdout), ["server_starting", "startup_failed"]);
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(field(lines[1], "reason"), Some("tls_file_invalid"));
    assert_eq!(field(lines[1], "listener"), Some("management"));
    assert_eq!(field(lines[1], "config_key"), Some("management.cert_file"));
    assert_eq!(
        run.stderr,
        "error[tls_file_invalid] key=management.cert_file: 인증서 파일을 읽을 수 없다.\n"
    );
    // Start-up stops before any listener binds, so every port is still free.
    assert_released(ports);
}

/// Public management with valid TLS and no account file: exit code 5 before
/// any bind.
fn refuses_public_management_without_an_account_with_exit_code_5() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let public = with_public_management(
        ports,
        &management_config(ports, "server-chain.pem", "server-key-p256.pem"),
    );
    let config = setup_with("public-no-account", &public);
    let _ = std::fs::remove_file(config.with_file_name("admin-accounts.toml"));
    let run = serve(&config);
    assert_eq!(run.code, Some(5), "{}", run.stderr);
    assert_eq!(events(&run.stdout), ["server_starting", "startup_failed"]);
    let expected = pairs(&[("accounts_file_missing", "admin.accounts_file")]);
    assert_eq!(startup_failures(&run.stdout), expected);
    assert_eq!(run.errors(), expected);
    run.assert_no_secret();
    // The account stage comes before any listener.
    assert_released(ports);
}

/// spec: operations.md:OPS-002
#[test]
fn fails_before_ready_and_frees_every_port_when_a_port_is_taken_or_tls_fails() {
    // OPS-002 has two conditions; this test runs both.
    fails_with_exit_code_4_and_releases_listeners_when_a_port_is_taken();
    checks_tls_files_before_binding_a_taken_status_port();
}

fn fails_with_exit_code_4_and_releases_listeners_when_a_port_is_taken() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = valid_setup("port-taken", ports);
    let taken = TcpListener::bind(("127.0.0.1", ports.tcp_viewer))
        .unwrap_or_else(|error| panic!("take port: {error}"));
    let run = serve(&config);
    assert_eq!(run.code, Some(4), "{}", run.stderr);
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(events(&run.stdout), ["server_starting", "startup_failed"]);
    assert_eq!(field(lines[1], "reason"), Some("listener_bind_failed"));
    assert_eq!(field(lines[1], "listener"), Some("tcp_viewer"));
    assert_eq!(
        field(lines[1], "config_key"),
        Some("relay.tcp_viewer.listen")
    );
    assert_eq!(field(lines[1], "detail"), Some("이미 사용 중이다."));
    assert!(!run.stdout.contains("127.0.0.1"), "{}", run.stdout);
    assert_eq!(
        run.stderr,
        format!(
            "error[listener_bind_failed] key=relay.tcp_viewer.listen: 127.0.0.1:{}에 바인드할 수 없다. 이미 사용 중이다.\n",
            ports.tcp_viewer
        )
    );
    drop(taken);
    // Status and management were bound before the failure and are free again.
    assert_released(ports);
}

/// Path of a fixture in `crates/adapters/tests/fixtures/tls`, for a TOML
/// literal string (no escapes, so Windows separators are safe).
fn tls_file(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("crates/adapters/tests/fixtures/tls")
        .join(name)
        .display()
        .to_string()
}

/// Valid config with an enabled WSS entry point that uses `cert` and `key`.
fn wss_config(ports: Ports, cert: &str, key: &str) -> String {
    let wss_port = TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .unwrap_or_else(|error| panic!("free port: {error}"))
        .port();
    format!(
        "{}\n[relay.wss_viewer]\nenabled = true\nlisten = '127.0.0.1:{wss_port}'\nallow = ['127.0.0.1']\ncert_file = '{}'\nkey_file = '{}'\n",
        config_text(ports),
        tls_file(cert),
        tls_file(key)
    )
}

/// Valid config whose management listener uses `cert` and `key`.
fn management_config(ports: Ports, cert: &str, key: &str) -> String {
    config_text(ports).replacen(
        "[management]\n",
        &format!(
            "[management]\ncert_file = '{}'\nkey_file = '{}'\n",
            tls_file(cert),
            tls_file(key)
        ),
        1,
    )
}

/// `config` with the management listener on the public address `0.0.0.0`.
/// Only tests whose start-up stops before any bind use it.
fn with_public_management(ports: Ports, config: &str) -> String {
    config.replace(
        &format!("listen = \"127.0.0.1:{}\"", ports.management),
        &format!("listen = \"0.0.0.0:{}\"", ports.management),
    )
}

/// spec: operations.md:OPS-002 partial
#[test]
fn fails_with_exit_code_3_when_wss_key_does_not_match() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = setup_with(
        "wss-key-mismatch",
        &wss_config(ports, "server-chain.pem", "other-key-p256.pem"),
    );
    let run = serve(&config);
    assert_eq!(run.code, Some(3), "{}", run.stderr);
    assert_eq!(events(&run.stdout), ["server_starting", "startup_failed"]);
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(field(lines[1], "reason"), Some("tls_file_invalid"));
    assert_eq!(field(lines[1], "listener"), Some("wss_viewer"));
    assert_eq!(
        field(lines[1], "config_key"),
        Some("relay.wss_viewer.key_file")
    );
    assert_eq!(field(lines[1], "detail"), None);
    assert_eq!(
        run.stderr,
        "error[tls_file_invalid] key=relay.wss_viewer.key_file: 개인키가 인증서와 맞지 않다.\n"
    );
    assert_released(ports);
}

/// spec: operations.md:OPS-002 partial
#[test]
fn fails_with_exit_code_3_when_management_certificate_is_missing() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = setup_with(
        "management-cert-missing",
        &management_config(ports, "no-such-file.pem", "server-key-p256.pem"),
    );
    let run = serve(&config);
    assert_eq!(run.code, Some(3), "{}", run.stderr);
    assert_eq!(events(&run.stdout), ["server_starting", "startup_failed"]);
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(field(lines[1], "reason"), Some("tls_file_invalid"));
    assert_eq!(field(lines[1], "listener"), Some("management"));
    assert_eq!(field(lines[1], "config_key"), Some("management.cert_file"));
    assert_eq!(
        run.stderr,
        "error[tls_file_invalid] key=management.cert_file: 인증서 파일을 읽을 수 없다.\n"
    );
    assert_released(ports);
}

fn checks_tls_files_before_binding_a_taken_status_port() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = setup_with(
        "tls-before-bind",
        &wss_config(ports, "server-chain.pem", "other-key-p256.pem"),
    );
    // Status is the first listener bound. If the TLS check ran after the
    // binds, this held port would fail with exit code 4 instead.
    let taken = TcpListener::bind(("127.0.0.1", ports.status))
        .unwrap_or_else(|error| panic!("hold status port: {error}"));
    let run = serve(&config);
    drop(taken);
    assert_eq!(run.code, Some(3), "{}", run.stderr);
    assert_eq!(events(&run.stdout), ["server_starting", "startup_failed"]);
    assert_eq!(
        run.stderr,
        "error[tls_file_invalid] key=relay.wss_viewer.key_file: 개인키가 인증서와 맞지 않다.\n"
    );
    assert_released(ports);
}

/// spec: operations.md:OPS-039 partial
#[test]
fn second_instance_fails_with_exit_code_4_and_leaves_the_first_running() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = valid_setup("second", ports);
    let mut first = Guard(Some(spawn(&config)));
    wait_listening(first.child(), ports);

    let second = serve(&config);
    assert_eq!(second.code, Some(4), "{}", second.stderr);
    assert_eq!(
        events(&second.stdout),
        [
            "server_starting",
            "startup_failed",
            "startup_failed",
            "startup_failed",
            "startup_failed"
        ]
    );
    let listeners: Vec<&str> = second
        .stdout
        .lines()
        .filter_map(|line| field(line, "listener"))
        .collect();
    assert_eq!(
        listeners,
        ["status", "management", "tcp_viewer", "tcp_server"]
    );
    assert_eq!(
        second.errors(),
        pairs(&[
            ("listener_bind_failed", "status.listen"),
            ("listener_bind_failed", "management.listen"),
            ("listener_bind_failed", "relay.tcp_viewer.listen"),
            ("listener_bind_failed", "relay.tcp_server.listen"),
        ])
    );

    assert!(
        matches!(first.child().try_wait(), Ok(None)),
        "first instance stopped"
    );
    assert!(ports.all().into_iter().all(connects));

    #[cfg(unix)]
    {
        let child = first.take();
        send_signal(&child, "TERM");
        let run = finish(child);
        assert_eq!(run.code, Some(0), "{}", run.stderr);
        assert_eq!(
            lifecycle_events(&run.stdout),
            ["server_starting", "server_ready", "server_stopped"]
        );
        assert_released(ports);
    }
}

#[cfg(unix)]
#[test]
fn stops_with_exit_code_0_on_interrupt_and_terminate() {
    let _lock = lock_ports();
    for signal in ["INT", "TERM"] {
        let ports = Ports::free();
        let config = valid_setup(&format!("signal-{signal}"), ports);
        let mut guard = Guard(Some(spawn(&config)));
        wait_listening(guard.child(), ports);
        let child = guard.take();
        send_signal(&child, signal);
        let run = finish(child);
        assert_eq!(run.code, Some(0), "SIG{signal}: {}", run.stderr);
        assert_eq!(
            lifecycle_events(&run.stdout),
            ["server_starting", "server_ready", "server_stopped"]
        );
        assert_eq!(run.stderr, "");
        assert_released(ports);
    }
}

#[cfg(unix)]
#[test]
fn warns_about_a_permissive_account_file_and_still_starts() {
    use std::os::unix::fs::PermissionsExt;

    let _lock = lock_ports();
    let ports = Ports::free();
    let config = valid_setup("permissive", ports);
    std::fs::set_permissions(
        config.with_file_name("admin-accounts.toml"),
        std::fs::Permissions::from_mode(0o644),
    )
    .unwrap_or_else(|error| panic!("chmod: {error}"));
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);
    let child = guard.take();
    send_signal(&child, "TERM");
    let run = finish(child);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(
        run.stderr
            .starts_with("warning[accounts_file_permissive] key=admin.accounts_file: "),
        "{}",
        run.stderr
    );
    assert_eq!(run.stderr.lines().count(), 1, "{}", run.stderr);
}

#[cfg(unix)]
#[test]
fn writes_the_log_to_the_configured_file_from_start_to_stop() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = valid_setup("log-file", ports);
    let text = config_text(ports).replacen("allow = [\"127.0.0.1\"]", "allow = [\"0.0.0.0/0\"]", 1)
        + "\n[log]\noutput = \"relay.log\"\n";
    write(&config, &text);
    let log_path = config.with_file_name("relay.log");
    std::fs::write(&log_path, "earlier line\n")
        .unwrap_or_else(|error| panic!("write log: {error}"));

    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);
    let child = guard.take();
    send_signal(&child, "TERM");
    let run = finish(child);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(run.stdout, "");
    assert_eq!(run.stderr, "");

    let log =
        std::fs::read_to_string(&log_path).unwrap_or_else(|error| panic!("read log: {error}"));
    let log = log
        .strip_prefix("earlier line\n")
        .unwrap_or_else(|| panic!("log was not appended: {log}"));
    assert_eq!(
        lifecycle_events(log),
        [
            "server_starting",
            "allow_all_source",
            "server_ready",
            "server_stopped"
        ]
    );
    let lines: Vec<&str> = log.lines().collect();
    assert_eq!(field(lines[0], "level"), Some("info"));
    assert_eq!(field(lines[0], "log_contract"), Some("ops-info/1.2.0"));
    assert_eq!(field(lines[1], "level"), Some("warn"));
    assert_eq!(field(lines[1], "listener"), Some("tcp_viewer"));
    assert_eq!(field(lines[1], "item"), Some("0.0.0.0/0"));
    assert_eq!(field(lines[1], "phase"), Some("startup"));
}

#[test]
fn fails_with_exit_code_3_when_the_log_file_cannot_be_opened() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = valid_setup("log-unavailable", ports);
    let text = config_text(ports) + "\n[log]\noutput = \"no-such-dir/relay.log\"\n";
    write(&config, &text);
    let run = serve(&config);
    assert_eq!(run.code, Some(3), "{}", run.stderr);
    assert_eq!(
        run.errors(),
        pairs(&[("log_file_unavailable", "log.output")])
    );
    // The log file is not open, so the line goes to stdout without the path.
    assert_eq!(events(&run.stdout), ["startup_failed"]);
    assert_eq!(
        startup_failures(&run.stdout),
        pairs(&[("log_file_unavailable", "log.output")])
    );
    assert_eq!(field(&run.stdout, "detail"), Some("상위 디렉터리가 없다."));
    assert!(!run.stdout.contains("no-such-dir"), "{}", run.stdout);
    assert_released(ports);
}

#[cfg(target_os = "linux")]
#[test]
fn warns_once_when_log_writes_fail_and_keeps_running() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = valid_setup("log-full", ports);
    write(
        &config,
        &(config_text(ports) + "\n[log]\noutput = \"/dev/full\"\n"),
    );
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);
    let child = guard.take();
    send_signal(&child, "TERM");
    let run = finish(child);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(run.stdout, "");
    assert!(
        run.stderr
            .starts_with("warning[log_write_failed] key=log.output: "),
        "{}",
        run.stderr
    );
    // server_starting and server_stopped both fail; only the first is told.
    assert_eq!(run.stderr.lines().count(), 1, "{}", run.stderr);
}

/// `config_text` with other allow lists for `tcp_viewer` and `tcp_server`
/// (TOML arrays) and the log in `relay.log`.
fn entry_config(ports: Ports, viewer_allow: &str, server_allow: &str) -> String {
    let default = "allow = [\"127.0.0.1\"]";
    let text = config_text(ports);
    let (viewer, server) = text
        .split_once(default)
        .unwrap_or_else(|| panic!("no allow line: {text}"));
    let server = server.replacen(default, &format!("allow = {server_allow}"), 1);
    format!("{viewer}allow = {viewer_allow}{server}\n[log]\noutput = \"relay.log\"\n")
}

/// Connects to the loopback `port` from `source`, any address of
/// 127.0.0.0/8. std cannot choose the source address, so tokio opens the
/// socket and hands it back as a blocking std stream.
fn connect_from(source: [u8; 4], port: u16) -> TcpStream {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .build()
        .unwrap_or_else(|error| panic!("runtime: {error}"));
    runtime.block_on(async {
        let socket =
            tokio::net::TcpSocket::new_v4().unwrap_or_else(|error| panic!("socket: {error}"));
        socket
            .bind(SocketAddr::from((source, 0)))
            .unwrap_or_else(|error| panic!("bind {source:?}: {error}"));
        let stream = socket
            .connect(SocketAddr::from(([127, 0, 0, 1], port)))
            .await
            .unwrap_or_else(|error| panic!("connect {port}: {error}"));
        let stream = stream
            .into_std()
            .unwrap_or_else(|error| panic!("into std: {error}"));
        stream
            .set_nonblocking(false)
            .unwrap_or_else(|error| panic!("blocking: {error}"));
        stream
    })
}

/// True when the server closes the connection (EOF or reset) within
/// `wait`, false when the connection is still open after it.
fn closed_within(stream: &mut TcpStream, wait: Duration) -> bool {
    stream
        .set_read_timeout(Some(wait))
        .unwrap_or_else(|error| panic!("read timeout: {error}"));
    let mut buf = [0; 64];
    match stream.read(&mut buf) {
        Ok(0) => true,
        Ok(n) => panic!("the server sent {n} bytes"),
        Err(error) => match error.kind() {
            ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted => true,
            ErrorKind::WouldBlock | ErrorKind::TimedOut => false,
            _ => panic!("read: {error}"),
        },
    }
}

/// Whether a log line is `event` on `listener` from `peer_ip`.
fn is(line: &str, event: &str, listener: &str, peer_ip: &str) -> bool {
    field(line, "event") == Some(event)
        && field(line, "listener") == Some(listener)
        && field(line, "peer_ip") == Some(peer_ip)
}

/// Waits until the log file has a line that `wanted` accepts and returns it.
fn wait_for_line(log: &Path, wanted: impl Fn(&str) -> bool) -> String {
    let deadline = Instant::now() + EXIT_LIMIT;
    loop {
        let text = std::fs::read_to_string(log).unwrap_or_default();
        if let Some(line) = text.lines().find(|line| wanted(line)) {
            return line.to_owned();
        }
        assert!(
            Instant::now() < deadline,
            "line not logged in {EXIT_LIMIT:?}: {text}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn read_log(log: &Path) -> String {
    std::fs::read_to_string(log).unwrap_or_else(|error| panic!("read {}: {error}", log.display()))
}

/// spec: relay.md:RLY-023 partial
/// spec: relay.md:RLY-024
/// spec: operations.md:OPS-037 partial
#[test]
fn checks_each_tcp_entry_point_against_its_own_allow_list() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = valid_setup("entry-allow", ports);
    write(
        &config,
        &entry_config(ports, r#"["127.0.0.2"]"#, r#"["127.0.0.3"]"#),
    );
    let log = config.with_file_name("relay.log");
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);

    let mut viewer = connect_from([127, 0, 0, 2], ports.tcp_viewer);
    let accepted_viewer = wait_for_line(&log, |line| {
        is(line, "connection_accepted", "tcp_viewer", "127.0.0.2")
    });
    assert_eq!(field(&accepted_viewer, "role"), Some("viewer"));
    assert_eq!(field(&accepted_viewer, "level"), Some("info"));

    // Each IP is allowed only on the other entry point. The bytes claim an
    // allowed IP; the server decides before it reads them.
    let claim = b"ID:1234 from 127.0.0.2 and 127.0.0.3";
    for (source, port, listener, ip) in [
        ([127, 0, 0, 3], ports.tcp_viewer, "tcp_viewer", "127.0.0.3"),
        ([127, 0, 0, 2], ports.tcp_server, "tcp_server", "127.0.0.2"),
    ] {
        let mut refused = connect_from(source, port);
        // The server may close first; a failed write changes nothing here.
        let _ = refused.write_all(claim);
        assert!(
            closed_within(&mut refused, EXIT_LIMIT),
            "{listener} kept {ip}"
        );
        let rejected = wait_for_line(&log, |line| is(line, "connection_rejected", listener, ip));
        assert_eq!(field(&rejected, "level"), Some("warn"));
        assert_eq!(field(&rejected, "stage"), Some("accept"));
        assert_eq!(field(&rejected, "reason"), Some("ip_not_allowed"));
        assert_eq!(field(&rejected, "conn_id"), None);
    }

    let mut server = connect_from([127, 0, 0, 3], ports.tcp_server);
    let accepted_server = wait_for_line(&log, |line| {
        is(line, "connection_accepted", "tcp_server", "127.0.0.3")
    });
    assert_eq!(field(&accepted_server, "role"), Some("server"));
    let viewer_id = field(&accepted_viewer, "conn_id").unwrap_or_default();
    let server_id = field(&accepted_server, "conn_id").unwrap_or_default();
    for id in [viewer_id, server_id] {
        let digits = id.strip_prefix('c').unwrap_or_default();
        assert!(
            !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()),
            "conn_id {id:?}"
        );
    }
    assert_ne!(viewer_id, server_id);

    // The refusals left the accepted connections open.
    assert!(!closed_within(&mut viewer, Duration::from_millis(300)));
    assert!(!closed_within(&mut server, Duration::from_millis(300)));
    let text = read_log(&log);
    for (listener, ip) in [("tcp_viewer", "127.0.0.3"), ("tcp_server", "127.0.0.2")] {
        assert!(
            !text
                .lines()
                .any(|line| is(line, "connection_accepted", listener, ip)),
            "{text}"
        );
    }
    assert!(!text.contains("ID:1234"), "{text}");
}

/// spec: websocket.md:WS-024
/// spec: operations.md:OPS-016 partial
#[test]
fn plaintext_ws_listener_is_off_by_default_and_warns_at_start_when_enabled() {
    let _lock = lock_ports();
    // Hold the WS port while the other ports are chosen, so that they differ.
    let holder =
        TcpListener::bind("127.0.0.1:0").unwrap_or_else(|error| panic!("free port: {error}"));
    let ws_port = holder
        .local_addr()
        .unwrap_or_else(|error| panic!("local addr: {error}"))
        .port();
    let ports = Ports::free();
    drop(holder);
    let ws_listen = format!("127.0.0.1:{ws_port}");
    let log_section = "\n[log]\noutput = \"relay.log\"\n";

    // Default: no WS section. No WS listener opens and no WS warning is logged.
    let config = setup_with("ws-default", &(config_text(ports) + log_section));
    let log = config.with_file_name("relay.log");
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);
    wait_for_line(&log, |line| field(line, "event") == Some("server_ready"));
    assert!(!connects(ws_port), "a default start opens no WS listener");
    let text = read_log(&log);
    assert!(!events(&text).contains(&"plaintext_ws_enabled"), "{text}");
    drop(guard);

    // Enabled: the WS listener opens, and the warning comes before server_ready.
    let ws_section = format!(
        "\n[relay.ws_viewer]\nenabled = true\nlisten = \"{ws_listen}\"\nallow = [\"127.0.0.1\"]\n"
    );
    let config = setup_with(
        "ws-enabled",
        &(config_text(ports) + &ws_section + log_section),
    );
    let log = config.with_file_name("relay.log");
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);
    wait_for_line(&log, |line| field(line, "event") == Some("server_ready"));
    assert!(
        connects(ws_port),
        "the enabled WS listener accepts TCP on {ws_listen}"
    );
    let text = read_log(&log);
    let order = events(&text);
    let warned = order
        .iter()
        .position(|event| *event == "plaintext_ws_enabled");
    let ready = order.iter().position(|event| *event == "server_ready");
    assert!(
        matches!((warned, ready), (Some(w), Some(r)) if w < r),
        "{text}"
    );
    let warning = text
        .lines()
        .find(|line| field(line, "event") == Some("plaintext_ws_enabled"))
        .unwrap_or_default();
    assert_eq!(field(warning, "level"), Some("warn"));
    assert_eq!(field(warning, "listener"), Some("ws_viewer"));
    assert_eq!(field(warning, "listen"), Some(ws_listen.as_str()));
    drop(guard);
}

/// spec: operations.md:OPS-040 partial
/// spec: operations.md:OPS-038 partial
#[test]
fn applies_item_boundaries_and_an_empty_list_at_accept() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = valid_setup("entry-bounds", ports);
    write(
        &config,
        &entry_config(
            ports,
            r#"["127.0.0.2-127.0.0.4", "127.0.0.8/30", "127.0.0.20"]"#,
            "[]",
        ),
    );
    let log = config.with_file_name("relay.log");
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);

    let mut open = Vec::new();
    for last in [2, 4, 8, 11, 20] {
        open.push(connect_from([127, 0, 0, last], ports.tcp_viewer));
        let ip = format!("127.0.0.{last}");
        wait_for_line(&log, |line| {
            is(line, "connection_accepted", "tcp_viewer", &ip)
        });
    }
    let refusals = [5, 7, 12, 19, 21]
        .map(|last| (last, ports.tcp_viewer, "tcp_viewer"))
        .into_iter()
        // The empty list of tcp_server refuses an address that tcp_viewer
        // allows.
        .chain([(2, ports.tcp_server, "tcp_server")]);
    for (last, port, listener) in refusals {
        let mut refused = connect_from([127, 0, 0, last], port);
        assert!(closed_within(&mut refused, EXIT_LIMIT), "{listener} {last}");
        let ip = format!("127.0.0.{last}");
        wait_for_line(&log, |line| is(line, "connection_rejected", listener, &ip));
    }
    for stream in &mut open {
        assert!(!closed_within(stream, Duration::from_millis(100)));
    }
}

/// spec: operations.md:OPS-038 partial
/// spec: operations.md:OPS-007 partial
#[cfg(unix)]
#[test]
fn closes_and_logs_every_held_connection_when_stopping() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = valid_setup("entry-stop", ports);
    write(
        &config,
        &entry_config(ports, r#"["0.0.0.0/0"]"#, r#"["127.0.0.3"]"#),
    );
    let log = config.with_file_name("relay.log");
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);

    // The whole-family item admits any IPv4 source after its warning.
    let mut held = connect_from([127, 0, 0, 2], ports.tcp_viewer);
    let accepted = wait_for_line(&log, |line| {
        is(line, "connection_accepted", "tcp_viewer", "127.0.0.2")
    });
    let conn_id = field(&accepted, "conn_id").unwrap_or_default().to_owned();

    let child = guard.take();
    send_signal(&child, "TERM");
    assert!(closed_within(&mut held, EXIT_LIMIT), "connection kept open");
    let run = finish(child);
    assert_eq!(run.code, Some(0), "{}", run.stderr);

    let text = read_log(&log);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(field(lines[1], "event"), Some("allow_all_source"));
    assert_eq!(field(lines[1], "item"), Some("0.0.0.0/0"));
    assert_eq!(
        lines.last().and_then(|line| field(line, "event")),
        Some("server_stopped")
    );
    let aborted = lines
        .iter()
        .find(|line| {
            field(line, "event") == Some("connection_aborted")
                && field(line, "conn_id") == Some(&conn_id)
        })
        .unwrap_or_else(|| panic!("no connection_aborted for {conn_id}: {text}"));
    assert_eq!(field(aborted, "level"), Some("info"));
    assert_eq!(field(aborted, "listener"), Some("tcp_viewer"));
    assert_eq!(field(aborted, "stage"), Some("initial_exchange"));
    assert_eq!(field(aborted, "reason"), Some("server_shutdown"));
    assert_eq!(field(aborted, "peer_ip"), Some("127.0.0.2"));
    assert!(aborted.contains("\"elapsed_ms\":"), "{aborted}");
    // Every accepted connection, the readiness probes included, is closed
    // and logged before the last line.
    let count = |event: &str| {
        lines
            .iter()
            .filter(|line| field(line, "event") == Some(event))
            .count()
    };
    assert_eq!(
        count("connection_accepted"),
        count("connection_aborted"),
        "{text}"
    );
    assert_released(ports);
}

#[cfg(target_os = "linux")]
#[test]
fn checks_an_ipv4_peer_of_a_dual_stack_listener_against_ipv4_items() {
    let _lock = lock_ports();
    let ports = Ports::free_on("[::]:0");
    let config = valid_setup("entry-dual-stack", ports);
    let text = entry_config(ports, r#"["127.0.0.2"]"#, r#"["127.0.0.3"]"#).replacen(
        &format!("listen = \"127.0.0.1:{}\"", ports.tcp_viewer),
        &format!("listen = \"[::]:{}\"", ports.tcp_viewer),
        1,
    );
    write(&config, &text);
    let log = config.with_file_name("relay.log");
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);

    // The [::] listener sees ::ffff:127.0.0.2; the log and the allow list
    // both use 127.0.0.2.
    let mut viewer = connect_from([127, 0, 0, 2], ports.tcp_viewer);
    wait_for_line(&log, |line| {
        is(line, "connection_accepted", "tcp_viewer", "127.0.0.2")
    });
    assert!(!closed_within(&mut viewer, Duration::from_millis(100)));
}

/// spec: operations.md:OPS-004 partial
#[test]
fn refuses_relay_connections_beyond_l_and_accepts_again_after_a_release() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = valid_setup("entry-limit", ports);
    write(
        &config,
        &(entry_config(ports, r#"["127.0.0.2"]"#, r#"["127.0.0.3"]"#)
            + "\n[limits]\nrelay_connections = 2\nwaiting = 1\n"),
    );
    let log = config.with_file_name("relay.log");
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);

    // Both entry points take places from the one L.
    let viewer = connect_from([127, 0, 0, 2], ports.tcp_viewer);
    let accepted = wait_for_line(&log, |line| {
        is(line, "connection_accepted", "tcp_viewer", "127.0.0.2")
    });
    let mut server = connect_from([127, 0, 0, 3], ports.tcp_server);
    wait_for_line(&log, |line| {
        is(line, "connection_accepted", "tcp_server", "127.0.0.3")
    });

    let mut refused = connect_from([127, 0, 0, 2], ports.tcp_viewer);
    assert!(closed_within(&mut refused, EXIT_LIMIT), "third kept");
    let rejected = wait_for_line(&log, |line| {
        is(line, "connection_rejected", "tcp_viewer", "127.0.0.2")
    });
    assert_eq!(field(&rejected, "reason"), Some("relay_connection_limit"));
    assert_eq!(field(&rejected, "stage"), Some("accept"));
    assert!(rejected.contains("\"limit\":2"), "{rejected}");
    // A source outside the allow list is refused for its IP even at L.
    let mut stranger = connect_from([127, 0, 0, 4], ports.tcp_viewer);
    assert!(closed_within(&mut stranger, EXIT_LIMIT), "stranger kept");
    let rejected = wait_for_line(&log, |line| {
        is(line, "connection_rejected", "tcp_viewer", "127.0.0.4")
    });
    assert_eq!(field(&rejected, "reason"), Some("ip_not_allowed"));
    assert!(!closed_within(&mut server, Duration::from_millis(100)));

    // The client leaves; the server notices, gives the place back, and
    // takes the next connection.
    drop(viewer);
    let conn_id = field(&accepted, "conn_id").unwrap_or_default().to_owned();
    let aborted = wait_for_line(&log, |line| {
        field(line, "event") == Some("connection_aborted")
            && field(line, "conn_id") == Some(conn_id.as_str())
    });
    assert_eq!(field(&aborted, "reason"), Some("peer_eof"));
    assert_eq!(field(&aborted, "level"), Some("info"));
    let mut next = connect_from([127, 0, 0, 2], ports.tcp_viewer);
    assert!(!closed_within(&mut next, Duration::from_millis(300)));
    let text = read_log(&log);
    let accepted_viewer = text
        .lines()
        .filter(|line| is(line, "connection_accepted", "tcp_viewer", "127.0.0.2"))
        .count();
    assert_eq!(accepted_viewer, 2, "{text}");
}

/// spec: relay.md:RLY-005 partial
/// spec: operations.md:OPS-006 partial
#[test]
fn tells_a_peer_close_from_an_initial_exchange_that_outlives_h() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = valid_setup("entry-handshake", ports);
    write(
        &config,
        &(entry_config(ports, r#"["127.0.0.2"]"#, r#"["127.0.0.3"]"#)
            + "\n[timeouts]\nhandshake_secs = 1\n"),
    );
    let log = config.with_file_name("relay.log");
    let mut guard = Guard(Some(spawn(&config)));
    wait_listening(guard.child(), ports);

    // Bytes that never finish an initial exchange: H closes the connection.
    let started = Instant::now();
    let mut slow = connect_from([127, 0, 0, 2], ports.tcp_viewer);
    slow.write_all(b"partial")
        .unwrap_or_else(|error| panic!("write: {error}"));
    assert!(closed_within(&mut slow, EXIT_LIMIT), "kept past H");
    assert!(started.elapsed() >= Duration::from_secs(1));
    let timed_out = wait_for_line(&log, |line| {
        is(line, "connection_aborted", "tcp_viewer", "127.0.0.2")
    });
    assert_eq!(field(&timed_out, "reason"), Some("handshake_timeout"));
    assert_eq!(field(&timed_out, "level"), Some("warn"));
    assert_eq!(field(&timed_out, "stage"), Some("initial_exchange"));
    let elapsed = timed_out
        .split("\"elapsed_ms\":")
        .nth(1)
        .and_then(|rest| rest.split([',', '}']).next())
        .and_then(|digits| digits.parse::<u64>().ok())
        .unwrap_or_else(|| panic!("no elapsed_ms: {timed_out}"));
    assert!(elapsed >= 1000, "{timed_out}");
    assert!(!read_log(&log).contains("partial"));

    // A peer that closes before sending anything is a peer_eof.
    let quitter = connect_from([127, 0, 0, 3], ports.tcp_server);
    wait_for_line(&log, |line| {
        is(line, "connection_accepted", "tcp_server", "127.0.0.3")
    });
    drop(quitter);
    let closed = wait_for_line(&log, |line| {
        is(line, "connection_aborted", "tcp_server", "127.0.0.3")
    });
    assert_eq!(field(&closed, "reason"), Some("peer_eof"));
    assert_eq!(field(&closed, "level"), Some("info"));
}

/// `GET path` on the loopback `port`: the status code and the body, or
/// `None` when the server closed the connection without a reply.
fn http_get(port: u16, path: &str) -> Option<(u16, String)> {
    http_request(port, "GET", path)
}

/// `method path` on the loopback `port` with no body: the status code and the
/// body, or `None` when the server closed the connection without a reply.
fn http_request(port: u16, method: &str, path: &str) -> Option<(u16, String)> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .unwrap_or_else(|error| panic!("connect {port}: {error}"));
    stream
        .set_read_timeout(Some(EXIT_LIMIT))
        .unwrap_or_else(|error| panic!("read timeout: {error}"));
    let request = format!("{method} {path} HTTP/1.1\r\nHost: status\r\nConnection: close\r\n\r\n");
    // A refused connection may close before the request is written.
    if stream.write_all(request.as_bytes()).is_err() {
        return None;
    }
    let mut raw = String::new();
    match stream.read_to_string(&mut raw) {
        Ok(_) if raw.is_empty() => return None,
        Ok(_) => {}
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted
            ) =>
        {
            return None;
        }
        Err(error) => panic!("read: {error}"),
    }
    let (head, body) = raw
        .split_once("\r\n\r\n")
        .unwrap_or_else(|| panic!("no header end: {raw:?}"));
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status line: {raw:?}"));
    Some((status, body.to_owned()))
}

/// spec: operations.md:OPS-013 partial
/// spec: operations.md:OPS-032
#[test]
fn answers_status_queries_without_login_once_ready() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = valid_setup("status-ready", ports);
    write(
        &config,
        &entry_config(ports, r#"["127.0.0.1"]"#, r#"["127.0.0.1"]"#),
    );
    let log = config.with_file_name("relay.log");
    let _guard = Guard(Some(spawn(&config)));

    let ready = wait_for_line(&log, |line| field(line, "event") == Some("server_ready"));
    assert_eq!(field(&ready, "level"), Some("info"));
    for (name, port) in [
        ("status", ports.status),
        ("management", ports.management),
        ("tcp_viewer", ports.tcp_viewer),
        ("tcp_server", ports.tcp_server),
    ] {
        assert!(ready.contains(&format!("\"name\":\"{name}\"")), "{ready}");
        assert!(
            ready.contains(&format!("\"listen\":\"127.0.0.1:{port}\"")),
            "{ready}"
        );
    }
    // The status listener is its own listener and needs no login.
    assert_eq!(
        http_get(ports.status, "/readyz"),
        Some((200, r#"{"status":"ready"}"#.to_owned()))
    );
    assert_eq!(
        http_get(ports.status, "/livez"),
        Some((200, r#"{"status":"alive"}"#.to_owned()))
    );
}

/// Loopback plaintext starts, and every API request without a session gets
/// `401 unauthenticated` with no data, before the path or method is judged.
fn answers_api_queries_with_401_without_login_on_loopback_plaintext() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = valid_setup("plaintext-loopback", ports);
    write(
        &config,
        &entry_config(ports, r#"["127.0.0.1"]"#, r#"["127.0.0.1"]"#),
    );
    let log = config.with_file_name("relay.log");
    let _guard = Guard(Some(spawn(&config)));
    let ready = wait_for_line(&log, |line| field(line, "event") == Some("server_ready"));
    assert!(
        ready.contains(&format!("\"listen\":\"127.0.0.1:{}\"", ports.management)),
        "{ready}"
    );
    // Plaintext on loopback starts. No session exists before login, so the
    // API answers 401 before it judges the path or the method. The query
    // paths and the termination path refuse alike, with no data in the body.
    for (method, path) in LOGIN_LESS_REQUESTS {
        let (status, body) = http_request(ports.management, method, path)
            .unwrap_or_else(|| panic!("no reply to {method} {path}"));
        assert_eq!(status, 401, "{method} {path}: {body}");
        assert!(
            body.contains(r#""code":"unauthenticated""#),
            "{method} {path}: {body}"
        );
    }
}

/// spec: operations.md:OPS-033 partial
/// spec: operations.md:OPS-004 partial
#[test]
fn refuses_status_connections_beyond_lm_and_keeps_lm_and_l_apart() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = valid_setup("status-limit", ports);
    write(
        &config,
        &(entry_config(ports, r#"["127.0.0.2"]"#, r#"["127.0.0.3"]"#)
            + "\n[limits]\nrelay_connections = 2\nwaiting = 1\nadmin_connections = 1\n"),
    );
    let log = config.with_file_name("relay.log");
    // No `wait_listening`: its probe of the status port would take the one
    // place of Lm for a moment.
    let _guard = Guard(Some(spawn(&config)));
    wait_for_line(&log, |line| field(line, "event") == Some("server_ready"));

    // An idle status connection holds the one place of Lm.
    let holder = connect_from([127, 0, 0, 5], ports.status);
    let mut refused = connect_from([127, 0, 0, 6], ports.status);
    assert!(closed_within(&mut refused, EXIT_LIMIT), "second kept");
    let rejected = wait_for_line(&log, |line| {
        is(line, "connection_rejected", "status", "127.0.0.6")
    });
    assert_eq!(field(&rejected, "reason"), Some("admin_connection_limit"));
    assert_eq!(field(&rejected, "stage"), Some("accept"));
    assert!(rejected.contains("\"limit\":1"), "{rejected}");

    // A full Lm does not stop relay connections, and the status connection
    // takes no place of L: both places of L are still free.
    let mut viewer = connect_from([127, 0, 0, 2], ports.tcp_viewer);
    wait_for_line(&log, |line| {
        is(line, "connection_accepted", "tcp_viewer", "127.0.0.2")
    });
    let mut server = connect_from([127, 0, 0, 3], ports.tcp_server);
    wait_for_line(&log, |line| {
        is(line, "connection_accepted", "tcp_server", "127.0.0.3")
    });
    let mut third = connect_from([127, 0, 0, 2], ports.tcp_viewer);
    assert!(closed_within(&mut third, EXIT_LIMIT), "third kept");
    let full = wait_for_line(&log, |line| {
        is(line, "connection_rejected", "tcp_viewer", "127.0.0.2")
    });
    assert_eq!(field(&full, "reason"), Some("relay_connection_limit"));

    // With L full, a status query is answered once Lm has a place again.
    drop(holder);
    let deadline = Instant::now() + EXIT_LIMIT;
    let answer = loop {
        if let Some(answer) = http_get(ports.status, "/readyz") {
            break answer;
        }
        assert!(Instant::now() < deadline, "Lm place not given back");
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(answer, (200, r#"{"status":"ready"}"#.to_owned()));
    assert!(!closed_within(&mut viewer, Duration::from_millis(100)));
    assert!(!closed_within(&mut server, Duration::from_millis(100)));
}

/// spec: operations.md:OPS-033 partial
/// spec: operations.md:OPS-004 partial
#[test]
fn refuses_management_connections_beyond_lm_and_keeps_lm_and_l_apart() {
    let _lock = lock_ports();
    let ports = Ports::free();
    let config = valid_setup("management-limit", ports);
    write(
        &config,
        &(entry_config(ports, r#"["127.0.0.2"]"#, r#"["127.0.0.3"]"#)
            + "\n[limits]\nrelay_connections = 2\nwaiting = 1\nadmin_connections = 1\n"),
    );
    let log = config.with_file_name("relay.log");
    // No `wait_listening`: its probe of the management port would take the one
    // place of Lm for a moment.
    let _guard = Guard(Some(spawn(&config)));
    wait_for_line(&log, |line| field(line, "event") == Some("server_ready"));

    // An idle management connection holds the one place of Lm.
    let holder = connect_from([127, 0, 0, 5], ports.management);
    let mut refused = connect_from([127, 0, 0, 6], ports.management);
    assert!(closed_within(&mut refused, EXIT_LIMIT), "second kept");
    let rejected = wait_for_line(&log, |line| {
        is(line, "connection_rejected", "management", "127.0.0.6")
    });
    assert_eq!(field(&rejected, "reason"), Some("admin_connection_limit"));
    assert_eq!(field(&rejected, "stage"), Some("accept"));
    assert!(rejected.contains("\"limit\":1"), "{rejected}");

    // A full Lm does not stop relay connections. The management connection
    // takes no place of L: both places of L are still free.
    let mut viewer = connect_from([127, 0, 0, 2], ports.tcp_viewer);
    wait_for_line(&log, |line| {
        is(line, "connection_accepted", "tcp_viewer", "127.0.0.2")
    });
    let mut server = connect_from([127, 0, 0, 3], ports.tcp_server);
    wait_for_line(&log, |line| {
        is(line, "connection_accepted", "tcp_server", "127.0.0.3")
    });
    let mut third = connect_from([127, 0, 0, 2], ports.tcp_viewer);
    assert!(closed_within(&mut third, EXIT_LIMIT), "third kept");
    let full = wait_for_line(&log, |line| {
        is(line, "connection_rejected", "tcp_viewer", "127.0.0.2")
    });
    assert_eq!(field(&full, "reason"), Some("relay_connection_limit"));

    // With Lm given back, a management request is answered again.
    drop(holder);
    let deadline = Instant::now() + EXIT_LIMIT;
    let answer = loop {
        if let Some(answer) = http_get(ports.management, "/") {
            break answer;
        }
        assert!(Instant::now() < deadline, "Lm place not given back");
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(answer.0, 303);
    assert!(!closed_within(&mut viewer, Duration::from_millis(100)));
    assert!(!closed_within(&mut server, Duration::from_millis(100)));
}
