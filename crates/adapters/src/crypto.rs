//! OS randomness, admin tokens, relay references, and opaque session IDs.
//! Design: docs/architectures/ops-log.md (`relay_id_ref`, ops-info-v1 §1.9),
//! docs/architectures/admin-accounts.md (password hash salts),
//! docs/architectures/management-session.md (admin tokens), and
//! docs/architectures/relay-session.md §3.3 (session ID permutation).

use std::fmt;

use aws_lc_rs::{constant_time, hmac, rand};
use vnc_relay_app::events::RelayIdRef;
use vnc_relay_app::ports::{RandomError, RelayIdSource, SessionIdSource, TokenSource};
use vnc_relay_domain::relay_id::RelayId;
use vnc_relay_domain::session::SessionId;

const KEY_BYTES: usize = 32;
const TOKEN_BYTES: usize = 32;

/// Fills `bytes` from the OS random source.
pub fn random_bytes(bytes: &mut [u8]) -> Result<(), CryptoError> {
    rand::fill(bytes).map_err(|_| CryptoError)
}

/// Admin session IDs and CSRF tokens: 32 OS random bytes as 64 lowercase hex
/// digits (management-api-v1 §1.4.1, §1.4.3). The comparison runs in
/// constant time.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemTokenSource;

impl TokenSource for SystemTokenSource {
    fn new_token(&self) -> Result<String, RandomError> {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut bytes = [0u8; TOKEN_BYTES];
        random_bytes(&mut bytes).map_err(|e| RandomError(e.to_string()))?;
        let mut text = String::with_capacity(TOKEN_BYTES * 2);
        for byte in bytes {
            text.push(char::from(HEX[usize::from(byte >> 4)]));
            text.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        Ok(text)
    }

    fn same_token(&self, expected: &str, given: &str) -> bool {
        constant_time::verify_slices_are_equal(expected.as_bytes(), given.as_bytes()).is_ok()
    }
}

/// The per-process key of `relay_id_ref`. It is made from OS random bytes,
/// never stored, and never written; `Debug` hides it.
pub struct RelayIdKey(hmac::Key);

impl RelayIdKey {
    /// A new key from 32 OS random bytes.
    pub fn generate() -> Result<Self, CryptoError> {
        let mut bytes = [0u8; KEY_BYTES];
        random_bytes(&mut bytes)?;
        Ok(Self::from_bytes(&bytes))
    }

    fn from_bytes(bytes: &[u8; KEY_BYTES]) -> Self {
        Self(hmac::Key::new(hmac::HMAC_SHA256, bytes))
    }

    /// HMAC-SHA-256 of the relay ID bytes used for matching, cut to its
    /// first 8 bytes.
    pub fn reference(&self, relay_id: &[u8]) -> RelayIdRef {
        let tag = hmac::sign(&self.0, relay_id);
        let mut prefix = [0u8; 8];
        prefix.copy_from_slice(&tag.as_ref()[..8]);
        RelayIdRef::from_hash_prefix(prefix)
    }
}

impl RelayIdSource for RelayIdKey {
    fn reference(&self, id: &RelayId) -> RelayIdRef {
        self.reference(id.as_str().as_bytes())
    }
}

impl fmt::Debug for RelayIdKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RelayIdKey(<redacted>)")
    }
}

/// Independent per-process key for the fixed six-round session ID permutation.
/// Generation is fallible only at startup; issuance never requests randomness.
/// See relay-session.md §3.3.
pub struct SessionIdKey(hmac::Key);

impl SessionIdKey {
    /// Creates an independent key from 32 OS random bytes.
    pub fn generate() -> Result<Self, CryptoError> {
        let mut bytes = [0u8; KEY_BYTES];
        random_bytes(&mut bytes)?;
        Ok(Self::from_bytes(&bytes))
    }

    /// Builds a key from supplied bytes, also allowing deterministic fixtures.
    /// Production startup uses `generate` instead of a persisted or fixed key.
    pub fn from_bytes(bytes: &[u8; KEY_BYTES]) -> Self {
        Self(hmac::Key::new(hmac::HMAC_SHA256, bytes))
    }

    fn round_function(&self, round: u8, right: u64) -> u64 {
        let mut context = hmac::Context::with_key(&self.0);
        context.update(b"vnc-relay/session-id/v1");
        context.update(&[round]);
        context.update(&right.to_be_bytes());
        let tag = context.sign();
        let mut prefix = [0; 8];
        prefix.copy_from_slice(&tag.as_ref()[..8]);
        u64::from_be_bytes(prefix)
    }

    fn permute(&self, counter: u128) -> [u8; 16] {
        let mut left = (counter >> 64) as u64;
        let mut right = counter as u64;
        for round in 0..6 {
            (left, right) = (right, left ^ self.round_function(round, right));
        }
        ((u128::from(left) << 64) | u128::from(right)).to_be_bytes()
    }
}

