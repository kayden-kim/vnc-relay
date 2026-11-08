//! Real-process recovery evidence for DASH-041~047, DASH-050~051, DASH-053,
//! MGT-040/043 and TODO 6.7: real competing writes, filesystem failures and relay traffic.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::live::{Admin, Browser, Fault, LIMIT, Server, must, wait_for};
use super::settings::{
    PermissionGuard, get_field_value, login_alice, open_settings, open_settings_edit, script_str,
    set_field_value,
};

const CONFIG: &str = "/api/v1/config";
const WAITING: &str = "limits.waiting";

fn submit(browser: &mut Browser) {
    browser.click("#settings-review");
    browser.wait("document.querySelector('dialog.config-dialog[open]') !== null");
    browser.click("dialog.config-dialog .dialog-actions .button-primary");
}

fn assert_js(browser: &mut Browser, expression: &str) {
    assert_eq!(browser.eval(expression), json!(true), "{expression}");
}

fn language(browser: &mut Browser, lang: &str) {
    browser.eval(&format!(
        "(()=>{{const e=document.querySelector('select.language-select-control');e.value={};e.dispatchEvent(new Event('change',{{bubbles:true}}));}})()",
        script_str(lang)
    ));
    browser.wait(&format!(
        "document.documentElement.lang === {}",
        script_str(lang)
    ));
}

fn item_value(config: &Value, key: &str) -> Value {
    config["items"]
        .as_array()
        .and_then(|items| items.iter().find(|item| item["key"] == key))
        .unwrap_or_else(|| panic!("missing config key {key}"))["value"]
        .clone()
}

fn assert_patch(server: &Server, status: u16, code: &str) -> Value {
    let traffic = server.proxy.traffic();
    let patch = traffic
        .iter()
        .rev()
        .find(|item| item.method == "PATCH" && item.path == CONFIG)
        .unwrap_or_else(|| panic!("PATCH missing"));
    assert_eq!(patch.status, Some(status), "{}", patch.response_body);
    let body: Value = must(serde_json::from_str(&patch.response_body));
    if !code.is_empty() {
        assert_eq!(body["error"]["code"], code);
    }
    body
}

fn no_replay(server: &Server, expected: usize) {
    thread::sleep(Duration::from_secs(3));
    assert_eq!(
        server.proxy.count("PATCH", CONFIG),
        expected,
        "no automatic PATCH"
    );
}

fn oracle_save(server: &Server, admin: &Admin, value: u64) -> Value {
    let config = server.query(admin, CONFIG);
    let session = server.query(admin, "/api/v1/session");
    let csrf = session["csrf_token"].as_str().unwrap_or("");
    let reply = server.request(
        "PATCH",
        CONFIG,
        &[
            ("Cookie", &admin.cookie),
            ("X-CSRF-Token", csrf),
            ("Content-Type", "application/json"),
        ],
        json!({"revision":config["revision"],"changes":{WAITING:value}})
            .to_string()
            .as_bytes(),
    );
    assert_eq!(reply.status, 200, "{}", reply.body);
    server.query(admin, CONFIG)
}

fn assert_view(browser: &mut Browser, server: &Server, config: &Value) {
    let expected: Vec<Value> = config["items"]
        .as_array()
        .unwrap_or_else(|| panic!("items missing"))
        .iter()
        .map(|item| json!({"key":item["key"], "value":item["value"]}))
        .collect();
    browser.wait("document.querySelector('.config-row') !== null");
    let traffic = server.proxy.traffic();
    let get = traffic
        .iter()
        .rev()
        .find(|item| item.method == "GET" && item.path == CONFIG && item.status == Some(200))
        .unwrap_or_else(|| panic!("successful view GET missing"));
    let received: Value = must(serde_json::from_str(&get.response_body));
    assert_eq!(
        received["revision"], config["revision"],
        "view fetched the actual current revision"
    );
    assert_eq!(received["file"], config["file"]);
    assert_eq!(received["last_reload"], config["last_reload"]);
    let actual = browser.eval("Array.from(document.querySelectorAll('.config-row')).map(e=>({key:e.dataset.configKey,value:e.querySelector('.config-value').innerText}))");
    let rows = actual.as_array().unwrap_or_else(|| panic!("rows missing"));
    assert_eq!(rows.len(), expected.len());
    for (row, item) in rows.iter().zip(expected.iter()) {
        assert_eq!(row["key"], item["key"], "GET order");
        let value = &item["value"];
        let text = row["value"].as_str().unwrap_or("");
        if let Some(values) = value.as_array() {
            if !values.is_empty() {
                assert_eq!(
                    text,
                    values
                        .iter()
                        .map(|v| v.as_str().unwrap_or(""))
                        .collect::<Vec<_>>()
                        .join("\n")
                );
            }
        } else if !value.is_null() {
            assert_eq!(
                text,
                value
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| value.to_string())
            );
        }
    }
    let state = config["file"]["state"].as_str().unwrap_or("");
    let text = browser.text();
    let state_copy = match state {
        "in_effect" => "설정 파일과 실행 중인 설정이 같습니다.",
        "pending" => "설정 파일에 재시작 없이 적용할 차이가 있습니다.",
        "restart_required" => "설정 파일에 재시작이 필요한 차이가 있어",
        other => panic!("unexpected view state {other}"),
    };
    assert!(text.contains(state_copy), "{text}");
    if config["last_reload"].is_null() {
        assert!(text.contains("이 프로세스의 재로드 이력이 없습니다."));
    } else {
        let reload = &config["last_reload"];
        let expected_time = browser.eval(&format!(
            "(()=>{{const d=new Date({});const p=n=>String(n).padStart(2,'0');const z=new Intl.DateTimeFormat('en-US',{{timeZoneName:'short'}}).formatToParts(d).find(p=>p.type==='timeZoneName')?.value;return `${{d.getFullYear()}}-${{p(d.getMonth()+1)}}-${{p(d.getDate())}} ${{p(d.getHours())}}:${{p(d.getMinutes())}}:${{p(d.getSeconds())}}${{z?' '+z:''}}`;}})()", reload["at"]));
        let actual_reload = browser.eval("document.querySelector('.config-reload').innerText");
        let actual_reload = actual_reload.as_str().unwrap_or("");
        assert!(
            actual_reload.contains(expected_time.as_str().unwrap_or("missing")),
            "{actual_reload}"
        );
        for key in reload["config_keys"]
            .as_array()
            .unwrap_or_else(|| panic!("reload keys"))
        {
            assert!(actual_reload.contains(key.as_str().unwrap_or("missing")));
        }
        let result = match reload["result"].as_str().unwrap_or("") {
            "applied" => "적용",
            "rejected" => "거절",
            "restart_required" => "재시작 필요",
            other => panic!("reload result {other}"),
        };
        assert!(actual_reload.contains(result), "{actual_reload}");
        assert!(actual_reload.contains("설정 편집"), "{actual_reload}");
    }
}

