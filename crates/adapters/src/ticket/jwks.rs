//! Access ticket public key file: a JWKS of P-256 keys (config-v4 §1.11).

use aws_lc_rs::signature::{ECDSA_P256_SHA256_FIXED, ParsedPublicKey};
use serde_json::{Map, Value};
use vnc_relay_app::reload::ports::ReloadTicketKeyStore;
use vnc_relay_domain::ticket::{
    MAX_KEYS, TicketKey, TicketKeyError, TicketKeyList, decode_base64url,
};

/// Largest accepted key file (config-v4 §1.11).
const MAX_FILE_BYTES: usize = 64 * 1024;

/// Parses JWKS bytes into the key list of one role.
///
/// Members other than those of config-v4 §1.11 are ignored. Errors carry a
/// fixed cause only, never file contents.
#[derive(Debug, Clone, Copy, Default)]
pub struct TicketJwksStore;

impl ReloadTicketKeyStore for TicketJwksStore {
    fn parse(&self, bytes: &[u8]) -> Result<TicketKeyList, TicketKeyError> {
        if bytes.len() > MAX_FILE_BYTES {
            return Err(TicketKeyError::TooLarge);
        }
        let text = std::str::from_utf8(bytes).map_err(|_| TicketKeyError::Format)?;
        let document: Value = serde_json::from_str(text).map_err(|_| TicketKeyError::Format)?;
        let keys = document
            .as_object()
            .and_then(|object| object.get("keys"))
            .and_then(Value::as_array)
            .ok_or(TicketKeyError::Format)?;
        if keys.is_empty() || keys.len() > MAX_KEYS {
            return Err(TicketKeyError::KeyCount);
        }
        let keys = keys
            .iter()
            .map(|key| {
                key.as_object()
                    .ok_or(TicketKeyError::Format)
                    .and_then(parse_key)
            })
            .collect::<Result<Vec<_>, _>>()?;
        TicketKeyList::new(keys)
    }
}

fn parse_key(key: &Map<String, Value>) -> Result<TicketKey, TicketKeyError> {
    if key.contains_key("d") {
        return Err(TicketKeyError::PrivateKey);
    }
    if member_text(key, "kty") != Some("EC") || member_text(key, "crv") != Some("P-256") {
        return Err(TicketKeyError::KeyType);
    }
    if key
        .get("use")
        .is_some_and(|value| value.as_str() != Some("sig"))
    {
        return Err(TicketKeyError::KeyUse);
    }
    if key
        .get("alg")
        .is_some_and(|value| value.as_str() != Some("ES256"))
    {
        return Err(TicketKeyError::Algorithm);
    }
    let kid = member_text(key, "kid").ok_or(TicketKeyError::KeyId)?;
    let x = coordinate(key, "x")?;
    let y = coordinate(key, "y")?;
    let key = TicketKey::new(kid.to_owned(), x, y)?;
    // Parsing the uncompressed point checks that it lies on P-256.
    ParsedPublicKey::new(&ECDSA_P256_SHA256_FIXED, key.uncompressed_point())
        .map_err(|_| TicketKeyError::NotOnCurve)?;
    Ok(key)
}

fn member_text<'a>(key: &'a Map<String, Value>, name: &str) -> Option<&'a str> {
    key.get(name).and_then(Value::as_str)
}

/// One 32-byte coordinate in unpadded base64url.
fn coordinate(key: &Map<String, Value>, name: &str) -> Result<[u8; 32], TicketKeyError> {
    member_text(key, name)
        .and_then(decode_base64url)
        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        .ok_or(TicketKeyError::Coordinates)
}

#[cfg(test)]
mod tests {
    use aws_lc_rs::rand::SystemRandom;
    use aws_lc_rs::signature::{ECDSA_P256_SHA256_FIXED_SIGNING, EcdsaKeyPair, KeyPair};

    use super::*;

    /// Unpadded base64url, written here so the test does not share the decoder.
    fn encode(bytes: &[u8]) -> String {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut text = String::new();
        for chunk in bytes.chunks(3) {
            let mut block = [0u8; 3];
            block[..chunk.len()].copy_from_slice(chunk);
            let value = u32::from(block[0]) << 16 | u32::from(block[1]) << 8 | u32::from(block[2]);
            for index in 0..=chunk.len() {
                let shift = 18 - 6 * index;
                text.push(char::from(TABLE[(value >> shift & 0x3f) as usize]));
            }
        }
        text
    }

