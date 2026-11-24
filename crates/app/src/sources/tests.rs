//! Counting rules, storage limit, and the sources query without transports.

use std::collections::{BTreeMap, HashMap};
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use vnc_relay_domain::conn_id::ConnId;
use vnc_relay_domain::entry_point::{ListenerKind, Role};
use vnc_relay_domain::handshake::Mode;
use vnc_relay_domain::ip_info::{Asn, CountryCode};
use vnc_relay_domain::peer_ip::PeerIp;
use vnc_relay_domain::session::{Id, Transport};

use super::report::Counts;
use super::{CAPACITY, PENDING_CAPACITY, SourceStats, SourceWindow, SourcesReport, SourcesService};
use crate::events::{AbortReason, Event, RejectReason, RelayIdRef, Stage};
use crate::ip_info::{IpDbKey, IpDbMetadata, IpDbRead, IpInfoService, IpLookup};
use crate::ports::{Clock, EventSink, IpDbFileGateway, IpDbLookupGateway};
use crate::relay::test_support::FakeRuntimeSettingsSource;

/// 2026-09-21T13:46:40Z in milliseconds.
const START_MS: u64 = 1_790_000_000_000;

/// Wall time the test moves by hand.
struct FakeClock(Mutex<u64>);

impl FakeClock {
    fn advance(&self, by: Duration) {
        *self.0.lock().expect("clock lock") += u64::try_from(by.as_millis()).expect("short step");
    }
}

impl Clock for FakeClock {
    fn monotonic(&self) -> Duration {
        Duration::ZERO
    }

    fn wall(&self) -> SystemTime {
        at(*self.0.lock().expect("clock lock"))
    }
}

fn at(millis: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_millis(millis)
}

/// A database whose records the test writes per address.
struct StubIpDbLookupGateway {
    metadata: IpDbMetadata,
    rows: HashMap<IpAddr, (Option<CountryCode>, Option<Asn>)>,
}

impl IpDbLookupGateway for StubIpDbLookupGateway {
    fn metadata(&self) -> &IpDbMetadata {
        &self.metadata
    }

    fn supports(&self, _key: IpDbKey) -> bool {
        true
    }

    fn country(&self, ip: IpAddr) -> Option<CountryCode> {
        self.rows.get(&ip).and_then(|row| row.0)
    }

    fn asn(&self, ip: IpAddr) -> Option<Asn> {
        self.rows.get(&ip).and_then(|row| row.1)
    }

    fn as_name(&self, ip: IpAddr) -> Option<(Asn, String)> {
        self.asn(ip)
            .map(|asn| (asn, format!("Example Net {}", asn.get())))
    }
}

/// No file is configured; the query shows `not_configured`.
struct StubIpDbFileGateway;

impl IpDbFileGateway for StubIpDbFileGateway {
    fn read(&self, _path: &std::path::Path) -> IpDbRead {
        unreachable!("no path is configured")
    }
}

struct StubEventSink;

impl EventSink for StubEventSink {
    fn record(&self, _event: &Event) {}
}

struct Fixture {
    clock: Arc<FakeClock>,
    lookup: Arc<IpLookup>,
    stats: Arc<SourceStats>,
    service: SourcesService,
    settings: Arc<FakeRuntimeSettingsSource>,
}

fn fixture() -> Fixture {
    let clock = Arc::new(FakeClock(Mutex::new(START_MS)));
    let lookup = Arc::new(IpLookup::new());
    let stats = Arc::new(SourceStats::new(
        Arc::clone(&lookup),
        Arc::clone(&clock) as Arc<dyn Clock>,
    ));
    let ip_info = Arc::new(IpInfoService::new(
        None,
        None,
        Arc::new(StubIpDbFileGateway),
        Arc::clone(&lookup),
        Arc::clone(&clock) as Arc<dyn Clock>,
        Arc::new(StubEventSink),
    ));
    let settings = FakeRuntimeSettingsSource::new();
    let service = SourcesService::new(
        Arc::clone(&stats),
        ip_info,
        Arc::clone(&settings) as Arc<dyn crate::reload::ports::RuntimeSettingsSource>,
    );
    Fixture {
        clock,
        lookup,
        stats,
        service,
        settings,
    }
}

impl Fixture {
    fn use_table(&self, rows: HashMap<IpAddr, (Option<CountryCode>, Option<Asn>)>) {
        let table: Arc<dyn IpDbLookupGateway> = Arc::new(StubIpDbLookupGateway {
            metadata: IpDbMetadata::new("DBIP-ASN-Lite", at(START_MS)),
            rows,
        });
        self.lookup.set(IpDbKey::Country, Arc::clone(&table));
        self.lookup.set(IpDbKey::Asn, table);
    }

