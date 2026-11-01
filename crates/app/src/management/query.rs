//! Management query service (management-api-v1 §1.6–§1.8, §1.10).
//!
//! Exposes point-in-time snapshots of relay runtime state, waiting queues,
//! active sessions, and recent closures without blocking relay streams or
//! consuming stream data.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use vnc_relay_domain::budget::ConnectionBudget;
use vnc_relay_domain::session::SessionState;

use crate::ports::{Clock, RelayLifecycle, RelayStateSource};
use crate::relay::{ActiveSessionSnapshot, RelaySnapshot};

/// One point-in-time server summary (management-api-v1 §1.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryView {
    pub as_of: SystemTime,
    pub server: ServerStateView,
    pub counts: CountsView,
    pub usage: UsageView,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerStateView {
    pub state: &'static str,
    pub uptime_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountsView {
    pub waiting: u64,
    pub active: u64,
    pub handshaking: u64,
    pub target_connecting: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageView {
    pub relay_connections: ResourceUsageView,
    pub admin_connections: ResourceUsageView,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceUsageView {
    pub current: u64,
    pub limit: u64,
}

/// A point-in-time snapshot of waiting connections (management-api-v1 §1.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitingListView {
    pub as_of: SystemTime,
    pub items: Vec<WaitingItemView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitingItemView {
    pub role: &'static str,
    pub relay_id: String,
    pub connected_at: SystemTime,
    pub waited_ms: u64,
}

/// A point-in-time snapshot of active sessions (management-api-v1 §1.8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionListView {
    pub as_of: SystemTime,
    pub items: Vec<ActiveSessionItemView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveSessionItemView {
    pub session_id: String,
    pub state: &'static str,
    pub mode: &'static str,
    pub viewer_transport: &'static str,
    pub started_at: SystemTime,
    pub bytes_viewer_to_target: u64,
    pub bytes_target_to_viewer: u64,
    pub viewer_address: SocketAddr,
    pub target_address: SocketAddr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionDetailView {
    pub as_of: SystemTime,
    pub session: ActiveSessionItemView,
}

/// A point-in-time snapshot of recently closed sessions (management-api-v1 §1.10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosedSessionsView {
    pub as_of: SystemTime,
    pub max_entries: u64,
    pub items: Vec<ClosedSessionItemView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosedSessionItemView {
    pub session_id: String,
    pub mode: &'static str,
    pub viewer_transport: &'static str,
    pub started_at: SystemTime,
    pub ended_at: SystemTime,
    pub bytes_viewer_to_target: u64,
    pub bytes_target_to_viewer: u64,
    pub close_cause: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionQueryError {
    InvalidId,
    NotFound,
}

/// Serves query requests over the relay state snapshot.
pub struct ManagementQueryService {
    relay_source: Arc<dyn RelayStateSource>,
    relay_budget: Arc<ConnectionBudget>,
    admin_budget: Arc<ConnectionBudget>,
    clock: Arc<dyn Clock>,
    start_mono: Duration,
}

impl ManagementQueryService {
    pub fn new(
        relay_source: Arc<dyn RelayStateSource>,
        relay_budget: Arc<ConnectionBudget>,
        admin_budget: Arc<ConnectionBudget>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        let start_mono = clock.monotonic();
        Self {
            relay_source,
            relay_budget,
            admin_budget,
            clock,
            start_mono,
        }
    }

    /// Captures a single snapshot of relay runtime state under one brief lock.
    fn capture_snapshot(&self) -> (RelaySnapshot, SystemTime, Duration) {
        let snapshot = self.relay_source.snapshot(self.clock.as_ref());
        let as_of = self.clock.wall();
        let now_mono = self.clock.monotonic();
        (snapshot, as_of, now_mono)
    }

    /// `GET /api/v1/summary`
    pub fn summary(&self) -> SummaryView {
        let (snapshot, as_of, now_mono) = self.capture_snapshot();
        let uptime_ms = now_mono.saturating_sub(self.start_mono).as_millis() as u64;

        let server_state = match snapshot.lifecycle {
            RelayLifecycle::Running => "ready",
            RelayLifecycle::Grace { .. } | RelayLifecycle::Force => "shutdown_grace",
        };

        SummaryView {
            as_of,
            server: ServerStateView {
                state: server_state,
                uptime_ms,
            },
            counts: CountsView {
                waiting: snapshot.waiting.len() as u64,
                active: snapshot.active.len() as u64,
                handshaking: snapshot.handshaking as u64,
                target_connecting: snapshot.target_connecting as u64,
            },
            usage: UsageView {
                relay_connections: ResourceUsageView {
                    current: self.relay_budget.current() as u64,
                    limit: self.relay_budget.limit() as u64,
                },
                admin_connections: ResourceUsageView {
                    current: self.admin_budget.current() as u64,
                    limit: self.admin_budget.limit() as u64,
                },
            },
        }
    }

    /// `GET /api/v1/waiting`
    pub fn waiting(&self) -> WaitingListView {
        let (snapshot, as_of, _now_mono) = self.capture_snapshot();
        let items = snapshot
            .waiting
            .into_iter()
            .map(|entry| WaitingItemView {
                role: entry.context.entry_point.role().as_str(),
                relay_id: entry.relay_id.as_str().to_owned(),
                connected_at: entry.context.accepted_wall,
                waited_ms: entry.waited.as_millis() as u64,
            })
            .collect();

        WaitingListView { as_of, items }
    }

    /// `GET /api/v1/sessions`
    pub fn sessions(&self) -> SessionListView {
        let (snapshot, as_of, _now_mono) = self.capture_snapshot();
        let items = snapshot
            .active
            .into_iter()
            .map(Self::map_active_session)
            .collect();

        SessionListView { as_of, items }
    }

    /// `GET /api/v1/sessions/{session_id}`
    pub fn session_detail(&self, session_id: &str) -> Result<SessionDetailView, SessionQueryError> {
        if !Self::is_valid_session_id(session_id) {
            return Err(SessionQueryError::InvalidId);
        }

        let (snapshot, as_of, _now_mono) = self.capture_snapshot();
        let session = snapshot
            .active
            .into_iter()
            .find(|entry| entry.session_id.as_str() == session_id)
            .map(Self::map_active_session)
            .ok_or(SessionQueryError::NotFound)?;

        Ok(SessionDetailView { as_of, session })
    }

    /// `GET /api/v1/closed-sessions`
    pub fn closed_sessions(&self) -> ClosedSessionsView {
        let (snapshot, as_of, _now_mono) = self.capture_snapshot();
        let items = snapshot
            .recent_closed
            .into_iter()
            .map(|closed| ClosedSessionItemView {
                session_id: closed.session_id.as_str().to_owned(),
                mode: closed.mode.as_str(),
                viewer_transport: closed.viewer_transport.as_str(),
                started_at: closed.started_at,
                ended_at: closed.ended_at,
                bytes_viewer_to_target: closed.bytes_viewer_to_target,
                bytes_target_to_viewer: closed.bytes_target_to_viewer,
                close_cause: closed.close_cause.as_str(),
            })
            .collect();

        ClosedSessionsView {
            as_of,
            max_entries: snapshot.max_entries as u64,
            items,
        }
    }

    fn map_active_session(entry: ActiveSessionSnapshot) -> ActiveSessionItemView {
        let state = match entry.state {
            SessionState::Closing | SessionState::Finished => "closing",
            SessionState::Active => "active",
        };
        let target_address = entry.peer.target_address();

        ActiveSessionItemView {
            session_id: entry.session_id.as_str().to_owned(),
            state,
            mode: entry.mode.as_str(),
            viewer_transport: entry.viewer_transport.as_str(),
            started_at: entry.started_at,
            bytes_viewer_to_target: entry.bytes_viewer_to_target,
            bytes_target_to_viewer: entry.bytes_target_to_viewer,
            viewer_address: entry.viewer.peer_addr,
            target_address,
        }
    }

    /// Enforces the session_id character set and length without regex.
    /// `^[a-zA-Z0-9_-]{1,64}$`
    pub fn is_valid_session_id(id: &str) -> bool {
        if id.is_empty() || id.len() > 64 {
            return false;
        }
        id.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::RelayIdRef;
    use crate::relay::{ClosedSessionSnapshot, ConnectionContext, WaitingSnapshot};
    use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
    use vnc_relay_domain::conn_id::ConnId;
    use vnc_relay_domain::entry_point::EntryPoint;
    use vnc_relay_domain::handshake::Mode;
    use vnc_relay_domain::peer_ip::PeerIp;
    use vnc_relay_domain::relay_id::RelayId;
    use vnc_relay_domain::session::{CloseCause, SessionId, ViewerTransport};

    struct TestClock {
        mono: AtomicU64,
        wall_offset_sec: AtomicI64,
        base_wall: SystemTime,
    }

    impl TestClock {
        fn new() -> Self {
            Self {
                mono: AtomicU64::new(10_000),
                wall_offset_sec: AtomicI64::new(0),
                base_wall: SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000),
            }
        }
    }

    impl Clock for TestClock {
        fn monotonic(&self) -> Duration {
            Duration::from_millis(self.mono.load(Ordering::Acquire))
        }

        fn wall(&self) -> SystemTime {
            let offset = self.wall_offset_sec.load(Ordering::Acquire);
            if offset >= 0 {
                self.base_wall + Duration::from_secs(offset as u64)
            } else {
                self.base_wall - Duration::from_secs((-offset) as u64)
            }
        }
    }

    struct DummyRelaySource {
        snapshot: std::sync::Mutex<RelaySnapshot>,
    }

    impl DummyRelaySource {
        fn new(snapshot: RelaySnapshot) -> Self {
            Self {
                snapshot: std::sync::Mutex::new(snapshot),
            }
        }
    }

    impl RelayStateSource for DummyRelaySource {
        fn snapshot(&self, _clock: &dyn Clock) -> RelaySnapshot {
            self.snapshot.lock().unwrap().clone()
        }
    }

    fn sample_snapshot() -> RelaySnapshot {
        RelaySnapshot {
            lifecycle: RelayLifecycle::Running,
            observed_at: Duration::from_millis(10_000),
            handshaking: 2,
            target_connecting: 1,
            max_entries: 10,
            waiting: vec![WaitingSnapshot {
                relay_id: RelayId::parse("123").unwrap(),
                relay_id_ref: RelayIdRef::from_hash_prefix([1; 8]),
                context: ConnectionContext {
                    conn_id: ConnId::next(),
                    entry_point: EntryPoint::TcpViewer,
                    peer_ip: PeerIp::from(
                        "192.0.2.1:5000".parse::<std::net::SocketAddr>().unwrap(),
                    ),
                    peer_addr: "192.0.2.1:5000".parse().unwrap(),
                    accepted_wall: SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000),
                    accepted_at: Duration::from_millis(8_000),
                    exchange_at: Duration::from_millis(8_500),
                },
                registered_at: Duration::from_millis(8_500),
                deadline: Duration::from_millis(18_500),
                waited: Duration::from_millis(1_500),
            }],
            active: vec![],
            recent_closed: vec![ClosedSessionSnapshot {
                session_id: SessionId::parse("ses_aaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap(),
                mode: Mode::Mode1,
                viewer_transport: ViewerTransport::Tcp,
                started_at: SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000),
                ended_at: SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_010),
                duration: Duration::from_secs(10),
                bytes_viewer_to_target: 1024,
                bytes_target_to_viewer: 2048,
                close_cause: CloseCause::PeerEof,
                relay_id_ref: None,
                side: None,
            }],
        }
    }

    /// spec: management.md:MGT-007 partial
    #[test]
    fn elapsed_and_uptime_never_negative_on_backward_wall_clock_step() {
        let clock = Arc::new(TestClock::new());
        let relay_budget = ConnectionBudget::new(10);
        let admin_budget = ConnectionBudget::new(5);
        let relay_source = Arc::new(DummyRelaySource::new(sample_snapshot()));

        let service = ManagementQueryService::new(
            relay_source,
            relay_budget,
            admin_budget,
            Arc::clone(&clock) as Arc<dyn Clock>,
        );

        // Advance monotonic time by 5,000 ms, but move wall-clock BACKWARD by 3,600 seconds
        clock.mono.fetch_add(5_000, Ordering::Release);
        clock.wall_offset_sec.store(-3600, Ordering::Release);

        let summary = service.summary();
        assert_eq!(summary.server.uptime_ms, 5_000);
        assert_eq!(summary.counts.waiting, 1);
        assert_eq!(summary.counts.handshaking, 2);
        assert_eq!(summary.counts.target_connecting, 1);

        let waiting = service.waiting();
        assert_eq!(waiting.items.len(), 1);
        assert_eq!(waiting.items[0].waited_ms, 1_500);
    }

    /// spec: management.md:MGT-001 partial
    /// spec: management.md:MGT-026 partial
    #[test]
    fn query_summary_counts_and_usage() {
        let clock = Arc::new(TestClock::new());
        let relay_budget = ConnectionBudget::new(20);
        let admin_budget = ConnectionBudget::new(10);
        let _p1 = relay_budget.try_acquire();
        let _p2 = relay_budget.try_acquire();
        let _a1 = admin_budget.try_acquire();

        let relay_source = Arc::new(DummyRelaySource::new(sample_snapshot()));
        let service = ManagementQueryService::new(
            relay_source,
            relay_budget,
            admin_budget,
            Arc::clone(&clock) as Arc<dyn Clock>,
        );

        let summary = service.summary();
        assert_eq!(summary.server.state, "ready");
        assert_eq!(summary.usage.relay_connections.current, 2);
        assert_eq!(summary.usage.relay_connections.limit, 20);
        assert_eq!(summary.usage.admin_connections.current, 1);
        assert_eq!(summary.usage.admin_connections.limit, 10);
        assert_eq!(summary.counts.handshaking, 2);
        assert_eq!(summary.counts.target_connecting, 1);
        assert_eq!(summary.counts.waiting, 1);
        assert_eq!(summary.counts.active, 0);
    }

    /// spec: management.md:MGT-003 partial
    /// spec: management.md:MGT-023 partial
    #[test]
    fn session_detail_validations() {
        let clock = Arc::new(TestClock::new());
        let relay_budget = ConnectionBudget::new(10);
        let admin_budget = ConnectionBudget::new(5);
        let relay_source = Arc::new(DummyRelaySource::new(sample_snapshot()));
        let service = ManagementQueryService::new(
            relay_source,
            relay_budget,
            admin_budget,
            Arc::clone(&clock) as Arc<dyn Clock>,
        );

        // Invalid IDs
        assert_eq!(
            service.session_detail(""),
            Err(SessionQueryError::InvalidId)
        );
        assert_eq!(
            service.session_detail("abc def"),
            Err(SessionQueryError::InvalidId)
        );
        assert_eq!(
            service.session_detail("abc/def"),
            Err(SessionQueryError::InvalidId)
        );
        assert_eq!(
            service.session_detail("abc?def"),
            Err(SessionQueryError::InvalidId)
        );
        assert_eq!(
            service.session_detail(&"a".repeat(65)),
            Err(SessionQueryError::InvalidId)
        );

        // Valid ID format but not in active list (even if in recent_closed, returns NotFound)
        assert_eq!(
            service.session_detail("ses_aaaaaaaaaaaaaaaaaaaaaaaaaa"),
            Err(SessionQueryError::NotFound)
        );
        assert_eq!(
            service.session_detail("unknown_id"),
            Err(SessionQueryError::NotFound)
        );
    }
}
