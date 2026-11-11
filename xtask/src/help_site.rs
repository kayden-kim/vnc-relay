//! `cargo xtask help-site`: builds the operator help site and checks it.
//!
//! The two mdBook books (`docs/help/ko/`, `docs/help/en/`) get the shared
//! theme `web/help/theme/` in a staging directory and are built into
//! `<target>/help-site/<language>/`. The site root files, the token CSS, and
//! the font files are added next to them. docs/architectures/xtask.md describes
//! the steps and the checks.

mod checks;
mod html;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{Error, cargo, package, tokens};

/// Book languages. The first one is the reference for the chapter list check.
pub const LANGUAGES: [&str; 2] = ["ko", "en"];
/// Theme shared by both books. Each staged book gets a copy as its `theme/`.
const THEME_DIR: &str = "web/help/theme";
/// Site root files: the language choice page, the missing page, and `assets/`.
const SITE_DIR: &str = "web/help/site";
/// Version output of the only mdBook release the theme is made for
/// (user decision 2026-11-11 (Q223)).
const MDBOOK_VERSION: &str = "mdbook v0.5.4";
/// Where the install steps are, for error messages.
const INSTALL_HINT: &str =
    "docs/how-to/publish-help.md 2.1절에 따라 mdBook 0.5.4를 설치하고 MDBOOK이나 PATH를 정한다";

/// Builds and checks the site. `Ok(false)` means a check found a problem.
pub fn run() -> Result<bool, Error> {
    let workspace = cargo::metadata()?;
    let root = &workspace.root;
    let mdbook = mdbook()?;
    let stage = workspace.target_dir.join("help-stage");
    let site = workspace.target_dir.join("help-site");
    remove_dir(&stage)?;
    remove_dir(&site)?;
    for language in LANGUAGES {
        let book = stage.join(language);
        package::copy_dir(&root.join("docs").join("help").join(language), &book)?;
        package::copy_dir(&root.join(THEME_DIR), &book.join("theme"))?;
        let output = Command::new(&mdbook)
            .arg("build")
            .arg(&book)
            .arg("--dest-dir")
            .arg(site.join(language))
            .output()
            .map_err(|error| {
                Error::new(format!("{}: 실행하지 못했다 ({error})", mdbook.display()))
            })?;
        if !output.status.success() {
            return Err(Error::new(format!(
                "mdbook build {language} 실패:\n{}",
                String::from_utf8_lossy(&output.stderr).trim_end()
            )));
        }
        // mdBook writes an English 404 page into each book. The site has one
        // missing page at its root (HLP-003).
        fs::remove_file(site.join(language).join("404.html"))?;
    }
    package::copy_dir(&root.join(SITE_DIR), &site)?;
    let assets = site.join("assets");
    fs::write(assets.join("tokens.css"), tokens::token_css(root)?)?;
    tokens::fonts::copy_into(root, &assets.join("fonts"))?;
    fs::copy(
        root.join(THEME_DIR).join("favicon.svg"),
        assets.join("favicon.svg"),
    )?;

    let problems = checks::check(root, &site)?;
    let mut out = std::io::stdout().lock();
    for problem in &problems {
        writeln!(out, "{problem}")?;
    }
    if problems.is_empty() {
        writeln!(
            out,
            "help-site: 통과 ({})",
            cargo::display_path(root, &site)
        )?;
        Ok(true)
    } else {
        writeln!(out, "help-site: 실패 ({}건)", problems.len())?;
        Ok(false)
    }
}

/// The mdBook executable: `MDBOOK` when set, otherwise `mdbook` on `PATH`.
/// Its version output must be `MDBOOK_VERSION`.
fn mdbook() -> Result<PathBuf, Error> {
    let program = std::env::var_os("MDBOOK").map_or_else(|| PathBuf::from("mdbook"), PathBuf::from);
    let output = Command::new(&program)
        .arg("--version")
        .output()
        .map_err(|error| {
            Error::new(format!(
                "mdBook을 실행하지 못했다 ({}: {error}). {INSTALL_HINT}",
                program.display()
            ))
        })?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = stdout.trim();
    if !output.status.success() || version != MDBOOK_VERSION {
        return Err(Error::new(format!(
            "mdBook 0.5.4가 필요하다 ({}: `{version}`). {INSTALL_HINT}",
            program.display()
        )));
    }
    Ok(program)
}

/// Removes a directory tree. A missing directory is not an error.
fn remove_dir(path: &Path) -> Result<(), Error> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Error::new(format!(
            "{}: 지우지 못했다 ({error})",
            path.display()
        ))),
    }
}
