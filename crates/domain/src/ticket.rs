//! Relay access ticket rules that need no JSON or cryptography
//! (websocket-relay-v2 §1.9, config-v4 §1.11).

use std::collections::{BTreeSet, HashSet};
use std::fmt;
use std::sync::Arc;

use crate::entry_point::Role;
use crate::relay_id::RelayId;

/// Fixed clock skew allowance in seconds (websocket-relay-v2 §1.9.3).
pub const CLOCK_SKEW_SECS: u64 = 5;
/// Maximum byte length of `kid`, `jti`, and `sub` (websocket-relay-v2 §1.9.2).
pub const ID_MAX_LEN: usize = 128;
/// Maximum number of public keys in one key file (config-v4 §1.11).
pub const MAX_KEYS: usize = 16;

/// Returns the fixed `aud` value of `role` (websocket-relay-v2 §1.9.2).
pub fn audience(role: Role) -> &'static str {
    match role {
        Role::Viewer => "vnc-relay/viewer",
        Role::Server => "vnc-relay/server",
    }
}

/// Returns whether `text` is a valid `kid`, `jti`, or `sub`: 1 to 128 bytes.
pub fn id_is_valid(text: &str) -> bool {
    (1..=ID_MAX_LEN).contains(&text.len())
}

/// Decodes unpadded base64url (RFC 7515 §2).
///
/// Rejects padding, characters outside the URL-safe alphabet, a length of
/// 1 mod 4, and non-zero trailing bits, so each byte string has exactly one
/// accepted encoding.
pub fn decode_base64url(text: &str) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();
    if bytes.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3 + 2);
    let mut buffer: u32 = 0;
    let mut bits = 0u32;
    for &byte in bytes {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            _ => return None,
        };
        buffer = (buffer << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    // The bits left after the last whole byte must be zero.
    (buffer == 0).then_some(out)
}

/// The parts of a JWS compact serialization (RFC 7515 §7.1).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TicketParts<'a> {
    /// `BASE64URL(header) '.' BASE64URL(payload)`, the bytes that are signed.
    pub signing_input: &'a str,
    pub header: &'a str,
    pub payload: &'a str,
    pub signature: &'a str,
}

impl fmt::Debug for TicketParts<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The parts together are the bearer ticket (ops-info-v2 §1.11).
        f.write_str("TicketParts(<redacted>)")
    }
}

/// Splits `text` into exactly three dot-separated parts.
/// Decoding of each part is left to the caller.
pub fn split_compact(text: &str) -> Option<TicketParts<'_>> {
    let (signing_input, signature) = text.rsplit_once('.')?;
    let (header, payload) = signing_input.split_once('.')?;
    if payload.contains('.') {
        return None;
    }
    Some(TicketParts {
        signing_input,
        header,
        payload,
        signature,
    })
}

/// Claims of a ticket whose types and identifier syntax are already checked
/// (websocket-relay-v2 §1.9.3 step 6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketClaims {
    pub iss: String,
    pub aud: String,
    pub rh: String,
    pub rid: RelayId,
    pub jti: String,
    pub iat: u64,
    pub exp: u64,
    pub sx: Option<u64>,
    pub sub: Option<String>,
}

/// Claim comparison and time rules of one role (websocket-relay-v2 §1.9.3 steps 7 to 11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TicketPolicy<'a> {
    pub role: Role,
    pub issuer: &'a str,
    pub relay_host: &'a str,
    pub max_lifetime_secs: u64,
    /// Unix seconds at which the `serve` process started.
    pub started_at_secs: u64,
}

