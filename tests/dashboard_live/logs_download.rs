//! Actual Chromium downloads from the real recent-log API, including aborts.

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

use super::live::{Browser, Fault, LIMIT, must, wait_for};
use super::logs::{generate, items, open, pause, ready, select_event, start};

fn files(dir: &Path) -> Vec<std::path::PathBuf> {
    must(fs::read_dir(dir))
        .map(|entry| must(entry).path())
        .collect()
}

pub fn complete_json_lines_snapshot_and_no_partial_files() {
    let server = start("dash-060", 60, 10000);
    let admin = server.login();
    generate(&server, 1005);
    let mut browser = Browser::start("dash-060", &["ko-KR"]);
    open(&mut browser, &server);
    pause(&mut browser);
    select_event(&mut browser, "connection_rejected");
    let dir = server.dir().join("downloads");
    must(fs::create_dir(&dir));
    browser.command(
        "Browser.setDownloadBehavior",
        json!({"behavior":"allow","downloadPath":dir,"eventsEnabled":true}),
    );
    let first_path = "/api/v1/logs?event=connection_rejected&limit=1000";
    let first = server.query(&admin, first_path);
    assert_eq!(items(&first).len(), 1000);
    assert_eq!(first["has_more"], true);
    let cursor = &items(&first)[0]["seq"];
    let older_path = format!("{first_path}&before={cursor}");
    let older = server.query(&admin, &older_path);
    assert_eq!(older["has_more"], false);
    assert!(items(&older).len() >= 5);
    let mut expected = items(&older).to_vec();
    expected.extend_from_slice(items(&first));
    let release = Arc::new(AtomicBool::new(false));
    server
        .proxy
        .once("GET", &older_path, Fault::Hold(Arc::clone(&release)));
    browser.click("#logs-download");
    wait_for("second real download page held", LIMIT, || {
        server
            .proxy
            .traffic()
            .iter()
            .any(|r| r.path == older_path && r.stage == "response_held")
    });
    assert!(
        files(&dir).is_empty(),
        "no file before the entire collection"
    );
    generate(&server, 3);
    release.store(true, Ordering::Release);
    wait_for("complete actual browser download", LIMIT, || {
        files(&dir)
            .iter()
            .any(|p| p.extension().is_some_and(|e| e == "jsonl"))
    });
    wait_for("browser download completed event", LIMIT, || {
        browser.eval("true");
        browser.network().as_array().is_some_and(|events| {
            events
                .iter()
                .any(|e| e["event"] == "Browser.downloadProgress" && e["state"] == "completed")
        })
    });
    let paths = files(&dir);
    assert_eq!(paths.len(), 1);
    let filename = paths[0].file_name().and_then(|n| n.to_str()).unwrap_or("");
    let stamp = filename
        .strip_prefix("vnc-relay-logs-")
        .and_then(|n| n.strip_suffix(".jsonl"))
        .unwrap_or_else(|| panic!("download name: {filename}"));
    assert_eq!(stamp.len(), 16);
    assert_eq!(&stamp[8..9], "T");
    assert_eq!(&stamp[15..], "Z");
    assert!(
        stamp
            .bytes()
            .enumerate()
            .all(|(i, b)| i == 8 || i == 15 || b.is_ascii_digit())
    );
    let bytes = must(fs::read(&paths[0]));
    let contents = must(String::from_utf8(bytes));
    assert!(contents.ends_with('\n') && !contents.contains('\r'));
    let downloaded: Vec<Value> = contents
        .lines()
        .map(|line| must(serde_json::from_str(line)))
        .collect();
    let wanted: Vec<_> = expected.iter().map(|item| item["entry"].clone()).collect();
    assert_eq!(
        downloaded, wanted,
        "all entry objects in seq order, no wrappers, no later events"
    );
    assert!(
        downloaded
            .iter()
            .all(|entry| entry.get("seq").is_none() && entry.get("entry").is_none())
    );
    assert_eq!(server.proxy.injected_count("GET", &older_path), 1);
    ready(&mut browser);
    must(fs::remove_file(&paths[0]));

    // The newest page now has three more events; recompute its real cursor.
    for failure in ["error", "leave", "unauthorized"] {
        let current = server.query(&admin, first_path);
        let next = format!("{first_path}&before={}", items(&current)[0]["seq"]);
        let before = server.proxy.injected_count("GET", &next);
        let held = Arc::new(AtomicBool::new(false));
        server
            .proxy
            .once("GET", first_path, Fault::Hold(Arc::clone(&held)));
        if failure == "error" {
            server.proxy.once("GET", &next, Fault::Error500("internal"));
        }
        browser.click("#logs-download");
        wait_for("download first real page held", LIMIT, || {
            server.proxy.traffic().iter().rev().any(|r| {
                r.path == first_path && r.stage == "response_held" && r.completed.is_none()
            })
        });
        assert!(
            files(&dir).is_empty(),
            "{failure} before release: {:?}",
            files(&dir)
        );
        if failure == "leave" {
            browser.click("a.navbar-item[href='/#/settings']");
            browser.wait("document.querySelector('#settings-refresh')?.disabled===false");
        } else if failure == "unauthorized" {
            browser.clear_cookies();
        }
        held.store(true, Ordering::Release);
        match failure {
            "error" => {
                browser.wait("document.body.innerText.includes('부분 파일은 저장하지 않았습니다')");
                assert_eq!(server.proxy.injected_count("GET", &next), before + 1);
            }
            "unauthorized" => {
                browser.wait(
                    "location.pathname==='/login/' && !!document.querySelector('#login-account')",
                );
                assert!(server.proxy.traffic().iter().any(|r| r.path == next
                    && r.forwarded
                    && !r.injected
                    && r.status == Some(401)));
                assert_eq!(
                    browser.eval(
                        "document.querySelectorAll('.log-row,.log-detail,#logs-download').length"
                    ),
                    json!(0)
                );
            }
            _ => {}
        }
        thread::sleep(Duration::from_millis(1200));
        assert!(
            files(&dir).is_empty(),
            "{failure} must never complete a partial file"
        );
        server.proxy.clear();
        if failure == "leave" {
            browser.navigate(&server.url("/#/logs"));
            ready(&mut browser);
            pause(&mut browser);
            select_event(&mut browser, "connection_rejected");
        }
    }
}
