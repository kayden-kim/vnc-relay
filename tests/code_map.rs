//! Checks code-map references, not the meaning of responsibility descriptions.
//! See docs/architectures/code-map.md, OPS-022.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn code_spans(text: &str) -> impl Iterator<Item = &str> {
    text.split('`')
        .enumerate()
        .filter_map(|(index, text)| (index % 2 == 1).then_some(text))
}

fn rust_files(path: &Path) -> Vec<PathBuf> {
    let mut pending = vec![path.to_path_buf()];
    let mut files = Vec::new();
    while let Some(path) = pending.pop() {
        if path.is_dir() {
            for entry in std::fs::read_dir(&path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
            {
                pending.push(
                    entry
                        .unwrap_or_else(|error| panic!("read module entry: {error}"))
                        .path(),
                );
            }
        } else if path.is_file() && path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    files
}

// Skip comments and literals before looking for declaration keywords. In
// particular, examples in doc comments and test strings are not definitions.
fn tokens(source: &str) -> Vec<&str> {
    let bytes = source.as_bytes();
    let mut result = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(b"//") {
            i += bytes[i..]
                .iter()
                .position(|byte| *byte == b'\n')
                .unwrap_or(bytes.len() - i);
        } else if bytes[i..].starts_with(b"/*") {
            i += 2;
            let mut depth = 1;
            while i < bytes.len() && depth > 0 {
                if bytes[i..].starts_with(b"/*") {
                    depth += 1;
                    i += 2;
                } else if bytes[i..].starts_with(b"*/") {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
        } else if bytes[i] == b'r' && matches!(bytes.get(i + 1), Some(b'#' | b'"')) {
            let start = i;
            i += 1;
            while bytes.get(i) == Some(&b'#') {
                i += 1;
            }
            if bytes.get(i) == Some(&b'"') {
                let hashes = i - start - 1;
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'"'
                        && bytes
                            .get(i + 1..i + 1 + hashes)
                            .is_some_and(|suffix| suffix.iter().all(|byte| *byte == b'#'))
                    {
                        i += 1 + hashes;
                        break;
                    }
                    i += 1;
                }
            } else {
                // Raw identifier, not a raw string.
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                result.push(&source[start..i]);
            }
        } else if bytes[i] == b'"'
            || (bytes[i] == b'\''
                && (bytes.get(i + 2) == Some(&b'\'') || bytes.get(i + 1) == Some(&b'\\')))
        {
            let quote = bytes[i];
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i = (i + 2).min(bytes.len());
                } else if bytes[i] == quote {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
        } else if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            result.push(&source[start..i]);
        } else {
            i += 1;
        }
    }
    result
}

fn definitions(source: &str) -> BTreeSet<&str> {
    tokens(source)
        .windows(2)
        .filter_map(|pair| {
            matches!(
                pair[0],
                "fn" | "struct" | "enum" | "trait" | "impl" | "type" | "const" | "mod"
            )
            .then_some(pair[1])
        })
        .collect()
}

fn symbols(root: &Path, path: &str) -> BTreeSet<String> {
    let path = root.join(path);
    let mut files = rust_files(&path);
    if path.extension().is_some_and(|extension| extension == "rs") {
        files.extend(rust_files(&path.with_extension("")));
    }
    files
        .into_iter()
        .flat_map(|file| {
            let source = std::fs::read_to_string(&file)
                .unwrap_or_else(|error| panic!("read {}: {error}", file.display()));
            definitions(&source)
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .collect()
}

fn missing_references(root: &Path, map: &str) -> Vec<String> {
    let mut in_section = false;
    let mut column = None;
    let mut checked = 0;
    let mut cache = BTreeMap::new();
    let mut missing = Vec::new();
    for (line, text) in map.lines().enumerate() {
        if text.starts_with("## ") {
            in_section = text.starts_with("## 2. ");
            column = None;
        }
        if !in_section {
            continue;
        }
        if !text.starts_with('|') {
            column = None;
            continue;
        }
        let cells: Vec<_> = text.split('|').skip(1).map(str::trim).collect();
        if let Some(index) = cells.iter().position(|cell| *cell == "코드 위치") {
            column = Some(index);
            continue;
        }
        let Some(cell) = column.and_then(|index| cells.get(index)) else {
            continue;
        };
        let mut current = None;
        for item in code_spans(cell) {
            if item.contains('/') {
                checked += 1;
                current = Some(item);
                if !root.join(item).exists() {
                    missing.push(format!("line {}: missing path `{item}`", line + 1));
                }
            } else if let Some(path) = current {
                let name = item.rsplit("::").next().unwrap_or(item);
                let defined = cache.entry(path).or_insert_with(|| symbols(root, path));
                if !defined.contains(name) {
                    missing.push(format!(
                        "line {}: `{path}` has no definition for `{item}`",
                        line + 1
                    ));
                }
            } else {
                missing.push(format!(
                    "line {}: symbol `{item}` has no preceding path",
                    line + 1
                ));
            }
        }
    }
    if checked == 0 {
        missing.push("section 2 has no code-location path references".to_owned());
    }
    missing
}

struct SourceFile {
    path: String,
    definitions: BTreeSet<String>,
    usages: BTreeSet<String>,
    words: BTreeSet<String>,
}

impl SourceFile {
    fn new(path: String, source: &str) -> Self {
        let words = tokens(source);
        let definitions = definitions(source).into_iter().map(str::to_owned).collect();
        let usages = words
            .iter()
            .enumerate()
            .filter_map(|(index, word)| {
                let declaration = index > 0
                    && matches!(
                        words[index - 1],
                        "fn" | "struct" | "enum" | "trait" | "impl" | "type" | "const" | "mod"
                    );
                (!declaration).then(|| (*word).to_owned())
            })
            .collect();
        Self {
            path,
            definitions,
            usages,
            words: words.into_iter().map(str::to_owned).collect(),
        }
    }
}

fn source_index(root: &Path) -> Vec<SourceFile> {
    [
        "src",
        "crates/domain/src",
        "crates/app/src",
        "crates/adapters/src",
    ]
    .into_iter()
    .flat_map(|dir| rust_files(&root.join(dir)))
    .map(|path| {
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        SourceFile::new(
            path.strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/"),
            &source,
        )
    })
    .collect()
}

fn leaf(symbol: &str) -> &str {
    symbol.rsplit("::").next().unwrap_or(symbol)
}

fn is_identifier(symbol: &str) -> bool {
    symbol.split("::").all(|part| {
        let mut bytes = part.bytes();
        bytes
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
            && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    })
}

fn defining_files<'a>(index: &'a [SourceFile], symbol: &str) -> Vec<&'a SourceFile> {
    let name = leaf(symbol);
    let owner = symbol.rsplit_once("::").map(|(owner, _)| leaf(owner));
    index
        .iter()
        .filter(|file| {
            let stem = Path::new(&file.path)
                .file_stem()
                .and_then(|stem| stem.to_str());
            if owner.is_none() && stem == Some(name) {
                return true;
            }
            file.definitions.contains(name)
                && owner.is_none_or(|owner| {
                    if owner.as_bytes()[0].is_ascii_uppercase() {
                        file.words.contains(owner)
                    } else {
                        stem == Some(owner)
                    }
                })
        })
        .collect()
}

fn location_paths(cell: &str) -> Vec<&str> {
    code_spans(cell).filter(|span| span.contains('/')).collect()
}

fn location_symbols(cell: &str) -> Vec<&str> {
    code_spans(cell)
        .filter(|span| is_identifier(span))
        .collect()
}

fn files_at<'a>(index: &'a [SourceFile], paths: &[&str]) -> Vec<&'a SourceFile> {
    index
        .iter()
        .filter(|file| {
            paths.iter().any(|path| {
                file.path == *path || (path.ends_with('/') && file.path.starts_with(path))
            })
        })
        .collect()
}

