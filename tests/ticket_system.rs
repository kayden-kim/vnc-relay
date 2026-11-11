//! End-to-end ticket scenarios through the shipped serve binary, file watcher,
//! management API, operation log, TCP, WS and fixture-authenticated WSS.

#[path = "../crates/adapters/tests/common/ticket.rs"]
mod ticket;
#[path = "common/ticket_server.rs"]
mod ticket_server;

use std::fs;
use std::net::TcpListener;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use ticket::Signer;
use ticket_server::*;

fn items(value: &Value) -> &[Value] {
    some(value["items"].as_array().map(|v| &**v))
}
fn accepted(server: &Server, jti: &str) -> Value {
    server.event(0, |e| {
        e["event"] == "ticket_accepted" && e["ticket_jti"] == jti
    })
}
fn rejection(server: &Server, after: usize, ticket: &str, reason: &str) -> Reply {
    let (reply, ws) = server.upgrade(false, "viewer", Some(ticket));
    assert_eq!(reply.status, 401, "{}", reply.raw);
    assert!(ws.is_none());
    assert_eq!(reply.header("WWW-Authenticate"), Some("VncRelayTicket"));
    server.event(after, |e| {
        e["event"] == "connection_rejected" && e["stage"] == "upgrade" && e["reason"] == reason
    });
    reply
}
fn no_secrets(text: &str, secrets: &[String]) {
    for secret in secrets {
        assert!(!text.contains(secret), "secret leaked to observable output");
    }
}
fn assert_private_surfaces(server: &Server, secrets: &[String]) {
    no_secrets(&server.log(), secrets);
    no_secrets(&server.stderr(), secrets);
    for path in ["/livez", "/readyz"] {
        let reply = http(server.status, "GET", path, &[], "");
        assert_eq!(reply.status, 200, "{}", reply.raw);
        no_secrets(&reply.raw, secrets);
    }
    for path in [
        "/api/v1/sessions",
        "/api/v1/closed-sessions",
        "/api/v1/logs?limit=1000",
        "/api/v1/config",
        "/api/v1/waiting",
        "/api/v1/summary",
    ] {
        let reply = server.get(path);
        assert_eq!(reply.status, 200, "{}", reply.raw);
        no_secrets(&reply.raw, secrets);
        if path == "/api/v1/sessions" {
            for session in items(&reply.json()) {
                let detail = server.get(&format!(
                    "/api/v1/sessions/{}",
                    some(session["session_id"].as_str())
                ));
                assert_eq!(detail.status, 200);
                no_secrets(&detail.raw, secrets);
            }
        }
    }
}
fn remember_ticket(secrets: &mut Vec<String>, ticket: &str) {
    secrets.push(ticket.to_owned());
    secrets.push(some(ticket.rsplit('.').next()).to_owned());
}
fn remember_keys(secrets: &mut Vec<String>, keys: &str) {
    secrets.push(keys.to_owned());
    let keys: Value = must(serde_json::from_str(keys));
    for key in some(keys["keys"].as_array()) {
        secrets.push(some(key["x"].as_str()).to_owned());
        secrets.push(some(key["y"].as_str()).to_owned());
    }
}

