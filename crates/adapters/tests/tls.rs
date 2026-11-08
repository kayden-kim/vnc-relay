// Contract tests of the built-in TLS acceptor (docs/architectures/tls.md,
// docs/contracts/config-v4.md §1.9).
//
// Fixtures under tests/fixtures/tls are static PEM files, committed as they are.
// They were generated once on WSL Ubuntu with OpenSSL 3.0.13, by the commands
// below. CA private keys were created in a temporary directory and removed after
// the run. Keys that no certificate uses (other-key-p256.pem, x25519-key.pem) exist
// only for negative tests.
//
// Generation, in an empty directory, in this order:
//
//   # ext.cnf: sections root_ca (CA:TRUE), inter_ca (CA:TRUE, pathlen:0),
//   #   leaf_localhost (CA:FALSE, serverAuth, SAN DNS:localhost,IP:127.0.0.1),
//   #   leaf_other_host (CA:FALSE, serverAuth, SAN DNS:other.example).
//   # expired.cnf: [ ca ] default_ca = ca_default; database = index.txt;
//   #   new_certs_dir = newcerts; serial = serial; default_md = sha256;
//   #   policy = policy_any (commonName = supplied); unique_subject = no;
//   #   copy_extensions = none.
//   # index.txt is empty, serial holds "01", newcerts/ exists.
//   issue() { openssl req -new -key "$2" -subj "$3" -out "$1.csr"
//     openssl x509 -req -in "$1.csr" -CA "$5" -CAkey "$6" -CAcreateserial \
//       -days 3650 -sha256 -extfile ext.cnf -extensions "$4" -out "$1.pem"; }
//   # issue <name> <leaf key> <subject> <extension section> <CA cert> <CA key>
//
//   openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 -out root-key.pem
//   openssl req -x509 -new -key root-key.pem -subj "/CN=VNC Relay Test Root CA" \
//     -days 3650 -sha256 -config ext.cnf -extensions root_ca -out root-ca.pem
//   openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 -out inter-key.pem
//   openssl req -new -key inter-key.pem -subj "/CN=VNC Relay Test Intermediate CA" -out inter.csr
//   openssl x509 -req -in inter.csr -CA root-ca.pem -CAkey root-key.pem -CAcreateserial \
//     -days 3650 -sha256 -extfile ext.cnf -extensions inter_ca -out inter.pem
//
//   openssl ecparam -name prime256v1 -genkey -noout -out server-key-p256-sec1.pem
//   openssl pkcs8 -topk8 -nocrypt -in server-key-p256-sec1.pem -out server-key-p256.pem
//   openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-384 -out ecdsa-p384-key.pem
//   openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-521 -out ecdsa-p521-key.pem
//   openssl ec -in ecdsa-p384-key.pem -out ecdsa-p384-key-sec1.pem
//   openssl ec -in ecdsa-p521-key.pem -out ecdsa-p521-key-sec1.pem
//   openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out rsa-key-pkcs8.pem
//   openssl rsa -in rsa-key-pkcs8.pem -traditional -out rsa-key-pkcs1.pem
//   openssl genpkey -algorithm ED25519 -out ed25519-key.pem
//   openssl genpkey -algorithm X25519 -out x25519-key.pem
//   openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 -out other-key-p256.pem
//   openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 -out other-host-key.pem
//   openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 -out untrusted-key.pem
//   openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 -out expired-key.pem
//
//   issue server-p256 server-key-p256.pem "/CN=localhost" leaf_localhost inter.pem inter-key.pem
//   issue ecdsa-p384 ecdsa-p384-key.pem "/CN=localhost" leaf_localhost inter.pem inter-key.pem
//   issue ecdsa-p521 ecdsa-p521-key.pem "/CN=localhost" leaf_localhost inter.pem inter-key.pem
//   issue rsa rsa-key-pkcs8.pem "/CN=localhost" leaf_localhost inter.pem inter-key.pem
//   issue ed25519 ed25519-key.pem "/CN=localhost" leaf_localhost inter.pem inter-key.pem
//   issue other-host other-host-key.pem "/CN=other.example" leaf_other_host inter.pem inter-key.pem
//
//   # Expired leaf: validity 2020-01-01 to 2020-02-01, signed by the intermediate.
//   openssl req -new -key expired-key.pem -subj "/CN=localhost" -out expired.csr
//   openssl ca -batch -config expired.cnf -cert inter.pem -keyfile inter-key.pem \
//     -extfile ext.cnf -extensions leaf_localhost \
//     -startdate 200101000000Z -enddate 200201000000Z -in expired.csr -out expired.pem
//
//   # Untrusted leaf: signed by a self-signed CA that the test client does not trust.
//   openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 -out untrusted-ca-key.pem
//   openssl req -x509 -new -key untrusted-ca-key.pem -subj "/CN=VNC Relay Untrusted Test CA" \
//     -days 3650 -sha256 -config ext.cnf -extensions root_ca -out untrusted-ca.pem
//   issue untrusted untrusted-key.pem "/CN=localhost" leaf_localhost untrusted-ca.pem untrusted-ca-key.pem
//
//   # Chain files: leaf first, then its issuer chain.
//   cat server-p256.pem inter.pem > server-chain.pem
//   cat ecdsa-p384.pem inter.pem > ecdsa-p384-chain.pem
//   cat ecdsa-p521.pem inter.pem > ecdsa-p521-chain.pem
//   cat rsa.pem inter.pem > rsa-chain.pem
//   cat ed25519.pem inter.pem > ed25519-chain.pem
//   cat other-host.pem inter.pem > other-host-chain.pem
//   cat expired.pem inter.pem > expired-chain.pem
//   cat untrusted.pem untrusted-ca.pem > untrusted-chain.pem
//
// Fixture files (23): root-ca.pem, server-chain.pem, server-key-p256.pem,
// server-key-p256-sec1.pem, ecdsa-p384-chain.pem, ecdsa-p384-key.pem,
// ecdsa-p384-key-sec1.pem, ecdsa-p521-chain.pem, ecdsa-p521-key.pem,
// ecdsa-p521-key-sec1.pem, rsa-chain.pem, rsa-key-pkcs8.pem,
// rsa-key-pkcs1.pem, ed25519-chain.pem, ed25519-key.pem, x25519-key.pem,
// other-key-p256.pem, expired-chain.pem, expired-key.pem, other-host-chain.pem,
// other-host-key.pem, untrusted-chain.pem, untrusted-key.pem.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use rustls::crypto::aws_lc_rs;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::version::{TLS12, TLS13};
use rustls::{
    CertificateError, ClientConfig, ProtocolVersion, RootCertStore, SupportedProtocolVersion,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use tokio_rustls::{TlsAcceptor, TlsConnector};
use vnc_relay_adapters::tls::{self, TlsError, TlsFile};

/// Upper bound for one handshake. A hang fails the test instead of blocking it.
const HANDSHAKE_LIMIT: Duration = Duration::from_secs(10);

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/tls")
        .join(name)
}

