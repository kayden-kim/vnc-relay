//! Adapters of VNC Relay: implementations of app ports and the outside I/O
//! (docs/architectures/code-map.md, layer "adapters").

pub mod accounts_file;
mod admission;
pub mod config_file;
pub mod crypto;
pub mod management_http;
pub mod net;
pub mod ops_log;
mod request_deadline;
pub mod status_http;
pub mod terminal;
pub mod tls;
mod tls_listener;
mod toml_text;
mod upgrade;
pub mod websocket;
