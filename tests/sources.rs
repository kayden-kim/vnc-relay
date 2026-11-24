//! Source statistics end to end (operations.md §3.9, management-api-v1
//! §1.17): a real `serve` process with generated MMDB files, WebSocket
//! connections through a trusted proxy, and the `/api/v1/sources` query.
//!
//! Addresses are documentation ranges (RFC 5737, RFC 3849) and AS numbers
//! are examples (RFC 5398).

use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};
use vnc_relay_testkit::live::{self, Admin, LIMIT, Reply, Server};
use vnc_relay_testkit::mmdb;
use vnc_relay_testkit::mmdb::MmdbWriter;

const BUILD_EPOCH: u64 = 1_761_955_200; // 2025-11-01T00:00:00Z

/// Allows the documentation clients on the WS Viewer path except 192.0.2.0/24,
/// trusts the loopback proxy, enables metrics, and polls the files quickly.
fn proxied(config: String, ip_info: &str) -> String {
    let config = config
        .replace(
            "[relay.ws]\n",
            "[relay.ws]\ntrusted_proxies = [\"127.0.0.1\"]\n",
        )
        .replace(
            "allow = [\"127.0.0.1\"]\nallowed_origins",
            "allow = [\"127.0.0.1\", \"198.51.100.0/24\", \"2001:db8::/32\"]\nallowed_origins",
        );
    format!("{config}\n[metrics]\nenabled = true\n[reload]\npoll_secs = 5\n{ip_info}")
}

fn country_db(code: &str) -> Vec<u8> {
    MmdbWriter::new("DBIP-Country-Lite", BUILD_EPOCH)
        .insert("198.51.100.0/24", mmdb::iso_country(code))
        .insert("2001:db8::/32", mmdb::iso_country("JP"))
        .to_bytes()
}

fn asn_db() -> Vec<u8> {
    MmdbWriter::new("GeoLite2-ASN", BUILD_EPOCH)
        .insert(
            "198.51.100.0/24",
            mmdb::maxmind_asn(64496, "Example Transit"),
        )
        .insert("192.0.2.0/24", mmdb::maxmind_asn(64511, "Example Edge"))
        .to_bytes()
}

