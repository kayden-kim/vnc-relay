//! Sessions after login: lookup, CSRF check, idle extension and logout
//! (management-api-v1 §1.4.2, §1.4.3; management.md §3.0).

use vnc_relay_domain::admin_session::Token;

use super::{AdminAuth, AuthError, SessionView};
use crate::ports::AdminSessionSource;

/// A request that has a live session. The token names the session for
/// `touch` and `logout`.
#[derive(Debug)]
pub struct AuthorizedAdmin {
    token: Token,
    session: SessionView,
}

impl AuthorizedAdmin {
    pub fn session(&self) -> &SessionView {
        &self.session
    }
}

impl AdminAuth {
    /// Finds the live session for a cookie value. A missing, malformed,
    /// expired, logged-out or restart-dropped cookie all give `Unauthenticated`.
    pub fn authorize(&self, cookie: Option<&str>) -> Result<AuthorizedAdmin, AuthError> {
        let token = cookie
            .and_then(|text| Token::parse(text).ok())
            .ok_or(AuthError::Unauthenticated)?;
        let mono = self.clock.monotonic();
        let wall = self.clock.wall();
        let mut state = self.lock();
        state.sessions.prune(mono);
        let session = state
            .sessions
            .get(&token)
            .ok_or(AuthError::Unauthenticated)?;
        let view = super::session_view(session, wall);
        Ok(AuthorizedAdmin {
            token,
            session: view,
        })
    }

    /// Checks the `X-CSRF-Token` header against the session. A missing or
    /// wrong value gives `CsrfInvalid`. The session stays.
    pub fn check_csrf(
        &self,
        admin: &AuthorizedAdmin,
        header: Option<&str>,
    ) -> Result<(), AuthError> {
        let given = header.ok_or(AuthError::CsrfInvalid)?;
        if self.tokens.same_token(admin.session.csrf.as_str(), given) {
            Ok(())
        } else {
            Err(AuthError::CsrfInvalid)
        }
    }

    /// Moves the idle deadline to `now + Si`, never past the absolute
    /// deadline. The adapter calls this only for user actions (MGT-R16).
    pub fn touch(&self, admin: &mut AuthorizedAdmin) {
        let mono = self.clock.monotonic();
        let wall = self.clock.wall();
        let mut state = self.lock();
        let Some(session) = state.sessions.get_mut(&admin.token) else {
            return;
        };
        if !session.is_live(mono) {
            return;
        }
        session.extend_idle(mono, wall, self.session_policy);
        admin.session = super::session_view(session, wall);
    }

    /// Ends the session of `admin`. Relay sessions are not touched (MGT-020).
    pub fn logout(&self, admin: &AuthorizedAdmin) {
        self.lock().sessions.remove(&admin.token);
    }
}

impl AdminSessionSource for AdminAuth {
    /// A session past its deadline is not counted even before a request
    /// prunes it.
    fn live_sessions(&self) -> usize {
        let now = self.clock.monotonic();
        self.lock().sessions.live_count(now)
    }
}

#[cfg(test)]
mod tests {
    use vnc_relay_domain::admin_session::Policy;

    use super::super::AuthError;
    use crate::management::login::LoggedIn;
    use crate::management::test_support::{self, Harness, PASSWORD};

    fn login(h: &Harness) -> LoggedIn {
        h.auth
            .login("alice", PASSWORD, test_support::peer(1), None)
            .unwrap_or_else(|error| panic!("{error:?}"))
    }

    // Only the idle deadline is checked here. The rule that polling does not
    // extend the session is checked in the adapter tests.
    /// spec: management.md:MGT-025 partial
    #[test]
    fn idle_deadline_ends_the_session_without_user_actions() {
        let h = test_support::harness();
        let login = login(&h);
        h.clock.set(test_support::secs(299));
        assert!(h.auth.authorize(Some(login.token.as_str())).is_ok());
        h.clock.set(test_support::secs(300));
        assert_eq!(
            h.auth.authorize(Some(login.token.as_str())).err(),
            Some(AuthError::Unauthenticated)
        );
    }

    // Each touch moves the idle deadline by Si. The absolute deadline (1000 s
    // here) caps it, and the cap is never moved.
    /// spec: management.md:MGT-025 partial
    #[test]
    fn user_actions_extend_idle_but_not_past_the_absolute_deadline() {
        let h = test_support::harness_with(
            Policy::new(test_support::secs(300), test_support::secs(1000)),
            test_support::limit_policy(),
        );
        let login = login(&h);

        h.clock.set(test_support::secs(200));
        let mut admin = h
            .auth
            .authorize(Some(login.token.as_str()))
            .unwrap_or_else(|error| panic!("{error:?}"));
        h.auth.touch(&mut admin);

        h.clock.set(test_support::secs(499));
        assert!(h.auth.authorize(Some(login.token.as_str())).is_ok());
        h.auth.touch(&mut admin);

        h.clock.set(test_support::secs(798));
        assert!(h.auth.authorize(Some(login.token.as_str())).is_ok());
        h.auth.touch(&mut admin);
        assert_eq!(
            admin.session().idle_expires_at,
            admin.session().absolute_expires_at
        );

        h.clock.set(test_support::secs(999));
        assert!(h.auth.authorize(Some(login.token.as_str())).is_ok());
        h.clock.set(test_support::secs(1000));
        assert_eq!(
            h.auth.authorize(Some(login.token.as_str())).err(),
            Some(AuthError::Unauthenticated)
        );
    }

