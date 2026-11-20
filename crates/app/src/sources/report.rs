//! The sources query: kept events of one window counted per address,
//! country, and AS number (management-api-v1 §1.17).
//! Design: docs/architectures/source-stats.md §4.2.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use vnc_relay_domain::ip_info::{Asn, CountryCode, is_local};

use super::{CAPACITY, Kind, SourceEvent, SourceStats, lock, millis};
use crate::ip_info::{IpDbStatus, IpInfoService};

/// Most address and ASN items in one answer.
const ITEM_LIMIT: usize = 100;

/// The `window` of the query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceWindow {
    Hour,
    Day,
}

impl SourceWindow {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hour => "1h",
            Self::Day => "24h",
        }
    }

    pub fn length(self) -> Duration {
        match self {
            Self::Hour => Duration::from_secs(3600),
            Self::Day => Duration::from_secs(86_400),
        }
    }
}

/// Event counts of one item or of the whole window.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub attempts: u64,
    pub rejected: u64,
    pub aborted: u64,
    /// Items count both session sides; `totals` counts started sessions.
    pub sessions: u64,
}

/// One item list with the number of distinct keys in the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing<T> {
    pub total: usize,
    pub items: Vec<T>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressItem {
    pub ip: IpAddr,
    pub local: bool,
    /// From the address's latest event.
    pub country: Option<CountryCode>,
    pub asn: Option<Asn>,
    pub as_name: Option<String>,
    pub counts: Counts,
    pub last_seen_at: SystemTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountryItem {
    pub country: Option<CountryCode>,
    pub addresses: usize,
    pub counts: Counts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsnItem {
    pub asn: Option<Asn>,
    pub as_name: Option<String>,
    pub addresses: usize,
    pub counts: Counts,
}

/// The `200` body of management-api-v1 §1.17 before JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcesReport {
    pub as_of: SystemTime,
    pub window: SourceWindow,
    pub window_start: SystemTime,
    pub capacity: usize,
    pub stored: usize,
    pub dropped_before: Option<SystemTime>,
    /// Country first, then ASN.
    pub databases: [IpDbStatus; 2],
    pub totals: Counts,
    pub addresses: Listing<AddressItem>,
    pub countries: Listing<CountryItem>,
    pub asns: Listing<AsnItem>,
}

/// Read side of the source statistics.
pub struct SourcesService {
    stats: Arc<SourceStats>,
    ip_info: Arc<IpInfoService>,
}

/// Running counts of one address.
struct AddressTally {
    counts: Counts,
    last_at: u64,
    country: Option<CountryCode>,
    asn: Option<Asn>,
}

/// Running counts of one country or AS number.
#[derive(Default)]
struct GroupTally {
    counts: Counts,
    addresses: usize,
    /// The address of the group's latest event, for the AS name.
    last_ip: Option<IpAddr>,
}

impl SourcesService {
    pub fn new(stats: Arc<SourceStats>, ip_info: Arc<IpInfoService>) -> Self {
        Self { stats, ip_info }
    }

    /// Counts the events whose time is at least `window_start` and at most
    /// `as_of`. The store lock is held only to copy those events.
    pub fn report(&self, window: SourceWindow) -> SourcesReport {
        let as_of = millis(self.stats.clock.wall());
        let start = as_of.saturating_sub(window_millis(window));
        let (events, stored, dropped_before) = {
            let store = lock(&self.stats.store);
            let from = store.events.partition_point(|event| event.at < start);
            let to = store.events.partition_point(|event| event.at <= as_of);
            let events: Vec<SourceEvent> =
                store.events.range(from..to.max(from)).copied().collect();
            (events, store.events.len(), store.dropped_before)
        };

        let mut totals = Counts::default();
        let mut by_address: HashMap<IpAddr, AddressTally> = HashMap::new();
        let mut by_country: HashMap<Option<CountryCode>, GroupTally> = HashMap::new();
        let mut by_asn: HashMap<Option<Asn>, GroupTally> = HashMap::new();
        let mut country_pairs: HashSet<(Option<CountryCode>, IpAddr)> = HashSet::new();
        let mut asn_pairs: HashSet<(Option<Asn>, IpAddr)> = HashSet::new();
        for event in &events {
            add(&mut totals, event.kind, true);
            let address = by_address.entry(event.ip).or_insert(AddressTally {
                counts: Counts::default(),
                last_at: event.at,
                country: event.country,
                asn: event.asn,
            });
            add(&mut address.counts, event.kind, false);
            // Events come in time order, so the last one seen is the latest.
            address.last_at = event.at;
            address.country = event.country;
            address.asn = event.asn;

            let country = by_country.entry(event.country).or_default();
            add(&mut country.counts, event.kind, false);
            if country_pairs.insert((event.country, event.ip)) {
                country.addresses += 1;
            }
            let asn = by_asn.entry(event.asn).or_default();
            add(&mut asn.counts, event.kind, false);
            asn.last_ip = Some(event.ip);
            if asn_pairs.insert((event.asn, event.ip)) {
                asn.addresses += 1;
            }
        }

        let lookup = &self.stats.lookup;
        let name_of = |ip: IpAddr, asn: Option<Asn>| -> Option<String> {
            let asn = asn?;
            lookup
                .as_name(ip)
                .and_then(|(found, name)| (found == asn).then_some(name))
        };

        let address_total = by_address.len();
        let mut addresses: Vec<(IpAddr, AddressTally)> = by_address.into_iter().collect();
        addresses.sort_by(|(left_ip, left), (right_ip, right)| {
            right
                .counts
                .attempts
                .cmp(&left.counts.attempts)
                .then(right.last_at.cmp(&left.last_at))
                .then(left_ip.cmp(right_ip))
        });
        addresses.truncate(ITEM_LIMIT);
        let addresses = addresses
            .into_iter()
            .map(|(ip, tally)| AddressItem {
                ip,
                local: is_local(ip),
                country: tally.country,
                asn: tally.asn,
                as_name: name_of(ip, tally.asn),
                counts: tally.counts,
                last_seen_at: at_millis(tally.last_at),
            })
            .collect();

        let country_total = by_country.len();
        let mut countries: Vec<CountryItem> = by_country
            .into_iter()
            .map(|(country, tally)| CountryItem {
                country,
                addresses: tally.addresses,
                counts: tally.counts,
            })
            .collect();
        countries.sort_by(|left, right| {
            right
                .counts
                .attempts
                .cmp(&left.counts.attempts)
                .then(null_last(&left.country, &right.country))
        });

        let asn_total = by_asn.len();
        let mut asns: Vec<(Option<Asn>, GroupTally)> = by_asn.into_iter().collect();
        asns.sort_by(|(left_asn, left), (right_asn, right)| {
            right
                .counts
                .attempts
                .cmp(&left.counts.attempts)
                .then(null_last(left_asn, right_asn))
        });
        asns.truncate(ITEM_LIMIT);
        let asns = asns
            .into_iter()
            .map(|(asn, tally)| AsnItem {
                asn,
                as_name: tally.last_ip.and_then(|ip| name_of(ip, asn)),
                addresses: tally.addresses,
                counts: tally.counts,
            })
            .collect();

        SourcesReport {
            as_of: at_millis(as_of),
            window,
            window_start: at_millis(start),
            capacity: CAPACITY,
            stored,
            dropped_before: dropped_before.map(at_millis),
            databases: self.ip_info.status(),
            totals,
            addresses: Listing {
                total: address_total,
                items: addresses,
            },
            countries: Listing {
                total: country_total,
                items: countries,
            },
            asns: Listing {
                total: asn_total,
                items: asns,
            },
        }
    }
}

/// Adds one event. `totals` counts started sessions, so only the Viewer
/// side, which every session has, counts there (management-api-v1 §1.17).
fn add(counts: &mut Counts, kind: Kind, totals: bool) {
    match kind {
        Kind::Attempt => counts.attempts += 1,
        Kind::Rejected => counts.rejected += 1,
        Kind::Aborted => counts.aborted += 1,
        Kind::ViewerSession => counts.sessions += 1,
        Kind::ServerSession if !totals => counts.sessions += 1,
        Kind::ServerSession => {}
    }
}

/// Ascending code or number; `None` after every value.
fn null_last<T: Ord>(left: &Option<T>, right: &Option<T>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn window_millis(window: SourceWindow) -> u64 {
    u64::try_from(window.length().as_millis()).unwrap_or(u64::MAX)
}

fn at_millis(at: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_millis(at)
}
