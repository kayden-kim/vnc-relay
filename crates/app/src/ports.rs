//! Ports: traits for outside functions that app use cases call. adapters
//! implements them (docs/architectures/code-map.md, layer "app").

use std::fmt;
use std::time::{Duration, SystemTime};

use vnc_relay_domain::account::{Account, Argon2Params, Password, PasswordHash};

use crate::accounts::{AccountsDiagnostic, AccountsWarning, AdminFailure, LoadedAccounts};
use crate::events::Event;

/// Records operation log events (ops-info-v1). Recording does not fail for
/// the caller: a sink that cannot write drops the line (ops-info-v1 §1.1),
/// so a log failure does not change the flow of a use case.
pub trait EventSink: Send + Sync {
    fn record(&self, event: &Event);
}

/// The administrator account file (cli-v1 §1.3, §1.6).
pub trait AccountStore {
    /// Reads and checks the file. `Ok(None)` when the file does not exist.
    fn read(&self) -> Result<Option<LoadedAccounts>, Vec<AccountsDiagnostic>>;

    /// Changes the file in one locked step: takes the lock, reads the file
    /// again (a missing file is an empty list), applies `change`, and
    /// replaces the file with the result. A `change` failure leaves the file
    /// as it was. Steps that fail after the replacement, and a lock that
    /// cannot be removed, go to `warn`.
    fn update(
        &self,
        change: &mut dyn FnMut(Vec<Account>) -> Result<Vec<Account>, AdminFailure>,
        warn: &mut dyn FnMut(AccountsWarning),
    ) -> Result<(), AdminFailure>;
}

/// Operator input of the `admin` commands (cli-v1 §1.5).
pub trait AccountInput {
    /// Writes `prompt` and reads one line without showing the typed text.
    fn hidden(&self, prompt: &str) -> Result<String, InputError>;

    /// Writes `prompt` and reads one visible line. `Ok(None)` at the end of
    /// input.
    fn line(&self, prompt: &str) -> Result<Option<String>, InputError>;

    /// The operator pressed Ctrl+C after the input was opened.
    fn canceled(&self) -> bool;
}

/// Why operator input gave no text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputError {
    /// Standard input is not a terminal.
    NotInteractive,
    /// Ctrl+C, or the end of input before any text.
    Canceled,
    /// The typed bytes are not UTF-8.
    InvalidText,
    /// Input cannot be read at all. The message says why (`runtime_unavailable`).
    Unavailable(String),
}

/// Makes password hashes (cli-v1 §1.4).
pub trait PasswordHasher {
    /// An Argon2id hash of `password` with a new salt and `params`.
    fn hash(&self, password: &Password, params: Argon2Params) -> Result<PasswordHash, HashError>;
}

/// Hashing failed: the OS random source or the hash memory was not
/// available. `0` is a Korean sentence without secret values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashError(pub String);

impl fmt::Display for HashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for HashError {}

/// Time for administrator sessions and the login limit. Deadlines use
/// `monotonic`. `wall` is only used to report expiry times. The app never
/// calls `Instant::now` or `SystemTime::now` (`crates/app/clippy.toml`).
pub trait Clock: Send + Sync {
    /// Time since a fixed start point. It never goes backwards.
    fn monotonic(&self) -> Duration;

    /// Wall-clock time for the expiry values in the session response.
    fn wall(&self) -> SystemTime;
}

/// Checks a typed password against a stored hash (management-api-v1 §1.5).
pub trait PasswordVerifier: Send + Sync {
    /// `Ok(false)` for a mismatch. `Err` only when the check itself failed.
    fn verify(&self, typed: &str, hash: &PasswordHash) -> Result<bool, HashError>;
}

/// Makes admin session tokens and compares them (management-api-v1 §1.4.3).
pub trait TokenSource: Send + Sync {
    /// A new random token of 64 lowercase hex digits.
    fn new_token(&self) -> Result<String, RandomError>;

