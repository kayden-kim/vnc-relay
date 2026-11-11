//! Real-process settings view (SCR-004) live browser tests: DASH-028~034, DASH-048~049, MGT-042.

use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

use super::live::{Browser, Fault, LIMIT, Server, must, wait_for};
use super::settings::{
    PendingFile, TICKET_JWKS, login_alice, open_settings, open_settings_edit, script_str,
};

const CONFIG: &str = "/api/v1/config";

fn ready(browser: &mut Browser) {
    browser.wait("document.querySelector('#settings-refresh')?.disabled === false");
}

fn refresh(browser: &mut Browser) {
    browser.click("#settings-refresh");
    ready(browser);
}

fn language(browser: &mut Browser, lang: &str) {
    browser.eval(&format!(
        "(() => {{ const s = document.querySelector('select'); s.value = {}; s.dispatchEvent(new Event('change', {{bubbles:true}})); }})()",
        script_str(lang)
    ));
    browser.wait(&format!(
        "document.documentElement.lang === {}",
        script_str(lang)
    ));
}

fn blocked(browser: &mut Browser) {
    assert_eq!(
        browser.eval("document.querySelector('#settings-edit')?.disabled ?? true"),
        json!(true)
    );
    assert_eq!(
        browser.eval(
            "document.querySelectorAll('input[data-config-key],textarea[data-config-key]').length"
        ),
        json!(0)
    );
    assert_eq!(browser.eval("!document.querySelector('#settings-review:not(:disabled),dialog[open] .button-primary')"), json!(true));
}

// Every row, its order/section, exact effective value, reload badge and diff marker
// is checked against the authenticated real-server oracle, not a second fixture.
fn assert_rows(browser: &mut Browser, raw: &Value, lang: &str) {
    let expected = json!({
        "items":raw["items"], "keys":raw["file"]["config_keys"],
        "empty":if lang == "ko" {"빈 목록"} else {"Empty list"},
        "null":if lang == "ko" {"값 없음"} else {"No value"},
        "dynamic":if lang == "ko" {"재시작 없이 적용"} else {"Applies without restart"},
        "restart":if lang == "ko" {"재시작 필요"} else {"Restart required"},
        "diff":if lang == "ko" {"파일과 다름"} else {"Differs from file"}
    });
    assert_eq!(browser.eval(&format!(r#"(() => {{
        const e = {expected};
        const rows = [...document.querySelectorAll('.config-row')];
        const groups = [...new Set(e.items.map(i => '[' + i.key.slice(0,i.key.lastIndexOf('.')) + ']'))];
        const titles = [...document.querySelectorAll('.card')].filter(c => c.querySelector('.config-list')).map(c => c.querySelector('h2').textContent);
        if (JSON.stringify(titles) !== JSON.stringify(groups) || rows.length !== e.items.length) return false;
        return e.items.every((item,index) => {{
            const row = rows[index], value = row.querySelector('.config-value');
            const labels = [...row.querySelectorAll('.config-application .badge')].map(b => b.textContent.trim());
            const expectedValue = item.value === null ? e.null : Array.isArray(item.value) ? (item.value.length ? item.value.join('') : e.empty) : String(item.value);
            const expectedLabels = [item.reload === 'dynamic' ? e.dynamic : e.restart, ...(e.keys.includes(item.key) ? [e.diff] : [])];
            return row.dataset.configKey === item.key && row.querySelector('dt').textContent === item.key &&
                row.closest('.card').querySelector('h2').textContent === '[' + item.key.slice(0,item.key.lastIndexOf('.')) + ']' &&
                value.textContent === expectedValue &&
                (!Array.isArray(item.value) || !item.value.length || JSON.stringify([...value.querySelectorAll('.config-data')].map(n => n.textContent)) === JSON.stringify(item.value)) &&
                JSON.stringify(labels) === JSON.stringify(expectedLabels);
        }});
    }})()"#)), json!(true), "effective rows disagree with raw GET: {raw}; DOM: {}", browser.text());
}

fn meta(browser: &mut Browser) -> Value {
    browser.eval("document.querySelector('.page-meta')?.textContent")
}

fn assert_query_time(browser: &mut Browser, server: &Server) -> Value {
    let response = server
        .proxy
        .traffic()
        .into_iter()
        .rev()
        .find(|r| r.method == "GET" && r.path == CONFIG && r.status == Some(200))
        .unwrap_or_else(|| panic!("successful browser config response missing"));
    let raw: Value = must(serde_json::from_str(&response.response_body));
    assert_eq!(
        browser.eval(&format!(r#"(() => {{
            const d = new Date({}), pad = n => String(n).padStart(2, '0');
            const stamp = `${{d.getFullYear()}}-${{pad(d.getMonth()+1)}}-${{pad(d.getDate())}} ${{pad(d.getHours())}}:${{pad(d.getMinutes())}}:${{pad(d.getSeconds())}}`;
            const zone = new Intl.DateTimeFormat('en-US', {{timeZoneName:'short'}}).formatToParts(d).find(p => p.type === 'timeZoneName')?.value;
            const label = document.documentElement.lang === 'ko' ? '조회 시각: ' : 'Retrieved at: ';
            return !!zone && document.querySelector('.page-meta').textContent === label + stamp + ' ' + zone;
        }})()"#, raw["as_of"])),
        json!(true),
        "query time must format the actual browser response as_of with its local timezone"
    );
    meta(browser)
}

