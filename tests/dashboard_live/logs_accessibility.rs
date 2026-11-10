//! CDP key input (not synthetic DOM key events) at both widths and languages.

use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use serde_json::json;

use super::live::{Browser, Fault, LIMIT, connect, must, read_reply, wait_for};
use super::logs::{API, generate, items, last_reply, paths, ready, start};

fn key(browser: &mut Browser, key: &str, code: u32, modifiers: u32) {
    for kind in ["keyDown", "keyUp"] {
        browser.command("Input.dispatchKeyEvent",json!({"type":kind,"key":key,"code":if key==" " {"Space"} else {key},"windowsVirtualKeyCode":code,"nativeVirtualKeyCode":code,"modifiers":modifiers}));
    }
}

fn tab_to(browser: &mut Browser, selector: &str) {
    // Compute only the shorter Tab direction. Every focus movement and action
    // below is still dispatched as a genuine browser keyboard event.
    let backward=browser.eval(&format!(r#"(()=>{{
        const nodes=[...document.querySelectorAll('a[href],button,input,select,textarea')].filter(n=>!n.disabled&&n.getBoundingClientRect().width>0);
        const from=nodes.indexOf(document.activeElement),to=nodes.indexOf(document.querySelector({}));
        if(to<0)throw new Error('keyboard target not reachable');
        return from>=0&&((from-to+nodes.length)%nodes.length)<((to-from+nodes.length)%nodes.length);
    }})()"#,json!(selector))).as_bool().unwrap_or(false);
    for _ in 0..260 {
        if browser.eval(&format!(
            "document.activeElement===document.querySelector({})",
            json!(selector)
        )) == json!(true)
        {
            return;
        }
        key(browser, "Tab", 9, if backward { 8 } else { 0 });
    }
    panic!("Tab failed to reach {selector}");
}

fn activate(browser: &mut Browser, selector: &str) {
    tab_to(browser, selector);
    browser.key("Enter");
}

fn layout(browser: &mut Browser) {
    assert_eq!(browser.eval(r#"(()=>{
        const links=[...document.querySelectorAll('.navbar-item')];
        return document.documentElement.scrollWidth<=innerWidth && links.length===3 &&
            [...document.querySelectorAll('input,select')].every(e=>!!document.querySelector('label[for="'+e.id+'"]')||!!e.closest('label')) &&
            [...document.querySelectorAll('[aria-describedby]')].every(e=>e.getAttribute('aria-describedby').split(' ').every(id=>document.getElementById(id))) &&
            !!document.querySelector('.log-list[aria-labelledby="logs-list-heading"]') &&
            !!document.querySelector('.log-table caption') &&
            [...document.querySelectorAll('.log-row button')].every(b=>document.getElementById(b.getAttribute('aria-controls')) && ['true','false'].includes(b.getAttribute('aria-expanded'))) &&
            !document.querySelector('.log-list')?.closest('[aria-live],[role=status],[role=alert]');
    })()"#),json!(true),"labels, descriptions, list semantics, expansion references and no row live region");
}

