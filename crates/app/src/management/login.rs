//! Login: checks an account and a password, then issues a session
//! (management-api-v1 §1.4.1, §1.5; management.md §3.0).

use vnc_relay_domain::account::{AccountName, Argon2Params, Password, PasswordHash};
use vnc_relay_domain::admin_session::{AdminCsrfToken, AdminSession, AdminSessionToken};
use vnc_relay_domain::peer_ip::PeerIp;

use super::{AdminAuth, AuthError, SessionView, session_view};
use crate::events::{Event, LoginFailReason};
use crate::ports::{HashError, PasswordHasher};

/// The password behind the dummy hash. It is hashed once at startup and
/// never stored. An unknown account is checked against the dummy hash, so it
/// takes the same password check as a known account (MGT-022).
const DUMMY_PASSWORD: &str = "dummy-password-for-unknown-accounts";

/// Makes the dummy hash that unknown accounts are checked against.
pub fn dummy_hash(
    hasher: &dyn PasswordHasher,
    params: Argon2Params,
) -> Result<PasswordHash, HashError> {
    let password =
        Password::parse(DUMMY_PASSWORD.to_owned()).map_err(|error| HashError(error.to_string()))?;
    hasher.hash(&password, params)
}

/// A new session after a successful login. The token goes only into the
/// `Set-Cookie` header.
#[derive(Debug)]
pub struct LoggedIn {
    pub token: AdminSessionToken,
    pub session: SessionView,
}

impl AdminAuth {
    /// Checks one login attempt. `source` is the peer IP. `prior` is the
    /// cookie value that the request sent, if any. A valid prior session
    /// ends when the login succeeds (management-api-v1 §1.4.1).
    pub fn login(
        &self,
        account: &str,
        password: &str,
        source: PeerIp,
        prior: Option<&str>,
    ) -> Result<LoggedIn, AuthError> {
        // A name that breaks the CLI rules can never be real. It is still
        // counted under the typed text (ops-info-v1 §1.8 admin_account rule).
        let name = AccountName::parse(account)
            .ok()
            .filter(|name| self.accounts.contains_key(name));
        let hash = name
            .as_ref()
            .and_then(|name| self.accounts.get(name))
            .unwrap_or(&self.dummy);

        // Every attempt runs one password check, and the check runs outside
        // the lock. This keeps the cost the same for unknown, locked and
        // wrong-password attempts.
        let verified = self
            .verifier
            .verify(password, hash)
            .map_err(|error| AuthError::Internal(error.0))?;

        // Tokens are made before the lock. A limited attempt may waste them.
        let tokens = match (&name, verified) {
            (Some(_), true) => Some(self.new_tokens()?),
            _ => None,
        };

        let mono = self.clock.monotonic();
        let wall = self.clock.wall();
        let mut state = self.lock();
        state.sessions.prune(mono);
        state.limiter.prune(mono);

        if let Some(scope) = state.limiter.check(account, source, mono) {
            drop(state);
            self.record(Event::AdminLoginFailed {
                reason: LoginFailReason::Limited,
                scope: Some(scope),
                admin_account: name,
                peer_ip: source,
            });
            return Err(AuthError::LoginFailed);
        }

        if let (Some(name), Some((token, csrf))) = (name.clone(), tokens) {
            if let Some(prior) = prior.and_then(|text| AdminSessionToken::parse(text).ok()) {
                state.sessions.remove(&prior);
            }
            let session = AdminSession::start(name.clone(), csrf, mono, self.session_policy);
            let view = session_view(&session, mono, wall);
            state.sessions.insert(token.clone(), session);
            drop(state);
            self.record(Event::AdminLoginSucceeded {
                admin_account: name,
                peer_ip: source,
            });
            return Ok(LoggedIn {
                token,
                session: view,
            });
        }

        let started = state.limiter.record_failure(account, source, mono);
        let lockout_secs = state.limiter.lockout().as_secs();
        drop(state);
        self.record(Event::AdminLoginFailed {
            reason: LoginFailReason::InvalidCredentials,
            scope: None,
            admin_account: name.clone(),
            peer_ip: source,
        });
        for scope in started {
            self.record(Event::AdminLoginLimited {
                scope,
                lockout_secs,
                admin_account: name.clone(),
                peer_ip: source,
            });
        }
        Err(AuthError::LoginFailed)
    }

