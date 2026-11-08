use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::live::{Admin, Browser, Fault, LIMIT, Server, must, wait_for};

const WAITING: &str = ".grid > .card:nth-child(4)";
const ACTIVE: &str = ".grid > .card:nth-child(5)";
const CLOSED: &str = ".grid > .card:nth-child(6)";
const KIB_BYTES: usize = 1536;
const MIB_BYTES: usize = 1_572_864;

// These expectations use the public display contract, not the application's
// formatter/model or a replacement fetch implementation.
const LOCAL_TIME: &str = r#"
const localTime = value => {
  const d = new Date(value);
  const pad = n => String(n).padStart(2, '0');
  const zone = new Intl.DateTimeFormat('en-US', {timeZoneName: 'short'})
    .formatToParts(d).find(p => p.type === 'timeZoneName')?.value;
  if (!zone) throw new Error('browser did not report its local timezone');
  return `${d.getFullYear()}-${pad(d.getMonth()+1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())} ${zone}`;
};
"#;

fn items(value: &Value) -> &[Value] {
    value["items"]
        .as_array()
        .unwrap_or_else(|| panic!("missing list items: {value}"))
}

fn text(value: &Value) -> &str {
    value
        .as_str()
        .unwrap_or_else(|| panic!("missing string: {value}"))
}

fn initial(value: &str) -> [u8; 250] {
    assert!(value.len() <= 250);
    let mut bytes = [0; 250];
    bytes[..value.len()].copy_from_slice(value.as_bytes());
    bytes
}

fn connect(port: u16) -> TcpStream {
    let stream = must(TcpStream::connect_timeout(
        &SocketAddr::from(([127, 0, 0, 1], port)),
        LIMIT,
    ));
    must(stream.set_read_timeout(Some(LIMIT)));
    must(stream.set_write_timeout(Some(LIMIT)));
    stream
}

fn ready(browser: &mut Browser) {
    browser.wait(
        r#"document.querySelector('#operations-refresh') !== null &&
        document.querySelectorAll('.grid > .card').length === 6 &&
        [...document.querySelectorAll('.card-status,.card-alert')].every(n => n.textContent.trim() === '') &&
        document.querySelector('.page-meta')?.textContent.startsWith('마지막 성공 갱신: ') &&
        !document.querySelector('.page-meta')?.textContent.includes('확인 불가')"#,
    );
}

pub(super) fn assert_list_counts(
    browser: &mut Browser,
    counts: [Option<usize>; 3],
    language: &str,
) {
    let counts = json!(counts);
    let language = json!(language);
    assert_eq!(
        browser.eval(&format!(
            r#"(() => {{
            const counts = {counts};
            const language = {language};
            const cards = [...document.querySelectorAll('.grid > .card')];
            return cards.length === 6 &&
              cards.slice(0, 3).every(card => !card.querySelector('.card-title .badge')) &&
              cards.slice(3).every((card, index) => {{
                const count = counts[index];
                const badges = card.querySelectorAll('.card-title > .badge');
                if (count === null) return badges.length === 0;
                const label = language === 'ko' ? `${{count}}건` : count === 1 ? '1 item' : `${{count}} items`;
                return badges.length === 1 && badges[0].textContent === label &&
                  badges[0].dataset.tone === 'neutral' && !badges[0].querySelector('svg') &&
                  card.querySelectorAll('tbody tr:not(.data-table-empty)').length === count;
              }});
            }})()"#,
        )),
        json!(true),
        "each list title must describe only its own displayed snapshot in the current language",
    );
}

fn assert_clean(browser: &mut Browser) {
    assert_eq!(
        browser.eval(
            r#"[...document.querySelectorAll('.card-status,.card-alert')].every(n => n.textContent.trim() === '') && !document.querySelector('[role=alert]')"#,
        ),
        json!(true),
        "normal real-server data must not contain loading, stale, or error notices",
    );
}

