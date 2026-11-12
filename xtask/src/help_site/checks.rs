//! Static checks of the built help site. docs/architectures/xtask.md lists
//! them (`help_site/checks.rs`). Each problem is one report line.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::LANGUAGES;
use super::html::{self, Tag};
use crate::Error;
use crate::static_server::percent_decode;

/// Links to files of this repository. The rest of the URL must name a path
/// that exists in the repository.
pub const REPOSITORY_BLOB: &str = "https://github.com/kayden-kim/vnc-relay/blob/main/";

/// The other external links that help pages may use (docs/help/AGENTS.md,
/// links): GitHub Releases, the UltraVNC and noVNC official sites, and the
/// sources and license texts of the third-party components on the notices page.
/// A prefix without a final `/` also matches when `/`, `?`, or `#` follows it.
pub const ALLOWED_LINKS: [&str; 11] = [
    "https://github.com/kayden-kim/vnc-relay/releases",
    "https://uvnc.com/",
    "https://www.uvnc.com/",
    "https://novnc.com/",
    "https://github.com/novnc/",
    "https://github.com/rust-lang/mdBook/",
    "https://github.com/FortAwesome/Font-Awesome/",
    "https://fontawesome.com/license/",
    "https://github.com/highlightjs/highlight.js/",
    "https://github.com/zenorocha/clipboard.js/",
    "https://github.com/orioncactus/pretendard/",
];

/// URL path where GitHub Pages serves the site. Root-relative links must start with it.
const MOUNT: &str = "/vnc-relay/";

/// Attribute that holds a URL, by element.
const URL_ATTRIBUTES: [(&str, &str); 10] = [
    ("a", "href"),
    ("link", "href"),
    ("script", "src"),
    ("img", "src"),
    ("iframe", "src"),
    ("source", "src"),
    ("audio", "src"),
    ("video", "src"),
    ("embed", "src"),
    ("object", "data"),
];

/// One HTML page of the site.
pub struct Page {
    tags: Vec<html::Tag>,
    ids: BTreeSet<String>,
    source: String,
}

impl Page {
    pub fn parse(source: String) -> Self {
        let tags = html::start_tags(&source);
        let ids = tags
            .iter()
            .filter_map(|tag| tag.attribute("id"))
            .map(str::to_owned)
            .collect();
        Self { tags, ids, source }
    }
}

/// Every file of the built site. Keys are paths relative to the site root with `/`.
#[derive(Default)]
pub struct Inventory {
    pub files: BTreeSet<String>,
    pub pages: BTreeMap<String, Page>,
    pub styles: BTreeMap<String, String>,
}

impl Inventory {
    /// Reads every file under `site`.
    pub fn read(site: &Path) -> Result<Self, Error> {
        let mut inventory = Self::default();
        collect(site, "", &mut inventory)?;
        Ok(inventory)
    }
}

fn collect(dir: &Path, prefix: &str, inventory: &mut Inventory) -> Result<(), Error> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        let relative = format!("{prefix}{name}");
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect(&path, &format!("{relative}/"), inventory)?;
            continue;
        }
        if name.ends_with(".html") {
            inventory
                .pages
                .insert(relative.clone(), Page::parse(read_text(&path)?));
        } else if name.ends_with(".css") {
            inventory.styles.insert(relative.clone(), read_text(&path)?);
        }
        inventory.files.insert(relative);
    }
    Ok(())
}

fn read_text(path: &Path) -> Result<String, Error> {
    std::fs::read_to_string(path)
        .map_err(|error| Error::new(format!("{}: 읽을 수 없다 ({error})", path.display())))
}

