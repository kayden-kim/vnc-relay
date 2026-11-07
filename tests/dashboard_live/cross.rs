use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::live::{Browser, Fault, LIMIT, PASSWORD, Server, must, wait_for};
use super::overview::assert_list_counts;

const OVERVIEW_PATHS: [&str; 4] = [
    "/api/v1/summary",
    "/api/v1/waiting",
    "/api/v1/sessions",
    "/api/v1/closed-sessions",
];
const DELAY: Duration = Duration::from_secs(3);

fn script_string(value: &str) -> String {
    must(serde_json::to_string(value))
}

fn text_at(browser: &mut Browser, selector: &str) -> String {
    let value = browser.eval(&format!(
        "document.querySelector({})?.textContent ?? ''",
        script_string(selector)
    ));
    match value.as_str() {
        Some(text) => text.to_owned(),
        None => panic!("selector text is not a string: {value}"),
    }
}

fn login(browser: &mut Browser, server: &Server) {
    browser.navigate(&server.url("/login/"));
    browser.fill("#login-account", "alice");
    browser.fill("#login-password", PASSWORD);
    browser.click("#login-submit");
    browser.wait_with_server(
        "document.querySelector('#operations-refresh') !== null",
        server,
    );
}

fn settled(browser: &mut Browser) {
    browser.wait(
        "document.querySelector('.page-meta') && ![...document.querySelectorAll('.card-status')].some(n => /Loading|Updating|조회 중|갱신 중/.test(n.textContent))",
    );
}

fn open_detail(browser: &mut Browser, id: &str) {
    browser.eval(&format!(
        "location.hash = {}",
        script_string(&format!("/sessions/{id}"))
    ));
    browser.wait(&format!(
        "document.querySelector('.fact-list') && document.querySelector('h1')?.textContent.includes({})",
        script_string(id)
    ));
    settled(browser);
}

fn screen_paths(detail: bool, id: &str) -> Vec<String> {
    if detail {
        vec![format!("/api/v1/sessions/{id}")]
    } else {
        OVERVIEW_PATHS
            .iter()
            .map(|path| (*path).to_owned())
            .collect()
    }
}

fn refresh_selector(detail: bool) -> &'static str {
    if detail {
        "#session-refresh"
    } else {
        "#operations-refresh"
    }
}

fn logout_selector(detail: bool) -> &'static str {
    if detail {
        "#session-logout"
    } else {
        "#operations-logout"
    }
}

fn inject_failures(server: &Server, paths: &[String]) {
    for path in paths {
        server.proxy.set("GET", path, Fault::Error500("internal"));
    }
}

fn assert_injected_failures(server: &Server, paths: &[String]) {
    let traffic = server.proxy.traffic();
    for path in paths {
        assert!(
            traffic.iter().any(|item| {
                item.method == "GET"
                    && item.path == *path
                    && item.injected
                    && !item.forwarded
                    && item.status == Some(500)
            }),
            "missing explicitly injected error evidence: {path}"
        );
    }
}

fn assert_login_only(browser: &mut Browser, id: &str) {
    browser.wait(
        "location.pathname === '/login/' && document.querySelector('#login-submit') !== null",
    );
    assert_eq!(browser.eval("document.querySelector('.app-main,.fact-list,#session-terminate,.summary-grid') === null"), json!(true));
    assert!(
        !browser.text().contains(id),
        "protected session remains on login"
    );
}

fn assert_queries_stop(server: &Server) {
    let before: Vec<_> = server
        .proxy
        .traffic()
        .into_iter()
        .filter(|item| item.method == "GET" && is_query(&item.path))
        .map(|item| (item.path, item.at))
        .collect();
    thread::sleep(Duration::from_millis(2300));
    let after: Vec<_> = server
        .proxy
        .traffic()
        .into_iter()
        .filter(|item| item.method == "GET" && is_query(&item.path))
        .map(|item| (item.path, item.at))
        .collect();
    assert_eq!(
        after, before,
        "protected polling continued after leaving the screen"
    );
}

fn is_query(path: &str) -> bool {
    OVERVIEW_PATHS.contains(&path) || path.starts_with("/api/v1/sessions/")
}

fn assert_no_editing(browser: &mut Browser) {
    assert_eq!(browser.eval(
        "document.querySelector('canvas,iframe,video,input:not([type=hidden]),textarea,[contenteditable=true]') === null && ![...document.querySelectorAll('a,button')].some(n => /remote screen|edit config|configuration editor|원격 화면|설정 편집/i.test(n.textContent))"
    ), json!(true));
}

