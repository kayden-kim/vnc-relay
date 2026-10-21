//! Login failure limit (management.md §3.0, management-api-v1 §1.5).
//!
//! Failures are counted per account name and per source IP. The failure
//! that goes over F starts a lock for `lockout`, and the key's failure
//! history is cleared. A locked attempt is not counted and does not move
//! the lock. A success does not reset the counts.

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::time::Duration;

use crate::peer_ip::PeerIp;

/// The limit values of config-v1 §1.2 `[management.login_limit]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoginLimitPolicy {
    per_account: usize,
    per_source: usize,
    window: Duration,
    lockout: Duration,
}

impl LoginLimitPolicy {
    /// `per_account` and `per_source` are F. A count above F starts a lock.
    pub fn new(per_account: u32, per_source: u32, window: Duration, lockout: Duration) -> Self {
        Self {
            per_account: usize::try_from(per_account).unwrap_or(usize::MAX),
            per_source: usize::try_from(per_source).unwrap_or(usize::MAX),
            window,
            lockout,
        }
    }
}

/// The key that a lock applies to. `ops-info-v1` §1.4 `scope` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitScope {
    Account,
    Source,
}

impl LimitScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Account => "account",
            Self::Source => "source",
        }
    }
}

/// The failure history and the lock of one key.
#[derive(Debug, Default)]
struct KeyState {
    failures: VecDeque<Duration>,
    locked_until: Option<Duration>,
}

impl KeyState {
    fn is_locked(&self, now: Duration) -> bool {
        self.locked_until.is_some_and(|until| now < until)
    }

    /// Clears an ended lock and drops failures older than the window.
    fn settle(&mut self, now: Duration, window: Duration) {
        if self.locked_until.is_some_and(|until| until <= now) {
            self.locked_until = None;
        }
        while let Some(&at) = self.failures.front() {
            if now.saturating_sub(at) < window {
                break;
            }
            self.failures.pop_front();
        }
    }

    fn is_empty(&self) -> bool {
        self.failures.is_empty() && self.locked_until.is_none()
    }
}

/// Counts one failure on `state`. Returns true when this failure starts a lock.
fn count_failure(
    state: &mut KeyState,
    now: Duration,
    window: Duration,
    limit: usize,
    lockout: Duration,
) -> bool {
    state.settle(now, window);
    if state.is_locked(now) {
        return false;
    }
    state.failures.push_back(now);
    if state.failures.len() <= limit {
        return false;
    }
    state.failures.clear();
    state.locked_until = Some(now.saturating_add(lockout));
    true
}

fn prune_keys<K: Eq + Hash>(keys: &mut HashMap<K, KeyState>, now: Duration, window: Duration) {
    keys.retain(|_, state| {
        state.settle(now, window);
        !state.is_empty()
    });
}

/// The login limit state of this process. It is kept in memory only.
#[derive(Debug)]
pub struct LoginLimiter {
    policy: LoginLimitPolicy,
    accounts: HashMap<String, KeyState>,
    sources: HashMap<PeerIp, KeyState>,
}

impl LoginLimiter {
    pub fn new(policy: LoginLimitPolicy) -> Self {
        Self {
            policy,
            accounts: HashMap::new(),
            sources: HashMap::new(),
        }
    }

    /// The length of one lock.
    pub fn lockout(&self) -> Duration {
        self.policy.lockout
    }

    /// Drops ended locks and old failures. A key with nothing left is removed.
    pub fn prune(&mut self, now: Duration) {
        let window = self.policy.window;
        prune_keys(&mut self.accounts, now, window);
        prune_keys(&mut self.sources, now, window);
    }

    /// The locked scope at `now`, if any. The account key is checked first.
    /// `account` is the typed name, which may not be a real account.
    pub fn check(&self, account: &str, source: PeerIp, now: Duration) -> Option<LimitScope> {
        if self
            .accounts
            .get(account)
            .is_some_and(|state| state.is_locked(now))
        {
            return Some(LimitScope::Account);
        }
        if self
            .sources
            .get(&source)
            .is_some_and(|state| state.is_locked(now))
        {
            return Some(LimitScope::Source);
        }
        None
    }

