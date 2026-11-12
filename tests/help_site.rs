//! Operator help site scenarios (docs/specs/help-site.md section 7).
//!
//! The site is built once with `cargo xtask help-site`, which needs mdBook 0.5.4
//! (docs/how-to/publish-help.md section 2.1). A loopback server in this file
//! serves the built site under `/vnc-relay/` the way GitHub Pages does: a
//! directory without a final slash gets a redirect, every missing path gets
//! `404.html` with HTTP 404, and no Content-Security-Policy header is sent.
//! Headless Chromium drives the pages over CDP. A missing mdBook or browser
//! fails the tests; they are not skipped.

#[path = "common/cdp.rs"]
mod cdp;

use std::fs;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Condvar, LazyLock, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;

use cdp::Browser;

/// Site path on GitHub Pages: `https://kayden-kim.github.io/vnc-relay/`.
const MOUNT: &str = "/vnc-relay/";
const LANGUAGES: [&str; 2] = ["ko", "en"];
const REPOSITORY_BLOB: &str = "https://github.com/kayden-kim/vnc-relay/blob/main/";
const IO_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_HEAD: usize = 8 * 1024;
/// Browsers that run at once. Every scenario starts its own Chromium.
const MAX_BROWSERS: usize = 4;
const STORAGE_KEY: &str = "vnc-relay.language";

/// Interface text that mdBook writes in English. None of it may stay visible or
/// in an accessible name on a Korean page (HELP-011).
const MDBOOK_ENGLISH: [&str; 9] = [
    "Table of contents",
    "Toggle Table of Contents",
    "Keyboard shortcuts",
    "to navigate between chapters",
    "to show this help",
    "to hide this help",
    "Previous chapter",
    "Next chapter",
    "Copy to clipboard",
];

/// The site texts of one language (docs/specs/help-site.md section 9).
struct Copy {
    language: &'static str,
    /// Browser language preferences that select this language.
    preferences: [&'static str; 2],
    brand: &'static str,
    base: &'static str,
    baseline: &'static str,
    toc: &'static str,
    search: &'static str,
    search_title: &'static str,
    search_help: &'static str,
    close: &'static str,
    other_language: &'static str,
    previous: &'static str,
    next: &'static str,
    copy: &'static str,
    copied: &'static str,
    copy_failed: &'static str,
    pager: &'static str,
    keys_title: &'static str,
    keys: [(&'static str, &'static str); 4],
    failed: &'static str,
    /// Configuration chapter title in the contents.
    configure: &'static str,
}

impl Copy {
    fn results(&self, count: usize) -> String {
        match (self.language, count) {
            ("ko", count) => format!("검색 결과 {count}건"),
            (_, 1) => "1 result".to_owned(),
            (_, count) => format!("{count} results"),
        }
    }

