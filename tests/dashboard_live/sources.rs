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

use super::live::{Admin, Browser, Fault, LIMIT, Server, connect, must, wait_for};
use super::mmdb::{MmdbWriter, iso_country, maxmind_asn};
use super::settings::{open_settings, open_settings_edit, set_field_value};

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
        .insert("198.51.100.0/24", iso_country("KR"))
        .insert("2001:db8::/32", iso_country("JP"))
        .to_bytes();
    let asn = MmdbWriter::new("GeoLite2-ASN", BUILD_EPOCH)
        .insert("198.51.100.0/24", maxmind_asn(64496, "Example Transit"))
        .to_bytes();
    must(fs::create_dir_all(dir.join("ipdb")));
    must(fs::write(dir.join("ipdb").join("country.mmdb"), country));
    must(fs::write(dir.join("ipdb").join("asn.mmdb"), asn));
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
    let mut stream = connect(server.ws_port);
    let request = format!(
        "GET /relay/viewer HTTP/1.1\r\nHost: localhost\r\nOrigin: http://127.0.0.1\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Protocol: binary\r\nX-Forwarded-For: {client}\r\n\r\n"
    );
    must(stream.write_all(request.as_bytes()));
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        must(stream.read_exact(&mut byte));
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
    wait_for("three source addresses", LIMIT, || {
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
    open_settings(&mut browser, &server);
    open_settings_edit(&mut browser, &server);
    set_field_value(&mut browser, "limits.waiting", "7");
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
    wait_for("real automatic sources poll", LIMIT, || {
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
    wait_for("the 24-hour request", LIMIT, || {
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
    wait_for("sources entry held", LIMIT, || {
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
    wait_for("late sources response released", LIMIT, || {
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