fn expected_inputs(raw: &Value) -> Value {
    let mut fields: Vec<_> = raw["items"]
        .as_array()
        .unwrap_or_else(|| panic!("config items missing"))
        .iter()
        .filter(|item| item["editable"] == true)
        .map(|item| {
            let value = &item["value"];
            let input = if value.is_boolean() {
                value.clone()
            } else {
                json!(value.as_array().map_or_else(
                    || if value.is_null() {
                        String::new()
                    } else {
                        value
                            .as_str()
                            .map_or_else(|| value.to_string(), str::to_owned)
                    },
                    |values| {
                        values
                            .iter()
                            .map(|v| {
                                v.as_str()
                                    .unwrap_or_else(|| panic!("string array required"))
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                    },
                ))
            };
            (
                item["key"].as_str().unwrap_or("").to_owned(),
                json!([item["key"], input]),
            )
        })
        .collect();
    assert!(
        !fields.is_empty(),
        "edit baseline must contain real editable settings"
    );
    fields.sort_by_key(|(key, _)| {
        if key.starts_with("relay.") && key.ends_with(".allow") {
            0
        } else if key.starts_with("relay.ticket.") {
            7
        } else if key.ends_with(".trusted_proxies") {
            1
        } else if key.ends_with(".allowed_origins") {
            2
        } else if key.starts_with("mode1.") {
            3
        } else if key.starts_with("limits.") {
            4
        } else if key.starts_with("timeouts.") {
            5
        } else {
            6
        }
    });
    json!(fields.into_iter().map(|(_, v)| v).collect::<Vec<_>>())
}

fn assert_nav(browser: &mut Browser, active: &str) {
    assert_eq!(
        browser.eval(&format!(
            r#"(() => {{
        const links = [...document.querySelectorAll('a.navbar-item')];
        return ['/', '/#/logs', '/#/settings'].every(href => {{
            const a = links.find(a => a.getAttribute('href') === href);
            const r = a?.getBoundingClientRect();
            return r?.width > 0 && r.height > 0 && r.left >= 0 && r.right <= innerWidth &&
                (a.getAttribute('aria-current') === 'page') === (href === {});
        }}) && links.filter(a => a.getAttribute('aria-current') === 'page').length === 1;
    }})()"#,
            script_str(active)
        )),
        json!(true)
    );
}

// DASH-028
pub fn settings_navigation_and_protected_direct_routes() {
    let server = Server::start_with_config("dash-028", Some(60), true);
    let mut browser = Browser::start("dash-028", &["ko-KR"]);
    for route in ["/#/settings", "/#/settings/edit", "/#/logs"] {
        browser.navigate(&server.url(route));
        browser.wait(
            "location.pathname === '/login/' && document.querySelector('#login-account') !== null",
        );
        assert_eq!(
            browser.eval("document.querySelectorAll('[data-config-key],.config-row').length"),
            json!(0)
        );
        assert!(!browser.text().contains("limits.waiting"));
        assert_eq!(
            browser.eval("document.querySelectorAll('.log-row,.log-detail,#logs-q').length"),
            json!(0)
        );
    }
    assert!(
        !server
            .proxy
            .traffic()
            .iter()
            .any(|r| r.path == CONFIG && r.status == Some(200)),
        "unauthenticated config GET must never succeed"
    );
    assert_eq!(server.request("GET", CONFIG, &[], b"").status, 401);
    login_alice(&mut browser, &server);
    let admin = server.login();
    let pair = server.pair(&admin, "II", "42801");
    for width in [1280, 390] {
        browser.command(
            "Emulation.setDeviceMetricsOverride",
            json!({"width":width,"height":900,"deviceScaleFactor":1,"mobile":false}),
        );
        browser.navigate(&server.url("/"));
        browser.wait("document.querySelector('#operations-refresh')?.disabled === false");
        assert_nav(&mut browser, "/");
        browser.click("a.navbar-item[href='/#/logs']");
        browser.wait("document.querySelector('#logs-download')?.disabled === false");
        assert_nav(&mut browser, "/#/logs");
        open_settings(&mut browser, &server);
        assert_nav(&mut browser, "/#/settings");
        open_settings_edit(&mut browser, &server);
        assert_nav(&mut browser, "/#/settings");
        browser.click("a.navbar-item[href='/']");
        browser.wait("document.querySelector('#operations-refresh')?.disabled === false");
        assert_nav(&mut browser, "/");
        browser.navigate(&server.url(&format!("/#/sessions/{}", pair.sid)));
        browser.wait("document.querySelector('#session-terminate') !== null");
        assert_nav(&mut browser, "/");
        open_settings(&mut browser, &server);
        assert_nav(&mut browser, "/#/settings");
        browser.click("a.navbar-item[href='/']");
        browser.wait("document.querySelector('#operations-refresh') !== null");
    }
}

