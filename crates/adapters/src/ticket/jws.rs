//! Strict ES256 compact JWS verification (websocket-relay-v2 §1.9.2–1.9.3).

use std::collections::HashSet;
use std::fmt;

use aws_lc_rs::signature::{ECDSA_P256_SHA256_FIXED, UnparsedPublicKey};
use serde_json::{Map, Value};
use vnc_relay_domain::relay_id::RelayId;
use vnc_relay_domain::ticket::{
    TicketClaims, TicketError, TicketKeyList, TicketPolicy, decode_base64url, id_is_valid,
    split_compact,
};

/// Only an authenticated, syntactically valid jti may accompany a refusal.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct TicketVerificationError {
    pub(crate) reason: TicketError,
    pub(crate) jti: Option<String>,
}

impl TicketVerificationError {
    pub(crate) fn code(&self) -> &'static str {
        self.reason.code()
    }
}

impl fmt::Display for TicketVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for TicketVerificationError {}

pub(crate) fn verify(
    compact: &str,
    keys: &TicketKeyList,
    policy: TicketPolicy<'_>,
    now_secs: u64,
) -> Result<TicketClaims, TicketVerificationError> {
    let invalid = || TicketVerificationError {
        reason: TicketError::Invalid,
        jti: None,
    };
    let parts = split_compact(compact).ok_or_else(invalid)?;
    let header = decode_base64url(parts.header).ok_or_else(invalid)?;
    let payload = decode_base64url(parts.payload).ok_or_else(invalid)?;
    let signature = decode_base64url(parts.signature).ok_or_else(invalid)?;
    let header = object(&header).ok_or_else(invalid)?;
    if text(&header, "alg") != Some("ES256") || header.contains_key("crit") {
        return Err(invalid());
    }
    let kid = text(&header, "kid")
        .filter(|kid| id_is_valid(kid))
        .ok_or_else(invalid)?;
    let key = keys.find(kid).ok_or_else(invalid)?;
    if signature.len() != 64 {
        return Err(invalid());
    }
    UnparsedPublicKey::new(&ECDSA_P256_SHA256_FIXED, key.uncompressed_point())
        .verify(parts.signing_input.as_bytes(), &signature)
        .map_err(|_| invalid())?;

    let payload = object(&payload).ok_or_else(invalid)?;
    let jti = text(&payload, "jti").filter(|jti| id_is_valid(jti));
    let rejected = |reason| TicketVerificationError {
        reason,
        jti: jti.map(str::to_owned),
    };
    let claims = claims(&payload).ok_or_else(|| rejected(TicketError::Invalid))?;
    policy.check(&claims, now_secs).map_err(rejected)?;
    Ok(claims)
}

fn text<'a>(object: &'a Map<String, Value>, name: &str) -> Option<&'a str> {
    object.get(name).and_then(Value::as_str)
}

fn claims(object: &Map<String, Value>) -> Option<TicketClaims> {
    let jti = text(object, "jti").filter(|jti| id_is_valid(jti))?;
    let sub = match object.get("sub") {
        None => None,
        Some(value) => Some(value.as_str().filter(|sub| id_is_valid(sub))?.to_owned()),
    };
    let sx = match object.get("sx") {
        None => None,
        Some(value) => Some(value.as_u64()?),
    };
    Some(TicketClaims {
        iss: text(object, "iss")?.to_owned(),
        aud: text(object, "aud")?.to_owned(),
        rh: text(object, "rh")?.to_owned(),
        rid: RelayId::parse(text(object, "rid")?).ok()?,
        jti: jti.to_owned(),
        iat: object.get("iat")?.as_u64()?,
        exp: object.get("exp")?.as_u64()?,
        sx,
        sub,
    })
}

fn object(bytes: &[u8]) -> Option<Map<String, Value>> {
    let value: Value = serde_json::from_slice(bytes).ok()?;
    let Value::Object(object) = value else {
        return None;
    };
    if duplicate_members(bytes) {
        return None;
    }
    Some(object)
}

