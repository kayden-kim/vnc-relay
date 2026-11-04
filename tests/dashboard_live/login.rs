use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::live::{Browser, Fault, LIMIT, PASSWORD, Server, wait_for};

const SESSION_PATH: &str = "/api/v1/session";
const QUERY_PATHS: [&str; 4] = [
    "/api/v1/summary",
    "/api/v1/waiting",
    "/api/v1/sessions",
    "/api/v1/closed-sessions",
];
const FAILED: &str = "로그인할 수 없습니다. 계정명과 비밀번호를 확인한 뒤 다시 시도하십시오.";
const REQUEST_FAILED: &str = "로그인 요청을 처리하지 못했습니다. 잠시 뒤 다시 시도하십시오.";
const WRONG_PASSWORD: &str = "wrong-live-login-password";

pub fn correct_credentials_open_overview_and_queries() {
    let server = Server::start("login-success", None);
    let _waiting = server.waiting("7417001");

    for width in [1280, 390] {
        let mut browser = Browser::start(&format!("login-success-{width}"), &["ko-KR"]);
        browser.set_width(width);
        open_login(&mut browser, &server);
        let before = server.proxy.count("POST", SESSION_PATH);
        let traffic_before = server.proxy.traffic().len();
        browser.fill("#login-account", "alice");
        browser.fill("#login-password", PASSWORD);
        browser.click("#login-submit");
        wait_for_overview(&mut browser);
        browser.wait("document.body.innerText.includes('7417001')");

        assert_eq!(server.proxy.count("POST", SESSION_PATH), before + 1);
        assert_eq!(server.proxy.injected_count("POST", SESSION_PATH), 0);
        let traffic = server.proxy.traffic();
        assert!(traffic.iter().skip(traffic_before).any(|request| {
            request.method == "POST"
                && request.path == SESSION_PATH
                && request.forwarded
                && !request.injected
                && request.status == Some(200)
                && request.completed.is_some()
        }));
        for path in QUERY_PATHS.into_iter().chain([SESSION_PATH]) {
            assert!(traffic.iter().skip(traffic_before).any(|request| {
                request.method == "GET"
                    && request.path == path
                    && request.forwarded
                    && !request.injected
                    && request.status == Some(200)
                    && request.completed.is_some()
            }));
        }
        assert_eq!(
            browser.eval(
                "location.pathname === '/' && !document.querySelector('#login-root') \
                 && !!document.querySelector('#operations-refresh') \
                 && document.documentElement.scrollWidth <= innerWidth",
            ),
            json!(true),
        );
    }
}