    fn no_results(&self, query: &str) -> String {
        if self.language == "ko" {
            format!("‘{query}’ 검색 결과가 없습니다.")
        } else {
            format!("No results for “{query}”.")
        }
    }
}

const KO: Copy = Copy {
    language: "ko",
    preferences: ["ko-KR", "ko"],
    brand: "VNC Relay 도움말",
    base: "main 기준",
    baseline: "기준-버전",
    toc: "목차",
    search: "검색",
    search_title: "도움말 검색",
    search_help: "두 글자 이상 입력하면 결과가 바로 바뀝니다.",
    close: "닫기",
    other_language: "English",
    previous: "이전",
    next: "다음",
    copy: "복사",
    copied: "복사했습니다.",
    copy_failed: "복사하지 못했습니다. 코드를 직접 선택해 복사하십시오.",
    pager: "장 이동",
    keys_title: "단축키",
    keys: [
        ("←→", "이전 장·다음 장"),
        ("/", "검색 열기"),
        ("?", "이 도움말 보기"),
        ("Esc", "검색·단축키 도움말 닫기"),
    ],
    failed: "검색 자료를 불러오지 못했습니다. 검색을 닫았다가 다시 여십시오.",
    configure: "설정",
};

const EN: Copy = Copy {
    language: "en",
    preferences: ["en-US", "en"],
    brand: "VNC Relay Help",
    base: "Based on main",
    baseline: "baseline",
    toc: "Contents",
    search: "Search",
    search_title: "Search help",
    search_help: "Type at least two characters to see results.",
    close: "Close",
    other_language: "한국어",
    previous: "Previous",
    next: "Next",
    copy: "Copy",
    copied: "Copied.",
    copy_failed: "Could not copy. Select the code and copy it manually.",
    pager: "Chapter navigation",
    keys_title: "Keyboard shortcuts",
    keys: [
        ("←→", "Previous or next chapter"),
        ("/", "Open search"),
        ("?", "Show this help"),
        ("Esc", "Close search or this help"),
    ],
    failed: "Could not load search data. Close search and open it again.",
    configure: "Configuration",
};

const COPIES: [&Copy; 2] = [&KO, &EN];

/// spec: help-site.md:HELP-001
#[test]
fn root_moves_to_the_browser_language_without_a_history_entry() {
    for (name, preferences, expected) in [
        ("root-ko", ["ko-KR", "en"].as_slice(), "ko"),
        ("root-fr-en", ["fr", "en-US"].as_slice(), "en"),
        ("root-fr", ["fr"].as_slice(), "en"),
    ] {
        browse(name, preferences, |browser, site| {
            browser.navigate(&site.url("ko/notices.html"));
            browser.go(&site.url(""));
            browser.wait(&at(&format!("{MOUNT}{expected}/")));
            // `replace` leaves no entry for the root, so Back skips it.
            browser.eval("history.back(); true");
            browser.wait(&at(&format!("{MOUNT}ko/notices.html")));
        });
    }
}

/// spec: help-site.md:HELP-002
#[test]
fn root_prefers_the_saved_language_and_offers_links_without_scripts() {
    browse("root-saved", &KO.preferences, |browser, site| {
        browser.navigate(&site.url("ko/notices.html"));
        browser.eval(&format!(
            "localStorage.setItem('{STORAGE_KEY}', 'en'); true"
        ));
        browser.go(&site.url(""));
        browser.wait(&at(&format!("{MOUNT}en/")));

        browser.click(".help-lang");
        browser.wait(&at(&format!("{MOUNT}ko/index.html")));
        assert_eq!(
            browser.eval(&format!("localStorage.getItem('{STORAGE_KEY}')")),
            json!("ko")
        );
        browser.go(&site.url(""));
        browser.wait(&at(&format!("{MOUNT}ko/")));

        browser.command(
            "Emulation.setScriptExecutionDisabled",
            json!({"value": true}),
        );
        browser.navigate(&site.url(""));
        let text = browser.text();
        for expected in [
            "VNC Relay 도움말",
            "VNC Relay Help",
            "언어를 고르십시오.",
            "Choose a language.",
        ] {
            assert!(
                text.contains(expected),
                "root without scripts: {expected} missing in {text}"
            );
        }
        assert_eq!(
            browser.eval("[...document.querySelectorAll('a[data-lang]')].map(a => a.textContent)"),
            json!(["한국어", "English"])
        );
        browser.click("a[data-lang=en]");
        browser.wait(&at(&format!("{MOUNT}en/")));
    });
}

/// spec: help-site.md:HELP-003
#[test]
fn language_link_opens_the_same_chapter_and_saves_the_choice() {
    browse("language-link", &KO.preferences, |browser, site| {
        browser.navigate(&site.url("ko/configure.html"));
        browser.click(".help-lang");
        browser.wait(&at(&format!("{MOUNT}en/configure.html")));
        assert_eq!(
            browser.eval(&format!("localStorage.getItem('{STORAGE_KEY}')")),
            json!("en")
        );

        browser.command(
            "Emulation.setScriptExecutionDisabled",
            json!({"value": true}),
        );
        browser.navigate(&site.url("ko/configure.html"));
        browser.click(".help-lang");
        browser.wait(&at(&format!("{MOUNT}en/index.html")));
    });
}

/// spec: help-site.md:HELP-004
#[test]
fn both_books_have_the_same_chapters_one_title_and_their_language() {
    let site = &*SITE;
    let chapters = chapters("ko");
    assert_eq!(
        chapters,
        self::chapters("en"),
        "chapter files or order differ"
    );
    for language in LANGUAGES {
        for chapter in &chapters {
            let path = site.root.join(language).join(chapter);
            let page = read(&path);
            assert!(
                page.contains(&format!("<html lang=\"{language}\"")),
                "{language}/{chapter}: document language is not {language}"
            );
            let main = page
                .split_once("<main>")
                .and_then(|(_, rest)| rest.split_once("</main>"))
                .map(|(main, _)| main)
                .unwrap_or_else(|| panic!("{language}/{chapter}: main missing"));
            assert_eq!(
                main.matches("<h1").count(),
                1,
                "{language}/{chapter}: one h1 expected in main"
            );
        }
        assert!(
            !site.root.join(language).join("404.html").exists(),
            "{language}/404.html: mdBook's missing page stayed in the book"
        );
    }
}

/// spec: help-site.md:HELP-005
#[test]
fn desktop_contents_collapse_and_stay_collapsed_on_the_next_chapter() {
    browse("toc-desktop", &KO.preferences, |browser, site| {
        browser.size(1280, 900, 1.0);
        browser.navigate(&site.url("ko/configure.html"));
        assert_eq!(
            browser.eval(
                "document.querySelector('#mdbook-sidebar a[aria-current=\"page\"]')?.textContent.trim()"
            ),
            json!(KO.configure)
        );
        assert_eq!(expanded(browser), "true");
        let open = content_width(browser);

        browser.click(".help-toc-toggle");
        browser.wait(
            "document.querySelector('.help-toc-toggle').getAttribute('aria-expanded') === 'false'",
        );
        let closed = content_width(browser);
        assert!(closed > open, "text column must widen: {open} -> {closed}");

        browser.navigate(&site.url("ko/install.html"));
        assert_eq!(expanded(browser), "false", "collapsed choice must persist");
        assert!(
            (content_width(browser) - closed).abs() < 1.0,
            "next chapter must open collapsed"
        );

        browser.click(".help-toc-toggle");
        browser.wait(
            "document.querySelector('.help-toc-toggle').getAttribute('aria-expanded') === 'true'",
        );
        assert!(
            content_width(browser) < closed,
            "text column must narrow again"
        );
    });
}

/// spec: help-site.md:HELP-006
#[test]
fn mobile_contents_open_below_the_top_bar_and_push_the_text_down() {
    browse("toc-mobile", &KO.preferences, |browser, site| {
        browser.size(390, 844, 1.0);
        browser.navigate(&site.url("ko/configure.html"));
        assert_eq!(expanded(browser), "false");
        assert_eq!(
            browser.eval("document.querySelector('#mdbook-sidebar').checkVisibility()"),
            json!(false),
            "contents must start closed"
        );
        browser.eval("document.querySelector('.help-toc-toggle').focus(); true");
        browser.press("Tab");
        assert_eq!(
            browser.eval("!document.activeElement.closest('#mdbook-sidebar')"),
            json!(true),
            "closed contents links must not take focus"
        );

        browser.click(".help-toc-toggle");
        browser.wait(
            "document.querySelector('.help-toc-toggle').getAttribute('aria-expanded') === 'true'",
        );
        let layout = browser.eval(
            "(() => { const r = (s) => document.querySelector(s).getBoundingClientRect();
              return { top: r('.help-top').bottom, toc: r('#mdbook-sidebar').top,
                tocBottom: r('#mdbook-sidebar').bottom, text: r('#mdbook-content').top,
                tocLeft: r('#mdbook-sidebar').left, tocRight: r('#mdbook-sidebar').right }; })()",
        );
        let value = |key: &str| layout[key].as_f64().unwrap_or(f64::NAN);
        assert!(
            (value("toc") - value("top")).abs() <= 1.0,
            "contents must open right below the top bar: {layout}"
        );
        assert!(
            value("text") >= value("tocBottom") - 0.5,
            "text must move below the contents: {layout}"
        );
        assert!(
            value("tocLeft") >= -0.5 && value("tocRight") <= 390.5,
            "contents must fit the width: {layout}"
        );

        browser.click(".help-toc-toggle");
        browser.wait(
            "document.querySelector('.help-toc-toggle').getAttribute('aria-expanded') === 'false' \
             && !document.querySelector('#mdbook-sidebar').checkVisibility()",
        );
        assert_eq!(
            browser.eval("document.activeElement?.classList.contains('help-toc-toggle')"),
            json!(true),
            "focus must stay on the contents button"
        );
    });
}

