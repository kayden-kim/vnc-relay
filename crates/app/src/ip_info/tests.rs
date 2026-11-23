//! Reading, keeping, and replacing IP information databases without files.

use std::collections::HashMap;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use vnc_relay_domain::ip_info::{Asn, CountryCode, Provider};

use super::{
    Fingerprint, IpDbError, IpDbFailure, IpDbKey, IpDbMetadata, IpDbRead, IpDbState, IpInfoService,
    IpLookup,
};
use crate::events::{Event, Phase};
use crate::ports::{Clock, EventSink, IpDbFileGateway, IpDbLookupGateway};

const WALL: u64 = 1_790_000_000;

struct FixedClock;

impl Clock for FixedClock {
    fn monotonic(&self) -> Duration {
        Duration::ZERO
    }

    fn wall(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(WALL)
    }
}

#[derive(Default)]
struct Recorded(Mutex<Vec<Event>>);

impl EventSink for Recorded {
    fn record(&self, event: &Event) {
        self.0.lock().expect("events lock").push(event.clone());
    }
}

impl Recorded {
    fn take(&self) -> Vec<Event> {
        std::mem::take(&mut *self.0.lock().expect("events lock"))
    }
}

/// A database with one record for every address.
struct OneRecord {
    metadata: IpDbMetadata,
    country: Option<CountryCode>,
    asn: Option<(Asn, String)>,
}

impl IpDbLookupGateway for OneRecord {
    fn metadata(&self) -> &IpDbMetadata {
        &self.metadata
    }

    fn supports(&self, key: IpDbKey) -> bool {
        match key {
            IpDbKey::Country => self.country.is_some(),
            IpDbKey::Asn => self.asn.is_some(),
        }
    }

    fn country(&self, _ip: IpAddr) -> Option<CountryCode> {
        self.country
    }

    fn asn(&self, _ip: IpAddr) -> Option<Asn> {
        self.asn.as_ref().map(|(asn, _)| *asn)
    }

    fn as_name(&self, _ip: IpAddr) -> Option<(Asn, String)> {
        self.asn.clone()
    }
}

/// What each path holds now. Counts the reads.
#[derive(Default)]
struct Files {
    content: Mutex<HashMap<PathBuf, Content>>,
    reads: Mutex<usize>,
}

#[derive(Clone)]
enum Content {
    /// A valid database: its content hash, type, country, and AS number.
    Db(u64, &'static str, Option<&'static str>, Option<u32>),
    Broken(u64),
    Missing,
}

impl Files {
    fn put(&self, path: &str, content: Content) {
        self.content
            .lock()
            .expect("files lock")
            .insert(PathBuf::from(path), content);
    }

    fn reads(&self) -> usize {
        *self.reads.lock().expect("reads lock")
    }
}

impl IpDbFileGateway for Files {
    fn read(&self, path: &Path) -> IpDbRead {
        *self.reads.lock().expect("reads lock") += 1;
        let content = self
            .content
            .lock()
            .expect("files lock")
            .get(path)
            .cloned()
            .unwrap_or(Content::Missing);
        match content {
            Content::Db(hash, database_type, country, asn) => IpDbRead {
                fingerprint: Fingerprint::Content(hash),
                result: Ok(Arc::new(OneRecord {
                    metadata: IpDbMetadata::new(
                        database_type,
                        SystemTime::UNIX_EPOCH + Duration::from_secs(1_780_000_000),
                    ),
                    country: country.and_then(CountryCode::parse),
                    asn: asn
                        .and_then(Asn::new)
                        .map(|asn| (asn, format!("Example Net {}", asn.get()))),
                })),
            },
            Content::Broken(hash) => IpDbRead {
                fingerprint: Fingerprint::Content(hash),
                result: Err(IpDbError {
                    failure: IpDbFailure::InvalidFormat,
                    detail: Some("메타데이터를 찾지 못했다.".to_owned()),
                }),
            },
            Content::Missing => IpDbRead {
                fingerprint: Fingerprint::Unreadable,
                result: Err(IpDbError {
                    failure: IpDbFailure::FileUnreadable,
                    detail: Some("파일을 찾을 수 없다.".to_owned()),
                }),
            },
        }
    }
}

struct Fixture {
    files: Arc<Files>,
    lookup: Arc<IpLookup>,
    log: Arc<Recorded>,
    service: IpInfoService,
}

