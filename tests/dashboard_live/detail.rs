use std::io::{ErrorKind, Read};
use std::net::TcpStream;
use std::time::Duration;

use serde_json::json;

use super::live::{Browser, Fault, LIMIT, Server, must, wait_for};

const GONE: &str = "The current session was not found";
const FAILED: &str = "Session termination failed. Check the status again.";
const UNKNOWN: &str = "The termination result cannot be confirmed. The session may have ended. The request is not sent again automatically. Check the status again.";
const CANCEL: &str = "dialog[open] .dialog-actions button:first-child";
const CONFIRM: &str = "dialog[open] .dialog-actions button:last-child";

fn detail_path(id: &str) -> String {
    format!("/api/v1/sessions/{id}")
}

fn terminate_path(id: &str) -> String {
    format!("{}/terminate", detail_path(id))
}

fn open_detail(browser: &mut Browser, server: &Server, id: &str) {
    browser.navigate(&server.url(&format!("/#/sessions/{id}")));
    browser.wait("document.querySelector('#session-terminate')?.disabled === false");
    assert_eq!(
        browser.eval("document.querySelector('h1 .session-id')?.textContent"),
        json!(id)
    );
}

fn open_confirmation(browser: &mut Browser, id: &str) {
    browser.click("#session-terminate");
    browser.wait("document.querySelector('dialog')?.open === true");
    let description =
        browser.eval("document.querySelector('dialog .dialog-description')?.textContent");
    let expected = format!(
        "The connection for session {id} will close. The relay between the Viewer and the Server or Mode I target will stop. A closed session does not resume."
    );
    assert_eq!(description, json!(expected));
}

fn assert_gone(browser: &mut Browser) {
    browser.wait(&format!(
        "document.querySelector('.session-notice')?.textContent.includes({}) && document.querySelector('#session-terminate')?.disabled === true",
        json!(GONE)
    ));
    assert_eq!(
        browser.eval("document.querySelectorAll('.fact-value').length"),
        json!(0)
    );
    assert_eq!(
        browser.eval("document.querySelector('dialog')?.open"),
        json!(false)
    );
    assert_eq!(
        browser.eval("document.querySelector('.session-notice a')?.getAttribute('href')"),
        json!("/")
    );
}

fn back_to_list(browser: &mut Browser) {
    browser.click(".page-head a[href='/']");
    browser.wait("location.hash === '' && !document.querySelector('#session-terminate') && document.querySelector('h1')?.textContent === 'Operations overview'");
}

fn assert_no_terminate(server: &Server) {
    assert!(
        server
            .proxy
            .traffic()
            .iter()
            .all(|request| { request.method != "POST" || !request.path.ends_with("/terminate") }),
        "opening or dismissing confirmation must not send a terminate request"
    );
}

fn assert_terminate_traffic(
    server: &Server,
    id: &str,
    forwarded: bool,
    injected: bool,
    status: u16,
) {
    let traffic = server.proxy.traffic();
    let posts: Vec<_> = traffic
        .iter()
        .filter(|request| request.method == "POST" && request.path.ends_with("/terminate"))
        .collect();
    assert_eq!(posts.len(), 1, "no duplicate or other-session terminate");
    assert_eq!(posts[0].path, terminate_path(id));
    assert_eq!(posts[0].forwarded, forwarded);
    assert_eq!(posts[0].injected, injected);
    assert_eq!(posts[0].status, Some(status));
    assert!(posts[0].completed.is_some());
    assert_eq!(
        server.proxy.injected_count("POST", &terminate_path(id)),
        usize::from(injected)
    );
}

fn assert_closed_socket(stream: &mut TcpStream) {
    must(stream.set_read_timeout(Some(LIMIT)));
    let mut byte = [0_u8; 1];
    match stream.read(&mut byte) {
        Ok(0) => {}
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted | ErrorKind::BrokenPipe
            ) => {}
        result => panic!("terminated real relay socket must be closed: {result:?}"),
    }
}

