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
/// `Content-Type` of a font file.
const WOFF2: &str = "font/woff2";

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
        path: "/login/fonts/Pretendard-Regular.subset.woff2",
        content_type: WOFF2,
        body: include_bytes!("../../../web/dashboard/login/fonts/Pretendard-Regular.subset.woff2"),
    },
    Asset {
        path: "/login/fonts/Pretendard-Bold.subset.woff2",
        content_type: WOFF2,
        body: include_bytes!("../../../web/dashboard/login/fonts/Pretendard-Bold.subset.woff2"),
    },
    Asset {
        path: "/app/tokens.css",
        content_type: CSS,
        body: include_bytes!("../../../web/dashboard/app/tokens.css"),
    },
    Asset {
        path: "/app/fonts/Pretendard-Regular.subset.woff2",
        content_type: WOFF2,
        body: include_bytes!("../../../web/dashboard/app/fonts/Pretendard-Regular.subset.woff2"),
    },
    Asset {
        path: "/app/fonts/Pretendard-Bold.subset.woff2",
        content_type: WOFF2,
        body: include_bytes!("../../../web/dashboard/app/fonts/Pretendard-Bold.subset.woff2"),
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
    Asset {
        path: "/",
        content_type: HTML,
        body: include_bytes!("../../../web/dashboard/app/index.html"),
    },
    Asset {
        path: "/app/app.css",
        content_type: CSS,
        body: include_bytes!("../../../web/dashboard/app/app.css"),
    },
    Asset {
        path: "/app/main.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/main.js"),
    },
    Asset {
        path: "/app/operations.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/operations.js"),
    },
    Asset {
        path: "/app/panels.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/panels.js"),
    },
    Asset {
        path: "/app/api.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/api.js"),
    },
    Asset {
        path: "/app/model.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/model.js"),
    },
    Asset {
        path: "/app/format.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/format.js"),
    },
    Asset {
        path: "/app/copy.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/copy.js"),
    },
    Asset {
        path: "/app/language.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/language.js"),
    },
    Asset {
        path: "/app/query.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/query.js"),
    },
    Asset {
        path: "/app/shell.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/shell.js"),
    },
    Asset {
        path: "/app/session.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/session.js"),
    },
    Asset {
        path: "/app/settings.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/settings.js"),
    },
    Asset {
        path: "/app/settings-model.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/settings-model.js"),
    },
    Asset {
        path: "/app/router.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/router.js"),
    },
    Asset {
        path: "/app/logs.js",
        content_type: JS,
        body: include_bytes!("../../../web/dashboard/app/logs.js"),
    },
];

/// The file served at `path`. Only an exact match is served.
pub fn find_asset(path: &str) -> Option<&'static Asset> {
    ASSETS.iter().find(|asset| asset.path == path)
}