/// JSON syntax was already checked by serde_json. Scan only structural tokens
/// and decode key strings so escaped spellings cannot hide duplicate members.
fn duplicate_members(bytes: &[u8]) -> bool {
    let mut stack: Vec<Option<HashSet<String>>> = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'{' => stack.push(Some(HashSet::new())),
            b'[' => stack.push(None),
            b'}' | b']' => {
                stack.pop();
            }
            b'"' => {
                let start = at;
                at += 1;
                while at < bytes.len() {
                    match bytes[at] {
                        b'\\' => at += 2,
                        b'"' => break,
                        _ => at += 1,
                    }
                }
                let end = at + 1;
                let mut next = end;
                while next < bytes.len() && bytes[next].is_ascii_whitespace() {
                    next += 1;
                }
                if bytes.get(next) == Some(&b':') {
                    let Some(Some(keys)) = stack.last_mut() else {
                        return true;
                    };
                    let Some(token) = bytes
                        .get(start..end)
                        .and_then(|key| std::str::from_utf8(key).ok())
                    else {
                        return true;
                    };
                    let Ok(key) = serde_json::from_str::<String>(token) else {
                        return true;
                    };
                    if !keys.insert(key) {
                        return true;
                    }
                }
            }
            _ => {}
        }
        at += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_lc_rs::rand::SystemRandom;
    use aws_lc_rs::signature::{ECDSA_P256_SHA256_FIXED_SIGNING, EcdsaKeyPair, KeyPair};
    use serde_json::json;
    use vnc_relay_domain::entry_point::Role;
    use vnc_relay_domain::ticket::TicketKey;

    fn encode(bytes: &[u8]) -> String {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut encoded = String::new();
        let mut bits = 0_u32;
        let mut count = 0;
        for byte in bytes {
            bits = (bits << 8) | u32::from(*byte);
            count += 8;
            while count >= 6 {
                count -= 6;
                encoded.push(ALPHABET[((bits >> count) & 63) as usize] as char);
            }
        }
        if count != 0 {
            encoded.push(ALPHABET[((bits << (6 - count)) & 63) as usize] as char);
        }
        encoded
    }

    fn signer() -> EcdsaKeyPair {
        let rng = SystemRandom::new();
        let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
        EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8.as_ref()).unwrap()
    }

    fn keys(signer: &EcdsaKeyPair) -> TicketKeyList {
        let point = signer.public_key().as_ref();
        TicketKeyList::new(vec![
            TicketKey::new(
                "key".into(),
                point[1..33].try_into().unwrap(),
                point[33..65].try_into().unwrap(),
            )
            .unwrap(),
        ])
        .unwrap()
    }

    fn sign(signer: &EcdsaKeyPair, header: &str, payload: &str) -> String {
        let input = format!(
            "{}.{}",
            encode(header.as_bytes()),
            encode(payload.as_bytes())
        );
        let signature = signer.sign(&SystemRandom::new(), input.as_bytes()).unwrap();
        format!("{input}.{}", encode(signature.as_ref()))
    }

    fn payload() -> Value {
        json!({"iss":"issuer.example", "aud":"vnc-relay/viewer", "rh":"relay.example", "rid":"0123", "jti":"id", "iat":100, "exp":160})
    }

    fn policy() -> TicketPolicy<'static> {
        TicketPolicy {
            role: Role::Viewer,
            issuer: "issuer.example",
            relay_host: "relay.example",
            max_lifetime_secs: 60,
            started_at_secs: 90,
        }
    }

    #[test]
    fn verifies_fixed_signature_and_ignores_unknown_members() {
        let signer = signer();
        let mut payload = payload();
        payload["unknown"] = json!({"text":"escaped \\\" and : { }", "array":[{"x":1},{"x":2}]});
        let ticket = sign(
            &signer,
            r#"{"alg":"ES256","kid":"key","typ":true,"jku":"ignored"}"#,
            &payload.to_string(),
        );
        let claims = verify(&ticket, &keys(&signer), policy(), 100).unwrap();
        assert_eq!(claims.rid.as_str(), "0123");
        assert_eq!(claims.jti, "id");
    }

    #[test]
    fn rejects_duplicate_decoded_keys_in_every_object() {
        for json in [
            r#"{"jti":"a","\u006ati":"b"}"#,
            r#"{"x":{"a":1,"a":2}}"#,
            r#"{"x":[{"a":1,"\u0061":2}]}"#,
        ] {
            assert!(object(json.as_bytes()).is_none());
        }
        assert!(object(br#"{"x":[{"a":1},{"a":2}],"text":"\\\""}"#).is_some());
    }

    #[test]
    fn authenticates_before_exposing_jti_and_rejects_strict_claim_types() {
        let signer = signer();
        let other = self::signer();
        let keys = keys(&signer);
        let header = r#"{"alg":"ES256","kid":"key"}"#;
        let ticket = sign(&other, header, &payload().to_string());
        assert_eq!(verify(&ticket, &keys, policy(), 100).unwrap_err().jti, None);
        for (name, value) in [
            ("iat", json!(100.0)),
            ("iat", json!(-1)),
            ("exp", json!("160")),
            ("sx", Value::Null),
            ("aud", json!(["vnc-relay/viewer"])),
            ("rid", json!("12x")),
            ("sub", json!("")),
            ("iss", Value::Null),
        ] {
            let mut payload = payload();
            payload[name] = value;
            let ticket = sign(&signer, header, &payload.to_string());
            let error = verify(&ticket, &keys, policy(), 100).unwrap_err();
            assert_eq!(error.reason, TicketError::Invalid, "{name}");
            assert_eq!(error.jti.as_deref(), Some("id"));
        }
    }

    #[test]
    fn accepts_u64_integer_claims_and_rejects_exponents_and_larger_values() {
        let signer = signer();
        let keys = keys(&signer);
        let header = r#"{"alg":"ES256","kid":"key"}"#;
        let claims = r#""iss":"issuer.example","aud":"vnc-relay/viewer","rh":"relay.example","rid":"0123","jti":"id","iat":100,"exp":160"#;
        let ticket = sign(
            &signer,
            header,
            &format!(r#"{{{claims},"sx":18446744073709551615}}"#),
        );
        assert_eq!(
            verify(&ticket, &keys, policy(), 100).unwrap().sx,
            Some(u64::MAX)
        );
        for sx in ["18446744073709551616", "1e3", "1000.0"] {
            let ticket = sign(&signer, header, &format!(r#"{{{claims},"sx":{sx}}}"#));
            let error = verify(&ticket, &keys, policy(), 100).unwrap_err();
            assert_eq!(error.reason, TicketError::Invalid, "{sx}");
            assert_eq!(error.jti.as_deref(), Some("id"), "{sx}");
        }
    }

    #[test]
    fn rejects_header_and_compact_forms_before_claims() {
        let signer = signer();
        let keys = keys(&signer);
        for header in [
            r#"{"alg":"none","kid":"key"}"#,
            r#"{"alg":"ES256"}"#,
            r#"{"alg":"ES256","kid":"unknown"}"#,
            r#"{"alg":"ES256","kid":"key","crit":null}"#,
            r#"{"alg":"ES256","kid":"key","\u006bid":"key"}"#,
            "[]",
        ] {
            let ticket = sign(&signer, header, &payload().to_string());
            assert_eq!(verify(&ticket, &keys, policy(), 100).unwrap_err().jti, None);
        }
        for compact in [
            "",
            "a.b",
            "a.b.c.d",
            "e30=.e30.AA",
            "e31.e30.AA",
            "e30.e30.MA",
        ] {
            assert_eq!(
                verify(compact, &keys, policy(), 100).unwrap_err().reason,
                TicketError::Invalid
            );
        }
    }
}
