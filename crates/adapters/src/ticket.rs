//! Access ticket adapters: the public key file (config-v4 §1.11) and the
//! ticket signature check (websocket-relay-v2 §1.9).
//! Design: docs/architectures/relay-access-ticket.md.

mod jwks;
pub(crate) mod jws;

pub use jwks::TicketJwksStore;
