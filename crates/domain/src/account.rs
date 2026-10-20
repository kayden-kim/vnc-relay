//! Administrator account values: the account name rule, the password rule
//! (cli-v1 §1.2), and the Argon2id hash string format (cli-v1 §1.4).

use std::fmt;
use std::ops::RangeInclusive;

/// Account name: 1 to 32 characters from `a-z`, `0-9`, `.`, `_`, `-`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AccountName(String);

/// The text breaks the account name rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountNameError;

impl fmt::Display for AccountNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("계정 이름은 소문자 영문·숫자·점·밑줄·하이픈 1~32자여야 한다")
    }
}

impl std::error::Error for AccountNameError {}

impl AccountName {
    pub const MAX_LEN: usize = 32;

    pub fn parse(text: &str) -> Result<Self, AccountNameError> {
        let allowed = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b);
        if (1..=Self::MAX_LEN).contains(&text.len()) && text.bytes().all(allowed) {
            Ok(Self(text.to_owned()))
        } else {
            Err(AccountNameError)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AccountName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A password that keeps the rule: 12 to 128 characters, at most 512 bytes
/// in UTF-8, no control character. The bytes are kept as typed, without
/// normalization. `Debug` hides the value.
#[derive(Clone, PartialEq, Eq)]
pub struct Password(String);

/// Which part of the password rule the text breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordError {
    /// Fewer than 12 characters.
    TooShort,
    /// More than 128 characters or more than 512 bytes.
    TooLong,
    /// A control character such as a line break or a tab.
    ControlCharacter,
}

impl fmt::Display for PasswordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::TooShort => "비밀번호가 12자보다 짧다",
            Self::TooLong => "비밀번호가 128자 또는 UTF-8 512바이트보다 길다",
            Self::ControlCharacter => "비밀번호에 제어 문자가 있다",
        })
    }
}

impl std::error::Error for PasswordError {}

impl Password {
    pub const MIN_CHARS: usize = 12;
    pub const MAX_CHARS: usize = 128;
    pub const MAX_BYTES: usize = 512;

    pub fn parse(text: String) -> Result<Self, PasswordError> {
        let chars = text.chars().count();
        if text.chars().any(char::is_control) {
            Err(PasswordError::ControlCharacter)
        } else if chars < Self::MIN_CHARS {
            Err(PasswordError::TooShort)
        } else if chars > Self::MAX_CHARS || text.len() > Self::MAX_BYTES {
            Err(PasswordError::TooLong)
        } else {
            Ok(Self(text))
        }
    }

    /// The typed bytes, for hashing. Never write them to output or logs.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl fmt::Debug for Password {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Password(<redacted>)")
    }
}

/// Argon2id cost parameters of a hash: memory (KiB), iterations, parallelism.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Argon2Params {
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

/// Parameters outside the approved ranges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Argon2ParamsError {
    /// At least one parameter is below its minimum. Takes precedence.
    BelowMinimum,
    AboveMaximum,
}

impl fmt::Display for Argon2ParamsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::BelowMinimum => "승인된 하한보다 작다",
            Self::AboveMaximum => "승인된 상한보다 크다",
        })
    }
}

impl std::error::Error for Argon2ParamsError {}

impl Argon2Params {
    /// Approved ranges, shared with `[admin.argon2]` (config-v1 §1.2).
    pub const MEMORY_KIB: RangeInclusive<u32> = 19_456..=1_048_576;
    pub const ITERATIONS: RangeInclusive<u32> = 2..=10;
    pub const PARALLELISM: RangeInclusive<u32> = 1..=16;

    pub fn check_range(&self) -> Result<(), Argon2ParamsError> {
        let pairs = [
            (self.memory_kib, Self::MEMORY_KIB),
            (self.iterations, Self::ITERATIONS),
            (self.parallelism, Self::PARALLELISM),
        ];
        if pairs.iter().any(|(value, range)| value < range.start()) {
            Err(Argon2ParamsError::BelowMinimum)
        } else if pairs.iter().any(|(value, range)| value > range.end()) {
            Err(Argon2ParamsError::AboveMaximum)
        } else {
            Ok(())
        }
    }
}

/// An Argon2id hash in PHC string form:
/// `$argon2id$v=19$m=<m>,t=<t>,p=<p>$<salt>$<hash>`.
///
/// Parsing checks the form only; the parameter ranges are checked by
/// [`Argon2Params::check_range`]. `Debug` hides the hash string.
#[derive(Clone, PartialEq, Eq)]
pub struct PasswordHash {
    encoded: String,
    params: Argon2Params,
}

/// Which part of the PHC string is wrong. `Display` names the part and never
/// quotes the string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordHashError {
    /// Not six `$`-separated parts starting with an empty part.
    Format,
    /// Not `argon2id`.
    Algorithm,
    /// Not `v=19`.
    Version,
    /// Not `m=<decimal>,t=<decimal>,p=<decimal>` in this order.
    Params,
    /// Not unpadded standard Base64 of at least 16 bytes.
    Salt,
    /// Not unpadded standard Base64 of exactly 32 bytes.
    Hash,
}