fn fixture(country_db: Option<&str>, asn_db: Option<&str>) -> Fixture {
    let files = Arc::new(Files::default());
    let lookup = Arc::new(IpLookup::new());
    let log = Arc::new(Recorded::default());
    let service = IpInfoService::new(
        country_db.map(PathBuf::from),
        asn_db.map(PathBuf::from),
        Arc::clone(&files) as Arc<dyn IpDbFileGateway>,
        Arc::clone(&lookup),
        Arc::new(FixedClock),
        Arc::clone(&log) as Arc<dyn EventSink>,
    );
    Fixture {
        files,
        lookup,
        log,
        service,
    }
}

fn public() -> IpAddr {
    "203.0.113.7".parse().expect("documentation address")
}

fn found(lookup: &IpLookup) -> (Option<&'static str>, Option<u32>) {
    let (country, asn) = lookup.find(public());
    let country = country.map(|code| match code.as_str() {
        "KR" => "KR",
        "US" => "US",
        _ => "other",
    });
    (country, asn.map(Asn::get))
}

/// spec: operations.md:OPS-095 partial
/// spec: operations.md:OPS-096 partial
#[test]
fn one_file_for_both_keys_is_read_once_and_logged_per_key() {
    let fixture = fixture(Some("ip.mmdb"), Some("ip.mmdb"));
    fixture.files.put(
        "ip.mmdb",
        Content::Db(1, "DBIP-ASN-Lite", Some("KR"), Some(64_496)),
    );

    fixture.service.load_startup();

    assert_eq!(fixture.files.reads(), 1);
    assert_eq!(fixture.service.paths(), vec![PathBuf::from("ip.mmdb")]);
    let events = fixture.log.take();
    let keys: Vec<_> = events
        .iter()
        .map(|event| match event {
            Event::IpDbLoaded {
                key,
                database_type,
                phase: Phase::Startup,
                ..
            } => (*key, database_type.as_str()),
            other => panic!("unexpected event {other:?}"),
        })
        .collect();
    assert_eq!(
        keys,
        vec![
            (IpDbKey::Country, "DBIP-ASN-Lite"),
            (IpDbKey::Asn, "DBIP-ASN-Lite")
        ]
    );
    assert_eq!(found(&fixture.lookup), (Some("KR"), Some(64_496)));
    let status = fixture.service.status();
    for item in &status {
        assert_eq!(item.state, IpDbState::Loaded);
        assert_eq!(item.error, None);
        let in_use = item.in_use.as_ref().expect("database in use");
        assert_eq!(in_use.metadata.provider, Provider::Dbip);
        assert_eq!(
            in_use.loaded_at,
            SystemTime::UNIX_EPOCH + Duration::from_secs(WALL)
        );
    }
}

/// spec: operations.md:OPS-096 partial
/// spec: management.md:MGT-063 partial
#[test]
fn missing_and_unsupported_files_warn_and_leave_values_empty() {
    let fixture = fixture(Some("asn-only.mmdb"), Some("missing.mmdb"));
    fixture.files.put(
        "asn-only.mmdb",
        Content::Db(1, "GeoLite2-ASN", None, Some(64_500)),
    );

    fixture.service.load_startup();

    let events = fixture.log.take();
    assert_eq!(
        events,
        vec![
            Event::IpDbUnavailable {
                key: IpDbKey::Country,
                reason: IpDbFailure::UnsupportedData,
                phase: Phase::Startup,
                detail: Some(
                    "국가 필드(country_code 또는 country.iso_code)를 찾지 못했다.".to_owned()
                ),
            },
            Event::IpDbUnavailable {
                key: IpDbKey::Asn,
                reason: IpDbFailure::FileUnreadable,
                phase: Phase::Startup,
                detail: Some("파일을 찾을 수 없다.".to_owned()),
            },
        ]
    );
    assert_eq!(found(&fixture.lookup), (None, None));
    let status = fixture.service.status();
    assert_eq!(status[0].state, IpDbState::Unavailable);
    assert_eq!(status[0].error, Some(IpDbFailure::UnsupportedData));
    assert_eq!(status[1].error, Some(IpDbFailure::FileUnreadable));
    assert!(status.iter().all(|item| item.in_use.is_none()));
}

/// spec: management.md:MGT-063 partial
#[test]
fn keys_without_a_path_are_not_configured() {
    let fixture = fixture(None, None);
    fixture.service.load_startup();

    assert!(fixture.log.take().is_empty());
    assert_eq!(fixture.files.reads(), 0);
    let status = fixture.service.status();
    assert!(
        status
            .iter()
            .all(|item| item.state == IpDbState::NotConfigured)
    );
    assert!(
        status
            .iter()
            .all(|item| item.error.is_none() && item.in_use.is_none())
    );
}