// DASH-029
pub fn settings_view_displays_all_effective_values_in_contract_order() {
    let long = format!("{}-accounts.toml", "long-path-segment-".repeat(8));
    let cert = format!("{}certificate.pem", "certificate-path-".repeat(8));
    let key = format!("{}private-key.pem", "private-key-path-".repeat(8));
    let viewer_keys = format!("{}viewer-public.jwks", "public-key-path-".repeat(8));
    let server_keys = format!("{}server-public.jwks", "public-key-path-".repeat(8));
    let server = Server::start_with_config_transform("dash-029", None, true, |config, dir| {
        must(fs::copy(dir.join("accounts.toml"), dir.join(&long)));
        must(fs::write(
            dir.join(&cert),
            "CERTIFICATE_CONTENT_MUST_NOT_APPEAR",
        ));
        must(fs::write(
            dir.join(&key),
            "PRIVATE_KEY_CONTENT_MUST_NOT_APPEAR",
        ));
        must(fs::write(dir.join(&viewer_keys), TICKET_JWKS));
        must(fs::write(dir.join(&server_keys), TICKET_JWKS));
        config
            .replace(
                "accounts_file = \"accounts.toml\"",
                &format!("accounts_file = {long:?}"),
            )
            .replace(
                "allowed_origins = [\"http://127.0.0.1\"]",
                "allowed_origins = []",
            )
            + &format!(
                "\n[relay.wss]\ncert_file = {cert:?}\nkey_file = {key:?}\n[relay.wss.viewer]\nenabled = false\n\
                 [relay.ticket.viewer]\nissuer = \"dashboard-viewer\"\nrelay_host = \"viewer.example.test\"\npublic_keys_file = {viewer_keys:?}\n\
                 [relay.ticket.server]\nissuer = \"dashboard-server\"\nrelay_host = \"server.example.test\"\npublic_keys_file = {server_keys:?}\n"
            )
    });
    let admin = server.login();
    let raw = server.query(&admin, CONFIG);
    let items = raw["items"]
        .as_array()
        .unwrap_or_else(|| panic!("items missing"));
    assert_eq!(items.len(), 75, "every v4 config key must be present");
    assert!(items.iter().any(|i| i["value"].is_null()));
    assert!(items.iter().any(|i| i["value"] == json!([])));
    assert!(
        !server.config_text().contains("buffer_bytes"),
        "exercise the contract default, not an explicit value"
    );
    assert!(
        items
            .iter()
            .any(|i| i["key"] == "limits.buffer_bytes" && i["value"] == 131072)
    );
    assert_eq!(
        items.iter().filter(|i| i["reload"] == "dynamic").count(),
        43
    );
    for name in [
        "relay.ws.trusted_proxies",
        "relay.wss.trusted_proxies",
        "relay.ws.viewer.allow",
        "relay.ws.server.allow",
        "relay.wss.viewer.allow",
        "relay.wss.server.allow",
        "relay.ws.viewer.allowed_origins",
        "relay.ws.server.allowed_origins",
        "relay.wss.viewer.allowed_origins",
        "relay.wss.server.allowed_origins",
    ] {
        assert!(
            items
                .iter()
                .any(|i| i["key"] == name && i["reload"] == "dynamic"),
            "v4 dynamic key {name}"
        );
    }
    let ticket_items: Vec<_> = items
        .iter()
        .filter(|i| {
            i["key"]
                .as_str()
                .is_some_and(|key| key.starts_with("relay.ticket."))
        })
        .collect();
    let mut expected_ticket_keys = Vec::new();
    for role in ["viewer", "server"] {
        for field in [
            "enabled",
            "issuer",
            "relay_host",
            "public_keys_file",
            "max_lifetime_secs",
        ] {
            expected_ticket_keys.push(format!("relay.ticket.{role}.{field}"));
        }
    }
    assert_eq!(
        ticket_items
            .iter()
            .map(|i| i["key"].as_str().unwrap_or(""))
            .collect::<Vec<_>>(),
        expected_ticket_keys
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
    );
    assert!(ticket_items.iter().all(|i| i["reload"] == "dynamic"));
    for role in ["viewer", "server"] {
        let public_keys = ticket_items
            .iter()
            .find(|i| i["key"] == format!("relay.ticket.{role}.public_keys_file"))
            .unwrap_or_else(|| panic!("missing ticket public key path"));
        assert_eq!(public_keys["editable"], false);
    }
    let closed = items
        .iter()
        .position(|i| i["key"] == "recent_closed.max_entries")
        .unwrap_or_else(|| panic!("closed limit"));
    assert_eq!(items[closed + 1]["key"], "recent_logs.max_entries");
    assert_eq!(items[closed + 1]["reload"], "restart");
    let mut browser = Browser::start("dash-029", &["ko-KR"]);
    login_alice(&mut browser, &server);
    open_settings(&mut browser, &server);
    for lang in ["ko", "en"] {
        language(&mut browser, lang);
        assert_rows(&mut browser, &raw, lang);
        let text = browser.text();
        for path in [&long, &cert, &key, &viewer_keys, &server_keys] {
            assert!(text.contains(path));
        }
        for secret in [
            "CERTIFICATE_CONTENT_MUST_NOT_APPEAR",
            "PRIVATE_KEY_CONTENT_MUST_NOT_APPEAR",
            "\"kty\"",
            "\"P-256\"",
            "axfR8uEsQkf4vOblY6RA8ncDfYEt6zOg9KE5RdiYwpY",
            "T-NC4v4af5uO5-tKfA-eFivOM1drMV7Oy7ZAaDe_UfU",
            "password_hash",
            "$argon2id$",
        ] {
            assert!(!text.contains(secret), "file contents exposed: {secret}");
        }
    }
    let _pending_file = PendingFile::write(
        &server,
        &server.config_text().replace("waiting = 8", "waiting = 12"),
    );
    refresh(&mut browser);
    let pending = server.query(&admin, CONFIG);
    assert_eq!(pending["file"]["state"], "pending");
    assert_rows(&mut browser, &pending, "en");
}