fn assert_unavailable_summary(server: &Server, browser: &mut Browser) {
    let before = server.proxy.injected_count("GET", "/api/v1/summary");
    server
        .proxy
        .set("GET", "/api/v1/summary", Fault::Error500("internal"));
    // A new page has no last successful snapshot. The actual failed request
    // makes its values unconfirmed; no fabricated normal JSON enters the UI.
    browser.reload();
    browser.wait(
        r#"document.querySelector('.grid > .card:first-child .card-alert')?.textContent.includes('조회 실패') &&
        [...document.querySelectorAll('.summary-grid .stat-value')].length === 6 &&
        [...document.querySelectorAll('.summary-grid .stat-value')].every(n => n.textContent === '확인 불가')"#,
    );
    assert_eq!(
        browser.eval(
            r#"document.querySelectorAll('.gauge').length === 2 &&
            [...document.querySelectorAll('.gauge')].every(n =>
              !n.hasAttribute('aria-valuenow') &&
              n.getAttribute('aria-valuetext') === '확인 불가' &&
              n.querySelector('.gauge-center .badge')?.textContent === '확인 불가' &&
              n.querySelector('.gauge-max')?.textContent === '상한 확인 불가') &&
            [...document.querySelectorAll('.summary-grid .stat-value')].every(n => n.textContent !== '0')"#,
        ),
        json!(true),
    );
    assert!(
        server.proxy.injected_count("GET", "/api/v1/summary") > before,
        "the unavailable branch must actually exercise the injected failure",
    );
    assert!(server.proxy.traffic().iter().any(|entry| {
        entry.method == "GET"
            && entry.path == "/api/v1/summary"
            && entry.injected
            && !entry.forwarded
            && entry.status == Some(500)
    }));
    server.proxy.clear();
}

