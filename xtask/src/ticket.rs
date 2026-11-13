//! `cargo xtask ticket`: test keys and access tickets for checking a relay by
//! hand (docs/architectures/xtask.md, docs/how-to/operate-relay.md section 3.10.1).
//!
//! The product only verifies tickets that an external issuer signed. Test
//! issuing stays in this development tool, outside the executable and the
//! release bundle (user decision 2026-11-13 (Q233)). Tickets follow
//! websocket-relay-v2 section 1.9.2. Public key files follow config-v4 section 1.11.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use aws_lc_rs::digest::{SHA256, digest};
use aws_lc_rs::encoding::AsBigEndian;
use aws_lc_rs::rand::{SystemRandom, fill};
use aws_lc_rs::signature::{ECDSA_P256_SHA256_FIXED_SIGNING, EcdsaKeyPair, KeyPair};
use serde_json::{Map, Value};

use crate::{Error, cargo};

/// Key directory under the Cargo target directory when `--dir` is not given.
const DEFAULT_DIR: &str = "ticket";
/// Issuer and relay host of the commented sample in examples/relay.toml.
const DEFAULT_ISSUER: &str = "issuer.example/prod";
const DEFAULT_RELAY_HOST: &str = "relay.example";
/// Default `max_lifetime_secs` (config-v4 section 1.2).
const DEFAULT_LIFETIME_SECS: u64 = 60;
/// Highest `max_lifetime_secs` a configuration accepts (config-v4 section 1.2).
const MAX_LIFETIME_SECS: u64 = 600;
/// Byte limit of `issuer` and `relay_host` (config-v4 section 1.2).
const MAX_CONFIG_TEXT: usize = 256;
/// Byte limit of `sub` and `kid` (websocket-relay-v2 section 1.9.2).
const MAX_CLAIM_TEXT: usize = 128;
/// Relay ID digits (repeater-handshake-v1 section 1.2).
const MAX_RELAY_ID_DIGITS: usize = 247;
/// `jti` needs at least 128 random bits (websocket-relay-v2 section 1.9.7).
const JTI_BYTES: usize = 16;
/// Bytes in a P-256 coordinate and in a private scalar.
const COORDINATE_BYTES: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Viewer,
    Server,
}

impl Role {
    fn parse(text: &str) -> Result<Self, String> {
        match text {
            "viewer" => Ok(Self::Viewer),
            "server" => Ok(Self::Server),
            _ => Err(format!("역할은 viewer 또는 server다: {text}")),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Viewer => "viewer",
            Self::Server => "server",
        }
    }

    /// Fixed `aud` of the role (websocket-relay-v2 section 1.9.2).
    fn audience(self) -> &'static str {
        match self {
            Self::Viewer => "vnc-relay/viewer",
            Self::Server => "vnc-relay/server",
        }
    }

    fn public_file(self) -> String {
        format!("{}-ticket.jwks", self.name())
    }

    fn private_file(self) -> String {
        format!("{}-ticket.private.jwk", self.name())
    }
}

/// Claims from the command line. `iat`, `exp`, `sx`, and `jti` are set when signing.
#[derive(Debug, PartialEq, Eq)]
pub struct Claims {
    relay_id: String,
    issuer: String,
    relay_host: String,
    lifetime_secs: u64,
    session_secs: Option<u64>,
    subject: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Keys {
        role: Role,
        dir: Option<PathBuf>,
    },
    Issue {
        role: Role,
        dir: Option<PathBuf>,
        claims: Claims,
    },
}