impl TicketPolicy<'_> {
    /// Checks steps 7 to 11 in contract order at `now` (Unix seconds, rounded down)
    /// and returns the first failure.
    ///
    /// Each contract row states the condition a ticket must meet; the code tests its
    /// negation. Step 11 requires `iat > start + 5`, so `iat <= start + 5` fails with
    /// `ticket_issued_before_start` (ops-info-v2 `connection_rejected` reasons, Q201).
    pub fn check(&self, claims: &TicketClaims, now: u64) -> Result<(), TicketError> {
        if claims.iss != self.issuer
            || claims.aud != audience(self.role)
            || claims.rh != self.relay_host
        {
            return Err(TicketError::Invalid);
        }
        if claims.iat >= claims.exp || claims.exp - claims.iat > self.max_lifetime_secs {
            return Err(TicketError::Invalid);
        }
        if claims.iat > now.saturating_add(CLOCK_SKEW_SECS) {
            return Err(TicketError::Invalid);
        }
        if now >= claims.exp.saturating_add(CLOCK_SKEW_SECS)
            || claims.sx.is_some_and(|sx| now >= sx)
        {
            return Err(TicketError::Expired);
        }
        if claims.iat <= self.started_at_secs.saturating_add(CLOCK_SKEW_SECS) {
            return Err(TicketError::IssuedBeforeStart);
        }
        Ok(())
    }
}

/// Why an upgrade was refused by the ticket check (websocket-relay-v2 §1.9.3, §1.9.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketError {
    /// No `ticket.` value was offered.
    Missing,
    /// More than one value, or a format, signature, claim, or role mismatch.
    Invalid,
    /// `exp + 5` or `sx` has been reached.
    Expired,
    /// `iat` is at or before the process start time plus 5 seconds.
    IssuedBeforeStart,
    /// The role's use record already holds this `jti`.
    Reused,
}

impl TicketError {
    /// Contract `reason` value.
    pub fn code(self) -> &'static str {
        match self {
            Self::Missing => "ticket_missing",
            Self::Invalid => "ticket_invalid",
            Self::Expired => "ticket_expired",
            Self::IssuedBeforeStart => "ticket_issued_before_start",
            Self::Reused => "ticket_reused",
        }
    }
}

impl fmt::Display for TicketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Missing => "no access ticket was offered",
            Self::Invalid => "the access ticket is not valid",
            Self::Expired => "the access ticket has expired",
            Self::IssuedBeforeStart => "the access ticket was issued before the server started",
            Self::Reused => "the access ticket was already used",
        })
    }
}

impl std::error::Error for TicketError {}

/// One P-256 public key from a key file (config-v4 §1.11).
/// The adapter that builds it checks that the point is on the curve.
#[derive(Clone, PartialEq, Eq)]
pub struct TicketKey {
    kid: String,
    x: [u8; 32],
    y: [u8; 32],
}

impl TicketKey {
    /// Checks the `kid` length (1 to 128 bytes).
    pub fn new(kid: String, x: [u8; 32], y: [u8; 32]) -> Result<Self, TicketKeyError> {
        if !id_is_valid(&kid) {
            return Err(TicketKeyError::KeyId);
        }
        Ok(Self { kid, x, y })
    }

    pub fn kid(&self) -> &str {
        &self.kid
    }

    /// Uncompressed SEC 1 point `0x04 || x || y`.
    pub fn uncompressed_point(&self) -> [u8; 65] {
        let mut point = [0u8; 65];
        point[0] = 0x04;
        point[1..33].copy_from_slice(&self.x);
        point[33..].copy_from_slice(&self.y);
        point
    }

    fn same_point(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl fmt::Debug for TicketKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Key file contents never appear in logs or diagnostics (config-v4 §1.11).
        f.write_str("TicketKey(<redacted>)")
    }
}

/// The public keys of one role, sorted by `kid` (config-v4 §1.11).
///
/// A list loaded from a file holds 1 to 16 keys with unique `kid` values.
/// The default list is empty and stands for a role whose check is off.
/// Equality ignores file order.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct TicketKeyList(Vec<TicketKey>);

impl TicketKeyList {
    /// Checks the key count and `kid` uniqueness.
    pub fn new(mut keys: Vec<TicketKey>) -> Result<Self, TicketKeyError> {
        if keys.is_empty() || keys.len() > MAX_KEYS {
            return Err(TicketKeyError::KeyCount);
        }
        keys.sort_by(|a, b| a.kid.cmp(&b.kid));
        if keys.windows(2).any(|pair| pair[0].kid == pair[1].kid) {
            return Err(TicketKeyError::DuplicateKeyId);
        }
        Ok(Self(keys))
    }

