//! Recording, reading, and history of metrics-v1 without transports.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use vnc_relay_domain::account::AccountName;
use vnc_relay_domain::conn_id::ConnId;
use vnc_relay_domain::entry_point::{EntryPoint, Listener, Role};
use vnc_relay_domain::handshake::Mode;
use vnc_relay_domain::login_limit::LimitScope;
use vnc_relay_domain::peer_ip::PeerIp;
use vnc_relay_domain::session::{CloseCause, Direction, SessionId, Transport};

use super::history::{HISTORY_KEYS, History, HistoryView, HistoryWindow};
use super::snapshot::{MetricsSnapshot, Number, PointValue};
use super::{MetricSources, Metrics, MetricsService, MetricsSink};
use crate::events::{
    AbortReason, AdminTerminateResult, Event, LoginFailReason, RejectReason, RelayIdRef, Stage,
    TargetFailureReason,
};
use crate::ports::{
    AdminSessionSource, Clock, CpuTimes, EventSink, OpenCount, ProcessStats, ProcessStatsSource,
    RelayStateSource,
};
use crate::relay::test_support::TestSettings;
use crate::relay::{RelayCounts, RelaySnapshot};
use crate::reload::ReloadTrigger;
use vnc_relay_domain::budget::ConnectionBudget;

/// Monotonic time the test moves by hand; wall time stays fixed.
struct TestClock(Mutex<Duration>);

impl TestClock {
    fn at(seconds: u64) -> Arc<Self> {
        Arc::new(Self(Mutex::new(Duration::from_secs(seconds))))
    }

    fn advance(&self, by: Duration) {
        *self.0.lock().expect("clock lock") += by;
    }
}

impl Clock for TestClock {
    fn monotonic(&self) -> Duration {
        *self.0.lock().expect("clock lock")
    }

    fn wall(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_790_000_000)
    }
}

/// Relay counts set by the test.
struct FixedRelay(RelayCounts);

impl RelayStateSource for FixedRelay {
    fn snapshot(&self, _clock: &dyn Clock) -> RelaySnapshot {
        unreachable!("metrics read counts only")
    }

    fn counts(&self) -> RelayCounts {
        self.0
    }
}

struct AdminSessions(usize);

impl AdminSessionSource for AdminSessions {
    fn live_sessions(&self) -> usize {
        self.0
    }
}

struct FixedProcess(ProcessStats);

impl ProcessStatsSource for FixedProcess {
    fn read(&self) -> ProcessStats {
        self.0
    }
}

/// A log writer whose every write fails: it keeps nothing.
struct FailingLog;

impl EventSink for FailingLog {
    fn record(&self, _event: &Event) {}
}

fn readable_process() -> ProcessStats {
    ProcessStats {
        cpu: Some(CpuTimes {
            user: Duration::from_millis(1500),
            system: Duration::from_millis(250),
        }),
        memory_usage: Some(52_428_800),
        memory_virtual: Some(104_857_600),
        threads: Some(9),
        open: OpenCount::Handles(Some(42)),
    }
}

struct Fixture {
    clock: Arc<TestClock>,
    metrics: Arc<Metrics>,
    relay_budget: Arc<ConnectionBudget>,
    service: MetricsService,
}

fn fixture(counts: RelayCounts, process: ProcessStats) -> Fixture {
    let clock = TestClock::at(100);
    let metrics = Arc::new(Metrics::new(clock.as_ref()));
    let relay_budget = ConnectionBudget::new();
    let service = MetricsService::new(
        Arc::clone(&metrics),
        MetricSources {
            relay: Arc::new(FixedRelay(counts)),
            relay_budget: Arc::clone(&relay_budget),
            admin_budget: ConnectionBudget::new(),
            settings: TestSettings::new(),
            admin: Arc::new(AdminSessions(3)),
            process: Arc::new(FixedProcess(process)),
        },
        Arc::clone(&clock) as Arc<dyn Clock>,
        "2026.11.6-test",
    );
    Fixture {
        clock,
        metrics,
        relay_budget,
        service,
    }
}