/// Reads the arguments after `ticket`. An error explains the usage problem.
pub fn parse(args: &[&str]) -> Result<Command, String> {
    match args {
        ["keys", role, rest @ ..] => {
            let role = Role::parse(role)?;
            let options = options(rest, &["--dir"])?;
            Ok(Command::Keys {
                role,
                dir: options.get("--dir").map(|dir| PathBuf::from(*dir)),
            })
        }
        ["issue", role, relay_id, rest @ ..] => {
            let role = Role::parse(role)?;
            let options = options(
                rest,
                &[
                    "--dir",
                    "--issuer",
                    "--relay-host",
                    "--lifetime",
                    "--session",
                    "--subject",
                ],
            )?;
            let claims = Claims {
                relay_id: relay_id_argument(relay_id)?,
                issuer: text(
                    options.get("--issuer").copied().unwrap_or(DEFAULT_ISSUER),
                    "--issuer",
                    MAX_CONFIG_TEXT,
                )?,
                relay_host: text(
                    options
                        .get("--relay-host")
                        .copied()
                        .unwrap_or(DEFAULT_RELAY_HOST),
                    "--relay-host",
                    MAX_CONFIG_TEXT,
                )?,
                lifetime_secs: options
                    .get("--lifetime")
                    .map_or(Ok(DEFAULT_LIFETIME_SECS), |value| {
                        seconds(value, "--lifetime", Some(MAX_LIFETIME_SECS))
                    })?,
                session_secs: options
                    .get("--session")
                    .map(|value| seconds(value, "--session", None))
                    .transpose()?,
                subject: options
                    .get("--subject")
                    .map(|value| text(value, "--subject", MAX_CLAIM_TEXT))
                    .transpose()?,
            };
            Ok(Command::Issue {
                role,
                dir: options.get("--dir").map(|dir| PathBuf::from(*dir)),
                claims,
            })
        }
        _ => Err("ticket 다음에는 keys <역할> 또는 issue <역할> <중계 ID>가 와야 한다".to_owned()),
    }
}

/// Runs a parsed command. Errors leave existing key files unchanged.
pub fn run(command: Command) -> Result<bool, Error> {
    let mut stdout = std::io::stdout().lock();
    match command {
        Command::Keys { role, dir } => {
            let files = write_keys(&key_dir(dir)?, role)?;
            writeln!(stdout, "공개 키: {}", files.public.display())?;
            writeln!(stdout, "개인키: {}", files.private.display())?;
            writeln!(stdout, "kid: {}", files.kid)?;
        }
        Command::Issue { role, dir, claims } => {
            let key = load_key(&key_dir(dir)?.join(role.private_file()))?;
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| Error::new("시스템 시각이 1970-01-01 이전이다"))?
                .as_secs();
            writeln!(stdout, "{}", issue(&key, role, &claims, now)?)?;
        }
    }
    Ok(true)
}

/// `--dir`, or `ticket/` under the Cargo target directory.
fn key_dir(dir: Option<PathBuf>) -> Result<PathBuf, Error> {
    match dir {
        Some(dir) => Ok(dir),
        None => Ok(cargo::metadata()?.target_dir.join(DEFAULT_DIR)),
    }
}

struct KeyFiles {
    public: PathBuf,
    private: PathBuf,
    kid: String,
}

