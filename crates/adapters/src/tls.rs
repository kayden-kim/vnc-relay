//! Built-in TLS: loads PEM certificate and private key files into a TLS
//! acceptor (docs/architectures/tls.md).

use std::fmt;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use rustls::crypto::aws_lc_rs;
use rustls::{InconsistentKeys, ServerConfig};
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls::TlsAcceptor;
use vnc_relay_app::reload::ports::{ReloadTlsStore, TlsBuildError};
use vnc_relay_app::reload::snapshot::{ReloadCell, ReloadSnapshot};

/// The two PEM files that make up the TLS identity of one listener.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsFile {
    Cert,
    Key,
}

impl TlsFile {
    fn label(self) -> &'static str {
        match self {
            Self::Cert => "인증서 파일",
            Self::Key => "개인키 파일",
        }
    }
}

/// Why a certificate or private key file cannot start a TLS acceptor.
#[derive(Debug)]
pub enum TlsError {
    /// The file does not exist or cannot be read.
    Unreadable { file: TlsFile },
    /// The file is not PEM, or it has no item of the expected kind.
    Malformed { file: TlsFile },
    /// The private key kind is not supported, or the key is damaged.
    KeyUnsupported,
    /// The private key does not belong to the end-entity certificate.
    KeyMismatch,
    /// rustls rejected the configuration for another reason.
    Config(rustls::Error),
}

impl TlsError {
    /// Diagnostic code shared by every TLS file error (docs/contracts/cli-v1.md).
    pub fn code(&self) -> &'static str {
        "tls_file_invalid"
    }

    /// The file that the config key of the diagnostic names.
    /// A key error names the key file; a configuration error names the certificate file.
    pub fn file(&self) -> TlsFile {
        match self {
            Self::Unreadable { file } | Self::Malformed { file } => *file,
            Self::KeyUnsupported | Self::KeyMismatch => TlsFile::Key,
            Self::Config(_) => TlsFile::Cert,
        }
    }
}

impl fmt::Display for TlsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable { file } => write!(f, "{}을 읽을 수 없다.", file.label()),
            Self::Malformed { file } => {
                write!(f, "{}이 PEM 형식이 아니거나 항목이 없다.", file.label())
            }
            Self::KeyUnsupported => f.write_str(
                "지원하지 않는 개인키 종류이거나 손상되었다. RSA, ECDSA(P-256·P-384·P-521), Ed25519만 쓸 수 있다.",
            ),
            Self::KeyMismatch => f.write_str("개인키가 인증서와 맞지 않다."),
            Self::Config(_) => f.write_str("TLS 설정을 만들 수 없다."),
        }
    }
}

impl std::error::Error for TlsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Config(err) => Some(err),
            _ => None,
        }
    }
}

/// Builds the TLS acceptor of one listener from its certificate chain and private key files.
///
/// The certificate file is read and parsed before the key file is read, so the
/// first broken file is the one reported. The acceptor negotiates TLS 1.2 and
/// TLS 1.3 with the aws-lc-rs provider. It requests no client certificate and
/// selects no ALPN protocol. A client that offers only TLS 1.0 or TLS 1.1 fails
/// the handshake.
pub fn load_acceptor(cert_file: &Path, key_file: &Path) -> Result<TlsAcceptor, TlsError> {
    let chain = parse_chain(&read_file(cert_file, TlsFile::Cert)?)?;
    let key = parse_key(&read_file(key_file, TlsFile::Key)?)?;
    build_acceptor(chain, key)
}

/// Builds the TLS acceptor of one listener from the PEM bytes of its certificate
/// chain and private key. It accepts the same input as `load_acceptor` and runs
/// the same checks (file-watch.md §7).
pub fn acceptor_from_pem(cert_pem: &[u8], key_pem: &[u8]) -> Result<TlsAcceptor, TlsError> {
    let chain = parse_chain(cert_pem)?;
    let key = parse_key(key_pem)?;
    build_acceptor(chain, key)
}

/// Reads the whole file. Any read error means the file is unreadable.
fn read_file(path: &Path, file: TlsFile) -> Result<Vec<u8>, TlsError> {
    fs::read(path).map_err(|_| TlsError::Unreadable { file })
}

/// Parses every certificate in the PEM bytes. The first one must be the end-entity certificate.
fn parse_chain(pem: &[u8]) -> Result<Vec<CertificateDer<'static>>, TlsError> {
    let chain = CertificateDer::pem_slice_iter(pem)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| TlsError::Malformed {
            file: TlsFile::Cert,
        })?;
    if chain.is_empty() {
        return Err(TlsError::Malformed {
            file: TlsFile::Cert,
        });
    }
    Ok(chain)
}

