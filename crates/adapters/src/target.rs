//! Mode I DNS resolution and exact-address TCP connection.
//! Policy, candidate order, and the shared absolute C deadline belong to app;
//! this adapter never connects to a hostname (relay-session.md §4.1).

use std::io;
use std::net::{IpAddr, SocketAddr};

use tokio::net::{TcpStream, lookup_host};
use vnc_relay_app::ports::{TargetConnector, TargetError};
use vnc_relay_app::settings::KeepaliveSettings;

use crate::net::TcpRelayStream;

/// Resolves once and connects only to the supplied, already checked address.
#[derive(Debug, Clone, Copy)]
pub struct TcpTargetConnector {
    keepalive: KeepaliveSettings,
}

impl TcpTargetConnector {
    pub fn new(keepalive: KeepaliveSettings) -> Self {
        Self { keepalive }
    }
}

impl TargetConnector for TcpTargetConnector {
    type Stream = TcpRelayStream;

    async fn resolve(&self, name: &str) -> Result<Vec<IpAddr>, TargetError> {
        let addresses = lookup_host((name, 0))
            .await
            .map_err(|_| TargetError::ResolveFailed)?;
        Ok(addresses.map(|address| address.ip()).collect())
    }

    async fn connect(&self, address: SocketAddr) -> Result<Self::Stream, TargetError> {
        let stream = TcpStream::connect(address).await.map_err(|error| {
            if error.kind() == io::ErrorKind::ConnectionRefused {
                TargetError::ConnectRefused
            } else {
                TargetError::ConnectError
            }
        })?;
        TcpRelayStream::with_keepalive(stream, &self.keepalive)
            .map_err(|_| TargetError::KeepaliveFailed)
    }
}