// DASH-030
pub fn in_effect_and_pending_settings_allow_edit_without_polling() {
    let server = Server::start_with_config("dash-030", Some(1), true);
    let admin = server.login();
    let mut browser = Browser::start("dash-030", &["ko-KR"]);
    login_alice(&mut browser, &server);
    open_settings(&mut browser, &server);
    assert_eq!(server.proxy.count("GET", CONFIG), 1);
    let raw = server.query(&admin, CONFIG);
    assert_eq!(raw["file"]["state"], "in_effect");
    assert!(
        browser
            .text()
            .contains("설정 파일과 실행 중인 설정이 같습니다.")
    );
    assert_rows(&mut browser, &raw, "ko");
    assert_eq!(
        browser.eval("document.querySelector('#settings-edit')?.disabled"),
        json!(false)
    );
    thread::sleep(Duration::from_secs(3));
    assert_eq!(server.proxy.count("GET", CONFIG), 1);
    let _pending_file = PendingFile::write(
        &server,
        &server.config_text().replace("waiting = 8", "waiting = 12"),
    );
    refresh(&mut browser);
    let pending = server.query(&admin, CONFIG);
    assert_eq!(pending["file"]["state"], "pending");
    assert!(
        !pending["file"]["config_keys"]
            .as_array()
            .is_none_or(Vec::is_empty)
    );
    assert!(browser.text().contains(
        "설정 파일에 재시작 없이 적용할 차이가 있습니다. 표시 값은 실행 중인 설정입니다."
    ));
    assert_rows(&mut browser, &pending, "ko");
    assert_eq!(
        browser.eval("document.querySelector('#settings-edit')?.disabled"),
        json!(false)
    );
    thread::sleep(Duration::from_secs(3));
    assert_eq!(server.proxy.count("GET", CONFIG), 2);
    assert!(
        server
            .proxy
            .traffic()
            .iter()
            .filter(|r| r.path == CONFIG)
            .all(|r| !r.headers.to_ascii_lowercase().contains("x-relay-poll"))
    );
    open_settings_edit(&mut browser, &server);
    assert_eq!(server.proxy.count("GET", CONFIG), 3);
    assert!(
        browser
            .eval("document.querySelectorAll('.config-input').length")
            .as_u64()
            .is_some_and(|n| n > 0)
    );
    thread::sleep(Duration::from_secs(3));
    assert_eq!(
        server.proxy.count("GET", CONFIG),
        3,
        "SCR-005 must not poll settings either"
    );
}

// DASH-031
pub fn restart_pending_settings_show_keys_and_block_edit() {
    let server = Server::start_with_config("dash-031", None, true);
    let admin = server.login();
    let mut browser = Browser::start("dash-031", &["ko-KR"]);
    login_alice(&mut browser, &server);
    open_settings(&mut browser, &server);
    let before = server.query(&admin, CONFIG);
    server.config_write(
        server
            .config_text()
            .replace("idle_secs = 300", "idle_secs = 400"),
    );
    refresh(&mut browser);
    let raw = server.query(&admin, CONFIG);
    assert_eq!(raw["file"]["state"], "restart_required");
    assert_eq!(raw["items"], before["items"]);
    assert!(
        raw["file"]["config_keys"]
            .as_array()
            .is_some_and(|keys| keys.contains(&json!("management.session.idle_secs")))
    );
    assert!(
        browser
            .text()
            .contains("설정 파일에 재시작이 필요한 차이가 있어 편집할 수 없습니다.")
    );
    assert_rows(&mut browser, &raw, "ko");
    blocked(&mut browser);
    browser.navigate(&server.url("/#/settings/edit"));
    ready(&mut browser);
    assert!(
        browser
            .text()
            .contains("설정 파일에 재시작이 필요한 차이가 있어 편집할 수 없습니다.")
    );
    for key in raw["file"]["config_keys"]
        .as_array()
        .unwrap_or_else(|| panic!("diff keys"))
    {
        assert!(browser.text().contains(key.as_str().unwrap_or("missing")));
    }
    blocked(&mut browser);
    assert_eq!(server.proxy.count("PATCH", CONFIG), 0);
}