// DASH-041: both an actual second administrator save and a byte-only comment edit.
pub fn settings_conflict_keeps_input_and_never_resends_automatically() {
    let server = Server::start_with_config("dash-041", None, true);
    let admin = server.login();
    let mut browser = Browser::start("dash-041", &["ko-KR"]);
    login_alice(&mut browser, &server);
    open_settings(&mut browser, &server);
    for external_comment in [false, true] {
        open_settings_edit(&mut browser, &server);
        set_field_value(&mut browser, WAITING, "11");
        if external_comment {
            server.config_write(format!(
                "{}\n# comment-only competing write\n",
                server.config_text()
            ));
        } else {
            let saved = oracle_save(&server, &admin, 9);
            assert_eq!(item_value(&saved, WAITING), 9);
        }
        submit(&mut browser);
        browser.wait("document.querySelector('#settings-reload') !== null");
        assert_patch(&server, 409, "config_conflict");
        assert_eq!(get_field_value(&mut browser, WAITING), "11");
        let text = browser.text();
        assert!(text.contains("그사이 설정 파일이 바뀌었습니다. 다시 불러오면 입력 내용을 버리고 현재 설정으로 바꿉니다."), "{text}");
        assert_js(
            &mut browser,
            "document.querySelector('#settings-review').disabled",
        );
        no_replay(&server, usize::from(external_comment) + 1);
        browser.click("#settings-cancel");
        browser.wait("document.querySelector('#settings-edit') !== null && !document.querySelector('#settings-edit').disabled");
    }
}

// DASH-042 / MGT-043: fresh revision is asserted in the next explicit PATCH.
pub fn config_conflict_reload_discards_input_and_uses_fresh_revision() {
    let server = Server::start_with_config("dash-042", None, true);
    let admin = server.login();
    let mut browser = Browser::start("dash-042", &["ko-KR"]);
    login_alice(&mut browser, &server);
    open_settings(&mut browser, &server);
    open_settings_edit(&mut browser, &server);
    let old = server.query(&admin, CONFIG);
    set_field_value(&mut browser, WAITING, "11");
    let fresh = oracle_save(&server, &admin, 10);
    assert_ne!(old["revision"], fresh["revision"]);
    submit(&mut browser);
    browser.wait("document.querySelector('#settings-reload') !== null");
    assert_patch(&server, 409, "config_conflict");
    browser.click("#settings-reload");
    browser.wait("document.querySelector('#settings-reload') === null && !document.querySelector('#settings-review').disabled");
    assert_eq!(
        get_field_value(&mut browser, WAITING),
        item_value(&fresh, WAITING).to_string()
    );
    assert_eq!(fresh["file"]["state"], "in_effect");
    // F1(P1): Right after reloading, change value on the same screen and submit.
    // Assert PATCH body revision matches fresh["revision"] from the reloaded GET and succeeds with 200.
    set_field_value(&mut browser, WAITING, "12");
    submit(&mut browser);
    browser
        .wait("location.hash === '#/settings' && document.querySelector('.config-row') !== null");
    let applied = assert_patch(&server, 200, "");
    assert_eq!(applied["applied"], json!([WAITING]));
    let traffic = server.proxy.traffic();
    let request: Value = must(serde_json::from_str(
        &traffic
            .iter()
            .rev()
            .find(|t| t.method == "PATCH")
            .unwrap_or_else(|| panic!("PATCH"))
            .body,
    ));
    assert_eq!(
        request["revision"], fresh["revision"],
        "PATCH must use the reloaded revision"
    );
    assert_eq!(request["changes"], json!({WAITING:12}));
    assert!(browser.text().contains("설정 1개를 적용했습니다"));
    assert_eq!(item_value(&server.query(&admin, CONFIG), WAITING), 12);
    no_replay(&server, 2);

    // Then test that a late old response held during GET does not overwrite new values.
    open_settings_edit(&mut browser, &server);
    let release = Arc::new(AtomicBool::new(false));
    let injected = server.proxy.injected_count("GET", CONFIG);
    server
        .proxy
        .once("GET", CONFIG, Fault::Hold(Arc::clone(&release)));
    browser.click("#settings-refresh");
    wait_for("old GET held", LIMIT, || {
        server
            .proxy
            .traffic()
            .iter()
            .any(|t| t.path == CONFIG && t.stage == "response_held")
    });
    browser.click("#settings-view");
    browser.wait("document.querySelector('#settings-edit') !== null && !document.querySelector('#settings-edit').disabled");
    let newest = oracle_save(&server, &admin, 9);
    assert_eq!(item_value(&newest, WAITING), 9);
    open_settings_edit(&mut browser, &server);
    assert_eq!(get_field_value(&mut browser, WAITING), "9");
    release.store(true, Ordering::Release);
    wait_for("old GET released", LIMIT, || {
        !server
            .proxy
            .traffic()
            .iter()
            .any(|t| t.stage == "response_held")
    });
    thread::sleep(Duration::from_millis(100));
    assert_eq!(
        get_field_value(&mut browser, WAITING),
        "9",
        "late old response must not replace fresh values"
    );
    assert_eq!(server.proxy.injected_count("GET", CONFIG), injected + 1);
    no_replay(&server, 2);
}