/// spec: help-site.md:HELP-007
#[test]
fn search_finds_sections_with_every_word_and_opens_the_first_result() {
    // `bodyless` is the one section of each book whose heading comes right before
    // a subsection. `heading_only` is a word in one section's heading that its
    // body does not contain (search rules: result items).
    for (copy, word, both, bodyless, heading_only) in [
        (
            &KO,
            "인증서",
            ["relay.toml", "계정"],
            "운영 › 인증서와 공개 키 파일 교체",
            ("엔드포인트", "참조 › HTTP 엔드포인트"),
        ),
        (
            &EN,
            "certificate",
            ["relay.toml", "account"],
            "Operations › Replace certificate and public key files",
            ("endpoints", "Reference › HTTP endpoints"),
        ),
    ] {
        browse(
            &format!("search-{}", copy.language),
            &copy.preferences,
            |browser, site| {
                browser.navigate(&site.url(&format!("{}/index.html", copy.language)));
                browser.click(".help-search-toggle");
                browser.wait("document.activeElement?.id === 'help-search-input'");

                let found = browser.results(copy, word);
                assert!(!found.is_empty(), "{word}: no results");
                for item in &found {
                    assert!(
                        item.marks.iter().any(|mark| mark.to_lowercase() == word),
                        "{word}: result without a marked match: {item:?}"
                    );
                    assert_eq!(
                        item.excerpt.is_none(),
                        item.label == bodyless,
                        "{word}: only the section without body text shows no excerpt: {item:?}"
                    );
                }
                assert!(
                    found.iter().any(|item| item.label.contains(" › ")),
                    "{word}: no chapter › section label: {found:?}"
                );
                assert!(
                    found.iter().any(|item| item.label == bodyless),
                    "{word}: no result for {bodyless}: {found:?}"
                );
                assert!(
                    found.iter().any(|item| item.excerpt.is_some()),
                    "{word}: no excerpt: {found:?}"
                );

                // A word only in a heading is marked in the label, and the
                // excerpt is the unmarked start of the body.
                let (heading_word, heading_label) = heading_only;
                let headed = browser.results(copy, heading_word);
                let item = headed
                    .iter()
                    .find(|item| item.label == heading_label)
                    .unwrap_or_else(|| {
                        panic!("{heading_word}: no result for {heading_label}: {headed:?}")
                    });
                assert!(
                    item.label_marks
                        .iter()
                        .any(|mark| mark.to_lowercase() == heading_word),
                    "{heading_word}: heading match not marked in the label: {item:?}"
                );
                assert!(
                    item.excerpt
                        .as_deref()
                        .is_some_and(|text| !text.is_empty() && !text.starts_with('…'))
                        && item.excerpt_marks.is_empty(),
                    "{heading_word}: excerpt must be the unmarked start of the body: {item:?}"
                );

                let upper = browser.results(copy, "TOML");
                let lower = browser.results(copy, "toml");
                assert!(!upper.is_empty(), "TOML: no results");
                assert_eq!(
                    hrefs(&upper),
                    hrefs(&lower),
                    "case must not change the results"
                );
                assert!(
                    upper
                        .iter()
                        .all(|item| item.marks.iter().any(|mark| mark.to_lowercase() == "toml")),
                    "TOML: result without a marked match"
                );

                let alone: Vec<Vec<String>> = both
                    .iter()
                    .map(|single| hrefs(&browser.results(copy, single)))
                    .collect();
                // The pair query runs last, so the list on screen holds its results.
                let pair = browser.results(copy, &both.join(" "));
                assert!(!pair.is_empty(), "{both:?}: no results");
                for (single, alone) in both.iter().zip(&alone) {
                    for href in hrefs(&pair) {
                        assert!(
                            alone.contains(&href),
                            "{href} matched both words but not {single} alone"
                        );
                    }
                }

                let first = &pair[0].href;
                browser.click(".help-results li a");
                let target = first
                    .strip_prefix(&site.origin())
                    .unwrap_or(first)
                    .to_owned();
                browser.wait(&format!(
                "location.pathname + location.hash === {} && document.readyState === 'complete'",
                json!(target)
            ));
                assert_eq!(
                browser.eval(
                    "location.hash === '' || !!document.getElementById(decodeURIComponent(location.hash.slice(1)))"
                ),
                json!(true),
                "result section {target} missing"
            );
            },
        );
    }
}

/// spec: help-site.md:HELP-008
#[test]
fn search_waits_for_two_characters_and_reports_no_results() {
    for (copy, single) in [(&KO, "인"), (&EN, "c")] {
        browse(
            &format!("search-empty-{}", copy.language),
            &copy.preferences,
            |browser, site| {
                browser.navigate(&site.url(&format!("{}/index.html", copy.language)));
                browser.click(".help-search-toggle");
                browser.wait("document.activeElement?.id === 'help-search-input'");
                // Load the search data first, so an empty status cannot be the loading state.
                browser.results(copy, "relay");

                browser.query(single);
                browser.wait("document.querySelector('#help-search-status').textContent === ''");
                assert_eq!(
                    browser.eval("document.querySelectorAll('.help-results li').length"),
                    json!(0)
                );
                assert_eq!(
                    browser.eval(
                        "(() => { const help = document.querySelector('#help-search-help'); \
                     return help.checkVisibility() ? help.textContent : null; })()"
                    ),
                    json!(copy.search_help)
                );

                browser.query("zqxjv");
                browser.wait(&format!(
                    "document.querySelector('#help-search-status').textContent === {}",
                    json!(copy.no_results("zqxjv"))
                ));
                assert_eq!(
                    browser.eval("document.querySelector('#help-search-input').value"),
                    json!("zqxjv")
                );
            },
        );
    }
}

/// spec: help-site.md:HELP-009
#[test]
fn search_reports_failed_data_and_reloads_it_when_reopened() {
    browse("search-failure", &KO.preferences, |browser, site| {
        browser.navigate(&site.url("ko/index.html"));
        browser.command(
            "Network.setBlockedURLs",
            json!({"urls": [site.url("ko/install.html")]}),
        );
        browser.click(".help-search-toggle");
        browser.wait("document.activeElement?.id === 'help-search-input'");
        browser.query("인증서");
        browser.wait(&format!(
            "document.querySelector('#help-search-status').textContent === {}",
            json!(KO.failed)
        ));
        assert_eq!(
            browser.eval("document.querySelectorAll('.help-results li').length"),
            json!(0)
        );

        browser.command("Network.setBlockedURLs", json!({"urls": []}));
        browser.press("Escape");
        browser.wait("document.querySelector('#help-search').hidden");
        browser.click(".help-search-toggle");
        browser.wait(&format!(
            "/^검색 결과 \\d+건$/.test(document.querySelector('#help-search-status').textContent) \
             && document.querySelector('#help-search-input').value === {}",
            json!("인증서")
        ));
    });
}

