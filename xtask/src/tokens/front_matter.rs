//! Reads the YAML front matter of DESIGN.md.
//!
//! Only the subset that DESIGN.md uses is accepted: maps nested by two-space
//! indentation, and scalars that are either double-quoted (no `"` or `\` inside)
//! or plain (ASCII letters, digits, `.`, `_`, `%`, `/`, `-`). Anything else is
//! an error, so syntax this parser does not understand cannot reach the tokens.

use crate::Error;

/// A parsed YAML value: a scalar, or a map in source order.
pub enum Value {
    Scalar(String),
    Map(Vec<(String, Value)>),
}

/// One non-empty line of the front matter.
struct Line {
    number: usize,
    indent: usize,
    key: String,
    value: Option<String>,
}

/// Parses the front matter of a document that starts with a `---` line.
pub fn parse_document(text: &str) -> Result<Vec<(String, Value)>, Error> {
    let source: Vec<&str> = text.lines().collect();
    if source.first().copied() != Some("---") {
        return Err(Error::new("DESIGN.md: 머리말이 `---`로 시작하지 않는다"));
    }
    let end = source
        .iter()
        .skip(1)
        .position(|line| *line == "---")
        .map(|offset| offset + 1)
        .ok_or_else(|| Error::new("DESIGN.md: 머리말을 닫는 `---`가 없다"))?;
    let lines = collect_lines(&source[1..end], 2)?;
    let mut pos = 0;
    parse_map(&lines, &mut pos, 0)
}

fn at(number: usize, message: &str) -> Error {
    Error::new(format!("DESIGN.md:{number}: {message}"))
}

/// Splits each non-empty line into indent, key, and optional scalar value.
/// `first_number` is the file line number of `lines[0]`.
fn collect_lines(lines: &[&str], first_number: usize) -> Result<Vec<Line>, Error> {
    let mut out = Vec::new();
    for (offset, raw) in lines.iter().enumerate() {
        let number = first_number + offset;
        if raw.trim().is_empty() || raw.trim_start().starts_with('#') {
            continue;
        }
        if raw.contains('\t') {
            return Err(at(number, "탭 문자는 쓸 수 없다"));
        }
        let indent = raw.len() - raw.trim_start_matches(' ').len();
        if indent % 2 != 0 {
            return Err(at(number, "들여쓰기는 두 칸 단위여야 한다"));
        }
        let content = raw.trim();
        let Some(colon) = content.find(':') else {
            return Err(at(number, "키 뒤에 콜론이 없다"));
        };
        let key = &content[..colon];
        if key.is_empty()
            || !key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(at(number, "키 이름 규칙에 맞지 않는다"));
        }
        let rest = content[colon + 1..].trim();
        let value = if rest.is_empty() {
            None
        } else {
            Some(parse_scalar(rest).map_err(|message| at(number, message))?)
        };
        out.push(Line {
            number,
            indent,
            key: key.to_string(),
            value,
        });
    }
    Ok(out)
}

/// Accepts a double-quoted scalar (inner text copied as is) or a plain scalar.
fn parse_scalar(rest: &str) -> Result<String, &'static str> {
    if let Some(inner) = rest.strip_prefix('"') {
        let Some(inner) = inner.strip_suffix('"') else {
            return Err("따옴표가 닫히지 않았다");
        };
        if inner.contains('"') || inner.contains('\\') {
            return Err("지원하지 않는 따옴표 안 문자가 있다");
        }
        return Ok(inner.to_string());
    }
    if rest
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '%' | '/' | '-'))
    {
        return Ok(rest.to_string());
    }
    Err("지원하지 않는 값 형식이다")
}

fn parse_map(
    lines: &[Line],
    pos: &mut usize,
    indent: usize,
) -> Result<Vec<(String, Value)>, Error> {
    let mut entries: Vec<(String, Value)> = Vec::new();
    while let Some(line) = lines.get(*pos) {
        if line.indent < indent {
            break;
        }
        if line.indent > indent {
            return Err(at(line.number, "들여쓰기가 맞지 않는다"));
        }
        if entries.iter().any(|(key, _)| *key == line.key) {
            return Err(at(line.number, "같은 키가 두 번 나왔다"));
        }
        *pos += 1;
        let value = match &line.value {
            Some(text) => {
                if lines.get(*pos).is_some_and(|next| next.indent > indent) {
                    return Err(at(line.number, "값 뒤에 들여쓴 줄이 올 수 없다"));
                }
                Value::Scalar(text.clone())
            }
            None => match lines.get(*pos) {
                Some(next) if next.indent == indent + 2 => {
                    Value::Map(parse_map(lines, pos, indent + 2)?)
                }
                _ => return Err(at(line.number, "값 없는 키 뒤에 들여쓴 줄이 없다")),
            },
        };
        entries.push((line.key.clone(), value));
    }
    Ok(entries)
}
