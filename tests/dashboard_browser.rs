//! Browser tests of the dashboard pages. Each page runs its own checks in the
//! browser and sets `data-result`. These tests start headless Chromium with
//! `--dump-dom` and read the result from the dumped DOM.
//!
//! Set `DASHBOARD_BROWSER` to the browser executable. Without it, the tests
//! look for Chrome or Edge in the usual install folders and on `PATH`. When no
//! browser is found, the tests fail. They are never skipped.
//!
//! The static server below repeats the policy of `xtask/src/dashboard_serve.rs`
//! (docs/contracts/management-api-v1.md §1.11, docs/conventions/frontend.md
//! §1.2). Keep the two in sync.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

/// Same policy string as `xtask/src/dashboard_serve.rs` (frontend.md §1.2, Q37).
const CSP: &str = "default-src 'self'; frame-ancestors 'none'";
const WIDTHS: [u32; 6] = [1280, 1024, 1023, 768, 390, 320];
const LANGS: [&str; 2] = ["ko", "en"];
const RUN_LIMIT: Duration = Duration::from_secs(60);
/// Browsers that run at once. Each run's `RUN_LIMIT` clock includes Chromium's
/// startup, which slows past the limit when every test starts its own browser.
const MAX_BROWSERS: usize = 4;
const IO_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_HEAD: usize = 8 * 1024;

// Chrome comes first: Edge's launcher can exit before it writes the DOM dump.
const ABSOLUTE_BROWSERS: [&str; 4] = [
    r"C:\Program Files\Google\Chrome\Application\chrome.exe",
    r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
];
const PATH_BROWSERS: [&str; 5] = [
    "google-chrome",
    "chromium",
    "chromium-browser",
    "msedge",
    "microsoft-edge",
];

/// Shown when no browser is found. The tests fail with this message.
const MISSING_BROWSER: &str = concat!(
    "Chromium is required for the dashboard browser tests. ",
    "Install Edge or Chrome, or set DASHBOARD_BROWSER to the browser executable.",
);

/// Makes temp folder names unique across the runs of this test process.
static RUN_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[test]
fn dashboard_components_pass_in_browser() {
    check_page("/tests/components.html");
}

#[test]
fn dashboard_preview_pass_in_browser() {
    check_page("/preview/index.html");
}

/// spec: dashboard.md:DASH-017 partial
#[test]
fn login_signs_in_with_correct_credentials_in_browser() {
    check_login_case("dash-017");
}

/// spec: dashboard.md:DASH-018 partial
#[test]
fn login_refuses_empty_and_wrong_input_in_browser() {
    check_login_case("dash-018");
}

/// spec: dashboard.md:DASH-024 partial
#[test]
fn login_picks_the_language_from_browser_preferences_in_browser() {
    check_login_case("dash-024");
}

/// spec: dashboard.md:DASH-025 partial
#[test]
fn login_keeps_the_chosen_language_and_account_name_in_browser() {
    check_login_case("dash-025");
}

/// spec: dashboard.md:DASH-027 partial
#[test]
fn login_shows_the_request_failed_notice_in_browser() {
    check_login_case("dash-027");
}

/// spec: dashboard.md:DASH-001 partial
#[test]
fn operations_shows_the_overview_in_browser() {
    check_localized_case("/tests/operations.html", "dash-001");
}

/// spec: dashboard.md:DASH-002 partial
#[test]
fn operations_shows_empty_lists_as_empty_not_error_in_browser() {
    check_localized_case("/tests/operations.html", "dash-002");
}

/// spec: dashboard.md:DASH-003 partial
#[test]
fn operations_shows_failure_and_keeps_stale_data_in_browser() {
    check_localized_case("/tests/operations.html", "dash-003");
}

/// spec: dashboard.md:DASH-004 partial
#[test]
fn operations_refresh_recovers_after_failure_in_browser() {
    check_localized_case("/tests/operations.html", "dash-004");
}

/// spec: dashboard.md:DASH-005 partial
#[test]
fn operations_limits_one_query_per_list_and_drops_late_responses_in_browser() {
    check_localized_case("/tests/operations.html", "dash-005");
}

/// spec: dashboard.md:DASH-008 partial
#[test]
fn operations_sends_expired_session_to_sign_in_in_browser() {
    check_localized_case("/tests/operations.html", "dash-008");
}