/// spec: help-site.md:HELP-010
#[test]
fn keyboard_reaches_every_control_with_a_ring_and_moves_between_chapters() {
    browse("keyboard", &KO.preferences, |browser, site| {
        browser.size(1280, 900, 1.0);
        browser.navigate(&site.url("ko/configure.html"));
        // A fresh page starts the tab order at the top of the document. The walk
        // ends when focus leaves the page or comes back to the first control.
        let mut areas = Vec::new();
        let mut names = Vec::new();
        for _ in 0..400 {
            browser.press("Tab");
            let focus = browser.focus();
            if focus["left"] == json!(true) || (!areas.is_empty() && focus["first"] == json!(true))
            {
                break;
            }
            assert_eq!(
                focus["ring"],
                json!(true),
                "no focus ring on {}",
                focus["name"]
            );
            if areas.is_empty() {
                assert_eq!(
                    focus["first"],
                    json!(true),
                    "first stop must be the contents button, not {}",
                    focus["name"]
                );
            }
            areas.push(focus["area"].as_str().unwrap_or("other").to_owned());
            names.push(focus["name"].as_str().unwrap_or_default().to_owned());
        }
        assert!(areas.len() < 400, "focus never left the page");
        let first = |name: &str| areas.iter().position(|area| area == name);
        let last = |name: &str| areas.iter().rposition(|area| area == name);
        assert!(
            first("main").is_some() && last("top") < first("toc") && last("toc") < first("main"),
            "tab order must go top bar, contents, text: {areas:?}"
        );
        // The walk also reaches the controls inside the text: a code copy button
        // and both chapter links of the pager.
        for (control, start) in [
            (
                "code copy button",
                format!("BUTTON.btn btn-outline sm {}", KO.copy),
            ),
            (
                "previous chapter link",
                "A.help-pager-link prev ".to_owned(),
            ),
            ("next chapter link", "A.help-pager-link next ".to_owned()),
        ] {
            assert!(
                names.iter().any(|name| name.starts_with(&start)),
                "the tab walk never reached the {control}: {names:?}"
            );
        }

        browser.press("/");
        browser.wait("document.activeElement?.id === 'help-search-input'");
        assert_eq!(
            browser.focus()["ring"],
            json!(true),
            "no focus ring on the search input"
        );
        browser.press("ArrowRight");
        thread::sleep(Duration::from_millis(300));
        assert_eq!(
            browser.eval("location.pathname"),
            json!(format!("{MOUNT}ko/configure.html")),
            "arrow keys inside the search input must not change the chapter"
        );
        browser.press("ShiftTab");
        let close = browser.focus();
        assert!(
            close["name"]
                .as_str()
                .unwrap_or_default()
                .contains(KO.close)
                && close["ring"] == json!(true),
            "search close button must take focus with a ring: {close}"
        );
        browser.press("Escape");
        browser.wait(
            "document.querySelector('#help-search').hidden \
             && document.activeElement?.classList.contains('help-search-toggle')",
        );
        browser.press("ArrowRight");
        browser.wait(&at(&format!("{MOUNT}ko/dashboard.html")));
        browser.press("ArrowLeft");
        browser.wait(&at(&format!("{MOUNT}ko/configure.html")));
    });
}

/// spec: help-site.md:HELP-011
#[test]
fn site_texts_and_shortcut_help_follow_the_page_language() {
    for copy in COPIES {
        browse(
            &format!("texts-{}", copy.language),
            &copy.preferences,
            |browser, site| {
                browser.size(1280, 900, 1.0);
                browser.navigate(&site.url(&format!("{}/configure.html", copy.language)));
                let texts = browser.eval(
                "(() => { const t = (s) => document.querySelector(s)?.textContent.trim() ?? null;
                  return {
                    brand: t('.help-brand'), base: t('.help-base'), toc: t('.help-toc-toggle'),
                    tocName: document.querySelector('#mdbook-sidebar').getAttribute('aria-label'),
                    search: t('.help-search-toggle'), language: t('.help-lang'),
                    pager: document.querySelector('nav.help-pager')?.getAttribute('aria-label') ?? null,
                    previous: t('.help-pager-link.prev .help-pager-dir'),
                    next: t('.help-pager-link.next .help-pager-dir'),
                    copy: t('.help-code button') }; })()",
            );
                assert_eq!(
                    texts,
                    json!({
                        "brand": copy.brand, "base": copy.base, "toc": copy.toc, "tocName": copy.toc,
                        "search": copy.search, "language": copy.other_language, "pager": copy.pager,
                        "previous": copy.previous, "next": copy.next, "copy": copy.copy,
                    })
                );

                browser.click(".help-search-toggle");
                browser.wait("document.activeElement?.id === 'help-search-input'");
                assert_eq!(
                browser.eval(
                    "(() => { const input = document.querySelector('#help-search-input');
                      return { label: input.labels[0]?.textContent ?? null,
                        close: document.querySelector('.help-search-head button').textContent.trim(),
                        help: document.querySelector('#help-search-help').textContent }; })()"
                ),
                json!({"label": copy.search_title, "close": copy.close, "help": copy.search_help})
            );
                browser.press("Escape");
                browser.wait("document.querySelector('#help-search').hidden");

                browser.press("?");
                browser.wait("document.activeElement?.id === 'mdbook-help-popup'");
                let dialog = browser.eval(
                    "(() => { const popup = document.querySelector('#mdbook-help-popup');
                  const title = document.getElementById(popup.getAttribute('aria-labelledby'));
                  return { role: popup.getAttribute('role'), title: title?.textContent ?? null,
                    keys: [...popup.querySelectorAll('li')].map((li) =>
                      [[...li.querySelectorAll('kbd')].map((k) => k.textContent).join(''),
                       li.lastElementChild.textContent]) }; })()",
                );
                assert_eq!(
                    dialog,
                    json!({
                        "role": "dialog", "title": copy.keys_title,
                        "keys": copy.keys.iter().map(|(keys, text)| json!([keys, text])).collect::<Vec<_>>(),
                    })
                );
                if copy.language == "ko" {
                    let visible = browser.eval(
                        "(() => { const names = [];
                      for (const e of document.querySelectorAll('*')) {
                        if (!e.checkVisibility()) continue;
                        for (const a of ['aria-label', 'title', 'alt']) {
                          if (e.hasAttribute(a)) names.push(e.getAttribute(a));
                        }
                      }
                      return names.join('\\n') + '\\n' + document.body.innerText; })()",
                    );
                    let visible = visible.as_str().unwrap_or_default();
                    for english in MDBOOK_ENGLISH {
                        assert!(
                            !visible.contains(english),
                            "mdBook English left on a Korean page: {english}"
                        );
                    }
                }
                browser.press("Escape");
                browser.wait("getComputedStyle(document.querySelector('#mdbook-help-container')).display === 'none'");
            },
        );
    }
}

/// spec: help-site.md:HELP-012
#[test]
fn copy_button_copies_the_code_and_reports_a_refused_clipboard() {
    browse("copy", &KO.preferences, |browser, site| {
        browser.navigate(&site.url("ko/configure.html"));
        browser.command(
            "Emulation.setFocusEmulationEnabled",
            json!({"enabled": true}),
        );
        browser.command(
            "Browser.grantPermissions",
            json!({"origin": site.origin(), "permissions": ["clipboardReadWrite", "clipboardSanitizedWrite"]}),
        );
        browser.click(".help-code button");
        browser.wait(&format!(
            "document.querySelector('.help-copy-status')?.textContent === {}",
            json!(KO.copied)
        ));
        let copied = browser.command(
            "Runtime.evaluate",
            json!({"expression": "navigator.clipboard.readText()", "awaitPromise": true,
                "returnByValue": true, "userGesture": true}),
        );
        let copied = copied["result"]["value"]
            .as_str()
            .unwrap_or_default()
            .replace("\r\n", "\n");
        let code = browser.eval(
            "(document.querySelector('.help-code pre code') ?? document.querySelector('.help-code pre')).textContent",
        );
        assert_eq!(
            copied,
            code.as_str().unwrap_or_default().replace("\r\n", "\n")
        );

        browser.command("Browser.resetPermissions", json!({}));
        for name in ["clipboard-write", "clipboard-read"] {
            browser.command(
                "Browser.setPermission",
                json!({"origin": site.origin(), "permission": {"name": name}, "setting": "denied"}),
            );
        }
        browser.click(".help-code button");
        browser.wait(&format!(
            "document.querySelector('.help-copy-status')?.textContent === {}",
            json!(KO.copy_failed)
        ));
    });
}

