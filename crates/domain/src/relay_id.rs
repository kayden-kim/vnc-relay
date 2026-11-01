//! Exact relay identifiers from repeater-handshake-v1 §1.2.

use std::fmt;

/// An owned ASCII digit string compared and hashed without normalization.
/// Leading zeroes remain significant (repeater-handshake-v1 §1.2).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RelayId(String);

/// An identifier violates the digit or length rule (repeater-handshake-v1 §1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayIdError {
    /// The value is not 1 to 247 ASCII digits.
    Invalid,
}

impl RelayId {
    /// Maximum ID length within the 250-byte request (repeater-handshake-v1 §1.2).
    pub const MAX_LEN: usize = 247;

    /// Validates and owns 1 to 247 ASCII digits without numeric conversion.
    /// See repeater-handshake-v1 §1.2.
    pub fn parse(text: &str) -> Result<Self, RelayIdError> {
        if text.is_empty()
            || text.len() > Self::MAX_LEN
            || !text.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(RelayIdError::Invalid);
        }
        Ok(Self(text.to_owned()))
    }

    /// Returns the original digits, including leading zeroes.
    /// See repeater-handshake-v1 §1.2.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for RelayId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RelayId(<redacted>)")
    }
}

impl RelayIdError {
    /// Contract error for an invalid ID (repeater-handshake-v1 §2).
    pub fn code(self) -> &'static str {
        match self {
            Self::Invalid => "handshake_invalid",
        }
    }
}

impl fmt::Display for RelayIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("relay ID must contain 1 to 247 ASCII digits")
    }
}

impl std::error::Error for RelayIdError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn invalid(text: &str) -> RelayIdError {
        match RelayId::parse(text) {
            Ok(id) => panic!("unexpected ID: {id:?}"),
            Err(error) => error,
        }
    }

    #[test]
    fn keeps_leading_zeroes_and_accepts_one_to_247_digits() {
        for text in ["0", "1", "0123", "123", &"9".repeat(RelayId::MAX_LEN)] {
            let id = RelayId::parse(text).unwrap_or_else(|error| panic!("{error}"));
            assert_eq!(id.as_str(), text);
            assert_eq!(RelayId::parse(text), Ok(id));
        }

        let zero_prefixed = RelayId::parse("0123").unwrap_or_else(|error| panic!("{error}"));
        let unprefixed = RelayId::parse("123").unwrap_or_else(|error| panic!("{error}"));
        assert_ne!(zero_prefixed, unprefixed);
        let mut ids = HashSet::new();
        assert!(ids.insert(zero_prefixed.clone()));
        assert!(!ids.insert(zero_prefixed));
        assert!(ids.insert(unprefixed));
        assert_eq!(ids.len(), 2);

        for text in [
            "",
            " ",
            "12 34",
            "123 ",
            " 123",
            "+123",
            "-123",
            "12-34",
            "12ab",
            "１２３",
            "١٢٣",
            "1\0",
            &"9".repeat(RelayId::MAX_LEN + 1),
        ] {
            assert_eq!(invalid(text).code(), "handshake_invalid", "{text:?}");
        }
    }

    #[test]
    fn hides_the_original_id_in_debug_and_errors() {
        let id = RelayId::parse("0123456789").unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(format!("{id:?}"), "RelayId(<redacted>)");
        let error = invalid("0123456789x");
        assert!(!error.to_string().contains("0123456789"));
    }
}