/// spec: dashboard.md:DASH-016 partial
#[test]
fn operations_shows_state_change_on_next_poll_in_browser() {
    check_localized_case("/tests/operations.html", "dash-016");
}

/// spec: dashboard.md:DASH-019 partial
#[test]
fn login_shows_the_unconfirmed_sign_out_notice_in_browser() {
    check_localized_case("/tests/login.html", "dash-019");
}

/// spec: dashboard.md:DASH-019 partial
#[test]
fn operations_sign_out_goes_to_sign_in_in_browser() {
    check_localized_case("/tests/operations.html", "dash-019");
}

/// spec: dashboard.md:DASH-020 partial
#[test]
fn operations_shows_closed_session_causes_in_browser() {
    check_localized_case("/tests/operations.html", "dash-020");
}

/// spec: dashboard.md:DASH-021 partial
#[test]
fn operations_shows_unknown_usage_as_unavailable_in_browser() {
    check_localized_case("/tests/operations.html", "dash-021");
}

/// spec: dashboard.md:DASH-022 partial
#[test]
fn operations_polls_at_the_configured_interval_in_browser() {
    check_localized_case("/tests/operations.html", "dash-022");
}

/// spec: dashboard.md:DASH-024 partial
#[test]
fn operations_follows_the_browser_language_in_browser() {
    check_localized_case("/tests/operations.html", "dash-024");
}

/// spec: dashboard.md:DASH-025 partial
#[test]
fn operations_keeps_the_chosen_language_in_browser() {
    check_localized_case("/tests/operations.html", "dash-025");
}

/// spec: dashboard.md:DASH-006 partial
#[test]
fn session_shows_the_facts_and_terminate_action_in_browser() {
    check_localized_case("/tests/session.html", "dash-006");
}

/// spec: dashboard.md:DASH-006 partial
#[test]
fn router_opens_session_detail_from_the_list_in_browser() {
    check_localized_case("/tests/router.html", "dash-006");
}

/// spec: dashboard.md:DASH-007 partial
#[test]
fn session_gone_before_or_during_refresh_shows_not_found_in_browser() {
    check_localized_case("/tests/session.html", "dash-007");
}

/// spec: dashboard.md:DASH-009 partial
/// spec: dashboard.md:DASH-015 partial
#[test]
fn session_cancel_sends_no_request_and_restores_focus_in_browser() {
    check_localized_case("/tests/session.html", "dash-009");
}

/// spec: dashboard.md:DASH-010 partial
#[test]
fn session_blocks_duplicate_terminate_and_shows_not_found_in_browser() {
    check_localized_case("/tests/session.html", "dash-010");
}

/// spec: dashboard.md:DASH-011 partial
#[test]
fn session_gone_before_or_after_confirm_shows_not_found_in_browser() {
    check_localized_case("/tests/session.html", "dash-011");
}

/// spec: dashboard.md:DASH-012 partial
#[test]
fn session_failed_terminate_is_not_complete_in_browser() {
    check_localized_case("/tests/session.html", "dash-012");
}

/// spec: dashboard.md:DASH-013 partial
#[test]
fn session_unknown_terminate_is_not_resent_in_browser() {
    check_localized_case("/tests/session.html", "dash-013");
}

/// spec: dashboard.md:DASH-014 partial
#[test]
fn session_direct_entry_without_session_goes_to_sign_in_in_browser() {
    check_localized_case("/tests/session.html", "dash-014");
}

/// spec: dashboard.md:DASH-026 partial
#[test]
fn session_invalid_id_sends_no_request_in_browser() {
    check_localized_case("/tests/session.html", "dash-026");
}

/// spec: dashboard.md:DASH-016 partial
#[test]
fn session_shows_state_change_on_next_poll_in_browser() {
    check_localized_case("/tests/session.html", "dash-016");
}

/// spec: dashboard.md:DASH-019 partial
#[test]
fn session_sign_out_removes_data_or_shows_unconfirmed_in_browser() {
    check_localized_case("/tests/session.html", "dash-019");
}

/// spec: dashboard.md:DASH-022 partial
#[test]
fn session_polls_at_configured_interval_without_overlap_in_browser() {
    check_localized_case("/tests/session.html", "dash-022");
}