/// spec: websocket.md:WS-038
/// spec: operations.md:OPS-073
#[test]
fn viewer_ws_wss_tickets_match_and_all_observable_surfaces_keep_secrets_private() {
    let _ports = lock();
    let viewer = Signer::new();
    let server_signer = Signer::new();
    let server = Server::start("privacy", &viewer.jwks(), &server_signer.jwks(), true);
    let mut secrets = Vec::new();
    remember_keys(&mut secrets, &viewer.jwks());
    remember_keys(&mut secrets, &server_signer.jwks());
    for secure in [false, true] {
        let rid = if secure { "2038" } else { "1038" };
        let jti = format!("privacy-{rid}");
        let ticket = viewer.sign(&claims("viewer", &jti, rid, Some(now() + 4)));
        remember_ticket(&mut secrets, &ticket);
        let (reply, ws) = server.upgrade(secure, "viewer", Some(&ticket));
        assert_eq!(reply.status, 101, "{}", reply.raw);
        no_secrets(&reply.raw, &secrets);
        let mut ws = some(ws);
        ws.data(b"RFB 000.000\n");
        let event = accepted(&server, &jti);
        assert_eq!(event["level"], "info");
        assert_eq!(event["role"], "viewer");
        assert_eq!(event["listener"], if secure { "wss" } else { "ws" });
        assert_eq!(event["peer_ip"], "127.0.0.1");
        assert!(event["conn_id"].is_string());
        assert_eq!(event["ticket_sub"], "system-test-subject");
        timestamp(&event["session_expires_at"]);
        let mut peer = server.tcp_peer("server", rid);
        ws.send(&request(&format!("ID:{rid}")));
        exchange(&mut ws, &mut peer);
        server.poll("/api/v1/sessions", |v| !items(v).is_empty());
        assert_private_surfaces(&server, &secrets);
        let before = server.events().len();
        let reused = rejection(&server, before, &ticket, "ticket_reused");
        no_secrets(&reused.raw, &secrets);
        let reused_event = server.event(before, |e| e["reason"] == "ticket_reused");
        assert_eq!(reused_event["ticket_jti"], jti);
        ws.close(4013, "session_expired");
        tcp_closed(&mut peer);
        server.poll("/api/v1/closed-sessions", |v| {
            items(v)
                .iter()
                .any(|s| s["viewer_transport"] == if secure { "wss" } else { "ws" })
        });
    }
    let bad_signer = Signer::new();
    let bad = bad_signer.sign(&claims("viewer", "bad-signature", "999", None));
    remember_ticket(&mut secrets, &bad);
    let before = server.events().len();
    let rejected = rejection(&server, before, &bad, "ticket_invalid");
    no_secrets(&rejected.raw, &secrets);
    let event = server.event(before, |e| {
        e["event"] == "connection_rejected" && e["reason"] == "ticket_invalid"
    });
    assert!(
        event.get("ticket_jti").is_none(),
        "unverified identity must not be logged"
    );
    assert_private_surfaces(&server, &secrets);
    // Preserve the actual x/y strings inside invalid input, so a parser diagnostic
    // echoing its input would be caught rather than merely checking a filename.
    let invalid_keys = format!("{} trailing-invalid-jwks", viewer.jwks());
    secrets.push(invalid_keys.clone());
    let before = server.events().len();
    must(fs::write(server.dir.join("viewer.json"), invalid_keys));
    let event = server.event(before, |e| e["event"] == "reload_rejected");
    assert_eq!(event["reason"], "ticket_keys_invalid");
    assert_private_surfaces(&server, &secrets);
}

fn change_ticket_settings(server: &Server, api: bool, enabled: bool, replacement: bool) {
    let issuer = if replacement {
        "replacement/viewer"
    } else {
        "issuer/viewer"
    };
    let host = if replacement {
        "replacement.example"
    } else {
        "relay.example"
    };
    let lifetime = if replacement { 10 } else { 60 };
    if api {
        server.patch(json!({"relay.ticket.viewer.enabled":enabled,
            "relay.ticket.viewer.issuer":issuer, "relay.ticket.viewer.relay_host":host,
            "relay.ticket.viewer.max_lifetime_secs":lifetime}));
    } else {
        let file = must(fs::read_to_string(server.dir.join("relay.toml")));
        let (head, tail) = some(file.split_once("[relay.ticket.viewer]"));
        let (_, server_tail) = some(tail.split_once("[relay.ticket.server]"));
        let text = format!(
            "{head}[relay.ticket.viewer]\nenabled = {enabled}\nissuer = \"{issuer}\"\nrelay_host = \"{host}\"\npublic_keys_file = \"viewer.json\"\nmax_lifetime_secs = {lifetime}\n[relay.ticket.server]{server_tail}"
        );
        let before = server.events().len();
        must(fs::write(server.dir.join("relay.toml"), text));
        server.event(before, |e| e["event"] == "reload_applied");
    }
    let view = server.query("/api/v1/config");
    for (key, expected) in [
        ("enabled", json!(enabled)),
        ("issuer", json!(issuer)),
        ("relay_host", json!(host)),
        ("max_lifetime_secs", json!(lifetime)),
    ] {
        let path = format!("relay.ticket.viewer.{key}");
        let item = some(items(&view).iter().find(|item| item["key"] == path));
        assert_eq!(item["value"], expected);
    }
}
fn replacement_claims(jti: &str, rid: &str) -> Value {
    let mut value = claims("viewer", jti, rid, None);
    value["iss"] = json!("replacement/viewer");
    value["rh"] = json!("replacement.example");
    value["exp"] = json!(some(value["iat"].as_u64()) + 10);
    value
}

