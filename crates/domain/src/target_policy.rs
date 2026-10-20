//! Mode I target allow items (config-v1 §1.5).

use std::fmt;

use crate::allow_list::{IpItem, IpItemError};
use crate::origin::is_dns_name;

/// The target part of an item: IP item or DNS name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetHost {
    Ip(IpItem),
    /// Lower-case name without a trailing dot.
    Name(String),
}

/// Inclusive port range. A single port has `start == end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortRange {
    pub start: u16,
    pub end: u16,
}

/// One `target:port` item of `mode1.allow_targets`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetItem {
    pub host: TargetHost,
    pub ports: PortRange,
}

/// Why a target item is not valid. All kinds map to the config error
/// `config_target_item_invalid`; the kind gives the detailed cause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetItemError {
    /// Not `target:port`, `[IPv6 item]:port`, or a missing part.
    Format,
    /// Port is not a decimal in 1..=65535, or is `*`.
    Port,
    /// Port range start is greater than its end.
    PortRangeReversed,
    /// The target looks like an IP item and is not a valid one.
    Ip(IpItemError),
    /// The target is not a valid DNS name.
    Name,
}

impl TargetItem {
    pub fn parse(text: &str) -> Result<Self, TargetItemError> {
        let (target, port, bracketed) = if let Some(rest) = text.strip_prefix('[') {
            let (target, after) = rest.split_once(']').ok_or(TargetItemError::Format)?;
            let port = after.strip_prefix(':').ok_or(TargetItemError::Format)?;
            (target, port, true)
        } else {
            let (target, port) = text.split_once(':').ok_or(TargetItemError::Format)?;
            if port.contains(':') {
                return Err(TargetItemError::Format);
            }
            (target, port, false)
        };
        if target.is_empty() {
            return Err(TargetItemError::Format);
        }
        let ports = parse_ports(port)?;
        let looks_like_ip = target.contains('/')
            || target.contains(':')
            || target
                .bytes()
                .all(|b| b.is_ascii_digit() || b == b'.' || b == b'-');
        let host = if bracketed || looks_like_ip {
            let item = IpItem::parse(target).map_err(TargetItemError::Ip)?;
            if bracketed != is_ipv6(&item) {
                // IPv6 items need brackets; IPv4 items must not have them.
                return Err(TargetItemError::Format);
            }
            TargetHost::Ip(item)
        } else {
            let name = target.strip_suffix('.').unwrap_or(target);
            if !name.is_ascii() || !is_dns_name(name) {
                return Err(TargetItemError::Name);
            }
            TargetHost::Name(name.to_ascii_lowercase())
        };
        Ok(Self { host, ports })
    }

    /// True when the target is an IP item that covers a whole address family
    /// (warning `allow_all_target`).
    pub fn covers_whole_family(&self) -> bool {
        matches!(&self.host, TargetHost::Ip(item) if item.covers_whole_family())
    }
}

/// Canonical text of the item, for the `item` field of `allow_all_target`.
impl fmt::Display for TargetItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.host {
            TargetHost::Ip(item) if is_ipv6(item) => write!(f, "[{item}]")?,
            TargetHost::Ip(item) => write!(f, "{item}")?,
            TargetHost::Name(name) => f.write_str(name)?,
        }
        if self.ports.start == self.ports.end {
            write!(f, ":{}", self.ports.start)
        } else {
            write!(f, ":{}-{}", self.ports.start, self.ports.end)
        }
    }
}

/// The Mode I target allow policy. An empty policy rejects every target.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TargetPolicy {
    items: Vec<TargetItem>,
}

impl TargetPolicy {
    pub fn new(items: Vec<TargetItem>) -> Self {
        Self { items }
    }

    pub fn items(&self) -> &[TargetItem] {
        &self.items
    }

    pub fn whole_family_items(&self) -> impl Iterator<Item = &TargetItem> {
        self.items.iter().filter(|item| item.covers_whole_family())
    }
}

