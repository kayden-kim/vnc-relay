//! SCR-008 real-process sources: menu entry, automatic polls, leaving and returning, and the counts, lookup links
//! and database states of a real `serve` with generated IP information databases and WebSocket clients behind a
//! trusted proxy. All scenario markers live in the root runner, not in these helpers.
//!
//! Addresses are documentation ranges (RFC 5737, RFC 3849) and AS numbers are examples (RFC 5398).

use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

use super::settings;
use vnc_relay_testkit::cdp::Browser;
use vnc_relay_testkit::dashboard_proxy::Fault;
use vnc_relay_testkit::live::{self, Admin, LIMIT, Server};
use vnc_relay_testkit::mmdb;
use vnc_relay_testkit::mmdb::MmdbWriter;

const API: &str = "/api/v1/sources";
const ENTRY: &str = "/api/v1/sources?window=1h";
const MENU: &str = "a.navbar-item[href='/#/sources']";
const BUILD_EPOCH: u64 = 1_761_955_200; // 2025-11-01T00:00:00Z

/// Path and lower-case request headers of every sources GET the browser sent, oldest first.
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
        "document.querySelector('#sources-refresh')?.disabled === false && document.querySelectorAll('[data-kpi]').length === 7 && document.querySelectorAll('[data-sources-card]').length === 4",
    );
}

fn is_poll(headers: &str) -> bool {
    headers.contains("x-relay-poll: 1")
}

/// Writes a DB-IP country database and a MaxMind GeoLite ASN database, trusts the loopback proxy on the WS
/// listener, and allows the documentation clients there.
fn with_databases(config: String, dir: &Path) -> String {
    let country = MmdbWriter::new("DBIP-Country-Lite", BUILD_EPOCH)
        .insert("198.51.100.0/24", mmdb::iso_country("KR"))
        .insert("2001:db8::/32", mmdb::iso_country("JP"))
        .to_bytes();
    let asn = MmdbWriter::new("GeoLite2-ASN", BUILD_EPOCH)
        .insert(
            "198.51.100.0/24",
            mmdb::maxmind_asn(64496, "Example Transit"),
        )
        .to_bytes();
    live::must(fs::create_dir_all(dir.join("ipdb")));
    live::must(fs::write(dir.join("ipdb").join("country.mmdb"), country));
    live::must(fs::write(dir.join("ipdb").join("asn.mmdb"), asn));
    let config = config
        .replace(
            "[relay.ws]\n",
            "[relay.ws]\ntrusted_proxies = [\"127.0.0.1\"]\n",
        )
        .replace(
            "allow = [\"127.0.0.1\"]\nallowed_origins",
            "allow = [\"127.0.0.1\", \"198.51.100.0/24\", \"2001:db8::/32\"]\nallowed_origins",
        );
    format!("{config}\n[ip_info]\ncountry_db = \"ipdb/country.mmdb\"\nasn_db = \"ipdb/asn.mmdb\"\n")
}

/// Opens a WebSocket Viewer through the trusted loopback proxy for `client` and returns the open stream.
fn ws_from(server: &Server, client: &str) -> TcpStream {
    let mut stream = live::connect(server.ws_port);
    let request = format!(
        "GET /relay/viewer HTTP/1.1\r\nHost: localhost\r\nOrigin: http://127.0.0.1\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Protocol: binary\r\nX-Forwarded-For: {client}\r\n\r\n"
    );
    live::must(stream.write_all(request.as_bytes()));
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        live::must(stream.read_exact(&mut byte));
        head.push(byte[0]);
        assert!(head.len() < 16384, "bounded WebSocket reply head");
    }
    assert!(
        String::from_utf8_lossy(&head).starts_with("HTTP/1.1 101"),
        "WebSocket Viewer from {client}: {}",
        String::from_utf8_lossy(&head)
    );
    stream
}

/// One non-local IPv4 client with a country and an ASN, one non-local IPv6 client with a country only, and one
/// local client on the TCP Viewer listener. The server records a connection when it ends, so each one is dropped;
/// returns once all three addresses are counted.
fn connect_clients(server: &Server, admin: &Admin) {
    drop(ws_from(server, "198.51.100.7"));
    drop(ws_from(server, "2001:db8::7"));
    drop(server.handshaking());
    live::wait_for("three source addresses", LIMIT, || {
        server.query(admin, ENTRY)["addresses"]["total"] == json!(3)
    });
}

