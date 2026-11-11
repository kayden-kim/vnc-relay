//! Captures the dashboard pictures of the operator help (docs/how-to/publish-help.md
//! section 3.4). This is a tool, not a scenario: its row is `#[ignore]` and runs
//! only on request. The server uses loopback addresses and the harness test
//! accounts only, as docs/help/AGENTS.md requires for pictures. It runs on
//! Linux only: the settings picture shows the server's absolute file paths,
//! and only the default `/tmp` directory keeps them free of user and machine names.

use std::fs;
use std::path::Path;

use serde_json::json;

use super::live::{Browser, Server, must};

/// Size of every picture in CSS pixels, at device scale 1.
const WIDTH: u32 = 1280;
const HEIGHT: u32 = 800;

pub fn capture() {
    let server = Server::start_with_config("help-capture", Some(5), true);
    assert!(
        server.dir().starts_with("/tmp"),
        "run the capture with TMPDIR unset: the settings picture would show {}",
        server.dir().display()
    );
    let admin = server.login();
    // A closed session for "recently closed", two active sessions, and one
    // waiting connection, so that every operations list has a row.
    let mut closed = server.pair(&admin, "II", "2001");
    closed.transfer(4_096, 65_536);
    closed.close();
    let mut mode2 = server.pair(&admin, "II", "1234");
    mode2.transfer(65_536, 1_048_576);
    let mut mode1 = server.pair(&admin, "I", "");
    mode1.transfer(8_192, 262_144);
    let _waiting = server.waiting("5678");

    let help = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("docs")
        .join("help");
    for (language, languages) in [("ko", ["ko-KR", "ko"]), ("en", ["en-US", "en"])] {
        let images = help.join(language).join("src").join("images");
        must(fs::create_dir_all(&images));
        let mut browser = Browser::start(&format!("help-capture-{language}"), &languages);
        browser.command(
            "Emulation.setDeviceMetricsOverride",
            json!({"width":WIDTH,"height":HEIGHT,"deviceScaleFactor":1,"mobile":false}),
        );

        browser.navigate(&server.url("/login/"));
        browser.fill("#login-account", "alice");
        shoot(
            &mut browser,
            &images.join("login.png"),
            "document.querySelector('#login-submit') !== null",
        );

        browser.login(&server);
        shoot(
            &mut browser,
            &images.join("operations.png"),
            &format!(
                "document.querySelector('#operations-refresh')?.disabled === false \
                 && document.body.innerText.includes('5678') && document.body.innerText.includes('{}')",
                mode2.sid
            ),
        );

        browser.navigate(&server.url(&format!("/#/sessions/{}", mode2.sid)));
        shoot(
            &mut browser,
            &images.join("session.png"),
            "document.querySelector('#session-terminate')?.disabled === false",
        );

        browser.navigate(&server.url("/#/logs"));
        shoot(
            &mut browser,
            &images.join("logs.png"),
            "document.querySelector('#logs-refresh')?.disabled === false \
             && !document.querySelector('.log-meta')?.textContent.includes('확인 불가') \
             && !document.querySelector('.log-meta')?.textContent.includes('Unavailable')",
        );

        browser.navigate(&server.url("/#/settings"));
        shoot(
            &mut browser,
            &images.join("settings.png"),
            "document.querySelector('#settings-edit')?.disabled === false",
        );

        browser.navigate(&server.url("/#/settings/edit"));
        shoot(
            &mut browser,
            &images.join("settings-edit.png"),
            "document.querySelector('#settings-review') !== null",
        );
    }
}

/// Waits until `ready` holds, no screen is loading, and the fonts are loaded.
/// Then removes focus and hover from the page and writes a viewport PNG to `path`.
fn shoot(browser: &mut Browser, path: &Path, ready: &str) {
    browser.wait(&format!(
        "({ready}) && !document.body.innerText.includes('Loading') \
         && !document.body.innerText.includes('조회 중') && document.fonts.status === 'loaded'"
    ));
    browser.eval("document.activeElement?.blur(); window.scrollTo(0, 0); true");
    browser.command(
        "Input.dispatchMouseEvent",
        json!({"type":"mouseMoved","x":0,"y":0}),
    );
    let shot = browser.command("Page.captureScreenshot", json!({"format":"png"}));
    let data = shot["data"]
        .as_str()
        .unwrap_or_else(|| panic!("screenshot data missing for {}", path.display()));
    must(fs::write(path, decode_base64(data)));
}

/// Standard base64 with padding, as CDP returns screenshot data.
fn decode_base64(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let mut buffer = 0_u32;
    let mut bits = 0;
    for byte in text.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            _ => panic!("screenshot data is not base64"),
        };
        buffer = (buffer << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    out
}