pub fn overview_known_data() {
    let server = Server::start("overview-known", Some(60));
    let admin = server.login();
    let mut mode_one = server.pair(&admin, "I", "41001");
    let mut mode_two = server.pair(&admin, "II", "41002");
    mode_one.transfer(KIB_BYTES, MIB_BYTES);
    mode_two.transfer(MIB_BYTES, KIB_BYTES);
    let waiting_viewer = server.waiting("41003");
    let mut waiting_server = connect(server.ports[3]);
    must(waiting_server.write_all(&initial("ID:41004")));
    wait_for("both real waiting roles", LIMIT, || {
        items(&server.query(&admin, "/api/v1/waiting")).len() == 2
    });
    let raw_active = server.query(&admin, "/api/v1/sessions");
    assert_eq!(items(&raw_active).len(), 2);
    for (sid, mode, outbound, inbound) in [
        (&mode_one.sid, "mode1", KIB_BYTES, MIB_BYTES),
        (&mode_two.sid, "mode2", MIB_BYTES, KIB_BYTES),
    ] {
        let item = items(&raw_active)
            .iter()
            .find(|item| item["session_id"] == *sid)
            .unwrap_or_else(|| panic!("missing real session {sid}"));
        assert_eq!(item["mode"], mode);
        assert_eq!(item["viewer_transport"], "tcp");
        assert_eq!(item["bytes_viewer_to_target"], json!(outbound));
        assert_eq!(item["bytes_target_to_viewer"], json!(inbound));
    }
    let mut browser = Browser::start("overview-known", &["ko-KR"]);
    browser.login(&server);
    ready(&mut browser);
    let raw_waiting = server.query(&admin, "/api/v1/waiting");
    assert_eq!(items(&raw_waiting).len(), 2);
    let evidence = browser.eval(&format!(
        r#"(() => {{
        {LOCAL_TIME}
        const active = {raw_active}.items;
        const waiting = {raw_waiting}.items;
        const cells = row => [...row.querySelectorAll('td')];
        const rows = selector => [...document.querySelectorAll(selector + ' tbody tr')];
        const values = (row, selector) => [...row.querySelectorAll(selector)].map(n => n.textContent.trim());
        const address = a => a.ip.includes(':') ? `[${{a.ip}}]:${{a.port}}` : `${{a.ip}}:${{a.port}}`;
        const byteText = n => n === 1536 ? '1.5 KiB' : n === 1572864 ? '1.5 MiB' : null;
        const activeRows = rows('{ACTIVE}');
        const waitingRows = rows('{WAITING}');
        const activeCorrect = active.length === 2 && activeRows.length === 2 && active.every(item => {{
          const row = activeRows.find(r => cells(r)[0]?.textContent === item.session_id);
          if (!row) return false;
          const c = cells(row);
          return c[1].textContent === (item.mode === 'mode1' ? 'Mode I' : 'Mode II') &&
            JSON.stringify(values(c[2], '.badge')) === JSON.stringify([item.viewer_transport.toUpperCase(), item.target_transport.toUpperCase()]) &&
            c[3].textContent === localTime(item.started_at) &&
            JSON.stringify(values(row, '.cell-bytes .cell-value')) === JSON.stringify([byteText(item.bytes_viewer_to_target), byteText(item.bytes_target_to_viewer)]) &&
            JSON.stringify(values(row, '.cell-bytes .cell-label')) === JSON.stringify(['Viewer->대상', '대상->Viewer']) &&
            JSON.stringify(values(row, '.cell-peer .cell-value')) === JSON.stringify([address(item.viewer_address), address(item.target_address)]) &&
            c[0].querySelector('a')?.getAttribute('href') === '/#/sessions/' + item.session_id &&
            c[6].querySelector('a')?.getAttribute('href') === '/#/sessions/' + item.session_id;
        }});
        const waitingCorrect = waiting.length === 2 && waitingRows.length === 2 && waiting.every(item => {{
          const row = waitingRows.find(r => cells(r)[1]?.textContent === item.relay_id);
          if (!row) return false;
          const c = cells(row);
          const clock = c[3].textContent;
          const parts = clock.split(':').map(Number);
          const elapsed = (parts[0]*3600 + parts[1]*60 + parts[2])*1000;
          return c[0].textContent === (item.role === 'viewer' ? 'Viewer' : 'Server') &&
            c[2].textContent === localTime(item.connected_at) && /^\d{{2}}:\d{{2}}:\d{{2}}$/.test(clock) &&
            elapsed >= 0 && elapsed <= item.waited_ms && item.waited_ms - elapsed < 10000 &&
            !row.querySelector('a,button,[role=button]');
        }});
        const stats = [...document.querySelectorAll('.summary-grid .stat-value')].map(n => n.textContent);
        return {{activeCorrect, waitingCorrect, roles: waiting.map(i => i.role).sort(),
          state: stats[0], uptime: /^\d{{2}}:\d{{2}}:\d{{2}}$/.test(stats[1]), waiting: stats[2], active: stats[3]}};
        }})()"#,
    ));
    assert_eq!(
        evidence,
        json!({"activeCorrect": true, "waitingCorrect": true, "roles": ["server", "viewer"],
            "state": "준비됨", "uptime": true, "waiting": "2", "active": "2"}),
    );
    assert_clean(&mut browser);
    assert_list_counts(&mut browser, [Some(2), Some(2), Some(0)], "ko");
    browser.eval(
        "(() => { const select = document.querySelector('select'); select.value = 'en'; select.dispatchEvent(new Event('change', {bubbles:true})); })()",
    );
    assert_list_counts(&mut browser, [Some(2), Some(2), Some(0)], "en");
    browser.eval(
        "(() => { const select = document.querySelector('select'); select.value = 'ko'; select.dispatchEvent(new Event('change', {bubbles:true})); })()",
    );
    assert_eq!(server.proxy.injected_count("GET", "/api/v1/summary"), 0);
    drop((waiting_viewer, waiting_server, mode_one, mode_two));
    assert_unavailable_summary(&server, &mut browser);
}

pub fn overview_empty_lists() {
    let server = Server::start("overview-empty", Some(60));
    let admin = server.login();
    for path in ["/api/v1/waiting", "/api/v1/sessions"] {
        assert!(items(&server.query(&admin, path)).is_empty());
    }
    let mut browser = Browser::start("overview-empty", &["ko-KR"]);
    browser.login(&server);
    ready(&mut browser);
    assert_eq!(
        browser.eval(&format!(
            r#"[document.querySelector('{WAITING} tbody')?.textContent.trim(), document.querySelector('{ACTIVE} tbody')?.textContent.trim()]"#,
        )),
        json!(["대기 없음", "활성 없음"]),
    );
    assert_clean(&mut browser);
    assert_list_counts(&mut browser, [Some(0), Some(0), Some(0)], "ko");
    assert!(server.proxy.traffic().iter().all(|entry| !entry.injected));
}

