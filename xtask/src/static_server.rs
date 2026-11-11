//! Loopback static file server shared by `dashboard-serve` and `help-serve`.
//!
//! It binds to `127.0.0.1` only and serves files under one directory and
//! nothing else. One connection is handled at a time. Each site sets the URL
//! prefix of its files, the page that `/` redirects to, an optional
//! Content-Security-Policy header, and an optional page for missing paths.

use std::fs;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::Error;

/// Upper bound on the request head (request line and headers).
const MAX_HEAD: usize = 8 * 1024;
const IO_TIMEOUT: Duration = Duration::from_secs(5);

/// One served directory.
pub struct Site {
    /// Canonical directory that holds the files, so path checks compare like with like.
    pub root: PathBuf,
    /// URL path that maps to `root`. It starts and ends with `/`.
    pub mount: &'static str,
    /// Where `/` redirects.
    pub home: &'static str,
    /// Value of the `Content-Security-Policy` header, if the site sends one.
    pub csp: Option<&'static str>,
    /// Page sent with status 404 for missing paths. `None` sends an empty 404.
    pub not_found: Option<PathBuf>,
}

/// Binds the loopback address. Port 0 lets the OS choose a free port.
pub fn bind(port: u16) -> Result<TcpListener, Error> {
    TcpListener::bind(("127.0.0.1", port))
        .map_err(|error| Error::new(format!("127.0.0.1:{port}에 연결하지 못했다: {error}")))
}

/// Serves `site` until the process is stopped.
pub fn run(listener: &TcpListener, site: &Site) -> Result<bool, Error> {
    for stream in listener.incoming() {
        // A failed accept stops the command. A failed connection is ignored.
        let stream = stream?;
        let _ = serve(stream, site);
    }
    Ok(true)
}

/// Handles one connection: read the request, write one response, close.
fn serve(mut stream: TcpStream, site: &Site) -> io::Result<()> {
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    let Some(head) = read_head(&mut stream)? else {
        return respond(&mut stream, site, "400 Bad Request", &[], &[], false);
    };
    let Some((method, target)) = request_line(&head) else {
        return respond(&mut stream, site, "400 Bad Request", &[], &[], false);
    };
    let send_body = match method {
        "GET" => true,
        "HEAD" => false,
        _ => {
            let allow = [("Allow", "GET, HEAD")];
            return respond(
                &mut stream,
                site,
                "405 Method Not Allowed",
                &allow,
                &[],
                false,
            );
        }
    };
    let path = target.split(['?', '#']).next().unwrap_or_default();
    match resolve(site, path) {
        Resolved::Redirect(location) => {
            let location = [("Location", location.as_str())];
            respond(&mut stream, site, "302 Found", &location, &[], false)
        }
        Resolved::Malformed => respond(&mut stream, site, "400 Bad Request", &[], &[], false),
        Resolved::Missing => not_found(&mut stream, site, send_body),
        Resolved::File(file) => match (content_type(&file), fs::read(&file)) {
            (Some(kind), Ok(body)) => {
                let kind = [("Content-Type", kind)];
                respond(&mut stream, site, "200 OK", &kind, &body, send_body)
            }
            _ => not_found(&mut stream, site, send_body),
        },
    }
}