// DASH-082
pub fn menu_entry_automatic_polls_leaving_and_returning() {
    let server = Server::start_with_config_transform("dash-082", Some(1), true, with_databases);
    let mut browser = Browser::start("dash-082", &["ko-KR"]);

    // Without a management session the direct address goes to the login screen and never reads the sources.
    browser.navigate(&server.url("/#/sources"));
    browser.wait(
        "location.pathname === '/login/' && document.querySelector('#login-account') !== null",
    );
    assert_eq!(
        browser.eval(
            "document.querySelectorAll('.sources-main,[data-kpi],[data-sources-card]').length"
        ),
        json!(0)
    );
    assert!(
        !server
            .proxy
            .traffic()
            .iter()
            .any(|r| r.path.starts_with(API) && r.status == Some(200)),
        "an unauthenticated sources GET never succeeds"
    );

    browser.login(&server);
    assert!(
        requests(&server).is_empty(),
        "the overview never reads the sources"
    );
    assert_eq!(
        browser.eval(
            "[...document.querySelectorAll('.navbar-nav a.navbar-item')].map(a => a.getAttribute('href')).join(' ')"
        ),
        json!("/ /#/metrics /#/sources /#/logs /#/settings"),
        "top bar order: overview, metrics, sources, logs, settings"
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
    assert_eq!(browser.eval("location.hash"), json!("#/sources"));
    assert_eq!(
        browser.eval(
            "document.querySelector(\"a.navbar-item[aria-current='page']\")?.getAttribute('href')"
        ),
        json!("/#/sources")
    );

    let entry = requests(&server);
    assert_eq!(entry[0].0, ENTRY, "the entry request reads the last hour");
    assert!(
        !entry[0].1.contains("x-relay-poll"),
        "the entry request is a user action: {}",
        entry[0].1
    );
    live::wait_for("real automatic sources poll", LIMIT, || {
        requests(&server)
            .iter()
            .skip(1)
            .any(|(path, headers)| path == ENTRY && is_poll(headers))
    });
    assert!(
        requests(&server)
            .iter()
            .skip(1)
            .all(|(path, headers)| path == ENTRY && is_poll(headers)),
        "every later request of the screen is an automatic poll of the same period"
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
        "leaving stops sources polling"
    );

    // Returning starts again from the last hour with a user request, even after another period was chosen.
    browser.click(MENU);
    ready(&mut browser);
    browser.eval(
        "(() => { const s = document.querySelector('#sources-period'); s.value = '24h'; s.dispatchEvent(new Event('change', { bubbles: true })); })()",
    );
    live::wait_for("the 24-hour request", LIMIT, || {
        requests(&server)
            .iter()
            .any(|(path, _)| path == "/api/v1/sources?window=24h")
    });
    ready(&mut browser);
    browser.click("a.navbar-item[href='/#/logs']");
    browser.wait("document.querySelector('#logs-refresh') !== null");
    thread::sleep(Duration::from_millis(300));
    let before = requests(&server).len();
    browser.click(MENU);
    ready(&mut browser);
    let back = requests(&server);
    assert_eq!(back[before].0, ENTRY, "returning reads the last hour again");
    assert!(!back[before].1.contains("x-relay-poll"));
    assert_eq!(
        browser.eval("document.querySelector('#sources-period').value"),
        json!("1h")
    );

    // A response that arrives after leaving does not cover the next screen.
    browser.click("a.navbar-item[href='/#/logs']");
    browser.wait("document.querySelector('#logs-refresh') !== null");
    let held = Arc::new(AtomicBool::new(false));
    server
        .proxy
        .once("GET", ENTRY, Fault::Hold(Arc::clone(&held)));
    let before = server.proxy.traffic().len();
    browser.click(MENU);
    live::wait_for("sources entry held", LIMIT, || {
        server
            .proxy
            .traffic()
            .iter()
            .skip(before)
            .any(|r| r.path == ENTRY && r.stage == "response_held")
    });
    browser.click("a.navbar-item[href='/#/logs']");
    browser.wait("document.querySelector('#logs-refresh') !== null");
    held.store(true, Ordering::Release);
    live::wait_for("late sources response released", LIMIT, || {
        server
            .proxy
            .traffic()
            .iter()
            .skip(before)
            .any(|r| r.path == ENTRY && r.completed.is_some())
    });
    thread::sleep(Duration::from_millis(300));
    assert_eq!(
        browser.eval(
            "location.hash === '#/logs' && document.querySelector('#logs-refresh') !== null && document.querySelector('.sources-main,[data-kpi]') === null"
        ),
        json!(true),
        "a late sources response does not cover the logs screen"
    );
    assert_eq!(server.proxy.injected_count("GET", ENTRY), 1);
}

// DASH-083, DASH-084 and DASH-085 with real values. The response shapes the fixtures cannot reach in a real
// process (150 addresses, unknown states, every provider) are covered by the browser scenarios.
pub fn real_counts_links_and_databases() {
    let server = Server::start_with_config_transform("dash-083", Some(1), false, with_databases);
    let admin = server.login();
    connect_clients(&server, &admin);
    let mut browser = Browser::start("dash-083", &["ko-KR"]);
    browser.login(&server);
    browser.navigate(&server.url("/#/sources"));
    ready(&mut browser);
    let oracle: Value = server.query(&admin, ENTRY);

    // Seven summary cards: the four totals and the three counts, in that order.
    let expected: Vec<Value> = ["attempts", "rejected", "aborted", "sessions"]
        .iter()
        .map(|key| json!(oracle["totals"][key].to_string()))
        .chain(
            ["addresses", "countries", "asns"]
                .iter()
                .map(|key| json!(oracle[key]["total"].to_string())),
        )
        .collect();
    assert_eq!(
        browser.eval(
            "[...document.querySelectorAll('[data-kpi] .metric')].map(e => e.textContent.trim())"
        ),
        json!(expected)
    );

    // The address table keeps the response order. Non-local addresses and ASNs link to ipinfo.io in a new tab;
    // the local address and missing values have no link.
    let rows = browser.eval(
        r#"[...document.querySelectorAll('[data-sources-card="addresses"] tbody tr')].map(tr => {
            const lookup = [...tr.querySelectorAll('a')].map(a => [a.getAttribute('href'), a.target, a.rel, a.getAttribute('aria-label')]);
            return {
                ip: tr.cells[0].querySelector('a, .mono').textContent,
                local: tr.cells[0].querySelector('.badge')?.textContent ?? null,
                country: tr.cells[1].textContent,
                asn: tr.cells[2].textContent,
                lookup,
            };
        })"#,
    );
    let items = oracle["addresses"]["items"]
        .as_array()
        .unwrap_or_else(|| panic!("address items"));
    assert_eq!(rows.as_array().map(Vec::len), Some(items.len()));
    let link = |value: &str| {
        json!([
            format!("https://ipinfo.io/{value}"),
            "_blank",
            "noopener noreferrer",
            format!("ipinfo.io에서 {value} 조회 (새 탭에서 열림)")
        ])
    };
    for (row, item) in rows.as_array().into_iter().flatten().zip(items) {
        let ip = item["ip"].as_str().unwrap_or_default();
        assert_eq!(row["ip"], json!(ip));
        let expected = match ip {
            "198.51.100.7" => json!({
                "ip": ip, "local": null, "country": "대한민국 (KR)", "asn": "AS64496Example Transit",
                "lookup": [link(ip), link("AS64496")],
            }),
            "2001:db8::7" => json!({
                "ip": ip, "local": null, "country": "일본 (JP)", "asn": "알 수 없음", "lookup": [link(ip)],
            }),
            "127.0.0.1" => json!({
                "ip": ip, "local": "로컬", "country": "알 수 없음", "asn": "알 수 없음", "lookup": [],
            }),
            other => panic!("unexpected source address {other}"),
        };
        assert_eq!(row, &expected, "address row {ip}");
    }
    assert_eq!(
        browser.eval(
            "[...document.querySelectorAll('[data-sources-card=\"countries\"] tbody tr')].map(tr => tr.cells[0].textContent)"
        ),
        json!(
            oracle["countries"]["items"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|item| match item["country"].as_str() {
                    Some("KR") => "대한민국 (KR)",
                    Some("JP") => "일본 (JP)",
                    None => "알 수 없음",
                    Some(other) => panic!("unexpected country {other}"),
                })
                .collect::<Vec<_>>()
        )
    );
    assert_eq!(
        browser.eval(
            "[...document.querySelectorAll('[data-sources-card=\"asns\"] tbody tr')].map(tr => tr.cells[0].textContent + '|' + tr.cells[1].textContent)"
        ),
        json!(
            oracle["asns"]["items"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|item| match item["asn"].as_u64() {
                    Some(64496) => "AS64496|Example Transit".to_owned(),
                    None => "알 수 없음|".to_owned(),
                    Some(other) => panic!("unexpected ASN {other}"),
                })
                .collect::<Vec<_>>()
        )
    );

    // Both databases are in use. Each provider is attributed once: DB-IP for countries, MaxMind for ASNs.
    assert_eq!(
        browser.eval(
            "[...document.querySelectorAll('[data-database]')].map(row => [row.dataset.database, row.querySelector('.badge')?.textContent, row.querySelector('dl')?.textContent.includes(row.dataset.database === 'country' ? 'DBIP-Country-Lite' : 'GeoLite2-ASN')])"
        ),
        json!([["country", "사용 중", true], ["asn", "사용 중", true]])
    );
    assert_eq!(
        browser.eval(
            "[...document.querySelectorAll('[data-provider]')].map(p => [p.dataset.provider, p.querySelector('a').getAttribute('href')])"
        ),
        json!([
            ["dbip", "https://db-ip.com"],
            ["maxmind_geolite", "https://www.maxmind.com"]
        ])
    );

    // The lookup links take keyboard focus. The test does not follow them, so nothing leaves the machine; the
    // dashboard page itself never requested ipinfo.io.
    assert_eq!(
        browser.eval(
            "[...document.querySelectorAll('.sources-main a[href^=\"https://ipinfo.io/\"]')].every(a => { a.focus(); return document.activeElement === a; })"
        ),
        json!(true)
    );
    assert_eq!(
        browser.eval(
            "location.hash === '#/sources' && performance.getEntriesByType('resource').every(e => !e.name.includes('ipinfo.io'))"
        ),
        json!(true)
    );
}

