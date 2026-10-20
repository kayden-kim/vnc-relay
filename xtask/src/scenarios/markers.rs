//! Finds scenario markers (`/// spec: <path>:<ID>`) and the test functions they label.
//!
//! A small lexer skips comments, strings, and character literals so that braces
//! inside them do not change the module nesting.

use super::spec::is_scenario_id;

/// One `/// spec:` marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marker {
    pub line: usize,
    /// Spec path relative to `docs/specs/`, for example `relay.md`.
    pub spec: String,
    pub id: String,
}

/// A test function with at least one valid marker.
#[derive(Debug, PartialEq, Eq)]
pub struct MarkedTest {
    /// Inline modules that enclose the function in this file, outermost first.
    pub modules: Vec<String>,
    pub name: String,
    pub markers: Vec<Marker>,
}

/// A marker the report cannot use.
#[derive(Debug, PartialEq, Eq)]
pub struct BadMarker {
    pub line: usize,
    pub text: String,
    pub reason: &'static str,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Scan {
    pub tests: Vec<MarkedTest>,
    pub bad: Vec<BadMarker>,
}

const MALFORMED: &str = "표식 형식이 `/// spec: <specs/ 기준 경로>.md:<시나리오 ID>`가 아니다.";
const NOT_ON_TEST: &str = "표식이 테스트 함수(`#[test]`·`#[tokio::test]`)에 붙어 있지 않다.";

pub fn scan(source: &str) -> Scan {
    let tokens = lex(source);
    let mut scan = Scan::default();
    let mut scopes: Vec<Option<&str>> = Vec::new();
    let mut pending: Vec<(usize, &str)> = Vec::new();
    let mut is_test = false;
    let mut k = 0;
    while k < tokens.len() {
        match tokens[k] {
            Token::Doc { line, text } => {
                if text.trim_start().starts_with("spec:") {
                    pending.push((line, text));
                }
                k += 1;
            }
            Token::Punct(b'#') => {
                let (path, next) = attribute(&tokens, k + 1);
                if path.last() == Some(&"test") {
                    is_test = true;
                }
                k = next;
            }
            Token::Ident("mod") => match (tokens.get(k + 1), tokens.get(k + 2)) {
                (Some(Token::Ident(name)), Some(Token::Punct(b'{'))) => {
                    flush(&mut pending, &mut is_test, &mut scan);
                    scopes.push(Some(name));
                    k += 3;
                }
                _ => k += 1,
            },
            Token::Ident("fn") => match tokens.get(k + 1) {
                Some(Token::Ident(name)) if is_test && !pending.is_empty() => {
                    let mut markers = Vec::new();
                    for (line, text) in pending.drain(..) {
                        match parse_marker(text) {
                            Some((spec, id)) => markers.push(Marker { line, spec, id }),
                            None => scan.bad.push(bad(line, text, MALFORMED)),
                        }
                    }
                    if !markers.is_empty() {
                        scan.tests.push(MarkedTest {
                            modules: scopes.iter().flatten().map(|m| (*m).to_owned()).collect(),
                            name: (*name).to_owned(),
                            markers,
                        });
                    }
                    is_test = false;
                    k += 2;
                }
                Some(Token::Ident(_)) => {
                    flush(&mut pending, &mut is_test, &mut scan);
                    k += 2;
                }
                _ => k += 1,
            },
            Token::Punct(b'{') => {
                flush(&mut pending, &mut is_test, &mut scan);
                scopes.push(None);
                k += 1;
            }
            Token::Punct(b'}') => {
                flush(&mut pending, &mut is_test, &mut scan);
                scopes.pop();
                k += 1;
            }
            Token::Punct(b';') => {
                flush(&mut pending, &mut is_test, &mut scan);
                k += 1;
            }
            _ => k += 1,
        }
    }
    flush(&mut pending, &mut is_test, &mut scan);
    scan
}

/// Ends the current item. Markers still pending did not reach a test function.
fn flush(pending: &mut Vec<(usize, &str)>, is_test: &mut bool, scan: &mut Scan) {
    for (line, text) in pending.drain(..) {
        let reason = if parse_marker(text).is_some() {
            NOT_ON_TEST
        } else {
            MALFORMED
        };
        scan.bad.push(bad(line, text, reason));
    }
    *is_test = false;
}