const DIAGNOSTICS: [(&str, &str, &str); 9] = [
    (
        "config_missing_key",
        "필수 설정 값이 없습니다.",
        "A required setting value is missing.",
    ),
    (
        "config_type_mismatch",
        "설정 값의 자료형이 맞지 않습니다.",
        "The setting value has an invalid type.",
    ),
    (
        "config_value_out_of_range",
        "설정 값이 허용 범위를 벗어났습니다.",
        "The setting value is outside its allowed range.",
    ),
    (
        "config_relation_violated",
        "설정 키 사이의 관계를 만족하지 않습니다.",
        "The relationship between setting keys is not valid.",
    ),
    (
        "config_ip_item_invalid",
        "IP·CIDR·범위 항목의 형식이 올바르지 않습니다.",
        "The IP, CIDR, or range item has an invalid format.",
    ),
    (
        "config_ip_range_reversed",
        "IP 범위의 시작 주소는 끝 주소 이하여야 합니다.",
        "The start address of an IP range must not exceed its end address.",
    ),
    (
        "config_ip_range_family_mixed",
        "IP 범위의 두 주소는 모두 IPv4이거나 모두 IPv6여야 합니다.",
        "Both addresses in an IP range must be IPv4 or both must be IPv6.",
    ),
    (
        "config_origin_invalid",
        "Origin 항목의 형식이 올바르지 않습니다.",
        "The Origin item has an invalid format.",
    ),
    (
        "config_target_item_invalid",
        "Mode I 대상 항목의 형식이 올바르지 않습니다.",
        "The Mode I target item has an invalid format.",
    ),
];

fn blocked(browser: &mut Browser) {
    assert_js(
        browser,
        "document.querySelectorAll('input[data-config-key],textarea[data-config-key],#settings-review,dialog[open]').length === 0",
    );
    assert_js(
        browser,
        "document.querySelector('#settings-view') !== null && document.querySelector('#settings-reload') !== null",
    );
}
fn rejection_replay(server: &Server, admin: &Admin, status: u16, code: &str) -> Fault {
    let raw = server.query(admin, CONFIG);
    let session = server.query(admin, "/api/v1/session");
    let reply = server.request(
        "PATCH",
        CONFIG,
        &[
            ("Cookie", &admin.cookie),
            ("X-CSRF-Token", session["csrf_token"].as_str().unwrap_or("")),
            ("Content-Type", "application/json"),
        ],
        json!({"revision":raw["revision"],"changes":{WAITING:10}})
            .to_string()
            .as_bytes(),
    );
    assert_eq!(reply.status, status, "{}", reply.body);
    let body: Value = must(serde_json::from_str(&reply.body));
    assert_eq!(body["error"]["code"], code);
    if code == "config_restart_pending" {
        assert_eq!(body["error"]["config_keys"], raw["file"]["config_keys"]);
        assert!(
            !raw["file"]["config_keys"]
                .as_array()
                .unwrap_or_else(|| panic!("restart keys"))
                .is_empty()
        );
    } else if code == "config_file_invalid" {
        assert_eq!(body["error"]["diagnostics"], raw["file"]["diagnostics"]);
        assert!(
            !raw["file"]["diagnostics"]
                .as_array()
                .unwrap_or_else(|| panic!("file diagnostics"))
                .is_empty()
        );
    }
    Fault::ReplayResponse(reply.status, reply.headers, reply.body)
}