const DENY: &str = "relay.deny.sources";

fn deny_button(ip: &str, action: &str) -> String {
    format!(
        "[data-sources-card='addresses'] button[data-deny-address='{ip}'][data-deny-action='{action}']"
    )
}

fn wait_for_deny_button(browser: &mut Browser, ip: &str, action: &str) {
    browser.wait(&format!(
        "document.querySelector(\"{}\") !== null",
        deny_button(ip, action)
    ));
}

/// The effective value of the deny list, read with the oracle session.
fn deny_value(server: &Server, admin: &Admin) -> Value {
    let config = server.query(admin, "/api/v1/config");
    config["items"]
        .as_array()
        .and_then(|items| items.iter().find(|item| item["key"] == DENY))
        .map(|item| item["value"].clone())
        .unwrap_or_else(|| panic!("{DENY} is in the configuration"))
}

/// The `deny_entries` of one address of the last-hour report, read with the oracle session.
fn address_entries(server: &Server, admin: &Admin, ip: &str) -> Value {
    let report = server.query(admin, ENTRY);
    report["addresses"]["items"]
        .as_array()
        .and_then(|items| items.iter().find(|item| item["ip"] == ip))
        .map(|item| item["deny_entries"].clone())
        .unwrap_or_else(|| panic!("{ip} is among the sources"))
}

