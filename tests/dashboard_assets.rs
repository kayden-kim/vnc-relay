//! The dashboard asset table matches the files of `web/dashboard/` (frontend
//! convention §1.2): every file under `login/` and `app/` is listed, every
//! listed path has a file, and `preview/` and `tests/` are never served.

use std::fs;
use std::path::{Path, PathBuf};

use vnc_relay_adapters::dashboard_assets::ASSETS;

/// The `Content-Type` of a served file, by its extension. A path ending in `/`
/// serves its `index.html`. Any other extension is not served.
fn content_type_of(path: &str) -> &'static str {
    if path.ends_with('/') {
        return "text/html; charset=utf-8";
    }
    match Path::new(path).extension().and_then(|ext| ext.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("woff2") => "font/woff2",
        _ => panic!("{path} has an extension that is not served"),
    }
}

/// Every file under `dir`, recursively, in sorted order.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let entries =
        fs::read_dir(dir).unwrap_or_else(|error| panic!("reads {}: {error}", dir.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|error| panic!("reads an entry of {}: {error}", dir.display()))
            .path();
        if path.is_dir() {
            files.extend(files_under(&path));
        } else {
            files.push(path);
        }
    }
    files.sort();
    files
}

/// The served path and the file on disk of every file under `login/` and
/// `app/`. `app/index.html` serves `/`, `app/<rest>` serves `/app/<rest>`, and
/// `login/index.html` serves `/login/`, and `login/<rest>` serves `/login/<rest>`.
fn disk_files() -> Vec<(String, PathBuf)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("web/dashboard");
    let mut served = Vec::new();
    for area in ["login", "app"] {
        for file in files_under(&root.join(area)) {
            let relative = file
                .strip_prefix(&root)
                .unwrap_or_else(|error| {
                    panic!("{} is under web/dashboard: {error}", file.display())
                })
                .to_string_lossy()
                .replace('\\', "/");
            // Check the extension of every file on disk, so an unserved type fails here.
            content_type_of(&relative);
            let path = if relative == "app/index.html" {
                "/".to_owned()
            } else if relative == "login/index.html" {
                "/login/".to_owned()
            } else if let Some(rest) = relative.strip_prefix("app/") {
                format!("/app/{rest}")
            } else {
                format!("/{relative}")
            };
            served.push((path, file));
        }
    }
    served
}

#[test]
fn asset_table_lists_exactly_the_dashboard_files() {
    let disk = disk_files();
    let mut on_disk: Vec<&str> = disk.iter().map(|(path, _)| path.as_str()).collect();
    on_disk.sort_unstable();
    let mut listed: Vec<&str> = ASSETS.iter().map(|asset| asset.path).collect();
    listed.sort_unstable();
    // Equal sorted lists also rule out duplicate entries in the table.
    assert_eq!(
        listed, on_disk,
        "asset table and web/dashboard files differ"
    );
}

#[test]
fn asset_table_entries_have_matching_type_and_bytes() {
    let disk = disk_files();
    for asset in ASSETS {
        let segments_are_not_preview_or_tests = asset
            .path
            .split('/')
            .all(|segment| segment != "preview" && segment != "tests");
        assert!(
            segments_are_not_preview_or_tests,
            "{} is a preview or test page and must not be served",
            asset.path
        );
        assert_eq!(
            asset.content_type,
            content_type_of(asset.path),
            "Content-Type of {}",
            asset.path
        );
        let (_, file) = disk
            .iter()
            .find(|(path, _)| path == asset.path)
            .unwrap_or_else(|| panic!("{} has no file under web/dashboard", asset.path));
        let bytes =
            fs::read(file).unwrap_or_else(|error| panic!("reads {}: {error}", file.display()));
        assert!(
            asset.body == bytes.as_slice(),
            "{} differs from {}; rebuild after editing the file",
            asset.path,
            file.display()
        );
    }
}
