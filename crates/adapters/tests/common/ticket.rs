//! Test-only P-256 ticket signer, shared with the root system tests by path.

use aws_lc_rs::rand::SystemRandom;
use aws_lc_rs::signature::{
    ECDSA_P256_SHA256_FIXED_SIGNING, EcdsaKeyPair, EcdsaSigningAlgorithm, KeyPair,
};
use serde_json::{Value, json};
use vnc_relay_adapters::ticket::TicketJwksStore;
use vnc_relay_app::reload::ports::ReloadTicketKeyStore;
use vnc_relay_domain::ticket::TicketKeyList;

/// Generates fresh keys in each test. Private key bytes never leave this helper.
pub struct Signer {
    pair: EcdsaKeyPair,
    pub keys: TicketKeyList,
}

impl Signer {
    /// Generates a P-256 key that signs the ES256 fixed-width R||S format.
    pub fn new() -> Self {
        Self::with_algorithm(&ECDSA_P256_SHA256_FIXED_SIGNING)
    }

    /// Allows ASN.1 signing for tests that must reject DER signatures.
    pub fn with_algorithm(algorithm: &'static EcdsaSigningAlgorithm) -> Self {
        let document = must(EcdsaKeyPair::generate_pkcs8(
            algorithm,
            &SystemRandom::new(),
        ));
        let pair = must(EcdsaKeyPair::from_pkcs8(algorithm, document.as_ref()));
        let mut signer = Self {
            pair,
            keys: TicketKeyList::default(),
        };
        signer.keys = must(TicketJwksStore.parse(signer.jwks().as_bytes()));
        signer
    }

    /// Returns a JWKS document with this signer's public key and kid `key`.
    pub fn jwks(&self) -> String {
        let point = self.pair.public_key().as_ref();
        json!({
            "keys": [{
                "kty": "EC", "crv": "P-256", "kid": "key",
                "x": encode(&point[1..33]), "y": encode(&point[33..65])
            }]
        })
        .to_string()
    }

    /// Signs exact JSON text, allowing header overrides and malformed payloads.
    pub fn sign_text(&self, header: &str, payload: &str) -> String {
        let input = format!(
            "{}.{}",
            encode(header.as_bytes()),
            encode(payload.as_bytes())
        );
        let signature = must(self.pair.sign(&SystemRandom::new(), input.as_bytes()));
        format!("{input}.{}", encode(signature.as_ref()))
    }

    /// Signs caller-supplied claims using the valid default ES256 header.
    pub fn sign(&self, claims: &Value) -> String {
        self.sign_text(r#"{"alg":"ES256","kid":"key"}"#, &claims.to_string())
    }
}

fn must<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|error| panic!("test signer failed: {error:?}"))
}

fn encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut text = String::new();
    for chunk in bytes.chunks(3) {
        let mut block = [0_u8; 3];
        block[..chunk.len()].copy_from_slice(chunk);
        let value = u32::from(block[0]) << 16 | u32::from(block[1]) << 8 | u32::from(block[2]);
        for index in 0..=chunk.len() {
            text.push(char::from(TABLE[(value >> (18 - 6 * index) & 63) as usize]));
        }
    }
    text
}
