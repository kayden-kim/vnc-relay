//! Use cases of VNC Relay. External I/O goes through port traits
//! (docs/architectures/code-map.md, layer "app").

pub mod accounts;
pub mod events;
pub mod ip_info;
pub mod lifecycle;
pub mod management;
pub mod metrics;
pub mod ports;
pub mod relay;
pub mod reload;
pub mod settings;
pub mod sources;