    /// Makes the session token and the CSRF token. Both come from the token source.
    fn new_tokens(&self) -> Result<(AdminSessionToken, AdminCsrfToken), AuthError> {
        let token = self
            .tokens
            .new_token()
            .map_err(|error| AuthError::Internal(error.0))?;
        let csrf = self
            .tokens
            .new_token()
            .map_err(|error| AuthError::Internal(error.0))?;
        let token = AdminSessionToken::parse(&token)
            .map_err(|error| AuthError::Internal(error.to_string()))?;
        let csrf =
            AdminCsrfToken::parse(&csrf).map_err(|error| AuthError::Internal(error.to_string()))?;
        Ok((token, csrf))
    }
}

#[cfg(test)]
mod tests {
    use vnc_relay_domain::login_limit::{LimitScope, LoginLimitPolicy};

    use super::super::AuthError;
    use super::{DUMMY_PASSWORD, Password};
    use crate::events::{Event, LoginFailReason};
    use crate::management::test_support::{
        OTHER_PASSWORD, PASSWORD, harness, harness_with, peer, secs, session_policy,
    };

    #[test]
    fn dummy_password_is_a_valid_password() {
        assert!(Password::parse(DUMMY_PASSWORD.to_owned()).is_ok());
    }

    // The session is issued here. Cookie attributes and HTTP status are not
    // checked in this unit test, so the row is only partly covered.
    /// spec: management.md:MGT-017 partial
    #[test]
    fn correct_password_issues_a_session_and_records_success() {
        let h = harness();
        let login = h
            .auth
            .login("alice", PASSWORD, peer(1), None)
            .unwrap_or_else(|error| panic!("{error:?}"));
        assert_eq!(login.session.account.as_str(), "alice");
        assert!(h.auth.authorize(Some(login.token.as_str())).is_ok());
        assert!(
            h.events
                .all()
                .iter()
                .any(|event| matches!(event, Event::AdminLoginSucceeded { .. }))
        );
    }

    // The HTTP response bodies are compared in the adapter tests. Here only the
    // returned error and the event fields are compared.
    /// spec: management.md:MGT-018 partial
    #[test]
    fn wrong_password_and_unknown_account_give_the_same_refusal() {
        let h = harness();
        let wrong = h
            .auth
            .login("alice", "wrong-password-1", peer(1), None)
            .err();
        let unknown = h
            .auth
            .login("nobody", "wrong-password-1", peer(1), None)
            .err();
        assert_eq!(wrong, Some(AuthError::LoginFailed));
        assert_eq!(unknown, Some(AuthError::LoginFailed));

        // The event names the account only when the account is real (ops-info-v1 §1.8).
        let events = h.events.all();
        assert!(events.iter().any(|event| matches!(
            event,
            Event::AdminLoginFailed {
                reason: LoginFailReason::InvalidCredentials,
                admin_account: Some(_),
                ..
            }
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            Event::AdminLoginFailed {
                reason: LoginFailReason::InvalidCredentials,
                admin_account: None,
                ..
            }
        )));
    }

    // Only the number of password checks is compared. Response time is not
    // measured, so this does not cover the timing rule of MGT-022.
    /// spec: management.md:MGT-022 partial
    #[test]
    fn every_attempt_runs_one_password_check_including_unknown_and_limited() {
        let h = harness_with(
            session_policy(),
            LoginLimitPolicy::new(1, 100, secs(900), secs(900)),
        );
        let _ = h.auth.login("alice", "wrong-password-1", peer(1), None);
        let _ = h.auth.login("alice", "wrong-password-2", peer(1), None);
        let limited = h.auth.login("alice", PASSWORD, peer(1), None).err();
        let _ = h.auth.login("nobody", PASSWORD, peer(1), None);
        assert_eq!(limited, Some(AuthError::LoginFailed));
        assert_eq!(h.verifier.calls(), 4);
    }

