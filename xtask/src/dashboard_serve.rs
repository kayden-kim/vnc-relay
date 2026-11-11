//! `cargo xtask dashboard-serve`: serves `web/dashboard/` for local preview.
//!
//! The shared loopback server (`static_server`) serves static files and
//! nothing else: no API and no session. Response headers follow
//! docs/contracts/management-api-v1.md §1.11 and docs/conventions/frontend.md
//! §1.2.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use crate::Error;
use crate::static_server::{self, Site};

/// The page the root URL redirects to, and the first URL the command prints.
const PREVIEW_PAGE: &str = "/preview/index.html";
const TEST_PAGE: &str = "/tests/components.html";
/// Content-Security-Policy required by docs/conventions/frontend.md §1.2 (Q37).
/// Same string as tests/dashboard_browser.rs. Keep the two in sync.
const CSP: &str = "default-src 'self'; frame-ancestors 'none'";

/// Serves the dashboard on `127.0.0.1:port`. Runs until the process is stopped.
pub fn run(port: u16) -> Result<bool, Error> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| Error::new("xtask 폴더의 상위 폴더를 찾지 못했다"))?;
    let dashboard = repo.join("web").join("dashboard");
    // Canonical root, so path checks compare like with like.
    let root = fs::canonicalize(&dashboard).map_err(|error| {
        Error::new(format!(
            "{} 폴더를 열지 못했다: {error}",
            dashboard.display()
        ))
    })?;
    let site = Site {
        root,
        mount: "/",
        home: PREVIEW_PAGE,
        csp: Some(CSP),
        not_found: None,
    };
    let listener = static_server::bind(port)?;
    let port = listener.local_addr()?.port();
    {
        let mut out = io::stdout().lock();
        writeln!(out, "미리보기: http://127.0.0.1:{port}{PREVIEW_PAGE}")?;
        writeln!(out, "테스트 페이지: http://127.0.0.1:{port}{TEST_PAGE}")?;
        writeln!(out, "종료하려면 Ctrl+C를 누른다.")?;
    }
    static_server::run(&listener, &site)
}
