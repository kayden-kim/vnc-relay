//! Real-process settings edit (SCR-005) live browser tests: DASH-035~040, DASH-052.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde_json::{Value, json};

use super::live::{Browser, Fault, LIMIT, Server, must, wait_for};
use super::settings::{
    PendingFile, get_field_value, login_alice, open_settings, open_settings_edit, script_str,
    set_field_value,
};

fn items(config: &Value) -> &[Value] {
    config["items"]
        .as_array()
        .unwrap_or_else(|| panic!("config items missing: {config}"))
}

fn item<'a>(config: &'a Value, key: &str) -> &'a Value {
    items(config)
        .iter()
        .find(|item| item["key"] == key)
        .unwrap_or_else(|| panic!("config item missing: {key}"))
}

fn form_values(browser: &mut Browser) -> Value {
    browser.eval(
        "Object.fromEntries([...document.querySelectorAll('.config-input')].map(n=>[n.dataset.configKey,n.value]))",
    )
}

fn review(browser: &mut Browser) {
    browser.click("#settings-review");
    browser.wait("document.querySelector('dialog.config-dialog[open]') !== null");
}

fn apply(browser: &mut Browser) {
    browser.click("dialog.config-dialog .dialog-actions .button-primary");
}

fn view_ready(browser: &mut Browser) {
    browser.wait(
        "location.hash === '#/settings' && document.querySelector('#settings-edit') && !document.querySelector('#settings-refresh').disabled",
    );
}

fn field_error(browser: &mut Browser, key: &str, message: &str) {
    assert_eq!(
        browser.eval(&format!(
            r#"(() => {{
                const input = document.querySelector('[data-config-key="' + {} + '"]');
                const errors = input.closest('.config-field').querySelector('.config-field-errors');
                return input.getAttribute('aria-invalid') === 'true' &&
                    input.getAttribute('aria-describedby').split(' ').includes(errors.id) &&
                    errors.textContent.includes({});
            }})()"#,
            script_str(key),
            script_str(message)
        )),
        json!(true),
        "{key} must be associated with its own error: {message}"
    );
}

fn focused(browser: &mut Browser, key: &str, selected: Option<&str>) {
    assert_eq!(
        browser.eval("document.activeElement?.dataset.configKey"),
        json!(key),
        "first invalid field must receive focus"
    );
    if let Some(selected) = selected {
        assert_eq!(
            browser.eval("document.activeElement.value.slice(document.activeElement.selectionStart, document.activeElement.selectionEnd)"),
            json!(selected),
            "the entire original erroneous line must be selected"
        );
    }
}

fn switch_language(browser: &mut Browser, lang: &str) {
    browser.eval(&format!(
        "(() => {{ const select = document.querySelector('select'); select.value = {}; select.dispatchEvent(new Event('change', {{bubbles:true}})); }})()",
        script_str(lang)
    ));
    browser.wait(&format!(
        "document.documentElement.lang === {}",
        script_str(lang)
    ));
}

fn patch_reply(server: &Server, count: usize) -> Value {
    wait_for("completed real config PATCH", LIMIT, || {
        server
            .proxy
            .traffic()
            .iter()
            .filter(|entry| entry.method == "PATCH" && entry.path == "/api/v1/config")
            .nth(count - 1)
            .is_some_and(|entry| entry.completed.is_some())
    });
    let traffic = server.proxy.traffic();
    let patch = traffic
        .iter()
        .filter(|entry| entry.method == "PATCH" && entry.path == "/api/v1/config")
        .nth(count - 1)
        .unwrap_or_else(|| panic!("PATCH {count} missing"));
    must(serde_json::from_str(&patch.response_body))
}

