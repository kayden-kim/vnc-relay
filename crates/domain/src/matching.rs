//! Metadata-only Mode II matching and relay ID occupancy (relay.md §3.3).
//!
//! The caller holds one lock around these transitions and the corresponding
//! socket, Lw permit, and session changes. This table owns none of those resources.
//! Time values are monotonic durations supplied by the caller; no I/O is polled.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::time::Duration;

use crate::conn_id::ConnId;
use crate::entry_point::Role;
use crate::relay_id::RelayId;
use crate::session::SessionId;

/// One registered waiter's metadata (config-v2 §1.2, W and Lw).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaitingEntry {
    /// The accepted connection; delayed removal must compare this value.
    pub conn_id: ConnId,
    /// The role assigned by the entry point, not by the request payload.
    pub role: Role,
    /// The monotonic time of successful registration, not socket acceptance.
    pub registered_at: Duration,
    /// The absolute W boundary. Equality with the current time is expired.
    pub deadline: Duration,
}

/// A claimed pair before session activation (relay.md RLY-013 and RLY-014).
///
/// The role-ordered connection IDs identify the claim. No session ID is needed
/// until activation. Accepted connection IDs must never be reused by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchClaim {
    /// The Viewer connection, regardless of arrival order.
    pub viewer_conn_id: ConnId,
    /// The Server connection, regardless of arrival order.
    pub server_conn_id: ConnId,
    /// The removed waiter. The caller returns its Lw permit when claiming.
    pub waiter: WaitingEntry,
    /// The time matching ended W; activation does not restart or extend W.
    pub matched_at: Duration,
}

impl MatchClaim {
    /// Returns the first connection's wait at the matching transition.
    /// See ops-info-v1 §1.5, `session_started.waited_ms`.
    pub fn waited(&self) -> Duration {
        self.matched_at.saturating_sub(self.waiter.registered_at)
    }

    fn same_connections(&self, other: &Self) -> bool {
        self.viewer_conn_id == other.viewer_conn_id
            && self.server_conn_id == other.server_conn_id
    }
}

/// The two session phases that retain a Mode II ID (relay.md RLY-015/016).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionPhase {
    /// Relay has started and final cleanup has not begun.
    Active,
    /// Cleanup has begun, but sockets and L permits are not yet released.
    Closing,
}

/// An exact relay ID's occupancy (relay-session.md §3.3).
///
/// A session stores its ID only once; changing Active to Closing does not clone
/// or reallocate it. All non-waiting occupancies block both arriving roles.
#[derive(Debug, PartialEq, Eq)]
pub enum IdOccupancy {
    /// One connection holds one Lw place.
    Waiting(WaitingEntry),
    /// Two connections have left waiting, but have not started a session.
    Claimed(MatchClaim),
    /// One active or closing session holds the ID and no Lw place.
    Session {
        /// The session that alone may release this occupancy after cleanup.
        session_id: SessionId,
        /// The claimed Viewer, retained to prevent double dispatch until release.
        viewer_conn_id: ConnId,
        /// The claimed Server, retained to prevent double dispatch until release.
        server_conn_id: ConnId,
        /// Active or Closing; both exclude new arrivals.
        phase: SessionPhase,
    },
}

/// The bounded result of one arrival transition (relay-session.md §4.2).
#[derive(Debug, PartialEq, Eq)]
pub enum ArrivalOutcome {
    /// Registered a new waiter; the caller must attach its Lw permit and socket.
    Waiting(WaitingEntry),
    /// Removed one waiter and claimed both connections; return the waiter's Lw.
    Claimed(MatchClaim),
    /// Removed only an expired waiter. The incoming connection is untouched.
    ///
    /// Remove the old app entry and return its Lw permit, then retry the same
    /// arrival under the same outer lock. This is not an arrival rejection or
    /// registration. At most one such result precedes the terminal decision.
    Expired(WaitingEntry),
    /// Left all existing occupancy unchanged. DuplicateId and WaitingFull close
    /// only the incoming socket. ConnectionOccupied is a caller error: do not
    /// close or remove the connection already owned by the existing entry.
    Rejected(MatchingError),
}