/// Writes a new P-256 key pair for `role`. Existing files are never replaced,
/// and a failed run leaves neither new file behind.
fn write_keys(dir: &Path, role: Role) -> Result<KeyFiles, Error> {
    let public = dir.join(role.public_file());
    let private = dir.join(role.private_file());
    for path in [&private, &public] {
        if path.exists() {
            return Err(Error::new(format!(
                "{}이 이미 있어 덮어쓰지 않는다. 다른 --dir를 주거나 기존 파일을 지운다",
                path.display()
            )));
        }
    }
    let pair = EcdsaKeyPair::generate(&ECDSA_P256_SHA256_FIXED_SIGNING)
        .map_err(|_| Error::new("P-256 키를 만들지 못했다"))?;
    let point = pair.public_key().as_ref();
    let x = encode(&point[1..=COORDINATE_BYTES]);
    let y = encode(&point[COORDINATE_BYTES + 1..]);
    let scalar = pair
        .private_key()
        .as_be_bytes()
        .map_err(|_| Error::new("개인 스칼라를 꺼내지 못했다"))?;
    let kid = thumbprint(&x, &y);
    let mut key = Map::new();
    for (name, value) in [
        ("kty", "EC"),
        ("crv", "P-256"),
        ("kid", kid.as_str()),
        ("use", "sig"),
        ("alg", "ES256"),
        ("x", x.as_str()),
        ("y", y.as_str()),
    ] {
        key.insert(name.to_owned(), Value::from(value));
    }
    let mut jwks = Map::new();
    jwks.insert(
        "keys".to_owned(),
        Value::Array(vec![Value::Object(key.clone())]),
    );
    key.insert("d".to_owned(), Value::from(encode(scalar.as_ref())));

    fs::create_dir_all(dir).map_err(|error| path_error(dir, &error))?;
    let mut owner_only = OpenOptions::new();
    owner_only.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut owner_only, 0o600);
    write_new(&owner_only, &private, &Value::Object(key))?;
    let mut shared = OpenOptions::new();
    shared.write(true).create_new(true);
    if let Err(error) = write_new(&shared, &public, &Value::Object(jwks)) {
        let _ = fs::remove_file(&private);
        return Err(error);
    }
    Ok(KeyFiles {
        public,
        private,
        kid,
    })
}

/// Creates `path` and writes `value`. A failed write removes the new file.
fn write_new(options: &OpenOptions, path: &Path, value: &Value) -> Result<(), Error> {
    let mut text = serde_json::to_string_pretty(value)?;
    text.push('\n');
    let mut file = options
        .open(path)
        .map_err(|error| path_error(path, &error))?;
    if let Err(error) = file.write_all(text.as_bytes()) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(path_error(path, &error));
    }
    Ok(())
}

fn path_error(path: &Path, error: &std::io::Error) -> Error {
    Error::new(format!("{}: {error}", path.display()))
}