    /// True when both texts are equal. The comparison takes the same time
    /// whatever the first difference is.
    fn same_token(&self, expected: &str, given: &str) -> bool;
}

/// The OS random source could not give bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RandomError(pub String);

impl fmt::Display for RandomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for RandomError {}

/// Lifecycle input shared by relay stages (cli-v1 §1.8).
/// Initial exchange, target connection, and waiting stop at the first Grace.
/// Active sessions may finish normally until its absolute monotonic deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayLifecycle {
    Running,
    Grace { deadline: Duration },
    Force,
}

/// Transport close intent; adapters map it without exposing transport libraries.
/// See relay-session §3.2 and websocket-relay-v1 §1.4–1.6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayCloseAction {
    WriteHalf,
    HandshakeInvalid,
    HandshakeTimeout,
    ModeDisabled,
    RejectDuplicate,
    WaitingExpired,
    WaitingFull,
    TargetConnectFailed,
    TargetNotAllowed,
    TargetConnectionLimit,
    PeerEof,
    StreamError(crate::events::ConnectionErrorKind),
    DeadConnection,
    AdminTerminated,
    ServerShutdown,
}

/// Bounded payload stream with explicit half-close and terminal-close intent.
/// An empty payload message is not EOF. Successful writes count only payload
/// accepted by the transport, not bytes copied into an unbounded queue.
/// See relay spec RLY-003, RLY-019 and websocket-relay-v1 §1.4–1.6.
pub trait RelayStream: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send {
    /// The viewer transport metadata used in session views and events.
    fn transport(&self) -> vnc_relay_domain::session::ViewerTransport;

    /// The remaining payload length of an outbound frame whose header is
    /// already written; 0 for TCP.
    fn pending_write_len(&self) -> usize;

    /// Drives transport liveness without consuming payload or bypassing B.
    /// Idle maintenance registers a wake and stays Pending; TCP has no work.
    fn poll_maintenance(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Pending
    }

    /// Drives one idempotent close action; terminal completion does not replace
    /// dropping the stream before its L permit (relay-session §6).
    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        action: RelayCloseAction,
    ) -> std::task::Poll<std::io::Result<()>>;
}

/// One connection's type erasure, never a per-chunk allocation.
/// See relay-session §3.2.
pub type BoxRelayStream = Box<dyn RelayStream>;

/// Typed transport failure carried inside the standard AsyncRead/AsyncWrite
/// I/O error. Adapters retain WS protocol/text/size failures without exposing
/// their library types (websocket-relay-v1 §2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayStreamError {
    Transport(crate::events::ConnectionErrorKind),
    /// A transport liveness deadline expired, not an ordinary I/O failure.
    DeadConnection,
}

impl RelayStreamError {
    /// Wraps one contract kind without payload, raw IDs, or library details.
    pub fn new(kind: crate::events::ConnectionErrorKind) -> Self {
        Self::Transport(kind)
    }

    /// The original contract kind, suitable for events and close actions.
    pub fn kind(self) -> crate::events::ConnectionErrorKind {
        match self {
            Self::Transport(kind) => kind,
            Self::DeadConnection => crate::events::ConnectionErrorKind::IoError,
        }
    }

    /// Preserves the kind through the standard async I/O trait error channel.
    pub fn into_io_error(self) -> std::io::Error {
        std::io::Error::other(self)
    }

    /// Keeps a liveness verdict separate from the connection_error taxonomy.
    pub fn is_dead_connection(error: &std::io::Error) -> bool {
        error
            .get_ref()
            .and_then(|inner| inner.downcast_ref::<Self>())
            == Some(&Self::DeadConnection)
    }

    /// Reads adapter-provided kinds; untyped socket errors are io_error.
    pub fn classify(error: &std::io::Error) -> crate::events::ConnectionErrorKind {
        error
            .get_ref()
            .and_then(|inner| inner.downcast_ref::<Self>())
            .map_or(crate::events::ConnectionErrorKind::IoError, |typed| {
                typed.kind()
            })
    }
}