// DASH-035
pub fn settings_edit_exposes_only_enabled_editable_keys() {
    let server = Server::start_with_config_transform("dash-035", None, true, |config, _| {
        config
            .replace(
                "[relay.tcp_server]\nenabled = true",
                "[relay.tcp_server]\nenabled = false",
            )
            .replace("[mode1]\nenabled = true", "[mode1]\nenabled = false")
    });
    let admin = server.login();
    let config = server.query(&admin, "/api/v1/config");
    let mut browser = Browser::start("dash-035", &["ko-KR"]);
    login_alice(&mut browser, &server);
    open_settings(&mut browser, &server);
    open_settings_edit(&mut browser, &server);

    let expected: BTreeSet<_> = items(&config)
        .iter()
        .filter(|entry| entry["editable"] == true)
        .map(|entry| entry["key"].as_str().unwrap_or(""))
        .collect();
    let rendered: Vec<String> = must(serde_json::from_value(browser.eval(
        "[...document.querySelectorAll('input[data-config-key],textarea[data-config-key]')].map(n=>n.dataset.configKey)",
    )));
    assert_eq!(rendered.len(), expected.len(), "no duplicated inputs");
    assert_eq!(
        rendered.iter().map(String::as_str).collect::<BTreeSet<_>>(),
        expected
    );
    for key in [
        "relay.tcp_server.allow",
        "relay.wss_viewer.allow",
        "relay.wss_viewer.allowed_origins",
        "mode1.allow_targets",
        "relay.wss_viewer.cert_file",
        "relay.wss_viewer.key_file",
        "management.cert_file",
        "management.key_file",
        "management.config_edit",
    ] {
        assert!(
            !expected.contains(key),
            "disabled/TLS/restart key must not be editable: {key}"
        );
    }
    for entry in items(&config)
        .iter()
        .filter(|entry| entry["reload"] == "restart")
    {
        assert!(!expected.contains(entry["key"].as_str().unwrap_or("")));
    }
    assert_eq!(browser.eval(
        "[...document.querySelectorAll('.card')].filter(c=>c.querySelector('.settings-fields')).map(c=>c.querySelector('.card-heading').textContent)",
    ), json!(["출발지 IP 허용 목록", "접근 정책", "연결 상한", "시간 제한", "연결 유지"]));
    for entry in items(&config)
        .iter()
        .filter(|entry| entry["editable"] == true)
    {
        let key = entry["key"].as_str().unwrap_or("");
        let group = if key.starts_with("relay.") && key.ends_with(".allow") {
            "출발지 IP 허용 목록"
        } else if key.starts_with("relay.") || key.starts_with("mode1.") {
            "접근 정책"
        } else if key.starts_with("limits.") {
            "연결 상한"
        } else if key.starts_with("timeouts.") {
            "시간 제한"
        } else {
            "연결 유지"
        };
        assert_eq!(browser.eval(&format!(
            "document.querySelector('[data-config-key=\"' + {} + '\"]').closest('.card').querySelector('.card-heading').textContent",
            script_str(key)
        )), json!(group));
        let value = &entry["value"];
        let expected = if let Some(list) = value.as_array() {
            list.iter()
                .map(|value| value.as_str().unwrap_or(""))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            value.to_string()
        };
        assert_eq!(
            get_field_value(&mut browser, key),
            expected,
            "running input value for {key}"
        );
    }
}