fn write_db(dir: &Path, name: &str, bytes: &[u8]) {
    live::must(fs::create_dir_all(dir.join("ipdb")));
    live::must(fs::write(dir.join("ipdb").join(name), bytes));
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

fn sources_reply(server: &Server, admin: Option<&Admin>, query: &str) -> Reply {
    let mut headers = Vec::new();
    if let Some(admin) = admin {
        headers.push(("Cookie", admin.cookie.as_str()));
    }
    server.request("GET", &format!("/api/v1/sources{query}"), &headers, b"")
}

fn sources(server: &Server, admin: &Admin) -> Value {
    server.query(admin, "/api/v1/sources")
}

fn log_text(server: &Server) -> String {
    fs::read_to_string(server.dir().join("events.jsonl")).unwrap_or_default()
}

fn log_events(server: &Server) -> Vec<Value> {
    log_text(server)
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

fn names(events: &[Value]) -> Vec<&str> {
    events
        .iter()
        .filter_map(|event| event["event"].as_str())
        .collect()
}

/// The startup database events, which come between `server_starting` and `server_ready`.
fn startup_db_events(server: &Server) -> Vec<Value> {
    let events = log_events(server);
    let names = names(&events);
    let starting = names.iter().position(|name| *name == "server_starting");
    let ready = names.iter().position(|name| *name == "server_ready");
    let (Some(starting), Some(ready)) = (starting, ready) else {
        panic!("start events: {names:?}");
    };
    events[starting..ready]
        .iter()
        .filter(|event| {
            event["event"]
                .as_str()
                .is_some_and(|name| name.starts_with("ip_db_"))
        })
        .cloned()
        .collect()
}

fn by_ip(body: &Value) -> BTreeMap<String, Value> {
    body["addresses"]["items"]
        .as_array()
        .unwrap_or_else(|| panic!("address items"))
        .iter()
        .map(|item| {
            (
                item["ip"].as_str().unwrap_or_default().to_owned(),
                item.clone(),
            )
        })
        .collect()
}

fn counts(item: &Value) -> [u64; 4] {
    ["attempts", "rejected", "aborted", "sessions"].map(|key| {
        item[key]
            .as_u64()
            .unwrap_or_else(|| panic!("{key} in {item}"))
    })
}

/// spec: operations.md:OPS-096 partial
/// spec: operations.md:OPS-097 partial
/// spec: operations.md:OPS-098 partial
/// spec: operations.md:OPS-099 partial
/// spec: management.md:MGT-062 partial
/// spec: management.md:MGT-063 partial
#[test]
fn sources_count_proxied_clients_with_the_local_databases() {
    let server = Server::start_with_config_transform("sources", None, false, |config, dir| {
        write_db(dir, "country.mmdb", &country_db("KR"));
        write_db(dir, "asn.mmdb", &asn_db());
        proxied(
            config,
            "[ip_info]\ncountry_db = \"ipdb/country.mmdb\"\nasn_db = \"ipdb/asn.mmdb\"\n",
        )
    });
    let dir = server.dir().to_string_lossy().into_owned();

    // OPS-096: both databases load before readiness, without a path in the log.
    let startup = startup_db_events(&server);
    assert_eq!(
        startup
            .iter()
            .map(|event| (event["event"].clone(), event["config_key"].clone()))
            .collect::<Vec<_>>(),
        [
            (json!("ip_db_loaded"), json!("ip_info.country_db")),
            (json!("ip_db_loaded"), json!("ip_info.asn_db")),
        ]
    );
    assert_eq!(startup[0]["database_type"], "DBIP-Country-Lite");
    assert_eq!(startup[1]["build_time"], "2025-11-01T00:00:00.000Z");
    assert!(startup.iter().all(|event| event["phase"] == "startup"));

    // MGT-062: the route needs a session and refuses other methods and bad windows.
    let refused = sources_reply(&server, None, "");
    assert_eq!(refused.status, 401);
    assert!(refused.body.contains("unauthenticated"), "{}", refused.body);
    let admin = server.login();
    let empty = sources(&server, &admin);
    assert_eq!(empty["window"], "1h");
    assert_eq!(empty["capacity"], 100_000);
    assert_eq!(empty["stored"], 0);
    assert_eq!(empty["dropped_before"], Value::Null);
    assert_eq!(empty["addresses"], json!({"total": 0, "items": []}));
    for query in [
        "?window=1H",
        "?window=7d",
        "?window=",
        "?window=1h&window=24h",
    ] {
        let reply = sources_reply(&server, Some(&admin), query);
        assert_eq!(reply.status, 400, "{query}");
        assert!(reply.body.contains("invalid_request"), "{query}");
    }
    let put = server.request(
        "PUT",
        "/api/v1/sources",
        &[("Cookie", admin.cookie.as_str())],
        b"",
    );
    assert_eq!(put.status, 405);
    assert!(
        put.headers
            .lines()
            .any(|line| line.eq_ignore_ascii_case("allow: GET")),
        "{}",
        put.headers
    );
    let polled = server.request(
        "GET",
        "/api/v1/sources?window=24h",
        &[("Cookie", admin.cookie.as_str()), ("X-Relay-Poll", "1")],
        b"",
    );
    assert_eq!(polled.status, 200, "{}", polled.body);
    let polled: Value = live::must(serde_json::from_str(&polled.body));
    assert_eq!(polled["window"], "24h");
    let bad_poll = server.request(
        "GET",
        "/api/v1/sources",
        &[("Cookie", admin.cookie.as_str()), ("X-Relay-Poll", "2")],
        b"",
    );
    assert_eq!(bad_poll.status, 400);

    // OPS-098: a Mode II pair and a dropped handshake from loopback, WS clients
    // through the trusted proxy, one of them refused by the allow list.
    let pair = server.pair(&admin, "II", "246813579");
    drop(server.handshaking());
    let (status, kept) = ws_from(&server, "198.51.100.7");
    assert_eq!(status, 101);
    drop(kept);
    let (status, kept) = ws_from(&server, "2001:db8::7");
    assert_eq!(status, 101);
    drop(kept);
    let (status, _) = ws_from(&server, "192.0.2.9");
    assert_eq!(status, 403);
    let mut body = Value::Null;
    live::wait_for("source counts", LIMIT, || {
        body = sources(&server, &admin);
        body["totals"] == json!({"attempts": 6, "rejected": 1, "aborted": 3, "sessions": 1})
    });
    pair.close();

    // MGT-063: per-address values, local loopback, and the databases in use.
    let addresses = by_ip(&body);
    assert_eq!(body["addresses"]["total"], 4);
    assert_eq!(body["addresses"]["items"][0]["ip"], "127.0.0.1");
    let local = &addresses["127.0.0.1"];
    assert_eq!(counts(local), [3, 0, 1, 2]);
    assert_eq!(local["local"], true);
    assert_eq!(local["country"], Value::Null);
    assert_eq!(local["asn"], Value::Null);
    let transit = &addresses["198.51.100.7"];
    assert_eq!(counts(transit), [1, 0, 1, 0]);
    assert_eq!(transit["local"], false);
    assert_eq!(transit["country"], "KR");
    assert_eq!(transit["asn"], 64496);
    assert_eq!(transit["as_name"], "Example Transit");
    let v6 = &addresses["2001:db8::7"];
    assert_eq!(counts(v6), [1, 0, 1, 0]);
    assert_eq!(v6["country"], "JP");
    assert_eq!(v6["asn"], Value::Null);
    let refused_client = &addresses["192.0.2.9"];
    assert_eq!(counts(refused_client), [1, 1, 0, 0]);
    assert_eq!(refused_client["country"], Value::Null);
    assert_eq!(refused_client["asn"], 64511);
    assert_eq!(
        body["countries"]["items"].as_array().map(|items| items
            .iter()
            .map(|item| item["country"].clone())
            .collect::<Vec<_>>()),
        Some(vec![Value::Null, json!("JP"), json!("KR")])
    );
    assert_eq!(body["countries"]["items"][0]["addresses"], 2);
    assert_eq!(
        body["asns"]["items"].as_array().map(|items| items
            .iter()
            .map(|item| (item["asn"].clone(), item["as_name"].clone()))
            .collect::<Vec<_>>()),
        Some(vec![
            (Value::Null, Value::Null),
            (json!(64496), json!("Example Transit")),
            (json!(64511), json!("Example Edge")),
        ])
    );
    for (key, provider, kind) in [
        ("country", "dbip", "DBIP-Country-Lite"),
        ("asn", "maxmind_geolite", "GeoLite2-ASN"),
    ] {
        let db = &body["databases"][key];
        assert_eq!(db["state"], "loaded", "{key}");
        assert_eq!(db["error"], Value::Null, "{key}");
        assert_eq!(db["provider"], provider, "{key}");
        assert_eq!(db["database_type"], kind, "{key}");
        assert_eq!(db["build_time"], "2025-11-01T00:00:00.000Z", "{key}");
        assert!(db["loaded_at"].is_string(), "{key}");
    }
    assert!(
        !body.to_string().contains(&dir),
        "no database path in the query"
    );

    // OPS-097: a replaced country database applies to later events only.
    let reloads = |server: &Server| {
        log_events(server)
            .into_iter()
            .filter(|event| event["event"] == "ip_db_loaded" && event["phase"] == "reload")
            .count()
    };
    write_db(server.dir(), "country.mmdb", &country_db("DE"));
    live::wait_for("country database reloaded", Duration::from_secs(30), || {
        reloads(&server) == 1
    });
    let (status, kept) = ws_from(&server, "198.51.100.8");
    assert_eq!(status, 101);
    drop(kept);
    live::wait_for("later event counted", LIMIT, || {
        body = sources(&server, &admin);
        body["totals"]["aborted"] == 4
    });
    let addresses = by_ip(&body);
    assert_eq!(addresses["198.51.100.7"]["country"], "KR");
    assert_eq!(addresses["198.51.100.8"]["country"], "DE");
    assert!(
        log_events(&server).iter().all(|event| {
            event["event"] != "reload_applied" && event["event"] != "reload_rejected"
        }),
        "a database change is not a config reload"
    );

    // OPS-099: the statistics and the database paths stay out of /metrics and the log.
    let mut stream = live::connect(server.ports[1]);
    live::must(
        stream.write_all(b"GET /metrics HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"),
    );
    let metrics = live::read_reply(stream);
    assert_eq!(metrics.status, 200);
    for secret in [
        "198.51.100",
        "2001:db8",
        "192.0.2",
        "country",
        "64496",
        "ipdb",
    ] {
        assert!(!metrics.body.contains(secret), "/metrics holds {secret}");
    }
    let log = log_text(&server);
    assert!(
        !log.contains(&dir) && !log.contains("ipdb"),
        "no database path in the log"
    );
    for field in ["\"country\"", "\"asn\"", "\"as_name\""] {
        assert!(!log.contains(field), "the log holds {field}");
    }
}

/// spec: operations.md:OPS-096 partial
/// spec: management.md:MGT-063 partial
#[test]
fn unusable_databases_warn_and_the_server_still_starts() {
    let server =
        Server::start_with_config_transform("sources-broken", None, false, |config, dir| {
            write_db(dir, "garbage.mmdb", b"not an MMDB file");
            proxied(
                config,
                "[ip_info]\ncountry_db = \"ipdb/missing.mmdb\"\nasn_db = \"ipdb/garbage.mmdb\"\n",
            )
        });
    let startup = startup_db_events(&server);
    assert_eq!(
        startup
            .iter()
            .map(|event| (
                event["event"].clone(),
                event["config_key"].clone(),
                event["reason"].clone()
            ))
            .collect::<Vec<_>>(),
        [
            (
                json!("ip_db_unavailable"),
                json!("ip_info.country_db"),
                json!("file_unreadable")
            ),
            (
                json!("ip_db_unavailable"),
                json!("ip_info.asn_db"),
                json!("invalid_format")
            ),
        ]
    );
    let log = log_text(&server);
    assert!(!log.contains("missing.mmdb") && !log.contains("garbage.mmdb"));
    let admin = server.login();
    let body = sources(&server, &admin);
    for (key, error) in [("country", "file_unreadable"), ("asn", "invalid_format")] {
        let db = &body["databases"][key];
        assert_eq!(db["state"], "unavailable", "{key}");
        assert_eq!(db["error"], error, "{key}");
        assert_eq!(db["provider"], Value::Null, "{key}");
        assert_eq!(db["loaded_at"], Value::Null, "{key}");
    }

    // An ASN-only file named for both keys: read once, no country field.
    let server =
        Server::start_with_config_transform("sources-asn-only", None, false, |config, dir| {
            write_db(dir, "asn.mmdb", &asn_db());
            proxied(
                config,
                "[ip_info]\ncountry_db = \"ipdb/asn.mmdb\"\nasn_db = \"ipdb/asn.mmdb\"\n",
            )
        });
    let startup = startup_db_events(&server);
    assert_eq!(
        startup
            .iter()
            .map(|event| (event["event"].clone(), event["reason"].clone()))
            .collect::<Vec<_>>(),
        [
            (json!("ip_db_unavailable"), json!("unsupported_data")),
            (json!("ip_db_loaded"), Value::Null),
        ]
    );
    let admin = server.login();
    let body = sources(&server, &admin);
    assert_eq!(body["databases"]["country"]["error"], "unsupported_data");
    assert_eq!(body["databases"]["asn"]["state"], "loaded");
}

/// spec: management.md:MGT-063 partial
/// spec: management.md:MGT-064 partial
/// spec: operations.md:OPS-099 partial
#[test]
fn a_new_process_without_databases_starts_empty_and_counts_without_lookups() {
    let server = Server::start("sources-none", None);
    let admin = server.login();
    let body = sources(&server, &admin);
    assert_eq!(body["stored"], 0);
    assert_eq!(body["dropped_before"], Value::Null);
    assert_eq!(
        body["totals"],
        json!({"attempts": 0, "rejected": 0, "aborted": 0, "sessions": 0})
    );
    for list in ["addresses", "countries", "asns"] {
        assert_eq!(body[list], json!({"total": 0, "items": []}), "{list}");
    }
    for key in ["country", "asn"] {
        assert_eq!(
            body["databases"][key],
            json!({"state": "not_configured", "error": null, "database_type": null,
                "provider": null, "build_time": null, "loaded_at": null}),
            "{key}"
        );
    }
    assert!(
        !names(&log_events(&server))
            .iter()
            .any(|name| name.starts_with("ip_db_"))
    );
    drop(server.handshaking());
    let mut body = Value::Null;
    live::wait_for("loopback abort counted", LIMIT, || {
        body = sources(&server, &admin);
        body["totals"]["aborted"] == 1
    });
    assert_eq!(body["stored"], 2);
    let item = &body["addresses"]["items"][0];
    assert_eq!(item["ip"], "127.0.0.1");
    assert_eq!(item["local"], true);
    assert_eq!(item["country"], Value::Null);
    assert_eq!(item["asn"], Value::Null);
    assert_eq!(item["as_name"], Value::Null);
    assert!(body["window_start"].as_str() < body["as_of"].as_str());
}
