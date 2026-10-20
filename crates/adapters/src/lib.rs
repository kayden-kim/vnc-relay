//! Adapters of VNC Relay: implementations of app ports and the outside I/O
//! (docs/architectures/code-map.md, layer "adapters").

pub mod accounts_file;
pub mod config_file;
pub mod crypto;
pub mod net;
pub mod ops_log;
pub mod status_http;
mod toml_text;