    fn accept(&self, listener: ListenerKind, ip: &str) -> ConnId {
        let conn_id = ConnId::next();
        self.stats.record(&Event::ConnectionAccepted {
            conn_id,
            listener,
            peer_ip: peer(ip),
        });
        conn_id
    }

    fn refuse_at_accept(&self, listener: &'static str, ip: &str) {
        self.stats.record(&Event::ConnectionRejected {
            listener,
            conn_id: None,
            stage: Stage::Accept,
            reason: RejectReason::IpNotAllowed,
            peer_ip: peer(ip),
        });
    }

    fn abort(&self, conn_id: ConnId, listener: ListenerKind, ip: &str, forwarded: Option<&str>) {
        self.stats.record(&Event::ConnectionAborted {
            conn_id,
            listener,
            role: None,
            stage: Stage::Upgrade,
            reason: AbortReason::TlsUpgradeTimeout,
            peer_ip: peer(ip),
            forwarded_ip: forwarded.map(peer),
            elapsed_ms: 10,
        });
    }

    fn report(&self, window: SourceWindow) -> SourcesReport {
        self.service.report(window)
    }
}

fn peer(text: &str) -> PeerIp {
    PeerIp::new(ip(text))
}

fn ip(text: &str) -> IpAddr {
    text.parse().expect("test address")
}

fn session(mode: Mode, viewer: (ConnId, &str), server: Option<(ConnId, &str)>) -> Event {
    Event::SessionStarted {
        session_id: Id::from_block([7; 16]),
        mode,
        viewer_transport: Transport::Ws,
        server_transport: server.map(|_| Transport::Tcp),
        viewer_conn_id: viewer.0,
        viewer_peer_ip: peer(viewer.1),
        viewer_forwarded_ip: None,
        server_conn_id: server.map(|(conn_id, _)| conn_id),
        server_peer_ip: server.map(|(_, ip)| peer(ip)),
        server_forwarded_ip: None,
        relay_id_ref: server.map(|_| RelayIdRef::from_hash_prefix([1; 8])),
        waited_ms: server.map(|_| 5),
        target_ip: None,
        target_port: None,
        target_host: None,
    }
}

fn counts_of(report: &SourcesReport, address: &str) -> Counts {
    report
        .addresses
        .items
        .iter()
        .find(|item| item.ip == ip(address))
        .map(|item| item.counts)
        .unwrap_or_default()
}

fn counts(attempts: u64, rejected: u64, aborted: u64, sessions: u64) -> Counts {
    Counts {
        attempts,
        rejected,
        aborted,
        sessions,
    }
}

/// spec: operations.md:OPS-098 partial
/// spec: management.md:MGT-063 partial
#[test]
fn events_count_by_the_rules_of_each_kind() {
    let fixture = fixture();
    // TCP accept: an attempt at once.
    fixture.accept(ListenerKind::TcpViewer, "198.51.100.1");
    // Refused at accept: an attempt and a refusal.
    fixture.refuse_at_accept("ws", "198.51.100.2");
    // Management and status listeners are not relay connections.
    fixture.refuse_at_accept("management", "198.51.100.3");
    fixture.refuse_at_accept("status", "198.51.100.3");
    // A WS connection through a trusted proxy counts the forwarded address.
    let proxied = fixture.accept(ListenerKind::Ws, "192.0.2.10");
    fixture.stats.record(&Event::UpgradeRejected {
        listener: "ws",
        role: Some(Role::Viewer),
        conn_id: proxied,
        reason: RejectReason::IpNotAllowed,
        peer_ip: peer("192.0.2.10"),
        forwarded_ip: Some(peer("203.0.113.5")),
        origin: None,
        ticket_jti: None,
    });
    // A WSS refusal in the TLS stage settles its held accept.
    let tls = fixture.accept(ListenerKind::Wss, "198.51.100.4");
    fixture.stats.record(&Event::ConnectionRejected {
        listener: "wss",
        conn_id: Some(tls),
        stage: Stage::Tls,
        reason: RejectReason::TlsVersionUnsupported,
        peer_ip: peer("198.51.100.4"),
    });
    // An aborted TCP connection.
    let aborted = fixture.accept(ListenerKind::TcpServer, "198.51.100.6");
    fixture.abort(aborted, ListenerKind::TcpServer, "198.51.100.6", None);
    // Mode II: a WS Viewer and a TCP Server, one session.
    let viewer = fixture.accept(ListenerKind::Ws, "198.51.100.7");
    let server = fixture.accept(ListenerKind::TcpServer, "198.51.100.8");
    fixture.stats.record(&session(
        Mode::Mode2,
        (viewer, "198.51.100.7"),
        Some((server, "198.51.100.8")),
    ));
    // Mode I: a TCP Viewer, one session.
    let mode1 = fixture.accept(ListenerKind::TcpViewer, "198.51.100.9");
    fixture
        .stats
        .record(&session(Mode::Mode1, (mode1, "198.51.100.9"), None));

    let report = fixture.report(SourceWindow::Hour);

    assert_eq!(report.totals, counts(8, 3, 1, 2));
    assert_eq!(counts_of(&report, "198.51.100.1"), counts(1, 0, 0, 0));
    assert_eq!(counts_of(&report, "198.51.100.2"), counts(1, 1, 0, 0));
    assert_eq!(counts_of(&report, "198.51.100.3"), Counts::default());
    assert_eq!(counts_of(&report, "192.0.2.10"), Counts::default());
    assert_eq!(counts_of(&report, "203.0.113.5"), counts(1, 1, 0, 0));
    assert_eq!(counts_of(&report, "198.51.100.4"), counts(1, 1, 0, 0));
    assert_eq!(counts_of(&report, "198.51.100.6"), counts(1, 0, 1, 0));
    assert_eq!(counts_of(&report, "198.51.100.7"), counts(1, 0, 0, 1));
    assert_eq!(counts_of(&report, "198.51.100.8"), counts(1, 0, 0, 1));
    assert_eq!(counts_of(&report, "198.51.100.9"), counts(1, 0, 0, 1));
    assert_eq!(report.addresses.total, 8);
    // 8 attempts, 3 refusals, 1 abort, 2 Viewer and 1 Server session sides.
    assert_eq!(report.stored, 15);
}