/// spec: websocket.md:WS-046
#[test]
fn watched_and_api_ticket_reconfiguration_preserves_existing_bindings_timers_and_use_records() {
    let _ports = lock();
    for api in [false, true] {
        let original = Signer::new();
        let rotated = Signer::new();
        let server_signer = Signer::new();
        let server = Server::start(
            if api {
                "api-reconfigure"
            } else {
                "watch-reconfigure"
            },
            &original.jwks(),
            &server_signer.jwks(),
            true,
        );
        let replay = original.sign(&claims("viewer", "survives-rotation", "123", None));
        drop(server.ws(false, "viewer", &replay));
        let sx = now() + 4;
        let old_ticket = original.sign(&claims("viewer", "old-101", "0123", Some(sx)));
        let mut old = server.ws(false, "viewer", &old_ticket);
        let mismatch_ticket = original.sign(&claims("viewer", "old-mismatch", "0123", None));
        let mut mismatch = server.ws(true, "viewer", &mismatch_ticket);
        server.rotate("viewer", &rotated.jwks());
        change_ticket_settings(&server, api, false, true);
        let (reply, unticketed) = server.upgrade(false, "viewer", None);
        assert_eq!(reply.status, 101, "{}", reply.raw);
        let mut unticketed = some(unticketed);
        unticketed.data(b"RFB 000.000\n");
        drop(unticketed);
        mismatch.send(&request("ID:123"));
        mismatch.close(4012, "ticket_relay_mismatch");
        let mut peer = server.tcp_peer("server", "0123");
        old.send(&request("ID:0123"));
        exchange(&mut old, &mut peer);
        old.close(4013, "session_expired");
        assert!(now() >= sx, "old timer must not expire early");
        assert!(
            now() <= sx + 2,
            "old sx must survive reload and key rotation"
        );
        tcp_closed(&mut peer);
        change_ticket_settings(&server, api, true, true);
        let new_ticket = rotated.sign(&replacement_claims("new-generation", "555"));
        drop(server.ws(true, "viewer", &new_ticket));
        // Isolate each replaced field: the others and the signature are valid.
        for (field, value) in [
            ("iss", json!("issuer/viewer")),
            ("rh", json!("relay.example")),
            ("exp", json!(now() + 5 + 60)),
        ] {
            let mut wrong = replacement_claims(&format!("wrong-{field}"), "555");
            wrong[field] = value;
            let before = server.events().len();
            rejection(&server, before, &rotated.sign(&wrong), "ticket_invalid");
        }
        let before = server.events().len();
        rejection(
            &server,
            before,
            &original.sign(&replacement_claims("old-key", "555")),
            "ticket_invalid",
        );
        server.rotate("viewer", &original.jwks());
        change_ticket_settings(&server, api, true, false);
        let before = server.events().len();
        rejection(&server, before, &replay, "ticket_reused");
        let reused = server.event(before, |e| e["reason"] == "ticket_reused");
        assert_eq!(reused["ticket_jti"], "survives-rotation");
        // The same original bytes remain cryptographically/time valid; only the
        // preserved use record prevents a second 101 after both transitions.
        let fresh = original.sign(&claims("viewer", "fresh-original-key", "556", None));
        drop(server.ws(false, "viewer", &fresh));
    }
}