/// What the block status cell of one address shows: the badge text, the button action and the range note.
fn deny_cell(browser: &mut Browser, ip: &str) -> Value {
    browser.eval(&format!(
        r#"(() => {{
            const row = [...document.querySelectorAll('[data-sources-card="addresses"] tbody tr')].find(tr => tr.children[0].textContent.includes('{ip}'));
            const cell = row?.querySelector('td.src-deny');
            return {{
                badge: cell?.querySelector('.badge')?.textContent ?? null,
                button: cell?.querySelector('button')?.dataset.denyAction ?? null,
                note: cell?.querySelector('.src-deny-note')?.textContent ?? null,
                empty: cell?.childElementCount === 0,
            }};
        }})()"#
    ))
}

/// The first line of the reply to a WebSocket Viewer upgrade from `client` behind the trusted proxy. Empty when the
/// server closes the connection without a reply.
fn ws_reply_line(server: &Server, client: &str) -> String {
    let mut stream = live::connect(server.ws_port);
    live::must(stream.set_read_timeout(Some(Duration::from_secs(3))));
    let request = format!(
        "GET /relay/viewer HTTP/1.1\r\nHost: localhost\r\nOrigin: http://127.0.0.1\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Protocol: binary\r\nX-Forwarded-For: {client}\r\n\r\n"
    );
    live::must(stream.write_all(request.as_bytes()));
    let mut buffer = [0; 512];
    let read = stream.read(&mut buffer).unwrap_or(0);
    String::from_utf8_lossy(&buffer[..read])
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned()
}