/// Runs every check on the site built under `site`. `root` is the repository root.
pub fn check(root: &Path, site: &Path) -> Result<Vec<String>, Error> {
    let mut problems = Vec::new();
    let mut books = Vec::new();
    for language in LANGUAGES {
        let label = format!("docs/help/{language}/src/SUMMARY.md");
        let summary = read_text(&root.join(&label))?;
        books.push((language, label, chapters(&summary)));
    }
    let (reference_language, _, reference) = &books[0];
    for (language, label, chapters) in &books[1..] {
        if chapters != reference {
            problems.push(format!(
                "{label}: 장 목록이 {reference_language}와 다르다 ({reference_language}: {}; {language}: {})",
                reference.join(", "),
                chapters.join(", ")
            ));
        }
    }
    let inventory = Inventory::read(site)?;
    for (language, _, chapters) in &books {
        check_book(&inventory, language, chapters, &mut problems);
    }
    let repository_has = |path: &str| root.join(path).exists();
    check_references(&inventory, &repository_has, &mut problems);
    // mdBook writes each picture twice (the page image and its zoom copy).
    // One line per problem is enough.
    problems.dedup();
    Ok(problems)
}

/// Chapter files of a `SUMMARY.md` in order: the target of each `](…)` link.
pub fn chapters(summary: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in summary.lines() {
        let mut rest = line;
        while let Some(at) = rest.find("](") {
            rest = &rest[at + 2..];
            let Some(end) = rest.find(')') else {
                break;
            };
            out.push(rest[..end].trim().to_owned());
            rest = &rest[end + 1..];
        }
    }
    out
}

/// Chapter pages exist, carry the book language, and have one `h1` in `main`.
/// mdBook's English 404 page must not stay in the book.
pub fn check_book(
    inventory: &Inventory,
    language: &str,
    chapters: &[String],
    problems: &mut Vec<String>,
) {
    for chapter in chapters {
        let path = format!("{language}/{}", chapter.replace(".md", ".html"));
        let Some(page) = inventory.pages.get(&path) else {
            problems.push(format!("{path}: 장 페이지가 없다"));
            continue;
        };
        let lang = page
            .tags
            .iter()
            .find(|tag| tag.name == "html")
            .and_then(|tag| tag.attribute("lang"));
        if lang != Some(language) {
            problems.push(format!(
                "{path}: 문서 언어가 {language}가 아니다 ({})",
                lang.unwrap_or("없음")
            ));
        }
        let headings = headings_in_main(page);
        if headings != 1 {
            problems.push(format!(
                "{path}: main 안 h1이 {headings}개다 (1개여야 한다)"
            ));
        }
    }
    let not_found = format!("{language}/404.html");
    if inventory.files.contains(&not_found) {
        problems.push(format!("{not_found}: mdBook의 404 페이지가 책 안에 남았다"));
    }
}

/// Number of `h1` start tags between `<main` and `</main>`. mdBook's hidden
/// menu bar holds an `h1` outside `main`.
fn headings_in_main(page: &Page) -> usize {
    let Some(start) = page
        .tags
        .iter()
        .find(|tag| tag.name == "main")
        .map(|tag| tag.offset)
    else {
        return 0;
    };
    let end = page.source[start..]
        .find("</main>")
        .map_or(page.source.len(), |at| start + at);
    page.tags
        .iter()
        .filter(|tag| tag.name == "h1" && tag.offset > start && tag.offset < end)
        .count()
}

/// Links, resources, and images of every page, and `url()`/`@import` of every style sheet.
pub fn check_references(
    inventory: &Inventory,
    repository_has: &dyn Fn(&str) -> bool,
    problems: &mut Vec<String>,
) {
    for (path, page) in &inventory.pages {
        for tag in &page.tags {
            if tag.name == "img" && tag.attribute("alt").is_none_or(|alt| alt.trim().is_empty()) {
                problems.push(format!(
                    "{path}: 그림의 대체 텍스트가 비어 있다: {}",
                    tag.attribute("src").unwrap_or("")
                ));
            }
            let Some(url) = URL_ATTRIBUTES
                .iter()
                .find(|(name, _)| *name == tag.name)
                .and_then(|(_, attribute)| tag.attribute(attribute))
            else {
                continue;
            };
            if let Some(problem) = check_url(inventory, repository_has, path, tag, url) {
                problems.push(format!("{path}: {problem}"));
            }
        }
    }
    for (path, text) in &inventory.styles {
        for url in style_urls(text) {
            if let Some(problem) = check_style_url(inventory, path, &url) {
                problems.push(format!("{path}: {problem}"));
            }
        }
    }
}

