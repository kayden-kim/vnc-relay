//! Source IP deny list end to end (ADR-0013, operations.md §3.10): a real
//! `serve` process, file reloads and configuration edits, TCP and WebSocket
//! clients, the operations log, and the management API.
//!
//! The relay sees every test socket from loopback, so TCP checks deny
//! `127.0.0.1`. WebSocket checks go through the trusted loopback proxy and
//! use documentation addresses (RFC 5737, RFC 3849).

use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};

use serde_json::{Value, json};
use vnc_relay_testkit::live::{self, Admin, LIMIT, Reply, Server};

/// Trusts the loopback proxy on the WS listener and allows the documentation
/// clients on the WS Viewer path.
fn proxied(config: String) -> String {
    config
        .replace(
            "[relay.ws]\n",
            "[relay.ws]\ntrusted_proxies = [\"127.0.0.1\"]\n",
        )
        .replace(
            "allow = [\"127.0.0.1\"]\nallowed_origins",
            "allow = [\"127.0.0.1\", \"198.51.100.0/24\", \"2001:db8::/32\"]\nallowed_origins",
        )
}

fn log_events(server: &Server) -> Vec<Value> {
    fs::read_to_string(server.dir().join("events.jsonl"))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// Waits for a log line with every field of `fields`.
fn wait_for_event(server: &Server, label: &str, fields: &Value) -> Value {
    let wanted = fields
        .as_object()
        .unwrap_or_else(|| panic!("event fields are an object"));
    let mut found = Value::Null;
    live::wait_for(label, LIMIT, || {
        if let Some(event) = log_events(server)
            .into_iter()
            .find(|event| wanted.iter().all(|(key, value)| &event[key] == value))
        {
            found = event;
        }
        !found.is_null()
    });
    found
}

/// Appends a `[relay.deny]` table to the running file; the watcher applies it.
fn deny_by_file(server: &Server, sources: &str) {
    server.config_write(format!(
        "{}\n[relay.deny]\nsources = {sources}\n",
        server.config_text()
    ));
    wait_for_event(
        server,
        "deny list reload applied",
        &json!({"event": "reload_applied", "applied": ["relay.deny.sources"]}),
    );
}

/// The peer closed the connection without sending any byte.
fn assert_closed_silently(mut stream: TcpStream) {
    live::must(stream.set_read_timeout(Some(LIMIT)));
    let mut byte = [0; 1];
    assert_eq!(
        stream.read(&mut byte).unwrap_or(0),
        0,
        "closed without a reply"
    );
}

/// Opens a WebSocket Viewer through the trusted loopback proxy for `client`.
/// Returns the status and, after `101`, the open stream.
fn ws_from(server: &Server, client: &str) -> (u16, TcpStream) {
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
    let status = String::from_utf8_lossy(&head)
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("status line"));
    (status, stream)
}

/// One server frame: opcode and payload. Server frames are never masked.
fn read_frame(stream: &mut TcpStream) -> (u8, Vec<u8>) {
    let mut head = [0; 2];
    live::must(stream.read_exact(&mut head));
    let len = match head[1] & 0x7f {
        126 => {
            let mut bytes = [0; 2];
            live::must(stream.read_exact(&mut bytes));
            usize::from(u16::from_be_bytes(bytes))
        }
        127 => {
            let mut bytes = [0; 8];
            live::must(stream.read_exact(&mut bytes));
            usize::try_from(u64::from_be_bytes(bytes)).unwrap_or_else(|_| panic!("frame size"))
        }
        len => usize::from(len),
    };
    let mut payload = vec![0; len];
    live::must(stream.read_exact(&mut payload));
    (head[0] & 0x0f, payload)
}

/// Sends the initial request in one masked binary frame (a zero mask key).
fn ws_request(stream: &mut TcpStream, request: &str) {
    let payload = live::initial(request);
    let mut frame = vec![0x82, 0x80 | 126, 0, 250, 0, 0, 0, 0];
    frame.extend_from_slice(&payload);
    live::must(stream.write_all(&frame));
}

