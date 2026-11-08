//! Transport-independent initial requests (repeater-handshake-v1 §1–3).

use std::fmt;
use std::net::{IpAddr, Ipv6Addr};

use crate::allow_list::parse_address;
use crate::entry_point::{EntryPoint, Role};
use crate::origin::is_dns_name;
use crate::relay_id::RelayId;
use crate::target_policy::parse_port;

/// Exact initial request length (repeater-handshake-v1 §1.2).
pub const HANDSHAKE_REQUEST_LEN: usize = 250;

/// Bytes sent once to viewers, never servers (repeater-handshake-v1 §1.1 D).
pub const VIEWER_GREETING: &[u8; 12] = b"RFB 000.000\n";

/// Relay mode selected by the initial request (repeater-handshake-v1 §1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Mode1,
    Mode2,
}

impl Mode {
    /// Mode name used by operation events (ops-info-v1 §1.4).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mode1 => "mode1",
            Self::Mode2 => "mode2",
        }
    }
}

/// Enabled relay modes, checked before payload syntax (repeater-handshake-v1 §1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandshakePolicy {
    pub mode1_enabled: bool,
    pub mode2_enabled: bool,
}

/// One host and port, without DNS resolution or policy checks.
/// See repeater-handshake-v1 §1.3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetAddress {
    pub host: TargetHost,
    pub port: u16,
}

/// A single canonical IP address or normalized DNS name.
/// Unlike target policy items, this cannot contain a range or CIDR block.
/// See repeater-handshake-v1 §1.3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetHost {
    Ip(IpAddr),
    /// Lower-case ASCII name without a trailing dot.
    Name(String),
}

impl TargetAddress {
    /// Reuses config address, DNS, and port rules, with the narrower request
    /// grammar and RFC 4291 IPv6 support (repeater-handshake-v1 §1.3).
    fn parse(text: &str) -> Result<Self, HandshakeError> {
        let (host, port, bracketed) = if let Some(rest) = text.strip_prefix('[') {
            let (host, suffix) = rest.split_once(']').ok_or(HandshakeError::Invalid)?;
            let port = if suffix.is_empty() {
                None
            } else {
                Some(suffix.strip_prefix(':').ok_or(HandshakeError::Invalid)?)
            };
            (host, port, true)
        } else {
            match text.split_once(':') {
                Some((host, port)) => (host, Some(port), false),
                None => (text, None, false),
            }
        };
        let port = match port {
            Some(text) => {
                if text.starts_with('0') {
                    return Err(HandshakeError::Invalid);
                }
                parse_port(text).map_err(|_| HandshakeError::Invalid)?
            }
            None => 5900,
        };
        let host = if bracketed {
            let ip = host
                .parse::<Ipv6Addr>()
                .map_err(|_| HandshakeError::Invalid)?;
            TargetHost::Ip(IpAddr::V6(ip).to_canonical())
        } else if host
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.' || byte == b'-')
        {
            // Keep numeric address ranges out of the DNS branch, as target
            // policy parsing does. Numeric-and-dot input never falls back to DNS.
            TargetHost::Ip(parse_address(host).map_err(|_| HandshakeError::Invalid)?)
        } else {
            let name = host.strip_suffix('.').unwrap_or(host);
            if !is_dns_name(name) {
                return Err(HandshakeError::Invalid);
            }
            TargetHost::Name(name.to_ascii_lowercase())
        };
        Ok(Self { host, port })
    }
}

/// A validated request; its role comes from the entry point, not the payload.
/// Debug output hides request values (repeater-handshake-v1 §1.2 and §1.4).
#[derive(Clone, PartialEq, Eq)]
pub enum HandshakeRequest {
    Mode1(TargetAddress),
    Mode2(RelayId),
}

impl fmt::Debug for HandshakeRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Mode1(_) => "Mode1(<redacted>)",
            Self::Mode2(_) => "Mode2(<redacted>)",
        })
    }
}

/// Format and mode failures without I/O or timing state.
/// `Incomplete` alone does not establish peer EOF (repeater-handshake-v1 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeError {
    Incomplete,
    Invalid,
    ModeDisabled(Mode),
}