/// spec: relay.md:RLY-029
/// spec: relay.md:RLY-032
#[test]
fn both_ticket_roles_compare_rid_bytes_and_leave_the_waiting_opposite_peer_intact() {
    let _ports = lock();
    let viewer = Signer::new();
    let target = Signer::new();
    let server = Server::start("rid-binding", &viewer.jwks(), &target.jwks(), true);
    for (role, opposite, signer) in [("viewer", "server", &viewer), ("server", "viewer", &target)] {
        let mut waiting = server.tcp_peer(opposite, "123");
        let before_wait = server.poll("/api/v1/waiting", |v| items(v).len() == 1);
        assert_eq!(items(&before_wait)[0]["role"], opposite);
        assert_eq!(items(&before_wait)[0]["relay_id"], "123");
        for (index, ticket_rid) in ["999", "0123"].into_iter().enumerate() {
            let jti = format!("{role}-mismatch-{index}");
            let ticket = signer.sign(&claims(role, &jti, ticket_rid, None));
            let mut wrong = server.ws(index == 1, role, &ticket);
            wrong.send(&request("ID:123"));
            wrong.close(4012, "ticket_relay_mismatch");
            let event = server.event(0, |e| {
                e["event"] == "connection_rejected" && e["ticket_jti"] == jti
            });
            assert_eq!(event["stage"], "initial_exchange");
            assert_eq!(event["reason"], "ticket_relay_mismatch");
            assert_eq!(event["mode"], "mode2");
            let reference = some(event["relay_id_ref"].as_str());
            assert!(!reference.is_empty());
            assert_ne!(reference, "123");
            let conn = &event["conn_id"];
            assert!(
                !server
                    .events()
                    .iter()
                    .any(|e| e["conn_id"] == *conn && e["event"] == "waiting_registered")
            );
            let after = server.query("/api/v1/waiting");
            assert_eq!(items(&after).len(), 1);
            assert_eq!(
                items(&after)[0]["connected_at"],
                items(&before_wait)[0]["connected_at"]
            );
            assert_eq!(items(&after)[0]["role"], opposite);
            assert!(items(&server.query("/api/v1/sessions")).is_empty());
        }
        let correct = signer.sign(&claims(role, &format!("{role}-correct"), "123", None));
        let mut correct = server.ws(false, role, &correct);
        correct.send(&request("ID:123"));
        exchange(&mut correct, &mut waiting);
        assert!(items(&server.query("/api/v1/waiting")).is_empty());
        drop(correct);
        drop(waiting);
        server.poll("/api/v1/sessions", |v| items(v).is_empty());
    }
}

/// spec: relay.md:RLY-030
#[test]
fn ticket_mode_one_rejection_precedes_target_connect_but_not_disabled_mode_or_syntax() {
    let _ports = lock();
    for enabled in [true, false] {
        let viewer = Signer::new();
        let target = Signer::new();
        let server = Server::start(
            if enabled { "mode1-on" } else { "mode1-off" },
            &viewer.jwks(),
            &target.jwks(),
            enabled,
        );
        let listener = must(TcpListener::bind("127.0.0.1:0"));
        must(listener.set_nonblocking(true));
        let target = must(listener.local_addr()).to_string();
        let ticket = viewer.sign(&claims("viewer", "mode-one", "123", None));
        let mut ws = server.ws(false, "viewer", &ticket);
        ws.send(&request(&target));
        let reason = if enabled {
            "ticket_relay_mismatch"
        } else {
            "mode_disabled"
        };
        ws.close(if enabled { 4012 } else { 1008 }, reason);
        let event = server.event(0, |e| {
            e["event"] == "connection_rejected" && e["reason"] == reason
        });
        assert_eq!(event["stage"], "initial_exchange");
        assert_eq!(event["mode"], "mode1");
        if enabled {
            assert_eq!(event["ticket_jti"], "mode-one");
        }
        assert!(event.get("relay_id_ref").is_none());
        if enabled {
            let ticket = viewer.sign(&claims("viewer", "syntax-before-binding", "123", None));
            let mut ws = server.ws(true, "viewer", &ticket);
            ws.send(&request("invalid target"));
            ws.close(4008, "handshake_invalid");
        }
        let summary = server.query("/api/v1/summary");
        assert_eq!(summary["counts"]["target_connecting"], 0);
        assert_eq!(summary["counts"]["waiting"], 0);
        assert_eq!(summary["counts"]["active"], 0);
        // The allow policy explicitly permits this listening address. A forbidden
        // connect would remain queued even after the relay immediately closed it.
        match listener.accept() {
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
            other => panic!("ticket Mode I attempted a target connection: {other:?}"),
        }
    }
}