/// Opens a WS Viewer for `client` and sends `ID:<id>` after the greeting.
fn ws_viewer(server: &Server, client: &str, id: &str) -> TcpStream {
    let (status, mut stream) = ws_from(server, client);
    assert_eq!(status, 101, "{client} upgrades");
    live::must(stream.set_read_timeout(Some(LIMIT)));
    let (opcode, greeting) = read_frame(&mut stream);
    assert_eq!((opcode, greeting.as_slice()), (2, &b"RFB 000.000\n"[..]));
    ws_request(&mut stream, &format!("ID:{id}"));
    stream
}

/// Reads frames until the Close frame and returns its code and reason.
fn close_of(stream: &mut TcpStream) -> (u16, String) {
    loop {
        let (opcode, payload) = read_frame(stream);
        if opcode == 8 {
            assert!(payload.len() >= 2, "close frame carries a code");
            return (
                u16::from_be_bytes([payload[0], payload[1]]),
                String::from_utf8_lossy(&payload[2..]).into_owned(),
            );
        }
    }
}

fn waiting_ids(server: &Server, admin: &Admin) -> Vec<String> {
    server.query(admin, "/api/v1/waiting")["items"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item["relay_id"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// A management session with its CSRF token.
struct Editor {
    cookie: String,
    csrf: String,
}

fn sign_in(server: &Server) -> Editor {
    let body = json!({"account": "alice", "password": live::PASSWORD}).to_string();
    let reply = server.request(
        "POST",
        "/api/v1/session",
        &[("Content-Type", "application/json")],
        body.as_bytes(),
    );
    assert_eq!(reply.status, 200, "login: {}", reply.body);
    let cookie = reply
        .headers
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("set-cookie"))
        .and_then(|(_, value)| value.trim().split(';').next())
        .unwrap_or_else(|| panic!("login cookie"))
        .to_owned();
    let view: Value = live::must(serde_json::from_str(&reply.body));
    let csrf = view["csrf_token"]
        .as_str()
        .unwrap_or_else(|| panic!("csrf token"))
        .to_owned();
    Editor { cookie, csrf }
}

fn get(server: &Server, editor: &Editor, path: &str) -> Value {
    let reply = server.request("GET", path, &[("Cookie", editor.cookie.as_str())], b"");
    assert_eq!(reply.status, 200, "{path}: {}", reply.body);
    live::must(serde_json::from_str(&reply.body))
}

fn patch(server: &Server, editor: &Editor, changes: Value) -> Reply {
    let config = get(server, editor, "/api/v1/config");
    let revision = config["revision"]
        .as_str()
        .or_else(|| config["file"]["revision"].as_str())
        .unwrap_or_else(|| panic!("config revision: {config}"))
        .to_owned();
    let body = json!({"revision": revision, "changes": changes}).to_string();
    server.request(
        "PATCH",
        "/api/v1/config",
        &[
            ("Cookie", editor.cookie.as_str()),
            ("X-CSRF-Token", editor.csrf.as_str()),
            ("Content-Type", "application/json"),
        ],
        body.as_bytes(),
    )
}

fn address<'a>(sources: &'a Value, ip: &str) -> Option<&'a Value> {
    sources["addresses"]["items"]
        .as_array()?
        .iter()
        .find(|item| item["ip"] == ip)
}

