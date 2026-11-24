//! Real disabled/empty buffers, fault handling, recovery and idle expiry.

use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;

use super::logs;
use super::logs::API;
use vnc_relay_testkit::cdp::Browser;
use vnc_relay_testkit::dashboard_proxy::Fault;
use vnc_relay_testkit::live;
use vnc_relay_testkit::live::Server;

pub fn disabled_empty_failures_recovery_and_real_idle_expiry() {
    let disabled = logs::start("dash-061-disabled", 1, 0);
    let mut browser = Browser::start("dash-061-disabled", &["ko-KR"]);
    logs::open(&mut browser, &disabled);
    let raw = logs::last_reply(&disabled).1;
    assert!(logs::items(&raw).is_empty());
    assert!(raw["oldest_seq"].is_null() && raw["latest_seq"].is_null());
    assert_eq!(raw["max_entries"], 0);
    assert!(
        browser.text().contains("recent_logs.max_entries") && browser.text().contains("재시작")
    );
    assert_eq!(browser.eval("[...document.querySelectorAll('#logs-level-info,#logs-event,#logs-q,#logs-search,#logs-download,#logs-auto-refresh')].every(e=>e.disabled)"),json!(true));
    let before = logs::paths(&disabled).len();
    thread::sleep(Duration::from_millis(1300));
    assert_eq!(logs::paths(&disabled).len(), before);
    drop(browser);

    let server = logs::start("dash-061-failures", 60, 1000);
    let mut b = Browser::start("dash-061-failures", &["ko-KR"]);
    logs::open(&mut b, &server);
    logs::pause(&mut b);
    logs::search(&mut b, "no-recent-log-will-contain-this-value", true);
    assert!(logs::items(&logs::last_reply(&server).1).is_empty());
    assert!(b.text().contains("필터와 일치하는 최근 로그가 없습니다"));
    logs::search(&mut b, "", true);
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
            logs::refresh(&mut b);
            assert_eq!(
                b.eval(
                    "document.querySelector('.logs-main > .alert[data-tone=destructive]')===null"
                ),
                json!(true)
            );
            assert!(!logs::items(&logs::last_reply(&server).1).is_empty());
            logs::pause(&mut b);
        }
    }
    drop(b);

    // Si cannot be configured below 300 seconds. Observe the real deadline;
    // neither browser timers nor the server clock are replaced.
    let idle = logs::start("dash-061-idle", 1, 1000);
    let mut actor = Browser::start("dash-061-idle", &["ko-KR"]);
    logs::open(&mut actor, &idle);
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
    live::wait_for(
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
    let count = logs::paths(&idle).len();
    thread::sleep(Duration::from_millis(1300));
    assert_eq!(logs::paths(&idle).len(), count, "401 stops log polling");
    ready_after_fresh_login(&mut actor, &idle);
}

fn ready_after_fresh_login(browser: &mut Browser, server: &Server) {
    logs::open(browser, server);
    logs::ready(browser);
}
