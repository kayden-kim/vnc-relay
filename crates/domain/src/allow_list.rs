//! Source IP allow list items (config-v1 §1.3).

use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use crate::peer_ip::PeerIp;

/// One IP item: a single address, a CIDR block, or an inclusive range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpItem {
    Single(IpAddr),
    Cidr { network: IpAddr, prefix: u8 },
    Range { start: IpAddr, end: IpAddr },
}

/// Why an IP item string is not valid. Each kind maps to one config error
/// code (config-v1 §1.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpItemError {
    /// Bad notation, spaces, empty string, wildcard, prefix out of range,
    /// non-zero host bits, or an address inside `::ffff:0:0/96`.
    Invalid,
    /// Range start is greater than range end.
    RangeReversed,
    /// Range mixes IPv4 and IPv6.
    RangeFamilyMixed,
}

impl IpItem {
    pub fn parse(text: &str) -> Result<Self, IpItemError> {
        let item = if let Some((address, prefix)) = text.split_once('/') {
            let network = parse_address(address)?;
            let prefix = parse_prefix(prefix, network)?;
            if host_bits(network, prefix) != 0 {
                return Err(IpItemError::Invalid);
            }
            Self::Cidr { network, prefix }
        } else if let Some((start, end)) = text.split_once('-') {
            let start = parse_address(start)?;
            let end = parse_address(end)?;
            if start.is_ipv4() != end.is_ipv4() {
                return Err(IpItemError::RangeFamilyMixed);
            }
            if start > end {
                return Err(IpItemError::RangeReversed);
            }
            Self::Range { start, end }
        } else {
            Self::Single(parse_address(text)?)
        };
        if item.written_addresses().any(is_ipv4_mapped) {
            return Err(IpItemError::Invalid);
        }
        Ok(item)
    }

    /// True when this one item covers every address of its family, such as
    /// `0.0.0.0/0` or `::-ffff:...:ffff` (config-v1 §1.3, warning `allow_all_source`).
    pub fn covers_whole_family(&self) -> bool {
        match *self {
            Self::Single(_) => false,
            Self::Cidr { prefix, .. } => prefix == 0,
            Self::Range { start, end } => match (start, end) {
                (IpAddr::V4(start), IpAddr::V4(end)) => {
                    start == Ipv4Addr::UNSPECIFIED && end == Ipv4Addr::BROADCAST
                }
                (IpAddr::V6(start), IpAddr::V6(end)) => {
                    start == Ipv6Addr::UNSPECIFIED && u128::from(end) == u128::MAX
                }
                _ => false,
            },
        }
    }

    /// True when `ip` is this address, inside this block, or inside this
    /// inclusive range. An item of the other address family never matches.
    pub fn contains(&self, ip: IpAddr) -> bool {
        match *self {
            Self::Single(address) => address == ip,
            Self::Cidr { network, prefix } => {
                same_family(network, ip) && (bits(ip) & !host_mask(ip, prefix)) == bits(network)
            }
            Self::Range { start, end } => {
                same_family(start, ip) && (bits(start)..=bits(end)).contains(&bits(ip))
            }
        }
    }

    fn written_addresses(&self) -> impl Iterator<Item = IpAddr> {
        let (first, second) = match *self {
            Self::Single(address) => (address, None),
            Self::Cidr { network, .. } => (network, None),
            Self::Range { start, end } => (start, Some(end)),
        };
        std::iter::once(first).chain(second)
    }
}

/// Canonical text of the item. Used where a contract asks for the item
/// string, for example the `item` field of `allow_all_source`.
impl fmt::Display for IpItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Single(address) => write!(f, "{address}"),
            Self::Cidr { network, prefix } => write!(f, "{network}/{prefix}"),
            Self::Range { start, end } => write!(f, "{start}-{end}"),
        }
    }
}

/// The allow list of one entry point. An empty list rejects every source.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AllowList {
    items: Vec<IpItem>,
}

impl AllowList {
    pub fn new(items: Vec<IpItem>) -> Self {
        Self { items }
    }

    pub fn items(&self) -> &[IpItem] {
        &self.items
    }

    /// Items that each cover a whole address family.
    pub fn whole_family_items(&self) -> impl Iterator<Item = &IpItem> {
        self.items.iter().filter(|item| item.covers_whole_family())
    }