pub fn failed_reads_distinguish_empty_and_stale() {
    for detail in [false, true] {
        let server = Server::start("cross-first-and-stale", Some(1));
        let admin = server.login();
        let mut pair = server.pair(&admin, "II", "63003");
        pair.transfer(2048, 4096);
        let paths = screen_paths(detail, &pair.sid);
        let mut browser = Browser::start("cross-first-and-stale", &["en-US"]);
        if detail {
            login(&mut browser, &server);
            settled(&mut browser);
            inject_failures(&server, &paths);
            browser.eval(&format!(
                "location.hash = {}",
                script_string(&format!("/sessions/{}", pair.sid))
            ));
        } else {
            inject_failures(&server, &paths);
            login(&mut browser, &server);
        }
        browser.wait("document.querySelector('.card-alert .alert') !== null");
        settled(&mut browser);
        let first = browser.text();
        assert!(
            first.contains("Failed"),
            "first failure is not an error: {first}"
        );
        assert!(
            !first.contains("No waiting")
                && !first.contains("No active")
                && !first.contains("No recent")
        );
        assert!(!first.contains("2.0 KiB") && !first.contains("4.0 KiB"));
        assert!(!first.contains("Stale"));
        assert!(text_at(&mut browser, ".page-meta").contains("Unavailable"));
        if !detail {
            assert_eq!(browser.eval("document.querySelector('.summary-grid .stat-value')?.textContent === 'Unavailable'"), json!(true));
            assert_list_counts(&mut browser, [None, None, None], "en");
            assert_eq!(
                browser.eval("document.querySelectorAll('table').length"),
                json!(0)
            );
        } else {
            assert_eq!(
                browser.eval("document.querySelector('.fact-list') === null"),
                json!(true)
            );
        }
        assert_injected_failures(&server, &paths);
        server.proxy.clear();
        browser.click(refresh_selector(detail));
        browser.wait("document.body.textContent.includes('2.0 KiB') && document.body.textContent.includes('4.0 KiB')");
        settled(&mut browser);
        if !detail {
            assert_list_counts(&mut browser, [Some(0), Some(1), Some(0)], "en");
        }
        let success = text_at(&mut browser, ".page-meta");
        inject_failures(&server, &paths);
        browser.click(refresh_selector(detail));
        browser.wait("[...document.querySelectorAll('.card-status')].some(n => n.textContent.includes('Stale'))");
        settled(&mut browser);
        let stale = browser.text();
        assert!(stale.contains("Failed") && stale.contains("Stale"));
        assert!(
            stale.contains(&pair.sid) && stale.contains("2.0 KiB") && stale.contains("4.0 KiB")
        );
        assert!(stale.contains("Last successful"));
        assert_eq!(text_at(&mut browser, ".page-meta"), success);
        assert_injected_failures(&server, &paths);
        if !detail {
            assert_list_counts(&mut browser, [Some(0), Some(1), Some(0)], "en");
        }
    }
}

pub fn explicit_refresh_recovers_only_on_success() {
    for detail in [false, true] {
        let server = Server::start("cross-refresh", Some(1));
        let admin = server.login();
        let mut pair = server.pair(&admin, "II", "63004");
        let mut browser = Browser::start("cross-refresh", &["en-US"]);
        login(&mut browser, &server);
        browser.wait(&format!(
            "document.body.textContent.includes({})",
            script_string(&pair.sid)
        ));
        if detail {
            open_detail(&mut browser, &pair.sid);
        }
        settled(&mut browser);
        let success = text_at(&mut browser, ".page-meta");
        let paths = screen_paths(detail, &pair.sid);
        inject_failures(&server, &paths);
        thread::sleep(Duration::from_millis(1100));
        browser.click(refresh_selector(detail));
        browser.wait("[...document.querySelectorAll('.card-status')].some(n => n.textContent.includes('Stale'))");
        settled(&mut browser);
        assert_eq!(
            text_at(&mut browser, ".page-meta"),
            success,
            "failure changed success timestamp"
        );
        assert_injected_failures(&server, &paths);
        if !detail {
            assert_list_counts(&mut browser, [Some(0), Some(1), Some(0)], "en");
        }
        pair.transfer(8192, 16384);
        server.proxy.clear();
        for path in &paths {
            server.proxy.set("GET", path, Fault::Delay(DELAY));
        }
        let before: Vec<_> = paths
            .iter()
            .map(|path| server.proxy.count("GET", path))
            .collect();
        let refreshed = Instant::now();
        browser.click(refresh_selector(detail));
        browser.wait("[...document.querySelectorAll('.card-status')].some(n => n.textContent.includes('Updating'))");
        assert_eq!(
            text_at(&mut browser, ".page-meta"),
            success,
            "request start changed success timestamp"
        );
        if !detail {
            assert_list_counts(&mut browser, [Some(0), Some(1), Some(0)], "en");
        }
        browser.wait("document.body.textContent.includes('8.0 KiB') && document.body.textContent.includes('16.0 KiB')");
        settled(&mut browser);
        assert_ne!(text_at(&mut browser, ".page-meta"), success);
        assert_eq!(
            browser.eval("document.querySelector('.card-alert .alert') === null"),
            json!(true)
        );
        for (path, count) in paths.iter().zip(before) {
            assert!(
                server.proxy.count("GET", path) > count,
                "explicit Refresh did not request {path}"
            );
            assert!(
                server.proxy.traffic().iter().any(|item| item.path == *path
                    && item.at >= refreshed
                    && item.forwarded
                    && item.status == Some(200)
                    && !item
                        .headers
                        .lines()
                        .any(|line| line.to_ascii_lowercase().starts_with("x-relay-poll:"))),
                "recovery was not an explicit user Refresh: {path}"
            );
        }
    }
}