fn review_settings(browser: &mut Browser) {
    browser.click("#settings-review");
    browser.wait("document.querySelector('dialog.config-dialog[open]') !== null");
}

fn apply_settings(browser: &mut Browser) {
    browser.click("dialog.config-dialog .dialog-actions .button-primary");
}

/// Opens SCR-005, replaces the deny list with `value`, confirms, and waits for the settings view.
fn save_deny_list(browser: &mut Browser, server: &Server, value: &str) {
    settings::open_settings(browser, server);
    settings::open_settings_edit(browser, server);
    settings::set_field_value(browser, DENY, value);
    review_settings(browser);
    apply_settings(browser);
    browser.wait(
        "location.hash === '#/settings' && document.querySelector('#settings-edit') !== null && document.querySelector('#settings-refresh')?.disabled === false",
    );
}

// DASH-089 with a real configuration: the dialog sends nothing until it is confirmed, one GET and one PATCH change the
// file, the address stops connecting, and the screen queries again. DASH-090 (a) unblocks it again.
pub fn block_and_unblock_one_address() {
    const IP: &str = "198.51.100.7";
    let server = Server::start_with_config_transform("dash-089", Some(1), true, with_databases);
    let admin = server.login();
    connect_clients(&server, &admin);
    assert!(
        ws_reply_line(&server, IP).starts_with("HTTP/1.1 101"),
        "the address connects before it is blocked"
    );
    let mut browser = Browser::start("dash-089", &["ko-KR"]);
    browser.login(&server);
    browser.navigate(&server.url("/#/sources"));
    ready(&mut browser);
    let report = server.query(&admin, ENTRY);
    assert_eq!(report["deny_editable"], json!(true));
    assert!(
        report["addresses"]["items"]
            .as_array()
            .into_iter()
            .flatten()
            .all(|item| item["deny_entries"] == json!([])),
        "nothing is blocked yet"
    );
    assert_eq!(deny_value(&server, &admin), json!([]));
    assert_eq!(
        browser
            .eval("document.querySelectorAll('[data-sources-card=\"addresses\"] thead th').length"),
        json!(9),
        "the address table has the block status column"
    );
    assert_eq!(
        deny_cell(&mut browser, IP),
        json!({"badge": null, "button": "block", "note": null, "empty": false})
    );
    assert_eq!(
        deny_cell(&mut browser, "127.0.0.1"),
        json!({"badge": null, "button": null, "note": null, "empty": true}),
        "a local address that is not blocked has an empty cell"
    );

    // Opening the dialog sends nothing. Cancel and Escape close it and return the focus to the button.
    browser.click(&deny_button(IP, "block"));
    browser.wait("document.querySelector('dialog.dialog[open]') !== null");
    assert_eq!(
        browser.eval("document.activeElement?.textContent.trim()"),
        json!("취소")
    );
    assert_eq!(
        browser.eval("document.querySelector('dialog.dialog p.mono').textContent"),
        json!(IP)
    );
    browser.key("Escape");
    browser.wait("document.querySelector('dialog.dialog[open]') === null");
    assert_eq!(
        browser.eval("document.activeElement?.dataset.denyAddress"),
        json!(IP)
    );
    browser.click(&deny_button(IP, "block"));
    browser.wait("document.querySelector('dialog.dialog[open]') !== null");
    browser.click("dialog.dialog .dialog-actions button:first-child");
    browser.wait("document.querySelector('dialog.dialog[open]') === null");
    assert_eq!(
        browser.eval("document.activeElement?.dataset.denyAddress"),
        json!(IP)
    );
    assert_eq!(server.proxy.count("GET", "/api/v1/config"), 0);
    assert_eq!(server.proxy.count("PATCH", "/api/v1/config"), 0);

    // Confirm: one GET, one PATCH with the CSRF token, then a query without X-Relay-Poll.
    let before = requests(&server).len();
    browser.click(&deny_button(IP, "block"));
    browser.wait("document.querySelector('dialog.dialog[open]') !== null");
    browser.click("dialog.dialog .dialog-actions button:last-child");
    wait_for_deny_button(&mut browser, IP, "unblock");
    assert_eq!(server.proxy.count("GET", "/api/v1/config"), 1);
    assert_eq!(server.proxy.count("PATCH", "/api/v1/config"), 1);
    let patch = server
        .proxy
        .traffic()
        .into_iter()
        .find(|r| r.method == "PATCH" && r.path == "/api/v1/config")
        .unwrap_or_else(|| panic!("the PATCH went through the proxy"));
    assert_eq!(patch.status, Some(200));
    assert!(patch.headers.to_ascii_lowercase().contains("x-csrf-token:"));
    assert_eq!(deny_value(&server, &admin), json!([IP]));
    let text = server.config_text();
    assert!(
        text.split("[relay.deny]")
            .nth(1)
            .is_some_and(|rest| rest.contains(IP)),
        "the file has the entry under [relay.deny]: {text}"
    );
    assert_eq!(address_entries(&server, &admin, IP), json!([IP]));
    assert_eq!(
        browser.eval("document.querySelector('.sources-result .alert')?.textContent"),
        json!("주소를 차단했습니다: 198.51.100.7")
    );
    assert_eq!(
        browser.eval("document.querySelector('.sources-result .alert')?.getAttribute('role')"),
        json!("status")
    );
    assert_eq!(
        deny_cell(&mut browser, IP),
        json!({"badge": "차단됨", "button": "unblock", "note": null, "empty": false})
    );
    assert!(
        requests(&server)
            .iter()
            .skip(before)
            .any(|(path, headers)| path == ENTRY && !is_poll(headers)),
        "the screen queried again as a user action"
    );
    assert!(
        !ws_reply_line(&server, IP).starts_with("HTTP/1.1 101"),
        "the blocked address no longer connects"
    );

    // Unblock removes the row's entry.
    browser.click(&deny_button(IP, "unblock"));
    browser.wait("document.querySelector('dialog.dialog[open]') !== null");
    assert_eq!(
        browser.eval("document.querySelector('dialog.dialog .dialog-title').textContent"),
        json!("차단 해제 확인")
    );
    browser.click("dialog.dialog .dialog-actions button:last-child");
    wait_for_deny_button(&mut browser, IP, "block");
    assert_eq!(server.proxy.count("PATCH", "/api/v1/config"), 2);
    assert_eq!(deny_value(&server, &admin), json!([]));
    assert_eq!(
        browser.eval("document.querySelector('.sources-result .alert')?.textContent"),
        json!("차단을 해제했습니다: 198.51.100.7")
    );
    assert!(
        ws_reply_line(&server, IP).starts_with("HTTP/1.1 101"),
        "the unblocked address connects again"
    );
}