/// spec: dashboard.md:DASH-024 partial
#[test]
fn session_follows_the_browser_language_in_browser() {
    check_localized_case("/tests/session.html", "dash-024");
}

/// spec: dashboard.md:DASH-025 partial
#[test]
fn session_keeps_the_chosen_language_in_browser() {
    check_localized_case("/tests/session.html", "dash-025");
}

/// Runs one page at every width and language, then reports all failing runs at once.
fn check_page(page: &str) {
    let mut runs = Vec::new();
    for lang in LANGS {
        for width in WIDTHS {
            runs.push((format!("{page}?lang={lang}"), width));
        }
    }
    check_runs(&runs);
}

/// Runs one sign-in page scenario, `/tests/login.html?case=…`, at every width.
fn check_login_case(case: &str) {
    let runs: Vec<(String, u32)> = WIDTHS
        .iter()
        .map(|width| (format!("/tests/login.html?case={case}"), *width))
        .collect();
    check_runs(&runs);
}

/// Runs one scenario page, `{page}?case=…&lang=…`, in every language at every width.
fn check_localized_case(page: &str, case: &str) {
    let mut runs = Vec::new();
    for lang in LANGS {
        for width in WIDTHS {
            runs.push((format!("{page}?case={case}&lang={lang}"), width));
        }
    }
    check_runs(&runs);
}

/// Runs each (page path with query, width) pair in headless Chromium, then reports
/// all failing runs at once.
fn check_runs(runs: &[(String, u32)]) {
    let browser = match find_browser() {
        Ok(browser) => browser,
        Err(message) => panic!("{message}"),
    };
    let port = start_server();
    let mut failures = Vec::new();
    for (path, width) in runs {
        let url = format!("http://127.0.0.1:{port}{path}");
        let label = format!("{path} width={width}");
        match dump_dom(&browser, &url, *width) {
            Ok(dom) if dom.contains("data-result=\"pass\"") => {}
            Ok(dom) => failures.push(describe(&label, &dom)),
            Err(message) => failures.push(format!("{label}: {message}")),
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} browser runs failed:\n{}",
        failures.len(),
        runs.len(),
        failures.join("\n")
    );
}

/// Finds the browser: `DASHBOARD_BROWSER`, then the usual Edge and Chrome
/// install paths, then `PATH`.
fn find_browser() -> Result<PathBuf, String> {
    if let Some(value) = std::env::var_os("DASHBOARD_BROWSER") {
        let path = PathBuf::from(value);
        return if path.is_file() {
            Ok(path)
        } else {
            Err(format!(
                "DASHBOARD_BROWSER does not point to a file: {}",
                path.display()
            ))
        };
    }
    for candidate in ABSOLUTE_BROWSERS {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return Ok(path);
        }
    }
    for name in PATH_BROWSERS {
        if let Some(path) = on_path(name) {
            return Ok(path);
        }
    }
    Err(MISSING_BROWSER.to_string())
}

/// Finds `name` (or `name.exe`) in a `PATH` folder.
fn on_path(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths)
        .flat_map(|dir| [dir.join(name), dir.join(format!("{name}.exe"))])
        .find(|candidate| candidate.is_file())
}

/// Runs headless Chromium once and returns the DOM it dumps.
fn dump_dom(browser: &Path, url: &str, width: u32) -> Result<String, String> {
    let run = RUN_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("vr-dashboard-browser-{}-{run}", std::process::id()));
    fs::create_dir_all(&dir)
        .map_err(|error| format!("cannot create {}: {error}", dir.display()))?;
    let dom_path = dir.join("dom.html");
    let result = run_chromium(browser, url, width, &dir, &dom_path);
    // Best effort: the browser may still hold files for a moment after exit.
    let _ = fs::remove_dir_all(&dir);
    result
}

static BROWSERS_RUNNING: Mutex<usize> = Mutex::new(0);
static BROWSER_EXITED: Condvar = Condvar::new();

/// One of the `MAX_BROWSERS` slots, held until the browser has exited.
struct BrowserSlot;

