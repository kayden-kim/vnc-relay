//! Bounded in-memory history containing only finalized active sessions.
//! See management-api-v1 §1.10 and ADR-0003.

use std::collections::VecDeque;

use crate::session::ClosedSession;

/// The newest R closed summaries, ordered by descending cleanup wall time.
/// Construction starts empty; no waiting or failed-connection data is accepted.
#[derive(Debug)]
pub struct RecentClosedList {
    max_entries: usize,
    entries: VecDeque<ClosedSession>,
}

impl RecentClosedList {
    /// Starts empty with the supplied R. Zero disables retention.
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries,
            entries: VecDeque::new(),
        }
    }

    /// Returns the configured maximum retained summary count R.
    pub fn max_entries(&self) -> usize {
        self.max_entries
    }

    /// Returns the current retained count, never greater than R.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether there are no retained summaries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Retains a finalized summary if it belongs to the newest R by `ended_at`.
    /// Evicts the oldest first, without temporarily growing the count above R.
    /// Equal timestamps place the newly supplied summary before existing ones.
    /// Returns the rejected incoming summary when R is zero or it is too old
    /// to retain; otherwise returns the displaced oldest summary, if any.
    pub fn push(&mut self, closed: ClosedSession) -> Option<ClosedSession> {
        if self.max_entries == 0 {
            return Some(closed);
        }
        let index = self
            .entries
            .iter()
            .position(|entry| entry.ended_at() <= closed.ended_at())
            .unwrap_or(self.entries.len());
        if index >= self.max_entries {
            return Some(closed);
        }
        let evicted = if self.entries.len() == self.max_entries {
            self.entries.pop_back()
        } else {
            None
        };
        self.entries.insert(index, closed);
        evicted
    }

    /// Borrows retained summaries newest first, without copying them.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &ClosedSession> + ExactSizeIterator {
        self.entries.iter()
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use crate::handshake::Mode;
    use crate::session::{CloseCause, Direction, Session, SessionId, Side, Transport};

    use super::*;

    fn closed(seed: u8, ended_secs: u64, cause: CloseCause) -> ClosedSession {
        let mut session = Session::new(
            SessionId::from_block([seed; 16]),
            Mode::Mode2,
            Transport::Ws,
            Transport::Wss,
            SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            Duration::from_secs(10),
        );
        session.record_write(Direction::ViewerToTarget, 7).unwrap();
        session.record_write(Direction::TargetToViewer, 19).unwrap();
        match cause {
            CloseCause::PeerEof => {
                session
                    .observe_eof(Side::Server, Duration::from_secs(12))
                    .unwrap();
                session
                    .observe_eof(Side::Viewer, Duration::from_secs(13))
                    .unwrap();
                session
                    .complete_direction(Direction::ViewerToTarget)
                    .unwrap();
                session
                    .complete_direction(Direction::TargetToViewer)
                    .unwrap();
            }
            CloseCause::ConnectionError => {
                session
                    .io_error(Direction::TargetToViewer, Side::Server)
                    .unwrap();
            }
            CloseCause::AdminTerminated => {
                session.begin_admin_close(Duration::from_secs(12)).unwrap();
            }
            CloseCause::SessionExpired => {
                session
                    .begin_session_expiry(Duration::from_secs(12))
                    .unwrap();
            }
            CloseCause::DeadConnection => {
                session.dead_connection(Side::Viewer).unwrap();
            }
            CloseCause::CloseTimeout => {
                session
                    .observe_eof(Side::Viewer, Duration::from_secs(12))
                    .unwrap();
                session
                    .expire_close(Duration::from_secs(17), Duration::from_secs(5))
                    .unwrap();
            }
            CloseCause::ServerShutdown => {
                session.force_server_shutdown().unwrap();
            }
            CloseCause::InternalError => {
                return session
                    .finish_after_owner_loss(
                        false,
                        SystemTime::UNIX_EPOCH + Duration::from_secs(ended_secs),
                        Duration::from_secs(20),
                    )
                    .unwrap_or_else(|e| panic!("owner loss: {e}"));
            }
        }
        session
            .finish(
                SystemTime::UNIX_EPOCH + Duration::from_secs(ended_secs),
                Duration::from_secs(20),
            )
            .unwrap()
    }

    /// spec: management.md:MGT-024 partial
    #[test]
    fn zero_capacity_stores_no_closed_summary() {
        let mut list = RecentClosedList::new(0);
        for seed in 0..5 {
            let _ = list.push(closed(seed, u64::from(seed) + 2, CloseCause::PeerEof));
            assert!(list.is_empty());
            assert_eq!(list.len(), 0);
            assert_eq!(list.iter().count(), 0);
        }
        assert_eq!(list.max_entries(), 0);
    }

    /// spec: management.md:MGT-024 partial
    #[test]
    fn evicts_only_oldest_and_iterates_newest_first_at_capacity() {
        for capacity in [1, 3] {
            let mut list = RecentClosedList::new(capacity);
            assert!(list.is_empty());
            assert_eq!(list.max_entries(), capacity);
            for seed in 0..8u8 {
                let _ = list.push(closed(seed, u64::from(seed) + 2, CloseCause::PeerEof));
                let expected_count = capacity.min(usize::from(seed) + 1);
                assert_eq!(list.len(), expected_count);
                assert_eq!(list.iter().len(), expected_count);
                let actual: Vec<_> = list.iter().map(|entry| entry.id().clone()).collect();
                let expected: Vec<_> = (0..=seed)
                    .rev()
                    .take(capacity)
                    .map(|value| SessionId::from_block([value; 16]))
                    .collect();
                assert_eq!(actual, expected);
            }
            assert_eq!(list.iter().rev().count(), capacity);
        }
    }

    /// spec: management.md:MGT-024 partial
    #[test]
    fn retains_newest_end_times_when_wall_clock_values_arrive_out_of_order() {
        let mut list = RecentClosedList::new(3);
        for (seed, time) in [(1, 10), (2, 30), (3, 20), (4, 5), (5, 25)] {
            let _ = list.push(closed(seed, time, CloseCause::ServerShutdown));
        }
        assert_eq!(
            list.iter()
                .map(|entry| entry.id().clone())
                .collect::<Vec<_>>(),
            [2, 5, 3].map(|seed| SessionId::from_block([seed; 16]))
        );
        assert_eq!(list.len(), 3);
        let _ = list.push(closed(6, 25, CloseCause::ServerShutdown));
        assert_eq!(
            list.iter()
                .map(|entry| entry.id().clone())
                .collect::<Vec<_>>(),
            [2, 6, 5].map(|seed| SessionId::from_block([seed; 16]))
        );
    }

    /// spec: management.md:MGT-024 partial
    #[test]
    fn zero_capacity_returns_rejected_incoming_summary() {
        let mut list = RecentClosedList::new(0);
        assert_eq!(
            list.push(closed(1, 20, CloseCause::AdminTerminated)),
            Some(closed(1, 20, CloseCause::AdminTerminated))
        );
        assert!(list.is_empty());
    }

    /// spec: management.md:MGT-024 partial
    #[test]
    fn capacity_insertion_returns_displaced_oldest_summary() {
        let mut list = RecentClosedList::new(2);
        assert_eq!(list.push(closed(1, 10, CloseCause::PeerEof)), None);
        assert_eq!(list.push(closed(2, 20, CloseCause::ServerShutdown)), None);
        assert_eq!(
            list.push(closed(3, 30, CloseCause::AdminTerminated)),
            Some(closed(1, 10, CloseCause::PeerEof))
        );
        assert_eq!(list.len(), 2);
    }

    /// spec: management.md:MGT-024 partial
    #[test]
    fn out_of_order_insertion_returns_rejected_or_displaced_summary() {
        let mut list = RecentClosedList::new(3);
        for (seed, time) in [(1, 10), (2, 30), (3, 20)] {
            assert_eq!(
                list.push(closed(seed, time, CloseCause::ServerShutdown)),
                None
            );
        }
        assert_eq!(
            list.push(closed(4, 5, CloseCause::ConnectionError)),
            Some(closed(4, 5, CloseCause::ConnectionError))
        );
        assert_eq!(
            list.push(closed(5, 25, CloseCause::AdminTerminated)),
            Some(closed(1, 10, CloseCause::ServerShutdown))
        );
        assert_eq!(
            list.push(closed(6, 25, CloseCause::PeerEof)),
            Some(closed(3, 20, CloseCause::ServerShutdown))
        );
        assert_eq!(
            list.iter()
                .map(|entry| entry.id().clone())
                .collect::<Vec<_>>(),
            [2, 6, 5].map(|seed| SessionId::from_block([seed; 16]))
        );
    }

    /// spec: management.md:MGT-024 partial
    #[test]
    fn insertion_without_eviction_returns_none() {
        let mut list = RecentClosedList::new(4);
        for (seed, time) in [(1, 20), (2, 10), (3, 30), (4, 20)] {
            assert_eq!(
                list.push(closed(seed, time, CloseCause::ServerShutdown)),
                None
            );
        }
        assert_eq!(
            list.iter()
                .map(|entry| entry.id().clone())
                .collect::<Vec<_>>(),
            [3, 4, 1, 2].map(|seed| SessionId::from_block([seed; 16]))
        );
    }

    /// spec: management.md:MGT-023 partial
    #[test]
    fn retains_final_causes_metadata_times_and_directional_counters() {
        let mut list = RecentClosedList::new(7);
        let causes = [
            CloseCause::PeerEof,
            CloseCause::ConnectionError,
            CloseCause::AdminTerminated,
            CloseCause::DeadConnection,
            CloseCause::CloseTimeout,
            CloseCause::ServerShutdown,
            CloseCause::InternalError,
        ];
        for (seed, cause) in (0..7u8).zip(causes) {
            let _ = list.push(closed(seed, 20, cause));
        }
        assert_eq!(list.len(), 7);
        for (entry, cause) in list.iter().zip(causes.into_iter().rev()) {
            assert_eq!(entry.close_cause(), cause);
            assert_eq!(entry.mode(), Mode::Mode2);
            assert_eq!(entry.viewer_transport(), Transport::Ws);
            assert_eq!(entry.target_transport(), Transport::Wss);
            assert_eq!(
                entry.started_at(),
                SystemTime::UNIX_EPOCH + Duration::from_secs(1)
            );
            assert_eq!(
                entry.ended_at(),
                SystemTime::UNIX_EPOCH + Duration::from_secs(20)
            );
            assert_eq!(entry.duration(), Duration::from_secs(10));
            assert_eq!(entry.bytes(Direction::ViewerToTarget), 7);
            assert_eq!(entry.bytes(Direction::TargetToViewer), 19);
        }
    }

    /// spec: management.md:MGT-024 partial
    #[test]
    fn starts_empty_again_without_restoring_prior_process_history() {
        let mut old = RecentClosedList::new(2);
        let _ = old.push(closed(1, 20, CloseCause::AdminTerminated));
        assert_eq!(old.len(), 1);
        let restarted = RecentClosedList::new(old.max_entries());
        assert!(restarted.is_empty());
        assert_eq!(restarted.iter().count(), 0);
        assert_eq!(old.len(), 1);
    }
}