fn number(value: Number) -> String {
    match value {
        Number::Int(value) => value.to_string(),
        Number::Float(value) => value.to_string(),
    }
}

/// The points of one metric as `attr=value,... value` lines, in point order.
/// A histogram point shows `count=` and `sum=`.
fn series(snapshot: &MetricsSnapshot, name: &str) -> Vec<String> {
    let family = snapshot
        .families
        .iter()
        .find(|family| family.name == name)
        .unwrap_or_else(|| panic!("no metric {name}"));
    family
        .points
        .iter()
        .map(|point| {
            let attributes = point
                .attributes
                .iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect::<Vec<_>>()
                .join(",");
            let value = match &point.value {
                PointValue::Number(value) => number(*value),
                PointValue::Histogram { count, sum, .. } => format!("count={count} sum={sum}"),
            };
            if attributes.is_empty() {
                value
            } else {
                format!("{attributes} {value}")
            }
        })
        .collect()
}

fn buckets(snapshot: &MetricsSnapshot, name: &str) -> Vec<Vec<u64>> {
    snapshot
        .families
        .iter()
        .filter(|family| family.name == name)
        .flat_map(|family| &family.points)
        .filter_map(|point| match &point.value {
            PointValue::Histogram { bucket_counts, .. } => Some(bucket_counts.clone()),
            PointValue::Number(_) => None,
        })
        .collect()
}

fn peer() -> PeerIp {
    PeerIp::new(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 10)))
}

fn relay_id() -> RelayIdRef {
    RelayIdRef::from_hash_prefix([7; 8])
}

fn account() -> AccountName {
    AccountName::parse("alice").expect("account name")
}

fn session_started(mode: Mode, viewer: Transport, server: Option<Transport>) -> Event {
    Event::SessionStarted {
        session_id: SessionId::from_block([1; 16]),
        mode,
        viewer_transport: viewer,
        server_transport: server,
        viewer_conn_id: ConnId::next(),
        viewer_peer_ip: peer(),
        viewer_forwarded_ip: None,
        server_conn_id: None,
        server_peer_ip: None,
        server_forwarded_ip: None,
        relay_id_ref: None,
        waited_ms: server.map(|_| 250),
        target_ip: None,
        target_port: None,
        target_host: None,
    }
}