pub fn selected_detail_matches_real_session_and_returns_to_list() {
    for (mode, relay_id) in [("I", "51006"), ("II", "52006")] {
        let server = Server::start(&format!("detail-facts-{mode}"), Some(30));
        let admin = server.login();
        let mut pair = server.pair(&admin, mode, relay_id);
        pair.transfer(1536, 1_572_864);
        let raw = server.query(&admin, &detail_path(&pair.sid));
        let session = &raw["session"];
        assert_eq!(session["session_id"], json!(pair.sid));
        assert_eq!(
            session["mode"],
            json!(if mode == "I" { "mode1" } else { "mode2" })
        );
        assert_eq!(session["viewer_transport"], json!("tcp"));
        assert_eq!(session["target_transport"], json!("tcp"));
        assert_eq!(session["bytes_viewer_to_target"], json!(1536));
        assert_eq!(session["bytes_target_to_viewer"], json!(1_572_864));
        for (field, address) in [
            ("viewer_address", must(pair.viewer.local_addr())),
            ("target_address", must(pair.target.local_addr())),
        ] {
            assert_eq!(session[field]["ip"], json!(address.ip().to_string()));
            assert_eq!(session[field]["port"], json!(address.port()));
        }
        let mut browser = Browser::start(&format!("detail-facts-{mode}"), &["en-US"]);
        browser.login(&server);
        let selector = format!("a[href='/#/sessions/{}']:not(.button)", pair.sid);
        browser.click(&selector);
        browser.wait("document.querySelector('#session-terminate')?.disabled === false");
        assert_eq!(
            browser.eval("location.hash"),
            json!(format!("#/sessions/{}", pair.sid))
        );
        assert_eq!(
            browser.eval("document.querySelector('h1 .session-id')?.textContent"),
            json!(pair.sid)
        );
        let actual = browser
            .eval("Array.from(document.querySelectorAll('.fact-value'), node => node.textContent)");
        let started = browser.eval(&format!(
            "(() => {{ const date = new Date({}); const pad = n => String(n).padStart(2, '0'); const zone = new Intl.DateTimeFormat('en-US', {{timeZoneName: 'short'}}).formatToParts(date).find(p => p.type === 'timeZoneName').value; return `${{date.getFullYear()}}-${{pad(date.getMonth()+1)}}-${{pad(date.getDate())}} ${{pad(date.getHours())}}:${{pad(date.getMinutes())}}:${{pad(date.getSeconds())}} ${{zone}}`; }})()",
            session["started_at"]
        ));
        assert_eq!(
            actual,
            json!([
                if mode == "I" { "Mode I" } else { "Mode II" },
                "TCP",
                "TCP",
                started,
                must(pair.viewer.local_addr()).to_string(),
                must(pair.target.local_addr()).to_string()
            ])
        );
        assert_eq!(
            browser.eval("Array.from(document.querySelectorAll('.grid .stat-value'), node => node.textContent)"),
            json!(["1.5 KiB", "1.5 MiB"])
        );
        assert!(browser.text().contains("Addresses are socket peer addresses. Forwarded addresses are shown separately when a trusted proxy is used."));
        let reads: Vec<_> = server
            .proxy
            .traffic()
            .into_iter()
            .filter(|request| request.path == detail_path(&pair.sid))
            .collect();
        assert!(!reads.is_empty());
        assert!(
            reads.iter().all(|request| request.forwarded
                && !request.injected
                && request.status == Some(200))
        );
        back_to_list(&mut browser);
        browser.wait(&format!("!!document.querySelector({})", json!(selector)));
        assert_no_terminate(&server);
        pair.close();
    }
}

