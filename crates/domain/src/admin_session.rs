//! Administrator session values (management-api-v1 §1.4, management.md §3.0).
//!
//! A session lives in server memory only. Its token and its CSRF token are
//! 32 random bytes written as 64 lowercase hex digits (구현 판단(사용자 결정 없음)).
//! Both deadlines are on the monotonic clock. The session is live while the
//! current time is before both of them.

use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, SystemTime};

use crate::account::Name;

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
pub struct Token(String);

impl Token {
    pub fn parse(text: &str) -> Result<Self, TokenError> {
        check_token(text)?;
        Ok(Self(text.to_owned()))
    }

    /// The cookie value. Never write it to a log or to a response body.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Token(<redacted>)")
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
/// of config-v3 §1.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    idle: Duration,
    max_lifetime: Duration,
}

impl Policy {
    /// The config check keeps `idle` at or below `max_lifetime`.
    pub fn new(idle: Duration, max_lifetime: Duration) -> Self {
        Self { idle, max_lifetime }
    }
}

/// One administrator session. The account name and the CSRF token are fixed
/// for the whole session.
///
/// Each deadline is kept twice. The monotonic pair decides whether the
/// session is live. The wall-clock pair is the value that responses show. It
/// is fixed when the session is issued or extended, so a later wall-clock jump
/// does not change it (구현 판단(사용자 결정 없음)).
#[derive(Debug, Clone)]
pub struct AdminSession {
    account: Name,
    csrf: AdminCsrfToken,
    idle_deadline: Duration,
    absolute_deadline: Duration,
    idle_wall: SystemTime,
    absolute_wall: SystemTime,
}

impl AdminSession {
    /// A new session at monotonic time `now` and wall-clock time `wall`. The
    /// absolute deadline is `now + Sa`. The idle deadline is `now + Si`, never
    /// after the absolute deadline. The wall-clock pair follows the same rules.
    pub fn start(
        account: Name,
        csrf: AdminCsrfToken,
        now: Duration,
        wall: SystemTime,
        policy: Policy,
    ) -> Self {
        let absolute_deadline = now.saturating_add(policy.max_lifetime);
        let absolute_wall = wall.checked_add(policy.max_lifetime).unwrap_or(wall);
        let (idle_deadline, idle_wall) =
            idle_deadlines(now, wall, absolute_deadline, absolute_wall, policy);
        Self {
            account,
            csrf,
            idle_deadline,
            absolute_deadline,
            idle_wall,
            absolute_wall,
        }
    }

    /// A user action at monotonic time `now` and wall-clock time `wall` moves
    /// the idle deadlines to `+ Si`, but not past the absolute deadlines. The
    /// absolute deadlines never move.
    pub fn extend_idle(&mut self, now: Duration, wall: SystemTime, policy: Policy) {
        let (idle_deadline, idle_wall) = idle_deadlines(
            now,
            wall,
            self.absolute_deadline,
            self.absolute_wall,
            policy,
        );
        self.idle_deadline = idle_deadline;
        self.idle_wall = idle_wall;
    }

    /// True while `now` is before both monotonic deadlines.
    pub fn is_live(&self, now: Duration) -> bool {
        now < self.idle_deadline && now < self.absolute_deadline
    }

    pub fn account(&self) -> &Name {
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

    /// The idle deadline as wall-clock time, for responses.
    pub fn idle_wall(&self) -> SystemTime {
        self.idle_wall
    }

    /// The absolute deadline as wall-clock time, for responses.
    pub fn absolute_wall(&self) -> SystemTime {
        self.absolute_wall
    }
}

/// The new idle deadlines: `now + Si` and `wall + Si`, each capped at the
/// absolute deadline of the same clock.
fn idle_deadlines(
    now: Duration,
    wall: SystemTime,
    absolute: Duration,
    absolute_wall: SystemTime,
    policy: Policy,
) -> (Duration, SystemTime) {
    let idle = now.saturating_add(policy.idle).min(absolute);
    let idle_wall = wall
        .checked_add(policy.idle)
        .unwrap_or(absolute_wall)
        .min(absolute_wall);
    (idle, idle_wall)
}

/// The administrator sessions of this process. A restart empties it
/// (cli-v1 §1.7, management.md MGT-R18).
#[derive(Debug, Default)]
pub struct Table {
    sessions: HashMap<Token, AdminSession>,
}

impl Table {
    pub fn insert(&mut self, token: Token, session: AdminSession) {
        self.sessions.insert(token, session);
    }

    pub fn get(&self, token: &Token) -> Option<&AdminSession> {
        self.sessions.get(token)
    }

    pub fn get_mut(&mut self, token: &Token) -> Option<&mut AdminSession> {
        self.sessions.get_mut(token)
    }

    pub fn remove(&mut self, token: &Token) -> Option<AdminSession> {
        self.sessions.remove(token)
    }

    /// Drops every session that is not live at `now`.
    pub fn prune(&mut self, now: Duration) {
        self.sessions.retain(|_, session| session.is_live(now));
    }