/// One event of each counted kind, with known attributes.
fn counted_events() -> Vec<Event> {
    vec![
        Event::ConnectionAccepted {
            conn_id: ConnId::next(),
            listener: Listener::Ws,
            peer_ip: peer(),
        },
        Event::ConnectionAccepted {
            conn_id: ConnId::next(),
            listener: Listener::Ws,
            peer_ip: peer(),
        },
        Event::ConnectionAccepted {
            conn_id: ConnId::next(),
            listener: Listener::TcpViewer,
            peer_ip: peer(),
        },
        Event::ConnectionRejected {
            listener: "management",
            conn_id: None,
            stage: Stage::Accept,
            reason: RejectReason::AdminConnectionLimit { limit: 4 },
            peer_ip: peer(),
        },
        Event::UpgradeRejected {
            listener: "wss",
            role: Some(Role::Viewer),
            conn_id: ConnId::next(),
            reason: RejectReason::OriginNotAllowed,
            peer_ip: peer(),
            forwarded_ip: None,
            origin: Some("https://evil.example".to_owned()),
            ticket_jti: None,
        },
        Event::HandshakeRejected {
            conn_id: ConnId::next(),
            entry_point: EntryPoint::TcpServer,
            peer_ip: peer(),
            forwarded_ip: None,
            reason: RejectReason::HandshakeInvalid,
            mode: None,
        },
        Event::TicketRelayMismatch {
            conn_id: ConnId::next(),
            entry_point: EntryPoint::WsViewer,
            peer_ip: peer(),
            forwarded_ip: None,
            mode: Mode::Mode2,
            relay_id_ref: Some(relay_id()),
            ticket_jti: "jti-1".to_owned(),
        },
        Event::TargetRejected {
            conn_id: ConnId::next(),
            entry_point: EntryPoint::TcpViewer,
            peer_ip: peer(),
            forwarded_ip: None,
            reason: RejectReason::TargetNotAllowed,
            target_host: "db.internal".to_owned(),
            target_port: 5900,
            target_ip: None,
        },
        Event::RegistrationRejected {
            conn_id: ConnId::next(),
            entry_point: EntryPoint::WssServer,
            peer_ip: peer(),
            forwarded_ip: None,
            reason: RejectReason::WaitingFull { limit: 3 },
            relay_id_ref: relay_id(),
        },
        Event::ConnectionAborted {
            conn_id: ConnId::next(),
            listener: Listener::TcpViewer,
            role: None,
            stage: Stage::InitialExchange,
            reason: AbortReason::HandshakeTimeout,
            peer_ip: peer(),
            forwarded_ip: None,
            elapsed_ms: 10_000,
        },
        Event::RelayAborted {
            conn_id: ConnId::next(),
            entry_point: EntryPoint::WsServer,
            stage: Stage::Register,
            reason: AbortReason::PeerEof,
            peer_ip: peer(),
            forwarded_ip: None,
            elapsed_ms: 20,
            mode: Mode::Mode2,
        },
        Event::TicketAccepted {
            conn_id: ConnId::next(),
            listener: Listener::Wss,
            role: Role::Viewer,
            peer_ip: peer(),
            forwarded_ip: None,
            ticket_jti: "jti-2".to_owned(),
            ticket_sub: None,
            session_expires_at: None,
        },
        Event::WaitingExpired {
            conn_id: ConnId::next(),
            role: Role::Server,
            relay_id_ref: relay_id(),
            waited_ms: 5000,
        },
        Event::WaitingRemoved {
            conn_id: ConnId::next(),
            role: Role::Viewer,
            relay_id_ref: relay_id(),
            waited_ms: 1500,
            reason: crate::events::WaitingRemovalReason::PeerEof,
        },
        session_started(Mode::Mode2, Transport::Ws, Some(Transport::Tcp)),
        session_started(Mode::Mode1, Transport::Tcp, None),
        Event::SessionClosed {
            session_id: SessionId::from_block([1; 16]),
            mode: Mode::Mode2,
            viewer_transport: Transport::Ws,
            started_at: SystemTime::UNIX_EPOCH,
            duration_ms: 61_000,
            bytes_viewer_to_target: 10,
            bytes_target_to_viewer: 20,
            close_cause: CloseCause::PeerEof,
            side: None,
            error_kind: None,
            relay_id_ref: None,
            admin_account: None,
        },
        Event::AdminTerminate {
            admin_account: account(),
            peer_ip: peer(),
            session_id: None,
            result: AdminTerminateResult::Terminated,
        },
        Event::AdminTerminate {
            admin_account: account(),
            peer_ip: peer(),
            session_id: None,
            result: AdminTerminateResult::NotFound,
        },
        Event::AdminLoginSucceeded {
            admin_account: account(),
            peer_ip: peer(),
        },
        Event::AdminLoginFailed {
            reason: LoginFailReason::InvalidCredentials,
            scope: None,
            admin_account: Some(account()),
            peer_ip: peer(),
        },
        Event::AdminLoginFailed {
            reason: LoginFailReason::Limited,
            scope: Some(LimitScope::Source),
            admin_account: None,
            peer_ip: peer(),
        },
        Event::AdminLoginLimited {
            scope: LimitScope::Source,
            lockout_secs: 60,
            admin_account: None,
            peer_ip: peer(),
        },
        Event::ConfigEdited {
            admin_account: account(),
            peer_ip: peer(),
            config_keys: vec!["limits.waiting".to_owned()],
            revision: "r1".to_owned(),
        },
        Event::ConfigEditRejected {
            admin_account: account(),
            peer_ip: peer(),
            reason: "config_conflict".to_owned(),
            config_keys: None,
        },
        Event::ReloadApplied {
            trigger: ReloadTrigger::FileWatch,
            applied: vec!["limits.waiting".to_owned()],
        },
        Event::ReloadRejected {
            reason: "config_type_mismatch".to_owned(),
            config_key: Some("limits.waiting".to_owned()),
            detail: None,
        },
        Event::ReloadRestartRequired {
            config_keys: vec!["metrics.enabled".to_owned()],
        },
        // Not counted by any metric.
        Event::ServerStarting,
        Event::TargetConnectFailed {
            conn_id: ConnId::next(),
            entry_point: EntryPoint::TcpViewer,
            viewer_transport: Transport::Tcp,
            peer_ip: peer(),
            forwarded_ip: None,
            target_host: "db.internal".to_owned(),
            target_port: 5900,
            target_ip: None,
            elapsed_ms: 4000,
            reason: TargetFailureReason::ConnectTimeout,
        },
    ]
}

