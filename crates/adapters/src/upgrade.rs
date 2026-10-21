//! The WebSocket upgrade request of a WS or WSS viewer entry point
//! (websocket-relay-v1 §1.1): reading the request head, checking it in the
//! contract's order, and the answers (101 or a refusal). The checks and the
//! answer texts have no socket in them, so unit tests cover them directly.

use std::io;

use tokio::io::{AsyncRead, AsyncReadExt};
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;
use vnc_relay_app::events::RejectReason;
use vnc_relay_domain::origin::Origin;

/// Size of each read while the request head is collected.
const READ_CHUNK: usize = 1024;

/// Largest request head accepted, in bytes, blank line included. A head
/// that does not end within this size is refused with 400. The contract is
/// silent on a limit; this value is an implementation choice.
pub(crate) const HEAD_LIMIT: usize = 16 * 1024;

/// The only path of the relay endpoint (websocket-relay-v1 §1.1). A query
/// string after it is not part of the path.
const RELAY_PATH: &str = "/relay";

/// Result of reading one request head.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum HeadRead {
    /// The head up to and including its blank line. Bytes after it are dropped.
    Complete(Vec<u8>),
    /// The peer closed the connection before the head ended.
    Eof,
    /// The head did not end within `HEAD_LIMIT` bytes.
    TooLarge,
}

/// Reads a request head, stopping at the blank line that ends it.
pub(crate) async fn read_head<R>(io: &mut R) -> io::Result<HeadRead>
where
    R: AsyncRead + Unpin,
{
    let mut buf = Vec::new();
    loop {
        let mut chunk = [0_u8; READ_CHUNK];
        let read = io.read(&mut chunk).await?;
        if read == 0 {
            return Ok(HeadRead::Eof);
        }
        buf.extend_from_slice(&chunk[..read]);
        if let Some(end) = head_end(&buf) {
            if end > HEAD_LIMIT {
                return Ok(HeadRead::TooLarge);
            }
            buf.truncate(end);
            return Ok(HeadRead::Complete(buf));
        }
        if buf.len() >= HEAD_LIMIT {
            return Ok(HeadRead::TooLarge);
        }
    }
}

/// The length of the head in `buf`, blank line included, if it has ended.
fn head_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|at| at + 4)
}

/// Why an upgrade request is refused. Each refusal has one HTTP answer and
/// one reason in the `connection_rejected` line of stage `upgrade`
/// (ops-info-v1 §1.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// 400: a malformed request, a missing or wrong header, a duplicate
    /// Origin, an HTTP version below 1.1, or a head over the limit.
    Invalid,
    /// 404: the path is not `/relay`.
    NotFound,
    /// 405: the method is not GET.
    MethodNotAllowed,
    /// 426: `Sec-WebSocket-Version` is missing, repeated, or not 13.
    VersionUnsupported,
    /// 403: the Origin is not on the entry point's list. The raw header
    /// value is kept for the log.
    OriginNotAllowed(String),
    /// 400: the client offered subprotocols and none of them is `binary`.
    SubprotocolNotSupported,
    /// 403: the source IP failed the pre-upgrade recheck (§1.1).
    IpNotAllowed,
}

impl Refusal {
    /// The reason in the `connection_rejected` line.
    pub(crate) fn reason(&self) -> RejectReason {
        match self {
            Self::Invalid | Self::NotFound | Self::MethodNotAllowed | Self::VersionUnsupported => {
                RejectReason::InvalidUpgrade
            }
            Self::OriginNotAllowed(_) => RejectReason::OriginNotAllowed,
            Self::SubprotocolNotSupported => RejectReason::SubprotocolNotSupported,
            Self::IpNotAllowed => RejectReason::IpNotAllowed,
        }
    }

    /// The raw Origin value, when the refusal is about an Origin.
    pub(crate) fn origin(&self) -> Option<&str> {
        match self {
            Self::OriginNotAllowed(origin) => Some(origin.as_str()),
            _ => None,
        }
    }

    /// The HTTP answer: status line, the headers the status needs, and no body.
    pub(crate) fn answer(&self) -> Vec<u8> {
        let (status, extra) = match self {
            Self::Invalid | Self::SubprotocolNotSupported => ("400 Bad Request", ""),
            Self::NotFound => ("404 Not Found", ""),
            Self::MethodNotAllowed => ("405 Method Not Allowed", "Allow: GET\r\n"),
            Self::VersionUnsupported => ("426 Upgrade Required", "Sec-WebSocket-Version: 13\r\n"),
            Self::OriginNotAllowed(_) | Self::IpNotAllowed => ("403 Forbidden", ""),
        };
        format!("HTTP/1.1 {status}\r\n{extra}Content-Length: 0\r\nConnection: close\r\n\r\n")
            .into_bytes()
    }
}

