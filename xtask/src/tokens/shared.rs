//! Copies of the app components and styles that the sign-in page loads.
//!
//! `cargo xtask tokens` writes each copy under `web/dashboard/login/`, at the
//! same relative path as its source under `web/dashboard/app/`. A copy is the
//! source text with one header line in front. `--check` reports a copy that
//! differs from its source, and a file in a copy directory that the list does
//! not name.

use std::path::{Path, PathBuf};

use super::{DASHBOARD_DIR, first_difference, read_text};
use crate::Error;
use crate::cargo;

/// Files under `web/dashboard/app/` that the sign-in page loads. Each one is
/// copied to the same relative path under `web/dashboard/login/`.
const SHARED_FILES: [&str; 7] = [
    "components/icons.js",
    "components/alert.js",
    "components/button.js",
    "components/text-field.js",
    "components/language-select.js",
    "styles/base.css",
    "styles/components.css",
];

/// Directories under `web/dashboard/login/` whose files all come from `SHARED_FILES`.
const COPY_DIRS: [&str; 2] = ["components", "styles"];

/// The source path shown in the header, relative to the repository root.
fn source_label(name: &str) -> String {
    format!("{DASHBOARD_DIR}/app/{name}")
}

/// The copy text: a header line naming the source, then the source text as is.
fn render(source_label: &str, text: &str) -> String {
    let note = format!("생성 파일이다. 직접 고치지 않는다. 원본: {source_label}");
    let header = if source_label.ends_with(".css") {
        format!("/* {note} */")
    } else {
        format!("// {note}")
    };
    format!("{header}\n{text}")
}

fn app_path(root: &Path, name: &str) -> PathBuf {
    root.join(DASHBOARD_DIR).join("app").join(name)
}

fn login_path(root: &Path, name: &str) -> PathBuf {
    root.join(DASHBOARD_DIR).join("login").join(name)
}

/// Writes every copy from its source. Called by `cargo xtask tokens`.
pub(super) fn write(root: &Path) -> Result<(), Error> {
    for name in SHARED_FILES {
        let text = read_text(&app_path(root, name))?;
        let path = login_path(root, name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, render(&source_label(name), &text))?;
    }
    Ok(())
}

/// Compares every copy with its source. Also reports files in the copy
/// directories that `SHARED_FILES` does not list. Each problem is one line.
pub(super) fn check(root: &Path) -> Result<Vec<String>, Error> {
    let mut problems = Vec::new();
    for name in SHARED_FILES {
        let expected = render(&source_label(name), &read_text(&app_path(root, name))?);
        let label = format!("{DASHBOARD_DIR}/login/{name}");
        let actual = read_text(&login_path(root, name))?;
        if let Some(message) = first_difference(&label, &expected, &actual) {
            problems.push(message);
        }
    }
    let listed: Vec<String> = SHARED_FILES
        .iter()
        .map(|name| cargo::display_path(root, &login_path(root, name)))
        .collect();
    for dir in COPY_DIRS {
        let mut found = Vec::new();
        let copy_dir = root.join(DASHBOARD_DIR).join("login").join(dir);
        collect_files(&copy_dir, root, &mut found)?;
        for relative in found {
            if !listed.contains(&relative) {
                problems.push(format!("{relative}: 복사 목록에 없는 파일이다"));
            }
        }
    }
    Ok(problems)
}

/// Collects every file under `dir`, recursively. A missing directory has none.
fn collect_files(dir: &Path, root: &Path, out: &mut Vec<String>) -> Result<(), Error> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_files(&path, root, out)?;
        } else {
            out.push(cargo::display_path(root, &path));
        }
    }
    Ok(())
}