fn used_in(files: &[&SourceFile], symbol: &str) -> bool {
    files.iter().any(|file| file.usages.contains(leaf(symbol)))
}

// Alternatives start again at the entry. Arrows and list separators are
// sequential links; prose-only segments are not identifiers.
fn entry_chains(cell: &str) -> Vec<Vec<&str>> {
    cell.split("또는")
        .map(|alternative| {
            alternative
                .split(['→', '·', ','])
                .flat_map(|segment| {
                    let spans: Vec<_> = code_spans(segment).collect();
                    if spans.is_empty() {
                        vec![segment.trim()]
                    } else {
                        spans
                    }
                })
                .filter(|symbol| is_identifier(symbol))
                .collect()
        })
        .collect()
}

fn missing_connections(index: &[SourceFile], map: &str) -> Vec<String> {
    let mut section = 0;
    let mut missing = Vec::new();
    let mut entry_count = 0;
    let mut steps: Vec<(usize, Vec<&SourceFile>, Vec<&str>)> = Vec::new();
    for (line, text) in map.lines().enumerate() {
        if text.starts_with("## ") {
            section = if text.starts_with("## 3. ") {
                3
            } else if text.starts_with("## 4. ") {
                4
            } else {
                0
            };
        }
        if !text.starts_with('|') {
            continue;
        }
        let cells: Vec<_> = text.split('|').skip(1).map(str::trim).collect();
        if cells.len() < 4 {
            continue;
        }
        if section == 3 && cells[0] != "진입점 종류" && !cells[0].starts_with("---") {
            entry_count += 1;
            let paths = location_paths(cells[1]);
            let entries = files_at(index, &paths);
            if entries.is_empty() {
                missing.push(format!("line {}: entry has no source files", line + 1));
            }
            for chain in entry_chains(cells[3]) {
                if chain.is_empty() {
                    missing.push(format!("line {}: entry has no identifier chain", line + 1));
                }
                let mut previous = None;
                for symbol in chain {
                    let targets = defining_files(index, symbol);
                    if targets.is_empty() {
                        missing.push(format!("line {}: unresolved callee `{symbol}`", line + 1));
                    }
                    let mut sources = entries.clone();
                    if let Some(previous) = previous {
                        sources.extend(defining_files(index, previous));
                    }
                    if !used_in(&sources, symbol) {
                        missing.push(format!("line {}: missing entry link `{} → {symbol}` (usage outside a definition required)", line + 1, previous.unwrap_or(cells[0])));
                    }
                    previous = Some(symbol);
                }
            }
        } else if section == 4 && cells[0].parse::<usize>().is_ok() {
            let paths = location_paths(cells[1]);
            let files = files_at(index, &paths);
            let symbols = location_symbols(cells[1]);
            if files.is_empty() || symbols.is_empty() {
                missing.push(format!(
                    "line {}: flow step needs source files and symbols",
                    line + 1
                ));
            }
            for symbol in &symbols {
                if !files
                    .iter()
                    .any(|file| file.definitions.contains(leaf(symbol)))
                {
                    missing.push(format!(
                        "line {}: unresolved flow symbol `{symbol}`",
                        line + 1
                    ));
                }
            }
            steps.push((line + 1, files, symbols));
        }
    }
    if entry_count == 0 {
        missing.push("section 3 has no entry chains".to_owned());
    }
    if steps.len() < 2 {
        missing.push("section 4 has fewer than two flow steps".to_owned());
    }
    if let Some((_, entry_files, _)) = steps.first() {
        for pair in steps.windows(2) {
            let mut sources = pair[0].1.clone();
            sources.extend(entry_files.iter().copied());
            if !pair[1].2.iter().any(|symbol| used_in(&sources, symbol)) {
                missing.push(format!(
                    "line {} → {}: missing flow link to `{}`",
                    pair[0].0,
                    pair[1].0,
                    pair[1].2.join("`, `")
                ));
            }
        }
    }
    missing
}

