//! `cargo xtask tokens [--check]`: makes the token CSS from DESIGN.md.
//!
//! Writes `web/dashboard/login/tokens.css`, `web/dashboard/app/tokens.css`, and
//! the token block of the mockup between its markers. It also writes the copies
//! of app components and styles that the sign-in page loads (see `shared`) and
//! the copies of the font files (see `fonts`). `--check` compares those files
//! and runs the screen CSS check (docs/conventions/frontend.md section 2).

pub(crate) mod fonts;
mod front_matter;
mod screen_css;
mod shared;

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::io::Write;
use std::path::Path;

use front_matter::Value;

use crate::Error;
use crate::cargo;

const DESIGN_PATH: &str = "docs/design/DESIGN.md";
const MOCKUP_PATH: &str = "docs/design/mockups/dashboard.html";
const OUTPUT_PATHS: [&str; 2] = [
    "web/dashboard/login/tokens.css",
    "web/dashboard/app/tokens.css",
];
const DASHBOARD_DIR: &str = "web/dashboard";
const MOCKUP_START: &str = "/* DESIGN-TOKENS:START */";
const MOCKUP_END: &str = "/* DESIGN-TOKENS:END */";
/// Top-level keys that are read from DESIGN.md but never exported to CSS.
const IGNORED_KEYS: [&str; 4] = ["version", "name", "description", "components"];
const TYPE_KEYS: [(&str, &str); 6] = [
    ("fontFamily", "family"),
    ("fontSize", "size"),
    ("fontWeight", "weight"),
    ("lineHeight", "line"),
    ("letterSpacing", "tracking"),
    ("fontFeature", "feature"),
];
/// Keys every typography step must have. `fontFeature` is optional.
const REQUIRED_TYPE_KEYS: [&str; 5] = [
    "fontFamily",
    "fontSize",
    "fontWeight",
    "lineHeight",
    "letterSpacing",
];

/// One custom property, with its name (including `--`) and verbatim value.
pub struct Token {
    pub name: String,
    pub value: String,
}

/// Runs the command. `Ok(false)` means `--check` found a difference.
pub fn run(check: bool) -> Result<bool, Error> {
    let root = cargo::metadata()?.root;
    let design = read_text(&root.join(DESIGN_PATH))?;
    let tokens = tokens_from(&design)?;
    let css = render_css(&tokens, fonts::TOKEN_FILE_BASE);
    let mockup_css = render_css(&tokens, fonts::MOCKUP_BASE);
    if check {
        check_outputs(&root, &css, &mockup_css, &tokens)
    } else {
        write_outputs(&root, &css, &mockup_css)?;
        Ok(true)
    }
}

/// Reads the exported keys of the DESIGN.md front matter into tokens, in source order.
fn tokens_from(design: &str) -> Result<Vec<Token>, Error> {
    let entries = front_matter::parse_document(design)?;
    let mut tokens = Vec::new();
    for (key, value) in &entries {
        if IGNORED_KEYS.contains(&key.as_str()) {
            continue;
        }
        match (key.as_str(), value) {
            ("colors", Value::Map(map)) => push_flat(&mut tokens, "color", "colors", map)?,
            ("spacing", Value::Map(map)) => push_flat(&mut tokens, "space", "spacing", map)?,
            ("rounded", Value::Map(map)) => push_flat(&mut tokens, "radius", "rounded", map)?,
            ("typography", Value::Map(steps)) => push_typography(&mut tokens, steps)?,
            _ => {
                return Err(Error::new(format!(
                    "DESIGN.md: 허용하지 않는 키이거나 형식이 맞지 않는다: {key}"
                )));
            }
        }
    }
    Ok(tokens)
}

/// `colors.<name>`, `spacing.<name>`, `rounded.<name>` become `--<prefix>-<name>`.
fn push_flat(
    tokens: &mut Vec<Token>,
    prefix: &str,
    group: &str,
    map: &[(String, Value)],
) -> Result<(), Error> {
    for (name, value) in map {
        let path = format!("{group}.{name}");
        tokens.push(Token {
            name: format!("--{prefix}-{name}"),
            value: scalar_value(&path, value)?,
        });
    }
    Ok(())
}