/// A request that passed every check of §1.1 except the pre-upgrade IP recheck.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValidUpgrade {
    /// `Sec-WebSocket-Accept` for the request key.
    pub(crate) accept_key: String,
    /// The client offered `binary`, which the 101 echoes.
    pub(crate) binary: bool,
}

/// Checks a request head in the order of websocket-relay-v1 §1.1: request
/// syntax, then path, method, and HTTP version, then the upgrade headers and
/// `Sec-WebSocket-Version`, then `Sec-WebSocket-Key`, then Origin, then the
/// subprotocol. The first failed check decides the refusal. The IP recheck is
/// not part of this function; the caller runs it after a request passes.
pub(crate) fn check(head: &[u8], origins: &[Origin]) -> Result<ValidUpgrade, Refusal> {
    let text = String::from_utf8_lossy(head);
    let mut lines = text.split("\r\n");
    let Some(request_line) = lines.next() else {
        return Err(Refusal::Invalid);
    };
    // Syntax comes first, so a malformed line is refused before its path is judged.
    if has_control(request_line) {
        return Err(Refusal::Invalid);
    }
    let mut headers: Vec<(String, &str)> = Vec::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(Refusal::Invalid);
        };
        if !is_token(name) || has_control(value) {
            return Err(Refusal::Invalid);
        }
        headers.push((name.to_ascii_lowercase(), value.trim_matches([' ', '\t'])));
    }

    let mut parts = request_line.split(' ');
    let (Some(method), Some(target), Some(version), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(Refusal::Invalid);
    };
    let path = target.split_once('?').map_or(target, |(path, _)| path);
    if path != RELAY_PATH {
        return Err(Refusal::NotFound);
    }
    if method != "GET" {
        return Err(Refusal::MethodNotAllowed);
    }
    if !is_http_1_1_or_later(version) {
        return Err(Refusal::Invalid);
    }

    if values(&headers, "host").next().is_none() {
        return Err(Refusal::Invalid);
    }
    if !values(&headers, "upgrade").any(|value| has_token(value, "websocket")) {
        return Err(Refusal::Invalid);
    }
    if !values(&headers, "connection").any(|value| has_token(value, "upgrade")) {
        return Err(Refusal::Invalid);
    }
    let mut versions = values(&headers, "sec-websocket-version");
    if !matches!((versions.next(), versions.next()), (Some("13"), None)) {
        return Err(Refusal::VersionUnsupported);
    }
    let mut keys = values(&headers, "sec-websocket-key");
    let key = match (keys.next(), keys.next()) {
        (Some(key), None) if is_sixteen_byte_key(key) => key,
        _ => return Err(Refusal::Invalid),
    };

    let mut origin_values = values(&headers, "origin");
    if let Some(origin) = origin_values.next() {
        if origin_values.next().is_some() {
            return Err(Refusal::Invalid);
        }
        let allowed = Origin::parse(origin).is_ok_and(|parsed| origins.contains(&parsed));
        if !allowed {
            return Err(Refusal::OriginNotAllowed(origin.to_owned()));
        }
    }

    let tokens: Vec<&str> = values(&headers, "sec-websocket-protocol")
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .collect();
    if !tokens.is_empty() && !tokens.contains(&"binary") {
        return Err(Refusal::SubprotocolNotSupported);
    }

    Ok(ValidUpgrade {
        accept_key: derive_accept_key(key.as_bytes()),
        binary: tokens.contains(&"binary"),
    })
}

/// The 101 answer for an accepted upgrade. It echoes `binary` when the client
/// offered it, and nothing else.
pub(crate) fn switching_protocols(upgrade: &ValidUpgrade) -> Vec<u8> {
    let protocol = if upgrade.binary {
        "Sec-WebSocket-Protocol: binary\r\n"
    } else {
        ""
    };
    format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n{protocol}\r\n",
        upgrade.accept_key
    )
    .into_bytes()
}

/// The values of every header with this lower-case name, in order.
fn values<'a>(
    headers: &'a [(String, &'a str)],
    name: &'a str,
) -> impl Iterator<Item = &'a str> + 'a {
    headers
        .iter()
        .filter(move |(header, _)| header == name)
        .map(|(_, value)| *value)
}