/// spec: help-site.md:HELP-013
#[test]
fn baseline_link_opens_the_section_on_main_and_its_release_differences() {
    for copy in COPIES {
        browse(
            &format!("baseline-{}", copy.language),
            &copy.preferences,
            |browser, site| {
                browser.size(1280, 900, 1.0);
                browser.navigate(&site.url(&format!("{}/install.html", copy.language)));
                browser.click(".help-base");
                browser.wait(&format!(
                    "location.pathname === {} && decodeURIComponent(location.hash) === {} \
                 && document.readyState === 'complete'",
                    json!(format!("{MOUNT}{}/index.html", copy.language)),
                    json!(format!("#{}", copy.baseline))
                ));
                let section = browser.eval(&format!(
                "(() => {{ const parts = [];
                  for (let node = document.getElementById({})?.nextElementSibling;
                       node && node.tagName !== 'H2'; node = node.nextElementSibling) parts.push(node);
                  return {{
                    main: parts.some((n) => [...n.querySelectorAll('code')].some((c) => c.textContent === 'main')),
                    latest: parts.some((n) => !!n.querySelector('a[href=\"https://github.com/kayden-kim/vnc-relay/releases/latest\"]')),
                    differences: parts.reduce((sum, n) => sum + n.querySelectorAll('tbody tr').length, 0) }}; }})()",
                json!(copy.baseline)
            ));
                assert_eq!(
                    section["main"],
                    json!(true),
                    "{}: main baseline missing",
                    copy.language
                );
                assert_eq!(
                    section["latest"],
                    json!(true),
                    "{}: latest Release link missing",
                    copy.language
                );
                assert!(
                    section["differences"].as_u64().unwrap_or(0) >= 1,
                    "{}: difference list missing",
                    copy.language
                );
            },
        );
    }
}

/// spec: help-site.md:HELP-014
#[test]
fn missing_paths_show_the_styled_not_found_page_with_both_home_links() {
    browse("not-found", &EN.preferences, |browser, site| {
        for path in ["ko/missing.html", "a/b/c"] {
            browser.navigate(&site.url(path));
            let page = browser.eval(
                "(async () => { await document.fonts.ready;
                  const resources = performance.getEntriesByType('resource');
                  return { status: performance.getEntriesByType('navigation')[0].responseStatus,
                    text: document.body.innerText,
                    styles: resources.filter((r) => r.name.endsWith('.css')).length,
                    fonts: [...document.fonts].filter((f) => f.status === 'loaded').length,
                    failed: resources.filter((r) => r.responseStatus !== 200).map((r) => r.name) }; })()",
            );
            assert_eq!(page["status"], json!(404), "{path}: status");
            let text = page["text"].as_str().unwrap_or_default();
            for expected in [
                "페이지를 찾을 수 없습니다",
                "Page not found",
                "주소가 바뀌었거나 없는 페이지입니다. 도움말 첫 페이지에서 다시 찾으십시오.",
                "The page has moved or does not exist. Start again from the help home page.",
                "한국어 도움말 첫 페이지",
                "English help home",
            ] {
                assert!(text.contains(expected), "{path}: {expected} missing");
            }
            assert!(
                page["styles"].as_u64().unwrap_or(0) >= 2,
                "{path}: style sheets: {page}"
            );
            assert!(
                page["fonts"].as_u64().unwrap_or(0) >= 1,
                "{path}: no font loaded: {page}"
            );
            assert_eq!(page["failed"], json!([]), "{path}: failed requests");
        }
        browser.click("a[data-lang=ko]");
        browser.wait(&at(&format!("{MOUNT}ko/")));
        browser.navigate(&site.url("a/b/c"));
        browser.click("a[data-lang=en]");
        browser.wait(&at(&format!("{MOUNT}en/")));
    });
}

/// spec: help-site.md:HELP-015
#[test]
fn every_screen_requests_only_the_site_origin() {
    browse("same-origin", &KO.preferences, |browser, site| {
        let origin = format!("{}/", site.origin());
        let mut pages: Vec<String> = Vec::new();
        for language in LANGUAGES {
            pages.extend(
                chapters(language)
                    .iter()
                    .map(|chapter| format!("{language}/{chapter}")),
            );
        }
        pages.push("ko/missing.html".to_owned());
        for page in &pages {
            browser.navigate(&site.url(page));
            if page == "ko/index.html" {
                // The search data requests are part of the page too.
                browser.click(".help-search-toggle");
                browser.results(&KO, "relay");
            }
            let names = browser.eval(
                "(async () => { await document.fonts.ready;
                  return performance.getEntries().filter((e) => e.entryType === 'navigation' || e.entryType === 'resource')
                    .map((e) => e.name); })()",
            );
            for name in names.as_array().into_iter().flatten() {
                let name = name.as_str().unwrap_or_default();
                assert!(
                    name.starts_with(&origin) || name.starts_with("data:"),
                    "{page}: request outside the site origin: {name}"
                );
            }
        }
        // The root page moves on at once, so its requests come from the CDP log.
        browser.go(&site.url(""));
        browser.wait(&at(&format!("{MOUNT}ko/")));
        for event in browser.network().as_array().into_iter().flatten() {
            if let Some(url) = event["url"].as_str() {
                assert!(
                    url.starts_with(&origin) || url.starts_with("data:") || url == "about:blank",
                    "request outside the site origin: {url}"
                );
            }
        }
    });
}

/// spec: help-site.md:HELP-016
#[test]
fn every_link_and_picture_of_both_books_resolves() {
    browse("links", &KO.preferences, |browser, site| {
        let origin = site.origin();
        for language in LANGUAGES {
            for chapter in chapters(language) {
                let page = format!("{language}/{chapter}");
                browser.navigate(&site.url(&page));
                let found = browser.eval(
                    "(async () => {
                      await Promise.all([...document.images].map((i) => i.decode().catch(() => null)));
                      return {
                        images: [...document.querySelectorAll('main img')].map((i) => ({ src: i.src, alt: i.alt,
                          shown: i.checkVisibility(), loaded: i.complete && i.naturalWidth > 0 })),
                        links: [...document.querySelectorAll('a[href]')].map((a) => a.href) }; })()",
                );
                for image in found["images"].as_array().into_iter().flatten() {
                    assert!(
                        !image["alt"].as_str().unwrap_or_default().trim().is_empty(),
                        "{page}: empty alternative text: {image}"
                    );
                    if image["shown"] == json!(true) {
                        assert_eq!(
                            image["loaded"],
                            json!(true),
                            "{page}: picture not loaded: {image}"
                        );
                    }
                }
                for link in found["links"].as_array().into_iter().flatten() {
                    let link = link.as_str().unwrap_or_default();
                    if let Some(rest) = link.strip_prefix(&format!("{origin}{MOUNT}")) {
                        site.assert_target(&page, rest);
                    } else if let Some(path) = link.strip_prefix(REPOSITORY_BLOB) {
                        let path = decode(path.split(['#', '?']).next().unwrap_or_default());
                        assert!(
                            !path.contains("..") && repository().join(&path).exists(),
                            "{page}: repository link to a missing path: {link}"
                        );
                    } else {
                        assert!(
                            link.starts_with("https://"),
                            "{page}: link is not https: {link}"
                        );
                    }
                }
            }
        }
    });
}