/// spec: operations.md:OPS-085 partial
/// spec: operations.md:OPS-086 partial
/// spec: operations.md:OPS-088 partial
/// spec: operations.md:OPS-089 partial
#[test]
fn counts_each_event_under_its_log_attributes_even_when_the_write_fails() {
    let fixture = fixture(RelayCounts::default(), readable_process());
    let sink = MetricsSink::new(Arc::clone(&fixture.metrics), Arc::new(FailingLog));
    for event in counted_events() {
        sink.record(&event);
    }
    let snapshot = fixture.service.snapshot();

    assert_eq!(
        series(&snapshot, "vnc_relay.connection.accepted"),
        ["listener=tcp_viewer 1", "listener=ws 2"]
    );
    assert_eq!(
        series(&snapshot, "vnc_relay.connection.rejected"),
        [
            "listener=management,stage=accept,reason=admin_connection_limit 1",
            "listener=tcp_server,stage=initial_exchange,reason=handshake_invalid 1",
            "listener=tcp_viewer,stage=target_connect,reason=target_not_allowed 1",
            "listener=ws,stage=initial_exchange,reason=ticket_relay_mismatch 1",
            "listener=wss,stage=register,reason=waiting_full 1",
            "listener=wss,stage=upgrade,reason=origin_not_allowed 1",
        ]
    );
    assert_eq!(
        series(&snapshot, "vnc_relay.connection.aborted"),
        [
            "listener=tcp_viewer,stage=initial_exchange,reason=handshake_timeout 1",
            "listener=ws,stage=register,reason=peer_eof 1",
        ]
    );
    assert_eq!(
        series(&snapshot, "vnc_relay.ticket.accepted"),
        ["listener=wss,role=viewer 1"]
    );
    assert_eq!(
        series(&snapshot, "vnc_relay.waiting.duration"),
        [
            "result=expired count=1 sum=5",
            "result=matched count=1 sum=0.25",
            "result=removed count=1 sum=1.5",
        ]
    );
    assert_eq!(
        series(&snapshot, "vnc_relay.session.started"),
        [
            "mode=mode1,viewer_transport=tcp,target_transport=tcp 1",
            "mode=mode2,viewer_transport=ws,target_transport=tcp 1",
        ]
    );
    assert_eq!(
        series(&snapshot, "vnc_relay.session.closed"),
        ["mode=mode2,close_cause=peer_eof 1"]
    );
    assert_eq!(
        series(&snapshot, "vnc_relay.session.duration"),
        ["mode=mode2 count=1 sum=61"]
    );
    assert_eq!(
        series(&snapshot, "vnc_relay.admin.terminations"),
        ["result=not_found 1", "result=terminated 1"]
    );
    assert_eq!(
        series(&snapshot, "vnc_relay.admin.logins"),
        [
            "result=invalid_credentials 1",
            "result=limited 1",
            "result=succeeded 1",
        ]
    );
    assert_eq!(
        series(&snapshot, "vnc_relay.admin.lockouts"),
        ["scope=source 1"]
    );
    assert_eq!(
        series(&snapshot, "vnc_relay.config.edits"),
        [
            "result=edited 1",
            "result=rejected,reason=config_conflict 1"
        ]
    );
    assert_eq!(
        series(&snapshot, "vnc_relay.config.reloads"),
        [
            "result=applied,trigger=file_watch 1",
            "result=rejected,reason=config_type_mismatch 1",
            "result=restart_required 1",
        ]
    );
    // A target connect is recorded by the relay, not from the log event.
    assert_eq!(
        series(&snapshot, "vnc_relay.target_connect.duration"),
        Vec::<String>::new()
    );
}