/// spec: operations.md:OPS-097 partial
#[test]
fn a_bad_replacement_keeps_the_previous_database_and_warns_once() {
    let fixture = fixture(Some("country.mmdb"), None);
    fixture.files.put(
        "country.mmdb",
        Content::Db(1, "DBIP-Country-Lite", Some("KR"), None),
    );
    fixture.service.load_startup();
    fixture.log.take();
    let changed = [PathBuf::from("country.mmdb")];

    fixture.files.put(
        "country.mmdb",
        Content::Db(2, "ipinfo lite.mmdb", Some("US"), None),
    );
    fixture.service.refresh(&changed);
    assert!(matches!(
        fixture.log.take().as_slice(),
        [Event::IpDbLoaded {
            key: IpDbKey::Country,
            phase: Phase::Reload,
            ..
        }]
    ));
    assert_eq!(found(&fixture.lookup).0, Some("US"));

    fixture.files.put("country.mmdb", Content::Broken(3));
    fixture.service.refresh(&changed);
    assert_eq!(
        fixture.log.take(),
        vec![Event::IpDbUnavailable {
            key: IpDbKey::Country,
            reason: IpDbFailure::InvalidFormat,
            phase: Phase::Reload,
            detail: Some("메타데이터를 찾지 못했다.".to_owned()),
        }]
    );
    assert_eq!(found(&fixture.lookup).0, Some("US"));
    let status = fixture.service.status();
    assert_eq!(status[0].state, IpDbState::Stale);
    assert_eq!(status[0].error, Some(IpDbFailure::InvalidFormat));
    let in_use = status[0].in_use.as_ref().expect("previous database");
    assert_eq!(in_use.metadata.provider, Provider::Ipinfo);

    // The same broken content again: no second warning.
    fixture.service.refresh(&changed);
    assert!(fixture.log.take().is_empty());

    // Back to the content in use: the warning ends with a loaded line.
    fixture.files.put(
        "country.mmdb",
        Content::Db(2, "ipinfo lite.mmdb", Some("US"), None),
    );
    fixture.service.refresh(&changed);
    assert_eq!(fixture.log.take().len(), 1);
    assert_eq!(fixture.service.status()[0].state, IpDbState::Loaded);

    // Unchanged content and no failure: nothing to log.
    fixture.service.refresh(&changed);
    assert!(fixture.log.take().is_empty());
}

/// spec: operations.md:OPS-097 partial
#[test]
fn refresh_reads_only_the_changed_paths() {
    let fixture = fixture(Some("country.mmdb"), Some("asn.mmdb"));
    fixture.files.put(
        "country.mmdb",
        Content::Db(1, "GeoLite2-Country", Some("KR"), None),
    );
    fixture.files.put(
        "asn.mmdb",
        Content::Db(2, "GeoLite2-ASN", None, Some(64_501)),
    );
    fixture.service.load_startup();
    assert_eq!(fixture.files.reads(), 2);

    fixture.service.refresh(&[PathBuf::from("asn.mmdb")]);
    assert_eq!(fixture.files.reads(), 3);
    fixture.service.refresh(&[PathBuf::from("other.mmdb")]);
    assert_eq!(fixture.files.reads(), 3);
}

/// spec: management.md:MGT-063 partial
#[test]
fn local_addresses_are_never_looked_up() {
    let fixture = fixture(Some("ip.mmdb"), Some("ip.mmdb"));
    fixture.files.put(
        "ip.mmdb",
        Content::Db(1, "DBIP-ASN-Lite", Some("KR"), Some(64_496)),
    );
    fixture.service.load_startup();

    for local in [
        "127.0.0.1",
        "10.1.2.3",
        "192.168.0.9",
        "fe80::1",
        "100.64.0.1",
    ] {
        let ip: IpAddr = local.parse().expect("local address");
        assert_eq!(fixture.lookup.find(ip), (None, None), "{local}");
        assert_eq!(fixture.lookup.as_name(ip), None, "{local}");
    }
    assert_eq!(
        fixture.lookup.as_name(public()),
        Asn::new(64_496).map(|asn| (asn, "Example Net 64496".to_owned()))
    );
}

#[test]
fn metadata_keeps_the_first_128_characters_and_the_whole_type_for_the_provider() {
    let long = format!("GeoLite2-Country{}", "x".repeat(200));
    let metadata = IpDbMetadata::new(&long, SystemTime::UNIX_EPOCH);
    assert_eq!(metadata.database_type.chars().count(), 128);
    assert_eq!(metadata.provider, Provider::MaxmindGeolite);
}