// DASH-036
pub fn invalid_settings_inputs_focus_first_error_without_patch() {
    let server = Server::start_with_config("dash-036", None, true);
    let admin = server.login();
    let config = server.query(&admin, "/api/v1/config");
    let mut browser = Browser::start("dash-036", &["ko-KR"]);
    login_alice(&mut browser, &server);
    open_settings(&mut browser, &server);
    open_settings_edit(&mut browser, &server);

    // Empty lines alone are excluded, without trimming or sorting the other items.
    set_field_value(
        &mut browser,
        "relay.tcp_viewer.allow",
        "\n::1\n\n127.0.0.1\n",
    );
    review(&mut browser);
    assert_eq!(browser.eval(
        "[...document.querySelector('.config-change .config-body').children[3].children].map(n=>n.textContent)",
    ), json!(["::1", "127.0.0.1"]));
    browser.key("Escape");
    for invalid in [" ", "\t", " 127.0.0.1", "127.0.0.1 ", "127.0. 0.1"] {
        let draft = format!("127.0.0.1\n\n{invalid}");
        set_field_value(&mut browser, "relay.tcp_viewer.allow", &draft);
        browser.click("#settings-review");
        field_error(
            &mut browser,
            "relay.tcp_viewer.allow",
            "3줄: 항목에 공백을 넣지 마십시오.",
        );
        focused(&mut browser, "relay.tcp_viewer.allow", Some(invalid));
        assert_eq!(
            get_field_value(&mut browser, "relay.tcp_viewer.allow"),
            draft
        );
        assert!(
            browser
                .text()
                .contains("입력 오류가 1개 있습니다. 첫 오류로 이동합니다.")
        );
        assert_eq!(
            browser.eval("document.querySelector('dialog[open]') === null"),
            json!(true)
        );
    }
    set_field_value(&mut browser, "relay.tcp_viewer.allow", "127.0.0.1");
    let numeric = item(&config, "timeouts.connect_secs");
    let min = numeric["min"]
        .as_i64()
        .unwrap_or_else(|| panic!("missing min"));
    let max = numeric["max"]
        .as_i64()
        .unwrap_or_else(|| panic!("missing max"));
    assert_eq!(
        browser.eval("document.querySelector('[data-config-key=\"timeouts.connect_secs\"]').closest('.config-field').querySelector('p.config-help').textContent"),
        json!(format!("범위: {min}~{max}")),
        "integer help must show the real GET bounds"
    );
    assert_eq!(
        browser.eval("document.querySelector('[data-config-key=\"timeouts.connect_secs\"]').closest('.config-number-unit').querySelector('.config-help').textContent"),
        json!("초"),
        "seconds unit belongs to the integer field"
    );
    for (invalid, message) in [
        (String::new(), "정수를 입력하십시오.".to_owned()),
        ("1.5".to_owned(), "정수를 입력하십시오.".to_owned()),
        ("1e2".to_owned(), "정수를 입력하십시오.".to_owned()),
        (
            (min - 1).to_string(),
            format!("{min} 이상 {max} 이하로 입력하십시오."),
        ),
        (
            (max + 1).to_string(),
            format!("{min} 이상 {max} 이하로 입력하십시오."),
        ),
    ] {
        set_field_value(&mut browser, "timeouts.connect_secs", &invalid);
        browser.click("#settings-review");
        field_error(&mut browser, "timeouts.connect_secs", &message);
        focused(&mut browser, "timeouts.connect_secs", None);
        assert!(
            browser
                .text()
                .contains("입력 오류가 1개 있습니다. 첫 오류로 이동합니다.")
        );
    }
    set_field_value(
        &mut browser,
        "timeouts.connect_secs",
        &numeric["value"].to_string(),
    );
    let relay_limit = item(&config, "limits.relay_connections")["value"]
        .as_u64()
        .unwrap_or(0);
    set_field_value(
        &mut browser,
        "limits.waiting",
        &(relay_limit + 1).to_string(),
    );
    browser.click("#settings-review");
    field_error(
        &mut browser,
        "limits.waiting",
        "limits.waiting은 limits.relay_connections 이하여야 합니다.",
    );
    focused(&mut browser, "limits.waiting", None);
    // Multiple errors must focus the first field in displayed (not API table) order.
    set_field_value(&mut browser, "relay.tcp_viewer.allow", " ");
    browser.click("#settings-review");
    focused(&mut browser, "relay.tcp_viewer.allow", Some(" "));
    assert!(
        browser
            .text()
            .contains("입력 오류가 2개 있습니다. 첫 오류로 이동합니다.")
    );
    assert_eq!(
        server.proxy.count("PATCH", "/api/v1/config"),
        0,
        "all client errors must prevent PATCH"
    );
}