// DASH-043: replay unmodified real-server rejections without forwarding the browser PATCH.
// API 1.14 step 4 rejects stale revisions before step 5 validates the current file;
// consequently a normal old-GET client cannot itself receive these two step-5 rejections.
pub fn disabled_restart_pending_and_invalid_patch_results_remove_edit_actions() {
    let server = Server::start_with_config("dash-043", None, true);
    let admin = server.login();
    let original = server.config_text();
    let mut browser = Browser::start("dash-043", &["ko-KR"]);
    login_alice(&mut browser, &server);
    open_settings(&mut browser, &server);
    open_settings_edit(&mut browser, &server);
    let disabled_server = Server::start_with_config("dash-043-disabled-oracle", None, false);
    let disabled_admin = disabled_server.login();
    let disabled = rejection_replay(
        &disabled_server,
        &disabled_admin,
        403,
        "config_edit_disabled",
    );
    let injected = server.proxy.injected_count("PATCH", CONFIG);
    server.proxy.once("PATCH", CONFIG, disabled);
    set_field_value(&mut browser, WAITING, "10");
    submit(&mut browser);
    browser.wait("document.querySelector('#settings-review') === null");
    assert_patch(&server, 403, "config_edit_disabled");
    assert_eq!(server.proxy.injected_count("PATCH", CONFIG), injected + 1);
    for (lang, copy) in [
        ("ko", "편집 기능이 꺼져 있습니다."),
        ("en", "Editing is disabled."),
    ] {
        language(&mut browser, lang);
        blocked(&mut browser);
        let text = browser.text();
        assert!(
            text.contains(copy) && text.contains("management.config_edit = true"),
            "{text}"
        );
        assert!(text.contains(if lang == "ko" {
            "서버를 재시작하십시오"
        } else {
            "restart the server"
        }));
    }
    // Keep the real candidate on disk while its real rejection is replayed.
    let candidates = [
        original.replace("idle_secs = 300", "idle_secs = 301"),
        original.replacen("allow = [\"127.0.0.1\"]", "", 1),
        original.replacen("allow = [\"127.0.0.1\"]", "allow = 7", 1),
        original.replace("waiting = 8", "waiting = -1"),
        original.replace("idle_secs = 300", "idle_secs = 3601"),
        original.replacen("allow = [\"127.0.0.1\"]", "allow = [\"invalid-ip\"]", 1),
        original.replacen(
            "allow = [\"127.0.0.1\"]",
            "allow = [\"127.0.0.2-127.0.0.1\"]",
            1,
        ),
        original.replacen("allow = [\"127.0.0.1\"]", "allow = [\"127.0.0.1-::1\"]", 1),
        original.replace(
            "allowed_origins = [\"http://127.0.0.1\"]",
            "allowed_origins = [\"invalid-origin\"]",
        ),
        original.replace(
            "allow_targets = [\"127.0.0.1:1-65535\"]",
            "allow_targets = [\"invalid-target\"]",
        ),
        format!("{original}\n[unclosed"),
    ];
    for (index, candidate) in candidates.iter().enumerate() {
        server.config_write(&original);
        browser.click("#settings-view");
        browser.wait("document.querySelector('#settings-edit') !== null && !document.querySelector('#settings-edit').disabled");
        open_settings_edit(&mut browser, &server);
        set_field_value(&mut browser, WAITING, "10");
        server.config_write(candidate);
        let raw = server.query(&admin, CONFIG);
        let code = if index == 0 {
            "config_restart_pending"
        } else {
            "config_file_invalid"
        };
        let replay = rejection_replay(&server, &admin, 409, code);
        let injected = server.proxy.injected_count("PATCH", CONFIG);
        server.proxy.once("PATCH", CONFIG, replay);
        submit(&mut browser);
        browser.wait("document.querySelector('#settings-review') === null");
        let body = assert_patch(
            &server,
            409,
            if index == 0 {
                "config_restart_pending"
            } else {
                "config_file_invalid"
            },
        );
        assert_eq!(server.proxy.injected_count("PATCH", CONFIG), injected + 1);
        assert!(
            !server
                .proxy
                .traffic()
                .iter()
                .rev()
                .find(|t| t.method == "PATCH")
                .unwrap_or_else(|| panic!("PATCH"))
                .forwarded
        );
        if index > 0 && index <= 9 {
            assert!(
                raw["file"]["diagnostics"]
                    .as_array()
                    .unwrap_or_else(|| panic!("diagnostics"))
                    .iter()
                    .any(|d| d["code"] == DIAGNOSTICS[index - 1].0),
                "{raw}"
            );
        }
        for lang in ["ko", "en"] {
            language(&mut browser, lang);
            blocked(&mut browser);
            let text = browser.text();
            if index == 0 {
                assert_eq!(raw["file"]["state"], "restart_required");
                for key in body["error"]["config_keys"]
                    .as_array()
                    .unwrap_or_else(|| panic!("restart keys"))
                {
                    assert!(text.contains(key.as_str().unwrap_or("missing")), "{text}");
                }
                assert!(
                    text.contains(if lang == "ko" {
                        "재시작이 필요한 차이가 있어 편집할 수 없습니다"
                    } else {
                        "Editing is blocked. Update the file or restart the server"
                    }),
                    "{text}"
                );
            } else {
                assert!(
                    text.contains(if lang == "ko" {
                        "설정 파일의 검증에 실패해 편집할 수 없습니다"
                    } else {
                        "The configuration file failed validation. Editing is blocked."
                    }),
                    "{text}"
                );
                for diagnostic in raw["file"]["diagnostics"]
                    .as_array()
                    .unwrap_or_else(|| panic!("diagnostics"))
                {
                    assert!(text.contains(diagnostic["key"].as_str().unwrap_or("missing")));
                    let code = diagnostic["code"].as_str().unwrap_or("");
                    if let Some((_, ko, en)) = DIAGNOSTICS.iter().find(|entry| entry.0 == code) {
                        assert!(text.contains(if lang == "ko" { ko } else { en }), "{text}");
                        let message = diagnostic["message"].as_str().unwrap_or("");
                        if !message.is_empty() && message != *ko && message != *en {
                            assert!(
                                !text.contains(message),
                                "known code leaks server prose: {text}"
                            );
                        }
                    } else {
                        assert!(
                            text.contains(code)
                                && text
                                    .contains(diagnostic["message"].as_str().unwrap_or("missing")),
                            "{text}"
                        );
                    }
                }
            }
        }
    }
}