fn load(chain: &str, key: &str) -> Result<TlsAcceptor, TlsError> {
    tls::load_acceptor(&fixture(chain), &fixture(key))
}

fn load_ok(chain: &str, key: &str) -> TlsAcceptor {
    match load(chain, key) {
        Ok(acceptor) => acceptor,
        Err(error) => panic!("{chain} with {key} must load: {error}"),
    }
}

fn load_err(chain: &str, key: &str) -> TlsError {
    match load(chain, key) {
        Ok(_) => panic!("{chain} with {key} must not load"),
        Err(error) => error,
    }
}

/// A client that trusts only the test root CA and offers the given versions.
fn connector(versions: &[&'static SupportedProtocolVersion]) -> TlsConnector {
    let mut roots = RootCertStore::empty();
    let certs = CertificateDer::pem_file_iter(fixture("root-ca.pem"))
        .unwrap_or_else(|error| panic!("root CA opens: {error}"));
    for cert in certs {
        let cert = cert.unwrap_or_else(|error| panic!("root CA is PEM: {error}"));
        roots
            .add(cert)
            .unwrap_or_else(|error| panic!("root CA is a certificate: {error}"));
    }
    let config = ClientConfig::builder_with_provider(Arc::new(aws_lc_rs::default_provider()))
        .with_protocol_versions(versions)
        .unwrap_or_else(|error| panic!("client protocol versions: {error}"))
        .with_root_certificates(roots)
        .with_no_client_auth();
    TlsConnector::from(Arc::new(config))
}

/// Result of one loopback handshake, seen from each side.
struct Handshake {
    client: io::Result<Option<ProtocolVersion>>,
    server: io::Result<()>,
}

async fn handshake(
    acceptor: TlsAcceptor,
    connector: TlsConnector,
    host: &'static str,
) -> Handshake {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap_or_else(|error| panic!("loopback bind: {error}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|error| panic!("loopback address: {error}"));
    let server: JoinHandle<io::Result<()>> = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await?;
        acceptor.accept(tcp).await.map(|_| ())
    });
    let client = async move {
        let tcp = TcpStream::connect(addr).await?;
        let name = ServerName::try_from(host).map_err(io::Error::other)?;
        let stream = connector.connect(name, tcp).await?;
        Ok::<_, io::Error>(stream.get_ref().1.protocol_version())
    };
    let (client, server) =
        tokio::time::timeout(HANDSHAKE_LIMIT, async { tokio::join!(client, server) })
            .await
            .unwrap_or_else(|error| panic!("handshake finishes in time: {error}"));
    Handshake {
        client,
        server: server.unwrap_or_else(|error| panic!("server task completes: {error}")),
    }
}

/// The rustls error that a failed client handshake carries.
fn rustls_error(error: &io::Error) -> Option<&rustls::Error> {
    error.get_ref()?.downcast_ref::<rustls::Error>()
}

/// A TLS record with a ClientHello that offers `legacy_version`, no extensions,
/// and one cipher suite. The server must refuse it, since it offers no TLS 1.2.
fn client_hello(legacy_version: [u8; 2]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&legacy_version);
    body.extend_from_slice(&[0x5A; 32]); // random
    body.push(0); // legacy_session_id: empty
    body.extend_from_slice(&[0x00, 0x02, 0x00, 0x2F]); // TLS_RSA_WITH_AES_128_CBC_SHA
    body.extend_from_slice(&[0x01, 0x00]); // compression methods: null
    body.extend_from_slice(&[0x00, 0x00]); // extensions: none
    let mut handshake = vec![0x01]; // msg_type: client_hello
    handshake.extend_from_slice(&be_bytes(body.len(), 3));
    handshake.extend_from_slice(&body);
    let mut record = vec![0x16, 0x03, 0x01]; // handshake record
    record.extend_from_slice(&be_bytes(handshake.len(), 2));
    record.extend_from_slice(&handshake);
    record
}

