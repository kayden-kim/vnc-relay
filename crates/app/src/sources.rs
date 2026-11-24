//! Connection source statistics (operations.md §3.9): relay connection
//! events kept per client address with the country and AS number found
//! when each event was recorded. Design: docs/architectures/source-stats.md.
//!
//! `SourcesSink` hands every event to `SourceStats` on its way to the log
//! writer. `SourcesService` counts the kept events of one window on demand.

pub mod report;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, VecDeque};
use std::net::IpAddr;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use vnc_relay_domain::conn_id::ConnId;
use vnc_relay_domain::entry_point::ListenerKind;
use vnc_relay_domain::ip_info::{Asn, CountryCode};
use vnc_relay_domain::peer_ip::PeerIp;

use crate::events::{Event, Stage};
use crate::ip_info::IpLookup;
use crate::ports::{Clock, EventSink};

pub use report::{SourceWindow, SourcesReport, SourcesService};

/// Most events kept (operations.md §3.9 rule 6). Not configurable.
pub const CAPACITY: usize = 100_000;

/// Most WS and WSS accepts that wait for their connection's next event
/// (operations.md §3.9 rule 4).
const PENDING_CAPACITY: usize = 65_536;

/// What one kept event counts (operations.md §3.9 rule 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Attempt,
    Rejected,
    Aborted,
    /// The Viewer side of a started session; one per session.
    ViewerSession,
    /// The Mode II Server side of a started session.
    ServerSession,
}

/// One kept event: 40 bytes (checked below), so `CAPACITY` events take
/// about 4 MB.
#[derive(Debug, Clone, Copy)]
struct SourceEvent {
    /// UTC milliseconds since 1970.
    at: u64,
    ip: IpAddr,
    kind: Kind,
    country: Option<CountryCode>,
    asn: Option<Asn>,
}

const _: () = assert!(std::mem::size_of::<SourceEvent>() <= 40);

/// A WS or WSS accept: its time and socket peer address.
#[derive(Debug, Clone, Copy)]
struct Pending {
    at: u64,
    peer: IpAddr,
}

struct Store {
    /// Kept events in time order, oldest first.
    events: VecDeque<SourceEvent>,
    /// `ConnId` grows in accept order, so the first key is the oldest.
    pending: BTreeMap<ConnId, Pending>,
    /// Latest time among the events dropped over `CAPACITY`.
    dropped_before: Option<u64>,
}

/// The kept events of this process. Empty at start; a reload never
/// clears it (operations.md §3.9 rule 6).
pub struct SourceStats {
    lookup: Arc<IpLookup>,
    clock: Arc<dyn Clock>,
    store: Mutex<Store>,
}

/// What one event changes (source-stats.md §4.1).
#[derive(Default)]
struct Plan {
    /// A WS or WSS accept that waits for the next event of its connection.
    hold: Option<(ConnId, IpAddr)>,
    /// Held accepts this event settles, with the client address it gives.
    settle: [Option<(ConnId, IpAddr)>; 2],
    /// What the event itself counts at its own time.
    count: [Option<(Kind, IpAddr)>; 2],
}

impl SourceStats {
    pub fn new(lookup: Arc<IpLookup>, clock: Arc<dyn Clock>) -> Self {
        Self {
            lookup,
            clock,
            store: Mutex::new(Store {
                events: VecDeque::new(),
                pending: BTreeMap::new(),
                dropped_before: None,
            }),
        }
    }

    /// Keeps what `event` counts. Lookups run outside the lock; the lock is
    /// held only to move held accepts and to insert.
    pub fn record(&self, event: &Event) {
        let Some(plan) = plan(event) else {
            return;
        };
        let now = millis(self.clock.wall());
        let mut found: [Option<(u64, IpAddr, Kind)>; 5] = [None; 5];
        let mut next = 0;
        let mut push = |item: (u64, IpAddr, Kind)| {
            if let Some(slot) = found.get_mut(next) {
                *slot = Some(item);
                next += 1;
            }
        };
        if plan.hold.is_some() || plan.settle.iter().any(Option::is_some) {
            let mut store = lock(&self.store);
            if let Some((conn_id, peer)) = plan.hold {
                store.pending.insert(conn_id, Pending { at: now, peer });
                if store.pending.len() > PENDING_CAPACITY
                    && let Some((_, oldest)) = store.pending.pop_first()
                {
                    push((oldest.at, oldest.peer, Kind::Attempt));
                }
            }
            for (conn_id, client) in plan.settle.iter().flatten() {
                if let Some(held) = store.pending.remove(conn_id) {
                    push((held.at, *client, Kind::Attempt));
                }
            }
        }
        for (kind, client) in plan.count.iter().flatten() {
            push((now, *client, *kind));
        }
        if found[0].is_none() {
            return;
        }
        let mut events: [Option<SourceEvent>; 5] = [None; 5];
        // One event can add the same address twice; look it up once.
        let mut last: Option<SourceEvent> = None;
        for (slot, (at, ip, kind)) in events.iter_mut().zip(found.into_iter().flatten()) {
            let (country, asn) = match last {
                Some(seen) if seen.ip == ip => (seen.country, seen.asn),
                _ => self.lookup.find(ip),
            };
            last = Some(SourceEvent {
                at,
                ip,
                kind,
                country,
                asn,
            });
            *slot = last;
        }
        let mut store = lock(&self.store);
        for event in events.into_iter().flatten() {
            store.insert(event);
        }
    }
}