fn write_failure(server: &Server, browser: &mut Browser, admin: &Admin) -> PermissionGuard {
    let bytes = server.config_bytes();
    let before = server.query(admin, CONFIG);
    let guard = PermissionGuard::deny(server.dir());
    set_field_value(browser, WAITING, "14");
    submit(browser);
    browser.wait("document.body.innerText.includes('설정 파일을 쓰지 못했습니다')");
    assert_patch(server, 500, "config_write_failed");
    assert_eq!(get_field_value(browser, WAITING), "14");
    let text = browser.text();
    assert!(
        text.contains("파일과 실행 값은 바뀌지 않았습니다."),
        "{text}"
    );
    assert!(
        !text.contains("적용했습니다") && !text.contains("Applied "),
        "{text}"
    );
    assert_eq!(server.config_bytes(), bytes);
    let after = server.query(admin, CONFIG);
    assert_eq!(after["items"], before["items"]);
    assert_eq!(after["revision"], before["revision"]);
    assert_eq!(after["last_reload"], before["last_reload"]);
    no_replay(server, 1);
    guard
}

// DASH-044 / TODO 6.7: actual atomic-write permission failure on both operating systems.
pub fn config_write_failure_keeps_input_without_success_notice() {
    let server = Server::start_with_config("dash-044", None, true);
    let admin = server.login();
    let mut browser = Browser::start("dash-044", &["ko-KR"]);
    login_alice(&mut browser, &server);
    open_settings(&mut browser, &server);
    open_settings_edit(&mut browser, &server);
    let mut guard = write_failure(&server, &mut browser, &admin);
    guard.restore();
}

// DASH-045 / TODO 6.7: repair and retry exactly once with the retained draft.
pub fn explicit_settings_retry_after_write_repair_saves_once() {
    let server = Server::start_with_config("dash-045", None, true);
    let admin = server.login();
    let mut browser = Browser::start("dash-045", &["ko-KR"]);
    login_alice(&mut browser, &server);
    open_settings(&mut browser, &server);
    open_settings_edit(&mut browser, &server);
    let mut guard = write_failure(&server, &mut browser, &admin);
    guard.restore();
    assert_eq!(get_field_value(&mut browser, WAITING), "14");
    no_replay(&server, 1);
    submit(&mut browser);
    browser
        .wait("location.hash === '#/settings' && document.querySelector('.config-row') !== null");
    let applied = assert_patch(&server, 200, "");
    let count = applied["applied"]
        .as_array()
        .unwrap_or_else(|| panic!("applied missing"))
        .len();
    assert_eq!(count, 1);
    assert!(
        browser
            .text()
            .contains(&format!("설정 {count}개를 적용했습니다"))
    );
    let raw = server.query(&admin, CONFIG);
    assert_eq!(item_value(&raw, WAITING), 14);
    assert_view(&mut browser, &server, &raw);
    no_replay(&server, 2);
}

fn unknown(browser: &mut Browser) {
    browser.wait("document.body.innerText.includes('적용 결과를 확인할 수 없습니다')");
    assert_js(
        browser,
        "document.querySelector('#settings-review').disabled && document.querySelector('#settings-view') !== null",
    );
    let text = browser.text();
    assert!(
        text.contains(
            "요청을 자동으로 다시 보내지 않습니다. 설정 보기에서 현재 값을 확인하십시오."
        ),
        "{text}"
    );
    assert!(
        !text.contains("적용했습니다")
            && !text.contains("쓰지 못했습니다")
            && !text.contains("검증에 실패"),
        "must not guess success/failure: {text}"
    );
}

// DASH-046 / MGT-040: dropped committed response, timeout, and another server 5xx.
pub fn config_response_loss_and_timeout_require_get_before_retry() {
    let server = Server::start_with_config("dash-046", Some(2), true);
    let admin = server.login();
    let mut browser = Browser::start("dash-046", &["ko-KR"]);
    login_alice(&mut browser, &server);
    open_settings(&mut browser, &server);
    for (index, fault) in [
        Fault::DropResponse,
        Fault::Delay(Duration::from_secs(6)),
        Fault::Error500("internal"),
    ]
    .into_iter()
    .enumerate()
    {
        open_settings_edit(&mut browser, &server);
        let injected = server.proxy.injected_count("PATCH", CONFIG);
        server.proxy.once("PATCH", CONFIG, fault);
        set_field_value(&mut browser, WAITING, &(12 + index).to_string());
        let submitted = Instant::now();
        submit(&mut browser);
        unknown(&mut browser);
        assert_eq!(server.proxy.injected_count("PATCH", CONFIG), injected + 1);
        if index < 2 {
            assert_eq!(
                item_value(&server.query(&admin, CONFIG), WAITING),
                json!(12 + index),
                "upstream commit really occurred"
            );
        }
        if index == 1 {
            // The browser timer starts before the proxy reads the request.
            assert!(
                submitted.elapsed() >= Duration::from_secs(5),
                "query_timeout_secs must elapse"
            );
            assert_eq!(
                server.query(&admin, "/api/v1/session")["dashboard"]["query_timeout_secs"],
                5
            );
        }
        no_replay(&server, index + 1);
        browser.click("#settings-view");
        browser.wait("document.querySelector('.config-row') !== null");
        assert_view(&mut browser, &server, &server.query(&admin, CONFIG));
        no_replay(&server, index + 1);
    }
}