// DASH-032
pub fn invalid_and_unreadable_settings_keep_effective_values_and_localize_diagnostics() {
    let server = Server::start_with_config("dash-032", None, true);
    let admin = server.login();
    let original = server.config_text();
    let before = server.query(&admin, CONFIG);
    let mut browser = Browser::start("dash-032", &["ko-KR"]);
    login_alice(&mut browser, &server);
    open_settings(&mut browser, &server);
    let cases = [
        (
            "config_missing_key",
            original.replace("allow_targets = [\"127.0.0.1:1-65535\"]", ""),
            "필수 설정 값이 없습니다.",
            "A required setting value is missing.",
        ),
        (
            "config_type_mismatch",
            original.replace("waiting = 8", "waiting = \"eight\""),
            "설정 값의 자료형이 맞지 않습니다.",
            "The setting value has an invalid type.",
        ),
        (
            "config_value_out_of_range",
            original.replace("waiting = 8", "waiting = 0"),
            "설정 값이 허용 범위를 벗어났습니다.",
            "The setting value is outside its allowed range.",
        ),
        (
            "config_relation_violated",
            original.replace("idle_secs = 300", "idle_secs = 4000"),
            "설정 키 사이의 관계를 만족하지 않습니다.",
            "The relationship between setting keys is not valid.",
        ),
        (
            "config_ip_item_invalid",
            original.replace("allow = [\"127.0.0.1\"]", "allow = [\"invalid-ip\"]"),
            "IP·CIDR·범위 항목의 형식이 올바르지 않습니다.",
            "The IP, CIDR, or range item has an invalid format.",
        ),
        (
            "config_ip_range_reversed",
            original.replace(
                "allow = [\"127.0.0.1\"]",
                "allow = [\"127.0.0.2-127.0.0.1\"]",
            ),
            "IP 범위의 시작 주소는 끝 주소 이하여야 합니다.",
            "The start address of an IP range must not exceed its end address.",
        ),
        (
            "config_ip_range_family_mixed",
            original.replace("allow = [\"127.0.0.1\"]", "allow = [\"127.0.0.1-::1\"]"),
            "IP 범위의 두 주소는 모두 IPv4이거나 모두 IPv6여야 합니다.",
            "Both addresses in an IP range must be IPv4 or both must be IPv6.",
        ),
        (
            "config_origin_invalid",
            original.replace("http://127.0.0.1", "invalid-origin"),
            "Origin 항목의 형식이 올바르지 않습니다.",
            "The Origin item has an invalid format.",
        ),
        (
            "config_target_item_invalid",
            original.replace("127.0.0.1:1-65535", "invalid-target"),
            "Mode I 대상 항목의 형식이 올바르지 않습니다.",
            "The Mode I target item has an invalid format.",
        ),
    ];
    for (code, config, ko, en) in cases {
        server.config_write(config);
        refresh(&mut browser);
        let raw = server.query(&admin, CONFIG);
        assert_eq!(raw["file"]["state"], "invalid", "{code}");
        assert_eq!(raw["items"], before["items"]);
        assert!(
            raw["file"]["diagnostics"]
                .as_array()
                .is_some_and(|d| d.iter().any(|d| d["code"] == code)),
            "real server did not produce {code}: {raw}"
        );
        for (lang, translation) in [("ko", ko), ("en", en)] {
            language(&mut browser, lang);
            assert_rows(&mut browser, &raw, lang);
            assert!(browser.text().contains(if lang == "ko" {
                "설정 파일의 검증에 실패해"
            } else {
                "The configuration file failed validation."
            }));
            let diagnostics = browser.eval("[...document.querySelectorAll('.config-diagnostic')].map(d => ({key:d.querySelector('code').textContent,text:d.querySelector('p').textContent}))");
            for diagnostic in raw["file"]["diagnostics"]
                .as_array()
                .unwrap_or_else(|| panic!("diagnostics"))
            {
                assert!(
                    diagnostics.as_array().is_some_and(|rows| rows
                        .iter()
                        .any(|r| r["key"] == diagnostic["key"] && r["text"] == translation)),
                    "{code}/{lang}: {diagnostics}"
                );
            }
            assert!(
                !browser
                    .eval("document.querySelector('.config-diagnostics').textContent")
                    .as_str()
                    .unwrap_or("")
                    .contains(code),
                "known code must be translated, not shown raw"
            );
            blocked(&mut browser);
        }
        browser.eval("location.hash = '#/settings/edit'");
        browser.wait("document.querySelector('#settings-view') !== null");
        ready(&mut browser);
        for (lang, translation) in [("ko", ko), ("en", en)] {
            language(&mut browser, lang);
            assert!(browser.text().contains(translation));
            blocked(&mut browser);
        }
        browser.eval("location.hash = '#/settings'");
        browser.wait("document.querySelector('#settings-edit') !== null");
        ready(&mut browser);
    }
    // An unknown diagnostic code (outside §5.1's 9 translated codes, e.g. config_unknown_key)
    // is produced naturally by a real configuration file with an unrecognized section or key.
    server.config_write(format!("{original}\n[extra]\nkey = 1\n"));
    refresh(&mut browser);
    let raw = server.query(&admin, CONFIG);
    assert_eq!(raw["file"]["state"], "invalid");
    assert_eq!(raw["items"], before["items"]);
    let diagnostic = raw["file"]["diagnostics"]
        .as_array()
        .and_then(|d| d.iter().find(|d| d["code"] == "config_unknown_key"))
        .unwrap_or_else(|| panic!("config_unknown_key diagnostic missing: {raw}"));
    let message = diagnostic["message"].as_str().unwrap_or("missing message");
    let key = diagnostic["key"].as_str().unwrap_or("extra");
    let expected = format!("{message} (config_unknown_key)");
    for lang in ["ko", "en"] {
        language(&mut browser, lang);
        assert!(browser.text().contains(&expected));
        assert!(browser.text().contains(key));
        assert_rows(&mut browser, &raw, lang);
        blocked(&mut browser);
    }
    // Direct edit entry must also show the raw message/code and remove inputs/apply actions.
    browser.navigate(&server.url("/#/settings/edit"));
    ready(&mut browser);
    blocked(&mut browser);
    for lang in ["ko", "en"] {
        language(&mut browser, lang);
        assert!(browser.text().contains(if lang == "ko" {
            "설정 파일의 검증에 실패해"
        } else {
            "The configuration file failed validation."
        }));
        assert!(browser.text().contains(&expected));
        assert!(browser.text().contains(key));
        blocked(&mut browser);
    }
    must(fs::remove_file(server.config_path()));
    for route in ["/#/settings", "/#/settings/edit"] {
        browser.navigate(&server.url(route));
        ready(&mut browser);
        let raw = server.query(&admin, CONFIG);
        assert_eq!(raw["file"]["state"], "unreadable");
        assert_eq!(raw["items"], before["items"]);
        for lang in ["ko", "en"] {
            language(&mut browser, lang);
            let text = browser.text();
            assert!(text.contains(if lang == "ko" {
                "설정 파일을 읽지 못해"
            } else {
                "The configuration file could not be read."
            }));
            assert!(!text.contains(if lang == "ko" {
                "설정 파일의 검증에 실패해"
            } else {
                "The configuration file failed validation."
            }));
            if route == "/#/settings" {
                assert_rows(&mut browser, &raw, lang);
            }
            blocked(&mut browser);
        }
    }
    assert_eq!(server.proxy.count("PATCH", CONFIG), 0);
}