/// The lowest `width` big-endian bytes of `value`.
fn be_bytes(value: usize, width: usize) -> Vec<u8> {
    value.to_be_bytes()[std::mem::size_of::<usize>() - width..].to_vec()
}

async fn legacy_hello_refused(legacy_version: [u8; 2]) {
    let acceptor = load_ok("server-chain.pem", "server-key-p256.pem");
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap_or_else(|error| panic!("loopback bind: {error}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|error| panic!("loopback address: {error}"));
    let server = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await?;
        acceptor.accept(tcp).await.map(|_| ())
    });
    let mut tcp = TcpStream::connect(addr)
        .await
        .unwrap_or_else(|error| panic!("loopback connect: {error}"));
    tcp.write_all(&client_hello(legacy_version))
        .await
        .unwrap_or_else(|error| panic!("hello is sent: {error}"));
    let mut reply = Vec::new();
    tokio::time::timeout(HANDSHAKE_LIMIT, tcp.read_to_end(&mut reply))
        .await
        .unwrap_or_else(|error| panic!("server closes in time: {error}"))
        .unwrap_or_else(|error| panic!("reply is read: {error}"));
    // A refusal is an alert record (0x15) or a closed connection. A ServerHello (0x16) is a failure.
    assert!(
        reply.first().is_none_or(|&record| record == 0x15),
        "reply to {legacy_version:02x?} must not start a handshake: {reply:02x?}"
    );
    let server_result = server
        .await
        .unwrap_or_else(|error| panic!("server task completes: {error}"));
    assert!(
        server_result.is_err(),
        "server must reject a hello below TLS 1.2"
    );
}

