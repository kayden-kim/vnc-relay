//! OS random numbers and HMAC-SHA-256 with aws-lc-rs. Design:
//! docs/architectures/ops-log.md (`relay_id_ref`, ops-info-v1 §1.9).

use std::fmt;

use aws_lc_rs::{hmac, rand};
use vnc_relay_app::events::RelayIdRef;

const KEY_BYTES: usize = 32;

/// The per-process key of `relay_id_ref`. It is made from OS random bytes,
/// never stored, and never written; `Debug` hides it.
pub struct RelayIdKey(hmac::Key);

impl RelayIdKey {
    /// A new key from 32 OS random bytes.
    pub fn generate() -> Result<Self, CryptoError> {
        let mut bytes = [0u8; KEY_BYTES];
        rand::fill(&mut bytes).map_err(|_| CryptoError)?;
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
}