// DASH-037
pub fn settings_confirmation_shows_diffs_empty_list_warning_and_timing() {
    let server = Server::start_with_config("dash-037", None, true);
    let admin = server.login();
    let config = server.query(&admin, "/api/v1/config");
    let mut browser = Browser::start("dash-037", &["ko-KR"]);
    login_alice(&mut browser, &server);
    open_settings(&mut browser, &server);
    open_settings_edit(&mut browser, &server);
    set_field_value(&mut browser, "limits.waiting", "10");
    set_field_value(&mut browser, "relay.tcp_viewer.allow", "");
    set_field_value(
        &mut browser,
        "relay.ws_viewer.allowed_origins",
        "https://example.test\nhttp://localhost",
    );
    review(&mut browser);

    let list_text = |key: &str| {
        item(&config, key)["value"]
            .as_array()
            .unwrap_or_else(|| panic!("list missing: {key}"))
            .iter()
            .map(|value| value.as_str().unwrap_or(""))
            .collect::<String>()
    };
    let old_allow = list_text("relay.tcp_viewer.allow");
    let old_origins = list_text("relay.ws_viewer.allowed_origins");
    let expected = json!([
        [
            "limits.waiting",
            [
                "이전 값",
                item(&config, "limits.waiting")["value"].to_string(),
                "새 값",
                "10"
            ]
        ],
        [
            "relay.tcp_viewer.allow",
            [
                "이전 값",
                old_allow,
                "새 값",
                "빈 목록",
                "추가한 항목",
                "빈 목록",
                "뺀 항목",
                old_allow
            ]
        ],
        [
            "relay.ws_viewer.allowed_origins",
            [
                "이전 값",
                old_origins,
                "새 값",
                "https://example.testhttp://localhost",
                "추가한 항목",
                "https://example.testhttp://localhost",
                "뺀 항목",
                old_origins
            ]
        ]
    ]);
    let actual = browser.eval(
        "[...document.querySelectorAll('dialog .config-change')].map(c=>[c.querySelector('.card-heading').textContent,[...c.querySelector('.config-body').children].filter(n=>!n.classList.contains('alert')).map(n=>n.textContent)]).sort((a,b)=>a[0].localeCompare(b[0]))",
    );
    assert_eq!(
        actual, expected,
        "every changed key has exact old/new and added/removed values"
    );
    let dialog = browser.eval("document.querySelector('dialog.config-dialog').textContent");
    let text = dialog.as_str().unwrap_or("");
    assert!(text.contains("빈 허용 목록은 해당 키가 검사하는 모든 새 접속·요청을 거절합니다."));
    assert!(text.contains("새 연결부터 적용됩니다. 기존 연결·이미 시작한 타이머·이미 설정한 소켓 옵션은 이전 값을 유지합니다."));
    assert_eq!(server.proxy.count("PATCH", "/api/v1/config"), 0);
}

// DASH-038
pub fn settings_confirmation_cancel_and_escape_preserve_input_and_focus() {
    let server = Server::start_with_config("dash-038", None, true);
    let mut browser = Browser::start("dash-038", &["ko-KR"]);
    login_alice(&mut browser, &server);
    open_settings(&mut browser, &server);
    open_settings_edit(&mut browser, &server);
    set_field_value(&mut browser, "limits.waiting", "10");
    set_field_value(&mut browser, "relay.tcp_viewer.allow", "::1\n\n127.0.0.1");
    for escape in [false, true] {
        review(&mut browser);
        if escape {
            browser.key("Escape");
        } else {
            browser.click("dialog.config-dialog .dialog-actions .button-outline");
        }
        browser.wait("document.querySelector('dialog.config-dialog[open]') === null");
        assert_eq!(get_field_value(&mut browser, "limits.waiting"), "10");
        assert_eq!(
            get_field_value(&mut browser, "relay.tcp_viewer.allow"),
            "::1\n\n127.0.0.1"
        );
        assert_eq!(
            browser.eval("document.activeElement === document.querySelector('#settings-review')"),
            json!(true)
        );
        assert_eq!(
            server.proxy.count("PATCH", "/api/v1/config"),
            0,
            "neither cancel path may submit"
        );
    }
}

