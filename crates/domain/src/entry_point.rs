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

    /// Role of every connection this entry point accepts (ops-info-v1
    /// §1.4 `role`). WS and WSS entry points take only viewers.
    pub fn role(self) -> Role {
        match self {
            Self::TcpServer => Role::Server,
            Self::TcpViewer | Self::WsViewer | Self::WssViewer => Role::Viewer,
        }
    }
}

/// Role of a connection: the VNC viewer or the VNC server side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Viewer,
    Server,
}

impl Role {
    /// `role` value of operation events (ops-info-v1 §1.4).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Viewer => "viewer",
            Self::Server => "server",
        }
    }
}

impl fmt::Display for EntryPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