/// spec: operations.md:OPS-022
#[test]
fn code_map_paths_and_symbols_resolve_to_current_code() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let map = std::fs::read_to_string(root.join("docs/architectures/code-map.md"))
        .unwrap_or_else(|error| panic!("read code map: {error}"));
    let mut missing = missing_references(root, &map);
    missing.extend(missing_connections(&source_index(root), &map));
    assert!(
        missing.is_empty(),
        "code-map reference drift:\n{}",
        missing.join("\n")
    );
}

#[test]
fn definitions_ignore_comments_and_string_examples() {
    let source = r###"
        // fn commented() {}
        /* struct Hidden; /* fn nested() {} */ */
        const EXAMPLE: &str = "fn fake() {}";
        const RAW: &str = r#"struct Fake;"#;
        pub struct Actual;
        impl Actual { pub async fn method(&self) {} }
        type Alias = Actual;
        mod child;
    "###;
    let found = definitions(source);
    for name in ["Actual", "method", "Alias", "child", "EXAMPLE", "RAW"] {
        assert!(found.contains(name), "{name}");
    }
    for name in ["commented", "Hidden", "nested", "fake", "Fake"] {
        assert!(!found.contains(name), "{name}");
    }
}

#[test]
fn missing_paths_and_symbols_are_reported_together() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let map = "## 2. Code\n| 코드 위치 | 책임 |\n| --- | --- |\n| `missing/file.rs` `Ghost` | none |\n| `src/main.rs` `Missing::method` | none |\n## 3. End\n";
    let missing = missing_references(root, map);
    assert_eq!(missing.len(), 3);
    assert!(missing[0].contains("missing/file.rs"));
    assert!(missing[1].contains("Ghost"));
    assert!(missing[2].contains("Missing::method"));
}

#[test]
fn connections_require_usage_not_just_a_definition_or_example() {
    let index = vec![
        SourceFile::new(
            "src/entry.rs".to_owned(),
            "fn entry() { first(); } fn lonely() {} // missing()\n const EXAMPLE: &str = \"missing()\";",
        ),
        SourceFile::new("src/first.rs".to_owned(), "fn first() { second(); }"),
        SourceFile::new("src/second.rs".to_owned(), "fn second() {}"),
    ];
    let map = "## 3. Entry\n| CLI | `src/entry.rs` `entry` | start | `first` → `second` |\n## 4. Flow\n| 1 | `src/entry.rs` `entry` | a | b |\n| 2 | `src/first.rs` `first` | a | b |\n| 3 | `src/second.rs` `second` | a | b |\n";
    assert!(missing_connections(&index, map).is_empty());
    let detached = map.replace("`first` → `second`", "`lonely` 또는 `missing`");
    let missing = missing_connections(&index, &detached);
    assert!(
        missing
            .iter()
            .any(|problem| problem.contains("entry link `CLI → lonely`"))
    );
    assert!(
        missing
            .iter()
            .any(|problem| problem.contains("unresolved callee `missing`"))
    );
    let broken_flow = map.replace("`src/second.rs` `second`", "`src/entry.rs` `lonely`");
    assert!(
        missing_connections(&index, &broken_flow)
            .iter()
            .any(|problem| problem.contains("missing flow link"))
    );
}
