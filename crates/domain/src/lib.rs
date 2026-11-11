//! Business rules and values of VNC Relay.
//!
//! This crate does no I/O and depends on no external crate
//! (docs/architectures/code-map.md, layer "domain").

pub mod account;
pub mod admin_session;
pub mod allow_list;
pub mod budget;
pub mod conn_id;
pub mod entry_point;
pub mod forwarded_for;
pub mod handshake;
pub mod login_limit;
pub mod matching;
pub mod origin;
pub mod peer_ip;
pub mod recent_closed;
pub mod relay_id;
pub mod session;
pub mod target_policy;
pub mod ticket;