fn wait_delayed(server: &Server, path: &str, after: Instant) {
    wait_for("real forwarded response held by proxy", LIMIT, || {
        server.proxy.traffic().iter().any(|item| {
            item.path == path
                && item.at >= after
                && item.injected
                && item.forwarded
                && item.completed.is_none()
        })
    });
}

fn wait_delay_finished(server: &Server, path: &str, after: Instant) {
    wait_for("late real response completes", LIMIT, || {
        server.proxy.traffic().iter().any(|item| {
            item.path == path
                && item.at >= after
                && item.injected
                && item.forwarded
                && item.completed.is_some()
        })
    });
}

pub fn delayed_queries_do_not_overlap_or_overwrite_routes() {
    let server = Server::start("cross-delay", Some(1));
    let admin = server.login();
    let mut first = server.pair(&admin, "II", "63005");
    first.transfer(2048, 4096);
    let mut second = server.pair(&admin, "II", "63006");
    second.transfer(8192, 16384);
    let mut browser = Browser::start("cross-delay", &["en-US"]);
    let release_initial = Arc::new(AtomicBool::new(false));
    for path in OVERVIEW_PATHS.into_iter().skip(1) {
        server
            .proxy
            .once("GET", path, Fault::Hold(Arc::clone(&release_initial)));
    }
    login(&mut browser, &server);
    wait_for(
        "all initial list responses held after upstream completion",
        LIMIT,
        || {
            let traffic = server.proxy.traffic();
            OVERVIEW_PATHS.into_iter().skip(1).all(|path| {
                traffic.iter().any(|item| {
                    item.path == path
                        && item.forwarded
                        && item.upstream_response_at.is_some()
                        && item.stage == "response_held"
                        && item.completed.is_none()
                })
            })
        },
    );
    browser.wait(
        "[...document.querySelectorAll('.grid > .card')].slice(3).length === 3 && [...document.querySelectorAll('.grid > .card')].slice(3).every(card => card.querySelector('.card-status')?.textContent.includes('Loading'))",
    );
    assert_list_counts(&mut browser, [None, None, None], "en");
    release_initial.store(true, Ordering::Release);
    settled(&mut browser);
    assert_list_counts(&mut browser, [Some(0), Some(2), Some(0)], "en");
    let began = Instant::now();
    for path in OVERVIEW_PATHS {
        server.proxy.once("GET", path, Fault::Delay(DELAY));
    }
    browser.click("#operations-refresh");
    for path in OVERVIEW_PATHS {
        wait_delayed(&server, path, began);
    }
    assert_list_counts(&mut browser, [Some(0), Some(2), Some(0)], "en");
    let counts: Vec<_> = OVERVIEW_PATHS
        .iter()
        .map(|path| server.proxy.count("GET", path))
        .collect();
    browser.click("#operations-refresh");
    thread::sleep(Duration::from_millis(1300));
    for (path, count) in OVERVIEW_PATHS.iter().zip(counts) {
        assert_eq!(
            server.proxy.count("GET", path),
            count,
            "overlap during >P overview response"
        );
        assert_eq!(server.proxy.max_in_flight("GET", path), 1);
    }
    open_detail(&mut browser, &second.sid);
    for path in OVERVIEW_PATHS {
        wait_delay_finished(&server, path, began);
    }
    assert!(text_at(&mut browser, "h1").contains(&second.sid));
    assert!(browser.text().contains("8.0 KiB") && !browser.text().contains(&first.sid));

    let first_path = format!("/api/v1/sessions/{}", first.sid);
    let began = Instant::now();
    server.proxy.once("GET", &first_path, Fault::Delay(DELAY));
    browser.eval(&format!(
        "location.hash = {}",
        script_string(&format!("/sessions/{}", first.sid))
    ));
    wait_delayed(&server, &first_path, began);
    browser.click("#session-refresh");
    thread::sleep(Duration::from_millis(1300));
    assert_eq!(
        server.proxy.count("GET", &first_path),
        1,
        "detail request overlapped before completion"
    );
    assert_eq!(server.proxy.max_in_flight("GET", &first_path), 1);
    open_detail(&mut browser, &second.sid);
    let current = browser
        .eval("[...document.querySelectorAll('.fact-value,.stat-value')].map(n => n.textContent)");
    wait_delay_finished(&server, &first_path, began);
    assert_eq!(
        browser.eval(
            "[...document.querySelectorAll('.fact-value,.stat-value')].map(n => n.textContent)"
        ),
        current
    );
    assert!(text_at(&mut browser, "h1").contains(&second.sid));
    assert!(!browser.text().contains(&first.sid));

    let second_path = format!("/api/v1/sessions/{}", second.sid);
    let began = Instant::now();
    server.proxy.once("GET", &second_path, Fault::Delay(DELAY));
    browser.click("#session-refresh");
    wait_delayed(&server, &second_path, began);
    browser.eval("location.hash = ''");
    browser.wait("document.querySelector('#operations-refresh') !== null && document.querySelector('table') !== null");
    wait_delay_finished(&server, &second_path, began);
    assert!(text_at(&mut browser, "h1").contains("Operations"));
    assert_eq!(
        browser.eval("document.querySelector('.fact-list,#session-refresh') === null"),
        json!(true)
    );
    assert_eq!(server.proxy.max_in_flight("GET", &second_path), 1);
}

