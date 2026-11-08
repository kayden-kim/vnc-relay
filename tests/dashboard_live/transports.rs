//! Actual Mode I / Mode II TCP, WS and WSS streams and trusted-proxy clients.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::Arc;

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::{Message, WebSocket, client, client::IntoClientRequest};

use super::live::{Admin, Browser, LIMIT, Server, connect, initial, must, wait_for};

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
                connect(server.ports[3])
            });
        }
        let port = if transport == "wss" {
            wss
        } else {
            server.ws_port
        };
        let socket = connect(port);
        let wire = if transport == "wss" {
            let mut roots = RootCertStore::empty();
            for cert in must(CertificateDer::pem_file_iter(fixture("root-ca.pem"))) {
                must(roots.add(must(cert)));
            }
            let config = must(
                ClientConfig::builder_with_provider(Arc::new(
                    rustls::crypto::aws_lc_rs::default_provider(),
                ))
                .with_safe_default_protocol_versions(),
            )
            .with_root_certificates(roots)
            .with_no_client_auth();
            Wire::Tls(Box::new(StreamOwned::new(
                must(ClientConnection::new(
                    Arc::new(config),
                    must(ServerName::try_from("localhost")),
                )),
                socket,
            )))
        } else {
            Wire::Tcp(socket)
        };
        let path = if viewer { "/relay" } else { "/relay/server" };
        let mut request = must(format!("ws://localhost:{port}{path}").into_client_request());
        if forwarded {
            request.headers_mut().insert(
                "X-Forwarded-For",
                must(if viewer { "192.0.2.41" } else { "192.0.2.42" }.parse()),
            );
        }
        let (ws, response) = must(client(request, wire));
        assert_eq!(response.status().as_u16(), 101);
        let mut endpoint = Self::Ws(Box::new(ws));
        if viewer {
            assert_eq!(endpoint.receive(12), b"RFB 000.000\n");
        }
        endpoint
    }
    fn send(&mut self, data: &[u8]) {
        match self {
            Self::Tcp(s) => must(s.write_all(data)),
            Self::Ws(s) => must(s.send(Message::Binary(data.to_vec().into()))),
        }
    }
    fn receive(&mut self, len: usize) -> Vec<u8> {
        match self {
            Self::Tcp(s) => {
                let mut data = vec![0; len];
                must(s.read_exact(&mut data));
                data
            }
            Self::Ws(s) => {
                let mut bytes = Vec::with_capacity(len);
                while bytes.len() < len {
                    match must(s.read()) {
                        Message::Binary(data) => bytes.extend_from_slice(&data),
                        Message::Ping(_) => must(s.flush()),
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
    target.send(&initial(&format!("ID:{id}")));
    wait_for("real matrix server waiting", LIMIT, || {
        server.query(admin, "/api/v1/waiting")["items"]
            .as_array()
            .is_some_and(|items| items.iter().any(|i| i["relay_id"] == id))
    });
    let mut viewer = Endpoint::open(server, wss, viewer_transport, true, forwarded[0]);
    viewer.send(&initial(&format!("ID:{id}")));
    let mut session = Value::Null;
    wait_for("real matrix pair active", LIMIT, || {
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
    wait_for("matrix relay released", LIMIT, || {
        server.query(admin, "/api/v1/sessions")["items"]
            .as_array()
            .is_some_and(|items| items.is_empty())
    });
}

pub fn real_transport_matrix_and_forwarded_addresses() {
    for trust in [[false, false], [true, false], [false, true], [true, true]] {
        let reservation = must(TcpListener::bind("127.0.0.1:0"));
        let wss = must(reservation.local_addr()).port();
        let server = Server::start_with_config_transform(
            "dash-062",
            Some(60),
            false,
            |config, _| {
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
            },
        );
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
