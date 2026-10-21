//! Built-in TLS: loads PEM certificate and private key files into a TLS
//! acceptor (docs/architectures/tls.md).

use std::fmt;
use std::path::Path;
use std::sync::Arc;

use rustls::crypto::aws_lc_rs;
use rustls::{InconsistentKeys, ServerConfig};
use rustls_pki_types::pem::{self, PemObject};
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls::TlsAcceptor;

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
/// The acceptor negotiates TLS 1.2 and TLS 1.3 with the aws-lc-rs provider.
/// It requests no client certificate and selects no ALPN protocol.
/// A client that offers only TLS 1.0 or TLS 1.1 fails the handshake.
pub fn load_acceptor(cert_file: &Path, key_file: &Path) -> Result<TlsAcceptor, TlsError> {
    let chain = read_chain(cert_file)?;
    let key = read_key(key_file)?;
    let config = ServerConfig::builder_with_provider(Arc::new(aws_lc_rs::default_provider()))
        .with_protocol_versions(&[&rustls::version::TLS12, &rustls::version::TLS13])
        .map_err(TlsError::Config)?
        .with_no_client_auth()
        .with_single_cert(chain, key)
        .map_err(classify_config)?;
    Ok(TlsAcceptor::from(Arc::new(config)))
}

/// Reads every certificate in the file. The first one must be the end-entity certificate.
fn read_chain(path: &Path) -> Result<Vec<CertificateDer<'static>>, TlsError> {
    let chain = CertificateDer::pem_file_iter(path)
        .map_err(|err| pem_error(err, TlsFile::Cert))?
        .map(|item| item.map_err(|err| pem_error(err, TlsFile::Cert)))
        .collect::<Result<Vec<_>, _>>()?;
    if chain.is_empty() {
        return Err(TlsError::Malformed {
            file: TlsFile::Cert,
        });
    }
    Ok(chain)
}

/// Reads the first private key in the file.
fn read_key(path: &Path) -> Result<PrivateKeyDer<'static>, TlsError> {
    PrivateKeyDer::from_pem_file(path).map_err(|err| pem_error(err, TlsFile::Key))
}

fn pem_error(err: pem::Error, file: TlsFile) -> TlsError {
    match err {
        pem::Error::Io(_) => TlsError::Unreadable { file },
        _ => TlsError::Malformed { file },
    }
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