pub fn automatic_polls_expire_and_unauthorized_reads_clear_screens() {
    let server = Server::start("cross-expiry", Some(1));
    let admin = server.login();
    let pair = server.pair(&admin, "II", "63008");
    let detail_path = format!("/api/v1/sessions/{}", pair.sid);
    let mut overview = Browser::start("cross-expiry-overview", &["en-US"]);
    let mut detail = Browser::start("cross-expiry-detail", &["en-US"]);
    login(&mut overview, &server);
    overview.wait(&format!(
        "document.body.textContent.includes({})",
        script_string(&pair.sid)
    ));
    // Capture this actor before the second profile briefly opens overview
    // during login. A later /sessions request can belong to either profile.
    let overview_cookie = server
        .proxy
        .traffic()
        .iter()
        .rev()
        .find(|item| {
            item.method == "GET" && item.path == "/api/v1/sessions" && item.status == Some(200)
        })
        .and_then(|item| {
            item.headers.lines().find_map(|line| {
                line.split_once(':')
                    .filter(|(name, _)| name.eq_ignore_ascii_case("cookie"))
                    .map(|(_, value)| value.trim().to_owned())
            })
        })
        .unwrap_or_else(|| panic!("overview actor's authenticated cookie missing"));
    login(&mut detail, &server);
    open_detail(&mut detail, &pair.sid);
    settled(&mut overview);
    let idle_traffic = server.proxy.traffic();
    // Only the detail profile has requested this session's detail path.
    let detail_cookie = idle_traffic
        .iter()
        .rev()
        .find(|item| item.method == "GET" && item.path == detail_path && item.status == Some(200))
        .and_then(|item| {
            item.headers.lines().find_map(|line| {
                line.split_once(':')
                    .filter(|(name, _)| name.eq_ignore_ascii_case("cookie"))
                    .map(|(_, value)| value.trim().to_owned())
            })
        })
        .unwrap_or_else(|| panic!("detail actor's authenticated cookie missing"));
    assert!(
        overview_cookie != detail_cookie,
        "expiry actors must have independent sessions"
    );
    let has_cookie = |headers: &str, cookie: &str| {
        headers.lines().any(|line| {
            line.split_once(':').is_some_and(|(name, value)| {
                name.eq_ignore_ascii_case("cookie") && value.trim() == cookie
            })
        })
    };
    let last_activity = |cookie: &str| {
        idle_traffic
            .iter()
            .filter(|item| {
                item.method == "GET"
                    && item.path.starts_with("/api/v1/")
                    && item.status == Some(200)
                    && has_cookie(&item.headers, cookie)
                    && !item
                        .headers
                        .lines()
                        .any(|line| line.trim().eq_ignore_ascii_case("x-relay-poll: 1"))
            })
            .map(|item| item.at)
            .max()
            .unwrap_or_else(|| panic!("initial user activity missing"))
    };
    // A server touch occurs no earlier than the recorded request arrival.
    // Each first 401 must therefore be at least 300 real seconds after this
    // actor's final manual request, not merely present after a long test sleep.
    let overview_activity = last_activity(&overview_cookie);
    let detail_activity = last_activity(&detail_cookie);
    let observation_end = overview_activity.max(detail_activity) + Duration::from_secs(325);
    let idle_index = idle_traffic.len();
    let mut near_deadline = [false; 2];
    let mut last_dom_check = Instant::now() - Duration::from_secs(1);
    loop {
        let now = Instant::now();
        let traffic = server.proxy.traffic();
        let mut expired = [false; 2];
        for (index, (cookie, activity)) in [
            (&overview_cookie, overview_activity),
            (&detail_cookie, detail_activity),
        ]
        .into_iter()
        .enumerate()
        {
            let actor: Vec<_> = traffic
                .iter()
                .skip(idle_index)
                .filter(|item| {
                    item.method == "GET"
                        && is_query(&item.path)
                        && has_cookie(&item.headers, cookie)
                })
                .collect();
            for item in &actor {
                assert!(
                    item.headers
                        .lines()
                        .any(|line| line.trim().eq_ignore_ascii_case("x-relay-poll: 1")),
                    "user action extended the real idle observation"
                );
                assert!(
                    !item.injected && (item.status.is_none() || item.forwarded),
                    "expiry must use real server responses"
                );
            }
            if let Some(first) = actor.iter().find(|item| item.status == Some(401)) {
                assert!(
                    first.at.duration_since(activity) >= Duration::from_secs(300),
                    "actor {index} received 401 before its real Si deadline"
                );
                assert!(
                    actor.iter().any(|item| item.status == Some(200)
                        && item.at.duration_since(activity) >= Duration::from_secs(297)),
                    "actor {index} did not keep successful automatic polls near expiry"
                );
                expired[index] = true;
            }
        }
        if now.duration_since(last_dom_check) >= Duration::from_millis(500) {
            for (index, (browser, activity)) in [
                (&mut overview, overview_activity),
                (&mut detail, detail_activity),
            ]
            .into_iter()
            .enumerate()
            {
                let elapsed = now.duration_since(activity);
                if elapsed < Duration::from_secs(299) {
                    assert_eq!(
                        browser.eval(
                            "location.pathname === '/' && !!document.querySelector('.app-main')"
                        ),
                        json!(true),
                        "actor {index} lost protected screen before expiry"
                    );
                    assert!(
                        browser.text().contains(&pair.sid),
                        "actor {index} removed protected session data before expiry"
                    );
                    if elapsed >= Duration::from_secs(297) {
                        near_deadline[index] = true;
                    }
                }
            }
            last_dom_check = now;
        }
        if expired.into_iter().all(|expired| expired) {
            break;
        }
        assert!(
            now < observation_end,
            "both real Si expiries were not observed"
        );
        thread::sleep(Duration::from_millis(20));
    }
    assert!(
        near_deadline.into_iter().all(|observed| observed),
        "both protected screens must be observed near their own 300s deadline"
    );
    assert_login_only(&mut overview, &pair.sid);
    assert_login_only(&mut detail, &pair.sid);
    assert_queries_stop(&server);

    for (browser, is_detail) in [(&mut overview, false), (&mut detail, true)] {
        login(browser, &server);
        if is_detail {
            open_detail(browser, &pair.sid);
        } else {
            settled(browser);
        }
        browser.clear_cookies();
        let after = Instant::now();
        browser.click(refresh_selector(is_detail));
        assert_login_only(browser, &pair.sid);
        assert!(server.proxy.traffic().iter().any(|item| item.at >= after
            && is_query(&item.path)
            && item.forwarded
            && !item.injected
            && item.status == Some(401)));
    }
    assert_queries_stop(&server);
}

