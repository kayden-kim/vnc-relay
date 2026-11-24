//! SCR-007 real-process metrics: menu entry, automatic polls, leaving and returning, and the real idle expiry.
//! All scenario markers live in the root runner, not in these helpers.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;

use super::settings;
use vnc_relay_testkit::cdp::Browser;
use vnc_relay_testkit::dashboard_proxy::Fault;
use vnc_relay_testkit::live::{self, LIMIT, Server};

const API: &str = "/api/v1/metrics";
const MENU: &str = "a.navbar-item[href='/#/metrics']";

/// Path and lower-case request headers of every metrics GET the browser sent, oldest first.
fn requests(server: &Server) -> Vec<(String, String)> {
    server
        .proxy
        .traffic()
        .into_iter()
        .filter(|r| r.method == "GET" && r.path.starts_with(API))
        .map(|r| (r.path, r.headers.to_ascii_lowercase()))
        .collect()
}

fn ready(browser: &mut Browser) {
    browser.wait(
        "document.querySelector('#metrics-refresh')?.disabled === false && document.querySelectorAll('[data-kpi]').length === 8",
    );
}

fn is_poll(headers: &str) -> bool {
    headers.contains("x-relay-poll: 1")
}

// DASH-068
pub fn menu_entry_automatic_polls_leaving_and_returning() {
    let server = Server::start_with_config("dash-068", Some(1), true);
    let mut browser = Browser::start("dash-068", &["ko-KR"]);
    browser.login(&server);
    assert!(
        requests(&server).is_empty(),
        "the overview never reads metrics"
    );

    // Real SCR-005 input is dropped by the menu: no confirmation and no PATCH.
    settings::open_settings(&mut browser, &server);
    settings::open_settings_edit(&mut browser, &server);
    settings::set_field_value(&mut browser, "limits.waiting", "7");
    browser.click(MENU);
    ready(&mut browser);
    assert_eq!(
        browser.eval("document.querySelectorAll('dialog[open]').length"),
        json!(0)
    );
    assert_eq!(server.proxy.count("PATCH", "/api/v1/config"), 0);
    assert_eq!(browser.eval("location.hash"), json!("#/metrics"));
    assert_eq!(
        browser.eval(
            "document.querySelector(\"a.navbar-item[aria-current='page']\")?.getAttribute('href')"
        ),
        json!("/#/metrics")
    );

    let entry = requests(&server);
    assert_eq!(entry[0].0, API, "the entry request has no history_after");
    assert!(
        !entry[0].1.contains("x-relay-poll"),
        "the entry request is a user action: {}",
        entry[0].1
    );
    live::wait_for("real automatic metrics poll", LIMIT, || {
        requests(&server)
            .iter()
            .any(|(path, headers)| path.contains("history_after=") && is_poll(headers))
    });
    assert!(
        requests(&server)
            .iter()
            .skip(1)
            .all(|(path, headers)| is_poll(headers) && path.starts_with(API)),
        "every later request of the screen is an automatic poll"
    );

    // Leaving stops the polls.
    browser.click("a.navbar-item[href='/#/logs']");
    browser.wait("document.querySelector('#logs-refresh') !== null");
    thread::sleep(Duration::from_millis(500));
    let left = requests(&server).len();
    thread::sleep(Duration::from_millis(2500));
    assert_eq!(
        requests(&server).len(),
        left,
        "leaving stops metrics polling"
    );

    // Returning reads the whole trend again with a user request.
    browser.click(MENU);
    ready(&mut browser);
    let back = requests(&server);
    assert_eq!(back[left].0, API, "returning has no history_after");
    assert!(!back[left].1.contains("x-relay-poll"));

    // A response that arrives after leaving does not cover the next screen.
    browser.click("a.navbar-item[href='/#/logs']");
    browser.wait("document.querySelector('#logs-refresh') !== null");
    let held = Arc::new(AtomicBool::new(false));
    server
        .proxy
        .once("GET", API, Fault::Hold(Arc::clone(&held)));
    let before = server.proxy.traffic().len();
    browser.click(MENU);
    live::wait_for("metrics entry held", LIMIT, || {
        server
            .proxy
            .traffic()
            .iter()
            .skip(before)
            .any(|r| r.path == API && r.stage == "response_held")
    });
    browser.click("a.navbar-item[href='/#/logs']");
    browser.wait("document.querySelector('#logs-refresh') !== null");
    held.store(true, Ordering::Release);
    live::wait_for("late metrics response released", LIMIT, || {
        server
            .proxy
            .traffic()
            .iter()
            .skip(before)
            .any(|r| r.path == API && r.completed.is_some())
    });
    thread::sleep(Duration::from_millis(300));
    assert_eq!(
        browser.eval(
            "location.hash === '#/logs' && document.querySelector('#logs-refresh') !== null && document.querySelector('[data-kpi]') === null"
        ),
        json!(true),
        "a late metrics response does not cover the logs screen"
    );
    assert_eq!(server.proxy.injected_count("GET", API), 1);
}

// DASH-076
pub fn unauthenticated_poll_removes_data_and_idle_expiry_is_not_extended() {
    let server = Server::start_with_config("dash-076", Some(1), true);
    let mut actor = Browser::start("dash-076", &["ko-KR"]);
    actor.login(&server);
    actor.navigate(&server.url("/#/metrics"));
    ready(&mut actor);
    let baseline = server.proxy.traffic();
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

    // Si cannot be configured below 300 seconds. Observe the real deadline; neither browser timers nor the server
    // clock are replaced. Drain CDP events while observing the protected DOM, as DASH-008 does. A DOM read is not
    // a management request and does not touch the idle timer.
    let mut last_dom = Instant::now();
    let mut near_deadline = false;
    live::wait_for(
        "metrics real 300-second idle expiry",
        Duration::from_secs(325),
        || {
            let now = Instant::now();
            if now.duration_since(last_dom) >= Duration::from_millis(500) {
                let protected = actor.eval(
                    "location.pathname==='/' && document.querySelectorAll('[data-kpi]').length === 8",
                );
                let elapsed = now.duration_since(activity);
                if elapsed < Duration::from_secs(299) {
                    assert_eq!(
                        protected,
                        json!(true),
                        "metrics must remain visible before Si"
                    );
                    if elapsed >= Duration::from_secs(297) {
                        near_deadline = true;
                    }
                }
                last_dom = now;
            }
            server
                .proxy
                .traffic()
                .iter()
                .skip(begin)
                .any(|r| r.path.starts_with(API) && r.status == Some(401))
        },
    );
    assert!(
        near_deadline,
        "protected metrics DOM was observed near its own real deadline"
    );
    actor.wait("location.pathname==='/login/' && !!document.querySelector('#login-account')");
    assert!(
        actor.text().contains("로그인이 필요합니다"),
        "the login page shows CPY-075"
    );
    let traffic = server.proxy.traffic();
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
        actor.eval("document.querySelectorAll('[data-kpi],[data-chart],.metrics-main').length"),
        json!(0)
    );
    let count = requests(&server).len();
    thread::sleep(Duration::from_millis(1300));
    assert_eq!(requests(&server).len(), count, "401 stops metrics polling");
}