pub fn keyboard_languages_widths_and_state_preservation() {
    for width in [1280, 390] {
        for lang in ["ko", "en"] {
            let server = start("dash-063", 1, 2000);
            let admin = server.login();
            let session = server.pair(&admin, "II", "46301");
            let mut origin = connect(server.ws_port);
            must(origin.write_all(format!("GET /relay/viewer HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\nOrigin: https://{}.example\r\n\r\n","long-origin-".repeat(24)).as_bytes()));
            assert_eq!(read_reply(origin).status, 403);
            generate(&server, 105);
            let mut browser =
                Browser::start("dash-063", &[if lang == "ko" { "ko-KR" } else { "en-US" }]);
            browser.set_width(width);
            browser.login(&server);
            let release = Arc::new(AtomicBool::new(false));
            server
                .proxy
                .once("GET", API, Fault::Hold(Arc::clone(&release)));
            browser.navigate(&server.url("/#/logs"));
            wait_for("keyboard loading state", LIMIT, || {
                server
                    .proxy
                    .traffic()
                    .iter()
                    .any(|r| r.path == API && r.stage == "response_held")
            });
            assert_eq!(browser.eval("document.querySelector('#logs-refresh').disabled && document.querySelector('#logs-download').disabled"),json!(true));
            release.store(true, Ordering::Release);
            ready(&mut browser);
            assert_eq!(browser.eval("document.documentElement.lang"), json!(lang));
            layout(&mut browser);
            activate(&mut browser, "#logs-auto-refresh");
            assert_eq!(
                browser.eval(
                    "document.querySelector('#logs-auto-refresh').getAttribute('aria-checked')"
                ),
                json!("false")
            );
            activate(&mut browser, "#logs-refresh");
            ready(&mut browser);
            tab_to(&mut browser, "#logs-event");
            key(&mut browser, "End", 35, 0);
            browser.wait("document.querySelector('#logs-event').value!==''");
            ready(&mut browser);
            assert!(last_reply(&server).0.contains("event="));
            tab_to(&mut browser, "#logs-event");
            key(&mut browser, "Home", 36, 0);
            browser.wait("document.querySelector('#logs-event').value===''");
            ready(&mut browser);
            activate(&mut browser, "#logs-older");
            ready(&mut browser);
            assert_eq!(
                browser.eval("document.querySelector('#logs-older')===null"),
                json!(true)
            );
            let actual = server.query(&admin, "/api/v1/logs?event=server_ready");
            let seq = &items(&actual)[0]["seq"];
            let button = format!("#log-r-{seq} button");
            activate(&mut browser, &button);
            assert_eq!(
                browser.eval(&format!("!document.getElementById('log-d-{seq}').hidden")),
                json!(true)
            );
            layout(&mut browser);
            tab_to(&mut browser, "#logs-q");
            browser.command("Input.insertText", json!({"text":"draft preserved 🦀"}));
            browser.eval("document.querySelector('#logs-q').setSelectionRange(2,7)");
            // Turn on auto refresh by key, return to the input by Tab, then let
            // an actual new event trigger a complete render.
            activate(&mut browser, "#logs-auto-refresh");
            tab_to(&mut browser, "#logs-q");
            browser.eval("document.querySelector('#logs-q').setSelectionRange(2,7)");
            let count = paths(&server).len();
            generate(&server, 1);
            wait_for("keyboard automatic poll", LIMIT, || {
                paths(&server).len() > count
            });
            ready(&mut browser);
            assert_eq!(browser.eval(&format!("document.activeElement.id==='logs-q' && document.querySelector('#logs-q').value==='draft preserved 🦀' && document.querySelector('#logs-q').selectionStart===2 && document.querySelector('#logs-q').selectionEnd===7 && !document.getElementById('log-d-{seq}').hidden")),json!(true));
            activate(&mut browser, "#logs-auto-refresh");
            tab_to(&mut browser, "#logs-language");
            key(
                &mut browser,
                if lang == "ko" { "End" } else { "Home" },
                if lang == "ko" { 35 } else { 36 },
                0,
            );
            let changed = if lang == "ko" { "en" } else { "ko" };
            browser.wait(&format!(
                "document.documentElement.lang==={}",
                json!(changed)
            ));
            assert_eq!(browser.eval(&format!("document.querySelector('#logs-q').value==='draft preserved 🦀' && !document.getElementById('log-d-{seq}').hidden")),json!(true));
            // Search validation, empty result and clear, using keys only.
            tab_to(&mut browser, "#logs-q");
            browser.command("Input.dispatchKeyEvent",json!({"type":"keyDown","key":"a","code":"KeyA","windowsVirtualKeyCode":65,"modifiers":2}));
            browser.command("Input.dispatchKeyEvent",json!({"type":"keyUp","key":"a","code":"KeyA","windowsVirtualKeyCode":65,"modifiers":2}));
            browser.command("Input.insertText", json!({"text":"🦀".repeat(257)}));
            browser.key("Enter");
            browser.wait("document.querySelector('#logs-q').getAttribute('aria-invalid')==='true'");
            assert_eq!(browser.eval("document.activeElement.id==='logs-q' && document.querySelector('#logs-q').getAttribute('aria-describedby').includes('logs-q-error')"),json!(true));
            browser.command("Input.dispatchKeyEvent",json!({"type":"keyDown","key":"a","code":"KeyA","windowsVirtualKeyCode":65,"modifiers":2}));
            browser.command("Input.dispatchKeyEvent",json!({"type":"keyUp","key":"a","code":"KeyA","windowsVirtualKeyCode":65,"modifiers":2}));
            browser.command("Input.insertText", json!({"text":"unmatched-value"}));
            activate(&mut browser, "#logs-search");
            ready(&mut browser);
            assert!(items(&last_reply(&server).1).is_empty());
            for level in ["info", "warn", "error"] {
                tab_to(&mut browser, &format!("#logs-level-{level}"));
                key(&mut browser, " ", 32, 0);
            }
            assert_eq!(
                browser.eval("document.querySelector('#logs-refresh').disabled"),
                json!(true)
            );
            tab_to(&mut browser, "#logs-level-info");
            key(&mut browser, " ", 32, 0);
            ready(&mut browser);
            // Session-chip and malformed-ID states are real protected routes.
            browser.navigate(&server.url(&format!("/#/logs?session_id={}", session.sid)));
            ready(&mut browser);
            assert!(browser.text().contains(&session.sid));
            activate(&mut browser, "#logs-session-clear");
            ready(&mut browser);
            browser.navigate(&server.url("/#/logs?session_id=bad%2Fid"));
            browser.wait("!!document.querySelector('.logs-session-error')");
            activate(&mut browser, "#logs-session-clear");
            ready(&mut browser);
            layout(&mut browser);
            // Actual download is tested in DASH-060; here every width/language
            // reaches and activates the same button while collection is held.
            let download = Arc::new(AtomicBool::new(false));
            server.proxy.once(
                "GET",
                "/api/v1/logs?limit=1000",
                Fault::Hold(Arc::clone(&download)),
            );
            activate(&mut browser, "#logs-download");
            wait_for("keyboard download collecting", LIMIT, || {
                server
                    .proxy
                    .traffic()
                    .iter()
                    .any(|r| r.path == "/api/v1/logs?limit=1000" && r.stage == "response_held")
            });
            assert_eq!(browser.eval("document.querySelector('#logs-download').disabled && document.querySelector('#logs-q').disabled"),json!(true));
            activate(&mut browser, "a.navbar-item[href='/#/settings']");
            browser.wait("!!document.querySelector('#settings-refresh')");
            download.store(true, Ordering::Release);
            server.proxy.clear();
            browser.navigate(&server.url("/#/logs"));
            ready(&mut browser);
            let failed_download = server
                .proxy
                .injected_count("GET", "/api/v1/logs?limit=1000");
            server.proxy.once(
                "GET",
                "/api/v1/logs?limit=1000",
                Fault::Error500("internal"),
            );
            activate(&mut browser, "#logs-download");
            browser.wait("!!document.querySelector('.log-list .alert-tone-destructive')");
            assert_eq!(
                server
                    .proxy
                    .injected_count("GET", "/api/v1/logs?limit=1000"),
                failed_download + 1
            );
            layout(&mut browser);
            let refreshing = Arc::new(AtomicBool::new(false));
            server
                .proxy
                .once("GET", API, Fault::Hold(Arc::clone(&refreshing)));
            activate(&mut browser, "#logs-refresh");
            wait_for("keyboard refreshing state", LIMIT, || {
                server
                    .proxy
                    .traffic()
                    .iter()
                    .any(|r| r.path == API && r.stage == "response_held" && r.completed.is_none())
            });
            assert_eq!(browser.eval("!!document.querySelector('.log-list .card-title .badge') && document.querySelectorAll('.log-row').length>0"),json!(true));
            refreshing.store(true, Ordering::Release);
            ready(&mut browser);
            let stale = super::logs::seqs(&mut browser);
            server.proxy.once("GET", API, Fault::Error500("internal"));
            activate(&mut browser, "#logs-refresh");
            browser.wait("!!document.querySelector('.logs-main > .alert-tone-destructive')");
            assert_eq!(super::logs::seqs(&mut browser), stale);
            activate(&mut browser, "#logs-refresh");
            ready(&mut browser);
            browser.navigate(&server.url("/#/settings"));
            browser.wait("!!document.querySelector('#settings-refresh')");
            // First/stale failures and 400/405 are all keyboard recoverable.
            for (status, body) in [
                (500, r#"{"error":{"code":"internal"}}"#),
                (400, r#"{"error":{"code":"invalid_request"}}"#),
                (405, r#"{"error":{"code":"method_not_allowed"}}"#),
            ] {
                let injected = server.proxy.injected_count("GET", API);
                server.proxy.once("GET", API, Fault::Response(status, body));
                browser.navigate(&server.url("/#/logs"));
                browser.wait("!!document.querySelector('.logs-main > [role=alert]')");
                assert_eq!(server.proxy.injected_count("GET", API), injected + 1);
                activate(&mut browser, "#logs-refresh");
                ready(&mut browser);
                layout(&mut browser);
                browser.navigate(&server.url("/#/settings"));
                browser.wait("!!document.querySelector('#settings-refresh')");
            }
            browser.navigate(&server.url("/#/logs"));
            ready(&mut browser);
            browser.clear_cookies();
            activate(&mut browser, "#logs-refresh");
            browser.wait(
                "location.pathname==='/login/' && !!document.querySelector('#login-account')",
            );
            assert_eq!(
                browser.eval("document.querySelectorAll('.log-row,#logs-q').length"),
                json!(0)
            );
            session.close();
            // Disabled and evicted states use distinct real server settings.
            for max in [0, 8] {
                let constrained = start("dash-063-constrained", 1, max);
                browser.clear_cookies();
                browser.login(&constrained);
                browser.navigate(&constrained.url("/#/logs"));
                ready(&mut browser);
                if max == 0 {
                    assert_eq!(
                        browser.eval("document.querySelector('#logs-q').disabled"),
                        json!(true)
                    );
                    activate(&mut browser, ".logs-disabled a");
                    browser.wait("!!document.querySelector('#settings-refresh')");
                } else {
                    activate(&mut browser, "#logs-auto-refresh");
                    generate(&constrained, 12);
                    activate(&mut browser, "#logs-auto-refresh");
                    browser.wait("!!document.querySelector('.logs-main > .alert-tone-warning')");
                    layout(&mut browser);
                }
                thread::sleep(Duration::from_millis(20));
            }
        }
    }
}