pub fn relay_changes_appear_on_next_poll() {
    for detail in [false, true] {
        let server = Server::start("cross-state", Some(1));
        let admin = server.login();
        let mut pair = server.pair(&admin, "II", "63016");
        let mut browser = Browser::start("cross-state", &["en-US"]);
        login(&mut browser, &server);
        browser.wait(&format!(
            "document.body.textContent.includes({})",
            script_string(&pair.sid)
        ));
        if detail {
            open_detail(&mut browser, &pair.sid);
        }
        settled(&mut browser);
        let success = text_at(&mut browser, ".page-meta");
        assert_no_editing(&mut browser);
        let changed = Instant::now();
        pair.transfer(8192, 16384);
        browser.wait("document.body.textContent.includes('8.0 KiB') && document.body.textContent.includes('16.0 KiB')");
        let visible = Instant::now();
        settled(&mut browser);
        assert_ne!(text_at(&mut browser, ".page-meta"), success);
        let path = if detail {
            format!("/api/v1/sessions/{}", pair.sid)
        } else {
            "/api/v1/sessions".to_owned()
        };
        let traffic = server.proxy.traffic();
        let next = match traffic
            .iter()
            .find(|item| item.path == path && item.at >= changed)
        {
            Some(item) => item,
            None => panic!("changed state has no following normal poll"),
        };
        assert!(next.forwarded && !next.injected && next.status == Some(200));
        assert!(
            next.headers
                .lines()
                .any(|line| line.trim().eq_ignore_ascii_case("x-relay-poll: 1"))
        );
        let finished = match next.completed {
            Some(at) => at,
            None => panic!("state poll not completed"),
        };
        assert!(
            visible.duration_since(changed)
                <= Duration::from_secs(1)
                    + finished.duration_since(next.at)
                    + Duration::from_millis(300)
        );
        assert_no_editing(&mut browser);
        let id = pair.sid.clone();
        pair.close();
        if detail {
            browser.wait("document.body.textContent.includes('The current session was not found')");
            assert_eq!(
                browser.eval("document.querySelector('#session-terminate')?.disabled"),
                json!(true)
            );
        } else {
            browser.wait(&format!(
                "![...document.querySelectorAll('a[href]')].some(n => n.getAttribute('href') === {}) && document.body.textContent.includes({})",
                script_string(&format!("/#/sessions/{id}")), script_string(&id)
            ));
        }
        if !detail {
            settled(&mut browser);
            assert_list_counts(&mut browser, [Some(0), Some(0), Some(1)], "en");
        }
        assert_no_editing(&mut browser);
    }
}

