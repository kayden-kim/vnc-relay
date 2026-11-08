//! Trusted proxy client address selection (websocket-relay-v1 §1.8).

use std::fmt;
use std::net::{IpAddr, Ipv6Addr};

use crate::allow_list::AllowList;
use crate::peer_ip::PeerIp;

/// The inspected chain is missing, malformed, or contains only trusted proxies
/// (websocket-relay-v1 §1.8). No client address can be used in these cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForwardedForError;

impl ForwardedForError {
    /// The upgrade rejection reason of websocket-relay-v1 §2.1.
    pub fn code(self) -> &'static str {
        "forwarded_for_invalid"
    }
}

impl fmt::Display for ForwardedForError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for ForwardedForError {}

/// Selects the rightmost untrusted canonical IP from `X-Forwarded-For` fields
/// in received order (websocket-relay-v1 §1.8). The caller must first establish
/// that the socket peer is trusted. Elements left of the selected IP are ignored.
/// Missing fields, an inspected invalid element, and an all-trusted chain fail.
pub fn client_ip<'a, I>(fields: I, trusted: &AllowList) -> Result<PeerIp, ForwardedForError>
where
    I: IntoIterator<Item = &'a str>,
    I::IntoIter: DoubleEndedIterator,
{
    // Reversing both levels is equivalent to joining fields with commas, then
    // scanning right to left, without allocating or parsing the ignored prefix.
    for field in fields.into_iter().rev() {
        for element in field.split(',').rev() {
            let element = element.trim_matches([' ', '\t']);
            let ip = if let Some(bracketed) = element.strip_prefix('[') {
                bracketed
                    .strip_suffix(']')
                    .ok_or(ForwardedForError)?
                    .parse::<Ipv6Addr>()
                    .map(IpAddr::V6)
                    .map_err(|_| ForwardedForError)?
            } else {
                element.parse::<IpAddr>().map_err(|_| ForwardedForError)?
            };
            let ip = PeerIp::new(ip);
            if !trusted.permits(ip) {
                return Ok(ip);
            }
        }
    }
    Err(ForwardedForError)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::allow_list::IpItem;

    fn proxies(items: &[&str]) -> AllowList {
        AllowList::new(
            items
                .iter()
                .map(|text| IpItem::parse(text).expect("proxy item"))
                .collect(),
        )
    }

    fn peer(text: &str) -> PeerIp {
        PeerIp::new(text.parse().expect("IP literal"))
    }

    /// spec: websocket.md:WS-032 partial
    #[test]
    fn selects_rightmost_untrusted_ip_across_fields_and_ignores_left_prefix() {
        let trusted = proxies(&["192.0.2.0/24", "2001:db8:1::/48"]);
        let chains: &[&[&str]] = &[
            &[
                "unknown, 203.0.113.7",
                "\t198.51.100.8 \t, 192.0.2.9",
                "[2001:db8:1::2]",
            ],
            &["", "invalid,\t198.51.100.8\t, 192.0.2.9, [2001:db8:1::2]"],
            &["invalid,,198.51.100.8", "192.0.2.9, [2001:db8:1::2]"],
        ];
        for fields in chains {
            assert_eq!(
                client_ip(fields.iter().copied(), &trusted),
                Ok(peer("198.51.100.8"))
            );
        }
    }

    /// spec: websocket.md:WS-032 partial
    #[test]
    fn accepts_ipv4_and_bare_or_bracketed_ipv6_with_only_sp_htab_trimmed() {
        let trusted = AllowList::default();
        for (field, expected) in [
            (" \t198.51.100.8\t ", "198.51.100.8"),
            ("\t2001:DB8::8 ", "2001:db8::8"),
            (" [2001:DB8::8]\t", "2001:db8::8"),
            ("::ffff:198.51.100.8", "198.51.100.8"),
            ("[::FFFF:c633:6408]", "198.51.100.8"),
        ] {
            assert_eq!(client_ip([field], &trusted), Ok(peer(expected)));
        }
    }

    /// spec: websocket.md:WS-032 partial
    /// spec: websocket.md:WS-036 partial
    #[test]
    fn canonicalizes_mapped_ipv6_before_trust_comparison() {
        let trusted = proxies(&["192.0.2.0/24"]);
        assert_eq!(
            client_ip(
                ["198.51.100.8, [::ffff:192.0.2.9], ::FFFF:c000:020a"],
                &trusted
            ),
            Ok(peer("198.51.100.8"))
        );
        assert_eq!(
            client_ip(["[::ffff:192.0.2.9], ::FFFF:c000:020a"], &trusted),
            Err(ForwardedForError)
        );
    }

    /// spec: websocket.md:WS-033 partial
    #[test]
    fn rejects_every_inspected_invalid_literal() {
        let trusted = proxies(&["192.0.2.0/24"]);
        for invalid in [
            "",
            " \t ",
            "unknown",
            "_hidden",
            "example.test",
            "198.51.100",
            "198.51.100.256",
            "198.051.100.8",
            "198.51.100.8:5900",
            "[2001:db8::8]:5900",
            "[198.51.100.8]",
            "fe80::1%eth0",
            "[fe80::1%3]",
            "[2001:db8::8",
            "2001:db8::8]",
            "[2001:db8::8]]",
            "\r198.51.100.8",
            "198.51.100.8\n",
            "\u{a0}198.51.100.8",
            "198.51.100.8\u{a0}",
            "\u{b}198.51.100.8",
            "198.51.100.8\u{c}",
            "198.51. 100.8",
            "\"198.51.100.8\"",
        ] {
            assert_eq!(
                client_ip([invalid], &trusted),
                Err(ForwardedForError),
                "{invalid:?}"
            );
        }
    }

    /// spec: websocket.md:WS-033 partial
    #[test]
    fn rejects_invalid_elements_to_the_right_of_an_untrusted_ip() {
        let trusted = proxies(&["192.0.2.0/24"]);
        let chains: &[&[&str]] = &[
            &["198.51.100.8, unknown, 192.0.2.9"],
            &["198.51.100.8,", "192.0.2.9"],
            &["198.51.100.8", "", "192.0.2.9"],
            &["198.51.100.8", "192.0.2.9,"],
        ];
        for fields in chains {
            assert_eq!(
                client_ip(fields.iter().copied(), &trusted),
                Err(ForwardedForError)
            );
        }
    }

    /// spec: websocket.md:WS-033 partial
    /// spec: websocket.md:WS-036 partial
    #[test]
    fn rejects_missing_fields_and_all_trusted_chains_without_a_fallback() {
        let trusted = proxies(&["192.0.2.0/24", "2001:db8::/32"]);
        let chains: &[&[&str]] = &[
            &[],
            &["192.0.2.9"],
            &["192.0.2.9, [2001:db8::2]", "192.0.2.10"],
        ];
        for fields in chains {
            assert_eq!(
                client_ip(fields.iter().copied(), &trusted),
                Err(ForwardedForError)
            );
        }
        let trusted = proxies(&["0.0.0.0/0", "::/0"]);
        assert_eq!(
            client_ip(["198.51.100.8, 2001:db8::2"], &trusted),
            Err(ForwardedForError)
        );
        assert_eq!(ForwardedForError.code(), "forwarded_for_invalid");
    }
}
