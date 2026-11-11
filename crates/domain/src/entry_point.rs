//! Relay listeners and the role-specific entry points they serve.

use crate::session::Transport;

/// One bound relay socket (ops-info-v2 §1.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Listener {
    TcpViewer,
    TcpServer,
    Ws,
    Wss,
}

impl Listener {
    /// Relay listeners in startup bind order.
    pub const ALL: [Self; 4] = [Self::TcpViewer, Self::TcpServer, Self::Ws, Self::Wss];

    /// Stable operation-log listener name.
    pub fn name(self) -> &'static str {
        match self {
            Self::TcpViewer => "tcp_viewer",
            Self::TcpServer => "tcp_server",
            Self::Ws => "ws",
            Self::Wss => "wss",
        }
    }

    /// Default port of TCP listeners only (config-v4 §1.2).
    pub fn default_port(self) -> Option<u16> {
        match self {
            Self::TcpViewer => Some(5901),
            Self::TcpServer => Some(5500),
            Self::Ws | Self::Wss => None,
        }
    }

    /// Role known before reading bytes; WebSocket paths choose their role.
    pub fn fixed_role(self) -> Option<Role> {
        match self {
            Self::TcpViewer => Some(Role::Viewer),
            Self::TcpServer => Some(Role::Server),
            Self::Ws | Self::Wss => None,
        }
    }

    /// Transport shared by both roles of this listener.
    pub fn transport(self) -> Transport {
        match self {
            Self::TcpViewer | Self::TcpServer => Transport::Tcp,
            Self::Ws => Transport::Ws,
            Self::Wss => Transport::Wss,
        }
    }
}

/// A listener and role pair (config-v4 §1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EntryPoint {
    TcpViewer,
    TcpServer,
    WsViewer,
    WsServer,
    WssViewer,
    WssServer,
}

impl EntryPoint {
    /// All supported listener and role pairs.
    pub const ALL: [Self; 6] = [
        Self::TcpViewer,
        Self::TcpServer,
        Self::WsViewer,
        Self::WsServer,
        Self::WssViewer,
        Self::WssServer,
    ];

    /// Socket accepting this entry point.
    pub fn listener(self) -> Listener {
        match self {
            Self::TcpViewer => Listener::TcpViewer,
            Self::TcpServer => Listener::TcpServer,
            Self::WsViewer | Self::WsServer => Listener::Ws,
            Self::WssViewer | Self::WssServer => Listener::Wss,
        }
    }

    /// Role selected by the TCP listener or WebSocket request path.
    pub fn role(self) -> Role {
        match self {
            Self::TcpViewer | Self::WsViewer | Self::WssViewer => Role::Viewer,
            Self::TcpServer | Self::WsServer | Self::WssServer => Role::Server,
        }
    }

    /// Transport used by this role.
    pub fn transport(self) -> Transport {
        self.listener().transport()
    }

    /// Resolves a supported listener/role pair; TCP roles cannot be reversed.
    pub fn from_parts(listener: Listener, role: Role) -> Option<Self> {
        match (listener, role) {
            (Listener::TcpViewer, Role::Viewer) => Some(Self::TcpViewer),
            (Listener::TcpServer, Role::Server) => Some(Self::TcpServer),
            (Listener::Ws, Role::Viewer) => Some(Self::WsViewer),
            (Listener::Ws, Role::Server) => Some(Self::WsServer),
            (Listener::Wss, Role::Viewer) => Some(Self::WssViewer),
            (Listener::Wss, Role::Server) => Some(Self::WssServer),
            _ => None,
        }
    }

    /// Full config-v4 table path, distinct from the listener log name.
    pub fn config_table(self) -> &'static str {
        match self {
            Self::TcpViewer => "relay.tcp_viewer",
            Self::TcpServer => "relay.tcp_server",
            Self::WsViewer => "relay.ws.viewer",
            Self::WsServer => "relay.ws.server",
            Self::WssViewer => "relay.wss.viewer",
            Self::WssServer => "relay.wss.server",
        }
    }
}

/// Role of a connection: the VNC viewer or the VNC server side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    Viewer,
    Server,
}

impl Role {
    /// Stable operation-log role value (ops-info-v2 §1.4).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Viewer => "viewer",
            Self::Server => "server",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_points_round_trip_through_listener_and_role() {
        for entry in EntryPoint::ALL {
            assert_eq!(
                EntryPoint::from_parts(entry.listener(), entry.role()),
                Some(entry)
            );
            assert_eq!(entry.transport(), entry.listener().transport());
        }
        assert_eq!(
            EntryPoint::from_parts(Listener::TcpViewer, Role::Server),
            None
        );
        assert_eq!(
            EntryPoint::from_parts(Listener::TcpServer, Role::Viewer),
            None
        );
        assert_eq!(Listener::Ws.fixed_role(), None);
        assert_eq!(Listener::Wss.fixed_role(), None);
    }
}
