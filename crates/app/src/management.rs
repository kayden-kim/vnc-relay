//! Administrator login and sessions (management-api-v1 §1.4, §1.5).
//!
//! `AdminAuth` keeps the admin sessions and the login limit in memory.
//! The adapter calls it for `POST /api/v1/session` and for every other
//! `/api/v1/` request. The state lock covers only table and limiter
//! access. The password check and the event writes run outside the lock.

pub mod login;
pub mod query;
pub mod session;
pub mod terminate;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::SystemTime;

use vnc_relay_domain::account::{Account, AccountName, PasswordHash};
use vnc_relay_domain::admin_session::{
    AdminCsrfToken, AdminSession, AdminSessionPolicy, AdminSessionTable,
};
use vnc_relay_domain::login_limit::{LoginLimitPolicy, LoginLimiter};

use crate::events::Event;
use crate::ports::{Clock, EventSink, PasswordVerifier, TokenSource};

/// The ports that `AdminAuth` uses.
pub struct AdminPorts {
    pub verifier: Arc<dyn PasswordVerifier>,
    pub tokens: Arc<dyn TokenSource>,
    pub clock: Arc<dyn Clock>,
    pub events: Arc<dyn EventSink>,
}

/// Why an administrator request is refused. The adapter maps each value to a
/// response (management-api-v1 §1.4, §2.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    /// No live session for the cookie. Maps to `401 unauthenticated`.
    Unauthenticated,
    /// CSRF token missing or wrong. Maps to `403 csrf_invalid`.
    CsrfInvalid,
    /// Login refused for any reason. Maps to `401 login_failed`.
    LoginFailed,
    /// The account name is outside the account grammar. Maps to
    /// `400 invalid_request`. It is refused before any password check and is
    /// not counted against the login limit (구현 판단(사용자 결정 없음)).
    InvalidAccount,
    /// A port failed. Maps to `500 internal`. The text is for diagnosis only
    /// and never goes to a client.
    Internal(String),
}

/// The session values that a response shows (management-api-v1 §1.5).
#[derive(Debug, Clone)]
pub struct SessionView {
    pub account: AccountName,
    pub csrf: AdminCsrfToken,
    pub idle_expires_at: SystemTime,
    pub absolute_expires_at: SystemTime,
    /// The wall-clock time of the view. Only GET responses show it
    /// (management-api-v1 §1.1).
    pub as_of: SystemTime,
}

/// Session table and login limit, guarded by one lock.
struct AdminState {
    sessions: AdminSessionTable,
    limiter: LoginLimiter,
}

/// Administrator login, sessions and login limit. One instance lives per
/// process. A restart drops every session (MGT-R18).
pub struct AdminAuth {
    /// Real accounts by name. The unknown-account path uses `dummy` instead.
    accounts: HashMap<AccountName, PasswordHash>,
    dummy: PasswordHash,
    verifier: Arc<dyn PasswordVerifier>,
    tokens: Arc<dyn TokenSource>,
    clock: Arc<dyn Clock>,
    events: Arc<dyn EventSink>,
    session_policy: AdminSessionPolicy,
    state: Mutex<AdminState>,
}

impl AdminAuth {
    /// `dummy` is the hash of the dummy password (see `login::dummy_hash`).
    /// The session policy and the limit policy come from config-v2 §1.2.
    pub fn new(
        accounts: Vec<Account>,
        dummy: PasswordHash,
        ports: AdminPorts,
        session_policy: AdminSessionPolicy,
        limit_policy: LoginLimitPolicy,
    ) -> Self {
        let AdminPorts {
            verifier,
            tokens,
            clock,
            events,
        } = ports;
        Self {
            accounts: accounts
                .into_iter()
                .map(|account| (account.name, account.password_hash))
                .collect(),
            dummy,
            verifier,
            tokens,
            clock,
            events,
            session_policy,
            state: Mutex::new(AdminState {
                sessions: AdminSessionTable::default(),
                limiter: LoginLimiter::new(limit_policy),
            }),
        }
    }

    fn lock(&self) -> MutexGuard<'_, AdminState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn record(&self, event: Event) {
        self.events.record(&event);
    }
}

/// The session values for a response. The two deadlines are the wall-clock
/// values fixed when the session was issued or extended, so later clock
/// readings do not move them.
fn session_view(session: &AdminSession, wall: SystemTime) -> SessionView {
    SessionView {
        account: session.account().clone(),
        csrf: session.csrf().clone(),
        idle_expires_at: session.idle_wall(),
        absolute_expires_at: session.absolute_wall(),
        as_of: wall,
    }
}

#[cfg(test)]
mod test_support {
    use std::collections::HashMap;
    use std::net::{IpAddr, Ipv4Addr};
    use std::sync::{Arc, Mutex, PoisonError};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use vnc_relay_domain::account::{Account, AccountName, PasswordHash};
    use vnc_relay_domain::admin_session::AdminSessionPolicy;
    use vnc_relay_domain::login_limit::LoginLimitPolicy;
    use vnc_relay_domain::peer_ip::PeerIp;

    use super::{AdminAuth, AdminPorts};
    use crate::events::Event;
    use crate::ports::{Clock, EventSink, HashError, PasswordVerifier, RandomError, TokenSource};

    pub const PASSWORD: &str = "correct-horse-battery";
    pub const OTHER_PASSWORD: &str = "another-long-password";
    const SALT: &str = "c2FsdHNhbHRzYWx0c2FsdA";