struct WebSocket {
    stream: TcpStream,
}

impl WebSocket {
    fn open(port: u16) -> Self {
        let mut stream = connect(port);
        must(stream.write_all(b"GET /relay HTTP/1.1\r\nHost: localhost\r\nOrigin: http://127.0.0.1\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"));
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            must(stream.read_exact(&mut byte));
            head.push(byte[0]);
            assert!(head.len() < 16384, "bounded WebSocket handshake");
        }
        assert!(head.starts_with(b"HTTP/1.1 101 "), "{head:?}");
        let mut socket = Self { stream };
        assert_eq!(socket.frame(), (2, b"RFB 000.000\n".to_vec()));
        socket
    }

    fn send(&mut self, opcode: u8, bytes: &[u8]) {
        assert!(bytes.len() <= usize::from(u16::MAX));
        let mut frame = vec![0x80 | opcode];
        if bytes.len() < 126 {
            frame.push(0x80 | must(u8::try_from(bytes.len())));
        } else {
            frame.push(0xfe);
            frame.extend_from_slice(&must(u16::try_from(bytes.len())).to_be_bytes());
        }
        let mask = [0x13, 0x57, 0x9b, 0xdf];
        frame.extend_from_slice(&mask);
        frame.extend(bytes.iter().enumerate().map(|(i, byte)| byte ^ mask[i % 4]));
        must(self.stream.write_all(&frame));
        must(self.stream.flush());
    }

    fn frame(&mut self) -> (u8, Vec<u8>) {
        loop {
            let mut head = [0; 2];
            must(self.stream.read_exact(&mut head));
            assert_eq!(head[1] & 0x80, 0, "server frames are unmasked");
            let len = match head[1] & 0x7f {
                126 => {
                    let mut bytes = [0; 2];
                    must(self.stream.read_exact(&mut bytes));
                    usize::from(u16::from_be_bytes(bytes))
                }
                127 => {
                    let mut bytes = [0; 8];
                    must(self.stream.read_exact(&mut bytes));
                    must(usize::try_from(u64::from_be_bytes(bytes)))
                }
                value => usize::from(value),
            };
            assert!(len <= 1024 * 1024, "bounded WebSocket frame");
            let mut bytes = vec![0; len];
            must(self.stream.read_exact(&mut bytes));
            match head[0] & 15 {
                9 => self.send(10, &bytes),
                10 => {}
                opcode => return (opcode, bytes),
            }
        }
    }
}

fn close_with_protocol_error(server: &Server, admin: &Admin) -> String {
    let mut viewer = WebSocket::open(server.ws_port);
    let mut target = connect(server.ports[3]);
    must(target.write_all(&initial("ID:42002")));
    wait_for("WebSocket error target waiting", LIMIT, || {
        items(&server.query(admin, "/api/v1/waiting"))
            .iter()
            .any(|item| item["relay_id"] == "42002")
    });
    viewer.send(2, &initial("ID:42002"));
    let mut sid = String::new();
    wait_for("real WS session active", LIMIT, || {
        let list = server.query(admin, "/api/v1/sessions");
        if let Some(item) = items(&list)
            .iter()
            .find(|item| item["viewer_transport"] == "ws")
        {
            sid = text(&item["session_id"]).to_owned();
            true
        } else {
            false
        }
    });
    let outgoing = vec![0x31; KIB_BYTES];
    viewer.send(2, &outgoing);
    let mut received = vec![0; outgoing.len()];
    must(target.read_exact(&mut received));
    assert_eq!(received, outgoing);
    let incoming = vec![0x72; 3072];
    must(target.write_all(&incoming));
    let mut received = Vec::new();
    while received.len() < incoming.len() {
        let (opcode, bytes) = viewer.frame();
        assert_eq!(opcode, 2);
        received.extend_from_slice(&bytes);
    }
    assert_eq!(received, incoming);
    // Text frames are forbidden on the production binary relay. This is a real
    // transport I/O error, unlike an OS-dependent TCP reset or a proxy error.
    viewer.send(1, b"forbidden-text");
    let (opcode, payload) = viewer.frame();
    assert_eq!(opcode, 8);
    assert!(payload.len() >= 2);
    assert_eq!(u16::from_be_bytes([payload[0], payload[1]]), 1003);
    drop((viewer, target));
    wait_for("actual connection_error history", LIMIT, || {
        items(&server.query(admin, "/api/v1/closed-sessions"))
            .iter()
            .any(|item| item["session_id"] == sid && item["close_cause"] == "connection_error")
    });
    sid
}