/// Matching rejection or stale activation (repeater-handshake-v1 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchingError {
    /// A live same-role waiter or a non-waiting occupancy already uses the ID.
    DuplicateId,
    /// A new waiter would exceed Lw; an immediate match never takes this path.
    WaitingFull,
    /// This connection already occupies an ID, even if another ID was supplied.
    /// This is a caller double-dispatch error, not a contract rejection.
    ConnectionOccupied,
    /// The supplied connection pair does not own the current claim.
    InvalidClaim,
}

impl MatchingError {
    /// Returns a contract rejection code, or none for caller transition errors.
    /// See repeater-handshake-v1 §2 and ops-info-v1 §1.6.
    pub fn code(self) -> Option<&'static str> {
        match self {
            Self::DuplicateId => Some("duplicate_id"),
            Self::WaitingFull => Some("waiting_full"),
            Self::ConnectionOccupied | Self::InvalidClaim => None,
        }
    }
}

impl fmt::Display for MatchingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::DuplicateId => "relay ID is already occupied",
            Self::WaitingFull => "waiting connection limit reached",
            Self::ConnectionOccupied => "connection already occupies a relay ID",
            Self::InvalidClaim => "connection pair does not own the current claim",
        })
    }
}

impl std::error::Error for MatchingError {}

/// Exact-ID waiting and occupancy state, with no sockets or permit ownership.
///
/// See relay.md RLY-010 through RLY-016 and operations.md OPS-005. The caller
/// serializes every mutation with its app resource changes. A connection index
/// rejects double dispatch across IDs in every phase. The count is metadata, not
/// a second budget: hold exactly one Lw permit per Waiting entry before unlocking.
#[derive(Debug)]
pub struct WaitingTable {
    entries: HashMap<RelayId, IdOccupancy>,
    connections: HashSet<ConnId>,
    waiting_count: usize,
    waiting_limit: usize,
    wait_timeout: Duration,
}