impl fmt::Display for PasswordHashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Format => "`$argon2id$v=19$m=…,t=…,p=…$<솔트>$<해시>` 형식이 아니다",
            Self::Algorithm => "알고리즘이 `argon2id`가 아니다",
            Self::Version => "버전이 `v=19`가 아니다",
            Self::Params => "매개변수가 `m=<수>,t=<수>,p=<수>` 순서의 십진수가 아니다",
            Self::Salt => "솔트가 16바이트 이상의 패딩 없는 표준 Base64가 아니다",
            Self::Hash => "해시가 32바이트의 패딩 없는 표준 Base64가 아니다",
        })
    }
}

impl std::error::Error for PasswordHashError {}

impl PasswordHash {
    pub const MIN_SALT_LEN: usize = 16;
    pub const HASH_LEN: usize = 32;

    pub fn parse(text: &str) -> Result<Self, PasswordHashError> {
        let parts: Vec<&str> = text.split('$').collect();
        let [empty, algorithm, version, params, salt, hash] = parts.as_slice() else {
            return Err(PasswordHashError::Format);
        };
        if !empty.is_empty() {
            return Err(PasswordHashError::Format);
        }
        if *algorithm != "argon2id" {
            return Err(PasswordHashError::Algorithm);
        }
        if *version != "v=19" {
            return Err(PasswordHashError::Version);
        }
        let params = parse_params(params).ok_or(PasswordHashError::Params)?;
        if !base64_decoded_len(salt).is_some_and(|len| len >= Self::MIN_SALT_LEN) {
            return Err(PasswordHashError::Salt);
        }
        if base64_decoded_len(hash) != Some(Self::HASH_LEN) {
            return Err(PasswordHashError::Hash);
        }
        Ok(Self {
            encoded: text.to_owned(),
            params,
        })
    }

    pub fn params(&self) -> Argon2Params {
        self.params
    }

    /// The PHC string, for password verification. Never write it to output or logs.
    pub fn as_str(&self) -> &str {
        &self.encoded
    }
}

impl fmt::Debug for PasswordHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PasswordHash")
            .field("encoded", &"<redacted>")
            .field("params", &self.params)
            .finish()
    }
}

/// One administrator account from the account file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub name: AccountName,
    pub password_hash: PasswordHash,
}

fn parse_params(text: &str) -> Option<Argon2Params> {
    let mut fields = text.split(',');
    let params = Argon2Params {
        memory_kib: decimal_field(fields.next()?, "m=")?,
        iterations: decimal_field(fields.next()?, "t=")?,
        parallelism: decimal_field(fields.next()?, "p=")?,
    };
    fields.next().is_none().then_some(params)
}