/// Status 404 with the site's missing-path page, or with an empty body.
fn not_found(stream: &mut TcpStream, site: &Site, send_body: bool) -> io::Result<()> {
    let page = site
        .not_found
        .as_deref()
        .and_then(|path| fs::read(path).ok());
    match page {
        Some(body) => {
            let kind = [("Content-Type", "text/html; charset=utf-8")];
            respond(stream, site, "404 Not Found", &kind, &body, send_body)
        }
        None => respond(stream, site, "404 Not Found", &[], &[], false),
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
#[derive(Debug, PartialEq)]
enum Resolved {
    /// `/`, the mount without its final `/`, or a directory without a final `/`.
    Redirect(String),
    /// A file under the root.
    File(PathBuf),
    /// The path contains a forbidden sequence or escape (400).
    Malformed,
    /// Missing, outside the mount or the root, or a directory without `index.html` (404).
    Missing,
}

/// Maps a request path (query and fragment already removed) to a file under the root.
fn resolve(site: &Site, raw: &str) -> Resolved {
    if raw == "/" {
        return Resolved::Redirect(site.home.to_owned());
    }
    if site.mount.len() > 1 && raw == site.mount.trim_end_matches('/') {
        return Resolved::Redirect(site.mount.to_owned());
    }
    let Some(rest) = raw.strip_prefix(site.mount) else {
        return Resolved::Missing;
    };
    let Some(decoded) = percent_decode(rest) else {
        return Resolved::Malformed;
    };
    if decoded.contains("..") || decoded.contains(['\\', '\0', ':']) {
        return Resolved::Malformed;
    }
    let mut path = site.root.clone();
    for part in decoded.split('/').filter(|part| !part.is_empty()) {
        path.push(part);
    }
    let Ok(mut found) = fs::canonicalize(&path) else {
        return Resolved::Missing;
    };
    if found.is_dir() {
        // Relative links in an index page need the final `/`, as on GitHub Pages.
        if !raw.ends_with('/') {
            return Resolved::Redirect(format!("{raw}/"));
        }
        let Ok(index) = fs::canonicalize(found.join("index.html")) else {
            return Resolved::Missing;
        };
        found = index;
    }
    if found.starts_with(&site.root) && found.is_file() {
        Resolved::File(found)
    } else {
        Resolved::Missing
    }
}

/// Decodes `%XX` escapes. `None` for a broken escape or bytes that are not UTF-8.
pub fn percent_decode(raw: &str) -> Option<String> {
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

/// Content type for the extensions the sites serve. Other files are not served.
fn content_type(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    match extension.as_str() {
        "html" => Some("text/html; charset=utf-8"),
        "css" => Some("text/css; charset=utf-8"),
        "js" => Some("text/javascript; charset=utf-8"),
        "svg" => Some("image/svg+xml"),
        "png" => Some("image/png"),
        "woff2" => Some("font/woff2"),
        "txt" => Some("text/plain; charset=utf-8"),
        _ => None,
    }
}

/// Writes one response. `extra` holds the status-specific headers. The common
/// headers are added on every response, including errors and redirects.
fn respond<W: Write>(
    out: &mut W,
    site: &Site,
    status: &str,
    extra: &[(&str, &str)],
    body: &[u8],
    send_body: bool,
) -> io::Result<()> {
    let mut head = format!("HTTP/1.1 {status}\r\n");
    for (name, value) in extra {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    if let Some(csp) = site.csp {
        head.push_str(&format!("Content-Security-Policy: {csp}\r\n"));
    }
    head.push_str(&format!(
        "X-Content-Type-Options: nosniff\r\n\
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The xtask crate directory served under `/site/`: it has `src/main.rs`
    /// and a `src/` directory without `index.html`.
    fn site() -> Site {
        let root = fs::canonicalize(env!("CARGO_MANIFEST_DIR"))
            .unwrap_or_else(|error| panic!("canonical crate directory: {error}"));
        Site {
            root,
            mount: "/site/",
            home: "/site/home.html",
            csp: None,
            not_found: None,
        }
    }

    #[test]
    fn paths_redirect_resolve_or_fail_by_the_mount_and_root() {
        let site = site();
        let redirect = |to: &str| Resolved::Redirect(to.to_owned());
        assert_eq!(resolve(&site, "/"), redirect("/site/home.html"));
        assert_eq!(resolve(&site, "/site"), redirect("/site/"));
        assert_eq!(resolve(&site, "/site/src"), redirect("/site/src/"));
        assert_eq!(resolve(&site, "/site/src/"), Resolved::Missing);
        assert_eq!(
            resolve(&site, "/site/src/%6Dain.rs"),
            Resolved::File(site.root.join("src").join("main.rs"))
        );
        assert_eq!(resolve(&site, "/site/missing.html"), Resolved::Missing);
        assert_eq!(resolve(&site, "/other/src/main.rs"), Resolved::Missing);
        assert_eq!(resolve(&site, "/site/../Cargo.toml"), Resolved::Malformed);
        assert_eq!(
            resolve(&site, "/site/%2e%2e/Cargo.toml"),
            Resolved::Malformed
        );
        assert_eq!(resolve(&site, "/site/src%5Cmain.rs"), Resolved::Malformed);
        assert_eq!(resolve(&site, "/site/%zz"), Resolved::Malformed);
    }
}