fn bad(line: usize, text: &str, reason: &'static str) -> BadMarker {
    BadMarker {
        line,
        text: format!("///{text}").trim().to_owned(),
        reason,
    }
}

/// Parses the text after `///`. Returns `(spec path, scenario ID)`.
fn parse_marker(text: &str) -> Option<(String, String)> {
    let value = text.trim_start().strip_prefix("spec:")?.trim();
    let (spec, id) = value.rsplit_once(':')?;
    let valid_spec = spec.len() > ".md".len()
        && spec.ends_with(".md")
        && spec.split('/').all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && segment
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        });
    (valid_spec && is_scenario_id(id)).then(|| (spec.to_owned(), id.to_owned()))
}

/// Reads an attribute that starts after `#`. Returns its path and the index after `]`.
fn attribute<'a>(tokens: &[Token<'a>], start: usize) -> (Vec<&'a str>, usize) {
    let mut k = start;
    if tokens.get(k) == Some(&Token::Punct(b'!')) {
        k += 1;
    }
    if tokens.get(k) != Some(&Token::Punct(b'[')) {
        return (Vec::new(), start);
    }
    k += 1;
    let mut path = Vec::new();
    while let Some(Token::Ident(segment)) = tokens.get(k) {
        path.push(*segment);
        k += 1;
        if tokens.get(k) == Some(&Token::Punct(b':'))
            && tokens.get(k + 1) == Some(&Token::Punct(b':'))
        {
            k += 2;
        } else {
            break;
        }
    }
    let mut depth = 1;
    while k < tokens.len() && depth > 0 {
        match tokens[k] {
            Token::Punct(b'[') => depth += 1,
            Token::Punct(b']') => depth -= 1,
            _ => {}
        }
        k += 1;
    }
    (path, k)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Token<'a> {
    /// Outer line doc comment; `text` follows `///`.
    Doc {
        line: usize,
        text: &'a str,
    },
    Ident(&'a str),
    Punct(u8),
}

fn lex(source: &str) -> Vec<Token<'_>> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut line = 1;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\n' => {
                line += 1;
                i += 1;
            }
            b if b.is_ascii_whitespace() => i += 1,
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                let end = bytes[i..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .map_or(bytes.len(), |offset| i + offset);
                if let Some(text) = source[i..end].strip_prefix("///")
                    && !text.starts_with('/')
                {
                    tokens.push(Token::Doc { line, text });
                }
                i = end;
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => i = skip_block_comment(bytes, i, &mut line),
            b'"' => i = skip_string(bytes, i + 1, &mut line),
            b'\'' => i = skip_quote(source, i),
            b if b.is_ascii_digit() => {
                while i < bytes.len() && is_ident_byte(bytes[i]) {
                    i += 1;
                }
            }
            b if b == b'_' || b.is_ascii_alphabetic() || b >= 0x80 => {
                let start = i;
                while i < bytes.len() && is_ident_byte(bytes[i]) {
                    i += 1;
                }
                let word = &source[start..i];
                i = match (word, bytes.get(i)) {
                    ("b" | "c", Some(b'"')) => skip_string(bytes, i + 1, &mut line),
                    ("b", Some(b'\'')) => skip_quote(source, i),
                    ("r" | "br" | "cr", Some(b'"' | b'#')) => {
                        match skip_raw_string(bytes, i, &mut line) {
                            Some(next) => next,
                            // Raw identifier such as `r#type`.
                            None => {
                                let ident_start = i + 1;
                                let mut end = ident_start;
                                while end < bytes.len() && is_ident_byte(bytes[end]) {
                                    end += 1;
                                }
                                tokens.push(Token::Ident(&source[ident_start..end]));
                                end
                            }
                        }
                    }
                    _ => {
                        tokens.push(Token::Ident(word));
                        i
                    }
                };
            }
            b => {
                tokens.push(Token::Punct(b));
                i += 1;
            }
        }
    }
    tokens
}

fn is_ident_byte(b: u8) -> bool {
    b == b'_' || b.is_ascii_alphanumeric() || b >= 0x80
}