pub fn logout_clears_both_screens_and_preserves_relay() {
    for detail in [false, true] {
        for lost in [false, true] {
            let server = Server::start("cross-logout", Some(1));
            let admin = server.login();
            let mut pair = server.pair(&admin, "II", "63019");
            pair.transfer(2048, 4096);
            let mut browser = Browser::start("cross-logout", &["en-US"]);
            login(&mut browser, &server);
            browser.wait(&format!(
                "document.body.textContent.includes({})",
                script_string(&pair.sid)
            ));
            if detail {
                open_detail(&mut browser, &pair.sid);
            }
            settled(&mut browser);
            if lost {
                server
                    .proxy
                    .once("DELETE", "/api/v1/session", Fault::DropResponse);
            }
            browser.click(logout_selector(detail));
            assert_login_only(&mut browser, &pair.sid);
            assert_eq!(
                browser.eval("document.querySelector('[data-notice=logout-unconfirmed]') !== null"),
                json!(lost)
            );
            let traffic = server.proxy.traffic();
            assert!(
                traffic.iter().any(|item| item.method == "DELETE"
                    && item.path == "/api/v1/session"
                    && item.forwarded
                    && item.injected == lost
                    && item.status == Some(204)),
                "logout did not really invalidate the session"
            );
            assert_queries_stop(&server);
            browser.eval("history.back()");
            thread::sleep(Duration::from_millis(300));
            assert_login_only(&mut browser, &pair.sid);
            for path in ["/".to_owned(), format!("/#/sessions/{}", pair.sid)] {
                browser.navigate(&server.url(&path));
                assert_login_only(&mut browser, &pair.sid);
            }
            assert_queries_stop(&server);
            pair.transfer(8192, 16384);
            let surviving = server.query(&admin, &format!("/api/v1/sessions/{}", pair.sid));
            assert_eq!(surviving["session"]["session_id"], json!(pair.sid));
            assert_eq!(surviving["session"]["bytes_viewer_to_target"], json!(10240));
            assert_eq!(surviving["session"]["bytes_target_to_viewer"], json!(20480));
        }
    }
}

pub fn configured_and_default_poll_intervals() {
    for (poll, expected) in [(Some(1), 1_u64), (None, 2_u64)] {
        let server = Server::start("cross-poll", poll);
        let admin = server.login();
        assert_eq!(
            server.query(&admin, "/api/v1/session")["dashboard"]["poll_secs"],
            json!(expected)
        );
        let pair = server.pair(&admin, "II", "63022");
        let mut browser = Browser::start("cross-poll", &["en-US"]);
        login(&mut browser, &server);
        browser.wait(&format!(
            "document.body.textContent.includes({})",
            script_string(&pair.sid)
        ));
        for detail in [false, true] {
            if detail {
                open_detail(&mut browser, &pair.sid);
            }
            settled(&mut browser);
            let paths = screen_paths(detail, &pair.sid);
            for path in &paths {
                wait_for("three genuine automatic polls", LIMIT, || {
                    server
                        .proxy
                        .traffic()
                        .iter()
                        .filter(|item| {
                            item.path == *path
                                && item
                                    .headers
                                    .lines()
                                    .any(|line| line.trim().eq_ignore_ascii_case("x-relay-poll: 1"))
                                && item.status == Some(200)
                        })
                        .count()
                        >= 3
                });
                let traffic = server.proxy.traffic();
                let polls: Vec<_> = traffic
                    .iter()
                    .filter(|item| {
                        item.path == *path
                            && item
                                .headers
                                .lines()
                                .any(|line| line.trim().eq_ignore_ascii_case("x-relay-poll: 1"))
                    })
                    .take(3)
                    .collect();
                assert_eq!(polls.len(), 3);
                for window in polls.windows(2) {
                    let completed = match window[0].completed {
                        Some(at) => at,
                        None => panic!("previous poll is still outstanding"),
                    };
                    let pause = window[1].at.duration_since(completed);
                    assert!(
                        pause >= Duration::from_millis(expected * 1000 - 100),
                        "poll fired before server P: {pause:?}"
                    );
                    assert!(
                        pause <= Duration::from_millis(expected * 1000 + 1000),
                        "poll did not follow configured P: {pause:?}"
                    );
                    assert!(
                        window[1].at.duration_since(window[0].at)
                            >= Duration::from_millis(expected * 1000 - 100)
                    );
                }
                assert!(polls.iter().all(|item| item.forwarded && !item.injected));
                assert_eq!(server.proxy.max_in_flight("GET", path), 1);
            }
        }
    }
}

fn assert_language(browser: &mut Browser, language: &str, title: &str) {
    assert_eq!(
        browser.eval("document.documentElement.lang"),
        json!(language)
    );
    assert!(text_at(browser, "h1").contains(title));
    assert_eq!(
        browser.eval("document.querySelector('select')?.value"),
        json!(language)
    );
}

fn overview_data(browser: &mut Browser, id: &str) -> Value {
    browser.eval(&format!(
        "(() => {{ const row = [...document.querySelectorAll('tr')].find(n => n.querySelector('a')?.textContent === {}); if (!row) throw new Error('missing real active row'); return {{id:row.cells[0].textContent,mode:row.cells[1].textContent,transport:row.cells[2].textContent,time:row.cells[3].textContent,bytes:[...row.querySelectorAll('.cell-bytes .cell-value')].map(n=>n.textContent),addresses:[...row.querySelectorAll('.fact-address')].map(n=>n.textContent)}}; }})()",
        script_string(id)
    ))
}

fn detail_data(browser: &mut Browser) -> Value {
    browser.eval("({facts:[...document.querySelectorAll('.fact-value')].map(n=>n.textContent),bytes:[...document.querySelectorAll('.stat-value')].map(n=>n.textContent)})")
}