/// spec: operations.md:OPS-087 partial
/// spec: operations.md:OPS-093 partial
#[test]
fn puts_each_value_in_the_first_bucket_whose_bound_is_at_least_the_value() {
    let fixture = fixture(RelayCounts::default(), readable_process());
    let metrics = &fixture.metrics;
    // Equal to the first bound, between two bounds, above the last bound.
    for elapsed in [5, 7, 200_000] {
        metrics.record_handshake(
            Listener::TcpViewer,
            Mode::Mode2,
            Duration::from_millis(elapsed),
        );
    }
    metrics.record_target_connect(Ok(()), Duration::from_millis(30));
    metrics.record_target_connect(
        Err(TargetFailureReason::ConnectTimeout),
        Duration::from_secs(10),
    );
    let snapshot = fixture.service.snapshot();

    let mut handshake = vec![0; super::SHORT_DURATION_BOUNDS.len() + 1];
    handshake[0] = 1;
    handshake[1] = 1;
    handshake[super::SHORT_DURATION_BOUNDS.len()] = 1;
    assert_eq!(
        buckets(&snapshot, "vnc_relay.handshake.duration"),
        [handshake]
    );
    assert_eq!(
        series(&snapshot, "vnc_relay.handshake.duration"),
        [format!(
            "listener=tcp_viewer,mode=mode2 count=3 sum={}",
            0.005 + 0.007 + 200.0
        )]
    );
    assert_eq!(
        series(&snapshot, "vnc_relay.target_connect.duration"),
        [
            "result=connect_timeout count=1 sum=10",
            "result=connected count=1 sum=0.03",
        ]
    );
    // 10 s equals the bound 10, the eleventh bound.
    let timeout = buckets(&snapshot, "vnc_relay.target_connect.duration")
        .into_iter()
        .next()
        .expect("connect_timeout point");
    assert_eq!(timeout.get(10), Some(&1));
    assert_eq!(timeout.iter().sum::<u64>(), 1);
}

/// spec: operations.md:OPS-086 partial
#[test]
fn adds_session_bytes_by_direction_and_shows_only_directions_that_carried_bytes() {
    let fixture = fixture(RelayCounts::default(), readable_process());
    assert_eq!(
        series(&fixture.service.snapshot(), "vnc_relay.session.io"),
        Vec::<String>::new()
    );

    fixture.metrics.add_io(Direction::ViewerToTarget, 10);
    fixture.metrics.add_io(Direction::ViewerToTarget, 32);
    assert_eq!(
        series(&fixture.service.snapshot(), "vnc_relay.session.io"),
        ["direction=viewer_to_target 42"]
    );

    fixture.metrics.add_io(Direction::TargetToViewer, 7);
    assert_eq!(
        series(&fixture.service.snapshot(), "vnc_relay.session.io"),
        [
            "direction=target_to_viewer 7",
            "direction=viewer_to_target 42"
        ]
    );
}

