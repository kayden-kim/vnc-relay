//! Mode I DNS resolution and exact-address TCP connection.
//! Policy, candidate order, and the shared absolute C deadline belong to app;
//! this adapter never connects to a hostname (relay-session.md §4.1).

use std::io;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use tokio::net::{TcpStream, lookup_host};
use vnc_relay_app::ports::{TargetConnector, TargetError};
use vnc_relay_app::reload::ports::RuntimeSettingsSource;

use crate::net::TcpRelayStream;

/// Resolves once and connects only to the supplied, already checked address.
#[derive(Clone)]
pub struct TcpTargetConnector {
    runtime: Arc<dyn RuntimeSettingsSource>,
}

impl TcpTargetConnector {
    pub fn new(runtime: Arc<dyn RuntimeSettingsSource>) -> Self {
        Self { runtime }
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
        let mut keepalive = None;
        self.runtime
            .with_current(&mut |settings| keepalive = Some(settings.keepalive));
        let keepalive = keepalive.ok_or(TargetError::KeepaliveFailed)?;
        let stream = TcpStream::connect(address).await.map_err(|error| {
            if error.kind() == io::ErrorKind::ConnectionRefused {
                TargetError::ConnectRefused
            } else {
                TargetError::ConnectError
            }
        })?;
        TcpRelayStream::with_keepalive(stream, &keepalive).map_err(|_| TargetError::KeepaliveFailed)
    }
}
