//! OS random numbers, admin session tokens, and HMAC-SHA-256 with aws-lc-rs.
//! Design: docs/architectures/ops-log.md (`relay_id_ref`, ops-info-v1 §1.9),
//! docs/architectures/admin-accounts.md (password hash salts), and
//! docs/architectures/management-session.md (session IDs, CSRF tokens).

use std::fmt;

use aws_lc_rs::{constant_time, hmac, rand};
use vnc_relay_app::events::RelayIdRef;
use vnc_relay_app::ports::{RandomError, TokenSource};

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

impl fmt::Debug for RelayIdKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RelayIdKey(<redacted>)")
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
        assert!(token.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
        assert!(source.same_token(&token, &token));
        assert!(!source.same_token(&token, "0"));
    }
}