/// spec: operations.md:OPS-089 partial
#[test]
fn counts_websocket_ping_timeouts_by_listener() {
    let fixture = fixture(RelayCounts::default(), readable_process());
    fixture.metrics.record_ping_timeout(Listener::Wss);
    fixture.metrics.record_ping_timeout(Listener::Ws);
    fixture.metrics.record_ping_timeout(Listener::Wss);

    assert_eq!(
        series(
            &fixture.service.snapshot(),
            "vnc_relay.websocket.ping_timeouts"
        ),
        ["listener=ws 1", "listener=wss 2"]
    );
}

/// spec: operations.md:OPS-083 partial
/// spec: management.md:MGT-057 partial
#[test]
fn reads_current_counts_limits_and_every_fixed_combination_in_contract_order() {
    let mut counts = RelayCounts::default();
    counts.handshaking = 2;
    counts.target_connecting = 1;
    counts.add_waiting(Role::Server, Transport::Wss);
    counts.add_session(Mode::Mode2, Transport::Ws, Transport::Tcp);
    let fixture = fixture(counts, readable_process());
    let _held = fixture.relay_budget.try_acquire(20).expect("relay place");
    fixture.clock.advance(Duration::from_millis(12_500));
    let snapshot = fixture.service.snapshot();

    let names: Vec<&str> = snapshot.families.iter().map(|family| family.name).collect();
    assert_eq!(
        names,
        [
            "vnc_relay.connection.accepted",
            "vnc_relay.connection.rejected",
            "vnc_relay.connection.aborted",
            "vnc_relay.connection.in_progress",
            "vnc_relay.relay_connection.usage",
            "vnc_relay.relay_connection.limit",
            "vnc_relay.admin_connection.usage",
            "vnc_relay.admin_connection.limit",
            "vnc_relay.handshake.duration",
            "vnc_relay.waiting.count",
            "vnc_relay.waiting.duration",
            "vnc_relay.session.count",
            "vnc_relay.session.started",
            "vnc_relay.session.closed",
            "vnc_relay.session.duration",
            "vnc_relay.session.io",
            "vnc_relay.target_connect.duration",
            "vnc_relay.ticket.accepted",
            "vnc_relay.websocket.ping_timeouts",
            "vnc_relay.admin.logins",
            "vnc_relay.admin.lockouts",
            "vnc_relay.admin.terminations",
            "vnc_relay.admin_session.count",
            "vnc_relay.config.reloads",
            "vnc_relay.config.edits",
            "process.uptime",
            "process.cpu.time",
            "process.memory.usage",
            "process.memory.virtual",
            "process.thread.count",
            "process.windows.handle.count",
        ]
    );
    assert_eq!(snapshot.service_version, "2026.11.6-test");
    assert_eq!(
        series(&snapshot, "vnc_relay.connection.in_progress"),
        ["stage=initial_exchange 2", "stage=target_connect 1"]
    );
    assert_eq!(series(&snapshot, "vnc_relay.relay_connection.usage"), ["1"]);
    assert_eq!(
        series(&snapshot, "vnc_relay.relay_connection.limit"),
        ["20"]
    );
    assert_eq!(series(&snapshot, "vnc_relay.admin_connection.usage"), ["0"]);
    assert_eq!(
        series(&snapshot, "vnc_relay.admin_connection.limit"),
        ["10"]
    );
    assert_eq!(
        series(&snapshot, "vnc_relay.waiting.count"),
        [
            "role=server,transport=tcp 0",
            "role=server,transport=ws 0",
            "role=server,transport=wss 1",
            "role=viewer,transport=tcp 0",
            "role=viewer,transport=ws 0",
            "role=viewer,transport=wss 0",
        ]
    );
    let sessions = series(&snapshot, "vnc_relay.session.count");
    assert_eq!(sessions.len(), 12);
    assert_eq!(
        sessions.first().map(String::as_str),
        Some("mode=mode1,viewer_transport=tcp,target_transport=tcp 0")
    );
    let busy: Vec<&String> = sessions
        .iter()
        .filter(|line| !line.ends_with(" 0"))
        .collect();
    assert_eq!(
        busy,
        ["mode=mode2,viewer_transport=ws,target_transport=tcp 1"]
    );
    assert!(
        !sessions
            .iter()
            .any(|line| line.starts_with("mode=mode1") && !line.contains("target_transport=tcp"))
    );
    assert_eq!(series(&snapshot, "vnc_relay.admin_session.count"), ["3"]);
    assert_eq!(series(&snapshot, "process.uptime"), ["12.5"]);
    assert_eq!(
        series(&snapshot, "process.cpu.time"),
        ["cpu.mode=system 0.25", "cpu.mode=user 1.5"]
    );
    assert_eq!(series(&snapshot, "process.windows.handle.count"), ["42"]);
    assert!(snapshot.families.iter().all(|family| family.available));
}