// DASH-090 (b), (c) and (d) with real values: a CIDR entry written through SCR-005 makes a badge with the range note
// and no button, a local address has no button, and a server with editing off has no buttons and the CPY-302 line.
pub fn range_entries_local_rows_and_edit_off() {
    const RANGE_NOTE: &str = "범위 항목으로 차단되어 있습니다. 설정에서 해제하십시오.";
    const EDIT_OFF: &str = "차단하려면 설정 파일에서 management.config_edit = true로 바꾸고 VNC Relay를 재시작하십시오.";
    let server = Server::start_with_config_transform("dash-090", Some(1), true, with_databases);
    let admin = server.login();
    connect_clients(&server, &admin);
    let mut browser = Browser::start("dash-090", &["ko-KR"]);
    browser.login(&server);
    save_deny_list(&mut browser, &server, "198.51.100.0/24");
    assert_eq!(deny_value(&server, &admin), json!(["198.51.100.0/24"]));
    assert_eq!(
        address_entries(&server, &admin, "198.51.100.7"),
        json!(["198.51.100.0/24"])
    );
    browser.click(MENU);
    ready(&mut browser);
    assert_eq!(
        deny_cell(&mut browser, "198.51.100.7"),
        json!({"badge": "차단됨", "button": null, "note": RANGE_NOTE, "empty": false}),
        "a range entry shows the badge and the note, never a button"
    );
    assert_eq!(
        deny_cell(&mut browser, "2001:db8::7"),
        json!({"badge": null, "button": "block", "note": null, "empty": false})
    );
    assert_eq!(
        deny_cell(&mut browser, "127.0.0.1"),
        json!({"badge": null, "button": null, "note": null, "empty": true})
    );
    assert_eq!(
        browser.eval(
            "[...document.querySelectorAll('[data-sources-card=\"addresses\"] .card-head .card-description')].map(p => p.textContent).includes('차단하려면 설정 파일에서 management.config_edit = true로 바꾸고 VNC Relay를 재시작하십시오.')"
        ),
        json!(false),
        "with editing on there is no CPY-302"
    );

    let off = Server::start_with_config_transform("dash-090-off", Some(1), false, with_databases);
    let off_admin = off.login();
    connect_clients(&off, &off_admin);
    let mut off_browser = Browser::start("dash-090-off", &["ko-KR"]);
    off_browser.login(&off);
    off_browser.navigate(&off.url("/#/sources"));
    ready(&mut off_browser);
    assert_eq!(off.query(&off_admin, ENTRY)["deny_editable"], json!(false));
    assert_eq!(
        off_browser.eval("document.querySelectorAll('[data-deny-action]').length"),
        json!(0)
    );
    assert_eq!(
        off_browser.eval(
            "[...document.querySelectorAll('[data-sources-card=\"addresses\"] .card-head .card-description')].map(p => p.textContent).at(-1)"
        ),
        json!(EDIT_OFF)
    );
}

