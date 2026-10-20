//! Argon2id password hashes (cli-v1 §1.4) with the argon2 crate.
//! Design: docs/architectures/admin-accounts.md.

use argon2::{Algorithm, Argon2, Params, PasswordHasher as _, Version};
use vnc_relay_app::ports::{HashError, PasswordHasher};
use vnc_relay_domain::account::{Argon2Params, Password, PasswordHash};

use crate::crypto::random_bytes;

const SALT_BYTES: usize = 16;
const HASH_BYTES: usize = 32;

/// Argon2id, version 19, 32-byte output, 16-byte salt from the OS random
/// source.
#[derive(Debug, Clone, Copy, Default)]
pub struct Argon2Hasher;

impl PasswordHasher for Argon2Hasher {
    fn hash(&self, password: &Password, params: Argon2Params) -> Result<PasswordHash, HashError> {
        let mut salt = [0u8; SALT_BYTES];
        random_bytes(&mut salt).map_err(|e| HashError(e.to_string()))?;
        let params = Params::new(
            params.memory_kib,
            params.iterations,
            params.parallelism,
            Some(HASH_BYTES),
        )
        .map_err(|e| HashError(format!("해시 매개변수를 쓸 수 없다: {e}.")))?;
        let encoded = Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
            .hash_password_with_salt(password.as_bytes(), &salt)
            .map_err(|e| HashError(format!("비밀번호 해시를 만들 수 없다: {e}.")))?
            .to_string();
        PasswordHash::parse(&encoded)
            .map_err(|e| HashError(format!("만든 해시 문자열이 계정 파일 형식과 다르다: {e}.")))
    }
}

#[cfg(test)]
mod tests {
    use argon2::PasswordVerifier as _;

    use super::*;

    /// The lowest approved cost keeps the test fast.
    const PARAMS: Argon2Params = Argon2Params {
        memory_kib: 19_456,
        iterations: 2,
        parallelism: 1,
    };

    /// spec: operations.md:OPS-029 partial
    #[test]
    fn makes_salted_argon2id_hashes_that_verify_the_password() {
        let password =
            Password::parse("correct horse battery".to_owned()).unwrap_or_else(|e| panic!("{e}"));
        let first = Argon2Hasher
            .hash(&password, PARAMS)
            .unwrap_or_else(|e| panic!("{e}"));
        let second = Argon2Hasher
            .hash(&password, PARAMS)
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(
            first
                .as_str()
                .starts_with("$argon2id$v=19$m=19456,t=2,p=1$")
        );
        assert_eq!(first.params(), PARAMS);
        // A new salt each time.
        assert_ne!(first.as_str(), second.as_str());
        let argon2 = Argon2::default();
        assert!(
            argon2
                .verify_password(b"correct horse battery", first.as_str())
                .is_ok()
        );
        assert!(
            argon2
                .verify_password(b"correct horse batterz", first.as_str())
                .is_err()
        );
    }
}
