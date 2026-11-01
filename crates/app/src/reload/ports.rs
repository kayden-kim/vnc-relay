//! Ports of the reload service. The adapters implement them with the file system and rustls.

use std::fmt;
use std::future::Future;
use std::io;
use std::path::Path;
use std::time::Duration;

/// Reads whole files, used for certificate and key bytes (file-watch-v1 §4 step 6).
pub trait ReloadFileStore: Send + Sync {
    /// Reads the whole file at `path`.
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
}

/// Builds a TLS acceptor from PEM bytes (config-v2 §1.9, file-watch-v1 §4 step 6).
///
/// The acceptor type `A` is opaque to the app layer.
pub trait ReloadTlsStore<A>: Send + Sync {
    /// Builds an acceptor from the certificate and key PEM bytes.
    fn build(&self, cert_pem: &[u8], key_pem: &[u8]) -> Result<A, TlsBuildError>;
}

/// Why a certificate and key pair could not be built (config-v2 §2.1 `tls_file_invalid`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsBuildError {
    /// Fixed short cause such as `"pair mismatch"`, `"key format"`, or `"certificate format"`.
    /// It never contains file contents or paths.
    pub detail: &'static str,
}

impl fmt::Display for TlsBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.detail)
    }
}

impl std::error::Error for TlsBuildError {}

/// The watcher's only wait (file-watch-v1 §4). Tests drive it manually.
pub trait ReloadClock: Send + Sync {
    /// Completes once `duration` has passed.
    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + Send;
}