/// spec: operations.md:OPS-036 partial
#[tokio::test]
async fn tls12_client_completes_handshake() {
    let outcome = handshake(
        load_ok("server-chain.pem", "server-key-p256.pem"),
        connector(&[&TLS12]),
        "localhost",
    )
    .await;
    let version = outcome
        .client
        .unwrap_or_else(|error| panic!("TLS 1.2 handshake succeeds: {error}"));
    assert_eq!(version, Some(ProtocolVersion::TLSv1_2));
    outcome
        .server
        .unwrap_or_else(|error| panic!("server accepts TLS 1.2: {error}"));
}

/// spec: operations.md:OPS-036 partial
#[tokio::test]
async fn tls13_client_completes_handshake() {
    let outcome = handshake(
        load_ok("server-chain.pem", "server-key-p256.pem"),
        connector(&[&TLS13]),
        "localhost",
    )
    .await;
    let version = outcome
        .client
        .unwrap_or_else(|error| panic!("TLS 1.3 handshake succeeds: {error}"));
    assert_eq!(version, Some(ProtocolVersion::TLSv1_3));
    outcome
        .server
        .unwrap_or_else(|error| panic!("server accepts TLS 1.3: {error}"));
}

/// spec: operations.md:OPS-036 partial
#[tokio::test]
async fn tls10_and_tls11_hellos_are_refused() {
    legacy_hello_refused([0x03, 0x01]).await;
    legacy_hello_refused([0x03, 0x02]).await;
}

#[tokio::test]
async fn supported_key_kinds_complete_tls13_handshake() {
    let kinds = [
        (
            "server-chain.pem",
            "server-key-p256.pem",
            "ECDSA P-256 PKCS#8",
        ),
        (
            "server-chain.pem",
            "server-key-p256-sec1.pem",
            "ECDSA P-256 SEC1",
        ),
        (
            "ecdsa-p384-chain.pem",
            "ecdsa-p384-key.pem",
            "ECDSA P-384 PKCS#8",
        ),
        (
            "ecdsa-p384-chain.pem",
            "ecdsa-p384-key-sec1.pem",
            "ECDSA P-384 SEC1",
        ),
        (
            "ecdsa-p521-chain.pem",
            "ecdsa-p521-key.pem",
            "ECDSA P-521 PKCS#8",
        ),
        (
            "ecdsa-p521-chain.pem",
            "ecdsa-p521-key-sec1.pem",
            "ECDSA P-521 SEC1",
        ),
        ("rsa-chain.pem", "rsa-key-pkcs8.pem", "RSA PKCS#8"),
        ("rsa-chain.pem", "rsa-key-pkcs1.pem", "RSA PKCS#1"),
        ("ed25519-chain.pem", "ed25519-key.pem", "Ed25519"),
    ];
    for (chain, key, kind) in kinds {
        let outcome = handshake(load_ok(chain, key), connector(&[&TLS13]), "localhost").await;
        let version = outcome
            .client
            .unwrap_or_else(|error| panic!("{kind}: client handshake failed: {error}"));
        assert_eq!(version, Some(ProtocolVersion::TLSv1_3), "{kind}");
        outcome
            .server
            .unwrap_or_else(|error| panic!("{kind}: server handshake failed: {error}"));
    }
}

#[test]
fn x25519_key_is_unsupported() {
    let error = load_err("server-chain.pem", "x25519-key.pem");
    assert!(matches!(error, TlsError::KeyUnsupported), "{error:?}");
    assert_eq!(error.file(), TlsFile::Key);
}

#[test]
fn key_of_another_certificate_is_mismatch() {
    let error = load_err("server-chain.pem", "other-key-p256.pem");
    assert!(matches!(error, TlsError::KeyMismatch), "{error:?}");
    assert_eq!(error.file(), TlsFile::Key);
}