// DASH-047: GET is the oracle after response loss, including last_reload and file state.
pub fn unknown_edit_result_is_checked_with_get_without_patch_replay() {
    let server = Server::start_with_config("dash-047", None, true);
    let admin = server.login();
    let mut browser = Browser::start("dash-047", &["ko-KR"]);
    login_alice(&mut browser, &server);
    open_settings(&mut browser, &server);
    open_settings_edit(&mut browser, &server);
    server.proxy.once("PATCH", CONFIG, Fault::DropResponse);
    set_field_value(&mut browser, WAITING, "15");
    submit(&mut browser);
    unknown(&mut browser);
    assert_eq!(server.proxy.injected_count("PATCH", CONFIG), 1);
    let before = server.proxy.count("GET", CONFIG);
    browser.click("#settings-view");
    browser
        .wait("location.hash === '#/settings' && document.querySelector('.config-row') !== null");
    assert_eq!(server.proxy.count("GET", CONFIG), before + 1);
    let raw = server.query(&admin, CONFIG);
    assert_eq!(item_value(&raw, WAITING), 15);
    assert!(!raw["last_reload"].is_null());
    assert_view(&mut browser, &server, &raw);
    no_replay(&server, 1);
}

fn layout(browser: &mut Browser) {
    assert_js(
        browser,
        "document.documentElement.scrollWidth <= document.documentElement.clientWidth",
    );
    assert_js(
        browser,
        "Array.from(document.querySelectorAll('.config-data,.config-field-error,.config-diagnostic p')).every(e=>e.scrollWidth<=e.clientWidth+1 && getComputedStyle(e).textOverflow!=='ellipsis' && getComputedStyle(e).webkitLineClamp==='none')",
    );
    assert_js(
        browser,
        "(()=>{const scope=document.querySelector('dialog[open]')??document;const rows=[...scope.querySelectorAll('.config-field,.config-row,.config-change,.config-diagnostic')];return rows.every((a,i)=>rows.slice(i+1).every(b=>{if(a.contains(b)||b.contains(a))return true;const x=a.getBoundingClientRect(),y=b.getBoundingClientRect();return x.bottom<=y.top+1||y.bottom<=x.top+1||x.right<=y.left+1||y.right<=x.left+1}))})()",
    );
}

// DASH-050: real long lists, server diagnostic, all language/width combinations and keyboard dialog.
pub fn settings_screens_preserve_input_and_accessibility_in_both_languages_and_widths() {
    let origins: Vec<String> = (0..16)
        .map(|i| format!("https://{}.{}.example.com", "long-hostname".repeat(4), i))
        .collect();
    let origins_json = must(serde_json::to_string(&origins));
    let server = Server::start_with_config_transform("dash-050", None, true, |config, _| {
        config.replace("[\"http://127.0.0.1\"]", &origins_json)
    });
    let admin = server.login();
    let mut browser = Browser::start("dash-050", &["ko-KR"]);
    login_alice(&mut browser, &server);
    open_settings(&mut browser, &server);
    let raw = server.query(&admin, CONFIG);
    assert_eq!(
        item_value(&raw, "relay.ws_viewer.allowed_origins"),
        json!(origins)
    );
    for width in [1280, 390] {
        browser.set_width(width);
        for lang in ["ko", "en"] {
            language(&mut browser, lang);
            layout(&mut browser);
            assert_js(
                &mut browser,
                &format!(
                    "Array.from(document.querySelectorAll('[data-config-key=\"relay.ws_viewer.allowed_origins\"] .config-value-list div')).map(e=>e.textContent).join('\\n') === {}",
                    script_str(&origins.join("\n"))
                ),
            );
        }
    }
    open_settings_edit(&mut browser, &server);
    let draft = (0..24)
        .map(|i| format!("2001:db8:1234:5678:90ab:cdef:1234:{i:x}"))
        .collect::<Vec<_>>()
        .join("\n");
    let invalid_input = format!("{draft}\nnot-an-ip");
    for width in [1280, 390] {
        browser.set_width(width);
        for lang in ["ko", "en"] {
            language(&mut browser, lang);
            set_field_value(&mut browser, "relay.tcp_viewer.allow", &invalid_input);
            submit(&mut browser);
            browser.wait("document.querySelector('.config-field-error') !== null");
            assert_patch(&server, 400, "config_invalid");
            assert_js(
                &mut browser,
                "document.activeElement.dataset.configKey === 'relay.tcp_viewer.allow' && document.activeElement.value.slice(document.activeElement.selectionStart, document.activeElement.selectionEnd) === 'not-an-ip'",
            );
            assert_eq!(
                get_field_value(&mut browser, "relay.tcp_viewer.allow"),
                invalid_input,
            );
            let other_lang = if lang == "ko" { "en" } else { "ko" };
            language(&mut browser, other_lang);
            assert_eq!(
                get_field_value(&mut browser, "relay.tcp_viewer.allow"),
                invalid_input,
            );
            let text = browser.text();
            assert!(
                text.contains(if other_lang == "ko" {
                    "25줄: IP·CIDR·범위 항목의 형식이 올바르지 않습니다."
                } else {
                    "Line 25: The IP, CIDR, or range item has an invalid format."
                }),
                "{text}"
            );
            assert_js(
                &mut browser,
                "Array.from(document.querySelectorAll('.config-input')).every(e=>document.querySelector('label[for=\"'+e.id+'\"]') && e.getAttribute('aria-describedby').split(' ').every(id=>document.getElementById(id)))",
            );
            assert_js(
                &mut browser,
                "(()=>{const e=document.querySelector('[aria-invalid=true]');return !!e && e.getAttribute('aria-describedby').split(' ').some(id=>document.getElementById(id).querySelector('.config-field-error'))})()",
            );
            layout(&mut browser);
            language(&mut browser, lang);
        }
    }
    set_field_value(&mut browser, "relay.tcp_viewer.allow", &draft);
    for width in [1280, 390] {
        browser.set_width(width);
        for lang in ["ko", "en"] {
            language(&mut browser, lang);
            browser.eval("document.querySelector('#settings-review').focus()");
            assert_js(
                &mut browser,
                "document.activeElement.id === 'settings-review' && !document.activeElement.disabled",
            );
            browser.command("Input.dispatchKeyEvent", json!({"type":"keyDown","key":"Enter","code":"Enter","windowsVirtualKeyCode":13,"nativeVirtualKeyCode":13,"text":"\r","unmodifiedText":"\r"}));
            browser.command("Input.dispatchKeyEvent", json!({"type":"keyUp","key":"Enter","code":"Enter","windowsVirtualKeyCode":13,"nativeVirtualKeyCode":13}));
            browser.wait("document.querySelector('dialog.config-dialog[open]') !== null");
            assert_js(
                &mut browser,
                "document.querySelector('dialog[open] .dialog-actions button') === document.activeElement",
            );
            assert_js(
                &mut browser,
                "(()=>{const d=document.querySelector('dialog[open]');return ['aria-labelledby','aria-describedby'].every(a=>d.getAttribute(a)?.split(' ').every(id=>!!document.getElementById(id)))})()",
            );
            assert_js(
                &mut browser,
                "document.querySelector('dialog[open]').innerText.includes('2001:db8:1234:5678:90ab:cdef:1234:17')",
            );
            layout(&mut browser);
            for _ in 0..5 {
                browser.key("Tab");
                assert_js(
                    &mut browser,
                    "document.querySelector('dialog[open]').contains(document.activeElement)",
                );
            }
            for kind in ["keyDown", "keyUp"] {
                browser.command("Input.dispatchKeyEvent", json!({"type":kind,"key":"Tab","code":"Tab","windowsVirtualKeyCode":9,"nativeVirtualKeyCode":9,"modifiers":8}));
            }
            assert_js(
                &mut browser,
                "document.querySelector('dialog[open]').contains(document.activeElement)",
            );
            browser.key("Escape");
            browser.wait("document.querySelector('dialog[open]') === null");
            assert_js(
                &mut browser,
                "document.activeElement.id === 'settings-review'",
            );
        }
    }
    no_replay(&server, 4);
}

