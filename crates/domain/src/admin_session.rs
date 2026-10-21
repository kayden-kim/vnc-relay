//! Administrator session values (management-api-v1 §1.4, management.md §3.0).
//!
//! A session lives in server memory only. Its token and its CSRF token are
//! 32 random bytes written as 64 lowercase hex digits (구현 판단(사용자 결정 없음)).
//! Both deadlines are on the monotonic clock. The session is live while the
//! current time is before both of them.

use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use crate::account::AccountName;

/// Length of a session token and of a CSRF token in characters.
pub const TOKEN_LEN: usize = 64;

/// The text is not a token of the form this module writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenError;

impl fmt::Display for TokenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("토큰은 소문자 16진수 64자여야 한다")
    }
}

impl std::error::Error for TokenError {}

fn check_token(text: &str) -> Result<(), TokenError> {
    let is_lower_hex = |byte: u8| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte);
    if text.len() == TOKEN_LEN && text.bytes().all(is_lower_hex) {
        Ok(())
    } else {
        Err(TokenError)
    }
}

/// The value of the session cookie. It is the key of the session table.
/// `Debug` hides the value.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AdminSessionToken(String);

impl AdminSessionToken {
    pub fn parse(text: &str) -> Result<Self, TokenError> {
        check_token(text)?;
        Ok(Self(text.to_owned()))
    }

    /// The cookie value. Never write it to a log or to a response body.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for AdminSessionToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AdminSessionToken(<redacted>)")
    }
}

/// The CSRF token of one session (management-api-v1 §1.4.3). It is the same
/// for the whole session. `Debug` hides the value.
#[derive(Clone)]
pub struct AdminCsrfToken(String);

impl AdminCsrfToken {
    pub fn parse(text: &str) -> Result<Self, TokenError> {
        check_token(text)?;
        Ok(Self(text.to_owned()))
    }

    /// The token value. Never write it to a log or to a URL.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for AdminCsrfToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AdminCsrfToken(<redacted>)")
    }
}

/// Lifetimes of administrator sessions: Si (`idle`) and Sa (`max_lifetime`)
/// of config-v1 §1.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdminSessionPolicy {
    idle: Duration,
    max_lifetime: Duration,
}

impl AdminSessionPolicy {
    /// The config check keeps `idle` at or below `max_lifetime`.
    pub fn new(idle: Duration, max_lifetime: Duration) -> Self {
        Self { idle, max_lifetime }
    }
}

/// One administrator session. The account name and the CSRF token are fixed
/// for the whole session.
#[derive(Debug, Clone)]
pub struct AdminSession {
    account: AccountName,
    csrf: AdminCsrfToken,
    idle_deadline: Duration,
    absolute_deadline: Duration,
}

impl AdminSession {
    /// A new session at `now`. The absolute deadline is `now + Sa`. The idle
    /// deadline is `now + Si`, never after the absolute deadline.
    pub fn start(
        account: AccountName,
        csrf: AdminCsrfToken,
        now: Duration,
        policy: AdminSessionPolicy,
    ) -> Self {
        let absolute_deadline = now.saturating_add(policy.max_lifetime);
        Self {
            account,
            csrf,
            idle_deadline: idle_deadline_from(now, absolute_deadline, policy),
            absolute_deadline,
        }
    }

    /// A user action at `now` moves the idle deadline to `now + Si`, but not
    /// past the absolute deadline. The absolute deadline never moves.
    pub fn extend_idle(&mut self, now: Duration, policy: AdminSessionPolicy) {
        self.idle_deadline = idle_deadline_from(now, self.absolute_deadline, policy);
    }

    /// True while `now` is before both deadlines.
    pub fn is_live(&self, now: Duration) -> bool {
        now < self.idle_deadline && now < self.absolute_deadline
    }

    pub fn account(&self) -> &AccountName {
        &self.account
    }

    pub fn csrf(&self) -> &AdminCsrfToken {
        &self.csrf
    }

    pub fn idle_deadline(&self) -> Duration {
        self.idle_deadline
    }

    pub fn absolute_deadline(&self) -> Duration {
        self.absolute_deadline
    }
}