#[test]
fn missing_files_are_unreadable() {
    let missing = fixture("no-such-file.pem");
    let cert_error = tls::load_acceptor(&missing, &fixture("server-key-p256.pem"))
        .err()
        .unwrap_or_else(|| panic!("missing certificate file fails"));
    assert!(
        matches!(
            cert_error,
            TlsError::Unreadable {
                file: TlsFile::Cert
            }
        ),
        "{cert_error:?}"
    );
    assert_eq!(cert_error.code(), "tls_file_invalid");
    let key_error = tls::load_acceptor(&fixture("server-chain.pem"), &missing)
        .err()
        .unwrap_or_else(|| panic!("missing key file fails"));
    assert!(
        matches!(key_error, TlsError::Unreadable { file: TlsFile::Key }),
        "{key_error:?}"
    );
}

#[test]
fn non_pem_and_missing_items_are_malformed() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let not_pem = tls::load_acceptor(&manifest, &fixture("server-key-p256.pem"))
        .err()
        .unwrap_or_else(|| panic!("non-PEM certificate file fails"));
    assert!(
        matches!(
            not_pem,
            TlsError::Malformed {
                file: TlsFile::Cert
            }
        ),
        "{not_pem:?}"
    );
    // A certificate file has no private key item.
    let no_key = load_err("server-chain.pem", "server-chain.pem");
    assert!(
        matches!(no_key, TlsError::Malformed { file: TlsFile::Key }),
        "{no_key:?}"
    );
    // A key file has no certificate item.
    let no_cert = load_err("server-key-p256.pem", "server-key-p256.pem");
    assert!(
        matches!(
            no_cert,
            TlsError::Malformed {
                file: TlsFile::Cert
            }
        ),
        "{no_cert:?}"
    );
}

/// spec: websocket.md:WS-013 partial
#[tokio::test]
async fn expired_certificate_fails_client_without_plaintext_fallback() {
    let outcome = handshake(
        load_ok("expired-chain.pem", "expired-key.pem"),
        connector(&[&TLS13]),
        "localhost",
    )
    .await;
    let error = outcome
        .client
        .err()
        .unwrap_or_else(|| panic!("expired certificate must fail the client"));
    assert!(
        matches!(
            rustls_error(&error),
            Some(rustls::Error::InvalidCertificate(
                CertificateError::Expired | CertificateError::ExpiredContext { .. }
            ))
        ),
        "{error:?}"
    );
    assert!(outcome.server.is_err(), "server sees the failed handshake");
}

/// spec: websocket.md:WS-013 partial
#[tokio::test]
async fn untrusted_issuer_fails_client_without_plaintext_fallback() {
    let outcome = handshake(
        load_ok("untrusted-chain.pem", "untrusted-key.pem"),
        connector(&[&TLS13]),
        "localhost",
    )
    .await;
    let error = outcome
        .client
        .err()
        .unwrap_or_else(|| panic!("untrusted issuer must fail the client"));
    assert!(
        matches!(
            rustls_error(&error),
            Some(rustls::Error::InvalidCertificate(
                CertificateError::UnknownIssuer
            ))
        ),
        "{error:?}"
    );
    assert!(outcome.server.is_err(), "server sees the failed handshake");
}

/// spec: websocket.md:WS-013 partial
#[tokio::test]
async fn host_mismatch_fails_client_without_plaintext_fallback() {
    let outcome = handshake(
        load_ok("other-host-chain.pem", "other-host-key.pem"),
        connector(&[&TLS13]),
        "localhost",
    )
    .await;
    let error = outcome
        .client
        .err()
        .unwrap_or_else(|| panic!("host mismatch must fail the client"));
    assert!(
        matches!(
            rustls_error(&error),
            Some(rustls::Error::InvalidCertificate(
                CertificateError::NotValidForName | CertificateError::NotValidForNameContext { .. }
            ))
        ),
        "{error:?}"
    );
    assert!(outcome.server.is_err(), "server sees the failed handshake");
}