/// spec: help-site.md:HELP-017
#[test]
fn notices_name_every_shipped_component_with_version_license_and_source() {
    let shipped = shipped_components(&SITE.root);
    browse("notices", &KO.preferences, |browser, site| {
        for language in LANGUAGES {
            let page = format!("{language}/notices.html");
            browser.navigate(&site.url(&page));
            let rows = browser.eval(
                "[...document.querySelector('main table').querySelectorAll('tbody tr')].map((tr) =>
                   [...tr.cells].map((td) => ({ text: td.textContent.trim(),
                     href: td.querySelector('a')?.href ?? null })))",
            );
            let rows = rows.as_array().cloned().unwrap_or_default();
            for component in &shipped {
                assert!(
                    rows.iter().any(|row| row[0]["text"]
                        .as_str()
                        .unwrap_or_default()
                        .contains(component)),
                    "{page}: shipped component {component} is not listed"
                );
            }
            for row in &rows {
                let cell = |index: usize| row[index]["text"].as_str().unwrap_or_default();
                assert!(!cell(0).is_empty(), "{page}: component name missing: {row}");
                assert!(
                    cell(1).chars().next().is_some_and(|c| c.is_ascii_digit())
                        && cell(1).contains('.'),
                    "{page}: version missing: {row}"
                );
                assert!(!cell(2).is_empty(), "{page}: license missing: {row}");
                assert!(
                    row[3]["href"]
                        .as_str()
                        .is_some_and(|href| href.starts_with("https://")),
                    "{page}: source link missing: {row}"
                );
                let text = row[4]["href"]
                    .as_str()
                    .unwrap_or_else(|| panic!("{page}: license text link missing: {row}"));
                if let Some(rest) = text.strip_prefix(&format!("{}{MOUNT}", site.origin())) {
                    site.assert_target(&page, rest);
                } else {
                    assert!(
                        text.starts_with("https://"),
                        "{page}: license text link: {text}"
                    );
                }
            }
            let section = browser.eval(
                "(() => { const id = [...document.querySelectorAll('main h2')].find((h) => h.textContent.includes('clipboard.js'))?.id;
                  const parts = [];
                  for (let node = document.getElementById(id)?.nextElementSibling; node && node.tagName !== 'H2'; node = node.nextElementSibling) parts.push(node.textContent);
                  return parts.join(' '); })()",
            );
            let section = section.as_str().unwrap_or_default();
            assert!(
                section.contains("MIT License") && section.contains("Permission is hereby granted"),
                "{page}: clipboard.js license text missing"
            );
        }
    });
}

/// spec: help-site.md:HELP-018
#[test]
fn narrow_and_zoomed_pages_scroll_only_inside_code_and_tables() {
    browse("widths", &KO.preferences, |browser, site| {
        let mut pages: Vec<String> = Vec::new();
        for language in LANGUAGES {
            pages.extend(
                chapters(language)
                    .iter()
                    .map(|chapter| format!("{language}/{chapter}")),
            );
        }
        pages.push("ko/missing.html".to_owned());
        // 320px wide, and a 1280px window at 200% zoom: 640 CSS pixels at scale 2.
        for (width, height, scale) in [(320, 640, 1.0), (640, 450, 2.0)] {
            browser.size(width, height, scale);
            for page in &pages {
                browser.navigate(&site.url(page));
                browser.assert_no_page_scroll(&format!("{page} at {width}px x{scale}"));
            }
        }
        browser.command(
            "Emulation.setScriptExecutionDisabled",
            json!({"value": true}),
        );
        for (width, height, scale) in [(320, 640, 1.0), (640, 450, 2.0)] {
            browser.size(width, height, scale);
            browser.navigate(&site.url(""));
            browser.assert_no_page_scroll(&format!("root at {width}px x{scale}"));
        }
    });
}

/// One search result: link, label, marked words, and excerpt.
#[derive(Debug)]
struct Found {
    href: String,
    label: String,
    /// Marked words of the label and the excerpt.
    marks: Vec<String>,
    /// Marked words of the label only.
    label_marks: Vec<String>,
    /// Excerpt text. `None` when the result shows only its link.
    excerpt: Option<String>,
    /// Marked words of the excerpt only.
    excerpt_marks: Vec<String>,
}

fn hrefs(found: &[Found]) -> Vec<String> {
    found.iter().map(|item| item.href.clone()).collect()
}

impl Browser {
    /// Starts a navigation without waiting for its document. The help root
    /// replaces itself at once, so its own document may never be seen.
    fn go(&mut self, url: &str) {
        let result = self.command("Page.navigate", json!({"url": url}));
        assert!(
            result.get("errorText").is_none(),
            "browser navigation failed: {}",
            result["errorText"]
        );
    }

    fn size(&mut self, width: u32, height: u32, scale: f64) {
        self.command(
            "Emulation.setDeviceMetricsOverride",
            json!({"width": width, "height": height, "deviceScaleFactor": scale, "mobile": false}),
        );
    }

