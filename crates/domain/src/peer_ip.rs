//! Peer IP of a connection as the operation log and the source IP allow
//! list see it (ops-info-v1 §1.10, config-v2 §1.3).

use std::fmt;
use std::net::{IpAddr, SocketAddr};

/// The real socket peer IP. An IPv4-mapped IPv6 address becomes the IPv4
/// address, so the log shows the address that the allow list compares.
/// It has no port and no zone index. `Display` writes dotted decimal for
/// IPv4 and the lowercase RFC 5952 form for IPv6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PeerIp(IpAddr);

impl PeerIp {
    pub fn new(ip: IpAddr) -> Self {
        Self(ip.to_canonical())
    }

    pub fn ip(self) -> IpAddr {
        self.0
    }
}

impl From<SocketAddr> for PeerIp {
    fn from(addr: SocketAddr) -> Self {
        Self::new(addr.ip())
    }
}

impl fmt::Display for PeerIp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv6Addr, SocketAddrV6};

    fn shown(text: &str) -> String {
        let ip: IpAddr = text.parse().unwrap_or_else(|e| panic!("parse {text}: {e}"));
        PeerIp::new(ip).to_string()
    }

    #[test]
    fn writes_ipv4_mapped_addresses_as_ipv4() {
        assert_eq!(shown("::ffff:192.0.2.1"), "192.0.2.1");
        assert_eq!(shown("::FFFF:c000:0201"), "192.0.2.1");
        assert_eq!(shown("192.0.2.1"), "192.0.2.1");
        // Only the mapped block changes; other IPv6 addresses stay IPv6.
        assert_eq!(shown("64:ff9b::c000:201"), "64:ff9b::c000:201");
    }

    #[test]
    fn writes_ipv6_in_lowercase_rfc5952_form() {
        assert_eq!(
            shown("2001:0DB8:0000:0000:0000:0000:0000:0005"),
            "2001:db8::5"
        );
        // The first of two equal zero runs is shortened.
        assert_eq!(shown("2001:db8:0:0:1:0:0:1"), "2001:db8::1:0:0:1");
        // The longer zero run wins.
        assert_eq!(shown("2001:0:0:1:0:0:0:1"), "2001:0:0:1::1");
        // A single zero group is not shortened.
        assert_eq!(shown("2001:db8:0:1:1:1:1:1"), "2001:db8:0:1:1:1:1:1");
    }

    #[test]
    fn leaves_out_port_and_zone_index() {
        let addr = SocketAddr::V6(SocketAddrV6::new(
            Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1),
            5901,
            0,
            3,
        ));
        assert_eq!(PeerIp::from(addr).to_string(), "fe80::1");
        let addr: SocketAddr = "[::ffff:198.51.100.21]:5500"
            .parse()
            .unwrap_or_else(|e| panic!("parse: {e}"));
        assert_eq!(PeerIp::from(addr).to_string(), "198.51.100.21");
    }
}