/// spec: relay.md:RLY-033 partial
/// spec: relay.md:RLY-034 partial
/// spec: operations.md:OPS-101 partial
/// spec: websocket.md:WS-047 partial
#[test]
fn file_reload_closes_denied_tcp_connections_and_refuses_new_ones_at_accept() {
    let server = Server::start("deny-tcp", None);
    let admin = server.login();
    let _waiting = server.waiting("4101");
    live::wait_for("TCP Viewer waiting", LIMIT, || {
        waiting_ids(&server, &admin).contains(&"4101".to_owned())
    });
    let _pair = server.pair(&admin, "II", "4102");

    // The deny list overrides the TCP allow list (`127.0.0.1`).
    deny_by_file(&server, "[\"127.0.0.1\"]");
    let removed = wait_for_event(
        &server,
        "waiting connection removed",
        &json!({"event": "waiting_removed", "reason": "ip_denied"}),
    );
    assert_eq!(removed["level"], "info");
    let closed = wait_for_event(
        &server,
        "active session closed",
        &json!({"event": "session_closed", "close_cause": "ip_denied"}),
    );
    assert_eq!(closed["level"], "info");
    assert!(closed.get("side").is_none(), "{closed}");
    live::wait_for("registries emptied", LIMIT, || {
        waiting_ids(&server, &admin).is_empty()
            && server.query(&admin, "/api/v1/sessions")["items"]
                .as_array()
                .is_some_and(Vec::is_empty)
    });

    // New TCP connections close before any byte, with an accept-stage reason.
    assert_closed_silently(live::connect(server.ports[2]));
    let rejected = wait_for_event(
        &server,
        "TCP accept refused",
        &json!({"event": "connection_rejected", "listener": "tcp_viewer", "stage": "accept", "reason": "ip_denied"}),
    );
    assert_eq!(rejected["peer_ip"], "127.0.0.1");
    assert_eq!(rejected["level"], "warn");

    // A direct WS peer outside `trusted_proxies` closes before the HTTP answer.
    let mut ws = live::connect(server.ws_port);
    let _ = ws.write_all(b"GET /relay/viewer HTTP/1.1\r\nHost: localhost\r\n\r\n");
    assert_closed_silently(ws);
    wait_for_event(
        &server,
        "WS accept refused",
        &json!({"event": "connection_rejected", "listener": "ws", "stage": "accept", "reason": "ip_denied"}),
    );
}

/// spec: websocket.md:WS-047 partial
/// spec: websocket.md:WS-048 partial
/// spec: relay.md:RLY-034 partial
#[test]
fn websocket_clients_of_a_new_deny_item_close_with_4014_and_new_upgrades_get_403() {
    let server =
        Server::start_with_config_transform("deny-ws", None, false, |config, _| proxied(config));
    let admin = server.login();
    let mut waiting = ws_viewer(&server, "198.51.100.8", "4201");
    live::wait_for("WS Viewer waiting", LIMIT, || {
        waiting_ids(&server, &admin).contains(&"4201".to_owned())
    });
    let mut active = ws_viewer(&server, "198.51.100.9", "4202");
    let mut target = live::connect(server.ports[3]);
    live::must(target.write_all(&live::initial("ID:4202")));
    live::wait_for("WS session active", LIMIT, || {
        server.query(&admin, "/api/v1/sessions")["items"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    });

    deny_by_file(&server, "[\"198.51.100.0/24\"]");
    assert_eq!(close_of(&mut waiting), (4014, "ip_denied".to_owned()));
    assert_eq!(close_of(&mut active), (4014, "ip_denied".to_owned()));
    wait_for_event(
        &server,
        "WS session closed",
        &json!({"event": "session_closed", "close_cause": "ip_denied"}),
    );

    // The forwarded client is denied before the role allow check: 403.
    let (status, _) = ws_from(&server, "198.51.100.10");
    assert_eq!(status, 403);
    let rejected = wait_for_event(
        &server,
        "upgrade refused",
        &json!({"event": "connection_rejected", "stage": "upgrade", "reason": "ip_denied"}),
    );
    assert_eq!(rejected["forwarded_ip"], "198.51.100.10");
    assert_eq!(rejected["peer_ip"], "127.0.0.1");
    // A client outside the list still upgrades through the same proxy.
    let (status, _) = ws_from(&server, "2001:db8::5");
    assert_eq!(status, 101);
}

/// spec: management.md:MGT-065 partial
/// spec: management.md:MGT-066 partial
#[test]
fn deny_items_are_edited_through_the_api_and_listed_per_source_address() {
    let server =
        Server::start_with_config_transform("deny-edit", None, true, |config, _| proxied(config));
    let editor = sign_in(&server);
    assert!(!server.config_text().contains("[relay.deny]"));

    let applied = patch(
        &server,
        &editor,
        json!({"relay.deny.sources": ["198.51.100.7", "198.51.100.0/24"]}),
    );
    assert_eq!(applied.status, 200, "{}", applied.body);
    let body: Value = live::must(serde_json::from_str(&applied.body));
    assert_eq!(body["applied"], json!(["relay.deny.sources"]));
    assert!(
        server
            .config_text()
            .contains("[relay.deny]\nsources = [\"198.51.100.7\", \"198.51.100.0/24\"]\n"),
        "{}",
        server.config_text()
    );
    let config = get(&server, &editor, "/api/v1/config");
    let item = config["items"]
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|item| item["key"] == "relay.deny.sources")
        })
        .unwrap_or_else(|| panic!("deny item"));
    assert_eq!(item["value"], json!(["198.51.100.7", "198.51.100.0/24"]));
    assert_eq!(item["editable"], true);

    assert_eq!(ws_from(&server, "198.51.100.7").0, 403);
    assert_eq!(ws_from(&server, "2001:db8::5").0, 101);
    let mut sources = Value::Null;
    live::wait_for("both addresses counted", LIMIT, || {
        sources = get(&server, &editor, "/api/v1/sources");
        address(&sources, "198.51.100.7").is_some() && address(&sources, "2001:db8::5").is_some()
    });
    assert_eq!(sources["deny_editable"], true);
    assert_eq!(
        address(&sources, "198.51.100.7").map(|item| item["deny_entries"].clone()),
        Some(json!(["198.51.100.7", "198.51.100.0/24"]))
    );
    assert_eq!(
        address(&sources, "2001:db8::5").map(|item| item["deny_entries"].clone()),
        Some(json!([]))
    );

    // An item that overlaps the enabled WS listener's trusted proxy fails.
    let overlap = patch(
        &server,
        &editor,
        json!({"relay.deny.sources": ["198.51.100.7", "127.0.0.0/8"]}),
    );
    assert_eq!(overlap.status, 400, "{}", overlap.body);
    let body: Value = live::must(serde_json::from_str(&overlap.body));
    assert_eq!(body["error"]["code"], "config_invalid");
    assert_eq!(
        body["error"]["diagnostics"][0]["code"],
        "config_deny_overlaps_proxy"
    );
    assert_eq!(
        body["error"]["diagnostics"][0]["key"],
        "relay.deny.sources[1]"
    );
    assert!(
        server
            .config_text()
            .contains("sources = [\"198.51.100.7\", \"198.51.100.0/24\"]")
    );

    // With editing off the list is read-only.
    let read_only = Server::start("deny-read-only", None);
    let viewer = sign_in(&read_only);
    assert_eq!(
        get(&read_only, &viewer, "/api/v1/sources")["deny_editable"],
        false
    );
    let refused = patch(
        &read_only,
        &viewer,
        json!({"relay.deny.sources": ["198.51.100.7"]}),
    );
    assert_eq!(refused.status, 403, "{}", refused.body);
    let body: Value = live::must(serde_json::from_str(&refused.body));
    assert_eq!(body["error"]["code"], "config_edit_disabled");
}

