//! Actual Mode I / Mode II TCP, WS and WSS streams and trusted-proxy clients.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::Arc;

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::{self, Message, WebSocket, client::IntoClientRequest};

use vnc_relay_testkit::cdp::Browser;
use vnc_relay_testkit::live::{self, Admin, LIMIT, Server};

fn fixture(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("crates/adapters/tests/fixtures/tls")
        .join(name)
        .to_string_lossy()
        .replace('\\', "/")
}

enum Wire {
    Tcp(TcpStream),
    Tls(Box<StreamOwned<ClientConnection, TcpStream>>),
}
impl Read for Wire {
    fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Tcp(s) => s.read(b),
            Self::Tls(s) => s.read(b),
        }
    }
}
impl Write for Wire {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Tcp(s) => s.write(b),
            Self::Tls(s) => s.write(b),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Tcp(s) => s.flush(),
            Self::Tls(s) => s.flush(),
        }
    }
}

enum Endpoint {
    Tcp(TcpStream),
    Ws(Box<WebSocket<Wire>>),
}
impl Endpoint {
    fn open(server: &Server, wss: u16, transport: &str, viewer: bool, forwarded: bool) -> Self {
        if transport == "tcp" {
            return Self::Tcp(if viewer {
                server.handshaking()
            } else {
                live::connect(server.ports[3])
            });
        }
        let port = if transport == "wss" {
            wss
        } else {
            server.ws_port
        };
        let socket = live::connect(port);
        let wire = if transport == "wss" {
            let mut roots = RootCertStore::empty();
            for cert in live::must(CertificateDer::pem_file_iter(fixture("root-ca.pem"))) {
                live::must(roots.add(live::must(cert)));
            }
            let config = live::must(
                ClientConfig::builder_with_provider(Arc::new(
                    rustls::crypto::aws_lc_rs::default_provider(),
                ))
                .with_safe_default_protocol_versions(),
            )
            .with_root_certificates(roots)
            .with_no_client_auth();
            Wire::Tls(Box::new(StreamOwned::new(
                live::must(ClientConnection::new(
                    Arc::new(config),
                    live::must(ServerName::try_from("localhost")),
                )),
                socket,
            )))
        } else {
            Wire::Tcp(socket)
        };
        let path = if viewer {
            "/relay/viewer"
        } else {
            "/relay/server"
        };
        let mut request = live::must(format!("ws://localhost:{port}{path}").into_client_request());
        if forwarded {
            request.headers_mut().insert(
                "X-Forwarded-For",
                live::must(if viewer { "192.0.2.41" } else { "192.0.2.42" }.parse()),
            );
        }
        let (ws, response) = live::must(tungstenite::client(request, wire));
        assert_eq!(response.status().as_u16(), 101);
        let mut endpoint = Self::Ws(Box::new(ws));
        if viewer {
            assert_eq!(endpoint.receive(12), b"RFB 000.000\n");
        }
        endpoint
    }
    fn send(&mut self, data: &[u8]) {
        match self {
            Self::Tcp(s) => live::must(s.write_all(data)),
            Self::Ws(s) => live::must(s.send(Message::Binary(data.to_vec().into()))),
        }
    }
    fn receive(&mut self, len: usize) -> Vec<u8> {
        match self {
            Self::Tcp(s) => {
                let mut data = vec![0; len];
                live::must(s.read_exact(&mut data));
                data
            }
            Self::Ws(s) => {
                let mut bytes = Vec::with_capacity(len);
                while bytes.len() < len {
                    match live::must(s.read()) {
                        Message::Binary(data) => bytes.extend_from_slice(&data),
                        Message::Ping(_) => live::must(s.flush()),
                        m => panic!("unexpected relay message {m:?}"),
                    }
                }
                assert_eq!(bytes.len(), len, "exact relay stream bytes");
                bytes
            }
        }
    }
}

fn assert_screens(browser: &mut Browser, server: &Server, session: &Value) {
    browser.navigate(&server.url("/"));
    browser.wait("document.querySelector('#operations-refresh')?.disabled===false");
    browser.wait(&format!(
        "[...document.querySelectorAll('a[href^=\"/#/sessions/\"]')].some(a=>a.textContent==={})",
        session["session_id"]
    ));
    super::overview::assert_real_transport(browser, session);
    browser.navigate(&server.url(&format!(
        "/#/sessions/{}",
        session["session_id"].as_str().unwrap_or("")
    )));
    browser.wait("document.querySelector('#session-terminate')?.disabled===false");
    super::detail::assert_real_transport_and_addresses(browser, session);
}

