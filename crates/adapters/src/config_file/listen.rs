//! `listen` address values (config-v4 §1.1 and the entry point default
//! ports of §1.2).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

/// Parses `"IP:port"` or `"[IPv6]:port"`. With `default_port`, an address
/// without a port is accepted as `"IPv4"` or `"[IPv6]"`. Host names, zone IDs,
/// unbracketed IPv6, and port 0 are rejected.
pub(super) fn parse_listen(text: &str, default_port: Option<u16>) -> Option<SocketAddr> {
    if text.contains('%') {
        return None;
    }
    if let Ok(address) = text.parse::<SocketAddr>() {
        return (address.port() != 0).then_some(address);
    }
    let port = default_port?;
    let ip = if let Some(inner) = text.strip_prefix('[') {
        IpAddr::V6(inner.strip_suffix(']')?.parse::<Ipv6Addr>().ok()?)
    } else {
        IpAddr::V4(text.parse::<Ipv4Addr>().ok()?)
    };
    Some(SocketAddr::new(ip, port))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_address_and_port_forms() {
        assert_eq!(
            parse_listen("203.0.113.10:5901", None),
            "203.0.113.10:5901".parse().ok()
        );
        assert_eq!(
            parse_listen("[2001:db8::1]:8443", None),
            "[2001:db8::1]:8443".parse().ok()
        );
        assert_eq!(
            parse_listen("203.0.113.10", Some(5500)),
            "203.0.113.10:5500".parse().ok()
        );
        assert_eq!(
            parse_listen("[2001:db8::1]", Some(5901)),
            "[2001:db8::1]:5901".parse().ok()
        );
        assert_eq!(
            parse_listen("203.0.113.10:6000", Some(5901)),
            "203.0.113.10:6000".parse().ok()
        );
    }

    #[test]
    fn rejects_forms_outside_the_contract() {
        for (text, default_port) in [
            ("203.0.113.10", None),
            ("[2001:db8::1]", None),
            ("2001:db8::1", Some(5901)),
            ("2001:db8::1", None),
            ("203.0.113.10:0", None),
            ("203.0.113.10:65536", None),
            ("localhost:5901", None),
            ("localhost", Some(5901)),
            ("203.0.113.010:5901", None),
            ("[fe80::1%1]:5901", None),
            (" 203.0.113.10:5901", None),
            ("", Some(5901)),
        ] {
            assert_eq!(parse_listen(text, default_port), None, "{text:?}");
        }
    }
}