pub fn disappeared_detail_before_read_or_during_refresh_is_gone() {
    for before_initial_read in [true, false] {
        let name = if before_initial_read {
            "detail-initial-gone"
        } else {
            "detail-refresh-gone"
        };
        let server = Server::start(name, Some(30));
        let admin = server.login();
        let pair = server.pair(&admin, "II", "52007");
        let id = pair.sid.clone();
        let path = detail_path(&id);
        let mut browser = Browser::start(name, &["en-US"]);
        browser.login(&server);
        if before_initial_read {
            pair.close();
            wait_for("real target gone before detail read", LIMIT, || {
                server
                    .request("GET", &path, &[("Cookie", &admin.cookie)], &[])
                    .status
                    == 404
            });
            browser.navigate(&server.url(&format!("/#/sessions/{id}")));
        } else {
            open_detail(&mut browser, &server, &id);
            server
                .proxy
                .once("GET", &path, Fault::DelayBefore(Duration::from_secs(2)));
            let reads = server.proxy.count("GET", &path);
            browser.click("#session-refresh");
            wait_for("refresh entered proxy", LIMIT, || {
                server.proxy.count("GET", &path) > reads
            });
            pair.close();
        }
        assert_gone(&mut browser);
        let traffic = server.proxy.traffic();
        let final_read = match traffic
            .iter()
            .rev()
            .find(|request| request.method == "GET" && request.path == path)
        {
            Some(request) => request,
            None => panic!("real detail read was not observed"),
        };
        assert!(final_read.forwarded);
        assert_eq!(final_read.status, Some(404));
        assert_eq!(final_read.injected, !before_initial_read);
        assert_eq!(
            server.proxy.injected_count("GET", &path),
            usize::from(!before_initial_read)
        );
        assert_no_terminate(&server);
        browser.click(".session-notice a[href='/']");
        browser.wait("location.hash === '' && document.querySelector('h1')?.textContent === 'Operations overview'");
        browser.wait(&format!(
            "!document.querySelector({})",
            json!(format!("a[href='/#/sessions/{id}']"))
        ));
    }
}

pub fn cancel_and_escape_preserve_real_session_and_restore_focus() {
    let server = Server::start("detail-cancel", Some(30));
    let admin = server.login();
    let mut pair = server.pair(&admin, "II", "52009");
    let mut browser = Browser::start("detail-cancel", &["en-US"]);
    browser.login(&server);
    open_detail(&mut browser, &server, &pair.sid);
    for escape in [false, true] {
        open_confirmation(&mut browser, &pair.sid);
        assert_no_terminate(&server);
        if escape {
            browser.key("Escape");
        } else {
            browser.click(CANCEL);
        }
        browser.wait("document.querySelector('dialog')?.open === false && document.activeElement?.id === 'session-terminate'");
        assert_no_terminate(&server);
        pair.transfer(17, 29);
        assert_eq!(
            server.query(&admin, &detail_path(&pair.sid))["session"]["session_id"],
            json!(pair.sid)
        );
    }
    assert_eq!(
        server
            .proxy
            .injected_count("POST", &terminate_path(&pair.sid)),
        0
    );
    pair.close();
}

pub fn confirmed_termination_blocks_duplicates_then_refreshes_list() {
    let server = Server::start("detail-confirm", Some(1));
    let admin = server.login();
    let mut pair = server.pair(&admin, "II", "52010");
    let mut browser = Browser::start("detail-confirm", &["en-US"]);
    browser.login(&server);
    open_detail(&mut browser, &server, &pair.sid);
    let path = terminate_path(&pair.sid);
    server
        .proxy
        .once("POST", &path, Fault::Delay(Duration::from_secs(3)));
    open_confirmation(&mut browser, &pair.sid);
    browser.click(CONFIRM);
    browser.wait("document.querySelector('#session-terminate')?.disabled === true && document.querySelector('#session-terminate')?.textContent.includes('Terminating') && document.querySelector('#session-refresh')?.disabled === true");
    wait_for("terminate forwarded while response held", LIMIT, || {
        server
            .proxy
            .traffic()
            .iter()
            .any(|request| request.path == path && request.forwarded && request.injected)
    });
    browser.click("#session-terminate");
    browser.click("#session-terminate");
    browser.click("#session-refresh");
    assert_eq!(server.proxy.count("POST", &path), 1);
    assert_eq!(
        browser.eval("document.querySelector('dialog')?.open"),
        json!(false)
    );
    assert_gone(&mut browser);
    assert_terminate_traffic(&server, &pair.sid, true, true, 200);
    assert_closed_socket(&mut pair.viewer);
    assert_closed_socket(&mut pair.target);
    browser.click(".session-notice a[href='/']");
    browser.wait("document.querySelector('h1')?.textContent === 'Operations overview' && document.body.textContent.includes('No active sessions')");
    assert_eq!(
        browser.eval(&format!(
            "document.querySelector({}) === null",
            json!(format!("a[href='/#/sessions/{}']", pair.sid))
        )),
        json!(true)
    );
    browser.wait(&format!("document.body.textContent.includes({}) && document.body.textContent.includes('Terminated by administrator')", json!(pair.sid)));
    assert_terminate_traffic(&server, &pair.sid, true, true, 200);
}