// DASH-091 with a real server: SCR-004 shows the key in its own table, SCR-005 has the input with its label and help,
// the confirmation warns only for new or changed entries, and the server's rejections map to the right line.
pub fn deny_list_input_warning_and_server_diagnostics() {
    let server = Server::start_with_config_transform("dash-091", Some(1), true, with_databases);
    let admin = server.login();
    let mut browser = Browser::start("dash-091", &["ko-KR"]);
    browser.login(&server);
    settings::open_settings(&mut browser, &server);
    assert_eq!(
        browser.eval(
            r#"(() => {
                const row = document.querySelector('[data-config-key="relay.deny.sources"]');
                return [row?.closest('.card')?.querySelector('.card-title')?.textContent, row?.textContent.includes('재시작 없이 적용'), row?.textContent.includes('빈 목록')];
            })()"#
        ),
        json!(["[relay.deny]", true, true]),
        "SCR-004 shows the key in [relay.deny] as applying without restart, empty"
    );
    settings::open_settings_edit(&mut browser, &server);
    assert_eq!(
        browser.eval(
            r#"(() => {
                const input = document.querySelector('[data-config-key="relay.deny.sources"]');
                const label = [...document.querySelectorAll('label')].find(l => l.htmlFor === input.id);
                const help = input.getAttribute('aria-describedby').split(/\s+/).map(id => document.getElementById(id)?.textContent ?? '');
                return [
                    input.closest('.card').querySelector('.card-title').textContent,
                    label.textContent,
                    help.some(text => text.startsWith('한 줄에 항목 하나를 입력하십시오.')),
                    help.some(text => text.startsWith('허용 목록보다 우선합니다.')),
                ];
            })()"#
        ),
        json!(["접근 정책", "차단 목록relay.deny.sources", true, true])
    );

    // A new entry warns inside its change; Escape closes the confirmation and sends nothing.
    settings::set_field_value(&mut browser, DENY, "198.51.100.0/24");
    review_settings(&mut browser);
    let warnings = r#"(() => {
        const dialog = document.querySelector('dialog.config-dialog');
        const change = [...dialog.querySelectorAll('.config-change')].find(c => c.querySelector('.card-heading')?.textContent === 'relay.deny.sources');
        return [change?.textContent.includes('예외: 차단 목록에 새로 넣거나 바꾼 항목'), dialog.textContent.includes('빈 허용 목록은 해당 키가')];
    })()"#;
    assert_eq!(browser.eval(warnings), json!([true, false]));
    browser.key("Escape");
    browser.wait("document.querySelector('dialog.config-dialog[open]') === null");
    assert_eq!(server.proxy.count("PATCH", "/api/v1/config"), 0);

    // The server rejects the trusted proxy address and a malformed entry, each on its line.
    let original = server.config_text();
    let errors = r#"(document.querySelector('[data-config-key="relay.deny.sources"]').closest('.config-field').querySelector('.config-field-errors')?.textContent ?? '')"#;
    settings::set_field_value(&mut browser, DENY, "127.0.0.1");
    review_settings(&mut browser);
    apply_settings(&mut browser);
    browser.wait(&format!("{errors}.includes('겹칩니다')"));
    assert_eq!(
        browser.eval(&format!("{errors}.includes('1줄: 차단 항목이 신뢰 프록시 주소와 겹칩니다. 프록시 주소는 차단할 수 없습니다.')")),
        json!(true)
    );
    settings::set_field_value(&mut browser, DENY, "198.51.100.0/24\nnot-an-address");
    review_settings(&mut browser);
    apply_settings(&mut browser);
    browser.wait(&format!("{errors}.includes('형식이 올바르지 않습니다')"));
    assert_eq!(
        browser.eval(&format!(
            "{errors}.includes('2줄: IP·CIDR·범위 항목의 형식이 올바르지 않습니다.')"
        )),
        json!(true)
    );
    assert_eq!(
        server.config_text(),
        original,
        "a rejected list changes nothing"
    );
    assert_eq!(deny_value(&server, &admin), json!([]));

    // A valid entry is saved. Removing it again warns nothing, not even for the empty list.
    settings::set_field_value(&mut browser, DENY, "198.51.100.0/24");
    review_settings(&mut browser);
    apply_settings(&mut browser);
    browser.wait(
        "location.hash === '#/settings' && document.querySelector('#settings-edit') !== null",
    );
    assert_eq!(deny_value(&server, &admin), json!(["198.51.100.0/24"]));
    live::wait_for("the watcher applied the saved list", LIMIT, || {
        server.query(&admin, "/api/v1/config")["file"]["state"] != "pending"
    });
    browser.wait("document.body.innerText.includes('설정 파일과 실행 중인 설정이 같습니다')");
    settings::open_settings_edit(&mut browser, &server);
    settings::set_field_value(&mut browser, DENY, "");
    review_settings(&mut browser);
    assert_eq!(browser.eval(warnings), json!([false, false]));
    apply_settings(&mut browser);
    browser.wait(
        "location.hash === '#/settings' && document.querySelector('#settings-edit') !== null",
    );
    assert_eq!(deny_value(&server, &admin), json!([]));
}