/// One URL of an HTML tag. `None` when it passes.
fn check_url(
    inventory: &Inventory,
    repository_has: &dyn Fn(&str) -> bool,
    page: &str,
    tag: &Tag,
    url: &str,
) -> Option<String> {
    // An empty address names no file: `[text]()` or `![alt]()` in a chapter.
    if url.trim().is_empty() {
        return Some(format!("빈 주소: <{}>", tag.name));
    }
    if is_external(url) {
        if tag.name != "a" {
            return Some(format!("외부 자원: <{}> {url}", tag.name));
        }
        if let Some(rest) = url.strip_prefix(REPOSITORY_BLOB) {
            let path = rest.split(['#', '?']).next().unwrap_or_default();
            let decoded = percent_decode(path).unwrap_or_default();
            if decoded.is_empty() || decoded.contains("..") || !repository_has(&decoded) {
                return Some(format!("저장소에 없는 경로: {url}"));
            }
            return None;
        }
        if !is_allowed_link(url) {
            return Some(format!("허용 목록 밖 외부 링크: {url}"));
        }
        return None;
    }
    if has_scheme(url) {
        return Some(format!("허용하지 않는 주소 형식: {url}"));
    }
    let (location, fragment) = match url.split_once('#') {
        Some((location, fragment)) => (location, Some(fragment)),
        None => (url, None),
    };
    let location = location.split('?').next().unwrap_or_default();
    let target = if location.is_empty() {
        page.to_owned()
    } else {
        match resolve(inventory, page, location) {
            Ok(target) => target,
            Err(problem) => return Some(format!("{problem}: {url}")),
        }
    };
    if !inventory.files.contains(&target) {
        let what = if tag.name == "img" {
            "그림 파일이 없다"
        } else {
            "링크 대상이 없다"
        };
        return Some(format!("{what}: {url}"));
    }
    let fragment = fragment.filter(|fragment| !fragment.is_empty())?;
    let decoded = percent_decode(fragment).unwrap_or_default();
    let found = inventory
        .pages
        .get(&target)
        .is_some_and(|target_page| target_page.ids.contains(&decoded));
    (!found).then(|| format!("절이 없다: {url}"))
}

/// One `url()` or `@import` address of a style sheet. `data:` addresses pass:
/// the browser makes no request for them.
fn check_style_url(inventory: &Inventory, sheet: &str, url: &str) -> Option<String> {
    if url.starts_with("data:") {
        return None;
    }
    if is_external(url) {
        return Some(format!("외부 자원: url({url})"));
    }
    if has_scheme(url) {
        return Some(format!("허용하지 않는 주소 형식: url({url})"));
    }
    let location = url.split(['?', '#']).next().unwrap_or_default();
    match resolve(inventory, sheet, location) {
        Ok(target) if inventory.files.contains(&target) => None,
        Ok(_) => Some(format!("링크 대상이 없다: url({url})")),
        Err(problem) => Some(format!("{problem}: url({url})")),
    }
}

fn is_external(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("//")
}

/// A scheme such as `mailto:` or `javascript:`: a `:` before any `/`, `?`, or `#`.
fn has_scheme(url: &str) -> bool {
    let head = url.split(['/', '?', '#']).next().unwrap_or_default();
    head.contains(':')
}

fn is_allowed_link(url: &str) -> bool {
    ALLOWED_LINKS.iter().any(|prefix| {
        url.strip_prefix(prefix).is_some_and(|rest| {
            prefix.ends_with('/') || rest.is_empty() || rest.starts_with(['/', '?', '#'])
        })
    })
}