pub fn natural_close_before_or_after_confirmation_never_targets_another_session() {
    for before_open in [true, false] {
        let name = if before_open {
            "detail-natural-before"
        } else {
            "detail-natural-after"
        };
        let server = Server::start(name, Some(30));
        let admin = server.login();
        let pair = server.pair(&admin, "II", "52011");
        let id = pair.sid.clone();
        let mut survivor = server.pair(&admin, "II", "52111");
        let mut browser = Browser::start(name, &["en-US"]);
        browser.login(&server);
        open_detail(&mut browser, &server, &id);
        if before_open {
            pair.close();
            open_confirmation(&mut browser, &id);
        } else {
            open_confirmation(&mut browser, &id);
            pair.close();
        }
        wait_for(
            "natural close before confirming stale dialog",
            LIMIT,
            || {
                server
                    .request("GET", &detail_path(&id), &[("Cookie", &admin.cookie)], &[])
                    .status
                    == 404
            },
        );
        browser.click(CONFIRM);
        assert_gone(&mut browser);
        assert_terminate_traffic(&server, &id, true, false, 404);
        survivor.transfer(31, 47);
        assert_eq!(
            server.query(&admin, &detail_path(&survivor.sid))["session"]["session_id"],
            json!(survivor.sid)
        );
        assert_eq!(
            server.proxy.count("POST", &terminate_path(&survivor.sid)),
            0
        );
        browser.click(".session-notice a[href='/']");
        browser.wait(&format!(
            "!!document.querySelector({})",
            json!(format!("a[href='/#/sessions/{}']", survivor.sid))
        ));
        survivor.close();
    }
}

pub fn injected_termination_failure_is_not_completion_and_offers_status_read() {
    let server = Server::start("detail-injected-failure", Some(30));
    let admin = server.login();
    let mut pair = server.pair(&admin, "II", "52012");
    let mut browser = Browser::start("detail-injected-failure", &["en-US"]);
    browser.login(&server);
    open_detail(&mut browser, &server, &pair.sid);
    let detail = detail_path(&pair.sid);
    let reads = server.proxy.count("GET", &detail);
    server.proxy.once(
        "POST",
        &terminate_path(&pair.sid),
        Fault::Error500("terminate_failed"),
    );
    open_confirmation(&mut browser, &pair.sid);
    browser.click(CONFIRM);
    browser.wait(&format!(
        "document.querySelector('.session-notice')?.textContent.includes({})",
        json!(FAILED)
    ));
    browser.wait("document.querySelector('#session-terminate')?.disabled === false");
    assert!(!browser.text().contains(GONE));
    assert!(!browser.text().contains(UNKNOWN));
    assert_eq!(
        browser.eval("document.querySelector('.session-notice button')?.textContent.trim()"),
        json!("Check status again")
    );
    assert!(server.proxy.count("GET", &detail) > reads);
    assert_terminate_traffic(&server, &pair.sid, false, true, 500);
    pair.transfer(41, 53);
    assert_eq!(
        server.query(&admin, &detail)["session"]["session_id"],
        json!(pair.sid)
    );
    let reads = server.proxy.count("GET", &detail);
    browser.click(".session-notice button");
    wait_for(
        "explicit status reread after injected failure",
        LIMIT,
        || server.proxy.count("GET", &detail) > reads,
    );
    browser.wait("document.querySelector('#session-terminate')?.disabled === false");
    let traffic = server.proxy.traffic();
    assert!(
        traffic
            .iter()
            .filter(|request| request.method == "GET" && request.path == detail)
            .all(|request| request.forwarded && !request.injected && request.status == Some(200))
    );
    assert_terminate_traffic(&server, &pair.sid, false, true, 500);
    pair.close();
}

