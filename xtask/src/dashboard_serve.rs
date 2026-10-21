//! `cargo xtask dashboard-serve`: serves `web/dashboard/` for local preview.
//!
//! The server binds to the loopback address only and serves static files
//! and nothing else: no API and no session. Response headers follow
//! docs/contracts/management-api-v1.md §1.11 and docs/conventions/frontend.md
//! §1.2. One connection is handled at a time.

use std::fs;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::Error;

/// The page the root URL redirects to, and the first URL the command prints.
const PREVIEW_PAGE: &str = "/preview/index.html";
const TEST_PAGE: &str = "/tests/components.html";
/// Upper bound on the request head (request line and headers).
const MAX_HEAD: usize = 8 * 1024;
const IO_TIMEOUT: Duration = Duration::from_secs(5);
/// Content-Security-Policy required by docs/conventions/frontend.md §1.2 (Q37).
/// Same string as tests/dashboard_browser.rs. Keep the two in sync.
const CSP: &str = "default-src 'self'; frame-ancestors 'none'";

/// Serves the dashboard on `127.0.0.1:port`. Runs until the process is stopped.
pub fn run(port: u16) -> Result<bool, Error> {
    let root = dashboard_root()?;
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|error| Error::new(format!("127.0.0.1:{port}에 연결하지 못했다: {error}")))?;
    {
        let mut out = io::stdout().lock();
        writeln!(out, "미리보기: http://127.0.0.1:{port}{PREVIEW_PAGE}")?;
        writeln!(out, "테스트 페이지: http://127.0.0.1:{port}{TEST_PAGE}")?;
        writeln!(out, "종료하려면 Ctrl+C를 누른다.")?;
    }
    for stream in listener.incoming() {
        // A failed accept stops the command. A failed connection is ignored.
        let stream = stream?;
        let _ = serve(stream, &root);
    }
    Ok(true)
}

/// `web/dashboard/` of this repository, canonicalised so path checks compare like with like.
fn dashboard_root() -> Result<PathBuf, Error> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| Error::new("xtask 폴더의 상위 폴더를 찾지 못했다"))?;
    let root = repo.join("web").join("dashboard");
    fs::canonicalize(&root)
        .map_err(|error| Error::new(format!("{} 폴더를 열지 못했다: {error}", root.display())))
}

/// Handles one connection: read the request, write one response, close.
fn serve(mut stream: TcpStream, root: &Path) -> io::Result<()> {
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    let Some(head) = read_head(&mut stream)? else {
        return respond(&mut stream, "400 Bad Request", &[], &[], false);
    };
    let Some((method, target)) = request_line(&head) else {
        return respond(&mut stream, "400 Bad Request", &[], &[], false);
    };
    let send_body = match method {
        "GET" => true,
        "HEAD" => false,
        _ => {
            let allow = [("Allow", "GET, HEAD")];
            return respond(&mut stream, "405 Method Not Allowed", &allow, &[], false);
        }
    };
    let path = target.split(['?', '#']).next().unwrap_or_default();
    match resolve(root, path) {
        Resolved::Redirect => {
            let location = [("Location", PREVIEW_PAGE)];
            respond(&mut stream, "302 Found", &location, &[], false)
        }
        Resolved::Malformed => respond(&mut stream, "400 Bad Request", &[], &[], false),
        Resolved::Missing => respond(&mut stream, "404 Not Found", &[], &[], false),
        Resolved::File(file) => match (content_type(&file), fs::read(&file)) {
            (Some(kind), Ok(body)) => {
                let kind = [("Content-Type", kind)];
                respond(&mut stream, "200 OK", &kind, &body, send_body)
            }
            _ => respond(&mut stream, "404 Not Found", &[], &[], false),
        },
    }
}

/// Reads up to the blank line that ends the head. Returns `Ok(None)` when the
/// head is longer than `MAX_HEAD` or the client closes before it ends.
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

/// Returns the method and raw request target. `None` means malformed (400).
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

/// What a request path resolves to.
enum Resolved {
    /// `/` redirects to the preview page.
    Redirect,
    /// A file under the root.
    File(PathBuf),
    /// The path contains a forbidden sequence or escape (400).
    Malformed,
    /// Missing, outside the root, or a directory without `index.html` (404).
    Missing,
}

/// Maps a request path (query and fragment already removed) to a file under `root`.
fn resolve(root: &Path, raw: &str) -> Resolved {
    if raw == "/" {
        return Resolved::Redirect;
    }
    let Some(decoded) = percent_decode(raw) else {
        return Resolved::Malformed;
    };
    if decoded.contains("..") || decoded.contains(['\\', '\0', ':']) {
        return Resolved::Malformed;
    }
    let mut path = root.to_path_buf();
    for part in decoded.split('/').filter(|part| !part.is_empty()) {
        path.push(part);
    }
    let Ok(mut found) = fs::canonicalize(&path) else {
        return Resolved::Missing;
    };
    if found.is_dir() {
        let Ok(index) = fs::canonicalize(found.join("index.html")) else {
            return Resolved::Missing;
        };
        found = index;
    }
    if found.starts_with(root) && found.is_file() {
        Resolved::File(found)
    } else {
        Resolved::Missing
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

/// Content type for the extensions the dashboard serves. Other files are not served.
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

/// Writes one response. `extra` holds the status-specific headers. The common
/// headers are added on every response, including errors and redirects.
fn respond<W: Write>(
    out: &mut W,
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
    out.write_all(head.as_bytes())?;
    if send_body {
        out.write_all(body)?;
    }
    out.flush()
}