// DASH-033
pub fn settings_without_reload_history_show_no_history() {
    let server = Server::start_with_config("dash-033", None, true);
    let raw = server.query(&server.login(), CONFIG);
    assert!(raw["last_reload"].is_null());
    let mut browser = Browser::start("dash-033", &["ko-KR"]);
    login_alice(&mut browser, &server);
    open_settings(&mut browser, &server);
    for (lang, message) in [
        ("ko", "이 프로세스의 재로드 이력이 없습니다."),
        ("en", "This process has no reload history."),
    ] {
        language(&mut browser, lang);
        assert_eq!(browser.eval("[...document.querySelectorAll('.card')].find(c => c.textContent.includes(document.documentElement.lang === 'ko' ? '마지막 재로드' : 'Last reload'))?.querySelector('.config-body')?.textContent"), json!(message));
        assert_eq!(
            browser.eval("document.querySelectorAll('.config-reload-row').length"),
            json!(0),
            "no fabricated failure/result or zero timestamp"
        );
    }
}

// DASH-034
pub fn disabled_edit_direct_route_keeps_view_without_inputs() {
    let server = Server::start_with_config("dash-034", None, false);
    let raw = server.query(&server.login(), CONFIG);
    assert_eq!(raw["edit_enabled"], false);
    let mut browser = Browser::start("dash-034", &["ko-KR"]);
    login_alice(&mut browser, &server);
    for route in ["/#/settings", "/#/settings/edit"] {
        browser.navigate(&server.url(route));
        ready(&mut browser);
        for lang in ["ko", "en"] {
            language(&mut browser, lang);
            if route == "/#/settings" {
                assert_rows(&mut browser, &raw, lang);
            }
            let text = browser.text();
            assert!(text.contains(if lang == "ko" {"편집 기능이 꺼져 있습니다. 설정 파일에서 management.config_edit = true로 바꾸고 서버를 재시작하십시오."} else {"Editing is disabled. Set management.config_edit = true in the configuration file and restart the server."}));
            blocked(&mut browser);
        }
    }
    assert_eq!(server.proxy.count("PATCH", CONFIG), 0);
}

