//! `cargo xtask help-serve`: serves the built help site for local preview.
//!
//! The site is served under `/vnc-relay/`, the GitHub Pages path, and a
//! missing path gets the site's `404.html` with status 404, as on GitHub
//! Pages. No CSP header is sent: GitHub Pages sends none, so only the pages'
//! own `meta` policy applies, here as in production.

use std::fs;
use std::io::{self, Write};

use crate::static_server::{self, Site};
use crate::{Error, cargo};

/// URL path of the site, and where `/` redirects.
const MOUNT: &str = "/vnc-relay/";

/// Serves `<target>/help-site/` on `127.0.0.1:port`. Runs until the process is stopped.
pub fn run(port: u16) -> Result<bool, Error> {
    let workspace = cargo::metadata()?;
    let built = workspace.target_dir.join("help-site");
    if !built.join("index.html").is_file() {
        return Err(Error::new(format!(
            "{}에 도움말 사이트가 없다. 먼저 `cargo xtask help-site`를 실행한다",
            built.display()
        )));
    }
    // Canonical root, so path checks compare like with like.
    let root = fs::canonicalize(&built)?;
    let site = Site {
        not_found: Some(root.join("404.html")),
        root,
        mount: MOUNT,
        home: MOUNT,
        csp: None,
    };
    let listener = static_server::bind(port)?;
    let port = listener.local_addr()?.port();
    {
        let mut out = io::stdout().lock();
        writeln!(out, "도움말: http://127.0.0.1:{port}{MOUNT}")?;
        writeln!(out, "종료하려면 Ctrl+C를 누른다.")?;
    }
    static_server::run(&listener, &site)
}