pub fn lost_real_termination_response_is_unknown_and_reads_before_any_resend() {
    let server = Server::start("detail-lost-response", Some(1));
    let admin = server.login();
    let mut pair = server.pair(&admin, "II", "52013");
    let mut browser = Browser::start("detail-lost-response", &["en-US"]);
    browser.login(&server);
    open_detail(&mut browser, &server, &pair.sid);
    let detail = detail_path(&pair.sid);
    let post = terminate_path(&pair.sid);
    server.proxy.once("POST", &post, Fault::DropResponse);
    server
        .proxy
        .once("GET", &detail, Fault::DelayBefore(Duration::from_secs(3)));
    open_confirmation(&mut browser, &pair.sid);
    browser.click(CONFIRM);
    browser.wait(&format!(
        "document.querySelector('.session-notice')?.textContent.includes({})",
        json!(UNKNOWN)
    ));
    assert!(!browser.text().contains(FAILED));
    assert!(!browser.text().contains(GONE));
    assert_eq!(
        browser.eval("document.querySelector('.session-notice button')?.textContent.trim()"),
        json!("Check status again")
    );
    assert_eq!(
        browser.eval("document.querySelector('#session-terminate')?.disabled"),
        json!(true)
    );
    assert_terminate_traffic(&server, &pair.sid, true, true, 200);
    assert_closed_socket(&mut pair.viewer);
    assert_closed_socket(&mut pair.target);
    let closed = server.query(&admin, "/api/v1/closed-sessions");
    let item = match closed["items"].as_array().and_then(|items| {
        items
            .iter()
            .find(|item| item["session_id"] == json!(pair.sid))
    }) {
        Some(item) => item,
        None => panic!("real terminate must have ended the selected relay"),
    };
    assert_eq!(item["close_cause"], json!("admin_terminated"));
    assert_gone(&mut browser);
    let traffic = server.proxy.traffic();
    let termination = match traffic
        .iter()
        .find(|request| request.method == "POST" && request.path == post)
    {
        Some(request) => request,
        None => panic!("missing terminate traffic"),
    };
    let reread = match traffic.iter().find(|request| {
        request.method == "GET" && request.path == detail && request.at > termination.at
    }) {
        Some(request) => request,
        None => panic!("unknown outcome must first query the selected status"),
    };
    assert!(reread.forwarded && reread.injected);
    assert_eq!(reread.status, Some(404));
    assert_eq!(server.proxy.injected_count("GET", &detail), 1);
    std::thread::sleep(Duration::from_millis(1200));
    assert_terminate_traffic(&server, &pair.sid, true, true, 200);
}