/// spec: operations.md:OPS-098 partial
/// spec: management.md:MGT-064 partial
#[test]
fn a_held_accept_counts_at_its_accept_time() {
    let fixture = fixture();
    let held = fixture.accept(ListenerKind::Wss, "198.51.100.20");
    fixture.clock.advance(Duration::from_secs(3601));
    fixture.accept(ListenerKind::TcpViewer, "198.51.100.21");
    fixture.abort(held, ListenerKind::Wss, "198.51.100.20", None);

    let hour = fixture.report(SourceWindow::Hour);
    let as_of = START_MS + 3_601_000;
    assert_eq!(hour.as_of, at(as_of));
    assert_eq!(hour.window_start, at(as_of - 3_600_000));
    // The attempt is one second older than the window.
    assert_eq!(counts_of(&hour, "198.51.100.20"), counts(0, 0, 1, 0));
    assert_eq!(hour.totals, counts(1, 0, 1, 0));

    let day = fixture.report(SourceWindow::Day);
    assert_eq!(day.window_start, at(as_of - 86_400_000));
    assert_eq!(counts_of(&day, "198.51.100.20"), counts(1, 0, 1, 0));
    let item = day
        .addresses
        .items
        .iter()
        .find(|item| item.ip == ip("198.51.100.20"))
        .expect("held address");
    assert_eq!(item.last_seen_at, at(as_of));
}

/// spec: management.md:MGT-064 partial
#[test]
fn the_window_starts_inclusive() {
    let fixture = fixture();
    fixture.accept(ListenerKind::TcpViewer, "198.51.100.22");
    fixture.clock.advance(Duration::from_secs(3600));

    assert_eq!(fixture.report(SourceWindow::Hour).totals.attempts, 1);
    fixture.clock.advance(Duration::from_millis(1));
    assert_eq!(fixture.report(SourceWindow::Hour).totals.attempts, 0);
    assert_eq!(fixture.report(SourceWindow::Day).totals.attempts, 1);
}

/// spec: operations.md:OPS-098 partial
#[test]
fn too_many_held_accepts_count_the_oldest_with_its_socket_address() {
    let fixture = fixture();
    fixture.accept(ListenerKind::Ws, "198.51.100.30");
    for _ in 0..PENDING_CAPACITY {
        fixture.accept(ListenerKind::Ws, "198.51.100.31");
    }

    let report = fixture.report(SourceWindow::Hour);
    assert_eq!(report.totals.attempts, 1);
    assert_eq!(counts_of(&report, "198.51.100.30"), counts(1, 0, 0, 0));
}