    /// spec: management.md:MGT-020 partial
    #[test]
    fn logout_ends_only_its_own_session() {
        let h = test_support::harness();
        let first = login(&h);
        let second = login(&h);
        let admin = h
            .auth
            .authorize(Some(first.token.as_str()))
            .unwrap_or_else(|error| panic!("{error:?}"));
        h.auth.logout(&admin);
        assert_eq!(
            h.auth.authorize(Some(first.token.as_str())).err(),
            Some(AuthError::Unauthenticated)
        );
        assert!(h.auth.authorize(Some(second.token.as_str())).is_ok());
    }

    // Only the CSRF check is covered. The HTTP status and the response body
    // are checked in the adapter tests.
    /// spec: management.md:MGT-014 partial
    #[test]
    fn csrf_missing_or_wrong_is_refused_and_keeps_the_session() {
        let h = test_support::harness();
        let login = login(&h);
        let admin = h
            .auth
            .authorize(Some(login.token.as_str()))
            .unwrap_or_else(|error| panic!("{error:?}"));
        let wrong = "0".repeat(64);
        assert_eq!(
            h.auth.check_csrf(&admin, None).err(),
            Some(AuthError::CsrfInvalid)
        );
        assert_eq!(
            h.auth.check_csrf(&admin, Some(wrong.as_str())).err(),
            Some(AuthError::CsrfInvalid)
        );
        assert!(h.auth.authorize(Some(login.token.as_str())).is_ok());
        assert!(
            h.auth
                .check_csrf(&admin, Some(admin.session().csrf.as_str()))
                .is_ok()
        );
    }

    /// spec: management.md:MGT-019 partial
    #[test]
    fn missing_malformed_unknown_or_expired_cookie_is_unauthenticated() {
        let h = test_support::harness();
        let login = login(&h);
        assert_eq!(
            h.auth.authorize(None).err(),
            Some(AuthError::Unauthenticated)
        );
        assert_eq!(
            h.auth.authorize(Some("not-a-token")).err(),
            Some(AuthError::Unauthenticated)
        );
        let zero = "0".repeat(64);
        assert_eq!(
            h.auth.authorize(Some(zero.as_str())).err(),
            Some(AuthError::Unauthenticated)
        );
        h.clock.set(test_support::secs(3600));
        assert_eq!(
            h.auth.authorize(Some(login.token.as_str())).err(),
            Some(AuthError::Unauthenticated)
        );
    }

    // A new AdminAuth stands for a restarted process. Its table is empty, so
    // the old cookie is refused before this process logs anyone in.
    /// spec: management.md:MGT-027 partial
    #[test]
    fn a_restarted_instance_refuses_the_old_cookie() {
        let before = test_support::harness();
        let login = before
            .auth
            .login("alice", PASSWORD, test_support::peer(1), None)
            .unwrap_or_else(|error| panic!("{error:?}"));

        let after = test_support::harness();
        assert_eq!(
            after.auth.authorize(Some(login.token.as_str())).err(),
            Some(AuthError::Unauthenticated)
        );
        assert!(
            after
                .auth
                .login("alice", PASSWORD, test_support::peer(1), None)
                .is_ok()
        );
    }

    // Covers the published values only. Relay continuity after expiry is not
    // checked here.
    // The published deadlines are fixed when they are issued. A wall-clock
    // jump, forward or back, does not change them. Only the monotonic clock
    // decides when the session ends.
    /// spec: management.md:MGT-025 partial
    #[test]
    fn wall_clock_jumps_do_not_move_the_published_deadlines() {
        let h = test_support::harness();
        let login = login(&h);
        let idle = login.session.idle_expires_at;
        let absolute = login.session.absolute_expires_at;

        h.clock
            .set_wall(test_support::base_wall() + test_support::secs(3600));
        let admin = h
            .auth
            .authorize(Some(login.token.as_str()))
            .unwrap_or_else(|error| panic!("{error:?}"));
        assert_eq!(admin.session().idle_expires_at, idle);
        assert_eq!(admin.session().absolute_expires_at, absolute);

        h.clock
            .set_wall(test_support::base_wall() - test_support::secs(3600));
        let admin = h
            .auth
            .authorize(Some(login.token.as_str()))
            .unwrap_or_else(|error| panic!("{error:?}"));
        assert_eq!(admin.session().idle_expires_at, idle);
        assert_eq!(admin.session().absolute_expires_at, absolute);
    }
}