    /// Counts the sessions live at `now`. Unlike `prune`, it removes nothing
    /// (metrics-v1 `vnc_relay.admin_session.count`).
    pub fn live_count(&self, now: Duration) -> usize {
        self.sessions
            .values()
            .filter(|session| session.is_live(now))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn account() -> Name {
        Name::parse("admin").unwrap_or_else(|error| panic!("{error:?}"))
    }

    fn csrf() -> AdminCsrfToken {
        AdminCsrfToken::parse(TOKEN).unwrap_or_else(|error| panic!("{error}"))
    }

    fn policy(idle: u64, max: u64) -> Policy {
        Policy::new(Duration::from_secs(idle), Duration::from_secs(max))
    }

    fn wall_at(secs: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
    }

    #[test]
    fn accepts_only_64_lowercase_hex_digits() {
        assert!(Token::parse(TOKEN).is_ok());
        assert!(Token::parse(&TOKEN.to_uppercase()).is_err());
        assert!(Token::parse(&TOKEN[1..]).is_err());
        assert!(Token::parse(&format!("{TOKEN}0")).is_err());
        assert!(AdminCsrfToken::parse("g".repeat(64).as_str()).is_err());
    }

    #[test]
    fn debug_output_hides_the_token_value() {
        let token = Token::parse(TOKEN).unwrap_or_else(|error| panic!("{error}"));
        assert!(!format!("{token:?}").contains(TOKEN));
        assert!(!format!("{:?}", csrf()).contains(TOKEN));
    }

    /// spec: management.md:MGT-025 partial
    #[test]
    fn idle_deadline_moves_only_on_extension_and_never_past_the_absolute_one() {
        let policy = policy(300, 1000);
        let mut session =
            AdminSession::start(account(), csrf(), Duration::ZERO, wall_at(0), policy);
        assert!(session.is_live(Duration::from_secs(299)));
        assert!(!session.is_live(Duration::from_secs(300)));

        session.extend_idle(Duration::from_secs(200), wall_at(200), policy);
        assert!(session.is_live(Duration::from_secs(499)));
        assert!(!session.is_live(Duration::from_secs(500)));

        session.extend_idle(Duration::from_secs(900), wall_at(900), policy);
        assert_eq!(session.idle_deadline(), Duration::from_secs(1000));
        assert!(!session.is_live(Duration::from_secs(1000)));
    }

    /// spec: management.md:MGT-025 partial
    #[test]
    fn absolute_deadline_is_not_moved_by_user_actions() {
        let policy = policy(300, 600);
        let mut session =
            AdminSession::start(account(), csrf(), Duration::ZERO, wall_at(0), policy);
        session.extend_idle(Duration::from_secs(550), wall_at(550), policy);
        assert_eq!(session.absolute_deadline(), Duration::from_secs(600));
        assert!(session.is_live(Duration::from_secs(599)));
        assert!(!session.is_live(Duration::from_secs(600)));
    }

    /// spec: management.md:MGT-025 partial
    #[test]
    fn wall_deadlines_are_the_values_at_issue_and_extension() {
        let policy = policy(300, 3600);
        let mut session =
            AdminSession::start(account(), csrf(), Duration::ZERO, wall_at(5000), policy);
        assert_eq!(session.idle_wall(), wall_at(5300));
        assert_eq!(session.absolute_wall(), wall_at(8600));

        session.extend_idle(Duration::from_secs(1000), wall_at(6000), policy);
        assert_eq!(session.idle_deadline(), Duration::from_secs(1300));
        assert_eq!(session.idle_wall(), wall_at(6300));
        assert_eq!(session.absolute_wall(), wall_at(8600));
    }

    #[test]
    fn prune_keeps_only_live_sessions() {
        let policy = policy(300, 3600);
        let mut table = Table::default();
        let token = Token::parse(TOKEN).unwrap_or_else(|error| panic!("{error}"));
        table.insert(
            token.clone(),
            AdminSession::start(account(), csrf(), Duration::ZERO, wall_at(0), policy),
        );
        table.prune(Duration::from_secs(300));
        assert!(table.get(&token).is_none());
    }

    /// spec: operations.md:OPS-088 partial
    #[test]
    fn live_count_skips_expired_sessions_without_removing_them() {
        let policy = policy(300, 3600);
        let mut table = Table::default();
        let expired = Token::parse(TOKEN).unwrap_or_else(|error| panic!("{error}"));
        let live = Token::parse("fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210")
            .unwrap_or_else(|error| panic!("{error}"));
        table.insert(
            expired.clone(),
            AdminSession::start(account(), csrf(), Duration::ZERO, wall_at(0), policy),
        );
        table.insert(
            live.clone(),
            AdminSession::start(
                account(),
                csrf(),
                Duration::from_secs(200),
                wall_at(200),
                policy,
            ),
        );

        assert_eq!(table.live_count(Duration::from_secs(300)), 1);
        assert!(table.get(&expired).is_some());
        assert!(table.get(&live).is_some());
        assert_eq!(table.live_count(Duration::from_secs(500)), 0);
    }
}