fn closed_item(server: &Server, id: &str) -> Value {
    let closed = server.poll("/api/v1/closed-sessions", |v| {
        items(v).iter().any(|s| s["session_id"] == id)
    });
    some(items(&closed).iter().find(|s| s["session_id"] == id)).clone()
}
fn active_item(server: &Server) -> Value {
    let sessions = server.poll("/api/v1/sessions", |v| items(v).len() == 1);
    items(&sessions)[0].clone()
}
fn assert_expired_session(server: &Server, active: &Value) {
    let id = some(active["session_id"].as_str());
    let closed = closed_item(server, id);
    assert_eq!(closed["close_cause"], "session_expired");
    for key in [
        "session_id",
        "mode",
        "viewer_transport",
        "target_transport",
        "started_at",
    ] {
        assert_eq!(
            closed[key], active[key],
            "closed field {key} retains its meaning"
        );
    }
    assert_eq!(closed["bytes_viewer_to_target"], 11);
    assert_eq!(closed["bytes_target_to_viewer"], 11);
    assert!(some(closed["ended_at"].as_str()) >= some(closed["started_at"].as_str()));
    assert!(closed.get("viewer_address").is_none());
    assert!(closed.get("target_address").is_none());
    let event = server.event(0, |e| {
        e["event"] == "session_closed" && e["session_id"] == id
    });
    assert_eq!(event["close_cause"], "session_expired");
    assert_eq!(event["level"], "info");
    assert!(event.get("side").is_none());
    assert!(event.get("admin_account").is_none());
    let logs = server.query(&format!("/api/v1/logs?session_id={id}&limit=1000"));
    assert!(items(&logs).iter().any(|entry| entry["entry"] == event));
    assert_eq!(server.get(&format!("/api/v1/sessions/{id}")).status, 404);
}