impl BrowserSlot {
    /// Waits until a slot is free, then takes it.
    fn acquire() -> Self {
        let mut running = BROWSERS_RUNNING
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        while *running >= MAX_BROWSERS {
            running = BROWSER_EXITED
                .wait(running)
                .unwrap_or_else(PoisonError::into_inner);
        }
        *running += 1;
        BrowserSlot
    }
}

impl Drop for BrowserSlot {
    fn drop(&mut self) {
        *BROWSERS_RUNNING
            .lock()
            .unwrap_or_else(PoisonError::into_inner) -= 1;
        BROWSER_EXITED.notify_one();
    }
}

/// Starts the browser and waits up to `RUN_LIMIT`. The DOM goes to a file, not a
/// pipe, so a full pipe cannot block the wait loop.
fn run_chromium(
    browser: &Path,
    url: &str,
    width: u32,
    dir: &Path,
    dom_path: &Path,
) -> Result<String, String> {
    let _slot = BrowserSlot::acquire();
    let dom_file = File::create(dom_path)
        .map_err(|error| format!("cannot create {}: {error}", dom_path.display()))?;
    let mut child = Command::new(browser)
        .args([
            "--headless",
            "--disable-gpu",
            "--no-first-run",
            "--no-default-browser-check",
        ])
        .arg(format!("--user-data-dir={}", dir.join("profile").display()))
        .arg(format!("--window-size={width},900"))
        .args(["--virtual-time-budget=10000", "--dump-dom", url])
        .stdin(Stdio::null())
        .stdout(Stdio::from(dom_file))
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("cannot start {}: {error}", browser.display()))?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() < RUN_LIMIT => thread::sleep(Duration::from_millis(100)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "browser did not exit within {} s",
                    RUN_LIMIT.as_secs()
                ));
            }
            Err(error) => return Err(format!("cannot wait for browser: {error}")),
        }
    }
    let bytes = fs::read(dom_path).map_err(|error| format!("cannot read DOM dump: {error}"))?;
    if bytes.is_empty() {
        return Err(format!("{} wrote no DOM dump", browser.display()));
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// One line for a failing run: the result and failure ids the page reported.
fn describe(label: &str, dom: &str) -> String {
    let result = attribute(dom, "data-result").unwrap_or_else(|| "missing".to_string());
    let failures = attribute(dom, "data-failures").unwrap_or_default();
    let csp = attribute(dom, "data-csp-violations")
        .map(|count| format!(" data-csp-violations={count}"))
        .unwrap_or_default();
    format!("{label}: data-result={result} data-failures=\"{failures}\"{csp}")
}

/// Value of the first `name="…"` attribute in the dumped DOM.
fn attribute(dom: &str, name: &str) -> Option<String> {
    let needle = format!(" {name}=\"");
    let start = dom.find(needle.as_str())? + needle.len();
    let end = dom[start..].find('"')? + start;
    Some(dom[start..end].to_string())
}

/// Starts the static server on a free loopback port. It runs until the test process exits.
fn start_server() -> u16 {
    let dashboard = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("web")
        .join("dashboard");
    let Ok(root) = fs::canonicalize(&dashboard) else {
        panic!("cannot open {}", dashboard.display());
    };
    let Ok(listener) = TcpListener::bind(("127.0.0.1", 0)) else {
        panic!("cannot bind 127.0.0.1");
    };
    let Ok(addr) = listener.local_addr() else {
        panic!("cannot read the bound address");
    };
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let root = root.clone();
            thread::spawn(move || {
                let _ = serve_connection(stream, &root);
            });
        }
    });
    addr.port()
}

/// Handles one connection: read the request, write one response, close.
fn serve_connection(mut stream: TcpStream, root: &Path) -> io::Result<()> {
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    let Some(head) = read_head(&mut stream)? else {
        return write_reply(&mut stream, "400 Bad Request", &[], &[], false);
    };
    let Some((method, target)) = request_line(&head) else {
        return write_reply(&mut stream, "400 Bad Request", &[], &[], false);
    };
    if method != "GET" && method != "HEAD" {
        let allow = [("Allow", "GET, HEAD")];
        return write_reply(&mut stream, "405 Method Not Allowed", &allow, &[], false);
    }
    let path = target.split(['?', '#']).next().unwrap_or_default();
    if path == "/" {
        let location = [("Location", "/preview/index.html")];
        return write_reply(&mut stream, "302 Found", &location, &[], false);
    }
    let file = match resolve(root, path) {
        Ok(file) => file,
        Err(status) => return write_reply(&mut stream, status, &[], &[], false),
    };
    let (Some(kind), Ok(body)) = (content_type(&file), fs::read(&file)) else {
        return write_reply(&mut stream, "404 Not Found", &[], &[], false);
    };
    let kind = [("Content-Type", kind)];
    write_reply(&mut stream, "200 OK", &kind, &body, method == "GET")
}

