//! SCR-006 real-process data, filters, search, cursors and session navigation.
//! All scenario markers live in the root runner, not in these helpers.

use std::fs;
use std::io::{Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

use super::live::{Admin, Browser, Fault, LIMIT, Server, initial, must, wait_for};

pub(super) const API: &str = "/api/v1/logs";

pub(super) fn start(name: &str, poll: u32, max: usize) -> Server {
    Server::start_with_config_transform(name, Some(poll), true, |config, _| {
        config
            + &format!(
                "\n[recent_logs]\nmax_entries = {max}\n[reload]\nsettle_secs = 1\npoll_secs = 5\n"
            )
    })
}

pub(super) fn items(raw: &Value) -> &[Value] {
    raw["items"]
        .as_array()
        .unwrap_or_else(|| panic!("log items missing: {raw}"))
}

pub(super) fn generate(server: &Server, count: usize) {
    // Each real invalid Repeater request records accepted + rejected. Read the
    // close before opening another socket so L and OS scheduling cannot hide it.
    for _ in 0..count {
        let mut wire = server.handshaking();
        must(wire.write_all(&initial("!")));
        let mut closed = [0; 1];
        assert_eq!(must(wire.read(&mut closed)), 0);
    }
}

pub(super) fn seed_error(server: &Server, admin: &Admin) {
    // ops-info-v2 §1.5: an actual poll read failure is error-level
    // reload_watch_failed. A directory cannot be read as the TOML file on either
    // OS. Restore the original file after observing the real event, not a timer.
    let path = server.config_path();
    let backup = server.dir().join("saved-config.toml");
    must(fs::rename(&path, &backup));
    must(fs::create_dir(&path));
    wait_for("real error-level watcher event", LIMIT, || {
        !items(&server.query(admin, "/api/v1/logs?level=error")).is_empty()
    });
    must(fs::remove_dir(&path));
    must(fs::rename(backup, path));
}

pub(super) fn ready(browser: &mut Browser) {
    browser.wait("document.querySelector('#logs-refresh')?.disabled === false && !document.querySelector('.log-meta')?.textContent.includes('확인 불가') && !document.querySelector('.log-meta')?.textContent.includes('Unavailable')");
}

pub(super) fn open(browser: &mut Browser, server: &Server) {
    browser.login(server);
    browser.navigate(&server.url("/#/logs"));
    ready(browser);
}

pub(super) fn pause(browser: &mut Browser) {
    if browser.eval("document.querySelector('#logs-auto-refresh')?.getAttribute('aria-checked')")
        == json!("true")
    {
        browser.click("#logs-auto-refresh");
    }
}

pub(super) fn paths(server: &Server) -> Vec<String> {
    server
        .proxy
        .traffic()
        .into_iter()
        .filter(|r| r.method == "GET" && r.path.starts_with(API))
        .map(|r| r.path)
        .collect()
}

pub(super) fn last_reply(server: &Server) -> (String, Value) {
    let traffic = server.proxy.traffic();
    let r = traffic
        .iter()
        .rev()
        .find(|r| r.path.starts_with(API) && r.status == Some(200) && r.completed.is_some())
        .unwrap_or_else(|| panic!("missing browser logs response"));
    assert!(r.forwarded && !r.response_body.is_empty());
    (r.path.clone(), must(serde_json::from_str(&r.response_body)))
}

pub(super) fn seqs(browser: &mut Browser) -> Value {
    browser.eval("[...document.querySelectorAll('.log-row')].map(r=>Number(r.id.slice(6)))")
}

pub(super) fn assert_items(browser: &mut Browser, raw: &Value) {
    let expected: Vec<_> = items(raw)
        .iter()
        .rev()
        .map(|item| item["seq"].clone())
        .collect();
    assert_eq!(
        seqs(browser),
        json!(expected),
        "latest-first exact real GET items"
    );
    assert_eq!(browser.eval(&format!(r#"(() => {{
        const items = {raw}.items;
        const pad = n => String(n).padStart(2,'0');
        const local = ts => {{const d=new Date(ts); const zone=new Intl.DateTimeFormat('en-US',{{timeZoneName:'short'}}).formatToParts(d).find(p=>p.type==='timeZoneName').value;return `${{d.getFullYear()}}-${{pad(d.getMonth()+1)}}-${{pad(d.getDate())}} ${{pad(d.getHours())}}:${{pad(d.getMinutes())}}:${{pad(d.getSeconds())}} ${{zone}}`;}};
        return items.every(item=>{{
            const r=document.getElementById('log-r-'+item.seq), e=item.entry;
            const fields=[...document.getElementById('log-d-'+item.seq).querySelectorAll('.log-detail > div')];
            return r.querySelector('.time').textContent===local(e.ts) && r.querySelector('.badge').textContent===e.level && r.querySelector('.ev').textContent===e.event &&
                fields.length===Object.keys(e).length && fields.every(f=>{{const k=f.querySelector('dt').textContent,v=e[k],text=f.querySelector('dd').textContent;return v!==null&&typeof v==='object'?JSON.stringify(JSON.parse(text))===JSON.stringify(v):text===String(v);}}) &&
                [...r.querySelectorAll('.summary > div')].every(f=>f.querySelector('dd').textContent===String(e[f.querySelector('dt').textContent]));
        }});
    }})()"#)), json!(true), "time, level, event, summary and every entry field");
}

pub(super) fn refresh(browser: &mut Browser) {
    browser.click("#logs-refresh");
    ready(browser);
}

pub(super) fn select_event(browser: &mut Browser, event: &str) {
    browser.eval(&format!("(()=>{{const e=document.querySelector('#logs-event');e.value={};e.dispatchEvent(new Event('change',{{bubbles:true}}));}})()", json!(event)));
    ready(browser);
}

pub(super) fn search(browser: &mut Browser, value: &str, enter: bool) {
    browser.fill("#logs-q", value);
    if enter {
        browser.key("Enter");
    } else {
        browser.click("#logs-search");
    }
    ready(browser);
}

pub fn first_load_and_all_entry_fields() {
    let server = start("dash-054", 60, 1000);
    let admin = server.login();
    seed_error(&server, &admin);
    generate(&server, 2);
    let mut browser = Browser::start("dash-054", &["ko-KR"]);
    browser.login(&server);
    let release = Arc::new(AtomicBool::new(false));
    server
        .proxy
        .once("GET", API, Fault::Hold(Arc::clone(&release)));
    browser.navigate(&server.url("/#/logs"));
    wait_for("first real logs held", LIMIT, || {
        server
            .proxy
            .traffic()
            .iter()
            .any(|r| r.path == API && r.stage == "response_held")
    });
    assert_eq!(browser.eval("document.querySelector('.log-meta').textContent.includes('조회 중') && document.querySelector('.log-meta').textContent.includes('확인 불가') && !document.querySelector('.log-meta').textContent.includes('0 / 0') && document.querySelectorAll('.log-row').length===0 && document.querySelector('#logs-download').disabled"), json!(true));
    release.store(true, Ordering::Release);
    ready(&mut browser);
    let (_, raw) = last_reply(&server);
    assert_items(&mut browser, &raw);
    let levels: std::collections::BTreeSet<_> = items(&raw)
        .iter()
        .filter_map(|i| i["entry"]["level"].as_str())
        .collect();
    assert_eq!(levels, ["error", "info", "warn"].into_iter().collect());
    assert!(
        items(&raw)
            .iter()
            .any(|i| i["entry"]["listeners"].is_array())
    );
    assert_eq!(browser.eval(&format!("document.querySelector('.log-meta').textContent.includes('보관 ' + ({}.latest_seq-{}.oldest_seq+1) + ' / 최대 1,000')",raw,raw)), json!(true));
    for item in items(&raw) {
        let seq = &item["seq"];
        browser.click(&format!("#log-r-{seq} .act button"));
        assert_eq!(browser.eval(&format!("!document.getElementById('log-d-{seq}').hidden && document.querySelector('#log-r-{seq} button').getAttribute('aria-expanded')==='true'")),json!(true));
        browser.click(&format!("#log-r-{seq} .act button"));
        assert_eq!(
            browser.eval(&format!("document.getElementById('log-d-{seq}').hidden")),
            json!(true)
        );
    }
    assert_eq!(server.proxy.injected_count("GET", API), 1);
    let direct = server.query(&admin, API);
    assert_eq!(raw["items"], direct["items"]);
}

pub fn level_event_filters_and_obsolete_reply() {
    let server = start("dash-055", 60, 1000);
    let admin = server.login();
    seed_error(&server, &admin);
    generate(&server, 2);
    let mut browser = Browser::start("dash-055", &["ko-KR"]);
    open(&mut browser, &server);
    pause(&mut browser);
    for level in ["info", "warn", "error"] {
        for candidate in ["info", "warn", "error"] {
            let selector = format!("#logs-level-{candidate}");
            let wanted = candidate == level;
            if browser.eval(&format!(
                "document.querySelector({}).checked",
                json!(selector)
            )) != json!(wanted)
            {
                browser.click(&selector);
            }
        }
        ready(&mut browser);
        let (path, raw) = last_reply(&server);
        assert!(path.contains(&format!("level={level}")));
        assert!(!items(&raw).is_empty());
        assert!(items(&raw).iter().all(|i| i["entry"]["level"] == level));
        assert_items(&mut browser, &raw);
        assert_eq!(raw["items"], server.query(&admin, &path)["items"]);
    }
    browser.click("#logs-level-info");
    ready(&mut browser);
    select_event(&mut browser, "server_ready");
    let (path, raw) = last_reply(&server);
    assert!(path.contains("level=info%2Cerror") && path.contains("event=server_ready"));
    assert_eq!(items(&raw).len(), 1);
    assert_items(&mut browser, &raw);
    browser.click("#logs-level-warn");
    ready(&mut browser);
    select_event(&mut browser, "");
    assert_eq!(last_reply(&server).0, API);
    for level in ["info", "warn"] {
        browser.click(&format!("#logs-level-{level}"));
        ready(&mut browser);
    }
    let before = paths(&server).len();
    browser.click("#logs-level-error");
    thread::sleep(Duration::from_millis(1200));
    assert_eq!(paths(&server).len(), before, "zero levels sends no request");
    assert!(browser.text().contains("수준을 하나 이상"));
    browser.click("#logs-level-info");
    ready(&mut browser);
    let held = Arc::new(AtomicBool::new(false));
    let old = "/api/v1/logs?level=info&event=server_ready";
    server
        .proxy
        .once("GET", old, Fault::Hold(Arc::clone(&held)));
    select_event_pending(&mut browser, "server_ready");
    wait_for("old filter response held", LIMIT, || {
        server
            .proxy
            .traffic()
            .iter()
            .any(|r| r.path == old && r.stage == "response_held")
    });
    select_event(&mut browser, "admin_login_succeeded");
    let (_, current) = last_reply(&server);
    held.store(true, Ordering::Release);
    wait_for("old filter released", LIMIT, || {
        server
            .proxy
            .traffic()
            .iter()
            .any(|r| r.path == old && r.completed.is_some())
    });
    thread::sleep(Duration::from_millis(100));
    assert_items(&mut browser, &current);
    assert_eq!(server.proxy.injected_count("GET", old), 1);
}

fn select_event_pending(browser: &mut Browser, event: &str) {
    browser.eval(&format!("(()=>{{const e=document.querySelector('#logs-event');e.value={};e.dispatchEvent(new Event('change',{{bubbles:true}}));}})()",json!(event)));
}

/// Values only, ASCII case-insensitive, like the log search `q`. `needle` is lowercase.
fn value_contains(value: &Value, needle: &str) -> bool {
    match value {
        Value::Object(map) => map.values().any(|v| value_contains(v, needle)),
        Value::Array(list) => list.iter().any(|v| value_contains(v, needle)),
        Value::String(text) => text.to_ascii_lowercase().contains(needle),
        other => other.to_string().contains(needle),
    }
}

pub fn unicode_value_search_and_boundaries() {
    let server = start("dash-056", 60, 1000);
    let admin = server.login();
    let mut numeric = server.pair(&admin, "II", "45601");
    numeric.transfer(517, 619);
    let numeric_sid = numeric.sid.clone();
    numeric.close();
    wait_for("numeric real close event", LIMIT, || {
        items(&server.query(&admin, "/api/v1/logs?event=session_closed"))
            .iter()
            .any(|i| i["entry"]["session_id"] == numeric_sid)
    });
    let mut browser = Browser::start("dash-056", &["ko-KR"]);
    open(&mut browser, &server);
    pause(&mut browser);
    for (value, enter) in [("s".to_owned(), true), ("🦀".repeat(256), false)] {
        let before = paths(&server).len();
        browser.fill("#logs-q", &value);
        assert_eq!(paths(&server).len(), before, "typing alone never queries");
        if enter {
            browser.key("Enter");
        } else {
            browser.click("#logs-search");
        }
        wait_for("search request submitted", LIMIT, || {
            paths(&server).len() > before
        });
        ready(&mut browser);
        let (path, raw) = last_reply(&server);
        assert!(
            path.contains("q="),
            "query missing for {value:?}: {path}; {}",
            browser.text()
        );
        assert_items(&mut browser, &raw);
        assert_eq!(raw["items"], server.query(&admin, &path)["items"]);
    }
    let before = paths(&server).len();
    browser.fill("#logs-q", &"🦀".repeat(257));
    browser.key("Enter");
    browser.wait("document.querySelector('#logs-q').getAttribute('aria-invalid')==='true'");
    assert_eq!(paths(&server).len(), before);
    search(&mut browser, "", false);
    assert!(!last_reply(&server).0.contains("q="));
    let raw = server.query(&admin, API);
    let ready_event = items(&raw)
        .iter()
        .find(|i| i["entry"]["event"] == "server_ready")
        .unwrap_or_else(|| panic!("real nested event"));
    let listen = ready_event["entry"]["listeners"][0]["listen"]
        .as_str()
        .unwrap_or("");
    // listeners is a real array of nested objects. Search their values, numbers
    // and ASCII mixed case; a field name alone must not match. server_ready has
    // both probed field names but neither in a value. Other entries may hold a
    // probe in a value: a random session_id can contain "ts".
    for value in ["SeRvEr_ReAdY", listen, "127.0.0.1", "listeners", "ts"] {
        search(&mut browser, value, true);
        let (path, actual) = last_reply(&server);
        let oracle = server.query(&admin, &path);
        assert_eq!(actual["items"], oracle["items"]);
        assert_items(&mut browser, &actual);
        let has_ready = items(&actual)
            .iter()
            .any(|i| i["seq"] == ready_event["seq"]);
        if value == "listeners" || value == "ts" {
            assert!(!has_ready, "field name {value:?} matched: {actual}");
            for item in items(&actual) {
                assert!(
                    value_contains(&item["entry"], value),
                    "{value:?} matched no value: {item}"
                );
            }
        } else {
            assert!(has_ready);
        }
    }
    search(&mut browser, "517", false);
    let (numeric_path, numeric_result) = last_reply(&server);
    assert_eq!(
        numeric_result["items"],
        server.query(&admin, &numeric_path)["items"]
    );
    assert!(
        items(&numeric_result)
            .iter()
            .any(|i| i["entry"]["session_id"] == numeric_sid
                && i["entry"]["bytes_viewer_to_target"] == 517)
    );
    search(&mut browser, "", true);
    assert_eq!(last_reply(&server).0, API);
}

pub fn polling_pages_pause_no_overlap_and_eviction() {
    let server = start("dash-057", 1, 10000);
    let admin = server.login();
    let mut browser = Browser::start("dash-057", &["ko-KR"]);
    open(&mut browser, &server);
    let (_, first) = last_reply(&server);
    let cursor = first["latest_seq"].as_u64().unwrap_or(0);
    let path = format!("{API}?after={cursor}");
    server
        .proxy
        .once("GET", &path, Fault::Delay(Duration::from_millis(1800)));
    generate(&server, 130);
    browser.wait(&format!(
        "document.querySelectorAll('.log-row').length > {}",
        items(&first).len() + 200
    ));
    pause(&mut browser);
    let increment = server.query(&admin, &format!("{API}?after={cursor}&limit=1000"));
    let mut expected = items(&first).to_vec();
    expected.extend_from_slice(items(&increment));
    assert_items(&mut browser, &json!({"items":expected}));
    let records = server.proxy.traffic();
    let polls: Vec<_> = records
        .iter()
        .filter(|r| {
            r.path.starts_with(API) && r.headers.to_ascii_lowercase().contains("x-relay-poll: 1")
        })
        .collect();
    assert!(polls.len() >= 2, "after must drain multiple pages");
    assert!(
        polls
            .iter()
            .all(|r| r.path.contains("after=") && !r.path.contains("before="))
    );
    let initial = records
        .iter()
        .find(|r| r.path == API && r.status == Some(200))
        .unwrap_or_else(|| panic!("initial read"));
    assert!(
        polls[0]
            .at
            .saturating_duration_since(initial.completed.unwrap_or(initial.at))
            >= Duration::from_millis(900)
    );
    for pair in polls.windows(2) {
        assert!(
            pair[1].at >= pair[0].completed.unwrap_or(pair[0].at),
            "no overlapping page/poll"
        );
    }
    assert_eq!(server.proxy.injected_count("GET", &path), 1);
    assert_eq!(browser.eval("(()=>{const s=[...document.querySelectorAll('.log-row')].map(r=>Number(r.id.slice(6)));return new Set(s).size===s.length && s.every((v,i)=>i===0||s[i-1]>v);})()"),json!(true));
    let paused = paths(&server).len();
    generate(&server, 1);
    thread::sleep(Duration::from_millis(1300));
    assert_eq!(paths(&server).len(), paused);
    refresh(&mut browser);
    let manual = server
        .proxy
        .traffic()
        .into_iter()
        .rev()
        .find(|r| r.path == API)
        .unwrap_or_else(|| panic!("manual request"));
    assert!(
        !manual
            .headers
            .to_ascii_lowercase()
            .contains("x-relay-poll:")
    );
    select_event(&mut browser, "server_ready");
    browser.click("#logs-auto-refresh");
    let before = paths(&server).len();
    generate(&server, 2);
    wait_for("nonmatching automatic response", LIMIT, || {
        paths(&server).len() > before
    });
    browser.wait("document.querySelector('#logs-refresh')?.disabled===false");
    assert!(
        !browser.text().contains("전체 이력이 아닙니다"),
        "filter mismatch is not eviction"
    );
    let small = start("dash-057-gap", 1, 8);
    let mut other = Browser::start("dash-057-gap", &["ko-KR"]);
    open(&mut other, &small);
    pause(&mut other);
    generate(&small, 12);
    other.click("#logs-auto-refresh");
    other.wait("document.body.innerText.includes('전체 이력이 아닙니다')");
    let current = small.query(&small.login(), API);
    assert_eq!(current["max_entries"], 8);
    assert!(current["oldest_seq"].as_u64().unwrap_or(0) > 8);
}

pub fn older_pages_match_real_cursor_results() {
    let server = start("dash-058", 60, 2000);
    let admin = server.login();
    generate(&server, 230);
    let mut browser = Browser::start("dash-058", &["ko-KR"]);
    open(&mut browser, &server);
    pause(&mut browser);
    let (_, first) = last_reply(&server);
    let mut collected = items(&first).to_vec();
    assert_eq!(collected.len(), 200);
    for _ in 0..4 {
        let cursor = collected
            .iter()
            .filter_map(|i| i["seq"].as_u64())
            .min()
            .unwrap_or(0);
        browser.click("#logs-older");
        ready(&mut browser);
        let (path, page) = last_reply(&server);
        assert_eq!(path, format!("{API}?before={cursor}"));
        assert_eq!(page["items"], server.query(&admin, &path)["items"]);
        collected.extend_from_slice(items(&page));
        collected.sort_by_key(|i| i["seq"].as_u64().unwrap_or(0));
        let all = json!({"items":collected});
        assert_items(&mut browser, &all);
        if page["has_more"] == false {
            assert_eq!(
                browser.eval("document.querySelector('#logs-older')===null"),
                json!(true)
            );
            assert_eq!(
                collected.len(),
                items(&server.query(&admin, "/api/v1/logs?limit=1000")).len()
            );
            return;
        }
    }
    panic!("older paging never reached has_more=false");
}

pub fn session_link_invalid_id_clear_and_retention_limit() {
    let server = start("dash-059", 60, 1000);
    let admin = server.login();
    let pair = server.pair(&admin, "II", "45901");
    let mut browser = Browser::start("dash-059", &["ko-KR"]);
    browser.login(&server);
    browser.navigate(&server.url(&format!("/#/sessions/{}", pair.sid)));
    browser.wait("document.querySelector('#session-refresh')?.disabled===false");
    browser.click(&format!("a[href='/#/logs?session_id={}']", pair.sid));
    ready(&mut browser);
    pause(&mut browser);
    assert!(browser.text().contains(&pair.sid));
    let (path, raw) = last_reply(&server);
    assert!(path.contains(&format!("session_id={}", pair.sid)));
    assert_eq!(raw["items"], server.query(&admin, &path)["items"]);
    assert!(
        items(&raw)
            .iter()
            .any(|i| i["entry"]["event"] == "session_started")
    );
    assert!(
        items(&raw)
            .iter()
            .any(|i| i["entry"]["conn_id"].is_string())
    );
    assert!(browser.text().contains("세션 시작 로그가 제거됐으면"));
    select_event(&mut browser, "connection_accepted");
    search(&mut browser, "127.0.0.1", true);
    browser.click("#logs-level-error");
    ready(&mut browser);
    browser.click("#logs-session-clear");
    ready(&mut browser);
    let cleared = last_reply(&server).0;
    assert!(
        cleared.contains("event=connection_accepted")
            && cleared.contains("level=info%2Cwarn")
            && cleared.contains("q=127.0.0.1")
            && !cleared.contains("session_id")
    );
    let before = paths(&server).len();
    browser.navigate(&server.url("/#/logs?session_id=invalid%2Fid"));
    browser.wait("document.querySelector('.logs-session-error')!==null");
    thread::sleep(Duration::from_millis(1200));
    assert_eq!(paths(&server).len(), before);
    assert!(browser.text().contains("잘못된 세션 식별자"));
    let small = start("dash-059-limited", 60, 8);
    let a = small.login();
    let active = small.pair(&a, "II", "45902");
    generate(&small, 12);
    let mut b = Browser::start("dash-059-limited", &["ko-KR"]);
    b.login(&small);
    b.navigate(&small.url(&format!("/#/logs?session_id={}", active.sid)));
    ready(&mut b);
    assert!(items(&last_reply(&small).1).is_empty());
    assert!(b.text().contains("세션 시작 로그가 제거됐으면"));
    pair.close();
    active.close();
}
