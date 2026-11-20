//! Values that IP information databases give and the local-address rule
//! (config-v4 §1.12, management-api-v1 §1.17).
//! Design: docs/architectures/source-stats.md.

use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::num::NonZeroU32;

/// ISO 3166-1 alpha-2 country code: exactly two ASCII uppercase letters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CountryCode([u8; 2]);

impl CountryCode {
    /// `None` unless `text` is two ASCII uppercase letters. Other values,
    /// such as `""` or `"eu"`, mean "no country" (config-v4 §1.12).
    pub fn parse(text: &str) -> Option<Self> {
        match text.as_bytes() {
            [first, second] if first.is_ascii_uppercase() && second.is_ascii_uppercase() => {
                Some(Self([*first, *second]))
            }
            _ => None,
        }
    }

    pub fn as_str(&self) -> &str {
        // `parse` admits ASCII letters only, so the bytes are UTF-8.
        std::str::from_utf8(&self.0).unwrap_or_default()
    }
}

impl fmt::Display for CountryCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Autonomous system number, 1 to 4294967295 (management-api-v1 §1.17).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Asn(NonZeroU32);

impl Asn {
    /// `None` for 0, which is not a usable number.
    pub fn new(number: u32) -> Option<Self> {
        NonZeroU32::new(number).map(Self)
    }

    /// The IPinfo Lite form: `AS` and decimal digits, such as `AS64496`.
    pub fn parse_prefixed(text: &str) -> Option<Self> {
        let digits = text.strip_prefix("AS")?;
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        digits.parse::<u32>().ok().and_then(Self::new)
    }

    pub fn get(self) -> u32 {
        self.0.get()
    }
}

/// Who made a database, for the attribution the dashboard shows
/// (config-v4 §1.12 "제공자"). It never selects the record fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Provider {
    Dbip,
    Ipinfo,
    MaxmindGeolite,
    Maxmind,
    Unknown,
}

impl Provider {
    /// Rules of config-v4 §1.12 in their order: `DBIP-` prefix, `ipinfo`
    /// anywhere in any case, `GeoLite` prefix, `GeoIP` prefix.
    pub fn from_database_type(database_type: &str) -> Self {
        if database_type.starts_with("DBIP-") {
            Self::Dbip
        } else if database_type.to_ascii_lowercase().contains("ipinfo") {
            Self::Ipinfo
        } else if database_type.starts_with("GeoLite") {
            Self::MaxmindGeolite
        } else if database_type.starts_with("GeoIP") {
            Self::Maxmind
        } else {
            Self::Unknown
        }
    }

    /// The `provider` value of management-api-v1 §1.17.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dbip => "dbip",
            Self::Ipinfo => "ipinfo",
            Self::MaxmindGeolite => "maxmind_geolite",
            Self::Maxmind => "maxmind",
            Self::Unknown => "unknown",
        }
    }
}

/// True for addresses that no database is asked about
/// (management-api-v1 §1.17 `local`): loopback, private (10.0.0.0/8,
/// 172.16.0.0/12, 192.168.0.0/16, fc00::/7), link-local (169.254.0.0/16,
/// fe80::/10), shared (100.64.0.0/10), and unspecified. An IPv4-mapped IPv6
/// address follows its IPv4 address.
pub fn is_local(ip: IpAddr) -> bool {
    match ip.to_canonical() {
        IpAddr::V4(ip) => is_local_v4(ip),
        IpAddr::V6(ip) => is_local_v6(ip),
    }
}

fn is_local_v4(ip: Ipv4Addr) -> bool {
    let [first, second, ..] = ip.octets();
    let shared = first == 100 && second & 0b1100_0000 == 64;
    ip.is_loopback() || ip.is_private() || ip.is_link_local() || shared || ip.is_unspecified()
}

fn is_local_v6(ip: Ipv6Addr) -> bool {
    ip.is_loopback() || ip.is_unique_local() || ip.is_unicast_link_local() || ip.is_unspecified()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(text: &str) -> IpAddr {
        text.parse()
            .unwrap_or_else(|error| panic!("parse {text}: {error}"))
    }

    /// spec: management.md:MGT-063 partial
    #[test]
    fn country_codes_are_two_uppercase_ascii_letters() {
        assert_eq!(
            CountryCode::parse("KR").map(|code| code.to_string()),
            Some("KR".to_owned())
        );
        for text in ["", "K", "kr", "Kr", "KOR", "K1", "ÄB", "--"] {
            assert_eq!(CountryCode::parse(text), None, "{text:?}");
        }
    }

    /// spec: management.md:MGT-063 partial
    #[test]
    fn as_numbers_run_from_one_to_the_u32_maximum() {
        assert_eq!(Asn::new(0), None);
        assert_eq!(Asn::new(64496).map(Asn::get), Some(64496));
        assert_eq!(Asn::new(u32::MAX).map(Asn::get), Some(u32::MAX));
        assert_eq!(Asn::parse_prefixed("AS64496").map(Asn::get), Some(64496));
        assert_eq!(
            Asn::parse_prefixed("AS4294967295").map(Asn::get),
            Some(u32::MAX)
        );
        for text in [
            "",
            "AS",
            "AS0",
            "as64496",
            "64496",
            "AS-1",
            "AS 1",
            "AS4294967296",
            "AS1x",
        ] {
            assert_eq!(Asn::parse_prefixed(text), None, "{text:?}");
        }
    }

    /// spec: operations.md:OPS-096 partial
    #[test]
    fn providers_follow_the_database_type_rules_in_order() {
        let cases = [
            ("DBIP-Country-Lite", Provider::Dbip),
            ("DBIP-ASN-Lite (compat=GeoLite2-ASN)", Provider::Dbip),
            ("ipinfo lite.mmdb", Provider::Ipinfo),
            ("IPinfo Lite", Provider::Ipinfo),
            ("GeoLite2-Country", Provider::MaxmindGeolite),
            ("GeoLite2-ASN", Provider::MaxmindGeolite),
            ("GeoIP2-Country", Provider::Maxmind),
            ("GeoIP2-City", Provider::Maxmind),
            ("geolite2-country", Provider::Unknown),
            ("dbip-country", Provider::Unknown),
            ("", Provider::Unknown),
        ];
        for (database_type, provider) in cases {
            assert_eq!(
                Provider::from_database_type(database_type),
                provider,
                "{database_type}"
            );
        }
        assert_eq!(Provider::MaxmindGeolite.as_str(), "maxmind_geolite");
    }

    /// spec: management.md:MGT-063 partial
    #[test]
    fn local_addresses_cover_the_contract_ranges_and_stop_at_their_edges() {
        let local = [
            "127.0.0.1",
            "10.0.0.0",
            "10.255.255.255",
            "172.16.0.1",
            "172.31.255.255",
            "192.168.0.15",
            "169.254.1.1",
            "100.64.0.0",
            "100.127.255.255",
            "0.0.0.0",
            "::1",
            "::",
            "fc00::1",
            "fdff:ffff::1",
            "fe80::1",
            "febf::1",
            "::ffff:192.168.0.1",
        ];
        for text in local {
            assert!(is_local(ip(text)), "{text}");
        }
        let public = [
            "9.255.255.255",
            "11.0.0.0",
            "172.15.255.255",
            "172.32.0.0",
            "192.167.255.255",
            "169.253.255.255",
            "100.63.255.255",
            "100.128.0.0",
            "192.0.2.1",
            "203.0.113.9",
            "2001:db8::5",
            "fec0::1",
            "fbff::1",
            "::ffff:203.0.113.9",
        ];
        for text in public {
            assert!(!is_local(ip(text)), "{text}");
        }
    }
}