fn assert_formats(data: &Value) {
    assert_eq!(data["bytes"], json!(["2.0 KiB", "4.0 KiB"]));
    let stamp = match data["time"].as_str() {
        Some(value) => value,
        None => panic!("missing start time: {data}"),
    };
    assert!(
        stamp.len() > 20
            && stamp.as_bytes().get(4) == Some(&b'-')
            && stamp.as_bytes().get(10) == Some(&b' ')
    );
}

pub fn browser_preferences_select_language_on_all_screens() {
    let server = Server::start("cross-preferences", Some(1));
    let admin = server.login();
    let mut pair = server.pair(&admin, "II", "63024");
    pair.transfer(2048, 4096);
    let mut baseline_overview = None;
    let mut baseline_detail = None;
    for (languages, chosen) in [
        (vec!["ko-KR"], "ko"),
        (vec!["en-US"], "en"),
        (vec!["ja-JP"], "en"),
        (vec!["ja-JP", "ko"], "ko"),
    ] {
        let mut browser = Browser::start("cross-preferences", &languages);
        browser.navigate(&server.url("/login/"));
        browser.wait("document.querySelector('#login-submit') !== null");
        assert_eq!(browser.eval("navigator.languages"), json!(languages));
        assert_eq!(
            browser.eval("localStorage.getItem('vnc-relay.language')"),
            Value::Null
        );
        assert_language(
            &mut browser,
            chosen,
            if chosen == "ko" {
                "관리자 로그인"
            } else {
                "administrator sign in"
            },
        );
        browser.fill("#login-account", "alice");
        browser.fill("#login-password", PASSWORD);
        browser.click("#login-submit");
        browser.wait_with_server(
            &format!(
                "document.querySelector('table') && document.body.textContent.includes({})",
                script_string(&pair.sid)
            ),
            &server,
        );
        assert_language(
            &mut browser,
            chosen,
            if chosen == "ko" {
                "운영 현황"
            } else {
                "Operations"
            },
        );
        let overview = overview_data(&mut browser, &pair.sid);
        assert_formats(&overview);
        match &baseline_overview {
            Some(previous) => assert_eq!(&overview, previous),
            None => baseline_overview = Some(overview),
        }
        open_detail(&mut browser, &pair.sid);
        assert_language(
            &mut browser,
            chosen,
            if chosen == "ko" { "세션" } else { "Session" },
        );
        let detail = detail_data(&mut browser);
        assert_eq!(detail["bytes"], json!(["2.0 KiB", "4.0 KiB"]));
        match &baseline_detail {
            Some(previous) => assert_eq!(&detail, previous),
            None => baseline_detail = Some(detail),
        }
        assert_eq!(
            browser.eval("localStorage.getItem('vnc-relay.language')"),
            Value::Null
        );
        browser.click("#session-logout");
        assert_login_only(&mut browser, &pair.sid);
    }
}

fn select_language(browser: &mut Browser, language: &str) {
    browser.click("select");
    browser.eval(&format!(
        "(() => {{ const select = document.querySelector('select'); select.value = {}; select.dispatchEvent(new Event('change', {{bubbles:true}})); select.blur(); }})()",
        script_string(language)
    ));
}