/// spec: operations.md:OPS-094 partial
/// spec: management.md:MGT-060 partial
#[test]
fn leaves_an_unreadable_value_unavailable_and_null_in_history_instead_of_zero() {
    let process = ProcessStats {
        memory_usage: None,
        open: OpenCount::FileDescriptors(None),
        ..readable_process()
    };
    let fixture = fixture(RelayCounts::default(), process);
    let snapshot = fixture.service.snapshot();

    let unavailable: Vec<&str> = snapshot
        .families
        .iter()
        .filter(|family| !family.available)
        .map(|family| family.name)
        .collect();
    assert_eq!(
        unavailable,
        ["process.memory.usage", "process.unix.file_descriptor.count"]
    );
    assert!(
        snapshot
            .families
            .iter()
            .filter(|family| !family.available)
            .all(|family| family.points.is_empty())
    );
    assert_eq!(series(&snapshot, "process.thread.count"), ["9"]);

    fixture.service.sample_history();
    let view = fixture.service.history_after(HistoryWindow::Hour, None);
    let point = view.points.first().expect("history point");
    let memory = HISTORY_KEYS
        .iter()
        .position(|key| *key == "process_memory_usage")
        .expect("memory key");
    for (index, value) in point.values.iter().enumerate() {
        if index == memory {
            assert_eq!(*value, None);
        } else {
            assert!(value.is_some(), "{}", HISTORY_KEYS[index]);
        }
    }
}

/// spec: management.md:MGT-058 partial
/// spec: management.md:MGT-061 partial
#[test]
fn takes_history_points_from_the_same_reading_as_the_snapshot() {
    let mut counts = RelayCounts::default();
    counts.add_waiting(Role::Viewer, Transport::Tcp);
    counts.add_waiting(Role::Server, Transport::Ws);
    counts.add_session(Mode::Mode1, Transport::Wss, Transport::Tcp);
    let fixture = fixture(counts, readable_process());
    let sink = MetricsSink::new(Arc::clone(&fixture.metrics), Arc::new(FailingLog));
    for event in counted_events() {
        sink.record(&event);
    }
    fixture.metrics.add_io(Direction::TargetToViewer, 99);

    fixture.service.sample_history();
    let view = fixture.service.history_after(HistoryWindow::Hour, None);
    assert_eq!(view.latest_seq, Some(0));
    let point = view.points.first().expect("history point");
    assert_eq!(point.seq, 0);
    assert_eq!(point.at, fixture.clock.wall());
    assert_eq!(
        point.values,
        [
            Some(Number::Int(1)),
            Some(Number::Int(2)),
            Some(Number::Int(3)),
            Some(Number::Int(6)),
            Some(Number::Int(2)),
            Some(Number::Int(0)),
            Some(Number::Int(99)),
            Some(Number::Float(1.75)),
            Some(Number::Int(52_428_800)),
        ]
    );
    // The point at ready is also the first point of the 24-hour history.
    assert_eq!(
        fixture.service.history_after(HistoryWindow::Day, None),
        view
    );
}