pub fn empty_wrong_and_locked_login_hide_protected_data() {
    for width in [1280, 390] {
        // A fresh process keeps the real account lockout independent of the other widths and rows.
        let server = Server::start(&format!("login-failures-{width}"), None);
        let _waiting = server.waiting("7418001");
        let mut browser = Browser::start(&format!("login-failures-{width}"), &["ko-KR"]);
        browser.set_width(width);
        open_login(&mut browser, &server);

        browser.click("#login-submit");
        browser.wait(
            "document.querySelector('#login-account').getAttribute('aria-invalid') === 'true' \
             && document.querySelector('#login-password').getAttribute('aria-invalid') === 'true'",
        );
        assert!(browser.text().contains("계정명을 입력하십시오."));
        assert!(browser.text().contains("비밀번호를 입력하십시오."));
        assert_eq!(server.proxy.count("POST", SESSION_PATH), 0);

        browser.fill("#login-password", WRONG_PASSWORD);
        browser.click("#login-submit");
        browser.wait(
            "document.querySelector('#login-account').getAttribute('aria-invalid') === 'true'",
        );
        assert_eq!(server.proxy.count("POST", SESSION_PATH), 0);

        browser.fill("#login-account", "alice");
        browser.fill("#login-password", "");
        browser.click("#login-submit");
        browser.wait(
            "document.querySelector('#login-password').getAttribute('aria-invalid') === 'true'",
        );
        assert_eq!(server.proxy.count("POST", SESSION_PATH), 0);
        assert_private_login(&mut browser, &server, "7418001");

        // An unknown account gets exactly the same copy as a known account with a wrong password.
        fail_credentials(&mut browser, &server, "nobody", WRONG_PASSWORD, 1);
        let unknown_notice = notice(&mut browser);
        assert_private_login(&mut browser, &server, "7418001");

        fail_credentials(&mut browser, &server, "alice", WRONG_PASSWORD, 2);
        let wrong_notice = notice(&mut browser);
        assert_eq!(unknown_notice, wrong_notice);
        assert_eq!(wrong_notice, json!(FAILED));
        assert_private_login(&mut browser, &server, "7418001");

        // F is an allowed count: the third alice failure exceeds per_account=2.
        fail_credentials(&mut browser, &server, "alice", WRONG_PASSWORD, 3);
        assert_eq!(notice(&mut browser), wrong_notice);
        assert_private_login(&mut browser, &server, "7418001");

        fail_credentials(&mut browser, &server, "alice", WRONG_PASSWORD, 4);
        assert_eq!(notice(&mut browser), wrong_notice);
        assert_private_login(&mut browser, &server, "7418001");

        // A correct password is still rejected by the real limiter, not by an injected response.
        fail_credentials(&mut browser, &server, "alice", PASSWORD, 5);
        assert_eq!(notice(&mut browser), wrong_notice);
        assert_private_login(&mut browser, &server, "7418001");
        assert_eq!(server.proxy.injected_count("POST", SESSION_PATH), 0);

        // Directly revisiting the protected document cannot reveal data after the locked login.
        browser.navigate(&server.url("/"));
        browser
            .wait("location.pathname === '/login/' && !!document.querySelector('#login-submit')");
        assert_private_login(&mut browser, &server, "7418001");
        assert!(server.proxy.traffic().iter().any(|request| {
            request.method == "GET"
                && request.path == "/"
                && request.forwarded
                && !request.injected
                && request.status == Some(303)
        }));
    }
}

pub fn login_transport_failures_are_distinct_and_block_duplicates() {
    for width in [1280, 390] {
        let server = Server::start(&format!("login-transport-{width}"), None);
        let _waiting = server.waiting("7427001");
        let mut browser = Browser::start(&format!("login-transport-{width}"), &["ko-KR"]);
        browser.set_width(width);
        open_login(&mut browser, &server);
        fail_credentials(&mut browser, &server, "alice", WRONG_PASSWORD, 1);
        let wrong_notice = notice(&mut browser);

        for (branch, fault, forwarded, status) in [
            (
                "contract-shaped-500",
                Fault::Error500("internal"),
                false,
                Some(500),
            ),
            ("connection-closed", Fault::Close, false, None),
            ("response-discarded", Fault::DropResponse, true, Some(200)),
            (
                "response-timeout",
                Fault::Delay(Duration::from_secs(11)),
                true,
                Some(200),
            ),
        ] {
            let before = server.proxy.count("POST", SESSION_PATH);
            let injected_before = server.proxy.injected_count("POST", SESSION_PATH);
            server.proxy.once("POST", SESSION_PATH, fault);
            browser.fill("#login-account", "alice");
            browser.fill("#login-password", PASSWORD);
            let started = Instant::now();

            // Submit events use the shipped form. One JS task also attempts both disabled-button
            // clicks and requestSubmit(), before any network response can finish the first attempt.
            assert_eq!(
                browser.eval(
                    "(() => { \
                       const form = document.querySelector('form'); \
                       const button = document.querySelector('#login-submit'); \
                       form.requestSubmit(); \
                       for (let i = 0; i < 8; i += 1) { button.click(); form.requestSubmit(); } \
                       return button.disabled && button.getAttribute('aria-busy') === 'true' \
                         && document.querySelector('[data-notice=checking] .alert-title')?.textContent \
                           === '로그인을 확인하는 중입니다.'; \
                     })()",
                ),
                json!(true),
                "{branch}: duplicate attempts must remain disabled while in flight",
            );
            wait_for("one login request reaches the proxy", LIMIT, || {
                server.proxy.count("POST", SESSION_PATH) == before + 1
            });
            browser.wait(
                "!!document.querySelector('[data-notice=request-failed]') \
                 && !document.querySelector('#login-submit').disabled",
            );
            if branch == "response-timeout" {
                assert!(started.elapsed() >= Duration::from_secs(9));
            }
            assert_eq!(notice(&mut browser), json!(REQUEST_FAILED), "{branch}");
            assert_ne!(notice(&mut browser), wrong_notice, "{branch}");
            assert_private_login(&mut browser, &server, "7427001");
            wait_for("injected login exchange finishes", LIMIT, || {
                server
                    .proxy
                    .traffic()
                    .iter()
                    .filter(|request| request.method == "POST" && request.path == SESSION_PATH)
                    .nth(before)
                    .is_some_and(|request| request.completed.is_some())
            });
            let traffic = server.proxy.traffic();
            let requests: Vec<_> = traffic
                .iter()
                .filter(|request| request.method == "POST" && request.path == SESSION_PATH)
                .collect();
            assert_eq!(requests.len(), before + 1, "{branch}: no duplicate request");
            let request = requests[before];
            assert!(
                request.injected,
                "{branch}: failure is explicitly proxy-injected"
            );
            assert_eq!(request.forwarded, forwarded, "{branch}");
            assert_eq!(request.status, status, "{branch}");
            assert_eq!(
                server.proxy.injected_count("POST", SESSION_PATH),
                injected_before + 1,
                "{branch}",
            );
            assert_eq!(server.proxy.max_in_flight("POST", SESSION_PATH), 1);
        }

        // An explicit retry goes through the real server after the transport faults are cleared.
        server.proxy.clear();
        browser.fill("#login-password", PASSWORD);
        browser.click("#login-submit");
        wait_for_overview(&mut browser);
        browser.wait("document.body.innerText.includes('7427001')");
    }
}