fn alice_cookie(server: &Server) -> String {
    server
        .proxy
        .traffic()
        .iter()
        .rev()
        .filter(|t| t.path == CONFIG)
        .find_map(|t| {
            t.headers.lines().find_map(|line| {
                line.split_once(':')
                    .filter(|(name, _)| name.eq_ignore_ascii_case("cookie"))
                    .map(|(_, value)| value.trim().to_owned())
            })
        })
        .unwrap_or_else(|| panic!("alice cookie missing"))
}

fn invalidate_alice(server: &Server) -> String {
    let cookie = alice_cookie(server);
    let info = server.request("GET", "/api/v1/session", &[("Cookie", &cookie)], b"");
    assert_eq!(info.status, 200);
    let info: Value = must(serde_json::from_str(&info.body));
    let reply = server.request(
        "DELETE",
        "/api/v1/session",
        &[
            ("Cookie", &cookie),
            ("X-CSRF-Token", info["csrf_token"].as_str().unwrap_or("")),
        ],
        b"",
    );
    assert_eq!(reply.status, 204);
    cookie
}

fn login_clean(browser: &mut Browser, expired: bool) {
    browser.wait(
        "location.pathname === '/login/' && document.querySelector('#login-submit') !== null",
    );
    assert_js(
        browser,
        "document.querySelectorAll('[data-config-key],.config-row,.config-input,dialog[open],#settings-review').length === 0",
    );
    assert_js(
        browser,
        "document.querySelector('#login-password').type === 'password' && document.querySelector('#login-password').value === '' && document.querySelector('#login-account').value === ''",
    );
    assert!(!browser.text().contains("limits.waiting"));
    if expired {
        assert!(
            browser
                .text()
                .contains("로그인이 필요합니다. 다시 로그인하십시오.")
        );
    }
}

