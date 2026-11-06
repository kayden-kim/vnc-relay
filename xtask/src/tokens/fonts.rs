//! The dashboard font files (docs/conventions/frontend.md section 2.4).
//!
//! The sources are in `web/dashboard/fonts/`, which is never served. `cargo
//! xtask tokens` copies each file, byte for byte, to `app/fonts/` and
//! `login/fonts/`, and the token CSS gets one `@font-face` rule per file.
//! `--check` reports a copy that differs from its source and a file in a copy
//! directory that `FONT_FACES` does not name.

use std::path::{Path, PathBuf};

use super::DASHBOARD_DIR;
use crate::Error;
use crate::cargo;

/// The family name in the `@font-face` rules. DESIGN.md puts it first in the stacks.
const FAMILY: &str = "Pretendard";

/// File name and `font-weight` of each font file (DESIGN.md Typography, Q139).
const FONT_FACES: [(&str, u16); 2] = [
    ("Pretendard-Regular.subset.woff2", 400),
    ("Pretendard-Bold.subset.woff2", 700),
];

/// The served areas under `web/dashboard/` that get a copy of every font file.
const COPY_AREAS: [&str; 2] = ["app", "login"];

/// The license of the font files, relative to the repository root. `cargo
/// xtask notices` puts its full text into the distribution notices.
pub(crate) const LICENSE_PATH: &str = "web/dashboard/fonts/LICENSE.txt";
/// The name, version, and license shown in the distribution notices.
pub(crate) const NOTICE_TITLE: &str = "Pretendard 1.3.9 (web/dashboard/fonts) | OFL-1.1";

/// `url()` base in the token files: the `fonts/` directory next to them.
pub(super) const TOKEN_FILE_BASE: &str = "fonts";
/// `url()` base in the mockup block: the sources, seen from `docs/design/mockups/`.
pub(super) const MOCKUP_BASE: &str = "../../../web/dashboard/fonts";

/// One `@font-face` rule per font file. `base` is the directory in `url()`.
pub(super) fn font_faces(base: &str) -> String {
    let mut out = String::new();
    for (file, weight) in FONT_FACES {
        out.push_str(&format!(
            "@font-face {{\n  font-family: {FAMILY};\n  font-style: normal;\n  font-weight: {weight};\n  font-display: swap;\n  src: url(\"{base}/{file}\") format(\"woff2\");\n}}\n"
        ));
    }
    out
}

fn source_path(root: &Path, file: &str) -> PathBuf {
    root.join(DASHBOARD_DIR).join("fonts").join(file)
}

fn copy_dir(root: &Path, area: &str) -> PathBuf {
    root.join(DASHBOARD_DIR).join(area).join("fonts")
}

/// Writes every copy from its source. Called by `cargo xtask tokens`.
pub(super) fn write(root: &Path) -> Result<(), Error> {
    for area in COPY_AREAS {
        std::fs::create_dir_all(copy_dir(root, area))?;
    }
    for (file, _) in FONT_FACES {
        let bytes = read_bytes(&source_path(root, file))?;
        for area in COPY_AREAS {
            std::fs::write(copy_dir(root, area).join(file), &bytes)?;
        }
    }
    Ok(())
}

/// Compares every copy with its source and reports files that the list does
/// not name. Each problem is one line.
pub(super) fn check(root: &Path) -> Result<Vec<String>, Error> {
    let mut problems = Vec::new();
    for (file, _) in FONT_FACES {
        let source = read_bytes(&source_path(root, file))?;
        for area in COPY_AREAS {
            let path = copy_dir(root, area).join(file);
            if read_bytes(&path)? != source {
                problems.push(format!(
                    "{}: 원본과 바이트가 다르다",
                    cargo::display_path(root, &path)
                ));
            }
        }
    }
    for area in COPY_AREAS {
        let dir = copy_dir(root, area);
        if !dir.exists() {
            continue;
        }
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            let listed = path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| FONT_FACES.iter().any(|(file, _)| *file == name));
            if !listed || path.is_dir() {
                problems.push(format!(
                    "{}: 복사 목록에 없는 파일이다",
                    cargo::display_path(root, &path)
                ));
            }
        }
    }
    Ok(problems)
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, Error> {
    std::fs::read(path)
        .map_err(|error| Error::new(format!("{}: 읽을 수 없다 ({error})", path.display())))
}