/// Whether a comma-separated header value has this token, ignoring case.
fn has_token(value: &str, token: &str) -> bool {
    value
        .split(',')
        .any(|item| item.trim().eq_ignore_ascii_case(token))
}

/// Whether an HTTP version is 1.1 or later.
fn is_http_1_1_or_later(version: &str) -> bool {
    let Some(numbers) = version.strip_prefix("HTTP/") else {
        return false;
    };
    let (major, minor) = numbers.split_once('.').unwrap_or((numbers, "0"));
    match (major.parse::<u32>(), minor.parse::<u32>()) {
        (Ok(major), Ok(minor)) => (major, minor) >= (1, 1),
        _ => false,
    }
}

/// Whether a `Sec-WebSocket-Key` has the shape of base64 for 16 bytes: 24
/// characters ending in `==`. The unused bits of the last group are not checked.
fn is_sixteen_byte_key(key: &str) -> bool {
    let bytes = key.as_bytes();
    bytes.len() == 24
        && bytes.ends_with(b"==")
        && bytes[..22]
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'+' || *byte == b'/')
}

/// Whether a header name is an RFC 9110 token: ASCII letters, digits, and
/// `!#$%&'*+-.^_`|~`. Empty names and names with spaces are not tokens.
fn is_token(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
}

/// Whether text has a control character other than tab. Bare CR, bare LF,
/// NUL, and DEL are refused; tab is allowed inside field values (RFC 9110 §5.5).
fn has_control(text: &str) -> bool {
    text.bytes()
        .any(|byte| (byte < 0x20 && byte != b'\t') || byte == 0x7f)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACCEPT: &str = "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=";

    /// The lines of a valid request. The key is the RFC 6455 example key.
    const VALID: [&str; 6] = [
        "GET /relay HTTP/1.1",
        "Host: relay.example:8443",
        "Upgrade: websocket",
        "Connection: Upgrade",
        "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==",
        "Sec-WebSocket-Version: 13",
    ];

    fn head(lines: &[&str]) -> Vec<u8> {
        let mut text = String::new();
        for line in lines {
            text.push_str(line);
            text.push_str("\r\n");
        }
        text.push_str("\r\n");
        text.into_bytes()
    }

    /// `VALID`, with every line that starts with `prefix` replaced by `with`,
    /// and `extra` lines added at the end.
    fn request(replace: Option<(&str, &str)>, extra: &[&str]) -> Vec<u8> {
        let mut lines: Vec<&str> = VALID
            .iter()
            .map(|&line| match replace {
                Some((prefix, with)) if line.starts_with(prefix) => with,
                _ => line,
            })
            .collect();
        lines.extend_from_slice(extra);
        head(&lines)
    }

    fn origins(list: &[&str]) -> Vec<Origin> {
        list.iter()
            .filter_map(|text| Origin::parse(text).ok())
            .collect()
    }

    #[test]
    fn valid_request_gets_the_rfc_accept_key() {
        assert_eq!(
            check(&request(None, &[]), &[]),
            Ok(ValidUpgrade {
                accept_key: ACCEPT.to_owned(),
                binary: false,
            })
        );
    }

    #[test]
    fn query_string_does_not_change_the_path() {
        let head = request(Some(("GET ", "GET /relay?room=1 HTTP/1.1")), &[]);
        assert!(check(&head, &[]).is_ok());
    }

    #[test]
    fn other_paths_are_not_found() {
        for target in ["/", "/relay/", "/relayx", "/api/v1/session"] {
            let line = format!("GET {target} HTTP/1.1");
            let head = request(Some(("GET ", line.as_str())), &[]);
            assert_eq!(check(&head, &[]), Err(Refusal::NotFound), "{target}");
        }
    }

    #[test]
    fn method_other_than_get_is_not_allowed() {
        let head = request(Some(("GET ", "POST /relay HTTP/1.1")), &[]);
        assert_eq!(check(&head, &[]), Err(Refusal::MethodNotAllowed));
    }

    #[test]
    fn http_below_1_1_is_invalid() {
        let head = request(Some(("GET ", "GET /relay HTTP/1.0")), &[]);
        assert_eq!(check(&head, &[]), Err(Refusal::Invalid));
    }

    #[test]
    fn malformed_request_line_is_invalid() {
        let head = head(&["GET /relay", "Host: relay.example"]);
        assert_eq!(check(&head, &[]), Err(Refusal::Invalid));
    }

    #[test]
    fn missing_host_is_invalid() {
        let head = head(&[
            "GET /relay HTTP/1.1",
            "Upgrade: websocket",
            "Connection: Upgrade",
            "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==",
            "Sec-WebSocket-Version: 13",
        ]);
        assert_eq!(check(&head, &[]), Err(Refusal::Invalid));
    }

    #[test]
    fn upgrade_header_must_name_websocket() {
        let head = request(Some(("Upgrade:", "Upgrade: h2c")), &[]);
        assert_eq!(check(&head, &[]), Err(Refusal::Invalid));
    }

    #[test]
    fn connection_header_must_name_upgrade() {
        let head = request(Some(("Connection:", "Connection: keep-alive")), &[]);
        assert_eq!(check(&head, &[]), Err(Refusal::Invalid));
    }

    #[test]
    fn key_must_be_one_sixteen_byte_value() {
        let short = request(
            Some(("Sec-WebSocket-Key:", "Sec-WebSocket-Key: c2hvcnQ=")),
            &[],
        );
        assert_eq!(check(&short, &[]), Err(Refusal::Invalid));
        let repeated = request(None, &["Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ=="]);
        assert_eq!(check(&repeated, &[]), Err(Refusal::Invalid));
    }

    #[test]
    fn version_other_than_13_is_426() {
        let head = request(
            Some(("Sec-WebSocket-Version:", "Sec-WebSocket-Version: 8")),
            &[],
        );
        assert_eq!(check(&head, &[]), Err(Refusal::VersionUnsupported));
    }

    #[test]
    fn duplicate_origin_is_invalid() {
        let head = request(
            None,
            &[
                "Origin: https://a.example:8443",
                "Origin: https://a.example:8443",
            ],
        );
        let allowed = origins(&["https://a.example:8443"]);
        assert_eq!(check(&head, &allowed), Err(Refusal::Invalid));
    }

    #[test]
    fn origin_on_the_list_is_accepted() {
        let head = request(None, &["Origin: https://a.example:8443"]);
        let allowed = origins(&["https://a.example:8443"]);
        assert!(check(&head, &allowed).is_ok());
    }

    #[test]
    fn origin_not_on_the_list_is_refused_with_its_value() {
        let head = request(None, &["Origin: https://b.example"]);
        let allowed = origins(&["https://a.example:8443"]);
        assert_eq!(
            check(&head, &allowed),
            Err(Refusal::OriginNotAllowed("https://b.example".to_owned()))
        );
    }

    #[test]
    fn empty_origin_list_refuses_every_origin() {
        let head = request(None, &["Origin: https://a.example:8443"]);
        assert_eq!(
            check(&head, &[]),
            Err(Refusal::OriginNotAllowed(
                "https://a.example:8443".to_owned()
            ))
        );
    }

    #[test]
    fn origin_with_another_port_or_scheme_is_refused() {
        let allowed = origins(&["https://a.example:8443"]);
        for origin in [
            "https://a.example",
            "http://a.example:8443",
            "https://x.a.example:8443",
        ] {
            let line = format!("Origin: {origin}");
            let head = request(None, &[line.as_str()]);
            assert!(
                matches!(check(&head, &allowed), Err(Refusal::OriginNotAllowed(_))),
                "{origin}"
            );
        }
    }

    #[test]
    fn binary_subprotocol_is_echoed() {
        let head = request(None, &["Sec-WebSocket-Protocol: json, binary"]);
        let Ok(upgrade) = check(&head, &[]) else {
            panic!("binary is accepted");
        };
        assert!(upgrade.binary);
        let answer = String::from_utf8_lossy(&switching_protocols(&upgrade)).into_owned();
        assert!(answer.starts_with("HTTP/1.1 101 Switching Protocols\r\n"));
        assert!(answer.contains("Sec-WebSocket-Protocol: binary\r\n"));
        assert!(answer.ends_with("\r\n\r\n"));
    }

    #[test]
    fn subprotocol_without_binary_is_refused() {
        let head = request(None, &["Sec-WebSocket-Protocol: json"]);
        assert_eq!(check(&head, &[]), Err(Refusal::SubprotocolNotSupported));
    }

    #[test]
    fn switching_protocols_without_subprotocol_has_no_protocol_header() {
        let upgrade = ValidUpgrade {
            accept_key: ACCEPT.to_owned(),
            binary: false,
        };
        let answer = String::from_utf8_lossy(&switching_protocols(&upgrade)).into_owned();
        assert_eq!(
            answer,
            format!(
                "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {ACCEPT}\r\n\r\n"
            )
        );
    }

    #[test]
    fn refusal_answers_carry_their_status_and_headers() {
        let text = |refusal: Refusal| String::from_utf8_lossy(&refusal.answer()).into_owned();
        assert!(text(Refusal::NotFound).starts_with("HTTP/1.1 404 Not Found\r\n"));
        assert!(text(Refusal::MethodNotAllowed).contains("Allow: GET\r\n"));
        assert!(text(Refusal::VersionUnsupported).contains("Sec-WebSocket-Version: 13\r\n"));
        assert!(text(Refusal::IpNotAllowed).starts_with("HTTP/1.1 403 Forbidden\r\n"));
        assert!(text(Refusal::Invalid).ends_with("Connection: close\r\n\r\n"));
    }

    #[test]
    fn refusal_reasons_match_the_ops_info_names() {
        assert_eq!(Refusal::Invalid.reason().as_str(), "invalid_upgrade");
        assert_eq!(
            Refusal::OriginNotAllowed(String::new()).reason().as_str(),
            "origin_not_allowed"
        );
        assert_eq!(
            Refusal::SubprotocolNotSupported.reason().as_str(),
            "subprotocol_not_supported"
        );
        assert_eq!(Refusal::IpNotAllowed.reason().as_str(), "ip_not_allowed");
    }

    #[tokio::test]
    async fn read_head_keeps_the_head_and_drops_what_follows() {
        let mut bytes: &[u8] = b"GET /relay HTTP/1.1\r\nHost: a\r\n\r\nextra";
        assert_eq!(
            read_head(&mut bytes).await.ok(),
            Some(HeadRead::Complete(
                b"GET /relay HTTP/1.1\r\nHost: a\r\n\r\n".to_vec()
            ))
        );
    }

    #[tokio::test]
    async fn read_head_reports_a_closed_peer() {
        let mut bytes: &[u8] = b"GET /relay HTTP/1.1\r\n";
        assert_eq!(read_head(&mut bytes).await.ok(), Some(HeadRead::Eof));
    }

    #[tokio::test]
    async fn read_head_refuses_a_head_over_the_limit() {
        let flood = vec![b'a'; HEAD_LIMIT + 1];
        let mut bytes: &[u8] = &flood;
        assert_eq!(read_head(&mut bytes).await.ok(), Some(HeadRead::TooLarge));
    }

    /// spec: websocket.md:WS-012 partial
    #[test]
    fn header_names_must_be_tokens() {
        for line in ["Bad(Name: x", "Bad\"Name: x", ": no name"] {
            let mut lines = VALID.to_vec();
            lines.push(line);
            assert_eq!(check(&head(&lines), &[]), Err(Refusal::Invalid), "{line}");
        }
    }

    /// spec: websocket.md:WS-012 partial
    #[test]
    fn control_characters_in_the_head_are_refused() {
        let cases = [
            request(Some(("GET ", "GET /relay\u{0} HTTP/1.1")), &[]),
            request(Some(("Host", "Host: a\u{0}b")), &[]),
            request(Some(("Host", "Host: a\rb")), &[]),
            request(Some(("Host", "Host: a\nb")), &[]),
        ];
        for bytes in cases {
            assert_eq!(check(&bytes, &[]), Err(Refusal::Invalid), "{bytes:?}");
        }
        let tab = request(None, &["X-Note: a\tb"]);
        assert!(check(&tab, &[]).is_ok(), "a tab inside a value is allowed");
    }

    /// spec: websocket.md:WS-012 partial
    #[test]
    fn syntax_is_checked_before_the_path() {
        let bytes = request(Some(("GET ", "GET /other HTTP/1.1")), &["Bad(Name: x"]);
        assert_eq!(check(&bytes, &[]), Err(Refusal::Invalid));
    }

    /// spec: websocket.md:WS-012 partial
    #[test]
    fn wrong_version_is_refused_before_a_bad_key() {
        let bytes = head(&[
            "GET /relay HTTP/1.1",
            "Host: relay.example:8443",
            "Upgrade: websocket",
            "Connection: Upgrade",
            "Sec-WebSocket-Key: c2hvcnQ=",
            "Sec-WebSocket-Version: 12",
        ]);
        assert_eq!(check(&bytes, &[]), Err(Refusal::VersionUnsupported));
    }
}