impl WaitingTable {
    /// Creates an empty table with configured Lw and W (config-v2 §1.2).
    pub fn new(waiting_limit: usize, wait_timeout: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            connections: HashSet::new(),
            waiting_count: 0,
            waiting_limit,
            wait_timeout,
        }
    }

    /// Returns the number of Waiting entries, excluding all claimed sessions.
    pub fn waiting_count(&self) -> usize {
        self.waiting_count
    }

    /// Returns the configured Lw metadata limit (config-v2 §1.2).
    pub fn waiting_limit(&self) -> usize {
        self.waiting_limit
    }

    /// Borrows occupancy for the exact original ID, without normalization.
    pub fn get(&self, relay_id: &RelayId) -> Option<&IdOccupancy> {
        self.entries.get(relay_id)
    }

    /// Borrows all ID occupancies without allocating a snapshot or sorting.
    pub fn iter(&self) -> impl Iterator<Item = (&RelayId, &IdOccupancy)> {
        self.entries.iter()
    }

    /// Borrows only Waiting metadata for expiry or management snapshots.
    pub fn waiting(&self) -> impl Iterator<Item = (&RelayId, &WaitingEntry)> {
        self.entries.iter().filter_map(|(id, occupancy)| {
            if let IdOccupancy::Waiting(waiter) = occupancy {
                Some((id, waiter))
            } else {
                None
            }
        })
    }

    /// Applies one arrival step atomically under the caller's lock.
    ///
    /// See relay-session.md §4.2. Expiry precedes role and capacity decisions.
    /// An Expired result requires resource removal and a retry under that same
    /// lock. A match needs no incoming Lw permit, even at full waiting capacity.
    /// Only a new map key clones the relay ID. The caller must remove observed
    /// EOF/errors before calling this method; remote FIN is never inferred.
    /// Double dispatch is rejected before any mutation, including expiry.
    pub fn arrive(
        &mut self,
        relay_id: &RelayId,
        role: Role,
        conn_id: ConnId,
        now: Duration,
    ) -> ArrivalOutcome {
        if self.connections.contains(&conn_id) {
            return ArrivalOutcome::Rejected(MatchingError::ConnectionOccupied);
        }
        if let Some(occupancy) = self.entries.get_mut(relay_id) {
            let IdOccupancy::Waiting(waiter) = occupancy else {
                return ArrivalOutcome::Rejected(MatchingError::DuplicateId);
            };
            if now >= waiter.deadline {
                let expired = *waiter;
                self.entries.remove(relay_id);
                self.connections.remove(&expired.conn_id);
                self.waiting_count -= 1;
                return ArrivalOutcome::Expired(expired);
            }
            if waiter.role == role {
                return ArrivalOutcome::Rejected(MatchingError::DuplicateId);
            }
            let (viewer_conn_id, server_conn_id) = match role {
                Role::Viewer => (conn_id, waiter.conn_id),
                Role::Server => (waiter.conn_id, conn_id),
            };
            let claim = MatchClaim {
                viewer_conn_id,
                server_conn_id,
                waiter: *waiter,
                matched_at: now,
            };
            *occupancy = IdOccupancy::Claimed(claim);
            self.connections.insert(conn_id);
            self.waiting_count -= 1;
            return ArrivalOutcome::Claimed(claim);
        }
        if self.waiting_count >= self.waiting_limit {
            return ArrivalOutcome::Rejected(MatchingError::WaitingFull);
        }
        let waiter = WaitingEntry {
            conn_id,
            role,
            registered_at: now,
            deadline: now.saturating_add(self.wait_timeout),
        };
        self.entries
            .insert(relay_id.clone(), IdOccupancy::Waiting(waiter));
        self.connections.insert(conn_id);
        self.waiting_count += 1;
        ArrivalOutcome::Waiting(waiter)
    }

    /// Removes this exact waiter only when now is at or beyond its W boundary.
    /// A delayed timer cannot remove a fresh waiter or a claimed pair (RLY-014).
    pub fn expire(
        &mut self,
        relay_id: &RelayId,
        conn_id: ConnId,
        now: Duration,
    ) -> Option<WaitingEntry> {
        match self.entries.get(relay_id) {
            Some(IdOccupancy::Waiting(waiter))
                if waiter.conn_id == conn_id && now >= waiter.deadline =>
            {
                self.remove_waiting(relay_id, conn_id)
            }
            _ => None,
        }
    }

    /// Removes only an observed EOF's current waiter (relay.md RLY-014).
    ///
    /// Call only after a read returned zero, not because a socket seems idle
    /// or its B buffer is full. After claiming, EOF belongs to the claim owner;
    /// cancel the claim before activation, or use session cleanup after it.
    pub fn observe_eof(&mut self, relay_id: &RelayId, conn_id: ConnId) -> Option<WaitingEntry> {
        self.remove_waiting(relay_id, conn_id)
    }

    /// Removes a waiter for an observed error or shutdown, comparing conn_id.
    /// The returned metadata identifies the app entry and Lw permit to remove.
    pub fn remove_waiting(&mut self, relay_id: &RelayId, conn_id: ConnId) -> Option<WaitingEntry> {
        let Some(IdOccupancy::Waiting(waiter)) = self.entries.get(relay_id) else {
            return None;
        };
        if waiter.conn_id != conn_id {
            return None;
        }
        let removed = *waiter;
        self.entries.remove(relay_id);
        self.connections.remove(&removed.conn_id);
        self.waiting_count -= 1;
        Some(removed)
    }

    /// Activates only the current role-ordered claimed pair (RLY-013/014).
    ///
    /// The caller first rechecks observed endpoint termination and shutdown
    /// under the same lock. A session ID is supplied only after matching; it is
    /// moved into occupancy, not cloned. ID uniqueness belongs to its producer.
    pub fn activate(
        &mut self,
        relay_id: &RelayId,
        claim: &MatchClaim,
        session_id: SessionId,
    ) -> Result<(), MatchingError> {
        let Some(occupancy) = self.entries.get_mut(relay_id) else {
            return Err(MatchingError::InvalidClaim);
        };
        let IdOccupancy::Claimed(current) = occupancy else {
            return Err(MatchingError::InvalidClaim);
        };
        if !current.same_connections(claim) {
            return Err(MatchingError::InvalidClaim);
        }
        let viewer_conn_id = current.viewer_conn_id;
        let server_conn_id = current.server_conn_id;
        *occupancy = IdOccupancy::Session {
            session_id,
            viewer_conn_id,
            server_conn_id,
            phase: SessionPhase::Active,
        };
        Ok(())
    }

    /// Cancels only this claimed pair before activation (RLY-014).
    /// The caller cleans both transferred connections; no Lw place remains.
    pub fn cancel_claim(&mut self, relay_id: &RelayId, claim: &MatchClaim) -> bool {
        if !matches!(
            self.entries.get(relay_id),
            Some(IdOccupancy::Claimed(current)) if current.same_connections(claim)
        ) {
            return false;
        }
        self.entries.remove(relay_id);
        self.connections.remove(&claim.viewer_conn_id);
        self.connections.remove(&claim.server_conn_id);
        true
    }

    /// Marks the matching active session Closing while retaining its ID.
    /// Returns false for stale IDs and already-closing sessions (RLY-015/016).
    pub fn begin_closing(&mut self, relay_id: &RelayId, session_id: &SessionId) -> bool {
        match self.entries.get_mut(relay_id) {
            Some(IdOccupancy::Session {
                session_id: current,
                phase,
                ..
            }) if current == session_id && *phase == SessionPhase::Active => {
                *phase = SessionPhase::Closing;
                true
            }
            _ => false,
        }
    }

    /// Releases only this Closing session after sockets and L are cleaned up.
    /// A delayed release cannot remove a new waiter, claim, or session (RLY-016).
    pub fn release(&mut self, relay_id: &RelayId, session_id: &SessionId) -> bool {
        let Some(IdOccupancy::Session {
            session_id: current,
            viewer_conn_id,
            server_conn_id,
            phase: SessionPhase::Closing,
        }) = self.entries.get(relay_id)
        else {
            return false;
        };
        if current != session_id {
            return false;
        }
        self.connections.remove(viewer_conn_id);
        self.connections.remove(server_conn_id);
        self.entries.remove(relay_id);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(text: &str) -> RelayId {
        RelayId::parse(text).unwrap_or_else(|error| panic!("{error}"))
    }

    fn at(seconds: u64) -> Duration {
        Duration::from_secs(seconds)
    }

    fn table(limit: usize) -> WaitingTable {
        WaitingTable::new(limit, at(10))
    }

    fn register(
        table: &mut WaitingTable,
        id: &RelayId,
        role: Role,
        conn: ConnId,
        now: Duration,
    ) -> WaitingEntry {
        match table.arrive(id, role, conn, now) {
            ArrivalOutcome::Waiting(waiter) => waiter,
            outcome => panic!("unexpected registration: {outcome:?}"),
        }
    }

    fn claim(table: &mut WaitingTable, id: &RelayId, first_role: Role) -> MatchClaim {
        let first = ConnId::next();
        register(table, id, first_role, first, at(1));
        let second_role = match first_role {
            Role::Viewer => Role::Server,
            Role::Server => Role::Viewer,
        };
        match table.arrive(id, second_role, ConnId::next(), at(2)) {
            ArrivalOutcome::Claimed(claim) => claim,
            outcome => panic!("unexpected matching: {outcome:?}"),
        }
    }

    /// spec: relay.md:RLY-010 partial
    #[test]
    fn matches_server_first() {
        matches_both_roles(Role::Server);
    }

    /// spec: relay.md:RLY-011 partial
    #[test]
    fn matches_viewer_first() {
        matches_both_roles(Role::Viewer);
    }

    fn matches_both_roles(first_role: Role) {
        let mut table = table(1);
        let id = id("0123");
        let claim = claim(&mut table, &id, first_role);
        assert_ne!(claim.viewer_conn_id, claim.server_conn_id);
        assert_eq!(claim.waiter.role, first_role);
        assert_eq!(
            claim.waiter.conn_id,
            match first_role {
                Role::Viewer => claim.viewer_conn_id,
                Role::Server => claim.server_conn_id,
            }
        );
        assert_eq!(claim.waiter.registered_at, at(1));
        assert_eq!(claim.waiter.deadline, at(11));
        assert_eq!(claim.waited(), at(1));
        assert_eq!(table.waiting_count(), 0);
        assert_eq!(table.waiting().count(), 0);
        assert_eq!(table.get(&id), Some(&IdOccupancy::Claimed(claim)));
        assert_eq!(table.iter().count(), 1);
    }

    /// spec: relay.md:RLY-012 partial
    #[test]
    fn keeps_roles_and_exact_ids_separate() {
        let mut table = table(8);
        let long_a = "0".repeat(RelayId::MAX_LEN);
        let long_b = format!("{}1", "0".repeat(RelayId::MAX_LEN - 1));
        let ids = [id("0123"), id("123"), id(&long_a), id(&long_b)];
        let mut waiters = Vec::new();
        for id in &ids {
            waiters.push(register(&mut table, id, Role::Server, ConnId::next(), at(1)));
        }
        assert_eq!(table.waiting_count(), 4);
        for (id, waiter) in ids.iter().zip(&waiters) {
            assert_eq!(
                table.arrive(id, Role::Server, ConnId::next(), at(2)),
                ArrivalOutcome::Rejected(MatchingError::DuplicateId)
            );
            assert_eq!(table.get(id), Some(&IdOccupancy::Waiting(*waiter)));
        }
        for (id, waiter) in ids.iter().zip(waiters) {
            let viewer = ConnId::next();
            let ArrivalOutcome::Claimed(claim) = table.arrive(id, Role::Viewer, viewer, at(3))
            else {
                panic!("different IDs must keep their own waiter");
            };
            assert_eq!(claim.viewer_conn_id, viewer);
            assert_eq!(claim.server_conn_id, waiter.conn_id);
        }
        assert_eq!(table.waiting_count(), 0);
        assert_eq!(table.iter().count(), 4);
    }

    /// spec: relay.md:RLY-015 partial
    #[test]
    fn preserves_the_existing_waiter_for_late_same_role_duplicates() {
        for role in [Role::Viewer, Role::Server] {
            let mut table = table(1);
            let id = id("1");
            let waiter = register(&mut table, &id, role, ConnId::next(), at(1));
            for now in [at(2), at(5), at(10)] {
                assert_eq!(
                    table.arrive(&id, role, ConnId::next(), now),
                    ArrivalOutcome::Rejected(MatchingError::DuplicateId)
                );
                assert_eq!(table.get(&id), Some(&IdOccupancy::Waiting(waiter)));
                assert_eq!(table.waiting_count(), 1);
            }
        }
    }

    /// spec: operations.md:OPS-005 partial
    /// spec: relay.md:RLY-013 partial
    #[test]
    fn rejects_new_waiting_at_capacity_but_allows_an_immediate_match() {
        let mut table = table(1);
        let first_id = id("1");
        let second_id = id("2");
        let waiter = register(&mut table, &first_id, Role::Server, ConnId::next(), at(1));
        assert_eq!(table.waiting_limit(), 1);
        assert_eq!(
            table.arrive(&second_id, Role::Viewer, ConnId::next(), at(2)),
            ArrivalOutcome::Rejected(MatchingError::WaitingFull)
        );
        assert_eq!(table.get(&second_id), None);
        assert_eq!(table.waiting_count(), 1);
        let ArrivalOutcome::Claimed(claim) =
            table.arrive(&first_id, Role::Viewer, ConnId::next(), at(2))
        else {
            panic!("a match does not need an extra Lw place");
        };
        assert_eq!(claim.waiter, waiter);
        assert_eq!(table.waiting_count(), 0);
        register(&mut table, &second_id, Role::Viewer, ConnId::next(), at(3));
        assert_eq!(table.waiting_count(), 1);
    }

    /// spec: relay.md:RLY-014 partial
    #[test]
    fn expires_at_the_absolute_w_boundary_before_processing_arrival() {
        for arrival_time in [at(11), at(12)] {
            for role in [Role::Viewer, Role::Server] {
                let mut table = table(1);
                let id = id("1");
                let old = register(&mut table, &id, Role::Server, ConnId::next(), at(1));
                let incoming = ConnId::next();
                assert_eq!(table.expire(&id, old.conn_id, at(10)), None);
                assert_eq!(
                    table.arrive(&id, role, incoming, arrival_time),
                    ArrivalOutcome::Expired(old)
                );
                assert_eq!(table.get(&id), None);
                assert_eq!(table.waiting_count(), 0);
                let fresh = register(&mut table, &id, role, incoming, arrival_time);
                assert_eq!(fresh.registered_at, arrival_time);
                assert_eq!(fresh.deadline, arrival_time + at(10));
                assert_eq!(table.expire(&id, old.conn_id, at(100)), None);
                assert_eq!(table.get(&id), Some(&IdOccupancy::Waiting(fresh)));
            }
        }
        let mut table = table(1);
        let id = id("2");
        let waiter = register(&mut table, &id, Role::Viewer, ConnId::next(), at(1));
        assert_eq!(table.expire(&id, waiter.conn_id, at(11)), Some(waiter));
        assert_eq!(table.expire(&id, waiter.conn_id, at(12)), None);
        assert_eq!(table.waiting_count(), 0);
    }

    /// spec: relay.md:RLY-014 partial
    #[test]
    fn matches_just_before_w_without_restarting_the_deadline() {
        let mut table = table(1);
        let id = id("1");
        let waiter = register(&mut table, &id, Role::Viewer, ConnId::next(), at(1));
        let before = at(11) - Duration::from_nanos(1);
        let ArrivalOutcome::Claimed(claim) =
            table.arrive(&id, Role::Server, ConnId::next(), before)
        else {
            panic!("the waiter is still valid");
        };
        assert_eq!(claim.waiter, waiter);
        assert_eq!(claim.matched_at, before);
        assert_eq!(table.expire(&id, waiter.conn_id, at(11)), None);
        assert_eq!(table.get(&id), Some(&IdOccupancy::Claimed(claim)));
    }

    /// spec: relay.md:RLY-014 partial
    #[test]
    fn orders_observed_eof_and_claiming_without_inferring_remote_fin() {
        let id = id("1");
        let mut eof_first = table(1);
        let old = register(&mut eof_first, &id, Role::Server, ConnId::next(), at(1));
        assert_eq!(eof_first.observe_eof(&id, ConnId::next()), None);
        assert_eq!(eof_first.observe_eof(&id, old.conn_id), Some(old));
        assert_eq!(eof_first.observe_eof(&id, old.conn_id), None);
        let fresh = register(&mut eof_first, &id, Role::Viewer, ConnId::next(), at(2));
        assert_eq!(eof_first.observe_eof(&id, old.conn_id), None);
        assert_eq!(eof_first.get(&id), Some(&IdOccupancy::Waiting(fresh)));

        let mut match_first = table(1);
        let claim = claim(&mut match_first, &id, Role::Server);
        assert_eq!(match_first.observe_eof(&id, claim.waiter.conn_id), None);
        assert!(match_first.cancel_claim(&id, &claim));
        assert!(!match_first.cancel_claim(&id, &claim));
        assert_eq!(
            match_first.activate(&id, &claim, SessionId::from_block([1; 16])),
            Err(MatchingError::InvalidClaim)
        );
        assert_eq!(match_first.get(&id), None);
        assert_eq!(match_first.waiting_count(), 0);
    }

    /// spec: relay.md:RLY-013 partial
    /// spec: relay.md:RLY-015 partial
    #[test]
    fn claims_each_connection_once_and_blocks_both_roles_until_cleanup() {
        let mut table = table(2);
        let id = id("1");
        let claim = claim(&mut table, &id, Role::Server);
        let session_id = SessionId::from_block([1; 16]);
        for phase in 0..3 {
            for role in [Role::Viewer, Role::Server] {
                for _ in 0..4 {
                    assert_eq!(
                        table.arrive(&id, role, ConnId::next(), at(3)),
                        ArrivalOutcome::Rejected(MatchingError::DuplicateId)
                    );
                }
            }
            assert_eq!(table.waiting_count(), 0);
            assert_eq!(table.remove_waiting(&id, claim.waiter.conn_id), None);
            assert_eq!(table.expire(&id, claim.waiter.conn_id, at(100)), None);
            match phase {
                0 => {
                    assert_eq!(table.activate(&id, &claim, session_id.clone()), Ok(()));
                    assert_eq!(
                        table.activate(&id, &claim, SessionId::from_block([2; 16])),
                        Err(MatchingError::InvalidClaim)
                    );
                    assert!(!table.release(&id, &session_id));
                }
                1 => {
                    assert!(table.begin_closing(&id, &session_id));
                    assert!(!table.begin_closing(&id, &session_id));
                }
                _ => {}
            }
        }
        assert!(table.release(&id, &session_id));
        assert!(!table.release(&id, &session_id));
        assert_eq!(table.get(&id), None);
    }

    /// spec: relay.md:RLY-013 partial
    #[test]
    fn never_pairs_a_connection_with_itself() {
        let mut table = table(1);
        let id = id("1");
        let conn_id = ConnId::next();
        let waiter = register(&mut table, &id, Role::Viewer, conn_id, at(1));
        assert_eq!(
            table.arrive(&id, Role::Server, conn_id, at(2)),
            ArrivalOutcome::Rejected(MatchingError::ConnectionOccupied)
        );
        assert_eq!(table.get(&id), Some(&IdOccupancy::Waiting(waiter)));
    }

    /// spec: relay.md:RLY-016 partial
    #[test]
    fn rejects_stale_activation_and_cleanup_after_same_id_reuse() {
        let mut table = table(2);
        let id = id("1");
        let old_claim = claim(&mut table, &id, Role::Viewer);
        let old_session = SessionId::from_block([1; 16]);
        let fresh_session = SessionId::from_block([2; 16]);
        assert_ne!(old_session, fresh_session);
        assert_eq!(table.activate(&id, &old_claim, old_session.clone()), Ok(()));
        assert!(!table.begin_closing(&id, &fresh_session));
        assert!(!table.cancel_claim(&id, &old_claim));
        assert!(table.begin_closing(&id, &old_session));
        assert!(!table.release(&id, &fresh_session));
        assert!(table.release(&id, &old_session));

        let first = ConnId::next();
        let fresh_waiter = register(&mut table, &id, Role::Server, first, at(1));
        assert!(!table.release(&id, &old_session));
        assert!(!table.begin_closing(&id, &old_session));
        assert_eq!(table.remove_waiting(&id, old_claim.waiter.conn_id), None);
        assert_eq!(table.get(&id), Some(&IdOccupancy::Waiting(fresh_waiter)));
        assert_eq!(
            table.activate(&id, &old_claim, old_session.clone()),
            Err(MatchingError::InvalidClaim)
        );
        let ArrivalOutcome::Claimed(fresh_claim) =
            table.arrive(&id, Role::Viewer, ConnId::next(), at(2))
        else {
            panic!("fresh pair must match");
        };
        assert_eq!(
            table.activate(&id, &old_claim, old_session.clone()),
            Err(MatchingError::InvalidClaim)
        );
        assert!(!table.cancel_claim(&id, &old_claim));
        assert!(!table.release(&id, &old_session));
        assert_eq!(table.get(&id), Some(&IdOccupancy::Claimed(fresh_claim)));
        assert_eq!(table.activate(&id, &fresh_claim, fresh_session.clone()), Ok(()));
        assert!(!table.begin_closing(&id, &old_session));
        assert!(!table.release(&id, &old_session));
        assert_eq!(
            table.get(&id),
            Some(&IdOccupancy::Session {
                session_id: fresh_session.clone(),
                viewer_conn_id: fresh_claim.viewer_conn_id,
                server_conn_id: fresh_claim.server_conn_id,
                phase: SessionPhase::Active,
            })
        );
        assert!(table.begin_closing(&id, &fresh_session));
        assert!(!table.release(&id, &old_session));
        assert!(table.release(&id, &fresh_session));
        assert_eq!(table.waiting_count(), 0);
    }

    /// spec: relay.md:RLY-016 partial
    #[test]
    fn verifies_both_role_ordered_connection_ids_when_activating() {
        let mut table = table(1);
        let id = id("1");
        let claim = claim(&mut table, &id, Role::Server);
        for wrong in [
            MatchClaim {
                viewer_conn_id: ConnId::next(),
                ..claim
            },
            MatchClaim {
                server_conn_id: ConnId::next(),
                ..claim
            },
            MatchClaim {
                viewer_conn_id: claim.server_conn_id,
                server_conn_id: claim.viewer_conn_id,
                ..claim
            },
        ] {
            assert_eq!(
                table.activate(&id, &wrong, SessionId::from_block([1; 16])),
                Err(MatchingError::InvalidClaim)
            );
            assert!(!table.cancel_claim(&id, &wrong));
            assert_eq!(table.get(&id), Some(&IdOccupancy::Claimed(claim)));
        }
        assert_eq!(
            table.activate(&id, &claim, SessionId::from_block([2; 16])),
            Ok(())
        );
    }

    /// spec: relay.md:RLY-013 partial
    #[test]
    fn rejects_double_dispatch_across_ids_in_every_occupied_phase() {
        let mut table = table(3);
        let first_id = id("1");
        let other_id = id("2");
        let first = ConnId::next();
        let waiter = register(&mut table, &first_id, Role::Viewer, first, at(1));
        for role in [Role::Viewer, Role::Server] {
            assert_eq!(
                table.arrive(&other_id, role, first, at(100)),
                ArrivalOutcome::Rejected(MatchingError::ConnectionOccupied)
            );
        }
        assert_eq!(table.get(&first_id), Some(&IdOccupancy::Waiting(waiter)));
        assert_eq!(table.get(&other_id), None);
        assert_eq!(table.waiting_count(), 1);
        assert_eq!(table.connections.len(), 1);
        let second = ConnId::next();
        let ArrivalOutcome::Claimed(claim) =
            table.arrive(&first_id, Role::Server, second, at(2))
        else {
            panic!("a different connection must match");
        };
        let session_id = SessionId::from_block([3; 16]);
        for phase in 0..3 {
            for conn_id in [first, second] {
                for role in [Role::Viewer, Role::Server] {
                    assert_eq!(
                        table.arrive(&other_id, role, conn_id, at(3)),
                        ArrivalOutcome::Rejected(MatchingError::ConnectionOccupied)
                    );
                }
            }
            assert_eq!(table.connections.len(), 2);
            assert_eq!(table.get(&other_id), None);
            match phase {
                0 => assert_eq!(table.activate(&first_id, &claim, session_id.clone()), Ok(())),
                1 => assert!(table.begin_closing(&first_id, &session_id)),
                _ => {}
            }
        }
        assert!(table.release(&first_id, &session_id));
        assert!(table.connections.is_empty());
    }

    /// spec: relay.md:RLY-014 partial
    #[test]
    fn removes_only_live_connection_index_entries_on_waiting_and_claim_cleanup() {
        let mut table = table(3);
        let first_id = id("1");
        let second_id = id("2");
        let first = register(&mut table, &first_id, Role::Viewer, ConnId::next(), at(1));
        let second = register(&mut table, &second_id, Role::Server, ConnId::next(), at(1));
        assert_eq!(table.connections.len(), 2);
        assert_eq!(table.expire(&first_id, second.conn_id, at(11)), None);
        assert_eq!(table.connections.len(), 2);
        assert_eq!(table.expire(&first_id, first.conn_id, at(11)), Some(first));
        assert!(!table.connections.contains(&first.conn_id));
        assert!(table.connections.contains(&second.conn_id));
        assert_eq!(table.observe_eof(&second_id, second.conn_id), Some(second));
        assert!(table.connections.is_empty());
        let claim = claim(&mut table, &first_id, Role::Server);
        assert_eq!(table.connections.len(), 2);
        assert!(table.cancel_claim(&first_id, &claim));
        assert!(table.connections.is_empty());
        let fresh = register(&mut table, &first_id, Role::Viewer, ConnId::next(), at(2));
        assert!(!table.cancel_claim(&first_id, &claim));
        assert_eq!(table.connections.len(), 1);
        assert!(table.connections.contains(&fresh.conn_id));
        assert_eq!(table.remove_waiting(&first_id, fresh.conn_id), Some(fresh));
        assert!(table.connections.is_empty());
    }

    #[test]
    fn exposes_only_contract_codes_for_arrival_rejections() {
        assert_eq!(MatchingError::DuplicateId.code(), Some("duplicate_id"));
        assert_eq!(MatchingError::WaitingFull.code(), Some("waiting_full"));
        assert_eq!(MatchingError::InvalidClaim.code(), None);
        assert_eq!(MatchingError::ConnectionOccupied.code(), None);
    }
}