    /// Counts one failed attempt on the account key and the source key.
    /// Returns the scopes whose lock starts with this failure.
    pub fn record_failure(
        &mut self,
        account: &str,
        source: PeerIp,
        now: Duration,
    ) -> Vec<LimitScope> {
        let window = self.policy.window;
        let lockout = self.policy.lockout;
        let mut started = Vec::new();
        let account_state = self.accounts.entry(account.to_owned()).or_default();
        if count_failure(account_state, now, window, self.policy.per_account, lockout) {
            started.push(LimitScope::Account);
        }
        let source_state = self.sources.entry(source).or_default();
        if count_failure(source_state, now, window, self.policy.per_source, lockout) {
            started.push(LimitScope::Source);
        }
        started
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use super::*;

    fn source(last: u8) -> PeerIp {
        PeerIp::new(IpAddr::V4(Ipv4Addr::new(192, 0, 2, last)))
    }

    fn secs(value: u64) -> Duration {
        Duration::from_secs(value)
    }

    /// spec: management.md:MGT-022 partial
    #[test]
    fn locks_on_the_failure_after_f_and_does_not_count_locked_attempts() {
        let policy = LoginLimitPolicy::new(2, 100, secs(900), secs(60));
        let mut limiter = LoginLimiter::new(policy);
        assert!(
            limiter
                .record_failure("admin", source(1), secs(0))
                .is_empty()
        );
        assert!(
            limiter
                .record_failure("admin", source(1), secs(1))
                .is_empty()
        );
        let started = limiter.record_failure("admin", source(1), secs(2));
        assert_eq!(started, vec![LimitScope::Account]);
        assert_eq!(
            limiter.check("admin", source(1), secs(3)),
            Some(LimitScope::Account)
        );

        // A locked attempt is not counted, so it does not move the lock end.
        assert!(
            limiter
                .record_failure("admin", source(1), secs(30))
                .is_empty()
        );
        assert_eq!(
            limiter.check("admin", source(1), secs(61)),
            Some(LimitScope::Account)
        );
        assert_eq!(limiter.check("admin", source(1), secs(62)), None);

        // After the lock ends, the history is clear: one failure does not lock.
        assert!(
            limiter
                .record_failure("admin", source(1), secs(62))
                .is_empty()
        );
        assert_eq!(limiter.check("admin", source(1), secs(63)), None);
    }

    #[test]
    fn failures_older_than_the_window_do_not_count() {
        let policy = LoginLimitPolicy::new(2, 100, secs(100), secs(60));
        let mut limiter = LoginLimiter::new(policy);
        assert!(
            limiter
                .record_failure("admin", source(1), secs(0))
                .is_empty()
        );
        assert!(
            limiter
                .record_failure("admin", source(1), secs(50))
                .is_empty()
        );
        // The first failure is 100 s old now, so it no longer counts.
        assert!(
            limiter
                .record_failure("admin", source(1), secs(100))
                .is_empty()
        );
        assert_eq!(limiter.check("admin", source(1), secs(100)), None);
    }

    /// spec: management.md:MGT-022 partial
    #[test]
    fn same_source_is_locked_for_other_accounts_too() {
        let policy = LoginLimitPolicy::new(100, 1, secs(900), secs(60));
        let mut limiter = LoginLimiter::new(policy);
        assert!(
            limiter
                .record_failure("alice", source(7), secs(0))
                .is_empty()
        );
        let started = limiter.record_failure("bob", source(7), secs(1));
        assert_eq!(started, vec![LimitScope::Source]);
        assert_eq!(
            limiter.check("carol", source(7), secs(2)),
            Some(LimitScope::Source)
        );
        assert_eq!(limiter.check("carol", source(8), secs(2)), None);
    }

    #[test]
    fn prune_removes_keys_with_nothing_left() {
        let policy = LoginLimitPolicy::new(5, 5, secs(100), secs(60));
        let mut limiter = LoginLimiter::new(policy);
        limiter.record_failure("gone", source(3), secs(0));
        limiter.prune(secs(100));
        assert!(limiter.accounts.is_empty());
        assert!(limiter.sources.is_empty());
    }
}
