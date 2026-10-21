//! Screen CSS checks from docs/conventions/frontend.md section 2.2.
//!
//! Screen CSS may use tokens only: no hex colours, `rgb(`, `hsl(`, `url(`, or
//! `@import`; no `px`, `em`, or `rem` outside `@media` preludes; `font-weight`
//! only as a token variable or `inherit`; and every `var(--x)` must be defined.
//! Comments are ignored. Each problem is `path:line: message`.

use std::collections::BTreeSet;

/// Checks `(path, text)` pairs. `tokens` holds the names from the generated token
/// file; custom properties declared in the screen CSS count as defined too.
pub fn check(files: &[(String, String)], tokens: &BTreeSet<String>) -> Vec<String> {
    let stripped: Vec<String> = files.iter().map(|(_, text)| strip_comments(text)).collect();
    let mut defined = tokens.clone();
    for text in &stripped {
        collect_declared(text, &mut defined);
    }
    let mut problems = Vec::new();
    for ((path, _), text) in files.iter().zip(&stripped) {
        check_colors_and_functions(path, text, &mut problems);
        check_declarations(path, text, &mut problems);
        check_variables(path, text, &defined, &mut problems);
    }
    problems
}

/// Blanks out `/* … */` comments, keeping newlines so line numbers stay right.
fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    loop {
        let Some(start) = rest.find("/*") else {
            out.push_str(rest);
            return out;
        };
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let (comment_end, next) = match after.find("*/") {
            Some(end) => (start + 2 + end + 2, &after[end + 2..]),
            None => (rest.len(), ""),
        };
        out.extend(rest[start..comment_end].chars().map(blank));
        rest = next;
    }
}

fn blank(c: char) -> char {
    if c == '\n' { '\n' } else { ' ' }
}

/// Adds every `--name:` declaration found in `text` to `defined`.
fn collect_declared(text: &str, defined: &mut BTreeSet<String>) {
    for (offset, _) in text.match_indices("--") {
        let starts_declaration = match text[..offset].chars().next_back() {
            None => true,
            Some(c) => c.is_whitespace() || matches!(c, '{' | ';'),
        };
        if !starts_declaration {
            continue;
        }
        let name: String = text[offset + 2..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        let after_name = text[offset + 2 + name.len()..].trim_start();
        if !name.is_empty() && after_name.starts_with(':') {
            defined.insert(format!("--{name}"));
        }
    }
}

/// 1-based line number of byte `offset` in `text`.
fn line_of(text: &str, offset: usize) -> usize {
    text[..offset].matches('\n').count() + 1
}

/// Hex colours (3, 4, 6, or 8 digits), `rgb(`, `rgba(`, `hsl(`, `hsla(`, `url(`, `@import`.
fn check_colors_and_functions(path: &str, text: &str, problems: &mut Vec<String>) {
    for (offset, _) in text.match_indices('#') {
        let digits = text[offset + 1..]
            .chars()
            .take_while(char::is_ascii_hexdigit)
            .count();
        let ident_follows = text[offset + 1 + digits..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if matches!(digits, 3 | 4 | 6 | 8) && !ident_follows {
            problems.push(format!(
                "{path}:{}: 16진수 색은 토큰 변수로 쓴다",
                line_of(text, offset)
            ));
        }
    }
    let lower = text.to_ascii_lowercase();
    for needle in ["rgb(", "rgba(", "hsl(", "hsla(", "url(", "@import"] {
        for (offset, _) in lower.match_indices(needle) {
            problems.push(format!(
                "{path}:{}: 금지된 문법이다: {needle}",
                line_of(text, offset)
            ));
        }
    }
}

/// Splits the text at `{`, `;`, and `}`. Lengths are checked in selectors and
/// declarations outside `@media`. `font-weight` is checked in every declaration.
fn check_declarations(path: &str, text: &str, problems: &mut Vec<String>) {
    let mut media_stack: Vec<bool> = Vec::new();
    let mut segment_start = 0;
    for (offset, c) in text.char_indices() {
        if !matches!(c, '{' | ';' | '}') {
            continue;
        }
        let segment = &text[segment_start..offset];
        let in_media = media_stack.iter().any(|is_media| *is_media);
        match c {
            '{' => {
                let is_media = segment.trim_start().starts_with("@media");
                if !is_media && !in_media {
                    check_lengths(path, text, segment_start, segment, problems);
                }
                media_stack.push(is_media);
            }
            _ => {
                if !in_media {
                    check_lengths(path, text, segment_start, segment, problems);
                }
                check_font_weight(path, text, segment_start, segment, problems);
                if c == '}' {
                    media_stack.pop();
                }
            }
        }
        segment_start = offset + 1;
    }
}

/// Finds `px`, `em`, and `rem` after a digit with no identifier characters after them.
fn check_lengths(path: &str, text: &str, base: usize, segment: &str, problems: &mut Vec<String>) {
    let bytes = segment.as_bytes();
    for unit in ["px", "em", "rem"] {
        for (position, _) in segment.match_indices(unit) {
            let digit_before = position
                .checked_sub(1)
                .and_then(|index| bytes.get(index))
                .is_some_and(u8::is_ascii_digit);
            let ident_after = bytes
                .get(position + unit.len())
                .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'-' || *b == b'_');
            if digit_before && !ident_after {
                problems.push(format!(
                    "{path}:{}: 길이 리터럴 {unit}은 미디어 쿼리 밖에서 쓰지 않는다",
                    line_of(text, base + position)
                ));
            }
        }
    }
}

fn check_font_weight(
    path: &str,
    text: &str,
    base: usize,
    segment: &str,
    problems: &mut Vec<String>,
) {
    let Some(rest) = segment.trim_start().strip_prefix("font-weight") else {
        return;
    };
    let Some(value) = rest.trim_start().strip_prefix(':') else {
        return;
    };
    let value = value.trim();
    let allowed =
        value == "inherit" || (value.starts_with("var(--type-") && value.ends_with("-weight)"));
    if !allowed {
        problems.push(format!(
            "{path}:{}: font-weight는 토큰 변수나 inherit만 쓴다",
            line_of(text, base)
        ));
    }
}

/// Every `var(--name` must name a defined custom property.
fn check_variables(path: &str, text: &str, defined: &BTreeSet<String>, problems: &mut Vec<String>) {
    for (offset, _) in text.match_indices("var(") {
        let rest = &text[offset + 4..];
        let end = rest.find([',', ')']).unwrap_or(rest.len());
        let name = rest[..end].trim();
        if !defined.contains(name) {
            problems.push(format!(
                "{path}:{}: 정의되지 않은 변수다: {name}",
                line_of(text, offset)
            ));
        }
    }
}