/// RFC 7638 JWK thumbprint of a P-256 public key: unique per key.
fn thumbprint(x: &str, y: &str) -> String {
    let canonical = format!(r#"{{"crv":"P-256","kty":"EC","x":"{x}","y":"{y}"}}"#);
    encode(digest(&SHA256, canonical.as_bytes()).as_ref())
}

struct SigningKey {
    pair: EcdsaKeyPair,
    kid: String,
}

/// Reads a private JWK written by `write_keys`. Errors never quote the file.
fn load_key(path: &Path) -> Result<SigningKey, Error> {
    let text = fs::read_to_string(path).map_err(|error| {
        if error.kind() == ErrorKind::NotFound {
            Error::new(format!(
                "{}에 개인키 파일이 없다. 먼저 같은 키 디렉터리로 `cargo xtask ticket keys`를 실행한다",
                path.display()
            ))
        } else {
            path_error(path, &error)
        }
    })?;
    let invalid = || Error::new(format!("{}의 개인 JWK 형식이 맞지 않다", path.display()));
    let Ok(Value::Object(key)) = serde_json::from_str::<Value>(&text) else {
        return Err(invalid());
    };
    let member = |name: &str| key.get(name).and_then(Value::as_str);
    if member("kty") != Some("EC") || member("crv") != Some("P-256") {
        return Err(invalid());
    }
    let kid = member("kid")
        .filter(|kid| (1..=MAX_CLAIM_TEXT).contains(&kid.len()))
        .ok_or_else(invalid)?;
    let [Some(x), Some(y), Some(d)] = ["x", "y", "d"].map(|name| {
        member(name)
            .and_then(decode)
            .filter(|bytes| bytes.len() == COORDINATE_BYTES)
    }) else {
        return Err(invalid());
    };
    let mut point = Vec::with_capacity(1 + 2 * COORDINATE_BYTES);
    point.push(4);
    point.extend_from_slice(&x);
    point.extend_from_slice(&y);
    let pair =
        EcdsaKeyPair::from_private_key_and_public_key(&ECDSA_P256_SHA256_FIXED_SIGNING, &d, &point)
            .map_err(|_| invalid())?;
    Ok(SigningKey {
        pair,
        kid: kid.to_owned(),
    })
}

/// Signs one ES256 ticket issued at `now` (Unix seconds).
fn issue(key: &SigningKey, role: Role, claims: &Claims, now: u64) -> Result<String, Error> {
    let after = |secs: u64| {
        now.checked_add(secs)
            .ok_or_else(|| Error::new("만료 시각이 2^64 - 1을 넘는다"))
    };
    let mut jti = [0_u8; JTI_BYTES];
    fill(&mut jti).map_err(|_| Error::new("jti 난수를 만들지 못했다"))?;
    let mut body = Map::new();
    body.insert("iss".to_owned(), Value::from(claims.issuer.as_str()));
    body.insert("aud".to_owned(), Value::from(role.audience()));
    body.insert("rh".to_owned(), Value::from(claims.relay_host.as_str()));
    body.insert("rid".to_owned(), Value::from(claims.relay_id.as_str()));
    body.insert("jti".to_owned(), Value::from(encode(&jti)));
    body.insert("iat".to_owned(), Value::from(now));
    body.insert("exp".to_owned(), Value::from(after(claims.lifetime_secs)?));
    if let Some(secs) = claims.session_secs {
        body.insert("sx".to_owned(), Value::from(after(secs)?));
    }
    if let Some(subject) = &claims.subject {
        body.insert("sub".to_owned(), Value::from(subject.as_str()));
    }
    let mut header = Map::new();
    header.insert("alg".to_owned(), Value::from("ES256"));
    header.insert("kid".to_owned(), Value::from(key.kid.as_str()));
    let input = format!(
        "{}.{}",
        encode(Value::Object(header).to_string().as_bytes()),
        encode(Value::Object(body).to_string().as_bytes())
    );
    let signature = key
        .pair
        .sign(&SystemRandom::new(), input.as_bytes())
        .map_err(|_| Error::new("ticket에 서명하지 못했다"))?;
    Ok(format!("{input}.{}", encode(signature.as_ref())))
}

/// Option values by name. Each option appears at most once and needs a value.
fn options<'a>(args: &[&'a str], known: &[&str]) -> Result<BTreeMap<&'a str, &'a str>, String> {
    let mut found = BTreeMap::new();
    let mut rest = args;
    while let [name, tail @ ..] = rest {
        if !known.contains(name) {
            return Err(format!("알 수 없는 인자다: {name}"));
        }
        let [value, tail @ ..] = tail else {
            return Err(format!("옵션 값이 없다: {name}"));
        };
        if found.insert(*name, *value).is_some() {
            return Err(format!("같은 옵션을 두 번 썼다: {name}"));
        }
        rest = tail;
    }
    Ok(found)
}

fn relay_id_argument(text: &str) -> Result<String, String> {
    if (1..=MAX_RELAY_ID_DIGITS).contains(&text.len()) && text.bytes().all(|b| b.is_ascii_digit()) {
        Ok(text.to_owned())
    } else {
        Err(format!(
            "중계 ID는 ASCII 숫자 1~{MAX_RELAY_ID_DIGITS}자리여야 한다: {text}"
        ))
    }
}

fn text(value: &str, name: &str, max: usize) -> Result<String, String> {
    if (1..=max).contains(&value.len()) {
        Ok(value.to_owned())
    } else {
        Err(format!("{name} 값은 1~{max}바이트여야 한다"))
    }
}

fn seconds(value: &str, name: &str, max: Option<u64>) -> Result<u64, String> {
    match (value.parse::<u64>(), max) {
        (Ok(secs), Some(max)) if (1..=max).contains(&secs) => Ok(secs),
        (Ok(secs), None) if secs >= 1 => Ok(secs),
        (_, Some(max)) => Err(format!(
            "{name} 값은 1~{max} 사이의 정수(초)여야 한다: {value}"
        )),
        (_, None) => Err(format!("{name} 값은 1 이상의 정수(초)여야 한다: {value}")),
    }
}