fn decimal_field(field: &str, prefix: &str) -> Option<u32> {
    let value = field.strip_prefix(prefix)?;
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

/// Decoded length of unpadded standard Base64 text, or `None` when the text
/// has other characters, padding, an impossible length, or non-zero unused
/// bits in the last character.
fn base64_decoded_len(text: &str) -> Option<usize> {
    let mut last = 0u8;
    for b in text.bytes() {
        last = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
    }
    let remainder = text.len() % 4;
    let unused_bits = match remainder {
        0 => 0,
        2 => 0x0f,
        3 => 0x03,
        _ => return None,
    };
    (last & unused_bits == 0).then_some(text.len() / 4 * 3 + remainder.saturating_sub(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 16 zero bytes and 32 zero bytes in unpadded Base64. Test data only.
    const SALT: &str = "AAAAAAAAAAAAAAAAAAAAAA";
    const HASH: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

    fn phc(params: &str, salt: &str, hash: &str) -> String {
        format!("$argon2id$v=19${params}${salt}${hash}")
    }

    #[test]
    fn accepts_names_inside_the_rule_only() {
        let longest = "a".repeat(32);
        for name in ["a", "alice", "ops.team_1-x", longest.as_str()] {
            assert_eq!(
                AccountName::parse(name).map(|n| n.to_string()),
                Ok(name.to_owned())
            );
        }
        let too_long = "a".repeat(33);
        for name in [
            "",
            too_long.as_str(),
            "Alice",
            "al ice",
            "앨리스",
            "a/b",
            "a@b",
        ] {
            assert_eq!(AccountName::parse(name), Err(AccountNameError), "{name}");
        }
    }

    #[test]
    fn parses_phc_strings_and_keeps_parameters() {
        let text = phc("m=19456,t=2,p=1", SALT, HASH);
        let hash = PasswordHash::parse(&text).unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(
            hash.params(),
            Argon2Params {
                memory_kib: 19_456,
                iterations: 2,
                parallelism: 1
            }
        );
        assert_eq!(hash.as_str(), text);
        // A longer salt is accepted on read.
        let long_salt = format!("{SALT}AAAA");
        assert!(PasswordHash::parse(&phc("m=65536,t=3,p=4", &long_salt, HASH)).is_ok());
    }

    #[test]
    fn rejects_each_wrong_part_with_its_own_cause() {
        use PasswordHashError::*;
        let cases = [
            ("argon2id$v=19$m=19456,t=2,p=1$S$H".to_owned(), Format),
            (
                format!("$argon2id$v=19$m=19456,t=2,p=1${SALT}${HASH}$x"),
                Format,
            ),
            (
                format!("$argon2i$v=19$m=19456,t=2,p=1${SALT}${HASH}"),
                Algorithm,
            ),
            (
                format!("$argon2id$v=16$m=19456,t=2,p=1${SALT}${HASH}"),
                Version,
            ),
            (phc("t=2,m=19456,p=1", SALT, HASH), Params),
            (phc("m=19456,t=2", SALT, HASH), Params),
            (phc("m=19456,t=2,p=1,keyid=a", SALT, HASH), Params),
            (phc("m=19456,t=-2,p=1", SALT, HASH), Params),
            (phc("m=,t=2,p=1", SALT, HASH), Params),
            // 15 bytes.
            (phc("m=19456,t=2,p=1", "AAAAAAAAAAAAAAAAAAAA", HASH), Salt),
            // Padding and non-zero unused bits.
            (phc("m=19456,t=2,p=1", &format!("{SALT}=="), HASH), Salt),
            (phc("m=19456,t=2,p=1", "AAAAAAAAAAAAAAAAAAAAAB", HASH), Salt),
            // 31 and 33 bytes.
            (phc("m=19456,t=2,p=1", SALT, &HASH[..42]), Hash),
            (phc("m=19456,t=2,p=1", SALT, &format!("{HASH}A")), Hash),
            (
                phc("m=19456,t=2,p=1", SALT, &format!("{}-", &HASH[..42])),
                Hash,
            ),
        ];
        for (text, cause) in cases {
            assert_eq!(PasswordHash::parse(&text).map(|_| ()), Err(cause), "{text}");
        }
    }

    #[test]
    fn checks_parameters_against_approved_ranges() {
        let params = |memory_kib, iterations, parallelism| Argon2Params {
            memory_kib,
            iterations,
            parallelism,
        };
        assert_eq!(params(19_456, 2, 1).check_range(), Ok(()));
        assert_eq!(params(1_048_576, 10, 16).check_range(), Ok(()));
        assert_eq!(
            params(19_455, 2, 1).check_range(),
            Err(Argon2ParamsError::BelowMinimum)
        );
        assert_eq!(
            params(19_456, 1, 1).check_range(),
            Err(Argon2ParamsError::BelowMinimum)
        );
        assert_eq!(
            params(19_456, 2, 0).check_range(),
            Err(Argon2ParamsError::BelowMinimum)
        );
        assert_eq!(
            params(1_048_577, 2, 1).check_range(),
            Err(Argon2ParamsError::AboveMaximum)
        );
        // Below one minimum and above another maximum reports the minimum.
        assert_eq!(
            params(19_455, 11, 1).check_range(),
            Err(Argon2ParamsError::BelowMinimum)
        );
    }

    /// spec: operations.md:OPS-030 partial
    #[test]
    fn accepts_passwords_inside_the_rule_and_names_the_broken_part() {
        let parse = |text: &str| Password::parse(text.to_owned()).map(|p| p.as_bytes().to_vec());
        let shortest = "a".repeat(12);
        // 128 characters of 4 bytes each: the character and byte limits meet.
        let longest = "😀".repeat(128);
        for text in [
            shortest.as_str(),
            "비밀번호는열두글자이상이다",
            longest.as_str(),
            "  spaces ok  ",
        ] {
            assert_eq!(parse(text), Ok(text.as_bytes().to_vec()), "{text}");
        }
        // Not normalized: the composed and decomposed forms stay different.
        assert_ne!(parse("caf\u{e9}-password"), parse("cafe\u{301}-password"));
        assert_eq!(parse(&"a".repeat(11)), Err(PasswordError::TooShort));
        assert_eq!(parse(""), Err(PasswordError::TooShort));
        assert_eq!(parse(&"a".repeat(129)), Err(PasswordError::TooLong));
        assert_eq!(parse(&"😀".repeat(129)), Err(PasswordError::TooLong));
        for text in [
            "password-with\n",
            "pass\tword-tab",
            "password\u{7f}del",
            "password\u{85}nel",
        ] {
            assert_eq!(
                parse(text),
                Err(PasswordError::ControlCharacter),
                "{text:?}"
            );
        }
        let password = Password::parse(shortest.clone()).unwrap_or_else(|e| panic!("{e}"));
        assert!(!format!("{password:?}").contains(&shortest));
    }

    #[test]
    fn debug_output_leaves_out_the_hash_string() {
        let text = phc("m=19456,t=2,p=1", SALT, HASH);
        let hash = PasswordHash::parse(&text).unwrap_or_else(|e| panic!("{e:?}"));
        assert!(!format!("{hash:?}").contains(HASH));
    }
}