impl HandshakeError {
    /// Contract error code without any request content (repeater-handshake-v1 §2).
    pub fn code(self) -> &'static str {
        match self {
            Self::Incomplete => "handshake_incomplete",
            Self::Invalid => "handshake_invalid",
            Self::ModeDisabled(_) => "mode_disabled",
        }
    }
}

impl fmt::Display for HandshakeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Incomplete => "initial request is incomplete",
            Self::Invalid => "initial request is invalid",
            Self::ModeDisabled(_) => "requested relay mode is disabled",
        })
    }
}

impl std::error::Error for HandshakeError {}

/// Accumulates exactly one request without per-feed allocation or early parsing.
/// Debug output never exposes bytes (repeater-handshake-v1 §1.2 and §1.4).
pub struct HandshakeParser {
    buffer: [u8; HANDSHAKE_REQUEST_LEN],
    filled: usize,
}

impl HandshakeParser {
    /// Creates an empty fixed-size request buffer (repeater-handshake-v1 §1.2).
    pub fn new() -> Self {
        Self {
            buffer: [0; HANDSHAKE_REQUEST_LEN],
            filled: 0,
        }
    }

    /// Consumes only the bytes still needed for the 250-byte request.
    /// The caller retains `input[consumed..]` as subsequent stream data.
    /// Returns zero after completion (repeater-handshake-v1 §1.1 E and §1.2).
    pub fn feed(&mut self, input: &[u8]) -> usize {
        let consumed = input.len().min(HANDSHAKE_REQUEST_LEN - self.filled);
        self.buffer[self.filled..self.filled + consumed].copy_from_slice(&input[..consumed]);
        self.filled += consumed;
        consumed
    }

    /// Reports whether all 250 bytes arrived, even if NUL arrived earlier.
    /// See repeater-handshake-v1 §1.2.
    pub fn is_complete(&self) -> bool {
        self.filled == HANDSHAKE_REQUEST_LEN
    }

    /// Parses only a complete request, ignoring everything after its first NUL.
    /// Server prefix validation precedes mode checks; disabled modes skip body
    /// validation, including UTF-8 decoding (repeater-handshake-v1 §1.2–1.3).
    pub fn finish(
        self,
        entry_point: EntryPoint,
        modes: HandshakePolicy,
    ) -> Result<HandshakeRequest, HandshakeError> {
        if !self.is_complete() {
            return Err(HandshakeError::Incomplete);
        }
        let end = self
            .buffer
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(HANDSHAKE_REQUEST_LEN);
        let value = &self.buffer[..end];
        let id = value.strip_prefix(b"ID:");
        if entry_point.role() == Role::Server && id.is_none() {
            return Err(HandshakeError::Invalid);
        }
        match id {
            Some(id) => {
                if !modes.mode2_enabled {
                    return Err(HandshakeError::ModeDisabled(Mode::Mode2));
                }
                let text = std::str::from_utf8(id).map_err(|_| HandshakeError::Invalid)?;
                RelayId::parse(text)
                    .map(HandshakeRequest::Mode2)
                    .map_err(|_| HandshakeError::Invalid)
            }
            None => {
                if !modes.mode1_enabled {
                    return Err(HandshakeError::ModeDisabled(Mode::Mode1));
                }
                let text = std::str::from_utf8(value).map_err(|_| HandshakeError::Invalid)?;
                TargetAddress::parse(text).map(HandshakeRequest::Mode1)
            }
        }
    }
}

impl Default for HandshakeParser {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for HandshakeParser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HandshakeParser")
            .field("filled", &self.filled)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    const ENABLED: HandshakePolicy = HandshakePolicy {
        mode1_enabled: true,
        mode2_enabled: true,
    };

    fn request(value: &[u8]) -> [u8; HANDSHAKE_REQUEST_LEN] {
        assert!(value.len() <= HANDSHAKE_REQUEST_LEN);
        let mut bytes = [0; HANDSHAKE_REQUEST_LEN];
        bytes[..value.len()].copy_from_slice(value);
        bytes
    }