/// Reads up to the blank line that ends the head. `None` when the head is too
/// long or the client closes first.
fn read_head(stream: &mut TcpStream) -> io::Result<Option<Vec<u8>>> {
    let mut head = Vec::new();
    let mut chunk = [0_u8; 1024];
    loop {
        if head.windows(4).any(|window| window == b"\r\n\r\n") {
            return Ok(Some(head));
        }
        if head.len() >= MAX_HEAD {
            return Ok(None);
        }
        let want = chunk.len().min(MAX_HEAD - head.len());
        let read = stream.read(&mut chunk[..want])?;
        if read == 0 {
            return Ok(None);
        }
        head.extend_from_slice(&chunk[..read]);
    }
}

/// Method and raw request target. `None` when the request line is malformed.
fn request_line(head: &[u8]) -> Option<(&str, &str)> {
    let end = head.windows(2).position(|pair| pair == b"\r\n")?;
    let line = std::str::from_utf8(&head[..end]).ok()?;
    let mut parts = line.split(' ');
    let (method, target, version) = (parts.next()?, parts.next()?, parts.next()?);
    let well_formed = parts.next().is_none()
        && !method.is_empty()
        && target.starts_with('/')
        && version.starts_with("HTTP/1.");
    well_formed.then_some((method, target))
}

/// Maps a request path to a file under `root`. `Err` carries the status line:
/// 400 for a forbidden sequence, 404 for anything missing or outside the root.
fn resolve(root: &Path, raw: &str) -> Result<PathBuf, &'static str> {
    let decoded = percent_decode(raw).ok_or("400 Bad Request")?;
    if decoded.contains("..") || decoded.contains(['\\', '\0', ':']) {
        return Err("400 Bad Request");
    }
    let mut path = root.to_path_buf();
    for part in decoded.split('/').filter(|part| !part.is_empty()) {
        path.push(part);
    }
    let found = fs::canonicalize(&path).map_err(|_| "404 Not Found")?;
    let file = if found.is_dir() {
        fs::canonicalize(found.join("index.html")).map_err(|_| "404 Not Found")?
    } else {
        found
    };
    if file.starts_with(root) && file.is_file() {
        Ok(file)
    } else {
        Err("404 Not Found")
    }
}

/// Decodes `%XX` escapes. `None` for a broken escape or bytes that are not UTF-8.
fn percent_decode(raw: &str) -> Option<String> {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = hex_value(*bytes.get(index + 1)?)?;
            let low = hex_value(*bytes.get(index + 2)?)?;
            out.push(high * 16 + low);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    (byte as char).to_digit(16).map(|digit| digit as u8)
}

/// Content type of the extensions the dashboard serves. Other files are not served.
fn content_type(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    match extension.as_str() {
        "html" => Some("text/html; charset=utf-8"),
        "css" => Some("text/css; charset=utf-8"),
        "js" => Some("text/javascript; charset=utf-8"),
        "svg" => Some("image/svg+xml"),
        _ => None,
    }
}

/// Writes one response with the common headers. A HEAD response keeps the
/// `Content-Length` of the body but sends no body.
fn write_reply(
    stream: &mut TcpStream,
    status: &str,
    extra: &[(&str, &str)],
    body: &[u8],
    send_body: bool,
) -> io::Result<()> {
    let mut head = format!("HTTP/1.1 {status}\r\n");
    for (name, value) in extra {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str(&format!(
        "Content-Security-Policy: {CSP}\r\n\
         X-Content-Type-Options: nosniff\r\n\
         Cache-Control: no-store\r\n\
         Referrer-Policy: no-referrer\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n",
        body.len()
    ));
    stream.write_all(head.as_bytes())?;
    if send_body {
        stream.write_all(body)?;
    }
    stream.flush()
}