impl SessionIdSource for SessionIdKey {
    fn session_id(&self, counter: u128) -> SessionId {
        SessionId::from_block(self.permute(counter))
    }
}

impl fmt::Debug for SessionIdKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SessionIdKey(<redacted>)")
    }
}

/// The OS random source failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CryptoError;

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("운영체제 난수를 읽을 수 없다.")
    }
}

impl std::error::Error for CryptoError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_hmac_sha256_reference_values() {
        // Values from Python `hmac.new(bytes(range(32)), data, sha256)`.
        let mut bytes = [0u8; KEY_BYTES];
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::try_from(i).unwrap_or_else(|e| panic!("{e}"));
        }
        let key = RelayIdKey::from_bytes(&bytes);
        assert_eq!(key.reference(b"1234").as_str(), "75a2b0661f593e7c");
        assert_eq!(key.reference(b"relay-id-1234").as_str(), "8800f76acc6dd1d2");
    }

    #[test]
    fn tokens_are_64_lowercase_hex_digits_and_compare_exactly() {
        let source = SystemTokenSource;
        let Ok(token) = source.new_token() else {
            panic!("the OS random source must answer in tests");
        };
        assert_eq!(token.len(), 64);
        assert!(
            token
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        );
        assert!(source.same_token(&token, &token));
        assert!(!source.same_token(&token, "0"));
    }

    #[test]
    fn relay_reference_port_keeps_exact_id_bytes() {
        let key = RelayIdKey::from_bytes(&[42; KEY_BYTES]);
        let plain = RelayId::parse("1234").unwrap_or_else(|error| panic!("{error}"));
        let leading_zero = RelayId::parse("01234").unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(RelayIdSource::reference(&key, &plain), key.reference(b"1234"));
        assert_eq!(
            RelayIdSource::reference(&key, &leading_zero),
            key.reference(b"01234")
        );
        assert_ne!(
            RelayIdSource::reference(&key, &plain),
            RelayIdSource::reference(&key, &leading_zero)
        );
    }

    #[test]
    fn session_ids_match_fixed_six_round_reference_vectors() {
        // Independently derived with Python hmac/hashlib and base64.b32encode.
        let bytes = std::array::from_fn(|index| index as u8);
        let key = SessionIdKey::from_bytes(&bytes);
        let same_key = SessionIdKey::from_bytes(&bytes);
        for (counter, expected) in [
            (0, "ses_vurq4rnpvlk3bxjgv23nu7zpce"),
            (1, "ses_j7cd5romitabd5b56ep32agrt4"),
            (2, "ses_hncxi4oiw3ygva3dfu6556gznu"),
            (u128::from(u64::MAX), "ses_kx2dgfrj3yu2yfifwwr7vhyfzu"),
            (1u128 << 64, "ses_wnoeycnwqagbyh5263hjm3zchi"),
            (1u128 << 127, "ses_zayx4zed5ojlwbkuw7xjwfdq4q"),
            (u128::MAX, "ses_4olh2turen4y2pafg3yfgstmlm"),
        ] {
            let id = key.session_id(counter);
            assert_eq!(id.as_str(), expected);
            assert_eq!(id, same_key.session_id(counter));
            assert_eq!(SessionId::parse(expected), Ok(id));
        }
    }

    #[test]
    fn session_permutation_has_an_inverse_and_distinct_counter_results() {
        use std::collections::HashSet;

        let key = SessionIdKey::from_bytes(&[0xa5; KEY_BYTES]);
        let mut issued = HashSet::new();
        for counter in (0..4096).chain([
            u128::from(u64::MAX),
            1u128 << 64,
            (1u128 << 64) + 1,
            1u128 << 127,
            u128::MAX - 1,
            u128::MAX,
        ]) {
            assert!(issued.insert(key.session_id(counter)));
            let output = u128::from_be_bytes(key.permute(counter));
            let mut left = (output >> 64) as u64;
            let mut right = output as u64;
            for round in (0..6).rev() {
                (left, right) = (right ^ key.round_function(round, left), left);
            }
            assert_eq!((u128::from(left) << 64) | u128::from(right), counter);
        }
    }

    #[test]
    fn relay_and_session_keys_hide_all_key_material_in_debug() {
        assert_eq!(
            format!("{:?}", RelayIdKey::from_bytes(&[0xa5; KEY_BYTES])),
            "RelayIdKey(<redacted>)"
        );
        assert_eq!(
            format!("{:?}", SessionIdKey::from_bytes(&[0x5a; KEY_BYTES])),
            "SessionIdKey(<redacted>)"
        );
    }
}