pub fn saved_language_survives_all_screens() {
    for (preferred, chosen) in [("ko-KR", "en"), ("en-US", "ko")] {
        let server = Server::start("cross-saved-language", Some(1));
        let admin = server.login();
        let mut pair = server.pair(&admin, "II", "63025");
        pair.transfer(2048, 4096);
        let mut browser = Browser::start("cross-saved-language", &[preferred]);
        browser.navigate(&server.url("/login/"));
        browser.fill("#login-account", "alice");
        select_language(&mut browser, chosen);
        assert_eq!(
            browser.eval("document.querySelector('#login-account')?.value"),
            json!("alice")
        );
        assert_language(
            &mut browser,
            chosen,
            if chosen == "ko" {
                "관리자 로그인"
            } else {
                "administrator sign in"
            },
        );
        assert_eq!(
            browser.eval("localStorage.getItem('vnc-relay.language')"),
            json!(chosen)
        );
        browser.fill("#login-password", PASSWORD);
        browser.click("#login-submit");
        browser.wait(&format!(
            "document.querySelector('table') && document.body.textContent.includes({})",
            script_string(&pair.sid)
        ));
        let overview = overview_data(&mut browser, &pair.sid);
        settled(&mut browser);
        assert_list_counts(&mut browser, [Some(0), Some(1), Some(0)], chosen);
        assert_formats(&overview);
        assert_language(
            &mut browser,
            chosen,
            if chosen == "ko" {
                "운영 현황"
            } else {
                "Operations"
            },
        );
        let opposite = if chosen == "ko" { "en" } else { "ko" };
        select_language(&mut browser, opposite);
        assert_language(
            &mut browser,
            opposite,
            if opposite == "ko" {
                "운영 현황"
            } else {
                "Operations"
            },
        );
        assert_eq!(overview_data(&mut browser, &pair.sid), overview);
        assert_list_counts(&mut browser, [Some(0), Some(1), Some(0)], opposite);
        select_language(&mut browser, chosen);
        assert_eq!(overview_data(&mut browser, &pair.sid), overview);
        assert_list_counts(&mut browser, [Some(0), Some(1), Some(0)], chosen);
        browser.reload();
        browser.wait(&format!(
            "document.querySelector('table') && document.body.textContent.includes({})",
            script_string(&pair.sid)
        ));
        assert_language(
            &mut browser,
            chosen,
            if chosen == "ko" {
                "운영 현황"
            } else {
                "Operations"
            },
        );
        assert_eq!(overview_data(&mut browser, &pair.sid), overview);
        settled(&mut browser);
        assert_list_counts(&mut browser, [Some(0), Some(1), Some(0)], chosen);
        open_detail(&mut browser, &pair.sid);
        let detail = detail_data(&mut browser);
        assert_language(
            &mut browser,
            chosen,
            if chosen == "ko" { "세션" } else { "Session" },
        );
        assert_unknown_consumer_values(
            &mut browser,
            &server.query(&admin, &format!("/api/v1/sessions/{}", pair.sid)),
        );
        select_language(&mut browser, opposite);
        assert_language(
            &mut browser,
            opposite,
            if opposite == "ko" {
                "세션"
            } else {
                "Session"
            },
        );
        assert_eq!(
            detail_data(&mut browser),
            detail,
            "language translated real session values"
        );
        select_language(&mut browser, chosen);
        assert_eq!(detail_data(&mut browser), detail);
        browser.reload();
        // The first-load view also draws `.fact-list`, with loading badges (Q143).
        browser.wait("document.querySelector('.fact-list') !== null");
        settled(&mut browser);
        assert_language(
            &mut browser,
            chosen,
            if chosen == "ko" { "세션" } else { "Session" },
        );
        assert_eq!(detail_data(&mut browser), detail);
        browser.click("#session-logout");
        assert_login_only(&mut browser, &pair.sid);
        assert_language(
            &mut browser,
            chosen,
            if chosen == "ko" {
                "관리자 로그인"
            } else {
                "administrator sign in"
            },
        );
        browser.reload();
        browser.wait("document.querySelector('#login-submit') !== null");
        assert_language(
            &mut browser,
            chosen,
            if chosen == "ko" {
                "관리자 로그인"
            } else {
                "administrator sign in"
            },
        );
        for item in server
            .proxy
            .traffic()
            .iter()
            .filter(|item| item.path.starts_with("/api/v1/"))
        {
            assert!(
                !item.path.contains('?'),
                "language query parameter sent: {}",
                item.path
            );
            for header in item.headers.lines().filter_map(|line| {
                line.split_once(':')
                    .map(|(name, _)| name.trim().to_ascii_lowercase())
            }) {
                assert!(!matches!(
                    header.as_str(),
                    "x-language" | "x-locale" | "content-language" | "language" | "locale"
                ));
            }
            if item.method == "POST" && item.path == "/api/v1/session" {
                let body: Value = must(serde_json::from_str(&item.body));
                let object = match body.as_object() {
                    Some(value) => value,
                    None => panic!("login body is not an object"),
                };
                assert_eq!(object.len(), 2);
                assert!(object.contains_key("account") && object.contains_key("password"));
            } else {
                assert!(item.body.is_empty(), "language data sent in an API body");
            }
        }
    }
}

fn assert_unknown_consumer_values(browser: &mut Browser, real_detail: &Value) {
    // The typed producer cannot emit future enum variants. This separate consumer
    // contract input is rendered by the real production parser and table component.
    // It is never an API response, proxy payload, or replacement fetch implementation.
    let source = must(serde_json::to_string(real_detail));
    let result = browser.eval(&format!(
        r#"(async () => {{
          const [model, panels, copy] = await Promise.all([
            import('/app/model.js'), import('/app/panels.js'), import('/app/copy.js')
          ]);
          const source = {source};
          const contractInput = {{
            as_of: source.as_of,
            items: [{{...source.session, mode:'future-mode', viewer_transport:'future-path'}}]
          }};
          const parsed = model.parseSessions(contractInput);
          const results = [];
          for (const language of ['ko','en']) {{
            const t = copy.COPY[language];
            const table = panels.activeTable(t, parsed.items, t.activeEmpty);
            document.body.append(table);
            try {{
              const row = table.querySelector('tbody tr');
              results.push({{
                language,
                mode: row.cells[1].textContent,
                transport: row.cells[2].textContent,
                id: row.cells[0].textContent,
                translatedAction: row.cells[6].textContent.trim() === t.details,
                translatedHeading: [...table.querySelectorAll('th')].some(n => n.textContent === t.activeColumns[1]),
                visible: table.getBoundingClientRect().height > 0
              }});
            }} finally {{ table.remove(); }}
          }}
          return results;
        }})()"#
    ));
    for index in 0..2 {
        assert_eq!(result[index]["mode"], json!("future-mode"));
        assert_eq!(result[index]["transport"], json!("future-path"));
        assert_eq!(result[index]["id"], real_detail["session"]["session_id"]);
        assert_eq!(result[index]["translatedAction"], json!(true));
        assert_eq!(result[index]["translatedHeading"], json!(true));
        assert_eq!(result[index]["visible"], json!(true));
    }
}
