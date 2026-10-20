//! Adapters of VNC Relay: implementations of app ports and the outside I/O
//! (docs/architectures/code-map.md, layer "adapters").

pub mod accounts_file;
pub mod config_file;
pub mod net;
mod toml_text;