    /// A PHC hash string with a 43-character hash made of `fill`. The last
    /// character of a 43-character hash must have zero unused bits, so `fill`
    /// must be `A`, `E` or `I`.
    pub fn hash_text(fill: char) -> String {
        format!(
            "$argon2id$v=19$m=19456,t=2,p=1${SALT}${}",
            fill.to_string().repeat(43)
        )
    }

    pub fn secs(value: u64) -> Duration {
        Duration::from_secs(value)
    }

    /// The wall-clock time at monotonic zero in the fake clock.
    pub fn base_wall() -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(1_700_000_000)
    }

    pub fn peer(last: u8) -> PeerIp {
        PeerIp::new(IpAddr::V4(Ipv4Addr::new(192, 0, 2, last)))
    }

    pub fn session_policy() -> AdminSessionPolicy {
        AdminSessionPolicy::new(secs(300), secs(3600))
    }

    pub fn limit_policy() -> LoginLimitPolicy {
        LoginLimitPolicy::new(5, 20, secs(900), secs(900))
    }

    pub fn account(name: &str, fill: char) -> Account {
        Account {
            name: AccountName::parse(name).unwrap_or_else(|error| panic!("{error}")),
            password_hash: PasswordHash::parse(&hash_text(fill))
                .unwrap_or_else(|error| panic!("{error}")),
        }
    }

    /// A clock that moves only when a test sets it. `set` moves the monotonic
    /// and wall clocks together. `set_wall` moves the wall clock alone, as a
    /// clock correction does.
    pub struct FakeClock {
        now: Mutex<Duration>,
        wall: Mutex<SystemTime>,
    }

    impl Default for FakeClock {
        fn default() -> Self {
            Self {
                now: Mutex::new(Duration::ZERO),
                wall: Mutex::new(base_wall()),
            }
        }
    }

    impl FakeClock {
        pub fn set(&self, now: Duration) {
            *self.now.lock().unwrap_or_else(PoisonError::into_inner) = now;
            *self.wall.lock().unwrap_or_else(PoisonError::into_inner) = base_wall() + now;
        }

        pub fn set_wall(&self, wall: SystemTime) {
            *self.wall.lock().unwrap_or_else(PoisonError::into_inner) = wall;
        }
    }

    impl Clock for FakeClock {
        fn monotonic(&self) -> Duration {
            *self.now.lock().unwrap_or_else(PoisonError::into_inner)
        }

        fn wall(&self) -> SystemTime {
            *self.wall.lock().unwrap_or_else(PoisonError::into_inner)
        }
    }

    /// Maps a hash string to its password and counts the checks.
    pub struct FakeVerifier {
        pub passwords: HashMap<String, String>,
        pub calls: Mutex<usize>,
    }

    impl FakeVerifier {
        pub fn calls(&self) -> usize {
            *self.calls.lock().unwrap_or_else(PoisonError::into_inner)
        }
    }

    impl PasswordVerifier for FakeVerifier {
        fn verify(&self, typed: &str, hash: &PasswordHash) -> Result<bool, HashError> {
            *self.calls.lock().unwrap_or_else(PoisonError::into_inner) += 1;
            Ok(self
                .passwords
                .get(hash.as_str())
                .is_some_and(|password| password == typed))
        }
    }

    /// Hands out 64 lowercase hex digits, counting up from 1.
    #[derive(Default)]
    pub struct FakeTokens {
        next: Mutex<u64>,
    }

    impl TokenSource for FakeTokens {
        fn new_token(&self) -> Result<String, RandomError> {
            let mut next = self.next.lock().unwrap_or_else(PoisonError::into_inner);
            *next += 1;
            Ok(format!("{:064x}", *next))
        }

        fn same_token(&self, expected: &str, given: &str) -> bool {
            expected == given
        }
    }

    #[derive(Default)]
    pub struct FakeEvents {
        events: Mutex<Vec<Event>>,
    }

    impl FakeEvents {
        pub fn all(&self) -> Vec<Event> {
            self.events
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }
    }

    impl EventSink for FakeEvents {
        fn record(&self, event: &Event) {
            self.events
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(event.clone());
        }
    }

    pub struct Harness {
        pub auth: AdminAuth,
        pub clock: Arc<FakeClock>,
        pub verifier: Arc<FakeVerifier>,
        pub events: Arc<FakeEvents>,
    }

    /// Two accounts: `alice` (password `PASSWORD`) and `bob` (`OTHER_PASSWORD`).
    pub fn harness() -> Harness {
        harness_with(session_policy(), limit_policy())
    }

    pub fn harness_with(session: AdminSessionPolicy, limit: LoginLimitPolicy) -> Harness {
        let clock = Arc::new(FakeClock::default());
        let verifier = Arc::new(FakeVerifier {
            passwords: HashMap::from([
                (hash_text('A'), PASSWORD.to_owned()),
                (hash_text('E'), OTHER_PASSWORD.to_owned()),
            ]),
            calls: Mutex::new(0),
        });
        let events = Arc::new(FakeEvents::default());
        let ports = AdminPorts {
            verifier: verifier.clone(),
            tokens: Arc::new(FakeTokens::default()),
            clock: clock.clone(),
            events: events.clone(),
        };
        let accounts = vec![account("alice", 'A'), account("bob", 'E')];
        let dummy = PasswordHash::parse(&hash_text('I')).unwrap_or_else(|error| panic!("{error}"));
        let auth = AdminAuth::new(accounts, dummy, ports, session, limit);
        Harness {
            auth,
            clock,
            verifier,
            events,
        }
    }
}