pub fn overview_recent_closed() {
    let server = Server::start("overview-history", Some(60));
    let admin = server.login();
    assert!(items(&server.query(&admin, "/api/v1/closed-sessions")).is_empty());
    let mut browser = Browser::start("overview-history", &["ko-KR"]);
    browser.login(&server);
    ready(&mut browser);
    assert_eq!(
        browser.eval(&format!(
            "document.querySelector('{CLOSED} tbody')?.textContent.trim()",
        )),
        json!("최근 종료 없음"),
    );
    assert_clean(&mut browser);
    assert_list_counts(&mut browser, [Some(0), Some(0), Some(0)], "ko");

    let mut eof = server.pair(&admin, "II", "42001");
    eof.transfer(KIB_BYTES, MIB_BYTES);
    let eof_sid = eof.sid.clone();
    eof.close();
    wait_for("actual EOF history", LIMIT, || {
        items(&server.query(&admin, "/api/v1/closed-sessions"))
            .iter()
            .any(|item| item["session_id"] == eof_sid && item["close_cause"] == "peer_eof")
    });
    let error_sid = close_with_protocol_error(&server, &admin);
    let mut terminated = server.pair(&admin, "I", "42003");
    terminated.transfer(MIB_BYTES, KIB_BYTES);
    browser.navigate(&server.url(&format!("/#/sessions/{}", terminated.sid)));
    browser.wait("document.querySelector('#session-terminate')?.disabled === false");
    browser.click("#session-terminate");
    browser.click("dialog[open] .button-destructive");
    wait_for("browser administrator termination", LIMIT, || {
        items(&server.query(&admin, "/api/v1/closed-sessions"))
            .iter()
            .any(|item| {
                item["session_id"] == terminated.sid && item["close_cause"] == "admin_terminated"
            })
    });
    let history = server.query(&admin, "/api/v1/closed-sessions");
    assert_eq!(items(&history).len(), 3);
    for (sid, cause, outgoing, incoming) in [
        (&eof_sid, "peer_eof", KIB_BYTES, MIB_BYTES),
        (&error_sid, "connection_error", KIB_BYTES, 3072),
        (&terminated.sid, "admin_terminated", MIB_BYTES, KIB_BYTES),
    ] {
        let item = items(&history)
            .iter()
            .find(|item| item["session_id"] == *sid)
            .unwrap_or_else(|| panic!("missing closed session {sid}"));
        assert_eq!(item["close_cause"], cause);
        assert_eq!(item["bytes_viewer_to_target"], json!(outgoing));
        assert_eq!(item["bytes_target_to_viewer"], json!(incoming));
        assert!(text(&item["ended_at"]) >= text(&item["started_at"]));
    }
    let terminate_path = format!("/api/v1/sessions/{}/terminate", terminated.sid);
    assert_eq!(server.proxy.count("POST", &terminate_path), 1);
    assert_eq!(server.proxy.injected_count("POST", &terminate_path), 0);
    browser.navigate(&server.url("/"));
    ready(&mut browser);
    assert_eq!(
        browser.eval(&format!(
            r#"(() => {{
            {LOCAL_TIME}
            const items = {history}.items;
            const rows = [...document.querySelectorAll('{CLOSED} tbody tr')];
            const causes = {{peer_eof:'정상 EOF', connection_error:'연결 오류', admin_terminated:'관리자 종료'}};
            const bytes = n => ({{1536:'1.5 KiB', 3072:'3.0 KiB', 1572864:'1.5 MiB'}})[n];
            return rows.length === 3 && items.every(item => {{
              const row = rows.find(r => r.querySelector('td')?.textContent === item.session_id);
              if (!row) return false;
              const c = row.querySelectorAll('td');
              return c.length === 4 && c[1].textContent === causes[item.close_cause] &&
                c[2].textContent === localTime(item.ended_at) &&
                JSON.stringify([...row.querySelectorAll('.cell-value')].map(n => n.textContent)) ===
                  JSON.stringify([bytes(item.bytes_viewer_to_target),bytes(item.bytes_target_to_viewer)]) &&
                JSON.stringify([...row.querySelectorAll('.cell-label')].map(n => n.textContent)) ===
                  JSON.stringify(['Viewer->대상','대상->Viewer']) &&
                !row.querySelector('a,button,[role=button]');
            }});
            }})()"#,
        )),
        json!(true),
        "all three actual close causes retain local timestamps and directional bytes without historical actions",
    );
    assert_clean(&mut browser);
    assert_list_counts(&mut browser, [Some(0), Some(0), Some(3)], "ko");
    assert!(server.proxy.traffic().iter().all(|entry| !entry.injected));
}