impl Store {
    /// Inserts in time order. A held accept is older than the event that
    /// settles it and records run on many threads, so an event can arrive
    /// later than a newer one. Over `CAPACITY` the oldest event goes.
    fn insert(&mut self, event: SourceEvent) {
        let index = self.events.partition_point(|kept| kept.at <= event.at);
        self.events.insert(index, event);
        if self.events.len() > CAPACITY
            && let Some(oldest) = self.events.pop_front()
        {
            self.dropped_before = Some(
                self.dropped_before
                    .map_or(oldest.at, |at| at.max(oldest.at)),
            );
        }
    }
}

/// The client address: the forwarded address when there is one, else the
/// socket peer address (operations.md §3.9 rule 3).
fn client(peer: PeerIp, forwarded: Option<PeerIp>) -> IpAddr {
    forwarded.unwrap_or(peer).ip()
}

fn is_relay_listener(name: &str) -> bool {
    ListenerKind::ALL
        .iter()
        .any(|listener| listener.name() == name)
}

/// The counting rules of operations.md §3.9 rules 2–4 for one event.
fn plan(event: &Event) -> Option<Plan> {
    let mut plan = Plan::default();
    match event {
        Event::ConnectionAccepted {
            conn_id,
            listener,
            peer_ip,
        } => match listener {
            ListenerKind::TcpViewer | ListenerKind::TcpServer => {
                plan.count[0] = Some((Kind::Attempt, peer_ip.ip()));
            }
            ListenerKind::Ws | ListenerKind::Wss => plan.hold = Some((*conn_id, peer_ip.ip())),
        },
        Event::ConnectionRejected {
            listener,
            conn_id,
            stage,
            peer_ip,
            ..
        } => {
            if !is_relay_listener(listener) {
                return None;
            }
            let ip = peer_ip.ip();
            if *stage == Stage::Accept {
                plan.count[0] = Some((Kind::Attempt, ip));
            }
            if let Some(conn_id) = conn_id {
                plan.settle[0] = Some((*conn_id, ip));
            }
            plan.count[1] = Some((Kind::Rejected, ip));
        }
        Event::UpgradeRejected {
            conn_id,
            peer_ip,
            forwarded_ip,
            ..
        }
        | Event::HandshakeRejected {
            conn_id,
            peer_ip,
            forwarded_ip,
            ..
        }
        | Event::TicketRelayMismatch {
            conn_id,
            peer_ip,
            forwarded_ip,
            ..
        }
        | Event::TargetRejected {
            conn_id,
            peer_ip,
            forwarded_ip,
            ..
        }
        | Event::RegistrationRejected {
            conn_id,
            peer_ip,
            forwarded_ip,
            ..
        } => {
            let ip = client(*peer_ip, *forwarded_ip);
            plan.settle[0] = Some((*conn_id, ip));
            plan.count[0] = Some((Kind::Rejected, ip));
        }
        Event::ConnectionAborted {
            conn_id,
            peer_ip,
            forwarded_ip,
            ..
        }
        | Event::RelayAborted {
            conn_id,
            peer_ip,
            forwarded_ip,
            ..
        } => {
            let ip = client(*peer_ip, *forwarded_ip);
            plan.settle[0] = Some((*conn_id, ip));
            plan.count[0] = Some((Kind::Aborted, ip));
        }
        Event::TicketAccepted {
            conn_id,
            peer_ip,
            forwarded_ip,
            ..
        }
        | Event::TargetConnectFailed {
            conn_id,
            peer_ip,
            forwarded_ip,
            ..
        }
        | Event::WaitingRegistered {
            conn_id,
            peer_ip,
            forwarded_ip,
            ..
        } => plan.settle[0] = Some((*conn_id, client(*peer_ip, *forwarded_ip))),
        Event::SessionStarted {
            viewer_conn_id,
            viewer_peer_ip,
            viewer_forwarded_ip,
            server_conn_id,
            server_peer_ip,
            server_forwarded_ip,
            ..
        } => {
            let viewer = client(*viewer_peer_ip, *viewer_forwarded_ip);
            plan.settle[0] = Some((*viewer_conn_id, viewer));
            plan.count[0] = Some((Kind::ViewerSession, viewer));
            if let (Some(conn_id), Some(peer)) = (server_conn_id, server_peer_ip) {
                let server = client(*peer, *server_forwarded_ip);
                plan.settle[1] = Some((*conn_id, server));
                plan.count[1] = Some((Kind::ServerSession, server));
            }
        }
        _ => return None,
    }
    Some(plan)
}

/// UTC milliseconds since 1970. Times before 1970 do not occur on a
/// running server and count as 0.
fn millis(at: SystemTime) -> u64 {
    at.duration_since(UNIX_EPOCH)
        .map(|since| u64::try_from(since.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

/// The operations-log writer with source counting in front. Like
/// `MetricsSink`, it counts whether or not the log write succeeds
/// (operations.md §3.9 rule 1).
pub struct SourcesSink {
    stats: Arc<SourceStats>,
    inner: Arc<dyn EventSink>,
}

impl SourcesSink {
    pub fn new(stats: Arc<SourceStats>, inner: Arc<dyn EventSink>) -> Self {
        Self { stats, inner }
    }
}

impl EventSink for SourcesSink {
    fn record(&self, event: &Event) {
        self.stats.record(event);
        self.inner.record(event);
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    // Nothing under this lock panics; a poisoned value is still whole.
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