impl fmt::Display for RelayStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeadConnection => f.write_str("dead_connection"),
            Self::Transport(kind) => f.write_str(kind.as_str()),
        }
    }
}

impl std::error::Error for RelayStreamError {}

/// DNS and single-address connection failures (ops-info-v1 §1.6).
/// The app owns the absolute resolve-plus-connect deadline C.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetError {
    ResolveFailed,
    ConnectRefused,
    ConnectError,
    /// K could not be applied. Terminal for the whole target request, but the
    /// external reason remains connect_error (relay spec RLY-R18).
    KeepaliveFailed,
}

impl fmt::Display for TargetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ResolveFailed => "resolve_failed",
            Self::ConnectRefused => "connect_refused",
            Self::ConnectError | Self::KeepaliveFailed => "connect_error",
        })
    }
}

impl std::error::Error for TargetError {}

/// Resolves once and connects only to an already policy-checked socket address.
/// The adapter captures current K at each socket dial start; it must not retain
/// startup K or change an established stream's options. Dropping a connect
/// future closes its in-progress socket. KeepaliveFailed aborts the request.
pub trait TargetConnector: Send + Sync {
    type Stream: RelayStream + 'static;

    fn resolve(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<Vec<std::net::IpAddr>, TargetError>> + Send;

    fn connect(
        &self,
        address: std::net::SocketAddr,
    ) -> impl Future<Output = Result<Self::Stream, TargetError>> + Send;
}

/// Computes a log-only keyed reference; never use it as a matching key.
/// See ops-info-v1 §1.9.
pub trait RelayIdSource: Send + Sync {
    fn reference(&self, id: &vnc_relay_domain::relay_id::RelayId) -> crate::events::RelayIdRef;
}

/// Applies the per-process keyed permutation to one never-reused counter.
/// No runtime randomness, failure path, or issued-ID set (relay-session §3.3).
pub trait SessionIdSource: Send + Sync {
    fn session_id(&self, counter: u128) -> vnc_relay_domain::session::SessionId;
}

/// Accepts a single-session cleanup command without borrowing relay streams.
pub trait RelaySessionControl: Send + Sync {
    fn request_terminate(
        &self,
        session_id: &str,
        account: vnc_relay_domain::account::AccountName,
        peer: vnc_relay_domain::peer_ip::PeerIp,
        clock: &dyn Clock,
        events: &dyn EventSink,
    ) -> Option<crate::management::terminate::TerminationTicket>;
}

/// Provides a snapshot of relay runtime state for management queries.
pub trait RelayStateSource: Send + Sync {
    fn snapshot(&self, clock: &dyn Clock) -> crate::relay::RelaySnapshot;
}

#[cfg(test)]
mod relay_stream_tests {
    use super::RelayStreamError;
    use crate::events::ConnectionErrorKind;

    /// spec: websocket.md:WS-019 partial
    /// spec: websocket.md:WS-008 partial
    /// spec: websocket.md:WS-010 partial
    #[test]
    fn preserves_typed_transport_error_kinds_in_the_io_error_channel() {
        for kind in [
            ConnectionErrorKind::IoError,
            ConnectionErrorKind::TlsFailed,
            ConnectionErrorKind::WsProtocolError,
            ConnectionErrorKind::WsTextMessage,
            ConnectionErrorKind::WsMessageTooLarge,
        ] {
            let error = RelayStreamError::new(kind).into_io_error();
            assert_eq!(RelayStreamError::classify(&error), kind);
            assert_eq!(error.to_string(), kind.as_str());
        }
    }

    #[test]
    fn classifies_untyped_socket_errors_without_exposing_their_details() {
        let error = std::io::Error::new(std::io::ErrorKind::ConnectionReset, "private detail");
        assert_eq!(
            RelayStreamError::classify(&error),
            ConnectionErrorKind::IoError
        );
    }
}