    /// Clicks the middle of the first match with real mouse events.
    fn click(&mut self, selector: &str) {
        let selector = json!(selector);
        self.wait(&format!(
            "(() => {{ const e = document.querySelector({selector}); return !!e && e.getBoundingClientRect().width > 0; }})()"
        ));
        let point = self.eval(&format!(
            "(() => {{ const e = document.querySelector({selector}); e.scrollIntoView({{block: 'center'}});
              const r = e.getBoundingClientRect(); return {{ x: r.x + r.width / 2, y: r.y + r.height / 2 }}; }})()"
        ));
        let (x, y) = (
            point["x"].as_f64().unwrap_or(0.0),
            point["y"].as_f64().unwrap_or(0.0),
        );
        for kind in ["mousePressed", "mouseReleased"] {
            self.command(
                "Input.dispatchMouseEvent",
                json!({"type": kind, "x": x, "y": y, "button": "left", "clickCount": 1}),
            );
        }
    }

    /// Presses one key as a keyboard user does. `ShiftTab` is Tab with Shift.
    fn press(&mut self, name: &str) {
        let (key, code, virtual_key, text, modifiers) = match name {
            "/" => ("/", "Slash", 191, "/", 0),
            "?" => ("?", "Slash", 191, "?", 8),
            "Escape" => ("Escape", "Escape", 27, "", 0),
            "Tab" => ("Tab", "Tab", 9, "", 0),
            "ShiftTab" => ("Tab", "Tab", 9, "", 8),
            "ArrowLeft" => ("ArrowLeft", "ArrowLeft", 37, "", 0),
            "ArrowRight" => ("ArrowRight", "ArrowRight", 39, "", 0),
            _ => panic!("unsupported key {name}"),
        };
        let event = |kind: &str| {
            json!({"type": kind, "key": key, "code": code, "windowsVirtualKeyCode": virtual_key,
                "nativeVirtualKeyCode": virtual_key, "modifiers": modifiers, "text": text,
                "unmodifiedText": text})
        };
        self.command(
            "Input.dispatchKeyEvent",
            event(if text.is_empty() {
                "rawKeyDown"
            } else {
                "keyDown"
            }),
        );
        self.command("Input.dispatchKeyEvent", event("keyUp"));
    }

    /// Where focus is: its page area, whether a focus ring shows, and a name for messages.
    fn focus(&mut self) -> serde_json::Value {
        self.eval(
            "(() => { const e = document.activeElement;
              if (!e || e === document.body) return { left: true };
              const s = getComputedStyle(e);
              const area = e.closest('.help-top') ? 'top' : e.closest('#mdbook-sidebar') ? 'toc'
                : e.closest('main') ? 'main' : 'other';
              return { left: false, area, first: e.classList.contains('help-toc-toggle'),
                ring: s.outlineStyle !== 'none' && parseFloat(s.outlineWidth) > 0,
                name: e.tagName + '.' + e.className + ' ' + (e.textContent || '').trim().slice(0, 30) }; })()",
        )
    }

    /// Replaces the search input with `query`, as typing does.
    fn query(&mut self, query: &str) {
        self.eval("(() => { const input = document.querySelector('#help-search-input'); input.focus(); input.select(); return true; })()");
        self.command("Input.insertText", json!({"text": query}));
    }

    /// Searches and returns the results once the result count is shown.
    fn results(&mut self, copy: &Copy, query: &str) -> Vec<Found> {
        self.query(query);
        let pattern = if copy.language == "ko" {
            "^검색 결과 \\d+건$"
        } else {
            "^\\d+ results?$"
        };
        self.wait(&format!(
            "(() => {{ const status = document.querySelector('#help-search-status').textContent;
              return new RegExp({}).test(status) || status === {}; }})()",
            json!(pattern),
            json!(copy.no_results(query))
        ));
        let found = self.eval(
            "({ status: document.querySelector('#help-search-status').textContent,
               items: [...document.querySelectorAll('.help-results li')].map((li) => {
                 const link = li.querySelector('a');
                 const excerpt = li.querySelector('p');
                 const marks = (node) => node ? [...node.querySelectorAll('mark')].map((m) => m.textContent) : [];
                 return { href: link.href, label: link.textContent, marks: marks(li),
                   labelMarks: marks(link), excerpt: excerpt ? excerpt.textContent : null,
                   excerptMarks: marks(excerpt) }; }) })",
        );
        let strings = |value: &serde_json::Value| -> Vec<String> {
            value
                .as_array()
                .into_iter()
                .flatten()
                .map(|text| text.as_str().unwrap_or_default().to_owned())
                .collect()
        };
        let items: Vec<Found> = found["items"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|item| Found {
                href: item["href"].as_str().unwrap_or_default().to_owned(),
                label: item["label"].as_str().unwrap_or_default().to_owned(),
                marks: strings(&item["marks"]),
                label_marks: strings(&item["labelMarks"]),
                excerpt: item["excerpt"].as_str().map(str::to_owned),
                excerpt_marks: strings(&item["excerptMarks"]),
            })
            .collect();
        let status = found["status"].as_str().unwrap_or_default();
        if !items.is_empty() {
            assert_eq!(
                status,
                copy.results(items.len()),
                "{query}: result count text"
            );
        }
        items
    }

    /// The page has no horizontal scroll. Code blocks and tables stay inside the
    /// viewport and scroll inside their own box when they are wider.
    fn assert_no_page_scroll(&mut self, label: &str) {
        let state = self.eval(
            "(async () => { await document.fonts.ready; const root = document.documentElement;
              const boxes = [...document.querySelectorAll('pre, .help-table')].filter((e) => e.checkVisibility());
              return { page: root.scrollWidth - root.clientWidth,
                outside: boxes.filter((e) => e.getBoundingClientRect().right > root.clientWidth + 0.5).length,
                stuck: boxes.filter((e) => e.scrollWidth > e.clientWidth
                  && !['auto', 'scroll'].includes(getComputedStyle(e).overflowX)).length }; })()",
        );
        assert_eq!(
            state,
            json!({"page": 0, "outside": 0, "stuck": 0}),
            "{label}: horizontal scroll"
        );
    }
}

/// The built site and the loopback server that serves it.
struct Site {
    root: PathBuf,
    port: u16,
}