// DASH-039
pub fn confirmed_settings_patch_sends_revision_csrf_and_shows_applied_count() {
    let server = Server::start_with_config_transform("dash-039", None, true, |config, _| {
        config.replace(
            "waiting = 8",
            "# Keep this comment and the neighboring spacing.\nwaiting  =  8  # waiting comment",
        )
    });
    let admin = server.login();
    let original = server.config_text();
    let mut browser = Browser::start("dash-039", &["ko-KR"]);
    login_alice(&mut browser, &server);
    open_settings(&mut browser, &server);
    let pending = original.replace("close_secs = 2", "close_secs = 3");
    assert_ne!(pending, original);
    let _pending = PendingFile::write(&server, &pending);
    let before = server.query(&admin, "/api/v1/config");
    assert_eq!(before["file"]["state"], "pending");
    assert_eq!(
        before["file"]["config_keys"],
        json!(["timeouts.close_secs"])
    );
    assert_eq!(item(&before, "timeouts.close_secs")["value"], 2);
    open_settings_edit(&mut browser, &server);
    set_field_value(&mut browser, "limits.waiting", "10");
    review(&mut browser);
    let released = Arc::new(AtomicBool::new(false));
    let injected = server.proxy.injected_count("PATCH", "/api/v1/config");
    server.proxy.once(
        "PATCH",
        "/api/v1/config",
        Fault::Hold(Arc::clone(&released)),
    );
    let gets = server.proxy.count("GET", "/api/v1/config");
    let immediately_before = server.query(&admin, "/api/v1/config");
    assert_eq!(immediately_before["file"]["state"], "pending");
    assert_eq!(
        immediately_before["file"]["config_keys"],
        json!(["timeouts.close_secs"])
    );
    apply(&mut browser);
    wait_for("real response held while saving", LIMIT, || {
        server
            .proxy
            .traffic()
            .iter()
            .any(|entry| entry.method == "PATCH" && entry.stage == "response_held")
    });
    assert_eq!(browser.eval(
        "[...document.querySelectorAll('.config-input,#settings-review,#settings-refresh')].every(n=>n.disabled) && document.querySelectorAll('dialog[open]').length === 0",
    ), json!(true), "inputs and duplicate review/apply must be disabled while saving");
    assert!(
        browser
            .text()
            .contains("설정을 저장하고 적용하는 중입니다.")
    );
    browser.eval("document.querySelector('#settings-review').click(); document.querySelector('#settings-review').click();");
    assert_eq!(server.proxy.count("PATCH", "/api/v1/config"), 1);
    released.store(true, Ordering::Release);
    view_ready(&mut browser);
    let reply = patch_reply(&server, 1);
    assert_eq!(reply["result"], "applied");
    assert_eq!(
        reply["applied"],
        json!(["limits.waiting", "timeouts.close_secs"])
    );
    let traffic = server.proxy.traffic();
    let patch = traffic
        .iter()
        .find(|entry| entry.method == "PATCH")
        .unwrap_or_else(|| panic!("PATCH missing"));
    let body: Value = must(serde_json::from_str(&patch.body));
    assert_eq!(
        body,
        json!({"revision":before["revision"],"changes":{"limits.waiting":10}})
    );
    let session = traffic
        .iter()
        .rev()
        .find(|entry| entry.method == "GET" && entry.path == "/api/v1/session")
        .unwrap_or_else(|| panic!("browser session missing"));
    let session: Value = must(serde_json::from_str(&session.response_body));
    let csrf = patch
        .headers
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("X-CSRF-Token"))
        .map(|(_, value)| value.trim())
        .unwrap_or("");
    assert_eq!(
        json!(csrf),
        session["csrf_token"],
        "send the actual browser session CSRF token"
    );
    assert!(!csrf.is_empty());
    assert!(!patch.headers.to_ascii_lowercase().contains("x-relay-poll:"));
    assert!(
        server.proxy.count("GET", "/api/v1/config") > gets,
        "view must re-query after success"
    );
    assert_eq!(
        server.proxy.injected_count("PATCH", "/api/v1/config"),
        injected + 1,
        "only response timing was held; real response delivered"
    );
    let count = reply["applied"].as_array().map_or(0, Vec::len);
    assert_eq!(
        count, 2,
        "one requested key applies together with the real pending key"
    );
    assert!(
        browser
            .text()
            .contains(&format!("설정 {count}개를 적용했습니다."))
    );
    assert_eq!(
        server.config_text(),
        pending.replace("waiting  =  8", "waiting  =  10"),
        "only edited value bytes change; comments/spacing/order preserved"
    );
    let after = server.query(&admin, "/api/v1/config");
    assert_eq!(item(&after, "limits.waiting")["value"], 10);
    assert_eq!(item(&after, "timeouts.close_secs")["value"], 3);
    assert_eq!(browser.eval("document.querySelector('.config-row[data-config-key=\"limits.waiting\"] .config-value').textContent"), item(&after, "limits.waiting")["value"].to_string());
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(
        server.proxy.count("PATCH", "/api/v1/config"),
        1,
        "no duplicate or automatic resend"
    );
}