fn is_ipv6(item: &IpItem) -> bool {
    match *item {
        IpItem::Single(address) => address.is_ipv6(),
        IpItem::Cidr { network, .. } => network.is_ipv6(),
        IpItem::Range { start, .. } => start.is_ipv6(),
    }
}

fn parse_ports(text: &str) -> Result<PortRange, TargetItemError> {
    let (start, end) = match text.split_once('-') {
        Some((start, end)) => (parse_port(start)?, parse_port(end)?),
        None => {
            let port = parse_port(text)?;
            (port, port)
        }
    };
    if start > end {
        return Err(TargetItemError::PortRangeReversed);
    }
    Ok(PortRange { start, end })
}

fn parse_port(text: &str) -> Result<u16, TargetItemError> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(TargetItemError::Port);
    }
    match text.parse::<u16>() {
        Ok(port) if port != 0 => Ok(port),
        _ => Err(TargetItemError::Port),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(text: &str) -> TargetItem {
        match TargetItem::parse(text) {
            Ok(item) => item,
            Err(error) => panic!("{text}: {error:?}"),
        }
    }

    fn err(text: &str) -> TargetItemError {
        match TargetItem::parse(text) {
            Ok(item) => panic!("{text} parsed as {item:?}"),
            Err(error) => error,
        }
    }

    #[test]
    fn accepts_contract_examples() {
        for (text, canonical) in [
            ("192.0.2.20:5900", "192.0.2.20:5900"),
            ("192.0.2.0/24:5900-5905", "192.0.2.0/24:5900-5905"),
            ("[2001:db8::/32]:5900", "[2001:db8::/32]:5900"),
            (
                "[2001:db8::1-2001:db8::9]:5900-5901",
                "[2001:db8::1-2001:db8::9]:5900-5901",
            ),
            ("pc1.example:5900", "pc1.example:5900"),
            ("PC1.Example.:5900", "pc1.example:5900"),
            ("192.0.2.1-192.0.2.9:1-65535", "192.0.2.1-192.0.2.9:1-65535"),
        ] {
            assert_eq!(ok(text).to_string(), canonical, "{text}");
        }
        assert_eq!(
            ok("pc1.example:5900").host,
            TargetHost::Name("pc1.example".to_owned())
        );
    }

    #[test]
    fn rejects_items_with_detailed_cause() {
        for (text, expected) in [
            ("192.0.2.20", TargetItemError::Format),
            ("2001:db8::1:5900", TargetItemError::Format),
            ("[2001:db8::1]5900", TargetItemError::Format),
            ("[192.0.2.1]:5900", TargetItemError::Format),
            (":5900", TargetItemError::Format),
            ("192.0.2.20:*", TargetItemError::Port),
            ("192.0.2.20:", TargetItemError::Port),
            ("192.0.2.20:0", TargetItemError::Port),
            ("192.0.2.20:65536", TargetItemError::Port),
            ("192.0.2.20:5905-5900", TargetItemError::PortRangeReversed),
            ("192.0.2:5900", TargetItemError::Ip(IpItemError::Invalid)),
            (
                "192.0.2.9-192.0.2.1:5900",
                TargetItemError::Ip(IpItemError::RangeReversed),
            ),
            (
                "[2001:db8::1-192.0.2.1]:5900",
                TargetItemError::Ip(IpItemError::RangeFamilyMixed),
            ),
            ("*.example:5900", TargetItemError::Name),
            ("-pc.example:5900", TargetItemError::Name),
            ("피시.example:5900", TargetItemError::Name),
        ] {
            assert_eq!(err(text), expected, "{text:?}");
        }
    }

    #[test]
    fn detects_ip_items_covering_a_whole_family() {
        assert!(ok("0.0.0.0/0:5900").covers_whole_family());
        assert!(ok("[::/0]:1-65535").covers_whole_family());
        assert!(!ok("192.0.2.0/24:5900").covers_whole_family());
        assert!(!ok("pc1.example:5900").covers_whole_family());
    }
}