    /// Finds the key whose `kid` equals `kid` byte for byte.
    pub fn find(&self, kid: &str) -> Option<&TicketKey> {
        self.0
            .binary_search_by(|key| key.kid.as_str().cmp(kid))
            .ok()
            .map(|index| &self.0[index])
    }

    /// Whether any key of `self` has the same point as a key of `other`.
    /// The two roles must use different signing keys (config-v4 §1.6).
    pub fn shares_key_with(&self, other: &Self) -> bool {
        self.0
            .iter()
            .any(|key| other.0.iter().any(|candidate| key.same_point(candidate)))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl fmt::Debug for TicketKeyList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TicketKeyList({} keys)", self.0.len())
    }
}

/// A public key file breaks config-v4 §1.11, or both roles share a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketKeyError {
    Unreadable,
    TooLarge,
    Format,
    KeyCount,
    KeyType,
    Coordinates,
    NotOnCurve,
    KeyId,
    DuplicateKeyId,
    KeyUse,
    Algorithm,
    PrivateKey,
    SharedAcrossRoles,
}

impl TicketKeyError {
    /// Contract error code (config-v4 §2.1).
    pub fn code(self) -> &'static str {
        "ticket_keys_invalid"
    }

    /// Fixed short cause. It never contains file contents or paths.
    pub fn detail(self) -> &'static str {
        match self {
            Self::Unreadable => "the public key file cannot be read",
            Self::TooLarge => "the public key file is larger than 64 KiB",
            Self::Format => "the public key file is not a UTF-8 JSON object with a keys array",
            Self::KeyCount => "the keys array must hold 1 to 16 keys",
            Self::KeyType => "kty must be EC and crv must be P-256",
            Self::Coordinates => "x and y must be unpadded base64url values of 32 bytes",
            Self::NotOnCurve => "the public key is not a point on P-256",
            Self::KeyId => "kid must be a string of 1 to 128 bytes",
            Self::DuplicateKeyId => "kid values must be unique in the file",
            Self::KeyUse => "use must be sig when present",
            Self::Algorithm => "alg must be ES256 when present",
            Self::PrivateKey => "the private key member d is not allowed",
            Self::SharedAcrossRoles => "the viewer and server roles use the same public key",
        }
    }
}

impl fmt::Display for TicketKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.detail())
    }
}

impl std::error::Error for TicketKeyError {}

/// The `(role, jti)` use record (websocket-relay-v2 §1.9.4).
///
/// Holds only tickets whose signature was verified. Each record is removed at
/// `exp + 5` seconds. There is no count limit.
#[derive(Default)]
pub struct TicketUseTable {
    viewer: HashSet<Arc<str>>,
    server: HashSet<Arc<str>>,
    expiry: BTreeSet<(u64, Role, Arc<str>)>,
}

impl TicketUseTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records `(role, jti)` at `now` (Unix seconds) unless it is already recorded.
    ///
    /// First removes the records whose `exp + 5` is at or before `now`; the time
    /// check rejects those tickets anyway. Returns `false` when the record exists.
    /// The caller holds one lock around this call, so the check and the write
    /// cannot be split by another request.
    pub fn try_record(&mut self, role: Role, jti: &str, exp: u64, now: u64) -> bool {
        while let Some((expires_at, _, _)) = self.expiry.first() {
            if *expires_at > now {
                break;
            }
            if let Some((_, role, jti)) = self.expiry.pop_first() {
                self.set_mut(role).remove(&jti);
            }
        }
        if self.set_mut(role).contains(jti) {
            return false;
        }
        let jti: Arc<str> = Arc::from(jti);
        self.set_mut(role).insert(Arc::clone(&jti));
        self.expiry
            .insert((exp.saturating_add(CLOCK_SKEW_SECS), role, jti));
        true
    }

    /// Number of records currently held.
    pub fn len(&self) -> usize {
        self.expiry.len()
    }

    pub fn is_empty(&self) -> bool {
        self.expiry.is_empty()
    }

    fn set_mut(&mut self, role: Role) -> &mut HashSet<Arc<str>> {
        match role {
            Role::Viewer => &mut self.viewer,
            Role::Server => &mut self.server,
        }
    }
}