// DASH-020 with a real session: saving the deny list closes the open session, and the closed list shows the neutral
// "ip_denied" badge with CPY-323.
pub fn deny_list_closes_an_open_session_with_ip_denied() {
    let server = Server::start_with_config("dash-020-denied", Some(1), true);
    let admin = server.login();
    let pair = server.pair(&admin, "II", "52031");
    let mut browser = Browser::start("dash-020-denied", &["ko-KR"]);
    browser.login(&server);
    save_deny_list(&mut browser, &server, "127.0.0.1");
    live::wait_for("the session closed by the deny list", LIMIT, || {
        server.query(&admin, "/api/v1/closed-sessions")["items"]
            .as_array()
            .is_some_and(|items| {
                items.iter().any(|item| {
                    item["session_id"] == pair.sid && item["close_cause"] == "ip_denied"
                })
            })
    });
    browser.click("a.navbar-item[href='/']");
    browser.wait("document.querySelector('#operations-refresh') !== null");
    browser.wait("[...document.querySelectorAll('.badge')].some(b => b.textContent === '차단 목록으로 종료')");
    assert_eq!(
        browser.eval(
            "(() => { const b = [...document.querySelectorAll('.badge')].find(b => b.textContent === '차단 목록으로 종료'); return [b.dataset.tone, b.querySelector('svg') === null]; })()"
        ),
        json!(["neutral", true]),
        "the cause badge is neutral and text only"
    );
    drop(pair);
}