// DASH-048 and MGT-042
pub fn config_query_failures_keep_stale_values_and_never_show_empty_success() {
    let server = Server::start_with_config("dash-048", None, true);
    let admin = server.login();
    let before = server.query(&admin, CONFIG);
    let mut browser = Browser::start("dash-048", &["ko-KR"]);
    login_alice(&mut browser, &server);
    for route in ["/#/settings", "/#/settings/edit"] {
        // Hold the first real response to observe loading separately from failure.
        let hold_injections = server.proxy.injected_count("GET", CONFIG);
        let released = Arc::new(AtomicBool::new(false));
        server
            .proxy
            .once("GET", CONFIG, Fault::Hold(Arc::clone(&released)));
        browser.navigate(&server.url(route));
        browser.wait("document.body.innerText.includes('조회 중')");
        wait_for("initial config response held", LIMIT, || {
            server
                .proxy
                .traffic()
                .iter()
                .any(|r| r.path == CONFIG && r.stage == "response_held")
        });
        assert_eq!(
            server.proxy.injected_count("GET", CONFIG),
            hold_injections + 1
        );
        assert_eq!(
            browser.eval("document.querySelectorAll('[data-config-key]').length"),
            json!(0)
        );
        blocked(&mut browser);
        released.store(true, Ordering::Release);
        ready(&mut browser);
        let first_time = meta(&mut browser);
        if route == "/#/settings" {
            assert_rows(&mut browser, &before, "ko");
        }
        let fields = expected_inputs(&before);
        if route == "/#/settings/edit" {
            assert_eq!(browser.eval("[...document.querySelectorAll('input[data-config-key],textarea[data-config-key]')].map(n => [n.dataset.configKey,n.type === 'checkbox' ? n.checked : n.value])"), fields, "successful edit baseline must match real GET values");
        }
        let injections = server.proxy.injected_count("GET", CONFIG);
        server
            .proxy
            .once("GET", CONFIG, Fault::Error500("internal"));
        refresh(&mut browser);
        assert!(browser.text().contains("조회 실패"));
        assert!(browser.text().contains("이전 데이터입니다."));
        assert_eq!(meta(&mut browser), first_time);
        let timestamp = first_time
            .as_str()
            .unwrap_or("")
            .trim_start_matches("조회 시각: ");
        assert!(
            !timestamp.is_empty()
                && browser
                    .text()
                    .contains(&format!("마지막 성공 시각 {timestamp}"))
        );
        if route == "/#/settings" {
            assert_rows(&mut browser, &before, "ko");
        } else {
            assert_eq!(browser.eval("[...document.querySelectorAll('input[data-config-key],textarea[data-config-key]')].map(n => [n.dataset.configKey,n.type === 'checkbox' ? n.checked : n.value])"), fields);
        }
        assert_eq!(server.proxy.injected_count("GET", CONFIG), injections + 1);
        assert_eq!(browser.eval("document.querySelector('#settings-review')?.disabled ?? document.querySelector('#settings-edit')?.disabled"), json!(true));
        assert!(!browser.text().contains("개를 적용했습니다"));
        // A new mount with no previous success must show failed, not empty values/stale success.
        browser.navigate(&server.url("/"));
        browser.wait("document.querySelector('#operations-refresh')?.disabled === false");
        server.proxy.once("GET", CONFIG, Fault::Close);
        browser.navigate(&server.url(route));
        ready(&mut browser);
        assert!(
            browser
                .text()
                .contains("조회에 실패했습니다. 새로고침으로 다시 시도하십시오."),
            "first GET failure must not retain stale state: {}",
            browser.text()
        );
        assert!(!browser.text().contains("이전 데이터입니다."));
        assert!(!browser.text().contains("개를 적용했습니다"));
        assert_eq!(
            browser.eval("document.querySelectorAll('[data-config-key]').length"),
            json!(0)
        );
        blocked(&mut browser);
        assert_eq!(server.proxy.injected_count("GET", CONFIG), injections + 2);
        assert_eq!(server.query(&admin, CONFIG)["items"], before["items"]);
    }
    assert_eq!(server.proxy.count("PATCH", CONFIG), 0);
}