fn open_login(browser: &mut Browser, server: &Server) {
    browser.navigate(&server.url("/login/"));
    browser.wait("!!document.querySelector('#login-submit')");
    assert_eq!(
        browser.eval(
            "document.documentElement.lang === 'ko' \
             && document.querySelector('#login-password').type === 'password' \
             && document.documentElement.scrollWidth <= innerWidth",
        ),
        json!(true),
    );
}

fn wait_for_overview(browser: &mut Browser) {
    browser.wait(
        "location.pathname === '/' && !!document.querySelector('#operations-refresh') \
         && document.querySelectorAll('table').length === 3 \
         && document.body.innerText.includes('활성 없음') \
         && document.body.innerText.includes('최근 종료 없음')",
    );
}

fn notice(browser: &mut Browser) -> Value {
    browser.eval("document.querySelector('[data-notice] .alert-title')?.textContent ?? null")
}

fn fail_credentials(
    browser: &mut Browser,
    server: &Server,
    account: &str,
    password: &str,
    attempts: usize,
) {
    browser.fill("#login-account", account);
    browser.fill("#login-password", password);
    browser.click("#login-submit");
    browser.wait(
        "!!document.querySelector('[data-notice=failed]') \
         && !document.querySelector('#login-submit').disabled \
         && document.querySelector('#login-password').value === ''",
    );
    assert_eq!(notice(browser), json!(FAILED));
    assert_eq!(
        browser.eval("document.querySelector('#login-account').value"),
        json!(account),
    );
    assert_eq!(server.proxy.count("POST", SESSION_PATH), attempts);
    wait_for("real credential refusal completes", LIMIT, || {
        server
            .proxy
            .traffic()
            .iter()
            .filter(|request| request.method == "POST" && request.path == SESSION_PATH)
            .nth(attempts - 1)
            .is_some_and(|request| {
                request.forwarded
                    && !request.injected
                    && request.status == Some(401)
                    && request.completed.is_some()
            })
    });
}

fn assert_private_login(browser: &mut Browser, server: &Server, protected_id: &str) {
    let secrets = json!([PASSWORD, WRONG_PASSWORD, protected_id]);
    assert_eq!(
        browser.eval(&format!(
            "location.pathname === '/login/' && !!document.querySelector('#login-root') \
             && !document.querySelector('#app-root, #operations-refresh, #session-terminate, table') \
             && document.querySelectorAll('[data-notice]').length <= 1 \
             && document.querySelector('#login-password').type === 'password' \
             && document.documentElement.scrollWidth <= innerWidth \
             && {secrets}.every(secret => !document.body.innerText.includes(secret) \
               && !location.href.includes(secret))"
        )),
        json!(true),
    );
    for path in QUERY_PATHS {
        assert_eq!(server.proxy.count("GET", path), 0);
    }
}