    /// Admission of one source (RLY-R16): true when any item contains the
    /// peer IP. `PeerIp` has already turned an IPv4-mapped IPv6 peer into
    /// IPv4, and items never hold mapped addresses, so such a peer matches
    /// IPv4 items.
    pub fn permits(&self, peer: PeerIp) -> bool {
        self.items.iter().any(|item| item.contains(peer.ip()))
    }
}

/// Parses one address. IPv6 text must not use brackets, zone IDs, or the
/// dotted IPv4 tail. IPv4 parts must not have leading zeros.
pub(crate) fn parse_address(text: &str) -> Result<IpAddr, IpItemError> {
    if text.contains(':') {
        if text.contains('.') {
            return Err(IpItemError::Invalid);
        }
        text.parse::<Ipv6Addr>()
            .map(IpAddr::V6)
            .map_err(|_| IpItemError::Invalid)
    } else {
        text.parse::<Ipv4Addr>()
            .map(IpAddr::V4)
            .map_err(|_| IpItemError::Invalid)
    }
}

fn parse_prefix(text: &str, network: IpAddr) -> Result<u8, IpItemError> {
    let max = if network.is_ipv4() { 32 } else { 128 };
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(IpItemError::Invalid);
    }
    match text.parse::<u8>() {
        Ok(prefix) if prefix <= max => Ok(prefix),
        _ => Err(IpItemError::Invalid),
    }
}

fn host_bits(network: IpAddr, prefix: u8) -> u128 {
    bits(network) & host_mask(network, prefix)
}

/// The address as an integer; IPv4 uses the low 32 bits.
fn bits(address: IpAddr) -> u128 {
    match address {
        IpAddr::V4(address) => u128::from(u32::from(address)),
        IpAddr::V6(address) => u128::from(address),
    }
}

/// Ones in the host part after `prefix` bits, for the family of `address`.
fn host_mask(address: IpAddr, prefix: u8) -> u128 {
    let unused = if address.is_ipv4() { 96 } else { 0 };
    u128::MAX
        .checked_shr(unused + u32::from(prefix))
        .unwrap_or(0)
}

fn same_family(a: IpAddr, b: IpAddr) -> bool {
    a.is_ipv4() == b.is_ipv4()
}