/// `typography.<step>.<key>` becomes `--type-<step>-<suffix>`.
fn push_typography(tokens: &mut Vec<Token>, steps: &[(String, Value)]) -> Result<(), Error> {
    for (step, value) in steps {
        let Value::Map(props) = value else {
            return Err(Error::new(format!(
                "DESIGN.md: typography.{step}는 키와 값의 묶음이어야 한다"
            )));
        };
        for required in REQUIRED_TYPE_KEYS {
            if !props.iter().any(|(key, _)| key == required) {
                return Err(Error::new(format!(
                    "DESIGN.md: typography.{step}에 {required}가 없다"
                )));
            }
        }
        for (key, prop) in props {
            let Some((_, suffix)) = TYPE_KEYS.iter().find(|(known, _)| *known == key.as_str())
            else {
                return Err(Error::new(format!(
                    "DESIGN.md: typography.{step}에 허용하지 않는 키가 있다: {key}"
                )));
            };
            let path = format!("typography.{step}.{key}");
            tokens.push(Token {
                name: format!("--type-{step}-{suffix}"),
                value: scalar_value(&path, prop)?,
            });
        }
    }
    Ok(())
}

/// A token value is copied as is. It must be a non-empty scalar with no CSS
/// structure characters, so it cannot break out of its declaration.
fn scalar_value(path: &str, value: &Value) -> Result<String, Error> {
    let Value::Scalar(text) = value else {
        return Err(Error::new(format!("DESIGN.md: {path}는 값이어야 한다")));
    };
    if text.is_empty() || text.contains([';', '{', '}']) {
        return Err(Error::new(format!(
            "DESIGN.md: {path}의 값이 CSS 값으로 맞지 않는다"
        )));
    }
    Ok(text.clone())
}

/// The generated file: one comment line with the source, one `:root` block,
/// then the `@font-face` rules. `font_base` is the font directory in `url()`.
fn render_css(tokens: &[Token], font_base: &str) -> String {
    let mut out = format!("/* Generated file. Do not edit. Source: {DESIGN_PATH} */\n");
    out.push_str(":root {\n");
    for token in tokens {
        out.push_str(&format!("  {}: {};\n", token.name, token.value));
    }
    out.push_str("}\n");
    out.push_str(&fonts::font_faces(font_base));
    out
}

fn write_outputs(root: &Path, css: &str, mockup_css: &str) -> Result<(), Error> {
    for relative in OUTPUT_PATHS {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, css)?;
    }
    shared::write(root)?;
    fonts::write(root)?;
    let mockup_path = root.join(MOCKUP_PATH);
    let mockup = read_text(&mockup_path)?;
    let updated = replace_mockup_block(&mockup, mockup_css)?;
    if updated != mockup {
        std::fs::write(&mockup_path, updated)?;
    }
    Ok(())
}

/// Compares the generated CSS with both token files and the mockup block, the
/// copies with their sources, and runs the screen CSS rules. Every difference
/// is printed. Returns `Ok(false)` if any exists.
fn check_outputs(
    root: &Path,
    css: &str,
    mockup_css: &str,
    tokens: &[Token],
) -> Result<bool, Error> {
    let mut out = std::io::stdout().lock();
    let mut problems: Vec<String> = Vec::new();
    for relative in OUTPUT_PATHS {
        let actual = read_text(&root.join(relative))?;
        if let Some(message) = first_difference(relative, css, &actual) {
            problems.push(message);
        }
    }
    let mockup = read_text(&root.join(MOCKUP_PATH))?;
    let block = mockup_block(&mockup)?;
    if let Some(message) = first_difference(MOCKUP_PATH, mockup_css, &block) {
        problems.push(message);
    }
    let mut files = Vec::new();
    collect_screen_css(&root.join(DASHBOARD_DIR), root, &mut files)?;
    let defined: BTreeSet<String> = tokens.iter().map(|token| token.name.clone()).collect();
    problems.extend(shared::check(root)?);
    problems.extend(fonts::check(root)?);
    problems.extend(screen_css::check(&files, &defined));
    for problem in &problems {
        writeln!(out, "{problem}")?;
    }
    let passed = problems.is_empty();
    if passed {
        writeln!(out, "tokens --check: 통과")?;
    } else {
        writeln!(out, "tokens --check: 실패 ({}건)", problems.len())?;
    }
    Ok(passed)
}