const BASE64URL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// base64url without padding (RFC 7515 section 2).
fn encode(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let mut block = [0_u8; 3];
        block[..chunk.len()].copy_from_slice(chunk);
        let value = u32::from(block[0]) << 16 | u32::from(block[1]) << 8 | u32::from(block[2]);
        for index in 0..=chunk.len() {
            text.push(char::from(
                BASE64URL[(value >> (18 - 6 * index) & 63) as usize],
            ));
        }
    }
    text
}

/// Strict base64url: no padding, no other characters, unused bits zero.
fn decode(text: &str) -> Option<Vec<u8>> {
    if text.len() % 4 == 1 {
        return None;
    }
    let mut bytes = Vec::with_capacity(text.len() * 3 / 4);
    let (mut buffer, mut bits) = (0_u32, 0_u32);
    for byte in text.bytes() {
        let value = BASE64URL.iter().position(|&symbol| symbol == byte)?;
        buffer = buffer << 6 | u32::try_from(value).ok()?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            bytes.push(u8::try_from(buffer >> bits).ok()?);
            buffer &= (1 << bits) - 1;
        }
    }
    (buffer == 0).then_some(bytes)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use aws_lc_rs::signature::{ECDSA_P256_SHA256_FIXED, UnparsedPublicKey};
    use serde_json::json;

    use super::*;

    /// Temporary key directory, removed when the test ends.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            Self(std::env::temp_dir().join(format!(
                "xtask-ticket-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            )))
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn json_part(part: &str) -> Value {
        serde_json::from_slice(&decode(part).unwrap()).unwrap()
    }

    #[test]
    fn issued_tickets_verify_with_the_written_public_key() {
        let scratch = Scratch::new();
        let files = write_keys(&scratch.0, Role::Server).unwrap();
        let jwks: Value =
            serde_json::from_str(&fs::read_to_string(&files.public).unwrap()).unwrap();
        let public = &jwks["keys"][0];
        let x = public["x"].as_str().unwrap();
        let y = public["y"].as_str().unwrap();
        assert_eq!(
            jwks,
            json!({"keys": [{"kty": "EC", "crv": "P-256", "kid": files.kid, "use": "sig", "alg": "ES256", "x": x, "y": y}]})
        );
        let mut point = vec![4];
        point.extend(decode(x).unwrap());
        point.extend(decode(y).unwrap());

        let key = load_key(&files.private).unwrap();
        let claims = Claims {
            relay_id: "0123".to_owned(),
            issuer: "issuer.test".to_owned(),
            relay_host: "relay.test".to_owned(),
            lifetime_secs: 45,
            session_secs: Some(120),
            subject: Some("session-1".to_owned()),
        };
        let ticket = issue(&key, Role::Server, &claims, 1_800_000_000).unwrap();
        let parts: Vec<&str> = ticket.split('.').collect();
        let [header, body, signature] = parts.as_slice() else {
            panic!("ticket must have three parts: {ticket}");
        };
        assert_eq!(json_part(header), json!({"alg": "ES256", "kid": files.kid}));
        let body = json_part(body);
        let jti = body["jti"].as_str().unwrap();
        assert_eq!(decode(jti).unwrap().len(), JTI_BYTES);
        assert_eq!(
            body,
            json!({
                "iss": "issuer.test", "aud": "vnc-relay/server", "rh": "relay.test", "rid": "0123",
                "jti": jti, "iat": 1_800_000_000_u64, "exp": 1_800_000_045_u64,
                "sx": 1_800_000_120_u64, "sub": "session-1"
            })
        );
        let signature = decode(signature).unwrap();
        assert_eq!(signature.len(), 64, "ES256 needs R||S, not DER");
        let signed = &ticket[..ticket.rfind('.').unwrap()];
        UnparsedPublicKey::new(&ECDSA_P256_SHA256_FIXED, &point)
            .verify(signed.as_bytes(), &signature)
            .unwrap();

        let again = issue(&key, Role::Server, &claims, 1_800_000_000).unwrap();
        let again = json_part(again.split('.').nth(1).unwrap());
        assert_ne!(again["jti"], body["jti"], "each ticket needs a fresh jti");
    }

    #[test]
    fn keys_never_replace_existing_files() {
        let scratch = Scratch::new();
        let viewer = write_keys(&scratch.0, Role::Viewer).unwrap();
        let before = [
            fs::read(&viewer.public).unwrap(),
            fs::read(&viewer.private).unwrap(),
        ];
        assert!(write_keys(&scratch.0, Role::Viewer).is_err());
        assert_eq!(
            [
                fs::read(&viewer.public).unwrap(),
                fs::read(&viewer.private).unwrap()
            ],
            before
        );

        fs::remove_file(&viewer.private).unwrap();
        assert!(
            write_keys(&scratch.0, Role::Viewer).is_err(),
            "a public key file alone also blocks new keys"
        );
        assert!(!viewer.private.exists());

        let server = write_keys(&scratch.0, Role::Server).unwrap();
        assert_ne!(server.kid, viewer.kid, "kid differs per key");
    }

    #[cfg(unix)]
    #[test]
    fn private_key_file_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let scratch = Scratch::new();
        let files = write_keys(&scratch.0, Role::Viewer).unwrap();
        let mode = fs::metadata(&files.private).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn base64url_matches_rfc_4648_and_rejects_non_canonical_text() {
        let vectors: [(&[u8], &str); 8] = [
            (b"", ""),
            (b"f", "Zg"),
            (b"fo", "Zm8"),
            (b"foo", "Zm9v"),
            (b"foob", "Zm9vYg"),
            (b"fooba", "Zm9vYmE"),
            (b"foobar", "Zm9vYmFy"),
            (&[0xfb, 0xff], "-_8"),
        ];
        for (bytes, text) in vectors {
            assert_eq!(encode(bytes), text);
            assert_eq!(decode(text).as_deref(), Some(bytes));
        }
        for text in ["Zg==", "Z", "Zh", "Zm9", "Zm+v", "Zm/v"] {
            assert_eq!(decode(text), None, "{text}");
        }
    }

    #[test]
    fn arguments_outside_relay_limits_are_usage_errors() {
        let longest = "9".repeat(MAX_RELAY_ID_DIGITS);
        let too_long = "9".repeat(MAX_RELAY_ID_DIGITS + 1);
        let subject = "s".repeat(MAX_CLAIM_TEXT);
        let long_subject = "s".repeat(MAX_CLAIM_TEXT + 1);
        let accepted: [&[&str]; 6] = [
            &["keys", "viewer"],
            &["keys", "server", "--dir", "keys"],
            &["issue", "viewer", longest.as_str()],
            &["issue", "server", "0", "--lifetime", "1"],
            &[
                "issue",
                "server",
                "0",
                "--lifetime",
                "600",
                "--session",
                "1",
            ],
            &["issue", "viewer", "1", "--subject", subject.as_str()],
        ];
        for args in accepted {
            assert!(parse(args).is_ok(), "{args:?}");
        }
        let rejected: [&[&str]; 14] = [
            &[],
            &["keys"],
            &["keys", "admin"],
            &["issue", "viewer"],
            &["issue", "viewer", ""],
            &["issue", "viewer", "12a"],
            &["issue", "viewer", too_long.as_str()],
            &["issue", "viewer", "1", "--lifetime", "0"],
            &["issue", "viewer", "1", "--lifetime", "601"],
            &["issue", "viewer", "1", "--session", "0"],
            &["issue", "viewer", "1", "--subject", long_subject.as_str()],
            &["issue", "viewer", "1", "--dir"],
            &["issue", "viewer", "1", "--dir", "a", "--dir", "b"],
            &["keys", "viewer", "--issuer", "issuer.test"],
        ];
        for args in rejected {
            assert!(parse(args).is_err(), "{args:?}");
        }
    }
}