/// Site-relative path of a link from the file `from`. A path that names a
/// directory (empty last segment or a directory in the site) becomes its
/// `index.html`. `Err` when the link leaves the site.
fn resolve(inventory: &Inventory, from: &str, location: &str) -> Result<String, &'static str> {
    let decoded = percent_decode(location).ok_or("잘못된 퍼센트 인코딩")?;
    let mut parts: Vec<&str> = Vec::new();
    let relative = if decoded.starts_with('/') {
        decoded.strip_prefix(MOUNT).ok_or("사이트 밖을 가리킨다")?
    } else {
        parts.extend(from.split('/'));
        parts.pop();
        decoded.as_str()
    };
    for segment in relative.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err("사이트 밖을 가리킨다");
                }
            }
            _ => parts.push(segment),
        }
    }
    let mut target = parts.join("/");
    let names_directory = relative.is_empty()
        || relative.ends_with('/')
        || inventory
            .files
            .iter()
            .any(|file| file.starts_with(&format!("{target}/")));
    if names_directory {
        target = if target.is_empty() {
            "index.html".to_owned()
        } else {
            format!("{target}/index.html")
        };
    }
    Ok(target)
}

/// Addresses in `url(…)` and `@import "…"` of a style sheet, comments removed.
fn style_urls(text: &str) -> Vec<String> {
    let mut stripped = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("/*") {
        stripped.push_str(&rest[..start]);
        rest = rest[start + 2..]
            .find("*/")
            .map_or("", |end| &rest[start + 2 + end + 2..]);
    }
    stripped.push_str(rest);
    let mut urls = Vec::new();
    let lower = stripped.to_ascii_lowercase();
    for (at, _) in lower.match_indices("url(") {
        let value = stripped[at + 4..].trim_start();
        let url = match value.chars().next() {
            Some(quote @ ('"' | '\'')) => value[1..].split(quote).next().unwrap_or_default(),
            _ => value.split(')').next().unwrap_or_default().trim_end(),
        };
        urls.push(url.to_owned());
    }
    for (at, _) in lower.match_indices("@import") {
        let value = stripped[at + 7..].trim_start();
        if let Some(quote @ ('"' | '\'')) = value.chars().next() {
            urls.push(
                value[1..]
                    .split(quote)
                    .next()
                    .unwrap_or_default()
                    .to_owned(),
            );
        }
    }
    urls
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(pages: &[(&str, &str)], others: &[&str]) -> Inventory {
        let mut inventory = Inventory::default();
        for (path, source) in pages {
            inventory.files.insert((*path).to_owned());
            inventory
                .pages
                .insert((*path).to_owned(), Page::parse((*source).to_owned()));
        }
        for path in others {
            inventory.files.insert((*path).to_owned());
        }
        inventory
    }

    fn problems(inventory: &Inventory) -> Vec<String> {
        let mut problems = Vec::new();
        check_references(
            inventory,
            &|path| path == "docs/specs/help-site.md",
            &mut problems,
        );
        problems
    }

    #[test]
    fn internal_links_resolve_pages_directories_and_encoded_sections() {
        let inventory = site(
            &[
                (
                    "ko/index.html",
                    concat!(
                        "<h2 id=\"기준-버전\">x</h2>",
                        "<a href=\"configure.html#%EC%A0%91%EC%86%8D-%ED%97%88%EC%9A%A9\">a</a>",
                        "<a href=\"#기준-버전\">b</a><a href=\"../en/\">c</a>",
                        "<a href=\"/vnc-relay/ko\">d</a><img src=\"images/a.png\" alt=\"화면\">"
                    ),
                ),
                ("ko/configure.html", "<h2 id=\"접속-허용\">x</h2>"),
                ("en/index.html", ""),
            ],
            &["ko/images/a.png"],
        );
        assert_eq!(problems(&inventory), Vec::<String>::new());
    }

    #[test]
    fn missing_targets_sections_and_alt_text_are_reported() {
        let inventory = site(
            &[(
                "ko/index.html",
                concat!(
                    "<a href=\"missing.html\">a</a><a href=\"index.html#없는-절\">b</a>",
                    "<a href=\"../../outside.html\">c</a><a href=\"/other/\">d</a>",
                    "<img src=\"images/none.png\" alt=\" \">"
                ),
            )],
            &[],
        );
        assert_eq!(
            problems(&inventory),
            [
                "ko/index.html: 링크 대상이 없다: missing.html",
                "ko/index.html: 절이 없다: index.html#없는-절",
                "ko/index.html: 사이트 밖을 가리킨다: ../../outside.html",
                "ko/index.html: 사이트 밖을 가리킨다: /other/",
                "ko/index.html: 그림의 대체 텍스트가 비어 있다: images/none.png",
                "ko/index.html: 그림 파일이 없다: images/none.png",
            ]
        );
    }

    #[test]
    fn empty_addresses_are_reported() {
        let inventory = site(
            &[(
                "ko/index.html",
                "<img src=\"\" alt=\"화면\"><a href=\" \">a</a><script src=\"\"></script>",
            )],
            &[],
        );
        assert_eq!(
            problems(&inventory),
            [
                "ko/index.html: 빈 주소: <img>",
                "ko/index.html: 빈 주소: <a>",
                "ko/index.html: 빈 주소: <script>",
            ]
        );
    }

    #[test]
    fn external_addresses_follow_the_allow_list_and_the_repository() {
        let inventory = site(
            &[(
                "en/index.html",
                concat!(
                    "<a href=\"https://github.com/kayden-kim/vnc-relay/blob/main/docs/specs/help-site.md#7\">a</a>",
                    "<a href=\"https://github.com/kayden-kim/vnc-relay/blob/main/docs/missing.md\">b</a>",
                    "<a href=\"https://github.com/kayden-kim/vnc-relay/releases/latest\">c</a>",
                    "<a href=\"https://github.com/kayden-kim/vnc-relay/releasesx\">d</a>",
                    "<a href=\"mailto:someone@example.test\">e</a>",
                    "<link rel=\"stylesheet\" href=\"https://cdn.example.test/a.css\">",
                    "<script src=\"//cdn.example.test/a.js\"></script>"
                ),
            )],
            &[],
        );
        assert_eq!(
            problems(&inventory),
            [
                "en/index.html: 저장소에 없는 경로: https://github.com/kayden-kim/vnc-relay/blob/main/docs/missing.md",
                "en/index.html: 허용 목록 밖 외부 링크: https://github.com/kayden-kim/vnc-relay/releasesx",
                "en/index.html: 허용하지 않는 주소 형식: mailto:someone@example.test",
                "en/index.html: 외부 자원: <link> https://cdn.example.test/a.css",
                "en/index.html: 외부 자원: <script> //cdn.example.test/a.js",
            ]
        );
    }

    #[test]
    fn style_sheet_addresses_must_stay_on_the_site() {
        let mut inventory = site(&[], &["assets/fonts/a.woff2"]);
        inventory.styles.insert(
            "assets/tokens.css".to_owned(),
            concat!(
                "/* url(https://comment.example.test/x) */",
                "@font-face { src: url(\"fonts/a.woff2\") format(\"woff2\"); }",
                "@import 'https://cdn.example.test/b.css';",
                ".a { background: url(data:image/svg+xml,abc); }",
                ".b { background: URL( fonts/missing.woff2 ); }"
            )
            .to_owned(),
        );
        assert_eq!(
            problems(&inventory),
            [
                "assets/tokens.css: 링크 대상이 없다: url(fonts/missing.woff2)",
                "assets/tokens.css: 외부 자원: url(https://cdn.example.test/b.css)",
            ]
        );
    }

    #[test]
    fn summary_chapters_keep_file_order() {
        let summary =
            "# 목차\n\n[소개](index.md)\n\n- [설치](install.md)\n\n---\n\n[고지](notices.md)\n";
        assert_eq!(chapters(summary), ["index.md", "install.md", "notices.md"]);
    }
}