// DASH-040
pub fn server_settings_diagnostics_preserve_input_and_map_original_lines() {
    let server = Server::start_with_config("dash-040", None, true);
    let original = server.config_text();
    // Real pending file: the form permits waiting=9 against running L=16;
    // candidate validation against the file's L=8 must reject it on the server.
    let pending = original.replace("relay_connections = 16", "relay_connections = 8");
    let mut browser = Browser::start("dash-040", &["ko-KR"]);
    login_alice(&mut browser, &server);
    open_settings(&mut browser, &server);
    // Establish the running baseline before changing the file, rather than
    // racing the watcher's initial startup read.
    let admin = server.login();
    assert_eq!(
        item(
            &server.query(&admin, "/api/v1/config"),
            "limits.relay_connections"
        )["value"],
        16
    );
    assert_ne!(pending, original);
    let _pending = PendingFile::write(&server, &pending);
    let config = server.query(&admin, "/api/v1/config");
    assert_eq!(config["file"]["state"], "pending");
    assert_eq!(item(&config, "limits.relay_connections")["value"], 16);
    open_settings_edit(&mut browser, &server);
    let ip = "127.0.0.1\n\n::1\nBAD-IP\n203.0.113.20-203.0.113.10\n192.0.2.1-2001:db8::1";
    let origin = "http://127.0.0.1\n\nhttps://example.test\nBAD-ORIGIN";
    let target = "127.0.0.1:5900\n\n127.0.0.1:5901\nBAD-TARGET";
    let translations = [
        (
            "config_value_out_of_range",
            "limits.waiting",
            "설정 값이 허용 범위를 벗어났습니다.",
            "The setting value is outside its allowed range.",
        ),
        (
            "config_ip_item_invalid",
            "relay.tcp_viewer.allow",
            "IP·CIDR·범위 항목의 형식이 올바르지 않습니다.",
            "The IP, CIDR, or range item has an invalid format.",
        ),
        (
            "config_ip_range_reversed",
            "relay.tcp_viewer.allow",
            "IP 범위의 시작 주소는 끝 주소 이하여야 합니다.",
            "The start address of an IP range must not exceed its end address.",
        ),
        (
            "config_ip_range_family_mixed",
            "relay.tcp_viewer.allow",
            "IP 범위의 두 주소는 모두 IPv4이거나 모두 IPv6여야 합니다.",
            "Both addresses in an IP range must be IPv4 or both must be IPv6.",
        ),
        (
            "config_origin_invalid",
            "relay.ws_viewer.allowed_origins",
            "Origin 항목의 형식이 올바르지 않습니다.",
            "The Origin item has an invalid format.",
        ),
        (
            "config_target_item_invalid",
            "mode1.allow_targets",
            "Mode I 대상 항목의 형식이 올바르지 않습니다.",
            "The Mode I target item has an invalid format.",
        ),
    ];
    for lang in ["ko", "en"] {
        switch_language(&mut browser, lang);
        set_field_value(&mut browser, "limits.waiting", "9");
        set_field_value(&mut browser, "relay.tcp_viewer.allow", ip);
        set_field_value(&mut browser, "relay.ws_viewer.allowed_origins", origin);
        set_field_value(&mut browser, "mode1.allow_targets", target);
        let draft = form_values(&mut browser);
        let count = server.proxy.count("PATCH", "/api/v1/config") + 1;
        review(&mut browser);
        let immediately_before = server.query(&admin, "/api/v1/config");
        assert_eq!(immediately_before["file"]["state"], "pending");
        assert_eq!(
            immediately_before["file"]["config_keys"],
            json!(["limits.relay_connections"])
        );
        apply(&mut browser);
        browser.wait("document.querySelectorAll('.config-field-error').length >= 6");
        let reply = patch_reply(&server, count);
        assert_eq!(reply["error"]["code"], "config_invalid");
        let traffic = server.proxy.traffic();
        let request = traffic
            .iter()
            .filter(|entry| entry.method == "PATCH" && entry.path == "/api/v1/config")
            .nth(count - 1)
            .unwrap_or_else(|| panic!("real PATCH missing"));
        assert!(request.forwarded && !request.injected && request.status == Some(400));
        let request: Value = must(serde_json::from_str(&request.body));
        assert_eq!(
            request["changes"]["relay.tcp_viewer.allow"],
            json!(
                ip.lines()
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<_>>()
            )
        );
        let diagnostics = reply["error"]["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("real diagnostics missing: {reply}"));
        let rendered_errors = browser.eval(r#"[...document.querySelectorAll('.config-field-error')].map(n=>n.textContent).join('\n')"#);
        let rendered_errors = rendered_errors.as_str().unwrap_or("");
        for (code, key, ko, en) in translations {
            let diagnostic = diagnostics
                .iter()
                .find(|entry| entry["code"] == code)
                .unwrap_or_else(|| panic!("real code {code} missing: {reply}"));
            if code == "config_ip_item_invalid" {
                assert_eq!(diagnostic["key"], "relay.tcp_viewer.allow[2]");
            }
            let translated = if lang == "ko" { ko } else { en };
            let indexed = diagnostic["key"].as_str().unwrap_or("").contains('[');
            let line = if code == "config_ip_range_reversed" {
                5
            } else if code == "config_ip_range_family_mixed" {
                6
            } else {
                4
            };
            let expected = if indexed {
                if lang == "ko" {
                    format!("{line}줄: {translated}")
                } else {
                    format!("Line {line}: {translated}")
                }
            } else {
                translated.to_owned()
            };
            field_error(&mut browser, key, &expected);
            let raw = diagnostic["message"].as_str().unwrap_or("");
            assert!(!raw.is_empty());
            assert!(
                !rendered_errors.contains(raw),
                "known diagnostics must not leak raw message: {code}"
            );
            assert!(!rendered_errors.contains(code));
        }
        assert_eq!(
            form_values(&mut browser),
            draft,
            "all draft inputs survive real validation failure"
        );
        focused(&mut browser, "relay.tcp_viewer.allow", Some("BAD-IP"));
        assert!(browser.text().contains(if lang == "ko" {
            "입력한 설정의 검증에 실패했습니다."
        } else {
            "The entered settings failed validation."
        }));
        assert!(!browser.text().contains(if lang == "ko" {
            "개를 적용했습니다"
        } else {
            "Applied "
        }));
        assert_eq!(server.config_text(), pending);
        assert_eq!(
            server.proxy.injected_count("PATCH", "/api/v1/config"),
            if lang == "ko" { 0 } else { 4 }
        );

        // These three codes cannot be produced by a well-typed editable PATCH:
        // required keys cannot be removed, types are fixed, and the relation
        // diagnostic concerns restart-only session keys. Exercise their public
        // config_invalid response contract, and the unknown-code fallback.
        for (code, ko, en, response) in [
            (
                "config_missing_key",
                "필수 설정 값이 없습니다.",
                "A required setting value is missing.",
                r#"{"error":{"code":"config_invalid","message":"Validation failed","diagnostics":[{"key":"relay.tcp_viewer.allow[2]","code":"config_missing_key","message":"RAW-UNKNOWN-DIAGNOSTIC"}]}}"#,
            ),
            (
                "config_type_mismatch",
                "설정 값의 자료형이 맞지 않습니다.",
                "The setting value has an invalid type.",
                r#"{"error":{"code":"config_invalid","message":"Validation failed","diagnostics":[{"key":"relay.tcp_viewer.allow[2]","code":"config_type_mismatch","message":"RAW-UNKNOWN-DIAGNOSTIC"}]}}"#,
            ),
            (
                "config_relation_violated",
                "설정 키 사이의 관계를 만족하지 않습니다.",
                "The relationship between setting keys is not valid.",
                r#"{"error":{"code":"config_invalid","message":"Validation failed","diagnostics":[{"key":"relay.tcp_viewer.allow[2]","code":"config_relation_violated","message":"RAW-UNKNOWN-DIAGNOSTIC"}]}}"#,
            ),
            (
                "unknown_diagnostic_code",
                "RAW-UNKNOWN-DIAGNOSTIC (unknown_diagnostic_code)",
                "RAW-UNKNOWN-DIAGNOSTIC (unknown_diagnostic_code)",
                r#"{"error":{"code":"config_invalid","message":"Validation failed","diagnostics":[{"key":"relay.tcp_viewer.allow[2]","code":"unknown_diagnostic_code","message":"RAW-UNKNOWN-DIAGNOSTIC"}]}}"#,
            ),
        ] {
            let injected = server.proxy.injected_count("PATCH", "/api/v1/config");
            server
                .proxy
                .once("PATCH", "/api/v1/config", Fault::Response(400, response));
            review(&mut browser);
            apply(&mut browser);
            browser.wait("document.querySelectorAll('.config-field-error').length === 1");
            let translated = if lang == "ko" { ko } else { en };
            let message = if lang == "ko" {
                format!("4줄: {translated}")
            } else {
                format!("Line 4: {translated}")
            };
            field_error(&mut browser, "relay.tcp_viewer.allow", &message);
            focused(&mut browser, "relay.tcp_viewer.allow", Some("BAD-IP"));
            let errors = browser.eval("document.querySelector('.config-field-error').textContent");
            if code != "unknown_diagnostic_code" {
                assert!(
                    !errors
                        .as_str()
                        .unwrap_or("")
                        .contains("RAW-UNKNOWN-DIAGNOSTIC")
                );
                assert!(!errors.as_str().unwrap_or("").contains(code));
            }
            assert_eq!(
                server.proxy.injected_count("PATCH", "/api/v1/config"),
                injected + 1
            );
            assert_eq!(
                form_values(&mut browser),
                draft,
                "all draft inputs survive contract diagnostics"
            );
            assert_eq!(browser.eval("location.hash"), json!("#/settings/edit"));
            assert!(!browser.text().contains(if lang == "ko" {
                "개를 적용했습니다"
            } else {
                "Applied "
            }));
        }
    }
    assert_eq!(
        server.config_text(),
        pending,
        "no validation rejection may change the file"
    );
    let after = server.query(&admin, "/api/v1/config");
    assert_eq!(after["items"], config["items"], "running values unchanged");
}

// DASH-052
pub fn unchanged_settings_return_without_patch_and_empty_applied_shows_zero() {
    let server = Server::start_with_config("dash-052", None, true);
    let admin = server.login();
    let mut browser = Browser::start("dash-052", &["ko-KR"]);
    login_alice(&mut browser, &server);
    open_settings(&mut browser, &server);
    open_settings_edit(&mut browser, &server);
    review(&mut browser);
    assert!(browser.text().contains("바뀐 설정이 없습니다."));
    apply(&mut browser);
    view_ready(&mut browser);
    assert_eq!(
        server.proxy.count("PATCH", "/api/v1/config"),
        0,
        "unchanged form never sends empty changes"
    );
    assert!(browser.text().contains("설정 0개를 적용했습니다."));

    // The file already contains the exact serialized value 10, while running
    // waiting is still 8. The form sends a nonempty real PATCH, but it changes
    // no file bytes and the server returns applied=[] (not a fabricated 200).
    let pending = server.config_text().replace("waiting = 8", "waiting = 10");
    let _pending = PendingFile::write(&server, &pending);
    let config = server.query(&admin, "/api/v1/config");
    assert_eq!(config["file"]["state"], "pending");
    assert_eq!(item(&config, "limits.waiting")["value"], 8);
    open_settings_edit(&mut browser, &server);
    set_field_value(&mut browser, "limits.waiting", "10");
    review(&mut browser);
    let immediately_before = server.query(&admin, "/api/v1/config");
    assert_eq!(immediately_before["file"]["state"], "pending");
    assert_eq!(
        immediately_before["file"]["config_keys"],
        json!(["limits.waiting"])
    );
    apply(&mut browser);
    view_ready(&mut browser);
    let reply = patch_reply(&server, 1);
    assert_eq!(reply["result"], "applied");
    assert_eq!(reply["applied"], json!([]));
    assert!(browser.text().contains("설정 0개를 적용했습니다."));
    assert_eq!(server.config_text(), pending);
    let after = server.query(&admin, "/api/v1/config");
    assert_eq!(
        after["items"], config["items"],
        "no-op PATCH must not apply pending values"
    );
    std::thread::sleep(Duration::from_secs(3));
    assert_eq!(
        server.proxy.count("PATCH", "/api/v1/config"),
        1,
        "empty applied must not trigger resend"
    );
    assert_eq!(server.proxy.injected_count("PATCH", "/api/v1/config"), 0);
    for patch in server
        .proxy
        .traffic()
        .iter()
        .filter(|entry| entry.method == "PATCH" && entry.path == "/api/v1/config")
    {
        let body: Value = must(serde_json::from_str(&patch.body));
        assert_eq!(
            body,
            json!({"revision":config["revision"],"changes":{"limits.waiting":10}})
        );
        assert!(patch.forwarded && !patch.injected && patch.status == Some(200));
    }
}