fn pair(
    server: &Server,
    admin: &Admin,
    browser: &mut Browser,
    wss: u16,
    transports: (&str, &str, [bool; 2]),
    id: &str,
) {
    let (viewer_transport, target_transport, forwarded) = transports;
    let mut target = Endpoint::open(server, wss, target_transport, false, forwarded[1]);
    target.send(&live::initial(&format!("ID:{id}")));
    live::wait_for("real matrix server waiting", LIMIT, || {
        server.query(admin, "/api/v1/waiting")["items"]
            .as_array()
            .is_some_and(|items| items.iter().any(|i| i["relay_id"] == id))
    });
    let mut viewer = Endpoint::open(server, wss, viewer_transport, true, forwarded[0]);
    viewer.send(&live::initial(&format!("ID:{id}")));
    let mut session = Value::Null;
    live::wait_for("real matrix pair active", LIMIT, || {
        session = server.query(admin, "/api/v1/sessions")["items"]
            .as_array()
            .and_then(|items| {
                if items.len() == 1 {
                    items.first()
                } else {
                    None
                }
            })
            .cloned()
            .unwrap_or(Value::Null);
        !session.is_null()
    });
    viewer.send(b"viewer-bytes");
    assert_eq!(target.receive(12), b"viewer-bytes");
    target.send(b"server-bytes");
    assert_eq!(viewer.receive(12), b"server-bytes");
    assert_eq!(session["viewer_transport"], viewer_transport);
    assert_eq!(session["target_transport"], target_transport);
    for (index, key) in ["viewer_forwarded_ip", "target_forwarded_ip"]
        .into_iter()
        .enumerate()
    {
        assert_eq!(
            session[key],
            if forwarded[index] {
                json!(if index == 0 {
                    "192.0.2.41"
                } else {
                    "192.0.2.42"
                })
            } else {
                Value::Null
            }
        );
    }
    let detail = server.query(
        admin,
        &format!(
            "/api/v1/sessions/{}",
            session["session_id"].as_str().unwrap_or("")
        ),
    );
    for field in [
        "viewer_transport",
        "target_transport",
        "viewer_address",
        "target_address",
        "viewer_forwarded_ip",
        "target_forwarded_ip",
    ] {
        assert_eq!(detail["session"][field], session[field]);
    }
    assert_screens(browser, server, &detail["session"]);
    drop(viewer);
    drop(target);
    live::wait_for("matrix relay released", LIMIT, || {
        server.query(admin, "/api/v1/sessions")["items"]
            .as_array()
            .is_some_and(|items| items.is_empty())
    });
}

/// Starts a relay with WS and WSS Viewer and Server listeners. `trust` turns on the trusted proxy for WS and for
/// WSS. `extra` is appended to the configuration. Returns the server and the WSS port.
fn start(name: &str, trust: [bool; 2], extra: &str) -> (Server, u16) {
    let reservation = live::must(TcpListener::bind("127.0.0.1:0"));
    let wss = live::must(reservation.local_addr()).port();
    let server = Server::start_with_config_transform(name, Some(60), false, |config, _| {
        drop(reservation);
        let proxies = |enabled| if enabled { "[\"127.0.0.1\"]" } else { "[]" };
        config
            .replace(
                "allow = [\"127.0.0.1\"]",
                "allow = [\"127.0.0.1\", \"192.0.2.0/24\"]",
            )
            .replace(
                "[relay.ws]\n",
                &format!("[relay.ws]\ntrusted_proxies = {}\n", proxies(trust[0])),
            )
            + &format!(
                "\n[relay.ws.server]\nenabled = true\nallow = [\"127.0.0.1\", \"192.0.2.0/24\"]\n[relay.wss]\nlisten = \"127.0.0.1:{wss}\"\ntrusted_proxies = {}\ncert_file = {:?}\nkey_file = {:?}\n[relay.wss.viewer]\nenabled = true\nallow = [\"127.0.0.1\", \"192.0.2.0/24\"]\n[relay.wss.server]\nenabled = true\nallow = [\"127.0.0.1\", \"192.0.2.0/24\"]\n",
                proxies(trust[1]),
                fixture("server-chain.pem"),
                fixture("server-key-p256.pem")
            )
            + extra
    });
    (server, wss)
}

