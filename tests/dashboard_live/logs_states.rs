//! Real disabled/empty buffers, fault handling, recovery and idle expiry.

use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;

use super::live::{Browser, Fault, wait_for};
use super::logs::{API, items, last_reply, open, paths, pause, ready, refresh, search, start};

pub fn disabled_empty_failures_recovery_and_real_idle_expiry() {
    let disabled = start("dash-061-disabled", 1, 0);
    let mut browser = Browser::start("dash-061-disabled", &["ko-KR"]);
    open(&mut browser, &disabled);
    let raw = last_reply(&disabled).1;
    assert!(items(&raw).is_empty());
    assert!(raw["oldest_seq"].is_null() && raw["latest_seq"].is_null());
    assert_eq!(raw["max_entries"], 0);
    assert!(
        browser.text().contains("recent_logs.max_entries") && browser.text().contains("재시작")
    );
    assert_eq!(browser.eval("[...document.querySelectorAll('#logs-level-info,#logs-event,#logs-q,#logs-search,#logs-download,#logs-auto-refresh')].every(e=>e.disabled)"),json!(true));
    let before = paths(&disabled).len();
    thread::sleep(Duration::from_millis(1300));
    assert_eq!(paths(&disabled).len(), before);
    drop(browser);

    let server = start("dash-061-failures", 60, 1000);
    let mut b = Browser::start("dash-061-failures", &["ko-KR"]);
    open(&mut b, &server);
    pause(&mut b);
    search(&mut b, "no-recent-log-will-contain-this-value", true);
    assert!(items(&last_reply(&server).1).is_empty());
    assert!(b.text().contains("필터와 일치하는 최근 로그가 없습니다"));
    search(&mut b, "", true);
    for first in [true, false] {
        for (label, fault, fragment) in [
            (
                "400",
                Fault::Response(
                    400,
                    r#"{"error":{"code":"invalid_request","message":"injected"}}"#,
                ),
                "필터를 확인한 뒤",
            ),
            (
                "405",
                Fault::Response(
                    405,
                    r#"{"error":{"code":"method_not_allowed","message":"injected"}}"#,
                ),
                "화면을 다시 불러오십시오",
            ),
            (
                "500",
                Fault::Error500("internal"),
                if first { "다시" } else { "이전" },
            ),
            (
                "timeout",
                Fault::Delay(Duration::from_secs(6)),
                if first { "다시" } else { "이전" },
            ),
        ] {
            let old_seq = super::logs::seqs(&mut b);
            let old_meta = b.eval("document.querySelector('.log-meta').textContent");
            let count = server.proxy.injected_count("GET", API);
            server.proxy.once("GET", API, fault);
            let at = Instant::now();
            if first {
                b.navigate(&server.url("/#/settings"));
                b.wait("!!document.querySelector('#settings-refresh')");
                b.navigate(&server.url("/#/logs"));
            } else {
                b.click("#logs-refresh");
            }
            b.wait("document.querySelector('.logs-main > .alert[data-tone=destructive]')!==null");
            assert!(b.text().contains(fragment), "{label}: {}", b.text());
            assert_eq!(server.proxy.injected_count("GET", API), count + 1);
            if label == "timeout" {
                assert!(at.elapsed() >= Duration::from_secs(5));
            }
            if first {
                assert_eq!(super::logs::seqs(&mut b), json!([]));
                assert!(b.text().contains("확인 불가"));
                assert!(!b.text().contains("필터와 일치하는 최근 로그가 없습니다"));
            } else {
                assert_eq!(super::logs::seqs(&mut b), old_seq);
                assert_eq!(
                    b.eval("document.querySelector('.log-meta').textContent"),
                    old_meta
                );
            }
            server.proxy.clear();
            refresh(&mut b);
            assert_eq!(
                b.eval(
                    "document.querySelector('.logs-main > .alert[data-tone=destructive]')===null"
                ),
                json!(true)
            );
            assert!(!items(&last_reply(&server).1).is_empty());
            pause(&mut b);
        }
    }
    drop(b);

    // Si cannot be configured below 300 seconds. Observe the real deadline;
    // neither browser timers nor the server clock are replaced.
    let idle = start("dash-061-idle", 1, 1000);
    let mut actor = Browser::start("dash-061-idle", &["ko-KR"]);
    open(&mut actor, &idle);
    let baseline = idle.proxy.traffic();
    let activity = baseline
        .iter()
        .filter(|r| {
            r.path.starts_with("/api/v1/")
                && r.status == Some(200)
                && !r.headers.to_ascii_lowercase().contains("x-relay-poll:")
        })
        .map(|r| r.at)
        .max()
        .unwrap_or_else(|| panic!("manual activity"));
    let begin = baseline.len();
    // Drain CDP network events while observing the protected DOM, as DASH-008
    // does. Leaving the CDP socket unread for Si can back-pressure Chromium.
    // A DOM read is not a management request and does not touch the idle timer.
    let mut last_dom = Instant::now();
    let mut near_deadline = false;
    wait_for(
        "logs real 300-second idle expiry",
        Duration::from_secs(325),
        || {
            let now = Instant::now();
            if now.duration_since(last_dom) >= Duration::from_millis(500) {
                let protected = actor.eval(
                    "location.pathname==='/' && document.querySelectorAll('.log-row').length>0",
                );
                let elapsed = now.duration_since(activity);
                if elapsed < Duration::from_secs(299) {
                    assert_eq!(protected, json!(true), "logs must remain visible before Si");
                    if elapsed >= Duration::from_secs(297) {
                        near_deadline = true;
                    }
                }
                last_dom = now;
            }
            idle.proxy
                .traffic()
                .iter()
                .skip(begin)
                .any(|r| r.path.starts_with(API) && r.status == Some(401))
        },
    );
    assert!(
        near_deadline,
        "protected log DOM was observed near its own real deadline"
    );
    actor.wait("location.pathname==='/login/' && !!document.querySelector('#login-account')");
    let traffic = idle.proxy.traffic();
    let polls: Vec<_> = traffic
        .iter()
        .skip(begin)
        .filter(|r| r.path.starts_with(API))
        .collect();
    assert!(polls.iter().all(|r| r.forwarded
        && !r.injected
        && r.headers.to_ascii_lowercase().contains("x-relay-poll: 1")));
    let expired = polls
        .iter()
        .find(|r| r.status == Some(401))
        .unwrap_or_else(|| panic!("real 401"));
    assert!(expired.at.duration_since(activity) >= Duration::from_secs(300));
    assert!(polls.iter().any(
        |r| r.status == Some(200) && r.at.duration_since(activity) >= Duration::from_secs(297)
    ));
    assert_eq!(
        actor.eval(
            "document.querySelectorAll('.log-row,.log-detail,#logs-download,#logs-q').length"
        ),
        json!(0)
    );
    let count = paths(&idle).len();
    thread::sleep(Duration::from_millis(1300));
    assert_eq!(paths(&idle).len(), count, "401 stops log polling");
    ready_after_fresh_login(&mut actor, &idle);
}

fn ready_after_fresh_login(browser: &mut Browser, server: &super::live::Server) {
    open(browser, server);
    ready(browser);
}