// DASH-049
pub fn explicit_settings_refresh_after_repair_updates_values_revision_and_time() {
    let server = Server::start_custom(
        "dash-049",
        None,
        true,
        "[reload]\npoll_secs = 5\nsettle_secs = 5\n",
    );
    let admin = server.login();
    let original = server.config_text();
    let before = server.query(&admin, CONFIG);
    let mut browser = Browser::start("dash-049", &["ko-KR"]);
    login_alice(&mut browser, &server);
    open_settings(&mut browser, &server);
    server.config_write("[broken\n");
    refresh(&mut browser);
    assert!(browser.text().contains("설정 파일의 검증에 실패해"));
    blocked(&mut browser);
    let failed_time = assert_query_time(&mut browser, &server);
    let injections = server.proxy.injected_count("GET", CONFIG);
    server
        .proxy
        .once("GET", CONFIG, Fault::Error500("internal"));
    refresh(&mut browser);
    assert!(browser.text().contains("조회 실패"));
    assert_eq!(meta(&mut browser), failed_time);
    assert_eq!(server.proxy.injected_count("GET", CONFIG), injections + 1);
    // The UI intentionally displays seconds, so cross one display tick before success.
    thread::sleep(Duration::from_millis(1100));
    // Repair the file, then make a real administrator change before the explicit GET.
    server.config_write(&original);
    let pending = server.query(&admin, CONFIG);
    let session = server.query(&admin, "/api/v1/session");
    let body = json!({"revision":pending["revision"],"changes":{"limits.waiting":12}}).to_string();
    let reply = server.request(
        "PATCH",
        CONFIG,
        &[
            ("Cookie", &admin.cookie),
            (
                "X-CSRF-Token",
                session["csrf_token"].as_str().unwrap_or("missing"),
            ),
            ("Content-Type", "application/json"),
        ],
        body.as_bytes(),
    );
    assert_eq!(reply.status, 200, "repair apply: {}", reply.body);
    let repaired = server.query(&admin, CONFIG);
    assert_ne!(repaired["revision"], before["revision"]);
    assert_ne!(repaired["items"], before["items"]);
    refresh(&mut browser);
    assert!(!browser.text().contains("조회 실패"));
    assert_rows(&mut browser, &repaired, "ko");
    assert_ne!(assert_query_time(&mut browser, &server), failed_time);
    assert_eq!(
        browser.eval("document.querySelector('#settings-edit')?.disabled"),
        json!(false)
    );
    // A held old view response arrives only after a newer edit mount has loaded.
    let released = Arc::new(AtomicBool::new(false));
    server
        .proxy
        .once("GET", CONFIG, Fault::Hold(Arc::clone(&released)));
    browser.click("#settings-refresh");
    wait_for("old config response held", LIMIT, || {
        server
            .proxy
            .traffic()
            .iter()
            .any(|r| r.path == CONFIG && r.stage == "response_held")
    });
    server.config_write(server.config_text() + "\n# new revision after held GET\n");
    browser.eval("location.hash = '#/settings/edit'");
    ready(&mut browser);
    let latest = server.query(&admin, CONFIG);
    assert_eq!(
        browser.eval("document.querySelector('#settings-review')?.disabled"),
        json!(false)
    );
    assert_eq!(
        browser.eval("document.querySelector('[data-config-key=\"limits.waiting\"]')?.value"),
        json!("12")
    );
    assert_ne!(latest["revision"], repaired["revision"]);
    let latest_time = assert_query_time(&mut browser, &server);
    let latest_fields = browser.eval(
        "[...document.querySelectorAll('input[data-config-key],textarea[data-config-key]')].map(n => [n.dataset.configKey,n.type === 'checkbox' ? n.checked : n.value])",
    );
    released.store(true, Ordering::Release);
    wait_for("held response completed", LIMIT, || {
        server
            .proxy
            .traffic()
            .iter()
            .filter(|r| r.path == CONFIG && r.injected)
            .all(|r| r.completed.is_some())
    });
    assert_eq!(server.proxy.injected_count("GET", CONFIG), injections + 2);
    assert_eq!(meta(&mut browser), latest_time);
    assert_eq!(browser.eval("location.hash"), json!("#/settings/edit"));
    assert_eq!(browser.eval("[...document.querySelectorAll('input[data-config-key],textarea[data-config-key]')].map(n => [n.dataset.configKey,n.type === 'checkbox' ? n.checked : n.value])"), latest_fields);
    server
        .proxy
        .once("GET", CONFIG, Fault::Error500("internal"));
    refresh(&mut browser);
    assert!(browser.text().contains("조회 실패"));
    assert_eq!(meta(&mut browser), latest_time);
    assert_eq!(
        browser.eval("document.querySelector('#settings-review')?.disabled"),
        json!(true)
    );
    server.config_write("[broken\n");
    refresh(&mut browser);
    blocked(&mut browser);
    let invalid_edit_time = assert_query_time(&mut browser, &server);
    server.config_write(format!(
        "{}\n# repaired for edit\n",
        original.replace("waiting = 8", "waiting = 12")
    ));
    thread::sleep(Duration::from_millis(1100));
    refresh(&mut browser);
    let latest = server.query(&admin, CONFIG);
    assert_ne!(assert_query_time(&mut browser, &server), invalid_edit_time);
    assert_eq!(
        browser.eval("document.querySelector('#settings-review')?.disabled"),
        json!(false)
    );
    assert_eq!(
        browser.eval("document.querySelector('[data-config-key=\"limits.waiting\"]')?.value"),
        json!("12")
    );
    assert_eq!(server.proxy.injected_count("GET", CONFIG), injections + 3);
    // The browser's next real PATCH must use the newly fetched revision.
    super::settings::set_field_value(&mut browser, "limits.waiting", "13");
    browser.click("#settings-review");
    browser.click("dialog[open] .button-primary");
    browser.wait("location.hash === '#/settings' && document.querySelector('#settings-edit')?.disabled === false");
    let patch = server
        .proxy
        .traffic()
        .into_iter()
        .find(|r| r.method == "PATCH" && r.path == CONFIG)
        .unwrap_or_else(|| panic!("browser PATCH missing"));
    let sent: Value = must(serde_json::from_str(&patch.body));
    assert_eq!(sent["revision"], latest["revision"]);
    assert_eq!(sent["changes"], json!({"limits.waiting":13}));
    assert_rows(&mut browser, &server.query(&admin, CONFIG), "ko");
    // A byte-identical PATCH is deliberately a no-op (§1.14 step 8).
    // Pending disk values must still reach the running snapshot via file watch.
    server.config_write(server.config_text().replace("waiting = 13", "waiting = 14"));
    let pending = server.query(&admin, CONFIG);
    let body = json!({"revision":pending["revision"],"changes":{"limits.waiting":14}}).to_string();
    let reply = server.request(
        "PATCH",
        CONFIG,
        &[
            ("Cookie", &admin.cookie),
            (
                "X-CSRF-Token",
                session["csrf_token"].as_str().unwrap_or("missing"),
            ),
            ("Content-Type", "application/json"),
        ],
        body.as_bytes(),
    );
    assert_eq!(reply.status, 200);
    let applied: Value = must(serde_json::from_str(&reply.body));
    assert_eq!(applied["applied"], json!([]));
    wait_for(
        "pending no-op file watch applies within poll plus settle",
        Duration::from_secs(11),
        || {
            let raw = server.query(&admin, CONFIG);
            raw["items"].as_array().is_some_and(|items| {
                items
                    .iter()
                    .any(|i| i["key"] == "limits.waiting" && i["value"] == 14)
            }) && raw["last_reload"]["trigger"] == "file_watch"
        },
    );
}