pub fn real_transport_matrix_and_forwarded_addresses() {
    for trust in [[false, false], [true, false], [false, true], [true, true]] {
        let (server, wss) = start("dash-062", trust, "");
        let admin = server.login();
        let mut browser = Browser::start("dash-062", &["en-US"]);
        browser.login(&server);
        if trust == [false, false] {
            let mode1 = server.pair(&admin, "I", "46200");
            let raw = server.query(&admin, &format!("/api/v1/sessions/{}", mode1.sid));
            assert_eq!(raw["session"]["target_transport"], "tcp");
            assert_screens(&mut browser, &server, &raw["session"]);
            mode1.close();
            for (i, v) in ["tcp", "ws", "wss"].into_iter().enumerate() {
                for (j, t) in ["tcp", "ws", "wss"].into_iter().enumerate() {
                    pair(
                        &server,
                        &admin,
                        &mut browser,
                        wss,
                        (v, t, [false, false]),
                        &format!("462{}{}", i, j),
                    );
                }
            }
        } else {
            pair(
                &server,
                &admin,
                &mut browser,
                wss,
                ("ws", "wss", trust),
                "46299",
            );
        }
    }
}

fn list(server: &Server, admin: &Admin, path: &str) -> Value {
    server.query(admin, path)["items"].clone()
}

fn count(server: &Server, admin: &Admin, path: &str) -> usize {
    list(server, admin, path).as_array().map_or(0, Vec::len)
}

/// Pairs a waiting Server endpoint with a new Viewer endpoint, moves bytes both ways, and closes both sockets.
fn close_mode2(
    server: &Server,
    admin: &Admin,
    wss: u16,
    mut target: Endpoint,
    viewer: (&str, bool),
    id: &str,
) {
    let closed = count(server, admin, "/api/v1/closed-sessions");
    let mut viewer = Endpoint::open(server, wss, viewer.0, true, viewer.1);
    viewer.send(&live::initial(&format!("ID:{id}")));
    live::wait_for("real list pair active", LIMIT, || {
        count(server, admin, "/api/v1/sessions") == 1
    });
    viewer.send(b"viewer-bytes");
    assert_eq!(target.receive(12), b"viewer-bytes");
    target.send(b"server-bytes");
    assert_eq!(viewer.receive(12), b"server-bytes");
    drop(viewer);
    drop(target);
    live::wait_for("real list pair closed", LIMIT, || {
        count(server, admin, "/api/v1/closed-sessions") == closed + 1
    });
}

// Copy from dashboard.md §9.3: CPY-055·056·057·180·181·182·199·200·201 and the day unit.
fn copy(language: &str) -> Value {
    if language == "ko" {
        json!({"started": "시작 시각", "closed": "종료 시각", "duration": "지속 시간", "viewer": "Viewer 주소",
            "target": "Server/대상 주소", "viewerForwarded": "Viewer 전달 주소", "targetForwarded": "Server/대상 전달 주소",
            "logs": "로그 보기", "forwarded": "전달 주소", "day": "일"})
    } else {
        json!({"started": "Started at", "closed": "Closed at", "duration": "Duration", "viewer": "Viewer address",
            "target": "Server/target address", "viewerForwarded": "Viewer forwarded address",
            "targetForwarded": "Server/target forwarded address", "logs": "View logs", "forwarded": "Forwarded address",
            "day": "d"})
    }
}

fn set_language(browser: &mut Browser, language: &str) {
    browser.eval(&format!(
        "(() => {{ const s = document.querySelector('#operations-language'); s.value = '{language}'; s.dispatchEvent(new Event('change', {{bubbles:true}})); }})()"
    ));
    browser.wait(&format!("document.documentElement.lang === '{language}'"));
}