/// spec: relay.md:RLY-031
/// spec: management.md:MGT-052
#[test]
fn sx_expires_every_lifecycle_state_and_earliest_side_with_distinct_management_cause() {
    let _ports = lock();
    let viewer = Signer::new();
    let target = Signer::new();
    let server = Server::start("session-expiry", &viewer.jwks(), &target.jwks(), true);
    // Initial exchange and waiting are tested over both logical roles/transports.
    for (role, secure, waiting) in [("viewer", false, false), ("server", true, true)] {
        let signer = if role == "viewer" { &viewer } else { &target };
        let ticket = signer.sign(&claims(
            role,
            &format!("expiry-{role}"),
            "310",
            Some(now() + 2),
        ));
        let before = server.events().len();
        let mut ws = server.ws(secure, role, &ticket);
        if waiting {
            ws.send(&request("ID:310"));
            server.poll("/api/v1/waiting", |v| items(v).len() == 1);
        }
        ws.close(4013, "session_expired");
        let event = server.event(before, |e| {
            e["event"]
                == if waiting {
                    "waiting_removed"
                } else {
                    "connection_aborted"
                }
                && e["reason"] == "session_expired"
        });
        assert_eq!(event["level"], "info");
        if !waiting {
            assert_eq!(event["stage"], "initial_exchange");
        }
        server.poll("/api/v1/waiting", |v| items(v).is_empty());
        assert!(items(&server.query("/api/v1/closed-sessions")).is_empty());
    }
    // A ticket on either logical side expires the entire mixed TCP/WS session.
    for (role, secure) in [
        ("viewer", false),
        ("viewer", true),
        ("server", false),
        ("server", true),
    ] {
        let signer = if role == "viewer" { &viewer } else { &target };
        let opposite = if role == "viewer" { "server" } else { "viewer" };
        let jti = format!("active-{role}-{secure}");
        let sx = now() + 2;
        let ticket = signer.sign(&claims(role, &jti, "311", Some(sx)));
        let mut ws = server.ws(secure, role, &ticket);
        let mut tcp = server.tcp_peer(opposite, "311");
        ws.send(&request("ID:311"));
        exchange(&mut ws, &mut tcp);
        let active = active_item(&server);
        ws.close(4013, "session_expired");
        assert!(now() >= sx && now() <= sx + 2);
        tcp_closed(&mut tcp);
        assert_expired_session(&server, &active);
    }
    // Reverse which side has the earlier deadline to rule out viewer-only or
    // server-only timers. The later side cannot govern either session.
    for viewer_first in [true, false] {
        let early = now() + 2;
        let late = early + 2;
        let viewer_ticket = viewer.sign(&claims(
            "viewer",
            &format!("two-viewer-{viewer_first}"),
            "312",
            Some(if viewer_first { early } else { late }),
        ));
        let server_ticket = target.sign(&claims(
            "server",
            &format!("two-server-{viewer_first}"),
            "312",
            Some(if viewer_first { late } else { early }),
        ));
        let mut v = server.ws(false, "viewer", &viewer_ticket);
        let mut s = server.ws(true, "server", &server_ticket);
        v.send(&request("ID:312"));
        s.send(&request("ID:312"));
        v.send(b"viewer-data");
        s.data(b"viewer-data");
        s.send(b"server-data");
        v.data(b"server-data");
        let active = active_item(&server);
        v.close(4013, "session_expired");
        s.close(4013, "session_expired");
        assert!(now() >= early, "session must not expire early");
        assert!(
            now() < late,
            "earlier sx, not later sx, controls the session"
        );
        assert_expired_session(&server, &active);
    }
    // No sx must remain usable beyond the short expiry windows exercised above.
    let ticket = viewer.sign(&claims("viewer", "no-session-expiry", "313", None));
    let mut ws = server.ws(true, "viewer", &ticket);
    let mut peer = server.tcp_peer("server", "313");
    ws.send(&request("ID:313"));
    exchange(&mut ws, &mut peer);
    let active = active_item(&server);
    let until = Instant::now() + Duration::from_secs(4);
    while Instant::now() < until {
        assert_eq!(
            items(&server.query("/api/v1/sessions"))[0]["session_id"],
            active["session_id"]
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    exchange(&mut ws, &mut peer);
    // Terminate concurrently with the close handshake so an artificial client
    // stall cannot turn an administrator termination into close_timeout.
    let port = server.management;
    let admin = server.admin.clone();
    let id = some(active["session_id"].as_str()).to_owned();
    let terminate_id = id.clone();
    let terminate = std::thread::spawn(move || {
        http(
            port,
            "POST",
            &format!("/api/v1/sessions/{terminate_id}/terminate"),
            &[("Cookie", &admin.0), ("X-CSRF-Token", &admin.1)],
            "",
        )
    });
    ws.close(4005, "admin_terminated");
    tcp_closed(&mut peer);
    let terminated = must(terminate.join());
    assert_eq!(terminated.status, 200, "{}", terminated.raw);
    assert_eq!(
        terminated.json()["closed_session"]["close_cause"],
        "admin_terminated"
    );
    let closed = server.query("/api/v1/closed-sessions");
    assert!(
        items(&closed)
            .iter()
            .any(|s| s["session_id"] == id && s["close_cause"] == "admin_terminated")
    );
    assert_eq!(
        items(&closed)
            .iter()
            .filter(|s| s["close_cause"] == "session_expired")
            .count(),
        6
    );
}