/// spec: management.md:MGT-058 partial
/// spec: management.md:MGT-059 partial
#[test]
fn keeps_the_newest_points_with_sequence_numbers_that_are_never_reused() {
    let hour = HistoryWindow::Hour;
    let mut history = History::new();
    let empty = history.after(hour, None);
    assert_eq!(empty.latest_seq, None);
    assert!(empty.points.is_empty());

    let at = SystemTime::UNIX_EPOCH;
    for _ in 0..=hour.capacity() {
        history.push(at, [None; HISTORY_KEYS.len()]);
    }
    let all = history.after(hour, None);
    let last = hour.capacity() as u64;
    assert_eq!(all.latest_seq, Some(last));
    assert_eq!(all.points.len(), hour.capacity());
    let seqs: Vec<u64> = all.points.iter().map(|point| point.seq).collect();
    assert_eq!(seqs, (1..=last).collect::<Vec<_>>());

    let newer: Vec<u64> = history
        .after(hour, Some(last - 2))
        .points
        .iter()
        .map(|point| point.seq)
        .collect();
    assert_eq!(newer, [last - 1, last]);
    // A number older than the oldest kept point returns every kept point.
    assert_eq!(history.after(hour, Some(0)).points.len(), hour.capacity());
    for after in [last, last + 1, u64::MAX] {
        let view = history.after(hour, Some(after));
        assert_eq!(view.latest_seq, Some(last));
        assert!(view.points.is_empty(), "after {after}");
    }
}

/// spec: management.md:MGT-061 partial
#[test]
fn keeps_every_sixth_point_for_24_hours_with_its_own_sequence_numbers() {
    let day = HistoryWindow::Day;
    assert_eq!(day.interval(), Duration::from_secs(60));
    assert_eq!(HistoryWindow::Hour.interval(), Duration::from_secs(10));
    let mut history = History::new();
    assert_eq!(
        history.after(day, None),
        HistoryView {
            latest_seq: None,
            points: Vec::new(),
        }
    );

    // One-hour point n is taken at 10n seconds and every value is n, so a
    // 24-hour point shows which one-hour point it came from.
    let start = SystemTime::UNIX_EPOCH;
    let point = |n: u64| {
        (
            start + Duration::from_secs(10 * n),
            [Some(Number::Int(n)); HISTORY_KEYS.len()],
        )
    };
    let last = day.capacity() as u64;
    // 6 × 1440 + 1 one-hour points give 1441 24-hour points.
    let total = 6 * last + 1;
    for n in 0..total {
        let (at, values) = point(n);
        history.push(at, values);
        if n < 13 {
            // Points 0, 6, and 12 open a new minute.
            let kept = history.after(day, None).points.len() as u64;
            assert_eq!(kept, n / 6 + 1, "after one-hour point {n}");
        }
    }

    let all = history.after(day, None);
    assert_eq!(all.latest_seq, Some(last));
    assert_eq!(all.points.len(), day.capacity());
    for (index, kept) in all.points.iter().enumerate() {
        // The oldest 24-hour point (seq 0, one-hour point 0) is gone.
        let seq = index as u64 + 1;
        let (at, values) = point(6 * seq);
        assert_eq!((kept.seq, kept.at, kept.values), (seq, at, values));
    }

    // `history_after` counts in 24-hour numbers.
    let newer: Vec<u64> = history
        .after(day, Some(last - 2))
        .points
        .iter()
        .map(|kept| kept.seq)
        .collect();
    assert_eq!(newer, [last - 1, last]);
    assert!(history.after(day, Some(last)).points.is_empty());

    // The one-hour history still keeps its newest 360 points, and its newest
    // point (8640) is also the newest 24-hour point.
    let hour = history.after(HistoryWindow::Hour, None);
    assert_eq!(hour.latest_seq, Some(total - 1));
    assert_eq!(hour.points.len(), HistoryWindow::Hour.capacity());
    let newest = hour.points.last().expect("one-hour point");
    let newest_day = all.points.last().expect("24-hour point");
    assert_eq!(
        (newest.seq, newest.at, newest.values),
        (total - 1, newest_day.at, newest_day.values)
    );
}