/// spec: operations.md:OPS-100 partial
#[test]
fn startup_exits_3_when_a_deny_item_overlaps_an_enabled_trusted_proxy() {
    let dir = std::env::temp_dir().join(format!("vnc-relay-deny-start-{}", std::process::id()));
    live::must(fs::create_dir_all(&dir));
    live::must(fs::write(
        dir.join("relay.toml"),
        r#"
[admin]
accounts_file = "accounts.toml"
[management]
listen = "127.0.0.1:18081"
[status]
listen = "127.0.0.1:18082"
[log]
output = "events.jsonl"
[relay.ws]
listen = "127.0.0.1:18090"
trusted_proxies = ["192.0.2.10"]
[relay.ws.viewer]
enabled = true
allow = ["198.51.100.0/24"]
allowed_origins = ["http://127.0.0.1"]
[relay.deny]
sources = ["203.0.113.9", "192.0.2.0/24"]
"#,
    ));
    let output = live::must(
        Command::new(env!("CARGO_BIN_EXE_vnc-relay"))
            .args(["serve", "--config"])
            .arg(dir.join("relay.toml"))
            .stdin(Stdio::null())
            .output(),
    );
    let _ = fs::remove_dir_all(&dir);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(
        stderr.contains("error[config_deny_overlaps_proxy]")
            && stderr.contains("relay.deny.sources[1]"),
        "{stderr}"
    );
    assert!(!stderr.contains("relay.deny.sources[0]"), "{stderr}");
}