    fn parse(
        value: &[u8],
        entry_point: EntryPoint,
        modes: HandshakePolicy,
    ) -> Result<HandshakeRequest, HandshakeError> {
        let mut parser = HandshakeParser::new();
        assert_eq!(parser.feed(&request(value)), HANDSHAKE_REQUEST_LEN);
        parser.finish(entry_point, modes)
    }

    fn error_code(result: Result<HandshakeRequest, HandshakeError>) -> &'static str {
        match result {
            Ok(request) => panic!("unexpected request: {request:?}"),
            Err(error) => error.code(),
        }
    }

    /// spec: relay.md:RLY-002 partial
    #[test]
    fn parses_the_same_request_at_every_split_boundary() {
        for (entry_point, value) in [
            (EntryPoint::TcpViewer, "192.0.2.30:5901"),
            (EntryPoint::WsViewer, "PC1.Example."),
            (EntryPoint::WssViewer, "[2001:db8::30]:5900"),
            (EntryPoint::TcpViewer, "ID:0123"),
            (EntryPoint::TcpServer, "ID:0123"),
            (EntryPoint::WsServer, "ID:0123"),
            (EntryPoint::WssServer, "ID:0123"),
        ] {
            let bytes = request(value.as_bytes());
            let expected = parse(value.as_bytes(), entry_point, ENABLED);
            assert!(expected.is_ok());
            for split in 0..=HANDSHAKE_REQUEST_LEN {
                let mut parser = HandshakeParser::new();
                assert_eq!(parser.feed(&bytes[..split]), split);
                assert_eq!(parser.is_complete(), split == HANDSHAKE_REQUEST_LEN);
                assert_eq!(parser.feed(&bytes[split..]), HANDSHAKE_REQUEST_LEN - split);
                assert!(parser.is_complete());
                assert_eq!(parser.finish(entry_point, ENABLED), expected, "{split}");
            }
            let mut parser = HandshakeParser::new();
            for (index, byte) in bytes.iter().enumerate() {
                assert_eq!(parser.feed(std::slice::from_ref(byte)), 1);
                assert_eq!(parser.is_complete(), index + 1 == HANDSHAKE_REQUEST_LEN);
            }
            assert_eq!(parser.finish(entry_point, ENABLED), expected);
        }
    }

    /// spec: relay.md:RLY-004 partial
    #[test]
    fn rejects_empty_values_invalid_ids_and_invalid_targets() {
        for entry_point in EntryPoint::ALL {
            for value in ["", "ID:", "ID:abc", "ID:12 34", "ID:+12", "ID:１２"] {
                assert_eq!(
                    error_code(parse(value.as_bytes(), entry_point, ENABLED)),
                    "handshake_invalid",
                    "{entry_point:?}: {value}"
                );
            }
            for value in [b"ID:\xff".as_slice(), b"\xff"] {
                assert_eq!(
                    error_code(parse(value, entry_point, ENABLED)),
                    "handshake_invalid"
                );
            }
        }
        for value in ["192.0.2.30", "XX:123", "123", "id:123"] {
            assert_eq!(
                error_code(parse(value.as_bytes(), EntryPoint::TcpServer, ENABLED)),
                "handshake_invalid"
            );
        }
        for value in [
            " ",
            "PC1.Example ",
            " pc1.example",
            "pc1.example: 5900",
            "pc1.example:0",
            "pc1.example:65536",
            "pc1.example:05900",
            "pc1.example:01",
            "pc1.example:",
            "pc1.example:+1",
            "pc1.example:-1",
            "pc1.example:1-2",
            "pc1.example:*",
            "192.00.2.30",
            "999.0.2.30",
            "1234",
            "192.0.2.30.",
            "192.0.2.0/24",
            "192.0.2.1-192.0.2.9",
            "1-2",
            "1-2:5900",
            "*.example",
            "a..example",
            "-pc.example",
            "pc-.example",
            "pc_1.example",
            "é.example",
            "[192.0.2.30]",
            "2001:db8::30",
            "2001:db8::30:5900",
            "[2001:db8::30",
            "[2001:db8::30]suffix",
            "[2001:db8::30]:",
            "[2001:db8::30]:05900",
            "[2001:db8::30]:65536",
            "[2001:db8::30]:5900:5901",
            "[fe80::1%3]",
            "[2001:db8::/32]",
            "[2001:db8::1-2001:db8::9]",
        ] {
            assert_eq!(
                error_code(parse(value.as_bytes(), EntryPoint::TcpViewer, ENABLED)),
                "handshake_invalid",
                "{value}"
            );
        }
        let long_label = format!("{}.example", "a".repeat(64));
        assert_eq!(
            error_code(parse(long_label.as_bytes(), EntryPoint::TcpViewer, ENABLED)),
            "handshake_invalid"
        );
    }

    /// spec: relay.md:RLY-022 partial
    #[test]
    fn rejects_disabled_modes_before_validating_their_payloads() {
        for mode1_enabled in [false, true] {
            for mode2_enabled in [false, true] {
                let modes = HandshakePolicy {
                    mode1_enabled,
                    mode2_enabled,
                };
                for entry_point in EntryPoint::ALL {
                    for value in [b"ID:0123".as_slice(), b"ID:", b"ID:abc", b"ID:\xff"] {
                        let result = parse(value, entry_point, modes);
                        if !mode2_enabled {
                            assert_eq!(result, Err(HandshakeError::ModeDisabled(Mode::Mode2)));
                            assert_eq!(error_code(result), "mode_disabled");
                        } else if value == b"ID:0123" {
                            assert!(matches!(result, Ok(HandshakeRequest::Mode2(_))));
                        } else {
                            assert_eq!(error_code(result), "handshake_invalid");
                        }
                    }
                    for value in [b"192.0.2.30".as_slice(), b"", b"bad target", b"\xff"] {
                        let result = parse(value, entry_point, modes);
                        if entry_point.role() == Role::Server {
                            assert_eq!(error_code(result), "handshake_invalid");
                        } else if !mode1_enabled {
                            assert_eq!(result, Err(HandshakeError::ModeDisabled(Mode::Mode1)));
                            assert_eq!(error_code(result), "mode_disabled");
                        } else if value == b"192.0.2.30" {
                            assert!(matches!(result, Ok(HandshakeRequest::Mode1(_))));
                        } else {
                            assert_eq!(error_code(result), "handshake_invalid");
                        }
                    }
                }
            }
        }
    }

    /// spec: relay.md:RLY-028 partial
    #[test]
    fn every_server_transport_accepts_mode_two_and_rejects_mode_one() {
        for entry in [
            EntryPoint::TcpServer,
            EntryPoint::WsServer,
            EntryPoint::WssServer,
        ] {
            assert!(matches!(
                parse(b"ID:0123", entry, ENABLED),
                Ok(HandshakeRequest::Mode2(_))
            ));
            for target in [b"192.0.2.30:5900".as_slice(), b"pc.example:5900"] {
                assert_eq!(
                    error_code(parse(target, entry, ENABLED)),
                    "handshake_invalid"
                );
            }
        }
    }

    #[test]
    fn ignores_padding_after_the_first_nul_and_accepts_no_nul_requests() {
        let expected = parse(b"ID:0123", EntryPoint::TcpServer, ENABLED);
        let mut bytes = [0xff; HANDSHAKE_REQUEST_LEN];
        bytes[..8].copy_from_slice(b"ID:0123\0");
        let mut parser = HandshakeParser::new();
        parser.feed(&bytes);
        assert_eq!(parser.finish(EntryPoint::TcpServer, ENABLED), expected);

        let id = "9".repeat(RelayId::MAX_LEN);
        let value = format!("ID:{id}");
        assert_eq!(value.len(), HANDSHAKE_REQUEST_LEN);
        assert_eq!(
            parse(value.as_bytes(), EntryPoint::TcpServer, ENABLED),
            Ok(HandshakeRequest::Mode2(
                RelayId::parse(&id).unwrap_or_else(|error| panic!("{error}"))
            ))
        );
        let name = [
            "a".repeat(63),
            "b".repeat(63),
            "c".repeat(63),
            "d".repeat(58),
        ]
        .join(".");
        assert_eq!(name.len(), HANDSHAKE_REQUEST_LEN);
        assert_eq!(
            parse(name.as_bytes(), EntryPoint::TcpViewer, ENABLED),
            Ok(HandshakeRequest::Mode1(TargetAddress {
                host: TargetHost::Name(name),
                port: 5900,
            }))
        );
    }

    #[test]
    fn parses_targets_with_the_existing_address_and_name_rules() {
        for (value, host, port) in [
            (
                "192.0.2.30",
                TargetHost::Ip(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 30))),
                5900,
            ),
            ("PC1.Example.:1", TargetHost::Name("pc1.example".into()), 1),
            (
                "pc-1.example:65535",
                TargetHost::Name("pc-1.example".into()),
                65535,
            ),
            ("localhost", TargetHost::Name("localhost".into()), 5900),
            (
                "[2001:0DB8:0000:0000:0000:0000:0000:0030]",
                TargetHost::Ip(IpAddr::V6(Ipv6Addr::new(
                    0x2001, 0xdb8, 0, 0, 0, 0, 0, 0x30,
                ))),
                5900,
            ),
            (
                "[::ffff:192.0.2.30]:5901",
                TargetHost::Ip(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 30))),
                5901,
            ),
            (
                "[::FFFF:c000:021e]:65535",
                TargetHost::Ip(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 30))),
                65535,
            ),
            (
                "[2001:db8::192.0.2.30]:1",
                TargetHost::Ip(IpAddr::V6(Ipv6Addr::new(
                    0x2001, 0xdb8, 0, 0, 0, 0, 0xc000, 0x021e,
                ))),
                1,
            ),
        ] {
            for entry_point in [
                EntryPoint::TcpViewer,
                EntryPoint::WsViewer,
                EntryPoint::WssViewer,
            ] {
                assert_eq!(
                    parse(value.as_bytes(), entry_point, ENABLED),
                    Ok(HandshakeRequest::Mode1(TargetAddress {
                        host: host.clone(),
                        port,
                    })),
                    "{value}"
                );
            }
        }
    }

    #[test]
    fn refuses_to_interpret_any_incomplete_request() {
        let bytes = request(b"ID:0123");
        for filled in 0..HANDSHAKE_REQUEST_LEN {
            for entry_point in EntryPoint::ALL {
                let mut parser = HandshakeParser::new();
                assert_eq!(parser.feed(&bytes[..filled]), filled);
                assert!(!parser.is_complete());
                assert_eq!(
                    error_code(parser.finish(
                        entry_point,
                        HandshakePolicy {
                            mode1_enabled: false,
                            mode2_enabled: false,
                        }
                    )),
                    "handshake_incomplete"
                );
            }
        }
    }

    #[test]
    fn leaves_trailing_bytes_with_the_caller_and_consumes_nothing_after_completion() {
        let mut input = request(b"ID:0123").to_vec();
        input.extend_from_slice(b"V>a\0\xff");
        for filled in [0, 1, 249, 250] {
            let mut parser = HandshakeParser::new();
            assert_eq!(parser.feed(&input[..filled]), filled);
            let consumed = parser.feed(&input[filled..]);
            assert_eq!(consumed, HANDSHAKE_REQUEST_LEN - filled);
            assert_eq!(&input[filled + consumed..], b"V>a\0\xff");
            assert_eq!(parser.feed(b"more data"), 0);
            assert_eq!(parser.feed(&[]), 0);
            assert_eq!(
                parser.finish(EntryPoint::TcpViewer, ENABLED),
                parse(b"ID:0123", EntryPoint::TcpViewer, ENABLED)
            );
        }
    }

    #[test]
    fn hides_request_values_in_debug_and_exposes_stable_mode_names() {
        assert_eq!(Mode::Mode1.as_str(), "mode1");
        assert_eq!(Mode::Mode2.as_str(), "mode2");
        let mut parser = HandshakeParser::default();
        parser.feed(&request(b"ID:0123456789"));
        assert!(!format!("{parser:?}").contains("0123456789"));
        let request = parser
            .finish(EntryPoint::TcpViewer, ENABLED)
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(format!("{request:?}"), "Mode2(<redacted>)");
        let request = parse(b"192.0.2.30", EntryPoint::TcpViewer, ENABLED)
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(format!("{request:?}"), "Mode1(<redacted>)");
        assert_eq!(VIEWER_GREETING, b"RFB 000.000\n");
    }
}
