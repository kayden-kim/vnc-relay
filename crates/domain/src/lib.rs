//! Business rules and values of VNC Relay.
//!
//! This crate does no I/O and depends on no external crate
//! (docs/architectures/code-map.md, layer "domain").

pub mod account;
pub mod allow_list;
pub mod entry_point;
pub mod origin;
pub mod target_policy;
