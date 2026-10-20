//! Entry points: relay listeners told apart by role and transport.

use std::fmt;

/// One of the four relay listeners. Management and status listeners are not
/// entry points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EntryPoint {
    TcpViewer,
    TcpServer,
    WsViewer,
    WssViewer,
}

impl EntryPoint {
    pub const ALL: [Self; 4] = [
        Self::TcpViewer,
        Self::TcpServer,
        Self::WsViewer,
        Self::WssViewer,
    ];

    /// Name used in the config table (`relay.<name>`) and in the `listener`
    /// field of operation events (ops-info-v1 §1.5).
    pub fn name(self) -> &'static str {
        match self {
            Self::TcpViewer => "tcp_viewer",
            Self::TcpServer => "tcp_server",
            Self::WsViewer => "ws_viewer",
            Self::WssViewer => "wss_viewer",
        }
    }

    /// Port used when a TCP entry point's `listen` gives only an address
    /// (config-v1 §1.2). WS and WSS entry points have no default port.
    pub fn default_port(self) -> Option<u16> {
        match self {
            Self::TcpViewer => Some(5901),
            Self::TcpServer => Some(5500),
            Self::WsViewer | Self::WssViewer => None,
        }
    }
}

impl fmt::Display for EntryPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