// DASH-051: real session invalidation in view/edit and explicit logout during held GET/PATCH.
pub fn settings_expiry_and_logout_remove_protected_values_and_leave_relays_alive() {
    let server = Server::start_with_config("dash-051", None, true);
    let admin = server.login();
    let mut pair = server.pair(&admin, "II", "65051");
    let mut browser = Browser::start("dash-051", &["ko-KR"]);
    for edit in [false, true] {
        login_alice(&mut browser, &server);
        open_settings(&mut browser, &server);
        if edit {
            open_settings_edit(&mut browser, &server);
            set_field_value(&mut browser, WAITING, "13");
        }
        let cookie = invalidate_alice(&server);
        if edit {
            submit(&mut browser);
        } else {
            browser.click("#settings-refresh");
        }
        login_clean(&mut browser, true);
        assert_eq!(
            server
                .request("GET", "/api/v1/session", &[("Cookie", &cookie)], b"")
                .status,
            401
        );
        let count = server.proxy.count("GET", CONFIG);
        thread::sleep(Duration::from_millis(150));
        assert_eq!(server.proxy.count("GET", CONFIG), count);
        pair.transfer(37, 51);
    }
    for edit in [false, true] {
        login_alice(&mut browser, &server);
        open_settings(&mut browser, &server);
        if edit {
            open_settings_edit(&mut browser, &server);
            set_field_value(&mut browser, WAITING, "14");
        }
        let cookie = alice_cookie(&server);
        let release = Arc::new(AtomicBool::new(false));
        let method = if edit { "PATCH" } else { "GET" };

        let injected = server.proxy.injected_count(method, CONFIG);
        server
            .proxy
            .once(method, CONFIG, Fault::Hold(Arc::clone(&release)));
        if edit {
            submit(&mut browser);
        } else {
            browser.click("#settings-refresh");
        }
        wait_for("request held during logout", LIMIT, || {
            server
                .proxy
                .traffic()
                .iter()
                .any(|t| t.path == CONFIG && t.stage == "response_held")
        });
        let logout_injected = server.proxy.injected_count("DELETE", "/api/v1/session");
        if edit {
            server
                .proxy
                .once("DELETE", "/api/v1/session", Fault::DropResponse);
        }
        browser.click("#settings-logout");
        login_clean(&mut browser, false);
        if edit {
            assert!(browser.text().contains(
                "로그아웃이 끝났는지 확인하지 못했습니다. 이 화면의 관리 데이터는 숨겼습니다."
            ));
            assert_eq!(
                server.proxy.injected_count("DELETE", "/api/v1/session"),
                logout_injected + 1
            );
        } else {
            let text = browser.text();
            assert!(
                text.contains("로그인이 필요합니다. 다시 로그인하십시오.")
                    && !text.contains(
                        "로그아웃이 끝났는지 확인하지 못했습니다. 이 화면의 관리 데이터는 숨겼습니다."
                    ),
                "successful logout must show CPY-075 and not CPY-076: {text}"
            );
            assert_js(&mut browser, "location.search === ''");
        }
        wait_for("in-flight request canceled in CDP", LIMIT, || {
            let network = browser.network();
            let events = network
                .as_array()
                .unwrap_or_else(|| panic!("network events"));
            let held_req_id = events.iter().rev().find_map(|e| {
                if e["event"] == "Network.requestWillBeSent"
                    && e["method"] == method
                    && e["url"]
                        .as_str()
                        .is_some_and(|u| u.ends_with("/api/v1/config"))
                {
                    e["request"].as_str()
                } else {
                    None
                }
            });
            held_req_id.is_some_and(|req_id| {
                events.iter().any(|e| {
                    e["event"] == "Network.loadingFailed"
                        && e["request"] == req_id
                        && e["canceled"] == true
                })
            })
        });
        assert_eq!(server.proxy.injected_count(method, CONFIG), injected + 1);
        assert_eq!(
            server
                .request("GET", "/api/v1/session", &[("Cookie", &cookie)], b"")
                .status,
            401
        );
        release.store(true, Ordering::Release);
        wait_for("logout held response released", LIMIT, || {
            !server
                .proxy
                .traffic()
                .iter()
                .any(|t| t.stage == "response_held")
        });
        thread::sleep(Duration::from_millis(100));
        login_clean(&mut browser, false);
        let count = server.proxy.count(method, CONFIG);
        thread::sleep(Duration::from_millis(150));
        assert_eq!(server.proxy.count(method, CONFIG), count);
        pair.transfer(41, 63);
    }
}

// DASH-053: all three exits discard without browser/native confirmation or PATCH.
pub fn leaving_settings_edit_discards_input_without_confirmation_or_patch() {
    let server = Server::start_with_config("dash-053", None, true);
    let admin = server.login();
    let mut browser = Browser::start("dash-053", &["ko-KR"]);
    login_alice(&mut browser, &server);
    open_settings(&mut browser, &server);
    let original = item_value(&server.query(&admin, CONFIG), WAITING).to_string();
    for selector in [
        "#settings-cancel",
        "a.navbar-item[href='/']",
        "a.navbar-item[href='/#/settings']",
    ] {
        open_settings_edit(&mut browser, &server);
        set_field_value(&mut browser, WAITING, "15");
        browser.eval("globalThis.exitConfirmations=0;globalThis.confirm=()=>{globalThis.exitConfirmations++;return false}");
        browser.click(selector);
        if selector == "a.navbar-item[href='/']" {
            browser.wait("document.querySelector('#operations-refresh') !== null");
            open_settings(&mut browser, &server);
        } else {
            browser.wait("location.hash === '#/settings' && document.querySelector('#settings-edit') !== null && !document.querySelector('#settings-edit').disabled");
        }
        assert_js(
            &mut browser,
            "document.querySelector('dialog[open]') === null && !globalThis.exitConfirmations",
        );
        assert_eq!(server.proxy.count("PATCH", CONFIG), 0);
        open_settings_edit(&mut browser, &server);
        assert_eq!(
            get_field_value(&mut browser, WAITING),
            original,
            "draft discarded on {selector}"
        );
        browser.click("#settings-cancel");
        browser.wait("document.querySelector('#settings-edit') !== null && !document.querySelector('#settings-edit').disabled");
    }
    assert_eq!(server.proxy.count("PATCH", CONFIG), 0);
}