fn idle_deadline_from(now: Duration, absolute: Duration, policy: AdminSessionPolicy) -> Duration {
    now.saturating_add(policy.idle).min(absolute)
}

/// The administrator sessions of this process. A restart empties it
/// (cli-v1 §1.7, management.md MGT-R18).
#[derive(Debug, Default)]
pub struct AdminSessionTable {
    sessions: HashMap<AdminSessionToken, AdminSession>,
}

impl AdminSessionTable {
    pub fn insert(&mut self, token: AdminSessionToken, session: AdminSession) {
        self.sessions.insert(token, session);
    }

    pub fn get(&self, token: &AdminSessionToken) -> Option<&AdminSession> {
        self.sessions.get(token)
    }

    pub fn get_mut(&mut self, token: &AdminSessionToken) -> Option<&mut AdminSession> {
        self.sessions.get_mut(token)
    }

    pub fn remove(&mut self, token: &AdminSessionToken) -> Option<AdminSession> {
        self.sessions.remove(token)
    }

    /// Drops every session that is not live at `now`.
    pub fn prune(&mut self, now: Duration) {
        self.sessions.retain(|_, session| session.is_live(now));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn account() -> AccountName {
        AccountName::parse("admin").unwrap_or_else(|error| panic!("{error:?}"))
    }

    fn csrf() -> AdminCsrfToken {
        AdminCsrfToken::parse(TOKEN).unwrap_or_else(|error| panic!("{error}"))
    }

    fn policy(idle: u64, max: u64) -> AdminSessionPolicy {
        AdminSessionPolicy::new(Duration::from_secs(idle), Duration::from_secs(max))
    }

    #[test]
    fn accepts_only_64_lowercase_hex_digits() {
        assert!(AdminSessionToken::parse(TOKEN).is_ok());
        assert!(AdminSessionToken::parse(&TOKEN.to_uppercase()).is_err());
        assert!(AdminSessionToken::parse(&TOKEN[1..]).is_err());
        assert!(AdminSessionToken::parse(&format!("{TOKEN}0")).is_err());
        assert!(AdminCsrfToken::parse("g".repeat(64).as_str()).is_err());
    }

    #[test]
    fn debug_output_hides_the_token_value() {
        let token = AdminSessionToken::parse(TOKEN).unwrap_or_else(|error| panic!("{error}"));
        assert!(!format!("{token:?}").contains(TOKEN));
        assert!(!format!("{:?}", csrf()).contains(TOKEN));
    }

    /// spec: management.md:MGT-025 partial
    #[test]
    fn idle_deadline_moves_only_on_extension_and_never_past_the_absolute_one() {
        let policy = policy(300, 1000);
        let mut session = AdminSession::start(account(), csrf(), Duration::ZERO, policy);
        assert!(session.is_live(Duration::from_secs(299)));
        assert!(!session.is_live(Duration::from_secs(300)));

        session.extend_idle(Duration::from_secs(200), policy);
        assert!(session.is_live(Duration::from_secs(499)));
        assert!(!session.is_live(Duration::from_secs(500)));

        session.extend_idle(Duration::from_secs(900), policy);
        assert_eq!(session.idle_deadline(), Duration::from_secs(1000));
        assert!(!session.is_live(Duration::from_secs(1000)));
    }

    /// spec: management.md:MGT-025 partial
    #[test]
    fn absolute_deadline_is_not_moved_by_user_actions() {
        let policy = policy(300, 600);
        let mut session = AdminSession::start(account(), csrf(), Duration::ZERO, policy);
        session.extend_idle(Duration::from_secs(550), policy);
        assert_eq!(session.absolute_deadline(), Duration::from_secs(600));
        assert!(session.is_live(Duration::from_secs(599)));
        assert!(!session.is_live(Duration::from_secs(600)));
    }

    #[test]
    fn prune_keeps_only_live_sessions() {
        let policy = policy(300, 3600);
        let mut table = AdminSessionTable::default();
        let token = AdminSessionToken::parse(TOKEN).unwrap_or_else(|error| panic!("{error}"));
        table.insert(
            token.clone(),
            AdminSession::start(account(), csrf(), Duration::ZERO, policy),
        );
        table.prune(Duration::from_secs(300));
        assert!(table.get(&token).is_none());
    }
}