/// spec: operations.md:OPS-099 partial
/// spec: management.md:MGT-064 partial
#[test]
fn the_oldest_events_go_first_over_the_limit() {
    let fixture = fixture();
    assert_eq!(fixture.report(SourceWindow::Day).stored, 0);
    // Held at START_MS, before every later event.
    let held = fixture.accept(ListenerKind::Ws, "198.51.100.40");
    for _ in 0..CAPACITY {
        fixture.clock.advance(Duration::from_millis(1));
        fixture.accept(ListenerKind::TcpViewer, "198.51.100.41");
    }
    let full = fixture.report(SourceWindow::Day);
    assert_eq!(full.stored, CAPACITY);
    assert_eq!(full.dropped_before, None);

    // The held attempt goes in first, is the oldest, and goes out at once.
    // The abort then pushes out the oldest TCP attempt.
    fixture.abort(held, ListenerKind::Ws, "198.51.100.40", None);

    let report = fixture.report(SourceWindow::Day);
    assert_eq!(report.capacity, 100_000);
    assert_eq!(report.stored, CAPACITY);
    assert_eq!(report.dropped_before, Some(at(START_MS + 1)));
    assert_eq!(counts_of(&report, "198.51.100.40"), counts(0, 0, 1, 0));
    assert_eq!(
        counts_of(&report, "198.51.100.41").attempts,
        u64::try_from(CAPACITY - 1).expect("small")
    );
}

/// spec: management.md:MGT-063 partial
#[test]
fn the_query_sorts_cuts_and_names_by_the_current_database() {
    let fixture = fixture();
    let address = |n: u8| IpAddr::V4(Ipv4Addr::new(198, 51, 100, n));
    let country = |n: u8| match n % 3 {
        0 => None,
        1 => CountryCode::parse("KR"),
        _ => CountryCode::parse("US"),
    };
    // Private-use AS numbers (RFC 6996): more than 100 are needed.
    let asn = |n: u8| {
        (n % 10 != 9)
            .then(|| 4_200_000_000 + u32::from(n))
            .and_then(Asn::new)
    };
    fixture.use_table(
        (0..150)
            .map(|n| (address(n), (country(n), asn(n))))
            .chain([(ip("10.0.0.1"), (CountryCode::parse("KR"), Asn::new(64_496)))])
            .collect(),
    );
    for _ in 0..10 {
        fixture.clock.advance(Duration::from_millis(1));
        fixture.accept(ListenerKind::TcpViewer, "10.0.0.1");
    }
    for n in 0..150 {
        for _ in 0..=(n % 4) {
            fixture.clock.advance(Duration::from_millis(1));
            fixture.accept(ListenerKind::TcpViewer, &address(n).to_string());
        }
    }
    // A newer ASN database moves 198.51.100.147 to another AS.
    let mut newer: HashMap<IpAddr, (Option<CountryCode>, Option<Asn>)> = (0..150)
        .map(|n| (address(n), (country(n), asn(n))))
        .collect();
    newer.insert(address(147), (CountryCode::parse("US"), Asn::new(64_497)));
    fixture.use_table(newer);

    let report = fixture.report(SourceWindow::Hour);

    // Addresses: 151 in all, the first 100 by attempts, then latest event.
    assert_eq!(report.addresses.total, 151);
    assert_eq!(report.addresses.items.len(), 100);
    let first = &report.addresses.items[0];
    assert_eq!(first.ip, ip("10.0.0.1"));
    assert!(first.local);
    assert_eq!(
        (first.country, first.asn, &first.as_name),
        (None, None, &None)
    );
    let rest: Vec<(u64, SystemTime)> = report.addresses.items[1..]
        .iter()
        .map(|item| (item.counts.attempts, item.last_seen_at))
        .collect();
    assert!(
        rest.windows(2)
            .all(|pair| pair[0].0 > pair[1].0 || (pair[0].0 == pair[1].0 && pair[0].1 > pair[1].1))
    );
    assert_eq!(rest[0].0, 4);
    let renamed = report
        .addresses
        .items
        .iter()
        .find(|item| item.ip == address(147))
        .expect("198.51.100.147 has 4 attempts");
    // The row keeps the AS number found when its events were recorded; the
    // current database names another AS, so no name is shown.
    assert_eq!(renamed.asn, Asn::new(4_200_000_147));
    assert_eq!(renamed.as_name, None);
    let named = report
        .addresses
        .items
        .iter()
        .find(|item| item.ip == address(143))
        .expect("198.51.100.143 has 4 attempts");
    assert_eq!(named.as_name.as_deref(), Some("Example Net 4200000143"));

    // Countries: every group, by attempts, then code with null last.
    let mut expected: BTreeMap<Option<&str>, (u64, usize)> = BTreeMap::new();
    for n in 0..150u8 {
        let entry = expected
            .entry(country(n).map(|code| if code.as_str() == "KR" { "KR" } else { "US" }))
            .or_default();
        entry.0 += u64::from(n % 4) + 1;
        entry.1 += 1;
    }
    expected.entry(None).or_default().0 += 10;
    expected.entry(None).or_default().1 += 1;
    assert_eq!(report.countries.total, 3);
    for item in &report.countries.items {
        let key = item
            .country
            .map(|code| if code.as_str() == "KR" { "KR" } else { "US" });
        assert_eq!((item.counts.attempts, item.addresses), expected[&key]);
    }
    assert!(report.countries.items.windows(2).all(|pair| {
        pair[0].counts.attempts > pair[1].counts.attempts
            || (pair[0].counts.attempts == pair[1].counts.attempts
                && match (pair[0].country, pair[1].country) {
                    (Some(left), Some(right)) => left < right,
                    (left, right) => left.is_some() || right.is_none(),
                })
    }));

    // ASNs: 135 numbers and null, cut to 100, null after equal numbers.
    assert_eq!(report.asns.total, 136);
    assert_eq!(report.asns.items.len(), 100);
    let null = report
        .asns
        .items
        .iter()
        .position(|item| item.asn.is_none())
        .expect("null ASN has the most attempts");
    assert_eq!(null, 0);
    assert_eq!(report.asns.items[0].addresses, 16);
    assert!(report.asns.items[1..].windows(2).all(|pair| {
        pair[0].counts.attempts > pair[1].counts.attempts
            || (pair[0].counts.attempts == pair[1].counts.attempts && pair[0].asn < pair[1].asn)
    }));

    // No database file is configured in this fixture.
    assert!(
        report
            .databases
            .iter()
            .all(|status| status.in_use.is_none())
    );
}