#[cfg(target_os = "linux")]
struct Backlog {
    listener: TcpListener,
    _fillers: Vec<TcpStream>,
}

#[cfg(target_os = "linux")]
impl Backlog {
    fn saturated() -> Self {
        let runtime = must(
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build(),
        );
        let listener = runtime.block_on(async {
            let socket = must(tokio::net::TcpSocket::new_v4());
            must(socket.bind(SocketAddr::from(([127, 0, 0, 1], 0))));
            must(must(socket.listen(1)).into_std())
        });
        let address = must(listener.local_addr());
        let mut fillers = Vec::new();
        for _ in 0..64 {
            match TcpStream::connect_timeout(&address, Duration::from_millis(100)) {
                Ok(stream) => fillers.push(stream),
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                    ) =>
                {
                    assert!(!fillers.is_empty());
                    return Self {
                        listener,
                        _fillers: fillers,
                    };
                }
                Err(error) => panic!("saturating target backlog: {error:?}"),
            }
        }
        panic!("OS listen backlog did not saturate");
    }
}

pub fn overview_connection_counts() {
    let server = Server::start("overview-counts", Some(60));
    let admin = server.login();
    let mut browser = Browser::start("overview-counts", &["ko-KR"]);
    browser.login(&server);
    ready(&mut browser);

    // Delay only the other real list requests before forwarding. They cannot
    // transiently change Lm while the independently observed summary is read.
    for path in [
        "/api/v1/waiting",
        "/api/v1/sessions",
        "/api/v1/closed-sessions",
    ] {
        server
            .proxy
            .set("GET", path, Fault::DelayBefore(Duration::from_secs(4)));
    }
    #[cfg(target_os = "linux")]
    let backlog = Backlog::saturated();
    #[cfg(target_os = "linux")]
    let target = must(backlog.listener.local_addr());
    #[cfg(not(target_os = "linux"))]
    let target = must(must(TcpListener::bind("127.0.0.1:0")).local_addr());
    let mut observed_in_browser = false;
    for _ in 0..5 {
        // Each attempt owns the whole raw-query and rendered-DOM observation.
        // A missed Windows refusal window cannot be repaired by polling the
        // same already-refused socket. Recreate every held socket as well, so
        // H and U do not expire while a later attempt is running.
        let mut management = connect(server.ports[0]);
        let mut status = connect(server.ports[1]);
        must(management.write_all(b"GET /"));
        must(status.write_all(b"GET /"));
        let h1 = server.handshaking();
        let h2 = server.handshaking();
        let mut connecting = server.handshaking();
        must(connecting.write_all(&initial(&target.to_string())));
        #[cfg(target_os = "linux")]
        let window = Duration::from_secs(10);
        #[cfg(not(target_os = "linux"))]
        let window = Duration::from_millis(1500);
        let deadline = Instant::now() + window;
        let mut observed = None;
        while Instant::now() < deadline {
            let summary = server.query(&admin, "/api/v1/summary");
            if summary["counts"]["handshaking"] == 2
                && summary["counts"]["target_connecting"] == 1
                && summary["usage"]["relay_connections"]["current"] == 4
                && summary["usage"]["admin_connections"]["current"] == 3
            {
                observed = Some(summary);
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        if let Some(observed) = observed {
            let before = server.proxy.count("GET", "/api/v1/summary");
            browser.click("#operations-refresh");
            let expected = format!(
                r#"(() => {{
                const raw = {observed};
                const summary = document.querySelector('.grid > .card:first-child');
                if (summary?.querySelector('.card-status')?.textContent.trim() !== '') return false;
                const stats = [...document.querySelectorAll('.summary-grid .stat-value')].map(n => n.textContent);
                const gauges = [...document.querySelectorAll('.gauge')];
                const count = n => new Intl.NumberFormat().format(n);
                return stats[2] === count(raw.counts.waiting) && stats[3] === count(raw.counts.active) &&
                  stats[4] === count(raw.counts.handshaking) && stats[5] === count(raw.counts.target_connecting) &&
                  [raw.usage.relay_connections,raw.usage.admin_connections].every((usage,i) => {{
                    const gauge = gauges[i];
                    return gauge?.getAttribute('aria-valuenow') === String(usage.current) &&
                      gauge.getAttribute('aria-valuemax') === String(usage.limit) &&
                      gauge.querySelector('.gauge-value')?.textContent === count(usage.current) &&
                      gauge.querySelector('.gauge-max')?.textContent === '상한 ' + count(usage.limit);
                  }});
                }})()"#,
            );
            while Instant::now() < deadline {
                let fresh_response = server
                    .proxy
                    .traffic()
                    .iter()
                    .filter(|item| item.method == "GET" && item.path == "/api/v1/summary")
                    .nth(before)
                    .is_some_and(|item| {
                        item.completed.is_some()
                            && item.forwarded
                            && !item.injected
                            && item.status == Some(200)
                    });
                if fresh_response && browser.eval(&expected) == json!(true) {
                    observed_in_browser = true;
                    break;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        drop((management, status, h1, h2, connecting));
        if observed_in_browser {
            break;
        }
        // Late list responses from this attempt must settle before another
        // attempt can compare known Lm=3. No state or successful result is faked.
        wait_for("previous pending observation fully released", LIMIT, || {
            let summary = server.query(&admin, "/api/v1/summary");
            summary["counts"]["handshaking"] == 0
                && summary["counts"]["target_connecting"] == 0
                && summary["usage"]["relay_connections"]["current"] == 0
                && summary["usage"]["admin_connections"]["current"] == 1
                && server
                    .proxy
                    .traffic()
                    .iter()
                    .all(|item| item.completed.is_some())
        });
    }
    assert!(
        observed_in_browser,
        "no complete raw-query and browser observation of the real pending connection"
    );
    assert_eq!(
        browser.eval(
            r#"[...document.querySelectorAll('.card-description')].map(n => n.textContent)"#,
        ),
        json!(["중계 측 연결 소켓의 현재 수와 상한입니다.", "관리·상태 리스너의 연결 소켓 현재 수와 상한입니다."]),
    );
    assert_eq!(server.proxy.injected_count("GET", "/api/v1/summary"), 0);
    server.proxy.clear();
    assert_unavailable_summary(&server, &mut browser);
}

pub(super) fn assert_real_transport(browser: &mut Browser, session: &Value) {
    assert_eq!(browser.eval(&format!(r#"(() => {{
        const s={session};
        const link=[...document.querySelectorAll('a[href^="/#/sessions/"]')].find(a=>a.textContent===s.session_id);
        const row=link?.closest('tr');
        return !!row && row.textContent.includes(s.viewer_transport.toUpperCase()) &&
            row.textContent.includes(s.target_transport.toUpperCase()) &&
            row.textContent.includes('→');
    }})()"#)),json!(true),"active list shows both actual transports: {session}; {}",browser.text());
}