    fn point() -> (String, String) {
        let rng = SystemRandom::new();
        let document = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng)
            .unwrap_or_else(|error| panic!("generate key: {error:?}"));
        let pair = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, document.as_ref())
            .unwrap_or_else(|error| panic!("load key: {error:?}"));
        let public = pair.public_key().as_ref();
        (encode(&public[1..33]), encode(&public[33..65]))
    }

    fn key_json(kid: &str, extra: &str) -> String {
        let (x, y) = point();
        format!(r#"{{"kty":"EC","crv":"P-256","kid":"{kid}","x":"{x}","y":"{y}"{extra}}}"#)
    }

    fn parse(text: &str) -> Result<TicketKeyList, TicketKeyError> {
        TicketJwksStore.parse(text.as_bytes())
    }

    #[test]
    fn accepts_p256_keys_and_ignores_other_members() {
        let text = format!(
            r#"{{"keys":[{},{}],"note":"ignored"}}"#,
            key_json("k1", r#","use":"sig","alg":"ES256","x5t":"ignored""#),
            key_json("k2", ""),
        );
        let keys = parse(&text).unwrap_or_else(|error| panic!("valid JWKS: {error:?}"));
        assert_eq!(keys.len(), 2);
        assert!(keys.find("k1").is_some());
        assert!(keys.find("k2").is_some());
    }

    #[test]
    fn rejects_each_broken_rule_with_a_fixed_cause() {
        let (x, y) = point();
        let cases = [
            ("[]".to_owned(), TicketKeyError::Format),
            (r#"{"keys":{}}"#.to_owned(), TicketKeyError::Format),
            (r#"{"keys":[]}"#.to_owned(), TicketKeyError::KeyCount),
            (
                format!(r#"{{"keys":[{}]}}"#, key_json("k", r#","d":"secret""#)),
                TicketKeyError::PrivateKey,
            ),
            (
                format!(
                    r#"{{"keys":[{{"kty":"RSA","crv":"P-256","kid":"k","x":"{x}","y":"{y}"}}]}}"#
                ),
                TicketKeyError::KeyType,
            ),
            (
                format!(
                    r#"{{"keys":[{{"kty":"EC","crv":"P-384","kid":"k","x":"{x}","y":"{y}"}}]}}"#
                ),
                TicketKeyError::KeyType,
            ),
            (
                format!(r#"{{"keys":[{}]}}"#, key_json("k", r#","use":"enc""#)),
                TicketKeyError::KeyUse,
            ),
            (
                format!(r#"{{"keys":[{}]}}"#, key_json("k", r#","alg":"ES384""#)),
                TicketKeyError::Algorithm,
            ),
            (
                format!(r#"{{"keys":[{}]}}"#, key_json("", "")),
                TicketKeyError::KeyId,
            ),
            (
                format!(r#"{{"keys":[{}]}}"#, key_json(&"k".repeat(129), "")),
                TicketKeyError::KeyId,
            ),
            (
                format!(
                    r#"{{"keys":[{{"kty":"EC","crv":"P-256","kid":"k","x":"{x}=","y":"{y}"}}]}}"#
                ),
                TicketKeyError::Coordinates,
            ),
            (
                format!(
                    r#"{{"keys":[{{"kty":"EC","crv":"P-256","kid":"k","x":"{}","y":"{y}"}}]}}"#,
                    encode(&[1u8; 31])
                ),
                TicketKeyError::Coordinates,
            ),
            (
                format!(
                    r#"{{"keys":[{{"kty":"EC","crv":"P-256","kid":"k","x":"{x}","y":"{}"}}]}}"#,
                    encode(&[1u8; 32])
                ),
                TicketKeyError::NotOnCurve,
            ),
            (
                format!(
                    r#"{{"keys":[{},{}]}}"#,
                    key_json("k", ""),
                    key_json("k", "")
                ),
                TicketKeyError::DuplicateKeyId,
            ),
        ];
        for (text, expected) in cases {
            assert_eq!(parse(&text).err(), Some(expected), "{text}");
        }
    }

    #[test]
    fn limits_the_file_size_and_key_count() {
        let seventeen = (0..17)
            .map(|index| key_json(&format!("k{index}"), ""))
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(
            parse(&format!(r#"{{"keys":[{seventeen}]}}"#)).err(),
            Some(TicketKeyError::KeyCount)
        );
        let padding = " ".repeat(MAX_FILE_BYTES);
        assert_eq!(
            parse(&format!(r#"{{"keys":[{}]}}{padding}"#, key_json("k", ""))).err(),
            Some(TicketKeyError::TooLarge)
        );
        assert_eq!(
            TicketJwksStore.parse(b"\xff").err(),
            Some(TicketKeyError::Format)
        );
    }
}
