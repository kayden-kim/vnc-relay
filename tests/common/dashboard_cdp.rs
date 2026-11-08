//! Dashboard actions on the shared CDP browser: real input, reload, viewport
//! and the real sign-in flow.

use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;

use super::cdp::Browser;
use super::{LIMIT, PASSWORD, Server, must};

impl Browser {
    pub fn reload(&mut self) {
        let previous =
            self.command("Page.getFrameTree", json!({}))["frameTree"]["frame"]["loaderId"].clone();
        self.command("Page.reload", json!({"ignoreCache":true}));
        let deadline = Instant::now() + LIMIT;
        while self.command("Page.getFrameTree", json!({}))["frameTree"]["frame"]["loaderId"]
            == previous
        {
            assert!(Instant::now() < deadline, "reload loader did not commit");
            thread::sleep(Duration::from_millis(20));
        }
        self.wait("document.readyState === 'complete'");
    }

    pub fn click(&mut self, selector: &str) {
        let selector = must(serde_json::to_string(selector));
        self.wait(&format!("(()=>{{const e=document.querySelector({selector});return !!e&&e.getBoundingClientRect().width>0}})()"));
        let point = self.eval(&format!("(()=>{{const e=document.querySelector({selector});e.scrollIntoView({{block:'center'}});const r=e.getBoundingClientRect();return {{x:r.x+r.width/2,y:r.y+r.height/2}}}})()"));
        let x = point["x"].as_f64().unwrap_or(0.0);
        let y = point["y"].as_f64().unwrap_or(0.0);
        self.command(
            "Input.dispatchMouseEvent",
            json!({"type":"mousePressed","x":x,"y":y,"button":"left","clickCount":1}),
        );
        self.command(
            "Input.dispatchMouseEvent",
            json!({"type":"mouseReleased","x":x,"y":y,"button":"left","clickCount":1}),
        );
    }

    pub fn fill(&mut self, selector: &str, value: &str) {
        let selector = must(serde_json::to_string(selector));
        self.wait(&format!("document.querySelector({selector}) !== null"));
        self.eval(&format!(
            "(()=>{{const e=document.querySelector({selector});e.focus();e.select()}})()"
        ));
        self.command("Input.insertText", json!({"text":value}));
    }

    pub fn key(&mut self, key: &str) {
        let code = match key {
            "Escape" => 27,
            "Enter" => 13,
            "Tab" => 9,
            _ => 0,
        };
        for kind in ["keyDown", "keyUp"] {
            self.command("Input.dispatchKeyEvent", json!({"type":kind,"key":key,"code":key,"windowsVirtualKeyCode":code,"nativeVirtualKeyCode":code,"text":if kind=="keyDown" && key=="Enter" {"\r"} else {""}}));
        }
    }

    pub fn wait_with_server(&mut self, expression: &str, server: &Server) {
        let deadline = Instant::now() + LIMIT;
        loop {
            if self.eval(expression).as_bool() == Some(true) {
                return;
            }
            let login_failed = self
                .eval(
                    "!!document.querySelector('[data-notice=request-failed],[data-notice=failed]')",
                )
                .as_bool()
                == Some(true);
            if login_failed || Instant::now() >= deadline {
                panic!(
                    "normal browser login/navigation failed; predicate={expression}; network={}; server={}",
                    self.network(),
                    server.diagnostics()
                );
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn login(&mut self, server: &Server) {
        self.navigate(&server.url("/login/"));
        self.fill("#login-account", "alice");
        self.fill("#login-password", PASSWORD);
        self.click("#login-submit");
        self.wait_with_server(
            "location.pathname === '/' && document.querySelector('#operations-refresh') !== null",
            server,
        );
        self.wait("!document.body.innerText.includes('Loading') && !document.body.innerText.includes('조회 중')");
    }

    pub fn clear_cookies(&mut self) {
        self.command("Network.clearBrowserCookies", json!({}));
    }

    pub fn set_width(&mut self, width: u32) {
        self.command(
            "Emulation.setDeviceMetricsOverride",
            json!({"width":width,"height":900,"deviceScaleFactor":1,"mobile":false}),
        );
    }
}