/// Finds the first line that differs. Returns `None` when the texts are equal.
/// Lines are compared with `split('\n')`, so a missing final newline also counts.
fn first_difference(label: &str, expected: &str, actual: &str) -> Option<String> {
    if expected == actual {
        return None;
    }
    let expected_lines: Vec<&str> = expected.split('\n').collect();
    let actual_lines: Vec<&str> = actual.split('\n').collect();
    let count = expected_lines.len().max(actual_lines.len());
    for index in 0..count {
        let expected_line = expected_lines.get(index).copied();
        let actual_line = actual_lines.get(index).copied();
        if expected_line != actual_line {
            return Some(format!(
                "{label}:{}: 생성 결과와 다르다 (생성: {}, 파일: {})",
                index + 1,
                expected_line.unwrap_or("<없음>"),
                actual_line.unwrap_or("<없음>")
            ));
        }
    }
    None
}

/// Positions of the start and end marker lines in the mockup.
fn marker_lines(mockup: &str) -> Result<(usize, usize), Error> {
    let lines: Vec<&str> = mockup.split('\n').collect();
    let starts: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim_end() == MOCKUP_START)
        .map(|(index, _)| index)
        .collect();
    let ends: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim_end() == MOCKUP_END)
        .map(|(index, _)| index)
        .collect();
    match (starts.as_slice(), ends.as_slice()) {
        ([start], [end]) if start < end => Ok((*start, *end)),
        _ => Err(Error::new(format!(
            "{MOCKUP_PATH}: 토큰 표식 `{MOCKUP_START}`과 `{MOCKUP_END}`이 한 번씩 있어야 한다"
        ))),
    }
}

/// The text between the mockup markers, with a final newline.
fn mockup_block(mockup: &str) -> Result<String, Error> {
    let (start, end) = marker_lines(mockup)?;
    let lines: Vec<&str> = mockup.split('\n').collect();
    Ok(format!("{}\n", lines[start + 1..end].join("\n")))
}

/// Replaces the lines between the mockup markers with the generated CSS.
fn replace_mockup_block(mockup: &str, css: &str) -> Result<String, Error> {
    let (start, end) = marker_lines(mockup)?;
    let lines: Vec<&str> = mockup.split('\n').collect();
    let css_lines: Vec<&str> = css.trim_end_matches('\n').split('\n').collect();
    let mut updated: Vec<&str> = lines[..=start].to_vec();
    updated.extend(css_lines);
    updated.extend_from_slice(&lines[end..]);
    Ok(updated.join("\n"))
}

/// Collects every `.css` file under `dir` except generated token files.
fn collect_screen_css(
    dir: &Path,
    root: &Path,
    out: &mut Vec<(String, String)>,
) -> Result<(), Error> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_screen_css(&path, root, out)?;
        } else if path.extension() == Some(OsStr::new("css"))
            && path.file_name() != Some(OsStr::new("tokens.css"))
        {
            let relative = cargo::display_path(root, &path);
            out.push((relative, read_text(&path)?));
        }
    }
    Ok(())
}

/// Reads a text file and turns CRLF into LF, so checkouts with autocrlf compare equal.
fn read_text(path: &Path) -> Result<String, Error> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| Error::new(format!("{}: 읽을 수 없다 ({error})", path.display())))?;
    Ok(text.replace("\r\n", "\n"))
}