/// Parses the first private key in the PEM bytes.
fn parse_key(pem: &[u8]) -> Result<PrivateKeyDer<'static>, TlsError> {
    PrivateKeyDer::from_pem_slice(pem).map_err(|_| TlsError::Malformed { file: TlsFile::Key })
}

/// Builds the rustls configuration and the acceptor from a parsed chain and key.
fn build_acceptor(
    chain: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
) -> Result<TlsAcceptor, TlsError> {
    let config = ServerConfig::builder_with_provider(Arc::new(aws_lc_rs::default_provider()))
        .with_protocol_versions(&[&rustls::version::TLS12, &rustls::version::TLS13])
        .map_err(TlsError::Config)?
        .with_no_client_auth()
        .with_single_cert(chain, key)
        .map_err(classify_config)?;
    Ok(TlsAcceptor::from(Arc::new(config)))
}

/// Maps the errors of the key and certificate check to the file that caused them.
fn classify_config(err: rustls::Error) -> TlsError {
    match err {
        rustls::Error::InconsistentKeys(InconsistentKeys::KeyMismatch) => TlsError::KeyMismatch,
        // rustls reports an unsupported or damaged private key as a general error.
        rustls::Error::General(_) => TlsError::KeyUnsupported,
        rustls::Error::InvalidCertificate(_) | rustls::Error::NoCertificatesPresented => {
            TlsError::Malformed {
                file: TlsFile::Cert,
            }
        }
        other => TlsError::Config(other),
    }
}

/// Maps a build failure to the detail of `TlsBuildError`. The detail names the
/// part that failed and never a path or the file contents.
fn build_detail(error: &TlsError) -> &'static str {
    match error {
        TlsError::Unreadable { file } | TlsError::Malformed { file } => match file {
            TlsFile::Cert => "certificate format",
            TlsFile::Key => "key format",
        },
        TlsError::KeyUnsupported => "key format",
        TlsError::KeyMismatch => "pair mismatch",
        TlsError::Config(_) => "invalid",
    }
}

/// The rustls build of a reloaded TLS identity. It takes the verified PEM bytes
/// (file-watch.md §7), so a reload never builds from paths.
#[derive(Debug, Clone, Copy)]
pub struct RustlsTlsStore;

impl ReloadTlsStore<TlsAcceptor> for RustlsTlsStore {
    fn build(&self, cert_pem: &[u8], key_pem: &[u8]) -> Result<TlsAcceptor, TlsBuildError> {
        acceptor_from_pem(cert_pem, key_pem).map_err(|error| TlsBuildError {
            detail: build_detail(&error),
        })
    }
}

/// Where a TLS listener takes its acceptor. Each connection reads the acceptor
/// current at its handshake, so a reload applies to new handshakes only
/// (file-watch.md §4).
#[derive(Clone)]
pub struct TlsSource {
    cell: Arc<ReloadCell<TlsAcceptor>>,
    pick: fn(&ReloadSnapshot<TlsAcceptor>) -> Option<&TlsAcceptor>,
}

impl TlsSource {
    /// The shared WSS listener's acceptor, used when either WSS role is enabled.
    pub fn wss(cell: Arc<ReloadCell<TlsAcceptor>>) -> Self {
        Self {
            cell,
            pick: pick_wss,
        }
    }

    /// The management acceptor of the snapshot. It is `None` while management
    /// TLS is not configured.
    pub fn management(cell: Arc<ReloadCell<TlsAcceptor>>) -> Self {
        Self {
            cell,
            pick: pick_management,
        }
    }

    /// The acceptor current now. `None` means the snapshot holds none for this
    /// listener, and the caller fails that connection.
    pub fn acceptor(&self) -> Option<TlsAcceptor> {
        let snapshot = self.cell.current();
        (self.pick)(&snapshot).cloned()
    }
}

impl fmt::Debug for TlsSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TlsSource").finish_non_exhaustive()
    }
}

fn pick_wss(snapshot: &ReloadSnapshot<TlsAcceptor>) -> Option<&TlsAcceptor> {
    snapshot.wss_tls.as_ref()
}

fn pick_management(snapshot: &ReloadSnapshot<TlsAcceptor>) -> Option<&TlsAcceptor> {
    snapshot.management_tls.as_ref()
}
