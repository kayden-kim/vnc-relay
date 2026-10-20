//! Origin values and their comparison (config-v1 §1.4).

use std::fmt;
use std::net::{Ipv4Addr, Ipv6Addr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scheme {
    Http,
    Https,
}

impl Scheme {
    fn default_port(self) -> u16 {
        match self {
            Self::Http => 80,
            Self::Https => 443,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Https => "https",
        }
    }
}

/// A normalized origin. Two origins match when they are equal: same scheme,
/// same lower-case host, and same effective port (80 or 443 when omitted).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Origin {
    scheme: Scheme,
    /// Lower-case host. An IPv6 host keeps its brackets and its written form.
    host: String,
    port: u16,
}

/// The text is not `scheme://host[:port]` as config-v1 §1.4 defines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OriginError;

impl fmt::Display for OriginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("origin is not scheme://host[:port]")
    }
}

impl std::error::Error for OriginError {}

impl Origin {
    /// Parses and normalizes an origin. The same function reads config values
    /// and request `Origin` headers, so both sides normalize the same way.
    pub fn parse(text: &str) -> Result<Self, OriginError> {
        if !text.is_ascii() {
            return Err(OriginError);
        }
        let (scheme, rest) = text.split_once("://").ok_or(OriginError)?;
        let scheme = if scheme.eq_ignore_ascii_case("http") {
            Scheme::Http
        } else if scheme.eq_ignore_ascii_case("https") {
            Scheme::Https
        } else {
            return Err(OriginError);
        };
        if rest.is_empty()
            || rest
                .bytes()
                .any(|b| matches!(b, b'/' | b'?' | b'#' | b'@') || b.is_ascii_whitespace())
        {
            return Err(OriginError);
        }
        let (host, port) = split_host_port(rest)?;
        let host = host.to_ascii_lowercase();
        if !is_valid_host(&host) {
            return Err(OriginError);
        }
        let port = match port {
            None => scheme.default_port(),
            Some(port) => parse_port(port).ok_or(OriginError)?,
        };
        Ok(Self { scheme, host, port })
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}", self.scheme.as_str(), self.host)?;
        if self.port != self.scheme.default_port() {
            write!(f, ":{}", self.port)?;
        }
        Ok(())
    }
}

fn split_host_port(text: &str) -> Result<(&str, Option<&str>), OriginError> {
    if text.starts_with('[') {
        let end = text.find(']').ok_or(OriginError)?;
        let (host, rest) = text.split_at(end + 1);
        match rest {
            "" => Ok((host, None)),
            _ => rest
                .strip_prefix(':')
                .map(|port| (host, Some(port)))
                .ok_or(OriginError),
        }
    } else {
        match text.split_once(':') {
            None => Ok((text, None)),
            Some((host, port)) => Ok((host, Some(port))),
        }
    }
}

fn parse_port(text: &str) -> Option<u16> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse::<u16>().ok().filter(|port| *port != 0)
}

/// ASCII domain name, IPv4 address, or bracketed IPv6 address.
fn is_valid_host(host: &str) -> bool {
    if let Some(inner) = host.strip_prefix('[') {
        return inner
            .strip_suffix(']')
            .is_some_and(|address| address.parse::<Ipv6Addr>().is_ok());
    }
    if host.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
        return host.parse::<Ipv4Addr>().is_ok();
    }
    is_dns_name(host)
}

/// Dot-separated labels of ASCII letters, digits, and hyphens. Each label is
/// 1 to 63 characters and does not start or end with a hyphen. The whole
/// name is at most 253 characters. Shared with Mode I target names.
pub(crate) fn is_dns_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 253
        && name.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origin(text: &str) -> Origin {
        match Origin::parse(text) {
            Ok(origin) => origin,
            Err(_) => panic!("{text} did not parse"),
        }
    }

    #[test]
    fn accepts_allowed_forms_and_normalizes_case_and_default_port() {
        assert_eq!(
            origin("https://viewer.example").to_string(),
            "https://viewer.example"
        );
        assert_eq!(
            origin("HTTPS://Viewer.Example").to_string(),
            "https://viewer.example"
        );
        assert_eq!(
            origin("https://viewer.example:8443").to_string(),
            "https://viewer.example:8443"
        );
        assert_eq!(
            origin("http://192.0.2.10:8080").to_string(),
            "http://192.0.2.10:8080"
        );
        assert_eq!(
            origin("http://[2001:DB8::1]").to_string(),
            "http://[2001:db8::1]"
        );
        assert_eq!(
            origin("https://xn--3e0b707e.example").to_string(),
            "https://xn--3e0b707e.example"
        );
    }

    #[test]
    fn rejects_text_outside_the_grammar() {
        for text in [
            "null",
            "viewer.example",
            "ftp://viewer.example",
            "https://",
            "https://viewer.example/",
            "https://viewer.example/path",
            "https://viewer.example?q=1",
            "https://viewer.example#frag",
            "https://user@viewer.example",
            "https://*.example",
            "https://viewer.example:0",
            "https://viewer.example:65536",
            "https://viewer.example:",
            "https://viewer.example:+80",
            "https://viewer .example",
            "https://한국.example",
            "https://2001:db8::1",
            "https://[2001:db8::1",
            "https://[viewer.example]",
            "https://192.0.2",
            "https://-viewer.example",
            "https://viewer..example",
        ] {
            assert_eq!(Origin::parse(text), Err(OriginError), "{text:?}");
        }
    }

    #[test]
    fn compares_scheme_host_and_effective_port_exactly() {
        assert_eq!(
            origin("https://viewer.example"),
            origin("https://viewer.example:443")
        );
        assert_eq!(
            origin("http://viewer.example"),
            origin("HTTP://VIEWER.EXAMPLE:80")
        );
        assert_ne!(
            origin("http://viewer.example"),
            origin("https://viewer.example")
        );
        assert_ne!(
            origin("https://viewer.example"),
            origin("https://a.viewer.example")
        );
        assert_ne!(
            origin("https://viewer.example:8443"),
            origin("https://viewer.example")
        );
        assert_ne!(
            origin("http://[2001:db8::1]"),
            origin("http://[2001:db8:0::1]")
        );
    }
}
