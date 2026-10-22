//! The dashboard files that the management listener serves (management-api-v1
//! §1.11, frontend convention §1.2). `include_bytes!` puts each file into the
//! executable. This table is the only list of served paths. The match test
//! `tests/dashboard_assets.rs` checks it against `web/dashboard/`.

/// `Content-Type` of an HTML page.
const HTML: &str = "text/html; charset=utf-8";
/// `Content-Type` of a stylesheet.
const CSS: &str = "text/css; charset=utf-8";
/// `Content-Type` of a module script.
const JS: &str = "text/javascript; charset=utf-8";

/// A file that the management listener serves.
pub struct Asset {
    /// The request path, compared exactly (for example `/app/tokens.css`).
    pub path: &'static str,
    /// The `Content-Type` header value.
    pub content_type: &'static str,
    /// The file bytes, built into the executable.
    pub body: &'static [u8],
}

/// The served files. Files under `preview/` and `tests/` are never listed, and
/// a file that is not listed here is not in the executable.
pub const ASSETS: &[Asset] = &[
    Asset {
        path: "/login/tokens.css",
        content_type: CSS,
        body: include_bytes!("../../../web/dashboard/login/tokens.css"),
    },
    Asset {
        path: "/login/",
        content_type: HTML,
        body: include_bytes!("../../../web/dashboard/login/index.html"),
    },
    Asset {
        path: "/login/login.css",
        content_type: CSS,
        body: include_bytes!("../../../web/dashboard/login/login.css"),
    },
    Asset {
        path: "/login/main.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/login/main.js"),
    },
    Asset {
        path: "/login/login.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/login/login.js"),
    },
    Asset {
        path: "/login/language.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/login/language.js"),
    },
    Asset {
        path: "/login/styles/base.css",
        content_type: CSS,
        body: include_bytes!("../../../web/dashboard/login/styles/base.css"),
    },
    Asset {
        path: "/login/styles/components.css",
        content_type: CSS,
        body: include_bytes!("../../../web/dashboard/login/styles/components.css"),
    },
    Asset {
        path: "/login/components/icons.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/login/components/icons.js"),
    },
    Asset {
        path: "/login/components/alert.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/login/components/alert.js"),
    },
    Asset {
        path: "/login/components/button.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/login/components/button.js"),
    },
    Asset {
        path: "/login/components/text-field.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/login/components/text-field.js"),
    },
    Asset {
        path: "/login/components/language-select.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/login/components/language-select.js"),
    },
    Asset {
        path: "/app/tokens.css",
        content_type: CSS,
        body: include_bytes!("../../../web/dashboard/app/tokens.css"),
    },
    Asset {
        path: "/app/styles/base.css",
        content_type: CSS,
        body: include_bytes!("../../../web/dashboard/app/styles/base.css"),
    },
    Asset {
        path: "/app/styles/components.css",
        content_type: CSS,
        body: include_bytes!("../../../web/dashboard/app/styles/components.css"),
    },
    Asset {
        path: "/app/components/navbar.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/components/navbar.js"),
    },
    Asset {
        path: "/app/components/card.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/components/card.js"),
    },
    Asset {
        path: "/app/components/gauge.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/components/gauge.js"),
    },
    Asset {
        path: "/app/components/dialog.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/components/dialog.js"),
    },
    Asset {
        path: "/app/components/link.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/components/link.js"),
    },
    Asset {
        path: "/app/components/data-table.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/components/data-table.js"),
    },
    Asset {
        path: "/app/components/language-select.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/components/language-select.js"),
    },
    Asset {
        path: "/app/components/stat.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/components/stat.js"),
    },
    Asset {
        path: "/app/components/alert.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/components/alert.js"),
    },
    Asset {
        path: "/app/components/badge.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/components/badge.js"),
    },
    Asset {
        path: "/app/components/text-field.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/components/text-field.js"),
    },
    Asset {
        path: "/app/components/button.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/components/button.js"),
    },
    Asset {
        path: "/app/components/icons.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/components/icons.js"),
    },
];

/// The file served at `path`. Only an exact match is served.
pub fn find_asset(path: &str) -> Option<&'static Asset> {
    ASSETS.iter().find(|asset| asset.path == path)
}