fn deny(texts: &[&str]) -> vnc_relay_domain::deny_list::DenyList {
    vnc_relay_domain::deny_list::DenyList::new(
        texts
            .iter()
            .map(|text| {
                let item = vnc_relay_domain::allow_list::IpItem::parse(text)
                    .unwrap_or_else(|error| panic!("{text}: {error:?}"));
                vnc_relay_domain::deny_list::DenyEntry::new((*text).to_owned(), item)
            })
            .collect(),
    )
}

fn deny_entries(report: &SourcesReport, address: &str) -> Vec<String> {
    report
        .addresses
        .items
        .iter()
        .find(|item| item.ip == ip(address))
        .map(|item| item.deny_entries.clone())
        .unwrap_or_else(|| panic!("missing address {address}"))
}

/// spec: management.md:MGT-065 partial
#[test]
fn each_address_lists_the_running_deny_items_that_cover_it() {
    let fixture = fixture();
    for address in [
        "203.0.113.9",
        "198.51.100.7",
        "203.0.113.50",
        "2001:db8::5",
        "192.168.0.15",
    ] {
        fixture.accept(ListenerKind::TcpViewer, address);
    }
    for edit in [false, true] {
        fixture.settings.change(|settings| {
            settings.management.config_edit = edit;
            settings.relay.deny = deny(&[
                "203.0.113.9",
                "198.51.100.0/24",
                "2001:DB8::1-2001:db8::ff",
                "192.168.0.0/16",
            ]);
        });
        let report = fixture.service.report(SourceWindow::Hour);
        assert_eq!(report.deny_editable, edit);
        assert_eq!(deny_entries(&report, "203.0.113.9"), ["203.0.113.9"]);
        assert_eq!(deny_entries(&report, "198.51.100.7"), ["198.51.100.0/24"]);
        assert_eq!(
            deny_entries(&report, "2001:db8::5"),
            ["2001:DB8::1-2001:db8::ff"]
        );
        assert!(deny_entries(&report, "203.0.113.50").is_empty());
        // Local addresses are filled the same way.
        assert_eq!(deny_entries(&report, "192.168.0.15"), ["192.168.0.0/16"]);
    }
    fixture
        .settings
        .change(|settings| settings.relay.deny = deny(&["203.0.113.9", "203.0.113.0/24"]));
    let report = fixture.service.report(SourceWindow::Hour);
    assert_eq!(
        deny_entries(&report, "203.0.113.9"),
        ["203.0.113.9", "203.0.113.0/24"]
    );
    assert_eq!(deny_entries(&report, "203.0.113.50"), ["203.0.113.0/24"]);
    fixture
        .settings
        .change(|settings| settings.relay.deny = deny(&[]));
    let report = fixture.service.report(SourceWindow::Hour);
    assert!(
        report
            .addresses
            .items
            .iter()
            .all(|item| item.deny_entries.is_empty())
    );
}