impl Site {
    fn origin(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    fn url(&self, path: &str) -> String {
        format!("{}{MOUNT}{path}", self.origin())
    }

    /// A site path (below the mount, maybe with a fragment) names a file, and
    /// the fragment names an element of that page.
    fn assert_target(&self, page: &str, rest: &str) {
        let (path, fragment) = rest.split_once('#').unwrap_or((rest, ""));
        let path = decode(path.split('?').next().unwrap_or_default());
        let mut file = self.root.join(&path);
        if path.is_empty() || path.ends_with('/') || file.is_dir() {
            file = file.join("index.html");
        }
        assert!(file.is_file(), "{page}: link to a missing page: {rest}");
        if !fragment.is_empty() {
            let id = decode(fragment);
            assert!(
                read(&file).contains(&format!("id=\"{id}\"")),
                "{page}: link to a missing section: {rest}"
            );
        }
    }
}

/// Built once for the whole test binary. A failed build fails every scenario.
static SITE: LazyLock<Site> = LazyLock::new(|| {
    let root = build();
    let port = serve(root.clone());
    Site { root, port }
});

fn repository() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Builds the site with `cargo xtask help-site` and returns its directory.
fn build() -> PathBuf {
    // Cargo releases its build lock before executing tests. This subprocess
    // cannot access the network; dependencies must be fetched during setup.
    let output = Command::new(env!("CARGO"))
        .current_dir(repository())
        .args([
            "run",
            "--offline",
            "--locked",
            "--quiet",
            "-p",
            "xtask",
            "--",
            "help-site",
        ])
        .output()
        .unwrap_or_else(|error| panic!("runs cargo xtask help-site: {error}"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "cargo xtask help-site failed: {stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // The last line is `help-site: 통과 (<site directory>)`.
    let shown = stdout
        .lines()
        .rev()
        .find_map(|line| line.strip_prefix("help-site: 통과 (")?.strip_suffix(')'))
        .unwrap_or_else(|| panic!("help-site result line missing: {stdout}"));
    fs::canonicalize(repository().join(shown))
        .unwrap_or_else(|error| panic!("site directory {shown}: {error}"))
}

/// Chapter pages of a book in contents order, from its `SUMMARY.md`.
fn chapters(language: &str) -> Vec<String> {
    let summary = read(
        &repository()
            .join("docs/help")
            .join(language)
            .join("src/SUMMARY.md"),
    );
    summary
        .lines()
        .filter_map(|line| {
            let target = line.split_once("](")?.1.split_once(')')?.0;
            target
                .strip_suffix(".md")
                .map(|name| format!("{name}.html"))
        })
        .collect()
}

/// Third-party parts that the built site carries, by the name its notices use.
fn shipped_components(root: &Path) -> Vec<&'static str> {
    let files: Vec<String> = fs::read_dir(root.join("ko"))
        .unwrap_or_else(|error| panic!("read built book: {error}"))
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    let has = |prefix: &str, suffix: &str| {
        files
            .iter()
            .any(|name| name.starts_with(prefix) && name.ends_with(suffix))
    };
    let mut shipped = Vec::new();
    if has("book-", ".js") {
        shipped.push("mdBook");
    }
    if has("highlight-", ".js") {
        shipped.push("highlight.js");
    }
    if has("clipboard-", ".js") {
        shipped.push("clipboard.js");
    }
    if read(&root.join("ko/index.html")).contains("fa-svg") {
        shipped.push("Font Awesome");
    }
    if root.join("assets/fonts").is_dir() {
        shipped.push("Pretendard");
    }
    shipped
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

/// `location.pathname` is `path` and the page has loaded.
fn at(path: &str) -> String {
    format!(
        "location.pathname === {} && document.readyState === 'complete'",
        json!(path)
    )
}

fn expanded(browser: &mut Browser) -> String {
    browser
        .eval("document.querySelector('.help-toc-toggle').getAttribute('aria-expanded')")
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

fn content_width(browser: &mut Browser) -> f64 {
    browser
        .eval("document.querySelector('#mdbook-content').getBoundingClientRect().width")
        .as_f64()
        .unwrap_or(f64::NAN)
}

/// Decodes `%XX` escapes. Invalid escapes stay as they are.
fn decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let escaped = (bytes[index] == b'%')
            .then(|| bytes.get(index + 1..index + 3))
            .flatten()
            .and_then(|hex| u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok());
        if let Some(byte) = escaped {
            out.push(byte);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

static RUNNING: Mutex<usize> = Mutex::new(0);
static FREED: Condvar = Condvar::new();

/// One of the `MAX_BROWSERS` places, held until the browser has exited.
struct Slot;

impl Slot {
    fn take() -> Self {
        let mut running = RUNNING.lock().unwrap_or_else(PoisonError::into_inner);
        while *running >= MAX_BROWSERS {
            running = FREED.wait(running).unwrap_or_else(PoisonError::into_inner);
        }
        *running += 1;
        Slot
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        *RUNNING.lock().unwrap_or_else(PoisonError::into_inner) -= 1;
        FREED.notify_one();
    }
}

/// Runs `check` in a new headless browser with the given language preferences.
fn browse(name: &str, preferences: &[&str], check: impl FnOnce(&mut Browser, &Site)) {
    let site = &*SITE;
    let _slot = Slot::take();
    // Declared after the slot, so the browser exits before the slot is freed.
    let mut browser = Browser::start(&format!("help-{name}"), preferences);
    check(&mut browser, site);
}

/// Serves `root` under `MOUNT` on a free loopback port until the test process exits.
fn serve(root: PathBuf) -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .unwrap_or_else(|error| panic!("bind 127.0.0.1: {error}"));
    let port = listener
        .local_addr()
        .unwrap_or_else(|error| panic!("read the bound address: {error}"))
        .port();
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let root = root.clone();
            thread::spawn(move || {
                let _ = answer(stream, &root);
            });
        }
    });
    port
}

/// Answers one request like GitHub Pages, then closes the connection.
fn answer(mut stream: TcpStream, root: &Path) -> io::Result<()> {
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    let deadline = Instant::now() + IO_TIMEOUT;
    let mut head = Vec::new();
    let mut chunk = [0_u8; 1024];
    while !head.windows(4).any(|window| window == b"\r\n\r\n") {
        let read = stream.read(&mut chunk)?;
        if read == 0 || head.len() > MAX_HEAD || Instant::now() > deadline {
            // An idle preconnection or a broken request: nothing to answer.
            return Ok(());
        }
        head.extend_from_slice(&chunk[..read]);
    }
    let line = String::from_utf8_lossy(&head);
    let mut parts = line.split(' ');
    let (method, target) = (
        parts.next().unwrap_or_default(),
        parts.next().unwrap_or_default(),
    );
    let path = target.split(['?', '#']).next().unwrap_or_default();
    let send_body = method != "HEAD";
    if path == "/" || path == MOUNT.trim_end_matches('/') {
        return reply(
            &mut stream,
            "302 Found",
            &[("Location", MOUNT)],
            b"",
            send_body,
        );
    }
    let Some(rest) = path.strip_prefix(MOUNT) else {
        return reply(&mut stream, "404 Not Found", &[], b"", send_body);
    };
    let decoded = decode(rest);
    let mut file = root.to_path_buf();
    let safe = !decoded.contains(['\\', '\0', ':']) && decoded.split('/').all(|part| part != "..");
    for part in decoded.split('/').filter(|part| !part.is_empty()) {
        file.push(part);
    }
    if safe && file.is_dir() {
        if !path.ends_with('/') {
            let location = format!("{path}/");
            return reply(
                &mut stream,
                "302 Found",
                &[("Location", &location)],
                b"",
                send_body,
            );
        }
        file.push("index.html");
    }
    match (safe, content_type(&file), fs::read(&file)) {
        (true, Some(kind), Ok(body)) => reply(
            &mut stream,
            "200 OK",
            &[("Content-Type", kind)],
            &body,
            send_body,
        ),
        _ => {
            let body = fs::read(root.join("404.html"))?;
            let kind = [("Content-Type", "text/html; charset=utf-8")];
            reply(&mut stream, "404 Not Found", &kind, &body, send_body)
        }
    }
}

fn content_type(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    Some(match extension.as_str() {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "woff2" => "font/woff2",
        "txt" => "text/plain; charset=utf-8",
        _ => return None,
    })
}

fn reply(
    stream: &mut TcpStream,
    status: &str,
    extra: &[(&str, &str)],
    body: &[u8],
    send_body: bool,
) -> io::Result<()> {
    let mut head = format!("HTTP/1.1 {status}\r\n");
    for (name, value) in extra {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str(&format!(
        "Cache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    ));
    stream.write_all(head.as_bytes())?;
    if send_body {
        stream.write_all(body)?;
    }
    stream.flush()
}
