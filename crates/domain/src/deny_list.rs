//! The relay-wide source IP deny list (config-v4 §1.2 `[relay.deny]`, §1.3).
//! See docs/decisions/ADR-0013-relay-source-deny-list.md.

use crate::allow_list::IpItem;
use crate::peer_ip::PeerIp;

/// One deny item and the text written in the config file. The text is kept
/// because the sources view returns covering items as written
/// (management-api-v1 §1.17 `deny_entries`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DenyEntry {
    text: String,
    item: IpItem,
}

impl DenyEntry {
    pub fn new(text: String, item: IpItem) -> Self {
        Self { text, item }
    }

    /// The item exactly as written in the file.
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn item(&self) -> &IpItem {
        &self.item
    }
}

/// The deny list shared by the six relay entry points. An empty list denies
/// nothing, the opposite of an empty allow list. Equality also compares the
/// written text: `deny_entries` returns it, so a notation change must reach
/// the running list.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DenyList {
    entries: Vec<DenyEntry>,
}

impl DenyList {
    pub fn new(entries: Vec<DenyEntry>) -> Self {
        Self { entries }
    }

    pub fn entries(&self) -> &[DenyEntry] {
        &self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// True when any item contains the client address (OPS-R55). `PeerIp` has
    /// already turned an IPv4-mapped IPv6 address into IPv4.
    pub fn denies(&self, client: PeerIp) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.item.contains(client.ip()))
    }

    /// Items that contain the client address, as written and in list order.
    pub fn covering(&self, client: PeerIp) -> impl Iterator<Item = &str> {
        self.entries
            .iter()
            .filter(move |entry| entry.item.contains(client.ip()))
            .map(DenyEntry::text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(texts: &[&str]) -> DenyList {
        DenyList::new(
            texts
                .iter()
                .map(|text| {
                    let item =
                        IpItem::parse(text).unwrap_or_else(|error| panic!("{text}: {error:?}"));
                    DenyEntry::new((*text).to_owned(), item)
                })
                .collect(),
        )
    }

    fn peer(text: &str) -> PeerIp {
        PeerIp::new(text.parse().unwrap_or_else(|e| panic!("parse {text}: {e}")))
    }

    /// spec: operations.md:OPS-100 partial
    #[test]
    fn an_empty_list_denies_nothing() {
        let deny = DenyList::default();
        assert!(!deny.denies(peer("192.0.2.1")));
        assert!(!deny.denies(peer("::1")));
        assert_eq!(deny.covering(peer("192.0.2.1")).count(), 0);
    }

    /// spec: management.md:MGT-065 partial
    #[test]
    fn lists_every_covering_item_as_written_in_list_order() {
        let deny = list(&[
            "203.0.113.0/24",
            "198.51.100.0/24",
            "203.0.113.9",
            "2001:DB8::1-2001:db8::ff",
        ]);
        let covering = |text| deny.covering(peer(text)).collect::<Vec<_>>();
        assert_eq!(covering("203.0.113.9"), ["203.0.113.0/24", "203.0.113.9"]);
        assert_eq!(covering("198.51.100.7"), ["198.51.100.0/24"]);
        assert_eq!(covering("2001:db8::5"), ["2001:DB8::1-2001:db8::ff"]);
        assert!(covering("192.0.2.50").is_empty());
        assert!(deny.denies(peer("::ffff:203.0.113.9")));
        assert!(!deny.denies(peer("192.0.2.50")));
    }

    #[test]
    fn equality_compares_items_and_their_written_text() {
        assert_eq!(list(&["192.0.2.1"]), list(&["192.0.2.1"]));
        assert_ne!(list(&["2001:DB8::1"]), list(&["2001:db8::1"]));
        assert_ne!(list(&["192.0.2.1"]), list(&["192.0.2.1", "192.0.2.2"]));
    }
}