    // The lockout is checked at the window edges. Time is set by the fake
    // clock. Exactly one more failure than F starts the lock. The lock expires
    // after `lockout`.
    /// spec: management.md:MGT-022 partial
    #[test]
    fn locked_account_refuses_the_correct_password_until_the_lock_ends() {
        let h = harness_with(
            session_policy(),
            LoginLimitPolicy::new(2, 100, secs(900), secs(60)),
        );
        for at in [0, 1, 2] {
            h.clock.set(secs(at));
            assert_eq!(
                h.auth
                    .login("alice", "wrong-password-1", peer(1), None)
                    .err(),
                Some(AuthError::LoginFailed)
            );
        }
        assert!(h.events.all().iter().any(|event| matches!(
            event,
            Event::AdminLoginLimited {
                scope: LimitScope::Account,
                lockout_secs: 60,
                ..
            }
        )));

        h.clock.set(secs(3));
        assert_eq!(
            h.auth.login("alice", PASSWORD, peer(1), None).err(),
            Some(AuthError::LoginFailed)
        );
        h.clock.set(secs(61));
        assert_eq!(
            h.auth.login("alice", PASSWORD, peer(1), None).err(),
            Some(AuthError::LoginFailed)
        );

        // After the lock ends, the counts start again. A wrong password is
        // refused, and the correct password then gets a session.
        h.clock.set(secs(62));
        assert_eq!(
            h.auth
                .login("alice", "wrong-password-1", peer(1), None)
                .err(),
            Some(AuthError::LoginFailed)
        );
        h.clock.set(secs(63));
        assert!(h.auth.login("alice", PASSWORD, peer(1), None).is_ok());
    }

    // The source lock counts failures for all accounts. A limited source
    // cannot log in to any account, but other sources still can.
    /// spec: management.md:MGT-022 partial
    #[test]
    fn same_source_is_limited_for_other_accounts_too() {
        let h = harness_with(
            session_policy(),
            LoginLimitPolicy::new(100, 2, secs(900), secs(60)),
        );
        let _ = h.auth.login("alice", "wrong-password-1", peer(9), None);
        let _ = h.auth.login("bob", "wrong-password-1", peer(9), None);
        let _ = h.auth.login("carol", "wrong-password-1", peer(9), None);
        assert!(h.events.all().iter().any(|event| matches!(
            event,
            Event::AdminLoginLimited {
                scope: LimitScope::Source,
                ..
            }
        )));

        assert_eq!(
            h.auth.login("alice", PASSWORD, peer(9), None).err(),
            Some(AuthError::LoginFailed)
        );
        assert!(h.auth.login("alice", PASSWORD, peer(10), None).is_ok());
        assert!(h.auth.login("bob", OTHER_PASSWORD, peer(10), None).is_ok());
    }

    // Contract §1.4.1: a new login ends a valid prior session. An unreadable
    // prior cookie is ignored.
    #[test]
    fn a_new_login_ends_the_prior_session() {
        let h = harness();
        let first = h
            .auth
            .login("alice", PASSWORD, peer(1), None)
            .unwrap_or_else(|error| panic!("{error:?}"));
        let second = h
            .auth
            .login("alice", PASSWORD, peer(1), Some(first.token.as_str()))
            .unwrap_or_else(|error| panic!("{error:?}"));
        assert!(h.auth.authorize(Some(first.token.as_str())).is_err());
        assert!(h.auth.authorize(Some(second.token.as_str())).is_ok());

        let third = h
            .auth
            .login("alice", PASSWORD, peer(1), Some("not-a-token"))
            .unwrap_or_else(|error| panic!("{error:?}"));
        assert!(h.auth.authorize(Some(third.token.as_str())).is_ok());
        assert!(h.auth.authorize(Some(second.token.as_str())).is_ok());
    }

    // Contract §2.1: a name outside the account grammar is a field error. It
    // is refused before any password check, writes no event, and does not
    // count against the source limit.
    #[test]
    fn account_outside_the_name_grammar_is_refused_and_not_counted() {
        let h = harness();
        let long = "x".repeat(33);
        for name in ["Alice", "al ice", "bob!", "가나다", "ALICE", long.as_str()] {
            let refused = h.auth.login(name, PASSWORD, peer(1), None).err();
            assert!(refused.is_some(), "{name} is refused");
            assert_ne!(refused, Some(AuthError::LoginFailed), "{name}");
        }
        assert_eq!(h.verifier.calls(), 0);
        assert!(h.events.all().is_empty());
        // Six refusals from one source would lock it under the default limit
        // of 5 per source. The valid login must still get through.
        assert!(h.auth.login("alice", PASSWORD, peer(1), None).is_ok());
    }
}