/// `start` points at `/*`. Block comments nest.
fn skip_block_comment(bytes: &[u8], start: usize, line: &mut usize) -> usize {
    let mut depth = 0;
    let mut i = start;
    while i < bytes.len() {
        match (bytes[i], bytes.get(i + 1)) {
            (b'/', Some(b'*')) => {
                depth += 1;
                i += 2;
            }
            (b'*', Some(b'/')) => {
                depth -= 1;
                i += 2;
                if depth == 0 {
                    return i;
                }
            }
            (b'\n', _) => {
                *line += 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
    bytes.len()
}

/// `start` points after the opening `"`.
fn skip_string(bytes: &[u8], start: usize, line: &mut usize) -> usize {
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                if bytes.get(i + 1) == Some(&b'\n') {
                    *line += 1;
                }
                i += 2;
            }
            b'"' => return i + 1,
            b'\n' => {
                *line += 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
    bytes.len()
}

/// `start` points at the `"` or first `#` after an `r` prefix. Returns `None`
/// when the bytes are not a raw string.
fn skip_raw_string(bytes: &[u8], start: usize, line: &mut usize) -> Option<usize> {
    let hashes = bytes[start..].iter().take_while(|&&b| b == b'#').count();
    if bytes.get(start + hashes) != Some(&b'"') {
        return None;
    }
    let mut i = start + hashes + 1;
    while i < bytes.len() {
        if bytes[i] == b'\n' {
            *line += 1;
        } else if bytes[i] == b'"'
            && bytes.len() >= i + 1 + hashes
            && bytes[i + 1..i + 1 + hashes].iter().all(|&b| b == b'#')
        {
            return Some(i + 1 + hashes);
        }
        i += 1;
    }
    Some(bytes.len())
}

/// `start` points at `'`. Skips a character literal; a lifetime or label keeps
/// its name for the next token.
fn skip_quote(source: &str, start: usize) -> usize {
    let bytes = source.as_bytes();
    if bytes.get(start + 1) == Some(&b'\\') {
        let mut i = start + 3;
        while i < bytes.len() && bytes[i] != b'\'' {
            i += 1;
        }
        return (i + 1).min(bytes.len());
    }
    match source[start + 1..].chars().next() {
        Some(c) if bytes.get(start + 1 + c.len_utf8()) == Some(&b'\'') => start + 2 + c.len_utf8(),
        _ => start + 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marker(line: usize, spec: &str, id: &str) -> Marker {
        Marker {
            line,
            spec: spec.to_owned(),
            id: id.to_owned(),
        }
    }

    #[test]
    fn links_markers_to_test_in_nested_modules() {
        let source = r#####"
fn helper() -> &'static str { "}{" }

#[cfg(test)]
mod tests {
    const BRACE: char = '{';
    const QUOTE: char = '\'';
    /* a { nested /* } */ comment */
    const RAW: &str = r#"/// spec: relay.md:RLY-999 }"#;

    mod matching {
        /// Pairs one Viewer with one Server.
        /// spec: relay.md:RLY-010
        /// spec: websocket.md:WS-001
        #[tokio::test(flavor = "multi_thread")]
        async fn pairs_in_order<'a>() {}
    }

    /// spec: auth/login.md:SCN-003
    #[test]
    #[ignore = "needs UltraVNC"]
    fn top() {}
}
"#####;
        let scan = scan(source);
        assert_eq!(scan.bad, []);
        assert_eq!(
            scan.tests,
            [
                MarkedTest {
                    modules: vec!["tests".to_owned(), "matching".to_owned()],
                    name: "pairs_in_order".to_owned(),
                    markers: vec![
                        marker(13, "relay.md", "RLY-010"),
                        marker(14, "websocket.md", "WS-001"),
                    ],
                },
                MarkedTest {
                    modules: vec!["tests".to_owned()],
                    name: "top".to_owned(),
                    markers: vec![marker(19, "auth/login.md", "SCN-003")],
                },
            ]
        );
    }

    #[test]
    fn reports_malformed_and_detached_markers() {
        let source = "
/// spec: relay.md:RLY-001, relay.md:RLY-002
#[test]
fn two_on_one_line() {}

/// spec: relay.md:RLY-003
fn not_a_test() {}

/// spec: relay.md:RLY-004
struct NotAFunction;

//// spec: relay.md:RLY-005
#[test]
fn plain_comment_is_ignored() {}
";
        let scan = scan(source);
        assert_eq!(scan.tests, []);
        let lines: Vec<(usize, &str)> = scan.bad.iter().map(|b| (b.line, b.reason)).collect();
        assert_eq!(lines, [(2, MALFORMED), (6, NOT_ON_TEST), (9, NOT_ON_TEST)]);
    }
}