fn is_ipv4_mapped(address: IpAddr) -> bool {
    matches!(address, IpAddr::V6(v6) if v6.to_ipv4_mapped().is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(text: &str) -> IpItem {
        match IpItem::parse(text) {
            Ok(item) => item,
            Err(error) => panic!("{text}: {error:?}"),
        }
    }

    fn err(text: &str) -> IpItemError {
        match IpItem::parse(text) {
            Ok(item) => panic!("{text} parsed as {item:?}"),
            Err(error) => error,
        }
    }

    #[test]
    fn accepts_single_cidr_and_range_in_both_families() {
        assert_eq!(ok("198.51.100.7").to_string(), "198.51.100.7");
        assert_eq!(ok("2001:DB8::1").to_string(), "2001:db8::1");
        assert_eq!(ok("198.51.100.0/24").to_string(), "198.51.100.0/24");
        assert_eq!(ok("2001:db8::/32").to_string(), "2001:db8::/32");
        assert_eq!(
            ok("203.0.113.10-203.0.113.20").to_string(),
            "203.0.113.10-203.0.113.20"
        );
        assert_eq!(
            ok("2001:db8::1-2001:db8::9").to_string(),
            "2001:db8::1-2001:db8::9"
        );
        assert_eq!(ok("192.0.2.5-192.0.2.5").to_string(), "192.0.2.5-192.0.2.5");
        assert_eq!(ok("192.0.2.1/32").to_string(), "192.0.2.1/32");
        assert_eq!(ok("2001:db8::1/128").to_string(), "2001:db8::1/128");
    }

    #[test]
    fn rejects_bad_notation() {
        for text in [
            "",
            " 192.0.2.1",
            "192.0.2.1 ",
            "*",
            "192.0.2.*",
            "192.0.2",
            "192.0.2.256",
            "192.0.02.1",
            "192.0.2.0/33",
            "2001:db8::/129",
            "192.0.2.0/",
            "192.0.2.0/+8",
            "192.0.2.1/24",
            "2001:db8::1/32",
            "[2001:db8::1]",
            "fe80::1%eth0",
            "::192.0.2.1",
            "192.0.2.1-",
            "192.0.2.1-192.0.2.2-192.0.2.3",
            "::ffff:192.0.2.1",
            "::ffff:c000:201",
            "::ffff:0:0/96",
            "::ffff:c000:201-::ffff:c000:209",
            "example.com",
        ] {
            assert_eq!(err(text), IpItemError::Invalid, "{text:?}");
        }
    }

    #[test]
    fn rejects_reversed_and_mixed_ranges() {
        assert_eq!(err("203.0.113.20-203.0.113.10"), IpItemError::RangeReversed);
        assert_eq!(err("2001:db8::9-2001:db8::1"), IpItemError::RangeReversed);
        assert_eq!(err("192.0.2.1-2001:db8::1"), IpItemError::RangeFamilyMixed);
        assert_eq!(err("2001:db8::1-192.0.2.1"), IpItemError::RangeFamilyMixed);
    }

    #[test]
    fn detects_items_covering_a_whole_family() {
        for text in [
            "0.0.0.0/0",
            "::/0",
            "0.0.0.0-255.255.255.255",
            "::-ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff",
        ] {
            assert!(ok(text).covers_whole_family(), "{text}");
        }
        for text in [
            "0.0.0.0/1",
            "128.0.0.0/1",
            "0.0.0.1-255.255.255.255",
            "0.0.0.0",
            "::/1",
        ] {
            assert!(!ok(text).covers_whole_family(), "{text}");
        }
        let list = AllowList::new(vec![ok("192.0.2.0/24"), ok("::/0")]);
        let whole: Vec<String> = list.whole_family_items().map(ToString::to_string).collect();
        assert_eq!(whole, ["::/0"]);
    }

    fn peer(text: &str) -> PeerIp {
        PeerIp::new(text.parse().unwrap_or_else(|e| panic!("parse {text}: {e}")))
    }

    fn list(items: &[&str]) -> AllowList {
        AllowList::new(items.iter().map(|text| ok(text)).collect())
    }

    /// spec: operations.md:OPS-040 partial
    #[test]
    fn permits_the_boundaries_of_each_item_and_nothing_just_outside() {
        let allow = list(&[
            "192.0.2.7",
            "198.51.100.0/30",
            "203.0.113.10-203.0.113.20",
            "2001:db8::7",
            "2001:db8:1::/127",
            "2001:db8:2::a-2001:db8:2::14",
        ]);
        for inside in [
            "192.0.2.7",
            "198.51.100.0",
            "198.51.100.3",
            "203.0.113.10",
            "203.0.113.20",
            "2001:db8::7",
            "2001:db8:1::",
            "2001:db8:1::1",
            "2001:db8:2::a",
            "2001:db8:2::14",
        ] {
            assert!(allow.permits(peer(inside)), "{inside}");
        }
        for outside in [
            "192.0.2.6",
            "192.0.2.8",
            "198.51.99.255",
            "198.51.100.4",
            "203.0.113.9",
            "203.0.113.21",
            "2001:db8::6",
            "2001:db8::8",
            "2001:db8:0:ffff:ffff:ffff:ffff:ffff",
            "2001:db8:1::2",
            "2001:db8:2::9",
            "2001:db8:2::15",
        ] {
            assert!(!allow.permits(peer(outside)), "{outside}");
        }
    }

    #[test]
    fn matches_only_items_of_the_same_address_family() {
        // 0.0.0.0/0 covers every IPv4 address but no IPv6 address, and the
        // IPv6 block whose low bits equal an IPv4 address does not match it.
        let allow = list(&["0.0.0.0/0", "::c000:207/128"]);
        assert!(allow.permits(peer("192.0.2.7")));
        assert!(!allow.permits(peer("2001:db8::1")));
        assert!(!list(&["::/0"]).permits(peer("192.0.2.7")));
        assert!(list(&["::/0"]).permits(peer("2001:db8::1")));
        assert!(!list(&["::-::ffff"]).permits(peer("0.0.0.1")));
    }

    #[test]
    fn checks_an_ipv4_mapped_peer_against_ipv4_items() {
        let allow = list(&["192.0.2.0/24"]);
        assert!(allow.permits(peer("::ffff:192.0.2.9")));
        assert!(!allow.permits(peer("::ffff:198.51.100.9")));
    }

    /// spec: operations.md:OPS-038 partial
    #[test]
    fn an_empty_list_rejects_every_source() {
        let allow = AllowList::default();
        assert!(!allow.permits(peer("192.0.2.1")));
        assert!(!allow.permits(peer("::1")));
    }
}