impl fmt::Debug for TicketUseTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TicketUseTable({} records)", self.expiry.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims(iat: u64, exp: u64, sx: Option<u64>) -> TicketClaims {
        TicketClaims {
            iss: "https://admin.example".to_owned(),
            aud: "vnc-relay/viewer".to_owned(),
            rh: "relay.example".to_owned(),
            rid: RelayId::parse("1234").unwrap(),
            jti: "j-1".to_owned(),
            iat,
            exp,
            sx,
            sub: None,
        }
    }

    fn policy(started_at_secs: u64) -> TicketPolicy<'static> {
        TicketPolicy {
            role: Role::Viewer,
            issuer: "https://admin.example",
            relay_host: "relay.example",
            max_lifetime_secs: 60,
            started_at_secs,
        }
    }

    fn key(kid: &str, x: u8) -> TicketKey {
        TicketKey::new(kid.to_owned(), [x; 32], [x.wrapping_add(1); 32]).unwrap()
    }

    #[test]
    fn decodes_only_canonical_unpadded_base64url() {
        assert_eq!(decode_base64url("").unwrap(), b"");
        assert_eq!(decode_base64url("Zg").unwrap(), b"f");
        assert_eq!(decode_base64url("Zm8").unwrap(), b"fo");
        assert_eq!(decode_base64url("Zm9v").unwrap(), b"foo");
        assert_eq!(decode_base64url("-_8").unwrap(), [0xfb, 0xff]);
        // Padding, the standard alphabet, a dangling character, and non-zero
        // trailing bits each have another accepted spelling or none at all.
        for text in ["Zg==", "Zm8=", "+/8", "Z", "Zm9vY", "Zh", "Zm9", "Z g"] {
            assert_eq!(decode_base64url(text), None, "{text}");
        }
    }

    #[test]
    fn splits_exactly_three_parts_and_keeps_the_signing_input() {
        let parts = split_compact("aGVhZA.Ym9keQ.c2ln").unwrap();
        assert_eq!(parts.signing_input, "aGVhZA.Ym9keQ");
        assert_eq!(
            (parts.header, parts.payload, parts.signature),
            ("aGVhZA", "Ym9keQ", "c2ln")
        );
        assert_eq!(split_compact("a.b").map(|p| p.header), None);
        assert_eq!(split_compact("a.b.c.d").map(|p| p.header), None);
        assert_eq!(split_compact("..").map(|p| p.signing_input), Some("."));
    }

    /// spec: websocket.md:WS-040 partial
    #[test]
    fn compares_issuer_audience_and_relay_host_byte_for_byte() {
        let base = claims(1000, 1060, None);
        assert_eq!(policy(0).check(&base, 1000), Ok(()));
        let changes: [fn(&mut TicketClaims); 3] = [
            |c| c.iss.push('/'),
            |c| c.aud = "vnc-relay/server".to_owned(),
            |c| c.rh = "Relay.example".to_owned(),
        ];
        for change in changes {
            let mut changed = base.clone();
            change(&mut changed);
            assert_eq!(policy(0).check(&changed, 1000), Err(TicketError::Invalid));
        }
        let server = TicketPolicy {
            role: Role::Server,
            ..policy(0)
        };
        let mut server_claims = base.clone();
        server_claims.aud = "vnc-relay/server".to_owned();
        assert_eq!(server.check(&server_claims, 1000), Ok(()));
    }

    /// spec: websocket.md:WS-040 partial
    #[test]
    fn rejects_bad_lifetimes_and_future_issue_times() {
        let policy = policy(0);
        assert_eq!(
            policy.check(&claims(1000, 1000, None), 1000),
            Err(TicketError::Invalid)
        );
        assert_eq!(
            policy.check(&claims(1000, 999, None), 1000),
            Err(TicketError::Invalid)
        );
        assert_eq!(
            policy.check(&claims(1000, 1061, None), 1000),
            Err(TicketError::Invalid)
        );
        assert_eq!(policy.check(&claims(1000, 1060, None), 1000), Ok(()));
        // iat may be up to 5 seconds ahead of the relay clock.
        assert_eq!(policy.check(&claims(1005, 1010, None), 1000), Ok(()));
        assert_eq!(
            policy.check(&claims(1006, 1010, None), 1000),
            Err(TicketError::Invalid)
        );
    }

    /// spec: websocket.md:WS-041 partial
    #[test]
    fn expires_at_exp_plus_skew_and_exactly_at_sx() {
        let policy = policy(0);
        let ticket = claims(1000, 1060, None);
        assert_eq!(policy.check(&ticket, 1064), Ok(()));
        assert_eq!(policy.check(&ticket, 1065), Err(TicketError::Expired));
        let session = claims(1000, 1060, Some(1030));
        assert_eq!(policy.check(&session, 1029), Ok(()));
        assert_eq!(policy.check(&session, 1030), Err(TicketError::Expired));
        // A time failure comes after the lifetime rule.
        assert_eq!(
            policy.check(&claims(1000, 1061, None), 2000),
            Err(TicketError::Invalid)
        );
    }

    /// spec: websocket.md:WS-043 partial
    #[test]
    fn rejects_tickets_issued_at_or_before_start_plus_skew() {
        assert_eq!(
            policy(1000).check(&claims(1005, 1060, None), 1010),
            Err(TicketError::IssuedBeforeStart)
        );
        assert_eq!(policy(1000).check(&claims(1006, 1060, None), 1010), Ok(()));
        // An expired ticket reports expiry first.
        assert_eq!(
            policy(1000).check(&claims(1000, 1010, None), 1020),
            Err(TicketError::Expired)
        );
    }

    #[test]
    fn key_lists_need_one_to_sixteen_unique_ids_and_ignore_order() {
        assert_eq!(
            TicketKeyList::new(Vec::new()),
            Err(TicketKeyError::KeyCount)
        );
        let many = (0..17).map(|i| key(&format!("k{i:02}"), i)).collect();
        assert_eq!(TicketKeyList::new(many), Err(TicketKeyError::KeyCount));
        assert_eq!(
            TicketKeyList::new(vec![key("a", 1), key("a", 2)]),
            Err(TicketKeyError::DuplicateKeyId)
        );
        assert_eq!(
            TicketKey::new(String::new(), [0; 32], [0; 32]),
            Err(TicketKeyError::KeyId)
        );
        assert_eq!(
            TicketKey::new("k".repeat(129), [0; 32], [0; 32]),
            Err(TicketKeyError::KeyId)
        );
        let list = TicketKeyList::new(vec![key("b", 2), key("a", 1)]).unwrap();
        assert_eq!(
            list,
            TicketKeyList::new(vec![key("a", 1), key("b", 2)]).unwrap()
        );
        assert_eq!(list.find("a").map(TicketKey::kid), Some("a"));
        assert_eq!(list.find("A").map(TicketKey::kid), None);
        assert_eq!(format!("{list:?}"), "TicketKeyList(2 keys)");
    }

    #[test]
    fn detects_the_same_point_under_any_kid() {
        let viewer = TicketKeyList::new(vec![key("v1", 1), key("v2", 2)]).unwrap();
        let server = TicketKeyList::new(vec![key("s1", 3)]).unwrap();
        assert!(!viewer.shares_key_with(&server));
        let shared = TicketKeyList::new(vec![key("s1", 3), key("other-name", 2)]).unwrap();
        assert!(viewer.shares_key_with(&shared));
    }

    /// spec: websocket.md:WS-042 partial
    #[test]
    fn records_each_role_and_jti_once_until_exp_plus_skew() {
        let mut table = TicketUseTable::new();
        assert!(table.try_record(Role::Viewer, "j", 1060, 1000));
        assert!(!table.try_record(Role::Viewer, "j", 1060, 1001));
        // The same jti of the other role is a separate record.
        assert!(table.try_record(Role::Server, "j", 1060, 1001));
        assert!(!table.try_record(Role::Viewer, "j", 1060, 1064));
        // At exp + 5 the record is removed.
        assert!(table.try_record(Role::Viewer, "j", 1100, 1065));
        assert_eq!(table.len(), 1);
    }
}