/// Real TCP, WS and WSS waiting entries and closed sessions on SCR-001, with a trusted WS proxy and an untrusted
/// WSS listener, and the closed row's log link chosen with the keyboard.
pub fn real_waiting_and_closed_rows_with_forwarded_addresses() {
    let (server, wss) = start(
        "dash-065",
        [true, false],
        "\n[recent_logs]\nmax_entries = 1000\n",
    );
    let admin = server.login();
    let mut browser = Browser::start("dash-065", &["ko-KR"]);
    browser.login(&server);

    let mut tcp_viewer = Endpoint::open(&server, wss, "tcp", true, false);
    tcp_viewer.send(&live::initial("ID:46501"));
    let mut ws_server = Endpoint::open(&server, wss, "ws", false, true);
    ws_server.send(&live::initial("ID:46502"));
    let mut wss_server = Endpoint::open(&server, wss, "wss", false, false);
    wss_server.send(&live::initial("ID:46503"));
    live::wait_for("three real waiting transports", LIMIT, || {
        count(&server, &admin, "/api/v1/waiting") == 3
    });
    let waiting = list(&server, &admin, "/api/v1/waiting");
    for (id, transport, forwarded) in [
        ("46501", "tcp", Value::Null),
        ("46502", "ws", json!("192.0.2.42")),
        ("46503", "wss", Value::Null),
    ] {
        let item = waiting
            .as_array()
            .and_then(|items| items.iter().find(|item| item["relay_id"] == id))
            .unwrap_or_else(|| panic!("missing waiting {id}: {waiting}"));
        assert_eq!(item["transport"], transport);
        assert_eq!(item["address"]["ip"], "127.0.0.1");
        assert_eq!(item["forwarded_ip"], forwarded);
    }
    browser.navigate(&server.url("/"));
    browser.wait("document.querySelector('#operations-refresh')?.disabled===false");
    browser.wait("document.querySelectorAll('.grid > .card:nth-child(4) tbody tr').length === 3");
    for (language, width) in [
        ("ko", 1280),
        ("en", 1280),
        ("ko", 390),
        ("en", 390),
        ("ko", 320),
    ] {
        set_language(&mut browser, language);
        browser.set_width(width);
        let shown = browser.eval(&format!(
            r#"(() => {{
            const items = {waiting};
            const label = {label};
            const rows = [...document.querySelectorAll('.grid > .card:nth-child(4) tbody tr')];
            const address = a => a.ip.includes(':') ? `[${{a.ip}}]:${{a.port}}` : `${{a.ip}}:${{a.port}}`;
            const lines = cell => [...cell.querySelectorAll('.cell-line')].map(l =>
              [l.querySelector('.cell-label')?.textContent ?? null, l.querySelector('.cell-value')?.textContent ?? null]);
            const page = document.documentElement;
            return page.scrollWidth <= page.clientWidth && rows.length === items.length && items.every(item => {{
              const row = rows.find(r => r.cells[1]?.textContent === item.relay_id);
              const expected = [[null, address(item.address)]]
                .concat(item.forwarded_ip === null ? [] : [[label, item.forwarded_ip]]);
              return !!row && row.cells[2].textContent === item.transport.toUpperCase()
                && JSON.stringify(lines(row.cells[3])) === JSON.stringify(expected);
            }});
            }})()"#,
            label = copy(language)["forwarded"],
        ));
        assert_eq!(
            shown,
            json!(true),
            "waiting rows in {language} at {width}px"
        );
    }

    close_mode2(&server, &admin, wss, ws_server, ("wss", false), "46502");
    close_mode2(&server, &admin, wss, wss_server, ("ws", true), "46503");
    let mode1 = server.pair(&admin, "I", "46504");
    mode1.close();
    live::wait_for("three real closed sessions", LIMIT, || {
        count(&server, &admin, "/api/v1/closed-sessions") == 3
    });
    let closed = list(&server, &admin, "/api/v1/closed-sessions");
    let forwarded: Vec<_> = closed
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    (
                        item["viewer_forwarded_ip"].clone(),
                        item["target_forwarded_ip"].clone(),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    assert!(
        forwarded.contains(&(Value::Null, json!("192.0.2.42"))),
        "{closed}"
    );
    assert!(
        forwarded.contains(&(json!("192.0.2.41"), Value::Null)),
        "{closed}"
    );
    assert!(forwarded.contains(&(Value::Null, Value::Null)), "{closed}");
    browser.navigate(&server.url("/"));
    browser.wait("document.querySelector('#operations-refresh')?.disabled===false");
    browser.wait("document.querySelectorAll('.grid > .card:nth-child(6) tbody tr').length === 3");
    for (language, width) in [
        ("ko", 1280),
        ("en", 1280),
        ("ko", 390),
        ("en", 390),
        ("en", 320),
    ] {
        set_language(&mut browser, language);
        browser.set_width(width);
        let shown = browser.eval(&format!(
            r#"(() => {{
            {local_time}
            const items = {closed};
            const t = {copy};
            const rows = [...document.querySelectorAll('.grid > .card:nth-child(6) tbody tr')];
            const address = a => a.ip.includes(':') ? `[${{a.ip}}]:${{a.port}}` : `${{a.ip}}:${{a.port}}`;
            const lines = cell => [...cell.querySelectorAll('.cell-line')].map(l =>
              [l.querySelector('.cell-label')?.textContent ?? null, l.querySelector('.cell-value')?.textContent ?? null]);
            const pad = n => String(n).padStart(2, '0');
            const length = ms => {{
              const s = Math.floor(ms / 1000);
              const days = Math.floor(s / 86400);
              const clock = `${{pad(Math.floor(s % 86400 / 3600))}}:${{pad(Math.floor(s % 3600 / 60))}}:${{pad(s % 60)}}`;
              return days > 0 ? `${{days}}${{t.day}} ${{clock}}` : clock;
            }};
            const mode = m => m === 'mode1' ? 'Mode I' : m === 'mode2' ? 'Mode II' : m;
            const page = document.documentElement;
            return page.scrollWidth <= page.clientWidth && rows.length === items.length && items.every(item => {{
              const row = rows.find(r => r.cells[0]?.textContent === item.session_id);
              if (!row) return false;
              const peers = [[t.viewer, address(item.viewer_address)]]
                .concat(item.viewer_forwarded_ip === null ? [] : [[t.viewerForwarded, item.viewer_forwarded_ip]])
                .concat([[t.target, address(item.target_address)]])
                .concat(item.target_forwarded_ip === null ? [] : [[t.targetForwarded, item.target_forwarded_ip]]);
              const link = row.cells[7].querySelector('a');
              return row.cells.length === 8 && row.cells[2].textContent === mode(item.mode)
                && JSON.stringify([...row.cells[3].querySelectorAll('.badge')].map(b => b.textContent))
                  === JSON.stringify([item.viewer_transport.toUpperCase(), item.target_transport.toUpperCase()])
                && JSON.stringify(lines(row.cells[4])) === JSON.stringify([[t.started, localTime(item.started_at)],
                  [t.closed, localTime(item.ended_at)], [t.duration, length(item.duration_ms)]])
                && JSON.stringify(lines(row.cells[6])) === JSON.stringify(peers)
                && link?.textContent === t.logs && link.getAttribute('href') === '/#/logs?session_id=' + item.session_id;
            }});
            }})()"#,
            local_time = super::overview::LOCAL_TIME,
            copy = copy(language),
        ));
        assert_eq!(shown, json!(true), "closed rows in {language} at {width}px");
    }

    let sid = closed[0]["session_id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let selector = format!("a[href='/#/logs?session_id={sid}']");
    browser.eval(&format!("document.querySelector(\"{selector}\").focus()"));
    assert_eq!(
        browser.eval("document.activeElement?.getAttribute('href')"),
        json!(format!("/#/logs?session_id={sid}"))
    );
    browser.key("Enter");
    browser.wait(&format!("location.hash === '#/logs?session_id={sid}'"));
    super::logs::ready(&mut browser);
    browser.wait(&format!(
        "[...document.querySelectorAll('.chip')].some(chip => chip.textContent.includes('{sid}'))"
    ));
    assert!(
        server
            .proxy
            .traffic()
            .iter()
            .any(|item| item.path.starts_with("/api/v1/logs?")
                && item.path.contains(&format!("session_id={sid}")))
    );
    assert!(browser.text().contains(&sid));
    drop(tcp_viewer);
}