pub fn unauthenticated_direct_detail_exposes_no_data_or_termination() {
    let server = Server::start("detail-no-session", Some(1));
    let admin = server.login();
    let mut pair = server.pair(&admin, "II", "52014");
    pair.transfer(73, 97);
    let mut browser = Browser::start("detail-no-session", &["en-US"]);
    browser.navigate(&server.url(&format!("/#/sessions/{}", pair.sid)));
    browser.wait(
        "location.pathname === '/login/' && !!document.querySelector('input[type=password]')",
    );
    let text = browser.text();
    for protected in [
        pair.sid.as_str(),
        "Addresses are socket peer addresses.",
        "Terminate session",
        "73",
        "97",
    ] {
        assert!(
            !text.contains(protected),
            "unauthenticated page leaked {protected}"
        );
    }
    assert_eq!(browser.eval("document.querySelectorAll('.fact-value, .grid .stat-value, #session-terminate, dialog').length"), json!(0));
    assert_no_terminate(&server);
    assert!(
        server
            .proxy
            .traffic()
            .iter()
            .filter(|request| request.path.starts_with("/api/v1/"))
            .all(|request| request.status == Some(401) && request.forwarded && !request.injected)
    );
    pair.transfer(19, 23);
    pair.close();
}

pub fn malformed_detail_ids_make_no_api_request_and_differ_from_real_not_found() {
    let server = Server::start("detail-invalid-id", Some(30));
    let mut browser = Browser::start("detail-invalid-id", &["en-US"]);
    browser.login(&server);
    let long = "a".repeat(65);
    for id in [
        "bad%20id",
        "bad%2Fid",
        "%",
        "%E0%A4%A",
        "%3Cscript%3E",
        long.as_str(),
    ] {
        let before = server.proxy.traffic().len();
        browser.navigate(&server.url(&format!("/#/sessions/{id}")));
        browser.wait(
            "document.querySelector('.session-notice')?.textContent.includes('Invalid session ID')",
        );
        assert!(!browser.text().contains(GONE));
        assert_eq!(browser.eval("document.querySelectorAll('.fact-value, .grid .stat-value, #session-terminate').length"), json!(0));
        let traffic = server.proxy.traffic();
        assert!(
            traffic
                .iter()
                .skip(before)
                .all(|request| !request.path.starts_with("/api/v1/")),
            "malformed id must send no management API request"
        );
    }
    back_to_list(&mut browser);
    let id = "genuine_missing_session_026";
    let path = detail_path(id);
    let before = server.proxy.count("GET", &path);
    browser.navigate(&server.url(&format!("/#/sessions/{id}")));
    assert_gone(&mut browser);
    assert!(!browser.text().contains("Invalid session ID"));
    assert_eq!(server.proxy.count("GET", &path), before + 1);
    assert_eq!(server.proxy.injected_count("GET", &path), 0);
    assert!(
        server
            .proxy
            .traffic()
            .iter()
            .any(|request| request.method == "GET"
                && request.path == path
                && request.forwarded
                && !request.injected
                && request.status == Some(404))
    );
    assert_no_terminate(&server);
    browser.click(".session-notice a[href='/']");
    browser.wait("location.hash === '' && document.querySelector('h1')?.textContent === 'Operations overview'");
}

pub(super) fn assert_real_transport_and_addresses(
    browser: &mut Browser,
    session: &serde_json::Value,
) {
    assert_eq!(browser.eval(&format!(r#"(() => {{
        const s={session}, facts=[...document.querySelectorAll('.fact-row')];
        const values=[...document.querySelectorAll('.fact-value')].map(n=>n.textContent);
        const address=a=>a.ip.includes(':')?`[${{a.ip}}]:${{a.port}}`:`${{a.ip}}:${{a.port}}`;
        const rows=[...document.querySelectorAll('.fact')];
        const text=document.body.innerText;
        return values[1]===s.viewer_transport.toUpperCase() && values[2]===s.target_transport.toUpperCase() &&
            values.includes(address(s.viewer_address)) && values.includes(address(s.target_address)) &&
            [s.viewer_forwarded_ip,s.target_forwarded_ip].filter(v=>v!==null).every(v=>values.includes(v)) &&
            text.includes('Viewer forwarded address')===(s.viewer_forwarded_ip!==null) &&
            text.includes('Server/target forwarded address')===(s.target_forwarded_ip!==null) &&
            text.includes('Addresses are socket peer addresses. Forwarded addresses are shown separately when a trusted proxy is used.');
    }})()"#)),json!(true),"detail preserves socket peers and displays only present forwarded addresses");
}
