//! Initial exchange, target connection, matching, and relay lifetime ownership.
//! See docs/architectures/relay-session.md.

use std::collections::HashMap;
use std::fmt;
use std::future::poll_fn;
use std::net::SocketAddr;
use std::pin::{Pin, pin};
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::Poll;
use std::time::{Duration, SystemTime};

use tokio::sync::watch;
use vnc_relay_domain::account::AccountName;
use vnc_relay_domain::budget::{BudgetPermit, ConnectionBudget};
use vnc_relay_domain::conn_id::ConnId;
use vnc_relay_domain::entry_point::EntryPoint;
use vnc_relay_domain::handshake::{HandshakeError, HandshakePolicy, HandshakeRequest, Mode};
use vnc_relay_domain::matching::{MatchClaim, WaitingTable};
use vnc_relay_domain::peer_ip::PeerIp;
use vnc_relay_domain::recent_closed::RecentClosedList;
use vnc_relay_domain::relay_id::RelayId;
use vnc_relay_domain::session::{
    CloseCause, ClosedSession, Direction, Session, SessionError, SessionId, SessionState,
    ViewerTransport,
};
use vnc_relay_domain::target_policy::TargetPolicy;

use crate::events::{
    AbortReason, AdminTerminateResult, Event, RejectReason, RelayIdRef, ShutdownTrigger, Stage,
};
use crate::ports::{
    BoxRelayStream, Clock, EventSink, RelayCloseAction, RelayIdSource, RelayLifecycle,
    RelaySessionControl, RelayStateSource, RelayStream, SessionIdSource, TargetConnector,
};

mod handshake;
mod matching;
mod session;
mod target;

/// Accepted connection metadata. Both times use the service's shared Clock.
/// H starts at `exchange_at`; event elapsed time starts at `accepted_at`.
/// See repeater-handshake-v1 §1.2 and ops-info-v1 §1.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionContext {
    pub conn_id: ConnId,
    pub entry_point: EntryPoint,
    pub peer_ip: PeerIp,
    pub peer_addr: SocketAddr,
    pub accepted_wall: SystemTime,
    pub accepted_at: Duration,
    pub exchange_at: Duration,
}

/// A connection whose request is valid. The caller owns its next step.
/// The stream is dropped before its L permit, even when preparation is discarded.
/// See relay spec §3.1 and operations spec §2 (B and L).
pub struct PreparedConnection<S> {
    stream: S,
    context: ConnectionContext,
    request: HandshakeRequest,
    trailing: Vec<u8>,
    buffer_limit: usize,
    permit: BudgetPermit,
}

struct ConnectionResources<S> {
    stream: S,
    permit: BudgetPermit,
}

impl<S> PreparedConnection<S> {
    /// Metadata from the admitted connection (ops-info-v1 §1.5).
    pub fn context(&self) -> ConnectionContext {
        self.context
    }

    /// Parsed request without exposing padding (repeater-handshake-v1 §1.2).
    pub fn request(&self) -> &HandshakeRequest {
        &self.request
    }

    /// Already-read stream prefix; deliver it before later reads, once.
    /// See repeater-handshake-v1 §1.1 E.
    pub fn trailing_bytes(&self) -> &[u8] {
        &self.trailing
    }

    /// Bytes already charged against this connection's B (operations spec §2).
    pub fn buffered_bytes(&self) -> usize {
        self.trailing.len()
    }

    /// Space left under B before the stored prefix is consumed (operations spec §2).
    pub fn remaining_buffer_bytes(&self) -> usize {
        self.buffer_limit - self.trailing.len()
    }

    /// Underlying stream. Stored trailing bytes still precede its unread bytes.
    /// See repeater-handshake-v1 §1.1 E.
    pub fn stream_mut(&mut self) -> &mut S {
        &mut self.stream
    }

    /// Moves all connection resources without reacquiring or releasing L.
    /// The returned prefix must precede reads from the stream (relay spec §3.1).
    pub fn into_parts(
        self,
    ) -> (
        S,
        ConnectionContext,
        HandshakeRequest,
        Vec<u8>,
        BudgetPermit,
    ) {
        (
            self.stream,
            self.context,
            self.request,
            self.trailing,
            self.permit,
        )
    }
}

impl<S> fmt::Debug for PreparedConnection<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreparedConnection")
            .field("context", &self.context)
            .field("request", &self.request)
            .field("buffered_bytes", &self.trailing.len())
            .finish_non_exhaustive()
    }
}

/// One terminal initial-exchange result. Its event has already been recorded.
/// See repeater-handshake-v1 §2 and ops-info-v1 §1.5–1.7.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayError {
    Rejected(HandshakeError),
    Aborted(AbortReason),
}

impl RelayError {
    /// Stable contract reason without request contents (repeater-handshake-v1 §2).
    pub fn code(self) -> &'static str {
        match self {
            Self::Rejected(error) => error.code(),
            Self::Aborted(reason) => reason.as_str(),
        }
    }
}

impl fmt::Display for RelayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for RelayError {}

/// Prepares connections and executes their configured relay lifetime.
/// Preparation remains independently usable (initial-exchange.md §4–6).
pub struct RelayService {
    modes: HandshakePolicy,
    handshake: Duration,
    buffer_bytes: usize,
    close_timeout: Duration,
    clock: Arc<dyn Clock>,
    events: Arc<dyn EventSink>,
}

impl RelayService {
    /// Uses validated H, B, and mode settings. All contexts use `clock`'s epoch.
    /// See config-v3 §1.2 and repeater-handshake-v1 §1.2.
    pub fn new(
        modes: HandshakePolicy,
        handshake: Duration,
        buffer_bytes: usize,
        close_timeout: Duration,
        clock: Arc<dyn Clock>,
        events: Arc<dyn EventSink>,
    ) -> Self {
        Self {
            modes,
            handshake,
            buffer_bytes,
            close_timeout,
            clock,
            events,
        }
    }

    /// Writes the viewer greeting and reads a request within one absolute H.
    /// Failure drops the stream and L permit before one rejection/abort event.
    /// Success returns every resource without choosing the next relay step.
    /// See relay spec §3.1, repeater-handshake-v1 §1–2, and ops-info-v1 §1.5–1.7.
    pub fn handle_connection<S>(
        &self,
        stream: S,
        context: ConnectionContext,
        permit: BudgetPermit,
        shutdown: watch::Receiver<RelayLifecycle>,
    ) -> impl Future<Output = Result<PreparedConnection<S>, RelayError>> + Send
    where
        S: RelayStream,
    {
        self.prepare(ConnectionResources { stream, permit }, context, shutdown)
    }

    async fn prepare<S>(
        &self,
        mut resources: ConnectionResources<S>,
        context: ConnectionContext,
        shutdown: watch::Receiver<RelayLifecycle>,
    ) -> Result<PreparedConnection<S>, RelayError>
    where
        S: RelayStream,
    {
        let deadline = context.exchange_at.saturating_add(self.handshake);
        let result = handshake::exchange(
            &mut resources.stream,
            context.entry_point,
            self.modes,
            self.buffer_bytes,
            deadline,
            self.clock.as_ref(),
            shutdown.clone(),
        )
        .await;
        match result {
            Ok((request, trailing)) => {
                let ConnectionResources { stream, permit } = resources;
                Ok(PreparedConnection {
                    stream,
                    context,
                    request,
                    trailing,
                    buffer_limit: self.buffer_bytes,
                    permit,
                })
            }
            Err(error) => {
                let action = match error {
                    RelayError::Rejected(HandshakeError::ModeDisabled(_)) => {
                        RelayCloseAction::ModeDisabled
                    }
                    RelayError::Rejected(_) => RelayCloseAction::HandshakeInvalid,
                    RelayError::Aborted(AbortReason::HandshakeTimeout) => {
                        RelayCloseAction::HandshakeTimeout
                    }
                    RelayError::Aborted(AbortReason::PeerEof) => RelayCloseAction::PeerEof,
                    RelayError::Aborted(AbortReason::ConnectionError(kind)) => {
                        RelayCloseAction::StreamError(kind)
                    }
                    RelayError::Aborted(AbortReason::ServerShutdown) => {
                        RelayCloseAction::ServerShutdown
                    }
                    RelayError::Aborted(AbortReason::DeadConnection) => {
                        RelayCloseAction::DeadConnection
                    }
                    RelayError::Aborted(AbortReason::TlsUpgradeTimeout) => {
                        RelayCloseAction::HandshakeTimeout
                    }
                };
                self.close_stream(&mut resources.stream, action, self.close_timeout, shutdown)
                    .await;
                drop(resources);
                match error {
                    RelayError::Rejected(error) => {
                        let (reason, mode) = match error {
                            HandshakeError::ModeDisabled(mode) => {
                                (RejectReason::ModeDisabled, Some(mode))
                            }
                            HandshakeError::Invalid | HandshakeError::Incomplete => {
                                (RejectReason::HandshakeInvalid, None)
                            }
                        };
                        self.events.record(&Event::HandshakeRejected {
                            conn_id: context.conn_id,
                            entry_point: context.entry_point,
                            peer_ip: context.peer_ip,
                            reason,
                            mode,
                        });
                    }
                    RelayError::Aborted(reason) => {
                        let elapsed = self.clock.monotonic().saturating_sub(context.accepted_at);
                        self.events.record(&Event::ConnectionAborted {
                            conn_id: context.conn_id,
                            entry_point: context.entry_point,
                            stage: Stage::InitialExchange,
                            reason,
                            peer_ip: context.peer_ip,
                            elapsed_ms: u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX),
                        });
                    }
                }
                Err(error)
            }
        }
    }

    async fn close_stream<S: RelayStream + ?Sized>(
        &self,
        stream: &mut S,
        action: RelayCloseAction,
        timeout: Duration,
        lifecycle: watch::Receiver<RelayLifecycle>,
    ) {
        self.close_bounded(timeout, lifecycle, |cx| {
            Pin::new(&mut *stream).poll_close(cx, action).map(|_| ())
        })
        .await;
    }

    async fn close_bounded(
        &self,
        timeout: Duration,
        mut lifecycle: watch::Receiver<RelayLifecycle>,
        mut poll_close: impl FnMut(&mut std::task::Context<'_>) -> Poll<()>,
    ) {
        let deadline = self.clock.monotonic().saturating_add(timeout);
        let mut timer = pin!(tokio::time::sleep(timeout));
        let mut watch_closed = false;
        loop {
            let phase = *lifecycle.borrow_and_update();
            if phase == RelayLifecycle::Force {
                // Force bounds disposal, not the best-effort first Close.
                // Poll once without waiting so WS can still send 1001 when
                // an initial exchange is stopped with G = 0.
                poll_fn(|cx| {
                    let _ = poll_close(cx);
                    Poll::Ready(())
                })
                .await;
                return;
            }
            let grace = match phase {
                RelayLifecycle::Grace { deadline } => Some(deadline),
                RelayLifecycle::Running | RelayLifecycle::Force => None,
            };
            let mut grace_timer = pin!(async {
                match grace {
                    Some(end) => {
                        tokio::time::sleep(end.saturating_sub(self.clock.monotonic())).await
                    }
                    None => std::future::pending::<()>().await,
                }
            });
            let mut changed = pin!(lifecycle.changed());
            let complete = poll_fn(|cx| {
                if !watch_closed && let Poll::Ready(result) = changed.as_mut().poll(cx) {
                    watch_closed = result.is_err();
                    return Poll::Ready(false);
                }
                let now = self.clock.monotonic();
                if now >= deadline
                    || grace.is_some_and(|end| now >= end)
                    || timer.as_mut().poll(cx).is_ready()
                    || grace_timer.as_mut().poll(cx).is_ready()
                {
                    return Poll::Ready(true);
                }
                poll_close(cx).map(|()| true)
            })
            .await;
            if complete {
                return;
            }
        }
    }
}

/// Required dispatch settings; no prepare-only or disabled-dispatch defaults.
pub struct RelayRuntimeConfig {
    pub wait_timeout: Duration,
    pub connect_timeout: Duration,
    pub close_timeout: Duration,
    pub recent_closed: usize,
    pub target_policy: TargetPolicy,
}

/// Runtime dependencies and the single atomic relay registry.
pub struct RelayRuntime<C> {
    connector: C,
    relay_ids: Arc<dyn RelayIdSource>,
    session_ids: Arc<dyn SessionIdSource>,
    relay_budget: Arc<ConnectionBudget>,
    waiting_budget: Arc<ConnectionBudget>,
    config: RelayRuntimeConfig,
    state: Mutex<RelayState>,
    lifecycle: watch::Sender<RelayLifecycle>,
}

struct RelayState {
    lifecycle: RelayLifecycle,
    matching: WaitingTable,
    waiting: HashMap<RelayId, matching::WaitingConnection>,
    active: HashMap<SessionId, ActiveSessionSnapshot>,
    controls: HashMap<SessionId, Arc<Mutex<SessionControl>>>,
    recent: RecentClosedList,
    recent_refs: HashMap<SessionId, RelayIdRef>,
    next_session: Option<u128>,
    connecting: usize,
    handshaking: usize,
}

impl RelayState {
    fn counted_active(&self) -> usize {
        self.active.len()
    }
}

/// One consistent management-facing view; transport addresses are not invented.
#[derive(Debug, Clone)]
pub struct RelaySnapshot {
    pub lifecycle: RelayLifecycle,
    pub observed_at: Duration,
    pub handshaking: usize,
    pub target_connecting: usize,
    pub max_entries: usize,
    pub waiting: Vec<WaitingSnapshot>,
    pub active: Vec<ActiveSessionSnapshot>,
    pub recent_closed: Vec<ClosedSessionSnapshot>,
}

#[derive(Debug, Clone)]
pub struct WaitingSnapshot {
    pub relay_id: RelayId,
    pub relay_id_ref: RelayIdRef,
    pub context: ConnectionContext,
    pub registered_at: Duration,
    pub deadline: Duration,
    pub waited: Duration,
}

/// The remote peer endpoint for an active session, preserving target address invariants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivePeer {
    Server(ConnectionContext),
    Target(SocketAddr),
}

impl ActivePeer {
    pub fn target_address(&self) -> SocketAddr {
        match self {
            Self::Server(context) => context.peer_addr,
            Self::Target(addr) => *addr,
        }
    }

    pub fn server(&self) -> Option<&ConnectionContext> {
        match self {
            Self::Server(context) => Some(context),
            Self::Target(_) => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActiveSessionSnapshot {
    pub session_id: SessionId,
    pub mode: Mode,
    pub viewer_transport: ViewerTransport,
    pub started_at: SystemTime,
    started_mono: Duration,
    pub duration: Duration,
    pub state: SessionState,
    pub viewer: ConnectionContext,
    pub peer: ActivePeer,
    pub relay_id_ref: Option<RelayIdRef>,
    pub bytes_viewer_to_target: u64,
    pub bytes_target_to_viewer: u64,
}

#[derive(Debug, Clone)]
pub struct ClosedSessionSnapshot {
    pub session_id: SessionId,
    pub mode: Mode,
    pub viewer_transport: ViewerTransport,
    pub started_at: SystemTime,
    pub ended_at: SystemTime,
    pub duration: Duration,
    pub bytes_viewer_to_target: u64,
    pub bytes_target_to_viewer: u64,
    pub close_cause: CloseCause,
    pub relay_id_ref: Option<RelayIdRef>,
    pub side: Option<vnc_relay_domain::session::Side>,
}

struct AdminClose {
    account: AccountName,
    peer: PeerIp,
}

/// The owner holds this lock only during a synchronous engine poll.
/// HTTP clones its handle under the registry lock, then releases that lock.
struct SessionControl {
    command: watch::Sender<Option<Duration>>,
    completion: watch::Sender<Option<Result<Arc<ClosedSessionSnapshot>, ()>>>,
    admin: Option<AdminClose>,
    closing: bool,
    completed: bool,
}

impl From<&ClosedSession> for ClosedSessionSnapshot {
    fn from(closed: &ClosedSession) -> Self {
        Self {
            session_id: closed.id().clone(),
            mode: closed.mode(),
            viewer_transport: closed.viewer_transport(),
            started_at: closed.started_at(),
            ended_at: closed.ended_at(),
            duration: closed.duration(),
            bytes_viewer_to_target: closed.bytes(Direction::ViewerToTarget),
            bytes_target_to_viewer: closed.bytes(Direction::TargetToViewer),
            close_cause: closed.close_cause(),
            relay_id_ref: None,
            side: closed.close_side(),
        }
    }
}

impl<C> RelayRuntime<C> {
    /// The L argument must be the same budget used to admit prepared connections.
    pub fn new(
        connector: C,
        relay_ids: Arc<dyn RelayIdSource>,
        session_ids: Arc<dyn SessionIdSource>,
        relay_budget: Arc<ConnectionBudget>,
        waiting_budget: Arc<ConnectionBudget>,
        config: RelayRuntimeConfig,
    ) -> Self {
        let (lifecycle, _) = watch::channel(RelayLifecycle::Running);
        let state = RelayState {
            lifecycle: RelayLifecycle::Running,
            matching: WaitingTable::new(waiting_budget.limit() as usize, config.wait_timeout),
            waiting: HashMap::new(),
            active: HashMap::new(),
            controls: HashMap::new(),
            recent: RecentClosedList::new(config.recent_closed),
            recent_refs: HashMap::new(),
            next_session: Some(0),
            connecting: 0,
            handshaking: 0,
        };
        Self {
            connector,
            relay_ids,
            session_ids,
            relay_budget,
            waiting_budget,
            config,
            state: Mutex::new(state),
            lifecycle,
        }
    }

    /// Initial exchange aborts on the first non-Running phase.
    pub fn lifecycle(&self) -> watch::Receiver<RelayLifecycle> {
        self.lifecycle.subscribe()
    }

    fn lock(&self) -> MutexGuard<'_, RelayState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Takes one registry lock and reads the supplied shared clock once.
    pub fn snapshot(&self, clock: &dyn Clock) -> RelaySnapshot {
        let state = self.lock();
        let now = clock.monotonic();
        let mut waiting: Vec<_> = state
            .waiting
            .iter()
            .map(|(id, entry)| WaitingSnapshot {
                relay_id: id.clone(),
                relay_id_ref: entry.reference.clone(),
                context: entry.connection.context,
                registered_at: entry.metadata.registered_at,
                deadline: entry.metadata.deadline,
                waited: now.saturating_sub(entry.metadata.registered_at),
            })
            .collect();
        waiting.sort_by_key(|entry| entry.context.accepted_wall);
        let mut active: Vec<_> = state
            .active
            .values()
            .map(|entry| {
                let mut entry = entry.clone();
                entry.duration = now.saturating_sub(entry.started_mono);
                entry
            })
            .collect();
        active.sort_by_key(|entry| entry.started_at);
        RelaySnapshot {
            lifecycle: state.lifecycle,
            observed_at: now,
            handshaking: state.handshaking,
            target_connecting: state.connecting,
            max_entries: self.config.recent_closed,
            waiting,
            active,
            recent_closed: state
                .recent
                .iter()
                .map(|closed| {
                    let mut snapshot = ClosedSessionSnapshot::from(closed);
                    snapshot.relay_id_ref = state.recent_refs.get(closed.id()).cloned();
                    snapshot
                })
                .collect(),
        }
    }

    /// Increments the handshaking counter and returns an RAII guard that
    /// decrements it on every exit path.
    pub fn track_handshaking<'a>(&'a self) -> HandshakingGuard<'a, C> {
        self.lock().handshaking += 1;
        HandshakingGuard(self)
    }

    fn force_if_due(&self, clock: &dyn Clock, events: &dyn EventSink) {
        let mut state = self.lock();
        if matches!(state.lifecycle, RelayLifecycle::Grace { deadline } if clock.monotonic() >= deadline)
        {
            self.force_locked(&mut state, events);
        }
    }

    fn force_locked(&self, state: &mut RelayState, events: &dyn EventSink) {
        if state.lifecycle == RelayLifecycle::Force {
            return;
        }
        state.lifecycle = RelayLifecycle::Force;
        let remaining = state.counted_active();
        if remaining > 0 {
            events.record(&Event::ShutdownForced {
                remaining_sessions: remaining,
            });
        }
        self.lifecycle.send_replace(RelayLifecycle::Force);
    }
}

impl<C: Send + Sync> RelaySessionControl for RelayRuntime<C> {
    fn request_terminate(
        &self,
        session_id: &str,
        account: AccountName,
        peer: PeerIp,
        clock: &dyn Clock,
        events: &dyn EventSink,
    ) -> Option<crate::management::terminate::TerminationTicket> {
        let id = SessionId::parse(session_id).ok()?;
        let control = self.lock().controls.get(&id).cloned()?;
        let mut control = control
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if control.completed {
            return None;
        }
        let mut state = self.lock();
        let accepted_at = clock.monotonic();
        if matches!(state.lifecycle, RelayLifecycle::Grace { deadline } if accepted_at >= deadline)
        {
            self.force_locked(&mut state, events);
        }
        let initiated = !control.closing && state.lifecycle != RelayLifecycle::Force;
        if initiated {
            control.closing = true;
            control.admin = Some(AdminClose { account, peer });
            if let Some(entry) = state.active.get_mut(&id) {
                entry.state = SessionState::Closing;
            }
            control.command.send_replace(Some(accepted_at));
        }
        Some(crate::management::terminate::TerminationTicket {
            initiated,
            completion: control.completion.subscribe(),
        })
    }
}

/// Terminal dispatch result; corresponding external failures are already logged.
#[derive(Debug)]
pub enum RelayDispatchError {
    Rejected(RejectReason),
    Aborted(AbortReason),
    Target(crate::events::TargetFailureReason),
    Matching(vnc_relay_domain::matching::MatchingError),
    Session(SessionError),
    SessionIdsExhausted,
    WaitingExpired,
}

impl fmt::Display for RelayDispatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rejected(reason) => f.write_str(reason.as_str()),
            Self::Aborted(reason) => f.write_str(reason.as_str()),
            Self::Target(reason) => f.write_str(reason.as_str()),
            Self::Matching(error) => error.fmt(f),
            Self::Session(error) => error.fmt(f),
            Self::SessionIdsExhausted => f.write_str("session identifier counter exhausted"),
            Self::WaitingExpired => f.write_str("waiting_expired"),
        }
    }
}

impl std::error::Error for RelayDispatchError {}

struct OwnedConnection {
    stream: BoxRelayStream,
    trailing: Vec<u8>,
    permit: BudgetPermit,
    context: ConnectionContext,
}

impl OwnedConnection {
    fn from_prepared<S: RelayStream + 'static>(
        prepared: PreparedConnection<S>,
    ) -> (Self, HandshakeRequest) {
        let (stream, context, request, trailing, permit) = prepared.into_parts();
        (
            Self {
                stream: Box::new(stream),
                trailing,
                permit,
                context,
            },
            request,
        )
    }

    fn into_endpoint(self) -> (RelayEndpoint, ConnectionContext) {
        let Self {
            stream,
            trailing,
            permit,
            context,
        } = self;
        (
            RelayEndpoint {
                stream,
                trailing,
                permit,
            },
            context,
        )
    }
}

struct RelayEndpoint {
    stream: BoxRelayStream,
    trailing: Vec<u8>,
    permit: BudgetPermit,
}

struct SessionPair {
    viewer: OwnedConnection,
    other: RelayEndpoint,
    peer: ActivePeer,
    target_host: Option<String>,
    relay: Option<(RelayId, RelayIdRef, MatchClaim)>,
}

struct ActiveGuard<'a, C> {
    runtime: &'a RelayRuntime<C>,
    session_id: SessionId,
    relay_id: Option<RelayId>,
    control: Arc<Mutex<SessionControl>>,
    clock: &'a dyn Clock,
    events: &'a dyn EventSink,
    model: Session,
    error_kind: Mutex<Option<crate::events::ConnectionErrorKind>>,
}

impl<C> Drop for ActiveGuard<'_, C> {
    fn drop(&mut self) {
        let mut control = self
            .control
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if control.completed {
            return;
        }
        let mut state = self.runtime.lock();
        if let Some(id) = &self.relay_id {
            state.matching.begin_closing(id, &self.session_id);
            state.matching.release(id, &self.session_id);
        }
        if let Some(entry) = state.active.remove(&self.session_id) {
            if let Some(accepted_at) = *control.command.borrow() {
                let _ = self.model.begin_admin_close(accepted_at);
            }
            if let Ok(closed) = self.model.finish_after_owner_loss(
                state.lifecycle != RelayLifecycle::Running,
                self.clock.wall(),
                self.clock.monotonic(),
            ) {
                self.events.record(&Event::SessionClosed {
                    session_id: closed.id().clone(),
                    mode: closed.mode(),
                    viewer_transport: closed.viewer_transport(),
                    started_at: closed.started_at(),
                    duration_ms: milliseconds(closed.duration()),
                    bytes_viewer_to_target: closed.bytes(Direction::ViewerToTarget),
                    bytes_target_to_viewer: closed.bytes(Direction::TargetToViewer),
                    close_cause: closed.close_cause(),
                    side: closed.close_side(),
                    error_kind: (*self
                        .error_kind
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner))
                    .filter(|_| closed.close_cause() == CloseCause::ConnectionError),
                    relay_id_ref: entry.relay_id_ref.clone(),
                    admin_account: control
                        .admin
                        .as_ref()
                        .filter(|_| closed.close_cause() == CloseCause::AdminTerminated)
                        .map(|admin| admin.account.clone()),
                });
                if let Some(reference) = entry.relay_id_ref {
                    state.recent_refs.insert(closed.id().clone(), reference);
                }
                if let Some(evicted) = state.recent.push(closed) {
                    state.recent_refs.remove(evicted.id());
                }
            }
        }
        state.controls.remove(&self.session_id);
        if let Some(admin) = &control.admin {
            self.events.record(&Event::AdminTerminate {
                admin_account: admin.account.clone(),
                peer_ip: admin.peer,
                session_id: Some(self.session_id.clone()),
                result: AdminTerminateResult::Failed,
            });
        }
        control.completed = true;
        control.completion.send_replace(Some(Err(())));
    }
}

/// An RAII guard that decrements the runtime's handshaking count on drop.
pub struct HandshakingGuard<'a, C>(&'a RelayRuntime<C>);

impl<'a, C> Drop for HandshakingGuard<'a, C> {
    fn drop(&mut self) {
        let mut state = self.0.lock();
        state.handshaking = state.handshaking.saturating_sub(1);
    }
}

impl<C: Send + Sync> RelayStateSource for RelayRuntime<C> {
    fn snapshot(&self, clock: &dyn Clock) -> RelaySnapshot {
        self.snapshot(clock)
    }
}

impl RelayService {
    /// Executes a prepared connection until rejection, handoff, or session cleanup.
    pub async fn dispatch<S, C>(
        &self,
        prepared: PreparedConnection<S>,
        runtime: &RelayRuntime<C>,
    ) -> Result<(), RelayDispatchError>
    where
        S: RelayStream + 'static,
        C: TargetConnector,
    {
        let (connection, request) = OwnedConnection::from_prepared(prepared);
        match request {
            HandshakeRequest::Mode1(address) => {
                target::dispatch(self, runtime, connection, address).await
            }
            HandshakeRequest::Mode2(id) => matching::dispatch(self, runtime, connection, id).await,
        }
    }

    /// First request stops admission; another request forces existing sessions.
    pub fn request_shutdown<C>(&self, runtime: &RelayRuntime<C>, grace: Duration) {
        self.begin_shutdown(runtime, grace, ShutdownTrigger::Requested);
    }

    /// Starts fatal grace once. A failure during an existing shutdown is not
    /// a second shutdown request and must not shorten G (ops-info-v1 §1.12).
    pub fn request_fatal_shutdown<C>(
        &self,
        runtime: &RelayRuntime<C>,
        grace: Duration,
        detail: String,
    ) {
        self.begin_shutdown(runtime, grace, ShutdownTrigger::FatalError { detail });
    }

    fn begin_shutdown<C>(
        &self,
        runtime: &RelayRuntime<C>,
        grace: Duration,
        trigger: ShutdownTrigger,
    ) {
        let mut state = runtime.lock();
        match state.lifecycle {
            RelayLifecycle::Running => {
                let deadline = self.clock.monotonic().saturating_add(grace);
                state.lifecycle = RelayLifecycle::Grace { deadline };
                self.events.record(&Event::ShutdownStarted {
                    trigger,
                    grace_secs: grace.as_secs(),
                    active_sessions: state.counted_active(),
                });
                runtime.lifecycle.send_replace(state.lifecycle);
                if grace.is_zero() {
                    runtime.force_locked(&mut state, self.events.as_ref());
                }
            }
            RelayLifecycle::Grace { .. } => {
                if matches!(trigger, ShutdownTrigger::Requested) {
                    runtime.force_locked(&mut state, self.events.as_ref());
                }
            }
            RelayLifecycle::Force => {}
        }
    }

    /// Drives the G deadline while entry-point owners still need forced cleanup.
    /// The process supervisor cancels and joins this task once those owners end.
    pub async fn drive_shutdown<C>(&self, runtime: &RelayRuntime<C>) {
        let mut lifecycle = runtime.lifecycle();
        if lifecycle
            .wait_for(|phase| *phase != RelayLifecycle::Running)
            .await
            .is_err()
        {
            return;
        }
        loop {
            let phase = *lifecycle.borrow_and_update();
            let deadline = match phase {
                RelayLifecycle::Force => return,
                RelayLifecycle::Grace { deadline } => deadline,
                RelayLifecycle::Running => continue,
            };
            let mut changed = pin!(lifecycle.changed());
            let mut timer = pin!(tokio::time::sleep(
                deadline.saturating_sub(self.clock.monotonic())
            ));
            let expired = poll_fn(|cx| {
                if self.clock.monotonic() >= deadline || timer.as_mut().poll(cx).is_ready() {
                    Poll::Ready(true)
                } else if changed.as_mut().poll(cx).is_ready() {
                    Poll::Ready(false)
                } else {
                    Poll::Pending
                }
            })
            .await;
            if expired {
                let mut state = runtime.lock();
                runtime.force_locked(&mut state, self.events.as_ref());
                return;
            }
        }
    }

    fn aborted(&self, context: ConnectionContext, stage: Stage, mode: Mode, reason: AbortReason) {
        self.events.record(&Event::RelayAborted {
            conn_id: context.conn_id,
            entry_point: context.entry_point,
            stage,
            reason,
            peer_ip: context.peer_ip,
            elapsed_ms: milliseconds(self.clock.monotonic().saturating_sub(context.accepted_at)),
            mode,
        });
    }

    async fn reject_connection<C>(
        &self,
        mut connection: OwnedConnection,
        action: RelayCloseAction,
        runtime: &RelayRuntime<C>,
    ) {
        self.close_stream(
            connection.stream.as_mut(),
            action,
            runtime.config.close_timeout,
            runtime.lifecycle(),
        )
        .await;
        drop(connection);
    }

    async fn close_pair(
        &self,
        pair: &mut SessionPair,
        action: RelayCloseAction,
        timeout: Duration,
        lifecycle: watch::Receiver<RelayLifecycle>,
    ) {
        let mut completed = [false; 2];
        self.close_bounded(timeout, lifecycle, |cx| {
            if !completed[0] {
                completed[0] = Pin::new(pair.viewer.stream.as_mut())
                    .poll_close(cx, action)
                    .is_ready();
            }
            if !completed[1] {
                completed[1] = Pin::new(pair.other.stream.as_mut())
                    .poll_close(cx, action)
                    .is_ready();
            }
            if completed == [true; 2] {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await;
    }

    async fn run_pair<C: TargetConnector>(
        &self,
        runtime: &RelayRuntime<C>,
        mut pair: SessionPair,
    ) -> Result<(), RelayDispatchError> {
        let mode = if pair.relay.is_some() {
            Mode::Mode2
        } else {
            Mode::Mode1
        };
        let transport = pair.viewer.stream.transport();
        let (command, admin_close) = watch::channel(None);
        let (completion, _) = watch::channel(None);
        let control = Arc::new(Mutex::new(SessionControl {
            command,
            completion,
            admin: None,
            closing: false,
            completed: false,
        }));
        let activation = {
            let mut state = runtime.lock();
            if state.lifecycle != RelayLifecycle::Running {
                if let Some((id, _, claim)) = &pair.relay {
                    state.matching.cancel_claim(id, claim);
                }
                Err(RelayDispatchError::Aborted(AbortReason::ServerShutdown))
            } else if let Some(counter) = state.next_session {
                if let Some((id, _, claim)) = &pair.relay
                    && !matches!(state.matching.get(id),
                        Some(vnc_relay_domain::matching::IdOccupancy::Claimed(current))
                        if current.viewer_conn_id == claim.viewer_conn_id
                        && current.server_conn_id == claim.server_conn_id)
                {
                    return Err(RelayDispatchError::Matching(
                        vnc_relay_domain::matching::MatchingError::InvalidClaim,
                    ));
                }
                let id = runtime.session_ids.session_id(counter);
                if let Some((relay_id, _, claim)) = pair
                    .relay
                    .as_ref()
                    .filter(|(_, _, claim)| self.clock.monotonic() >= claim.waiter.deadline)
                {
                    state.matching.cancel_claim(relay_id, claim);
                    Err(RelayDispatchError::WaitingExpired)
                } else {
                    if let Some((relay_id, _, claim)) = &pair.relay {
                        state
                            .matching
                            .activate(relay_id, claim, id.clone())
                            .map_err(RelayDispatchError::Matching)?;
                    }
                    state.next_session = counter.checked_add(1);
                    let started_mono = self.clock.monotonic();
                    let session =
                        Session::new(id.clone(), mode, transport, self.clock.wall(), started_mono);
                    state.active.insert(
                        id.clone(),
                        ActiveSessionSnapshot {
                            session_id: id,
                            mode,
                            viewer_transport: transport,
                            started_at: session.started_at(),
                            started_mono,
                            duration: Duration::ZERO,
                            state: SessionState::Active,
                            viewer: pair.viewer.context,
                            peer: pair.peer.clone(),
                            relay_id_ref: pair
                                .relay
                                .as_ref()
                                .map(|(_, reference, _)| reference.clone()),
                            bytes_viewer_to_target: 0,
                            bytes_target_to_viewer: 0,
                        },
                    );
                    state
                        .controls
                        .insert(session.id().clone(), Arc::clone(&control));
                    self.events.record(&Event::SessionStarted {
                        session_id: session.id().clone(),
                        mode,
                        viewer_transport: transport,
                        viewer_conn_id: pair.viewer.context.conn_id,
                        viewer_peer_ip: pair.viewer.context.peer_ip,
                        server_conn_id: match &pair.peer {
                            ActivePeer::Server(context) => Some(context.conn_id),
                            ActivePeer::Target(_) => None,
                        },
                        server_peer_ip: match &pair.peer {
                            ActivePeer::Server(context) => Some(context.peer_ip),
                            ActivePeer::Target(_) => None,
                        },
                        relay_id_ref: pair
                            .relay
                            .as_ref()
                            .map(|(_, reference, _)| reference.clone()),
                        waited_ms: pair
                            .relay
                            .as_ref()
                            .map(|(_, _, claim)| milliseconds(claim.waited())),
                        target_ip: match &pair.peer {
                            ActivePeer::Target(addr) => Some(addr.ip()),
                            ActivePeer::Server(_) => None,
                        },
                        target_port: match &pair.peer {
                            ActivePeer::Target(addr) => Some(addr.port()),
                            ActivePeer::Server(_) => None,
                        },
                        target_host: pair.target_host.take(),
                    });
                    Ok(session)
                }
            } else {
                if let Some((id, _, claim)) = &pair.relay {
                    state.matching.cancel_claim(id, claim);
                }
                Err(RelayDispatchError::SessionIdsExhausted)
            }
        };
        let model = match activation {
            Ok(model) => model,
            Err(error) => {
                let viewer = pair.viewer.context;
                let server = match &pair.peer {
                    ActivePeer::Server(context) => Some(*context),
                    ActivePeer::Target(_) => None,
                };
                let predecessor = pair
                    .relay
                    .as_ref()
                    .map(|(_, reference, claim)| (reference.clone(), claim.waiter));
                let expired = if matches!(error, RelayDispatchError::WaitingExpired) {
                    pair.relay
                        .as_ref()
                        .map(|(_, reference, claim)| (reference.clone(), claim.waiter))
                } else {
                    None
                };
                if expired.is_some() {
                    self.close_pair(
                        &mut pair,
                        RelayCloseAction::WaitingExpired,
                        runtime.config.close_timeout,
                        runtime.lifecycle(),
                    )
                    .await;
                }
                if matches!(error, RelayDispatchError::Aborted(_)) {
                    self.close_pair(
                        &mut pair,
                        RelayCloseAction::ServerShutdown,
                        runtime.config.close_timeout,
                        runtime.lifecycle(),
                    )
                    .await;
                }
                drop(pair);
                if let Some((reference, waiter)) = expired {
                    self.events.record(&Event::WaitingExpired {
                        conn_id: waiter.conn_id,
                        role: waiter.role,
                        relay_id_ref: reference,
                        waited_ms: milliseconds(
                            self.clock.monotonic().saturating_sub(waiter.registered_at),
                        ),
                    });
                }
                if matches!(error, RelayDispatchError::Aborted(_)) {
                    if let Some((reference, waiter)) = predecessor {
                        self.events.record(&Event::WaitingRemoved {
                            conn_id: waiter.conn_id,
                            role: waiter.role,
                            relay_id_ref: reference,
                            reason: crate::events::WaitingRemovalReason::ServerShutdown,
                            waited_ms: milliseconds(
                                self.clock.monotonic().saturating_sub(waiter.registered_at),
                            ),
                        });
                        let incoming = if viewer.conn_id == waiter.conn_id {
                            server
                        } else {
                            Some(viewer)
                        };
                        if let Some(context) = incoming {
                            self.aborted(
                                context,
                                Stage::Register,
                                mode,
                                AbortReason::ServerShutdown,
                            );
                        }
                    } else {
                        self.aborted(
                            viewer,
                            Stage::TargetConnect,
                            mode,
                            AbortReason::ServerShutdown,
                        );
                    }
                }
                return Err(error);
            }
        };
        let mut active_guard = ActiveGuard {
            runtime,
            session_id: model.id().clone(),
            relay_id: pair.relay.as_ref().map(|(id, _, _)| id.clone()),
            control: Arc::clone(&control),
            clock: self.clock.as_ref(),
            events: self.events.as_ref(),
            model,
            error_kind: Mutex::new(None),
        };
        // Declare stream ownership after its registry guard: cancellation drops
        // streams and permits before releasing active ID occupancy.
        let mut pair = pair;
        let relay_id = pair.relay.as_ref().map(|(id, _, _)| id);
        let observed = Mutex::new(SessionState::Active);
        let result = {
            let engine = session::run_session_controlled(
                pair.viewer.stream.as_mut(),
                pair.other.stream.as_mut(),
                std::mem::take(&mut pair.viewer.trailing),
                std::mem::take(&mut pair.other.trailing),
                &mut active_guard.model,
                self.clock.as_ref(),
                runtime.lifecycle(),
                self.buffer_bytes,
                runtime.config.close_timeout,
                admin_close,
                |session, error_kind| {
                    *observed
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = session.state();
                    *active_guard
                        .error_kind
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = error_kind;
                    let mut state = runtime.lock();
                    if let Some(snapshot) = state.active.get_mut(session.id()) {
                        snapshot.state = session.state();
                        snapshot.bytes_viewer_to_target = session.bytes(Direction::ViewerToTarget);
                        snapshot.bytes_target_to_viewer = session.bytes(Direction::TargetToViewer);
                    }
                    if session.state() == SessionState::Closing
                        && let Some(id) = relay_id
                    {
                        state.matching.begin_closing(id, session.id());
                    }
                },
            );
            let mut engine = pin!(engine);
            poll_fn(|cx| {
                runtime.force_if_due(self.clock.as_ref(), self.events.as_ref());
                // Acceptance and natural transitions share this synchronous poll lock.
                // No guard survives Pending, and registry callers release their lock first.
                let mut control = control
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                let result = engine.as_mut().poll(cx);
                control.closing |= *observed
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    != SessionState::Active;
                result
            })
            .await
        };
        let error_kind = result.map_err(RelayDispatchError::Session)?;
        let relay = pair.relay.take();
        // Both sockets must be gone before either L place is released.
        let OwnedConnection {
            stream: viewer_stream,
            permit: viewer_permit,
            ..
        } = pair.viewer;
        let RelayEndpoint {
            stream: other_stream,
            permit: other_permit,
            ..
        } = pair.other;
        drop(viewer_stream);
        drop(other_stream);
        drop(viewer_permit);
        drop(other_permit);
        let closed = active_guard
            .model
            .finish(self.clock.wall(), self.clock.monotonic())
            .map_err(RelayDispatchError::Session)?;
        let mut control = control
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut state = runtime.lock();
        if let Some((id, _, _)) = &relay {
            state.matching.begin_closing(id, closed.id());
            state.matching.release(id, closed.id());
        }
        state.active.remove(closed.id());
        state.controls.remove(closed.id());
        let reference = relay.map(|(_, reference, _)| reference);
        self.events.record(&Event::SessionClosed {
            session_id: closed.id().clone(),
            mode,
            viewer_transport: transport,
            started_at: closed.started_at(),
            duration_ms: milliseconds(closed.duration()),
            bytes_viewer_to_target: closed.bytes(Direction::ViewerToTarget),
            bytes_target_to_viewer: closed.bytes(Direction::TargetToViewer),
            close_cause: closed.close_cause(),
            side: closed.close_side(),
            error_kind,
            relay_id_ref: reference.clone(),
            admin_account: control
                .admin
                .as_ref()
                .filter(|_| closed.close_cause() == CloseCause::AdminTerminated)
                .map(|admin| admin.account.clone()),
        });
        let mut snapshot = ClosedSessionSnapshot::from(&closed);
        snapshot.relay_id_ref = reference.clone();
        if let Some(reference) = reference {
            state.recent_refs.insert(closed.id().clone(), reference);
        }
        if let Some(evicted) = state.recent.push(closed) {
            state.recent_refs.remove(evicted.id());
        }
        if let Some(admin) = &control.admin {
            self.events.record(&Event::AdminTerminate {
                admin_account: admin.account.clone(),
                peer_ip: admin.peer,
                session_id: Some(snapshot.session_id.clone()),
                result: AdminTerminateResult::Terminated,
            });
        }
        control.completed = true;
        control
            .completion
            .send_replace(Some(Ok(Arc::new(snapshot))));
        Ok(())
    }
}

fn milliseconds(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::io;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::task::Context;

    use tokio::io::{
        AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, DuplexStream, ReadBuf, duplex,
    };
    use tokio::time::Instant;
    use vnc_relay_domain::entry_point::Role;
    use vnc_relay_domain::handshake::HANDSHAKE_REQUEST_LEN;

    use super::*;
    use crate::ports::{RelayStreamError, TargetError};

    struct TestClock {
        epoch: Instant,
        offset: AtomicU64,
    }

    impl Clock for TestClock {
        fn monotonic(&self) -> Duration {
            self.epoch.elapsed() + Duration::from_millis(self.offset.load(Ordering::Relaxed))
        }
        fn wall(&self) -> SystemTime {
            SystemTime::UNIX_EPOCH + self.monotonic()
        }
    }

    type OnSessionClosed = Box<dyn Fn(&Event) + Send + Sync>;

    #[derive(Default)]
    struct Recorder(Mutex<Vec<Event>>, Mutex<Option<OnSessionClosed>>);
    impl EventSink for Recorder {
        fn record(&self, event: &Event) {
            if matches!(event, Event::SessionClosed { .. })
                && let Some(observe) = self.1.lock().expect("closed observer").as_ref()
            {
                observe(event);
            }
            self.0.lock().expect("events").push(event.clone());
        }
    }

    struct TestStream {
        stream: DuplexStream,
        actions: Arc<Mutex<Vec<RelayCloseAction>>>,
        failure: Option<crate::events::ConnectionErrorKind>,
        block_close: bool,
        on_drop: Option<Arc<dyn Fn() + Send + Sync>>,
        eof_signal: Option<Arc<std::sync::atomic::AtomicBool>>,
        panic_after_eof: Option<Arc<std::sync::atomic::AtomicBool>>,
    }

    impl std::fmt::Debug for TestStream {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("TestStream")
                .field("actions", &self.actions)
                .field("failure", &self.failure)
                .field("block_close", &self.block_close)
                .finish_non_exhaustive()
        }
    }

    impl Drop for TestStream {
        fn drop(&mut self) {
            if let Some(on_drop) = &self.on_drop {
                on_drop();
            }
        }
    }

    impl AsyncRead for TestStream {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buffer: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            assert!(
                !self
                    .panic_after_eof
                    .as_ref()
                    .is_some_and(|flag| flag.load(Ordering::SeqCst)),
                "controlled panic after peer EOF in the same engine poll"
            );
            if let Some(kind) = self.failure.take() {
                Poll::Ready(Err(RelayStreamError::new(kind).into_io_error()))
            } else {
                let before = buffer.filled().len();
                let result = Pin::new(&mut self.stream).poll_read(cx, buffer);
                if matches!(result, Poll::Ready(Ok(())))
                    && buffer.filled().len() == before
                    && let Some(flag) = &self.eof_signal
                {
                    flag.store(true, Ordering::SeqCst);
                }
                result
            }
        }
    }
    impl AsyncWrite for TestStream {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            bytes: &[u8],
        ) -> Poll<io::Result<usize>> {
            Pin::new(&mut self.stream).poll_write(cx, bytes)
        }
        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Pin::new(&mut self.stream).poll_flush(cx)
        }
        fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Pin::new(&mut self.stream).poll_shutdown(cx)
        }
    }
    impl RelayStream for TestStream {
        fn transport(&self) -> ViewerTransport {
            ViewerTransport::Tcp
        }
        fn pending_write_len(&self) -> usize {
            0
        }
        fn poll_close(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            action: RelayCloseAction,
        ) -> Poll<io::Result<()>> {
            self.actions.lock().expect("actions").push(action);
            if self.block_close {
                return Poll::Pending;
            }
            Pin::new(&mut self.stream).poll_shutdown(cx)
        }
    }
    struct Sources;
    impl RelayIdSource for Sources {
        fn reference(&self, _: &RelayId) -> RelayIdRef {
            // Deliberately collide references: matching must use original IDs.
            RelayIdRef::from_hash_prefix([7; 8])
        }
    }
    impl SessionIdSource for Sources {
        fn session_id(&self, counter: u128) -> SessionId {
            SessionId::from_block(counter.to_be_bytes())
        }
    }

    struct Attempt {
        delay: Duration,
        result: Result<TestStream, TargetError>,
    }
    struct Connector {
        resolved: Mutex<Result<Vec<std::net::IpAddr>, TargetError>>,
        resolve_delay: Mutex<Duration>,
        attempts: Mutex<VecDeque<Attempt>>,
        calls: Mutex<Vec<SocketAddr>>,
        names: Mutex<Vec<String>>,
    }
    impl TargetConnector for Connector {
        type Stream = TestStream;
        async fn resolve(&self, name: &str) -> Result<Vec<std::net::IpAddr>, TargetError> {
            self.names.lock().expect("names").push(name.to_owned());
            let delay = *self.resolve_delay.lock().expect("delay");
            tokio::time::sleep(delay).await;
            self.resolved.lock().expect("resolved").clone()
        }
        async fn connect(&self, address: SocketAddr) -> Result<TestStream, TargetError> {
            self.calls.lock().expect("calls").push(address);
            let attempt = self.attempts.lock().expect("attempts").pop_front();
            match attempt {
                Some(attempt) => {
                    tokio::time::sleep(attempt.delay).await;
                    attempt.result
                }
                None => std::future::pending().await,
            }
        }
    }

    struct Harness {
        service: RelayService,
        runtime: RelayRuntime<Connector>,
        clock: Arc<TestClock>,
        events: Arc<Recorder>,
        budget: Arc<ConnectionBudget>,
        waiting: Arc<ConnectionBudget>,
    }

    impl Harness {
        fn new(l: u32, lw: u32, b: usize, policy: &[&str]) -> Arc<Self> {
            let clock = Arc::new(TestClock {
                epoch: Instant::now(),
                offset: AtomicU64::new(0),
            });
            let events = Arc::new(Recorder::default());
            let budget = ConnectionBudget::new(l);
            let waiting = ConnectionBudget::new(lw);
            let connector = Connector {
                resolved: Mutex::new(Ok(Vec::new())),
                resolve_delay: Mutex::new(Duration::ZERO),
                attempts: Mutex::new(VecDeque::new()),
                calls: Mutex::new(Vec::new()),
                names: Mutex::new(Vec::new()),
            };
            Arc::new(Self {
                service: RelayService::new(
                    HandshakePolicy {
                        mode1_enabled: true,
                        mode2_enabled: true,
                    },
                    Duration::from_secs(10),
                    b,
                    Duration::from_secs(2),
                    clock.clone(),
                    events.clone(),
                ),
                runtime: RelayRuntime::new(
                    connector,
                    Arc::new(Sources),
                    Arc::new(Sources),
                    budget.clone(),
                    waiting.clone(),
                    RelayRuntimeConfig {
                        wait_timeout: Duration::from_secs(5),
                        connect_timeout: Duration::from_secs(4),
                        close_timeout: Duration::from_secs(2),
                        recent_closed: 2,
                        target_policy: TargetPolicy::new(
                            policy
                                .iter()
                                .map(|item| {
                                    vnc_relay_domain::target_policy::TargetItem::parse(item)
                                        .expect("policy")
                                })
                                .collect(),
                        ),
                    },
                ),
                clock,
                events,
                budget,
                waiting,
            })
        }
        fn new_with_recent(l: u32, lw: u32, b: usize, recent_closed: usize) -> Arc<Self> {
            let clock = Arc::new(TestClock {
                epoch: Instant::now(),
                offset: AtomicU64::new(0),
            });
            let events = Arc::new(Recorder::default());
            let budget = ConnectionBudget::new(l);
            let waiting = ConnectionBudget::new(lw);
            let connector = Connector {
                resolved: Mutex::new(Ok(Vec::new())),
                resolve_delay: Mutex::new(Duration::ZERO),
                attempts: Mutex::new(VecDeque::new()),
                calls: Mutex::new(Vec::new()),
                names: Mutex::new(Vec::new()),
            };
            Arc::new(Self {
                service: RelayService::new(
                    HandshakePolicy {
                        mode1_enabled: true,
                        mode2_enabled: true,
                    },
                    Duration::from_secs(10),
                    b,
                    Duration::from_secs(2),
                    clock.clone(),
                    events.clone(),
                ),
                runtime: RelayRuntime::new(
                    connector,
                    Arc::new(Sources),
                    Arc::new(Sources),
                    budget.clone(),
                    waiting.clone(),
                    RelayRuntimeConfig {
                        wait_timeout: Duration::from_secs(5),
                        connect_timeout: Duration::from_secs(4),
                        close_timeout: Duration::from_secs(2),
                        recent_closed,
                        target_policy: TargetPolicy::new(vec![]),
                    },
                ),
                clock,
                events,
                budget,
                waiting,
            })
        }

        async fn prepare_with_options(
            &self,
            role: Role,
            request: &str,
            trailing: &[u8],
            block_close: bool,
            on_drop: Option<Arc<dyn Fn() + Send + Sync>>,
        ) -> (
            PreparedConnection<TestStream>,
            DuplexStream,
            Arc<Mutex<Vec<RelayCloseAction>>>,
        ) {
            let (stream, mut peer) = duplex(4096);
            let actions = Arc::new(Mutex::new(Vec::new()));
            let stream = TestStream {
                stream,
                actions: actions.clone(),
                failure: None,
                block_close,
                on_drop,
                eof_signal: None,
                panic_after_eof: None,
            };
            let mut bytes = vec![0; HANDSHAKE_REQUEST_LEN];
            bytes[..request.len()].copy_from_slice(request.as_bytes());
            bytes.extend_from_slice(trailing);
            peer.write_all(&bytes).await.expect("request");
            let peer_addr = "192.0.2.2:12345"
                .parse::<SocketAddr>()
                .expect("fixture address");
            let context = ConnectionContext {
                conn_id: ConnId::next(),
                entry_point: if role == Role::Viewer {
                    EntryPoint::TcpViewer
                } else {
                    EntryPoint::TcpServer
                },
                peer_ip: PeerIp::new(peer_addr.ip()),
                peer_addr,
                accepted_wall: self.clock.wall(),
                accepted_at: self.clock.monotonic(),
                exchange_at: self.clock.monotonic(),
            };
            let prepared = self
                .service
                .handle_connection(
                    stream,
                    context,
                    self.budget.try_acquire().expect("L"),
                    self.runtime.lifecycle(),
                )
                .await
                .expect("prepared");
            if role == Role::Viewer {
                let mut greeting = [0; 12];
                peer.read_exact(&mut greeting).await.expect("greeting");
                assert_eq!(&greeting, vnc_relay_domain::handshake::VIEWER_GREETING);
            }
            (prepared, peer, actions)
        }

        async fn prepare(
            &self,
            role: Role,
            request: &str,
            trailing: &[u8],
        ) -> (
            PreparedConnection<TestStream>,
            DuplexStream,
            Arc<Mutex<Vec<RelayCloseAction>>>,
        ) {
            self.prepare_with_options(role, request, trailing, false, None)
                .await
        }

        fn start(
            self: &Arc<Self>,
            prepared: PreparedConnection<TestStream>,
        ) -> tokio::task::JoinHandle<Result<(), RelayDispatchError>> {
            let harness = self.clone();
            tokio::spawn(async move { harness.service.dispatch(prepared, &harness.runtime).await })
        }

        fn events(&self) -> Vec<Event> {
            self.events.0.lock().expect("events").clone()
        }
        fn snapshot(&self) -> RelaySnapshot {
            self.runtime.snapshot(self.clock.as_ref())
        }
        fn stop(&self) {
            self.service.request_shutdown(&self.runtime, Duration::ZERO);
        }
    }

    async fn settle() {
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
    }

    fn free_places(budget: &Arc<ConnectionBudget>) -> usize {
        let permits: Vec<_> = std::iter::from_fn(|| budget.try_acquire()).collect();
        permits.len()
    }

    /// spec: relay.md:RLY-010 partial
    /// spec: relay.md:RLY-011 partial
    /// spec: relay.md:RLY-012 partial
    /// spec: relay.md:RLY-003 partial
    /// spec: management.md:MGT-005 partial
    #[tokio::test(start_paused = true)]
    async fn matches_both_arrival_orders_and_finishes_once_with_exact_counters() {
        for first in [Role::Viewer, Role::Server] {
            let harness = Harness::new(2, 1, 16, &[]);
            let second = if first == Role::Viewer {
                Role::Server
            } else {
                Role::Viewer
            };
            let (prepared, mut first_peer, _) = harness.prepare(first, "ID:0123", b"first").await;
            let first_task = harness.start(prepared);
            settle().await;
            assert_eq!(harness.snapshot().waiting.len(), 1);
            assert_eq!(free_places(&harness.waiting), 0);
            let (prepared, mut second_peer, _) =
                harness.prepare(second, "ID:0123", b"second").await;
            let second_task = harness.start(prepared);
            settle().await;
            assert_eq!(harness.snapshot().active.len(), 1);
            assert!(harness.snapshot().waiting.is_empty());
            assert_eq!(free_places(&harness.waiting), 1);
            let mut received = [0; 6];
            first_peer
                .read_exact(&mut received)
                .await
                .expect("second prefix");
            assert_eq!(&received, b"second");
            let mut received = [0; 5];
            second_peer
                .read_exact(&mut received)
                .await
                .expect("first prefix");
            assert_eq!(&received, b"first");
            first_peer.shutdown().await.expect("first eof");
            second_peer.shutdown().await.expect("second eof");
            first_task.await.expect("task").expect("handoff");
            second_task.await.expect("task").expect("session");
            let snapshot = harness.snapshot();
            assert!(snapshot.active.is_empty());
            assert_eq!(snapshot.recent_closed.len(), 1);
            let closed = &snapshot.recent_closed[0];
            assert_eq!(
                closed.bytes_viewer_to_target,
                if first == Role::Viewer { 5 } else { 6 }
            );
            assert_eq!(
                closed.bytes_target_to_viewer,
                if first == Role::Viewer { 6 } else { 5 }
            );
            assert_eq!(closed.close_cause, CloseCause::PeerEof);
            assert_eq!(free_places(&harness.budget), 2);
            assert_eq!(
                harness
                    .events()
                    .iter()
                    .filter(|event| matches!(event, Event::SessionStarted { .. }))
                    .count(),
                1
            );
            assert_eq!(
                harness
                    .events()
                    .iter()
                    .filter(|event| matches!(event, Event::SessionClosed { .. }))
                    .count(),
                1
            );
        }
    }

    /// spec: relay.md:RLY-014 partial
    /// spec: relay.md:RLY-016 partial
    /// spec: operations.md:OPS-005 partial
    #[tokio::test(start_paused = true)]
    async fn waiting_cancellation_releases_resources_and_stale_cleanup_cannot_remove_replacement() {
        let harness = Harness::new(3, 1, 8, &[]);
        let (prepared, _peer, _) = harness.prepare(Role::Server, "ID:1", b"").await;
        let old_conn = prepared.context().conn_id;
        let task = harness.start(prepared);
        settle().await;
        task.abort();
        assert!(task.await.expect_err("cancelled").is_cancelled());
        assert!(harness.snapshot().waiting.is_empty());
        assert_eq!(free_places(&harness.waiting), 1);
        assert_eq!(free_places(&harness.budget), 3);
        let (prepared, _peer, _) = harness.prepare(Role::Server, "ID:1", b"").await;
        let task = harness.start(prepared);
        settle().await;
        let id = RelayId::parse("1").expect("id");
        assert!(
            harness
                .runtime
                .lock()
                .matching
                .remove_waiting(&id, old_conn)
                .is_none()
        );
        assert_eq!(harness.snapshot().waiting.len(), 1);
        harness.stop();
        task.await.expect("task").expect("shutdown waiting");
        assert_eq!(free_places(&harness.budget), 3);
    }

    /// spec: relay.md:RLY-014 partial
    /// spec: relay.md:RLY-025 partial
    #[tokio::test(start_paused = true)]
    async fn waiting_eof_and_exact_w_deadline_remove_only_that_connection() {
        for eof in [true, false] {
            let harness = Harness::new(2, 1, 8, &[]);
            let (prepared, mut peer, actions) = harness.prepare(Role::Server, "ID:7", b"").await;
            let task = harness.start(prepared);
            settle().await;
            if eof {
                peer.shutdown().await.expect("eof");
            } else {
                tokio::time::advance(Duration::from_secs(5)).await;
            }
            task.await.expect("task").expect("wait end");
            assert!(harness.snapshot().waiting.is_empty());
            assert_eq!(free_places(&harness.budget), 2);
            assert_eq!(free_places(&harness.waiting), 1);
            let events = harness.events();
            assert_eq!(
                events
                    .iter()
                    .filter(|event| matches!(
                        event,
                        Event::WaitingExpired { .. } | Event::WaitingRemoved { .. }
                    ))
                    .count(),
                1
            );
            assert!(actions.lock().expect("actions").contains(&if eof {
                RelayCloseAction::PeerEof
            } else {
                RelayCloseAction::WaitingExpired
            }));
        }
    }

    /// spec: relay.md:RLY-015 partial
    /// spec: relay.md:RLY-013 partial
    /// spec: operations.md:OPS-005 partial
    #[tokio::test(start_paused = true)]
    async fn rejects_duplicates_in_waiting_active_and_closing_without_taking_lw() {
        let harness = Harness::new(4, 1, 8, &[]);
        let (prepared, mut server, _) = harness.prepare(Role::Server, "ID:3", b"").await;
        let waiter = harness.start(prepared);
        settle().await;
        let (duplicate, _peer, actions) = harness.prepare(Role::Server, "ID:3", b"").await;
        assert!(matches!(
            harness.service.dispatch(duplicate, &harness.runtime).await,
            Err(RelayDispatchError::Rejected(RejectReason::DuplicateId))
        ));
        assert!(
            actions
                .lock()
                .expect("actions")
                .contains(&RelayCloseAction::RejectDuplicate)
        );
        let (prepared, _viewer, _) = harness.prepare(Role::Viewer, "ID:3", b"").await;
        let owner = harness.start(prepared);
        settle().await;
        for closing in [false, true] {
            if closing {
                server.shutdown().await.expect("eof");
                settle().await;
                assert_eq!(harness.snapshot().active[0].state, SessionState::Closing);
            }
            let (duplicate, _peer, actions) = harness.prepare(Role::Viewer, "ID:3", b"").await;
            assert!(matches!(
                harness.service.dispatch(duplicate, &harness.runtime).await,
                Err(RelayDispatchError::Rejected(RejectReason::DuplicateId))
            ));
            assert!(
                actions
                    .lock()
                    .expect("actions")
                    .contains(&RelayCloseAction::RejectDuplicate)
            );
            assert_eq!(harness.snapshot().active.len(), 1);
        }
        assert_eq!(free_places(&harness.waiting), 1);
        harness.stop();
        waiter.await.expect("waiter").expect("handoff");
        owner.await.expect("owner").expect("forced close");
    }

    /// spec: relay.md:RLY-013 partial
    /// spec: operations.md:OPS-005 partial
    #[tokio::test(start_paused = true)]
    async fn full_waiting_budget_still_allows_match_and_never_merges_ref_collisions() {
        let harness = Harness::new(5, 1, 8, &[]);
        let (prepared, _server, _) = harness.prepare(Role::Server, "ID:01", b"").await;
        let waiter = harness.start(prepared);
        settle().await;
        let (prepared, _other, actions) = harness.prepare(Role::Viewer, "ID:1", b"").await;
        assert!(matches!(
            harness.service.dispatch(prepared, &harness.runtime).await,
            Err(RelayDispatchError::Rejected(RejectReason::WaitingFull {
                limit: 1
            }))
        ));
        assert!(
            actions
                .lock()
                .expect("actions")
                .contains(&RelayCloseAction::WaitingFull)
        );
        assert_eq!(harness.snapshot().waiting[0].relay_id.as_str(), "01");
        let (prepared, _viewer, _) = harness.prepare(Role::Viewer, "ID:01", b"").await;
        let owner = harness.start(prepared);
        settle().await;
        assert_eq!(harness.snapshot().active.len(), 1);
        assert_eq!(free_places(&harness.waiting), 1);
        harness.stop();
        waiter.await.expect("waiter").expect("handoff");
        owner.await.expect("owner").expect("session");
    }

    /// spec: relay.md:RLY-018 partial
    /// spec: relay.md:RLY-014 partial
    #[tokio::test(start_paused = true)]
    async fn full_b_stops_reading_without_inventing_eof_then_drains_after_match() {
        let harness = Harness::new(2, 1, 4, &[]);
        let (prepared, mut server, _) = harness.prepare(Role::Server, "ID:5", b"1234").await;
        let waiter = harness.start(prepared);
        settle().await;
        server.write_all(b"56").await.expect("excess");
        server.shutdown().await.expect("fin behind B");
        settle().await;
        assert_eq!(harness.snapshot().waiting.len(), 1);
        let (prepared, mut viewer, _) = harness.prepare(Role::Viewer, "ID:5", b"").await;
        let owner = harness.start(prepared);
        let mut bytes = [0; 6];
        viewer
            .read_exact(&mut bytes)
            .await
            .expect("ordered payload");
        assert_eq!(&bytes, b"123456");
        viewer.shutdown().await.expect("eof");
        waiter.await.expect("waiter").expect("handoff");
        owner.await.expect("owner").expect("session");
        assert_eq!(
            harness.snapshot().recent_closed[0].bytes_target_to_viewer,
            6
        );
    }

    /// spec: operations.md:OPS-007 partial
    /// spec: operations.md:OPS-008 partial
    /// spec: operations.md:OPS-009 partial
    #[tokio::test(start_paused = true)]
    async fn grace_keeps_active_relay_until_deadline_and_emits_force_before_close() {
        for second_request in [false, true] {
            let harness = Harness::new(3, 1, 8, &[]);
            let (prepared, mut server, _) = harness.prepare(Role::Server, "ID:4", b"").await;
            let waiter = harness.start(prepared);
            settle().await;
            let (prepared, mut viewer, _) = harness.prepare(Role::Viewer, "ID:4", b"").await;
            let owner = harness.start(prepared);
            settle().await;
            harness
                .service
                .request_shutdown(&harness.runtime, Duration::from_secs(3));
            server.write_all(b"ok").await.expect("during G");
            let mut bytes = [0; 2];
            viewer.read_exact(&mut bytes).await.expect("G still relays");
            assert_eq!(&bytes, b"ok");
            assert!(matches!(
                *harness.runtime.lifecycle().borrow(),
                RelayLifecycle::Grace { .. }
            ));
            if second_request {
                harness
                    .service
                    .request_shutdown(&harness.runtime, Duration::from_secs(90));
            } else {
                tokio::time::advance(Duration::from_secs(3)).await;
            }
            waiter.await.expect("waiter").expect("handoff");
            owner.await.expect("owner").expect("force");
            assert_eq!(*harness.runtime.lifecycle().borrow(), RelayLifecycle::Force);
            let events = harness.events();
            let forced = events
                .iter()
                .position(|event| matches!(event, Event::ShutdownForced { .. }))
                .expect("forced");
            let closed = events
                .iter()
                .position(|event| matches!(event, Event::SessionClosed { .. }))
                .expect("closed");
            assert!(forced < closed);
            assert_eq!(
                events
                    .iter()
                    .filter(|event| matches!(event, Event::ShutdownForced { .. }))
                    .count(),
                1
            );
            assert_eq!(
                harness.snapshot().recent_closed[0].close_cause,
                CloseCause::ServerShutdown
            );
            assert_eq!(free_places(&harness.budget), 3);
        }
    }

    /// spec: operations.md:OPS-009 partial
    /// spec: operations.md:OPS-042 partial
    #[tokio::test(start_paused = true)]
    async fn forced_dispatch_records_retained_error_kind_only_after_streams_and_permits_are_released()
     {
        use crate::events::ConnectionErrorKind;
        use vnc_relay_domain::session::Side;

        for second_request in [false, true] {
            for kind in [
                ConnectionErrorKind::IoError,
                ConnectionErrorKind::WsProtocolError,
                ConnectionErrorKind::WsTextMessage,
                ConnectionErrorKind::WsMessageTooLarge,
            ] {
                let harness = Harness::new(3, 1, 8, &[]);
                let dropped = Arc::new(AtomicU64::new(0));
                let closed = Arc::new(AtomicU64::new(0));
                let on_drop: Arc<dyn Fn() + Send + Sync> = {
                    let dropped = Arc::clone(&dropped);
                    let budget = Arc::clone(&harness.budget);
                    Arc::new(move || {
                        assert_eq!(free_places(&budget), 1, "both permits outlive both streams");
                        dropped.fetch_add(1, Ordering::SeqCst);
                    })
                };
                *harness.events.1.lock().expect("closed observer") = Some({
                    let dropped = Arc::clone(&dropped);
                    let closed = Arc::clone(&closed);
                    let budget = Arc::clone(&harness.budget);
                    Box::new(move |event| {
                        assert_eq!(dropped.load(Ordering::SeqCst), 2);
                        assert_eq!(
                            free_places(&budget),
                            3,
                            "SessionClosed follows permit release"
                        );
                        assert!(matches!(
                            event,
                            Event::SessionClosed {
                                close_cause: CloseCause::ConnectionError,
                                side: Some(Side::Viewer),
                                error_kind: Some(actual),
                                ..
                            } if *actual == kind
                        ));
                        closed.fetch_add(1, Ordering::SeqCst);
                    })
                });
                let (prepared, _server, server_actions) = harness
                    .prepare_with_options(
                        Role::Server,
                        "ID:41",
                        b"",
                        true,
                        Some(Arc::clone(&on_drop)),
                    )
                    .await;
                let waiter = harness.start(prepared);
                settle().await;
                let (mut prepared, _viewer, viewer_actions) = harness
                    .prepare_with_options(Role::Viewer, "ID:41", b"", true, Some(on_drop))
                    .await;
                prepared.stream_mut().failure = Some(kind);
                let owner = harness.start(prepared);
                settle().await;
                assert_eq!(harness.snapshot().active.len(), 1);
                assert!(
                    viewer_actions
                        .lock()
                        .expect("actions")
                        .contains(&RelayCloseAction::StreamError(kind))
                );
                assert_eq!(dropped.load(Ordering::SeqCst), 0);
                assert_eq!(closed.load(Ordering::SeqCst), 0);
                assert_eq!(free_places(&harness.budget), 1);

                harness
                    .service
                    .request_shutdown(&harness.runtime, Duration::from_secs(1));
                if second_request {
                    harness
                        .service
                        .request_shutdown(&harness.runtime, Duration::ZERO);
                } else {
                    tokio::time::advance(Duration::from_secs(1)).await;
                }
                waiter.await.expect("waiter").expect("handoff");
                owner.await.expect("owner").expect("forced error cleanup");
                assert_eq!(closed.load(Ordering::SeqCst), 1);
                assert!(
                    server_actions
                        .lock()
                        .expect("actions")
                        .contains(&RelayCloseAction::StreamError(kind))
                );
                assert!(harness.snapshot().active.is_empty());
                assert_eq!(
                    harness
                        .events()
                        .iter()
                        .filter(|event| matches!(event, Event::SessionClosed { .. }))
                        .count(),
                    1
                );
            }
        }
    }

    /// spec: relay.md:RLY-006 partial
    /// spec: relay.md:RLY-007 partial
    /// spec: relay.md:RLY-008 partial
    /// spec: relay.md:RLY-009 partial
    #[tokio::test(start_paused = true)]
    async fn target_policy_dns_and_connection_failures_keep_distinct_reasons() {
        use crate::events::TargetFailureReason;
        for (policy, resolved, failure, expected) in [
            (vec![], Ok(vec![]), None, "target_not_allowed"),
            (
                vec!["host.example:5900"],
                Err(TargetError::ResolveFailed),
                None,
                "resolve_failed",
            ),
            (
                vec!["host.example:5900"],
                Ok(vec![]),
                None,
                "resolve_failed",
            ),
            (
                vec!["host.example:5900"],
                Ok(vec!["192.0.2.10".parse().expect("ip")]),
                None,
                "target_not_allowed",
            ),
            (
                vec!["host.example:5900", "192.0.2.10:5900"],
                Ok(vec!["192.0.2.10".parse().expect("ip")]),
                Some(TargetError::ConnectRefused),
                "connect_refused",
            ),
            (
                vec!["host.example:5900", "192.0.2.10:5900"],
                Ok(vec!["192.0.2.10".parse().expect("ip")]),
                Some(TargetError::ConnectError),
                "connect_error",
            ),
        ] {
            let harness = Harness::new(2, 1, 8, &policy);
            *harness.runtime.connector.resolved.lock().expect("resolve") = resolved;
            if let Some(error) = failure {
                harness
                    .runtime
                    .connector
                    .attempts
                    .lock()
                    .expect("attempts")
                    .push_back(Attempt {
                        delay: Duration::ZERO,
                        result: Err(error),
                    });
            }
            let (prepared, _peer, actions) = harness
                .prepare(Role::Viewer, "host.example:5900", b"")
                .await;
            let result = harness
                .service
                .dispatch(prepared, &harness.runtime)
                .await
                .expect_err("failure");
            assert_eq!(result.to_string(), expected);
            assert_eq!(free_places(&harness.budget), 2);
            assert!(harness.snapshot().active.is_empty());
            assert_eq!(harness.runtime.lock().next_session, Some(0));
            let events = harness.events();
            assert_eq!(events.len(), 1);
            if expected == "target_not_allowed" {
                assert!(matches!(
                    events[0],
                    Event::TargetRejected {
                        reason: RejectReason::TargetNotAllowed,
                        ..
                    }
                ));
                assert!(
                    actions
                        .lock()
                        .expect("actions")
                        .contains(&RelayCloseAction::TargetNotAllowed)
                );
            } else {
                let expected = match expected {
                    "resolve_failed" => TargetFailureReason::ResolveFailed,
                    "connect_refused" => TargetFailureReason::ConnectRefused,
                    _ => TargetFailureReason::ConnectError,
                };
                assert!(
                    matches!(events[0], Event::TargetConnectFailed { reason, .. } if reason == expected)
                );
                assert!(
                    actions
                        .lock()
                        .expect("actions")
                        .contains(&RelayCloseAction::TargetConnectFailed)
                );
            }
        }
    }

    /// spec: relay.md:RLY-006 partial
    /// spec: relay.md:RLY-009 partial
    /// spec: relay.md:RLY-003 partial
    /// spec: operations.md:OPS-023 partial
    #[tokio::test(start_paused = true)]
    async fn target_candidates_are_sequential_final_ip_checked_and_reuse_one_l_slot() {
        let harness = Harness::new(2, 1, 8, &["host.example:5900", "192.0.2.0/24:5900"]);
        *harness.runtime.connector.resolved.lock().expect("resolve") = Ok(vec![
            "203.0.113.1".parse().expect("blocked"),
            "192.0.2.10".parse().expect("first"),
            "::ffff:192.0.2.11".parse().expect("mapped"),
        ]);
        let (stream, mut target_peer) = duplex(4096);
        let target_actions = Arc::new(Mutex::new(Vec::new()));
        harness
            .runtime
            .connector
            .attempts
            .lock()
            .expect("attempts")
            .extend([
                Attempt {
                    delay: Duration::ZERO,
                    result: Err(TargetError::ConnectRefused),
                },
                Attempt {
                    delay: Duration::ZERO,
                    result: Ok(TestStream {
                        stream,
                        actions: target_actions,
                        failure: None,
                        block_close: false,
                        on_drop: None,
                        eof_signal: None,
                        panic_after_eof: None,
                    }),
                },
            ]);
        let (prepared, mut viewer, _) = harness
            .prepare(Role::Viewer, "host.example:5900", b"request")
            .await;
        viewer.shutdown().await.expect("viewer eof");
        target_peer.write_all(b"reply").await.expect("reply");
        target_peer.shutdown().await.expect("target eof");
        harness
            .service
            .dispatch(prepared, &harness.runtime)
            .await
            .expect("session");
        let mut request = [0; 7];
        target_peer.read_exact(&mut request).await.expect("request");
        assert_eq!(&request, b"request");
        let mut reply = [0; 5];
        viewer.read_exact(&mut reply).await.expect("reply");
        assert_eq!(&reply, b"reply");
        assert_eq!(
            *harness.runtime.connector.calls.lock().expect("calls"),
            [
                "192.0.2.10:5900".parse::<SocketAddr>().expect("first"),
                "192.0.2.11:5900".parse::<SocketAddr>().expect("second"),
            ]
        );
        assert_eq!(
            harness.snapshot().recent_closed[0].bytes_viewer_to_target,
            7
        );
        assert_eq!(
            harness.snapshot().recent_closed[0].bytes_target_to_viewer,
            5
        );
        assert_eq!(free_places(&harness.budget), 2);
    }

    /// spec: relay.md:RLY-009 partial
    /// spec: operations.md:OPS-023 partial
    #[tokio::test(start_paused = true)]
    async fn c_includes_dns_and_all_attempts_and_target_l_exhaustion_never_connects() {
        use crate::events::TargetFailureReason;
        let harness = Harness::new(2, 1, 8, &["host.example:5900", "192.0.2.10:5900"]);
        *harness.runtime.connector.resolved.lock().expect("resolve") =
            Ok(vec!["192.0.2.10".parse().expect("ip")]);
        *harness
            .runtime
            .connector
            .resolve_delay
            .lock()
            .expect("delay") = Duration::from_secs(3);
        harness
            .runtime
            .connector
            .attempts
            .lock()
            .expect("attempts")
            .push_back(Attempt {
                delay: Duration::from_secs(2),
                result: Err(TargetError::ConnectRefused),
            });
        let (prepared, _peer, _) = harness
            .prepare(Role::Viewer, "host.example:5900", b"")
            .await;
        assert!(matches!(
            harness.service.dispatch(prepared, &harness.runtime).await,
            Err(RelayDispatchError::Target(
                TargetFailureReason::ConnectTimeout
            ))
        ));
        assert!(matches!(
            harness.events()[0],
            Event::TargetConnectFailed {
                elapsed_ms: 4000,
                ..
            }
        ));
        assert_eq!(free_places(&harness.budget), 2);

        let harness = Harness::new(1, 1, 8, &["192.0.2.10:5900"]);
        let (prepared, _peer, _) = harness.prepare(Role::Viewer, "192.0.2.10:5900", b"").await;
        assert!(matches!(
            harness.service.dispatch(prepared, &harness.runtime).await,
            Err(RelayDispatchError::Rejected(
                RejectReason::RelayConnectionLimit { limit: 1 }
            ))
        ));
        assert!(
            harness
                .runtime
                .connector
                .calls
                .lock()
                .expect("calls")
                .is_empty()
        );
        assert!(
            harness
                .runtime
                .connector
                .names
                .lock()
                .expect("names")
                .is_empty()
        );
        assert_eq!(free_places(&harness.budget), 1);
    }

    /// spec: relay.md:RLY-014 partial
    /// spec: relay.md:RLY-016 partial
    #[tokio::test(start_paused = true)]
    async fn arrival_at_w_expires_old_conn_atomically_and_delayed_waiter_cannot_touch_new_one() {
        let harness = Harness::new(3, 1, 8, &[]);
        let (prepared, _old_peer, _) = harness.prepare(Role::Server, "ID:8", b"").await;
        let old_conn = prepared.context().conn_id;
        let mut old = Box::pin(harness.service.dispatch(prepared, &harness.runtime));
        poll_fn(|cx| {
            assert!(old.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        // Move the application clock to exactly W without polling the old owner.
        harness.clock.offset.fetch_add(5000, Ordering::Relaxed);
        let (prepared, _new_peer, _) = harness.prepare(Role::Viewer, "ID:8", b"").await;
        let new_conn = prepared.context().conn_id;
        let new_owner = harness.start(prepared);
        settle().await;
        assert_eq!(harness.snapshot().waiting[0].context.conn_id, new_conn);
        old.await.expect("old handoff end");
        assert_eq!(harness.snapshot().waiting[0].context.conn_id, new_conn);
        assert!(harness.events().iter().any(
            |event| matches!(event, Event::WaitingExpired { conn_id, .. } if *conn_id == old_conn)
        ));
        assert!(
            !harness
                .events()
                .iter()
                .any(|event| matches!(event, Event::SessionStarted { .. }))
        );
        harness.stop();
        new_owner.await.expect("new task").expect("shutdown");
    }

    /// spec: relay.md:RLY-020 partial
    /// spec: operations.md:OPS-007 partial
    #[tokio::test(start_paused = true)]
    async fn shutdown_aborts_connecting_and_waiting_without_issuing_session_ids() {
        let harness = Harness::new(4, 1, 8, &["192.0.2.10:5900"]);
        let (prepared, _server, _) = harness.prepare(Role::Server, "ID:9", b"").await;
        let waiter = harness.start(prepared);
        let (prepared, _viewer, _) = harness.prepare(Role::Viewer, "192.0.2.10:5900", b"").await;
        let connecting = harness.start(prepared);
        settle().await;
        assert_eq!(harness.snapshot().target_connecting, 1);
        harness
            .service
            .request_shutdown(&harness.runtime, Duration::from_secs(10));
        waiter.await.expect("waiter").expect("removed");
        assert!(matches!(
            connecting.await.expect("connecting"),
            Err(RelayDispatchError::Aborted(AbortReason::ServerShutdown))
        ));
        assert_eq!(harness.snapshot().target_connecting, 0);
        assert!(harness.snapshot().waiting.is_empty());
        assert!(harness.snapshot().active.is_empty());
        assert_eq!(harness.runtime.lock().next_session, Some(0));
        assert_eq!(free_places(&harness.budget), 4);
        assert!(!harness.events().iter().any(|event| matches!(
            event,
            Event::ShutdownForced { .. } | Event::SessionStarted { .. }
        )));
    }

    /// spec: relay.md:RLY-020 partial
    /// spec: relay.md:RLY-021 partial
    #[tokio::test(start_paused = true)]
    async fn typed_waiting_error_isolated_from_an_unrelated_waiter() {
        use crate::events::{ConnectionErrorKind, WaitingRemovalReason};
        let harness = Harness::new(3, 2, 8, &[]);
        let (prepared, _healthy_peer, _) = harness.prepare(Role::Server, "ID:10", b"").await;
        let healthy = harness.start(prepared);
        let (mut prepared, _bad_peer, actions) = harness.prepare(Role::Server, "ID:11", b"").await;
        prepared.stream_mut().failure = Some(ConnectionErrorKind::WsProtocolError);
        let failed = harness.start(prepared);
        settle().await;
        failed.await.expect("failed task").expect("removed");
        assert_eq!(harness.snapshot().waiting.len(), 1);
        assert_eq!(harness.snapshot().waiting[0].relay_id.as_str(), "10");
        assert!(
            actions
                .lock()
                .expect("actions")
                .contains(&RelayCloseAction::StreamError(
                    ConnectionErrorKind::WsProtocolError
                ))
        );
        assert!(harness.events().iter().any(|event| matches!(
            event,
            Event::WaitingRemoved {
                reason: WaitingRemovalReason::ConnectionError(ConnectionErrorKind::WsProtocolError),
                ..
            }
        )));
        harness.stop();
        healthy.await.expect("healthy").expect("removed");
    }

    /// spec: relay.md:RLY-016 partial
    /// spec: relay.md:RLY-020 partial
    #[tokio::test(start_paused = true)]
    async fn cancelling_active_owner_drops_streams_permits_and_exact_session_occupancy() {
        let harness = Harness::new(3, 1, 8, &[]);
        let (prepared, _server, _) = harness.prepare(Role::Server, "ID:12", b"").await;
        let waiter = harness.start(prepared);
        settle().await;
        let (prepared, _viewer, _) = harness.prepare(Role::Viewer, "ID:12", b"").await;
        let owner = harness.start(prepared);
        settle().await;
        assert_eq!(harness.snapshot().active.len(), 1);
        owner.abort();
        assert!(owner.await.expect_err("cancelled").is_cancelled());
        waiter.await.expect("waiter").expect("handoff");
        assert!(harness.snapshot().active.is_empty());
        assert!(
            harness
                .runtime
                .lock()
                .matching
                .get(&RelayId::parse("12").expect("id"))
                .is_none()
        );
        assert_eq!(free_places(&harness.budget), 3);
        assert_eq!(free_places(&harness.waiting), 1);
        assert_eq!(
            harness
                .events()
                .iter()
                .filter(|event| matches!(
                    event,
                    Event::SessionClosed {
                        close_cause: CloseCause::InternalError,
                        ..
                    }
                ))
                .count(),
            1,
        );
        let (prepared, _server, _) = harness.prepare(Role::Server, "ID:12", b"").await;
        let new_owner = harness.start(prepared);
        settle().await;
        assert_eq!(harness.snapshot().waiting.len(), 1);
        harness.stop();
        new_owner.await.expect("new owner").expect("shutdown");
    }

    /// spec: relay.md:RLY-013 partial
    /// spec: relay.md:RLY-016 partial
    #[tokio::test(start_paused = true)]
    async fn checked_session_counter_uses_last_value_once_and_exhaustion_releases_claim() {
        let harness = Harness::new(3, 1, 8, &[]);
        harness.runtime.lock().next_session = Some(u128::MAX);
        let (prepared, mut server, _) = harness.prepare(Role::Server, "ID:13", b"").await;
        let waiter = harness.start(prepared);
        settle().await;
        let (prepared, mut viewer, _) = harness.prepare(Role::Viewer, "ID:13", b"").await;
        let owner = harness.start(prepared);
        settle().await;
        assert_eq!(harness.runtime.lock().next_session, None);
        assert_eq!(
            harness.snapshot().active[0].session_id,
            SessionId::from_block(u128::MAX.to_be_bytes())
        );
        server.shutdown().await.expect("server eof");
        viewer.shutdown().await.expect("viewer eof");
        waiter.await.expect("waiter").expect("handoff");
        owner.await.expect("owner").expect("session");
        let (prepared, _server, _) = harness.prepare(Role::Server, "ID:13", b"").await;
        let waiter = harness.start(prepared);
        settle().await;
        let (prepared, _viewer, _) = harness.prepare(Role::Viewer, "ID:13", b"").await;
        assert!(matches!(
            harness.service.dispatch(prepared, &harness.runtime).await,
            Err(RelayDispatchError::SessionIdsExhausted)
        ));
        waiter.await.expect("waiter").expect("handoff");
        assert!(harness.snapshot().active.is_empty());
        assert!(
            harness
                .runtime
                .lock()
                .matching
                .get(&RelayId::parse("13").expect("id"))
                .is_none()
        );
        assert_eq!(free_places(&harness.budget), 3);
        assert_eq!(
            harness
                .events()
                .iter()
                .filter(|event| matches!(event, Event::SessionStarted { .. }))
                .count(),
            1
        );
    }

    #[tokio::test(start_paused = true)]
    async fn zero_grace_without_active_sessions_has_no_forced_event() {
        let harness = Harness::new(2, 1, 8, &[]);
        harness
            .service
            .request_shutdown(&harness.runtime, Duration::ZERO);
        harness
            .service
            .request_shutdown(&harness.runtime, Duration::ZERO);
        assert_eq!(*harness.runtime.lifecycle().borrow(), RelayLifecycle::Force);
        let events = harness.events();
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, Event::ShutdownStarted { .. }))
                .count(),
            1,
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::ShutdownForced { .. }))
        );
    }

    #[tokio::test(start_paused = true)]
    async fn zero_grace_with_active_session_records_force_once_before_close() {
        let harness = Harness::new(3, 1, 8, &[]);
        let (prepared, _server, _) = harness.prepare(Role::Server, "ID:4", b"").await;
        let waiter = harness.start(prepared);
        settle().await;
        let (prepared, _viewer, _) = harness.prepare(Role::Viewer, "ID:4", b"").await;
        let owner = harness.start(prepared);
        settle().await;
        assert_eq!(harness.snapshot().active.len(), 1);
        harness
            .service
            .request_shutdown(&harness.runtime, Duration::ZERO);
        harness
            .service
            .request_shutdown(&harness.runtime, Duration::ZERO);
        waiter
            .await
            .unwrap_or_else(|error| panic!("waiter: {error}"))
            .unwrap_or_else(|error| panic!("handoff: {error:?}"));
        owner
            .await
            .unwrap_or_else(|error| panic!("owner: {error}"))
            .unwrap_or_else(|error| panic!("force: {error:?}"));
        let events = harness.events();
        let forced = events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    Event::ShutdownForced {
                        remaining_sessions: 1
                    }
                )
            })
            .unwrap_or_else(|| panic!("one active session forced"));
        let closed = events
            .iter()
            .position(|event| matches!(event, Event::SessionClosed { .. }))
            .unwrap_or_else(|| panic!("forced session closed"));
        assert!(forced < closed);
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, Event::ShutdownForced { .. }))
                .count(),
            1
        );
        assert_eq!(
            harness.snapshot().recent_closed[0].close_cause,
            CloseCause::ServerShutdown
        );
        assert!(harness.snapshot().active.is_empty());
        assert_eq!(free_places(&harness.budget), 3);
    }

    /// spec: operations.md:OPS-008 partial
    #[tokio::test(start_paused = true)]
    async fn empty_shutdown_supervisor_reaches_force_without_a_forced_event() {
        let harness = Harness::new(2, 1, 8, &[]);
        harness
            .service
            .request_shutdown(&harness.runtime, Duration::from_secs(3));
        harness.service.drive_shutdown(&harness.runtime).await;
        assert_eq!(*harness.runtime.lifecycle().borrow(), RelayLifecycle::Force);
        assert!(
            !harness
                .events()
                .iter()
                .any(|event| matches!(event, Event::ShutdownForced { .. }))
        );
    }

    /// spec: relay.md:RLY-018 partial
    /// spec: relay.md:RLY-003 partial
    #[tokio::test(start_paused = true)]
    async fn waiting_partial_reads_append_without_reordering_the_prepared_prefix() {
        let harness = Harness::new(2, 1, 8, &[]);
        let (prepared, mut server, _) = harness.prepare(Role::Server, "ID:14", b"ab").await;
        let waiter = harness.start(prepared);
        settle().await;
        server.write_all(b"cd").await.expect("partial");
        settle().await;
        server.write_all(b"efgh").await.expect("fill B");
        settle().await;
        let (prepared, mut viewer, _) = harness.prepare(Role::Viewer, "ID:14", b"").await;
        let owner = harness.start(prepared);
        let mut bytes = [0; 8];
        viewer.read_exact(&mut bytes).await.expect("buffered bytes");
        assert_eq!(&bytes, b"abcdefgh");
        viewer.shutdown().await.expect("viewer eof");
        server.shutdown().await.expect("server eof");
        waiter.await.expect("waiter").expect("handoff");
        owner.await.expect("owner").expect("session");
    }

    /// spec: relay.md:RLY-014 partial
    #[tokio::test(start_paused = true)]
    async fn clock_crossing_w_during_claim_preparation_never_activates_or_issues_counter() {
        struct JumpSource(Arc<TestClock>);
        impl SessionIdSource for JumpSource {
            fn session_id(&self, counter: u128) -> SessionId {
                self.0.offset.fetch_add(5000, Ordering::Relaxed);
                SessionId::from_block(counter.to_be_bytes())
            }
        }
        let mut harness = Harness::new(3, 1, 8, &[]);
        let clock = harness.clock.clone();
        Arc::get_mut(&mut harness)
            .expect("unique harness")
            .runtime
            .session_ids = Arc::new(JumpSource(clock));
        let (prepared, _server, _) = harness.prepare(Role::Server, "ID:15", b"").await;
        let waiter = harness.start(prepared);
        settle().await;
        let (prepared, _viewer, _) = harness.prepare(Role::Viewer, "ID:15", b"").await;
        assert!(matches!(
            harness.service.dispatch(prepared, &harness.runtime).await,
            Err(RelayDispatchError::WaitingExpired)
        ));
        waiter.await.expect("waiter").expect("handoff");
        assert_eq!(harness.runtime.lock().next_session, Some(0));
        assert!(harness.snapshot().active.is_empty());
        assert_eq!(free_places(&harness.budget), 3);
        assert_eq!(free_places(&harness.waiting), 1);
        assert!(
            !harness
                .events()
                .iter()
                .any(|event| matches!(event, Event::SessionStarted { .. }))
        );
        assert_eq!(
            harness
                .events()
                .iter()
                .filter(|event| matches!(event, Event::WaitingExpired { .. }))
                .count(),
            1
        );
    }

    /// spec: relay.md:RLY-009 partial
    /// spec: operations.md:OPS-023 partial
    #[tokio::test(start_paused = true)]
    async fn target_k_failure_is_terminal_but_other_candidate_errors_are_aggregated() {
        for (first_error, expected_attempts) in [
            (TargetError::KeepaliveFailed, 1),
            (TargetError::ConnectError, 2),
        ] {
            let harness = Harness::new(2, 1, 8, &["host.example:5900", "192.0.2.0/24:5900"]);
            let first_ip = "192.0.2.10".parse().expect("first IP");
            let second_ip = "192.0.2.11".parse().expect("second IP");
            *harness.runtime.connector.resolved.lock().expect("resolve") =
                Ok(vec![first_ip, second_ip]);
            harness
                .runtime
                .connector
                .attempts
                .lock()
                .expect("attempts")
                .extend([
                    Attempt {
                        delay: Duration::ZERO,
                        result: Err(first_error),
                    },
                    Attempt {
                        delay: Duration::ZERO,
                        result: Err(TargetError::ConnectRefused),
                    },
                ]);
            let (prepared, _peer, _) = harness
                .prepare(Role::Viewer, "host.example:5900", b"")
                .await;
            assert!(matches!(
                harness.service.dispatch(prepared, &harness.runtime).await,
                Err(RelayDispatchError::Target(
                    crate::events::TargetFailureReason::ConnectError
                ))
            ));
            assert_eq!(
                harness.runtime.connector.calls.lock().expect("calls").len(),
                expected_attempts
            );
            assert!(harness.snapshot().active.is_empty());
            assert_eq!(harness.runtime.lock().next_session, Some(0));
            assert_eq!(free_places(&harness.budget), 2);
            assert!(matches!(
                harness.events()[0],
                Event::TargetConnectFailed {
                    reason: crate::events::TargetFailureReason::ConnectError,
                    target_ip: Some(ip),
                    ..
                } if ip == if expected_attempts == 1 { first_ip } else { second_ip }
            ));
        }
    }

    /// spec: relay.md:RLY-020 partial
    /// spec: operations.md:OPS-007 partial
    /// spec: operations.md:OPS-008 partial
    #[tokio::test(start_paused = true)]
    async fn pending_preactivation_close_pair_stops_at_grace_deadline_or_second_shutdown() {
        use vnc_relay_domain::matching::WaitingEntry;
        for second_shutdown in [false, true] {
            let harness = Harness::new(3, 1, 8, &[]);
            let (first_prep, _first_peer, first_actions) = harness
                .prepare_with_options(Role::Server, "ID:50", b"", true, None)
                .await;
            let (second_prep, _second_peer, second_actions) = harness
                .prepare_with_options(Role::Viewer, "ID:50", b"", true, None)
                .await;

            let (first_conn, _) = OwnedConnection::from_prepared(first_prep);
            let (second_conn, _) = OwnedConnection::from_prepared(second_prep);
            let (server_endpoint, server_context) = first_conn.into_endpoint();
            let claim = MatchClaim {
                viewer_conn_id: second_conn.context.conn_id,
                server_conn_id: server_context.conn_id,
                waiter: WaitingEntry {
                    role: Role::Server,
                    conn_id: server_context.conn_id,
                    registered_at: harness.clock.monotonic(),
                    deadline: harness.clock.monotonic() + Duration::from_secs(10),
                },
                matched_at: harness.clock.monotonic(),
            };
            let pair = SessionPair {
                viewer: second_conn,
                other: server_endpoint,
                peer: ActivePeer::Server(server_context),
                target_host: None,
                relay: Some((
                    RelayId::parse("50").expect("id"),
                    RelayIdRef::from_hash_prefix([1; 8]),
                    claim,
                )),
            };

            harness
                .service
                .request_shutdown(&harness.runtime, Duration::from_secs(3));
            let task = {
                let harness = harness.clone();
                tokio::spawn(async move { harness.service.run_pair(&harness.runtime, pair).await })
            };
            settle().await;
            assert!(
                first_actions
                    .lock()
                    .expect("actions")
                    .contains(&RelayCloseAction::ServerShutdown)
            );
            assert!(
                second_actions
                    .lock()
                    .expect("actions")
                    .contains(&RelayCloseAction::ServerShutdown)
            );
            assert_eq!(free_places(&harness.budget), 1);

            if second_shutdown {
                harness
                    .service
                    .request_shutdown(&harness.runtime, Duration::ZERO);
                assert!(task.await.expect("task").is_err());
            } else {
                tokio::time::advance(Duration::from_secs(3)).await;
                assert!(task.await.expect("task").is_err());
            }
            assert_eq!(free_places(&harness.budget), 3);
        }
    }

    /// spec: relay.md:RLY-020 partial
    /// spec: operations.md:OPS-007 partial
    #[tokio::test(start_paused = true)]
    async fn shutdown_during_preactivation_records_waiting_removed_for_predecessor_and_register_for_incoming_in_both_roles()
     {
        use crate::events::WaitingRemovalReason;
        use vnc_relay_domain::matching::WaitingEntry;
        for first_role in [Role::Server, Role::Viewer] {
            let second_role = if first_role == Role::Server {
                Role::Viewer
            } else {
                Role::Server
            };
            let harness = Harness::new(3, 1, 8, &[]);
            let (first_prep, _first_peer, _) = harness.prepare(first_role, "ID:55", b"").await;
            let (second_prep, _second_peer, _) = harness.prepare(second_role, "ID:55", b"").await;

            let (first_conn, _) = OwnedConnection::from_prepared(first_prep);
            let (second_conn, _) = OwnedConnection::from_prepared(second_prep);
            let (viewer, server) = if first_role == Role::Viewer {
                (first_conn, second_conn)
            } else {
                (second_conn, first_conn)
            };
            let (server_endpoint, server_context) = server.into_endpoint();
            let pred_conn_id = if first_role == Role::Viewer {
                viewer.context.conn_id
            } else {
                server_context.conn_id
            };
            let incoming_conn_id = if first_role == Role::Viewer {
                server_context.conn_id
            } else {
                viewer.context.conn_id
            };

            let claim = MatchClaim {
                viewer_conn_id: viewer.context.conn_id,
                server_conn_id: server_context.conn_id,
                waiter: WaitingEntry {
                    role: first_role,
                    conn_id: pred_conn_id,
                    registered_at: harness.clock.monotonic(),
                    deadline: harness.clock.monotonic() + Duration::from_secs(10),
                },
                matched_at: harness.clock.monotonic(),
            };
            let pair = SessionPair {
                viewer,
                other: server_endpoint,
                peer: ActivePeer::Server(server_context),
                target_host: None,
                relay: Some((
                    RelayId::parse("55").expect("id"),
                    RelayIdRef::from_hash_prefix([5; 8]),
                    claim,
                )),
            };

            harness
                .service
                .request_shutdown(&harness.runtime, Duration::from_secs(3));
            let result = harness.service.run_pair(&harness.runtime, pair).await;
            assert!(matches!(
                result,
                Err(RelayDispatchError::Aborted(AbortReason::ServerShutdown))
            ));

            let events = harness.events();
            let waiting_removed = events.iter().find(
                |e| matches!(e, Event::WaitingRemoved { conn_id, .. } if *conn_id == pred_conn_id),
            );
            assert!(
                waiting_removed.is_some(),
                "predecessor must receive WaitingRemoved"
            );
            assert!(
                matches!(waiting_removed.unwrap(), Event::WaitingRemoved { reason: WaitingRemovalReason::ServerShutdown, role, .. } if *role == first_role)
            );

            let aborted = events.iter().find(|e| matches!(e, Event::RelayAborted { conn_id, .. } if *conn_id == incoming_conn_id));
            assert!(aborted.is_some(), "incoming must receive RelayAborted");
            assert!(matches!(
                aborted.unwrap(),
                Event::RelayAborted {
                    stage: Stage::Register,
                    reason: AbortReason::ServerShutdown,
                    ..
                }
            ));

            assert!(!events.iter().any(|e| matches!(
                e,
                Event::RelayAborted {
                    stage: Stage::InitialExchange,
                    ..
                } | Event::ConnectionAborted {
                    stage: Stage::InitialExchange,
                    ..
                }
            )));
            assert_eq!(free_places(&harness.budget), 3);
        }
    }

    /// spec: operations.md:OPS-007 partial
    /// spec: operations.md:OPS-008 partial
    #[tokio::test(start_paused = true)]
    async fn controlled_clock_g_exact_completion_wake_forces_shutdown_before_closed() {
        let harness = Harness::new(3, 1, 8, &[]);
        let (prepared, mut server, _) = harness.prepare(Role::Server, "ID:70", b"").await;
        let waiter = harness.start(prepared);
        settle().await;
        let (prepared, mut viewer, _) = harness.prepare(Role::Viewer, "ID:70", b"").await;
        let owner = harness.start(prepared);
        settle().await;
        assert_eq!(harness.snapshot().active.len(), 1);

        harness
            .service
            .request_shutdown(&harness.runtime, Duration::from_secs(3));
        harness.clock.offset.fetch_add(3000, Ordering::Relaxed);
        viewer.shutdown().await.expect("viewer shutdown");
        server.shutdown().await.expect("server shutdown");
        waiter.await.expect("waiter").expect("handoff");
        owner.await.expect("owner").expect("forced session");

        let events = harness.events();
        let forced_idx = events.iter().position(|e| {
            matches!(
                e,
                Event::ShutdownForced {
                    remaining_sessions: 1
                }
            )
        });
        let closed_idx = events.iter().position(|e| {
            matches!(
                e,
                Event::SessionClosed {
                    close_cause: CloseCause::ServerShutdown,
                    ..
                }
            )
        });
        assert!(forced_idx.is_some(), "ShutdownForced must be recorded");
        assert!(
            closed_idx.is_some(),
            "SessionClosed with ServerShutdown must be recorded"
        );
        assert!(
            forced_idx.unwrap() < closed_idx.unwrap(),
            "ShutdownForced must precede SessionClosed"
        );
    }

    /// spec: operations.md:OPS-007 partial
    /// spec: operations.md:OPS-008 partial
    #[tokio::test(start_paused = true)]
    async fn cleanup_in_progress_is_counted_on_force_and_preserves_completed_engine_cause() {
        let harness = Harness::new(3, 1, 8, &[]);
        let harness_clone = harness.clone();
        let drop_fired = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let drop_fired_clone = drop_fired.clone();
        let on_drop = Arc::new(move || {
            drop_fired_clone.store(true, Ordering::SeqCst);
            assert!(
                harness_clone
                    .runtime
                    .lock()
                    .matching
                    .get(&RelayId::parse("71").expect("id"))
                    .is_some(),
                "ID must still be held during stream drop"
            );
            harness_clone
                .service
                .request_shutdown(&harness_clone.runtime, Duration::ZERO);
        });

        let (prepared, mut server, _) = harness
            .prepare_with_options(Role::Server, "ID:71", b"", false, Some(on_drop))
            .await;
        let waiter = harness.start(prepared);
        settle().await;
        let (prepared, mut viewer, _) = harness.prepare(Role::Viewer, "ID:71", b"").await;
        let owner = harness.start(prepared);
        settle().await;
        assert_eq!(harness.snapshot().active.len(), 1);

        harness
            .service
            .request_shutdown(&harness.runtime, Duration::from_secs(3));
        harness.clock.offset.fetch_add(2999, Ordering::Relaxed);

        server.shutdown().await.expect("server shutdown");
        viewer.shutdown().await.expect("viewer shutdown");
        waiter.await.expect("waiter").expect("handoff");
        owner.await.expect("owner").expect("completed session");

        assert!(
            drop_fired.load(Ordering::SeqCst),
            "on_drop hook must fire during stream drop"
        );
        let events = harness.events();
        let closed = events
            .iter()
            .find(|e| matches!(e, Event::SessionClosed { .. }))
            .expect("SessionClosed");
        assert!(
            matches!(
                closed,
                Event::SessionClosed {
                    close_cause: CloseCause::PeerEof,
                    ..
                }
            ),
            "close cause must stand as PeerEof"
        );
        assert!(
            events.iter().any(|e| matches!(
                e,
                Event::ShutdownForced {
                    remaining_sessions: 1
                }
            )),
            "the session remains active until streams, permits and registration are reclaimed"
        );
    }

    /// spec: management.md:MGT-005 partial
    #[tokio::test(start_paused = true)]
    async fn recent_refs_consistency_at_capacity_evicts_exact_reference() {
        let harness = Harness::new_with_recent(3, 1, 8, 2);
        let mut session_ids = Vec::new();
        for id_str in ["ID:81", "ID:82", "ID:83"] {
            let (prepared, mut server, _) = harness.prepare(Role::Server, id_str, b"").await;
            let waiter = harness.start(prepared);
            settle().await;
            let (prepared, mut viewer, _) = harness.prepare(Role::Viewer, id_str, b"").await;
            let owner = harness.start(prepared);
            settle().await;
            server.shutdown().await.expect("server eof");
            viewer.shutdown().await.expect("viewer eof");
            waiter.await.expect("waiter").expect("handoff");
            owner.await.expect("owner").expect("session");
            let snap = harness.snapshot();
            session_ids.push(snap.recent_closed[0].session_id.clone());
        }
        let snap = harness.snapshot();
        assert_eq!(snap.recent_closed.len(), 2);
        assert_eq!(snap.recent_closed[0].session_id, session_ids[2]);
        assert_eq!(snap.recent_closed[1].session_id, session_ids[1]);
        assert!(snap.recent_closed[0].relay_id_ref.is_some());
        assert!(snap.recent_closed[1].relay_id_ref.is_some());

        let state = harness.runtime.lock();
        assert_eq!(state.recent_refs.len(), 2);
        assert!(
            !state.recent_refs.contains_key(&session_ids[0]),
            "evicted session ref must be removed"
        );
        assert!(
            state.recent_refs.contains_key(&session_ids[1]),
            "retained session 2 ref must exist"
        );
        assert!(
            state.recent_refs.contains_key(&session_ids[2]),
            "retained session 3 ref must exist"
        );
    }

    /// spec: management.md:MGT-005 partial
    #[tokio::test(start_paused = true)]
    async fn recent_refs_consistency_with_zero_capacity_never_retains_references() {
        let harness = Harness::new_with_recent(3, 1, 8, 0);
        let (prepared, mut server, _) = harness.prepare(Role::Server, "ID:90", b"").await;
        let waiter = harness.start(prepared);
        settle().await;
        let (prepared, mut viewer, _) = harness.prepare(Role::Viewer, "ID:90", b"").await;
        let owner = harness.start(prepared);
        settle().await;
        server.shutdown().await.expect("server eof");
        viewer.shutdown().await.expect("viewer eof");
        waiter.await.expect("waiter").expect("handoff");
        owner.await.expect("owner").expect("session");

        let snap = harness.snapshot();
        assert!(
            snap.recent_closed.is_empty(),
            "R=0 must keep zero recent_closed"
        );
        let state = harness.runtime.lock();
        assert!(
            state.recent_refs.is_empty(),
            "R=0 must never retain recent_refs"
        );
    }

    /// spec: management.md:MGT-026 partial
    #[test]
    fn handshaking_guard_increments_and_decrements_on_every_exit_path() {
        let harness = Harness::new(4, 1, 8, &[]);
        assert_eq!(harness.snapshot().handshaking, 0);

        // 1. Normal drop
        {
            let guard = harness.runtime.track_handshaking();
            assert_eq!(harness.snapshot().handshaking, 1);
            drop(guard);
        }
        assert_eq!(harness.snapshot().handshaking, 0);

        // 2. Early return / error drop
        let early_return = || -> Result<(), ()> {
            let _guard = harness.runtime.track_handshaking();
            assert_eq!(harness.snapshot().handshaking, 1);
            Err(())
        };
        assert!(early_return().is_err());
        assert_eq!(harness.snapshot().handshaking, 0);

        // 3. Unwind / panic drop
        let panic_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = harness.runtime.track_handshaking();
            assert_eq!(harness.snapshot().handshaking, 1);
            panic!("test exit path");
        }));
        assert!(panic_result.is_err());
        assert_eq!(harness.snapshot().handshaking, 0);
    }

    /// spec: management.md:MGT-016 partial
    /// spec: management.md:MGT-026 partial
    #[tokio::test(start_paused = true)]
    async fn snapshot_copy_is_consistent_under_one_lock() {
        let harness = Harness::new(4, 1, 8, &["192.0.2.10:5900"]);
        let (prepared, _server, _) = harness.prepare(Role::Server, "ID:1", b"").await;
        let _waiter = harness.start(prepared);
        settle().await;

        let _guard = harness.runtime.track_handshaking();
        let snap = harness.snapshot();

        assert_eq!(snap.handshaking, 1);
        assert_eq!(snap.target_connecting, 0);
        assert_eq!(snap.waiting.len(), 1);
        assert_eq!(snap.active.len(), 0);
        assert_eq!(snap.max_entries, 2);
        assert_eq!(snap.waiting[0].relay_id.as_str(), "1");
    }

    /// Owner cancellation is a real cleanup-confirmation failure, not a fake
    /// transport error. The HTTP 500 branch is exercised separately at the service boundary.
    /// spec: management.md:MGT-012 partial
    /// spec: operations.md:OPS-011 partial
    #[tokio::test(start_paused = true)]
    async fn accepted_admin_owner_cancellation_releases_resources_and_reports_failure_after_closed()
    {
        let harness = Harness::new(4, 1, 8, &[]);
        let (prepared, _server, _) = harness.prepare(Role::Server, "ID:97", b"").await;
        let waiter = harness.start(prepared);
        for _ in 0..100 {
            if harness.snapshot().waiting.len() == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(harness.snapshot().waiting.len(), 1);
        let (prepared, _viewer, _) = harness.prepare(Role::Viewer, "ID:97", b"").await;
        let owner = harness.start(prepared);
        for _ in 0..100 {
            if harness.snapshot().active.len() == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let id = harness.snapshot().active[0].session_id.clone();
        let account = AccountName::parse("alice").unwrap_or_else(|e| panic!("account: {e}"));
        let peer = PeerIp::new(std::net::Ipv4Addr::LOCALHOST.into());
        let mut ticket = harness
            .runtime
            .request_terminate(
                id.as_str(),
                account.clone(),
                peer,
                harness.clock.as_ref(),
                harness.events.as_ref(),
            )
            .unwrap_or_else(|| panic!("active termination ticket"));
        assert!(ticket.initiated);
        owner.abort();
        assert!(
            owner
                .await
                .unwrap_or_else(|e| {
                    assert!(e.is_cancelled());
                    Err(RelayDispatchError::Aborted(AbortReason::ServerShutdown))
                })
                .is_err()
        );
        waiter
            .await
            .unwrap_or_else(|e| panic!("waiter: {e}"))
            .unwrap_or_else(|e| panic!("handoff: {e}"));
        ticket
            .completion
            .changed()
            .await
            .unwrap_or_else(|e| panic!("completion: {e}"));
        assert!(matches!(&*ticket.completion.borrow(), Some(Err(()))));
        assert!(harness.snapshot().active.is_empty());
        assert_eq!(free_places(&harness.budget), 4);
        assert!(
            harness
                .runtime
                .lock()
                .matching
                .get(&RelayId::parse("97").unwrap_or_else(|e| panic!("relay id: {e}")))
                .is_none()
        );
        let events = harness.events();
        let closed: Vec<_> = events
            .iter()
            .enumerate()
            .filter(|(_, event)| {
                matches!(
                    event, Event::SessionClosed { session_id, .. } if session_id == &id
                )
            })
            .collect();
        assert_eq!(closed.len(), 1);
        assert!(matches!(closed[0].1, Event::SessionClosed {
            close_cause: CloseCause::AdminTerminated, admin_account: Some(actual), ..
        } if actual == &account));
        let audit = events.iter().position(|event| matches!(event,
            Event::AdminTerminate { session_id: Some(actual), result: AdminTerminateResult::Failed, .. }
            if actual == &id)).unwrap_or_else(|| panic!("failed audit"));
        assert!(closed[0].0 < audit);
    }

    async fn active_for_review(
        harness: &Arc<Harness>,
        id: &str,
    ) -> (
        tokio::task::JoinHandle<Result<(), RelayDispatchError>>,
        tokio::task::JoinHandle<Result<(), RelayDispatchError>>,
        DuplexStream,
        DuplexStream,
    ) {
        let (prepared, server, _) = harness.prepare(Role::Server, id, b"").await;
        let waiter = harness.start(prepared);
        for _ in 0..100 {
            if harness.snapshot().waiting.len() == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(harness.snapshot().waiting.len(), 1);
        let (prepared, viewer, _) = harness.prepare(Role::Viewer, id, b"").await;
        let owner = harness.start(prepared);
        for _ in 0..100 {
            if harness.snapshot().active.len() == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(harness.snapshot().active.len(), 1);
        (owner, waiter, server, viewer)
    }

    /// spec: management.md:MGT-011 partial
    #[tokio::test(start_paused = true)]
    async fn acceptance_at_exact_g_deadline_reads_clock_under_registry_lock_and_joins_force() {
        struct AcceptanceClock<'a>(&'a RelayRuntime<Connector>, Duration);
        impl Clock for AcceptanceClock<'_> {
            fn monotonic(&self) -> Duration {
                assert!(
                    self.0.state.try_lock().is_err(),
                    "accept time must be read under registry lock"
                );
                self.1
            }
            fn wall(&self) -> SystemTime {
                SystemTime::UNIX_EPOCH + self.1
            }
        }
        let harness = Harness::new(4, 1, 8, &[]);
        let (owner, waiter, _server, _viewer) = active_for_review(&harness, "ID:98").await;
        let id = harness.snapshot().active[0].session_id.clone();
        let deadline = harness.clock.monotonic() + Duration::from_secs(1);
        // Do not wake the supervisor: acceptance itself must publish Force.
        harness.runtime.lock().lifecycle = RelayLifecycle::Grace { deadline };
        let clock = AcceptanceClock(&harness.runtime, deadline);
        let ticket = harness
            .runtime
            .request_terminate(
                id.as_str(),
                AccountName::parse("alice").unwrap_or_else(|e| panic!("{e}")),
                PeerIp::new(std::net::Ipv4Addr::LOCALHOST.into()),
                &clock,
                harness.events.as_ref(),
            )
            .unwrap_or_else(|| panic!("join ticket"));
        assert!(!ticket.initiated);
        assert_eq!(harness.runtime.lock().lifecycle, RelayLifecycle::Force);
        owner
            .await
            .unwrap_or_else(|e| panic!("{e}"))
            .unwrap_or_else(|e| panic!("{e}"));
        waiter
            .await
            .unwrap_or_else(|e| panic!("{e}"))
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(matches!(&*ticket.completion.borrow(), Some(Ok(closed))
            if closed.close_cause == CloseCause::ServerShutdown));
        assert!(!harness.events().iter().any(|event| matches!(
            event,
            Event::SessionClosed {
                close_cause: CloseCause::AdminTerminated,
                ..
            }
        )));
    }

    /// spec: operations.md:OPS-011 partial
    #[tokio::test(start_paused = true)]
    async fn owner_cancellation_uses_shutdown_cause_only_in_grace_or_force_and_retains_history() {
        for lifecycle in [
            RelayLifecycle::Running,
            RelayLifecycle::Grace {
                deadline: Duration::from_secs(60),
            },
            RelayLifecycle::Force,
        ] {
            let harness = Harness::new(4, 1, 8, &[]);
            let (owner, waiter, _server, _viewer) = active_for_review(&harness, "ID:99").await;
            harness.runtime.lock().lifecycle = lifecycle;
            owner.abort();
            match owner.await {
                Err(error) => assert!(error.is_cancelled()),
                Ok(_) => panic!("owner must be cancelled"),
            }
            waiter
                .await
                .unwrap_or_else(|e| panic!("{e}"))
                .unwrap_or_else(|e| panic!("{e}"));
            let expected = if lifecycle == RelayLifecycle::Running {
                CloseCause::InternalError
            } else {
                CloseCause::ServerShutdown
            };
            let snapshot = harness.snapshot();
            assert!(snapshot.active.is_empty());
            assert_eq!(snapshot.recent_closed.len(), 1);
            assert_eq!(snapshot.recent_closed[0].close_cause, expected);
            assert_eq!(free_places(&harness.budget), 4);
            let closed: Vec<_> = harness
                .events()
                .into_iter()
                .filter(|event| matches!(event, Event::SessionClosed { .. }))
                .collect();
            assert_eq!(closed.len(), 1);
            assert!(
                matches!(&closed[0], Event::SessionClosed { close_cause, .. } if *close_cause == expected)
            );
            assert_eq!(closed[0].level(), crate::events::Level::Warn);
        }
    }

    /// spec: relay.md:RLY-020 partial
    /// spec: operations.md:OPS-011 partial
    #[tokio::test(start_paused = true)]
    async fn panic_after_eof_in_one_engine_poll_keeps_log_and_history_identical() {
        let harness = Harness::new(4, 1, 8, &[]);
        let eof = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (mut prepared, _server, _) = harness.prepare(Role::Server, "ID:101", b"").await;
        prepared.stream_mut().panic_after_eof = Some(Arc::clone(&eof));
        let waiter = harness.start(prepared);
        for _ in 0..100 {
            if harness.snapshot().waiting.len() == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(harness.snapshot().waiting.len(), 1);
        let (mut prepared, mut viewer, _) = harness.prepare(Role::Viewer, "ID:101", b"").await;
        prepared.stream_mut().eof_signal = Some(Arc::clone(&eof));
        viewer
            .shutdown()
            .await
            .unwrap_or_else(|e| panic!("viewer EOF: {e}"));
        let owner = harness.start(prepared);
        match owner.await {
            Err(error) => assert!(error.is_panic()),
            Ok(_) => panic!("controlled stream must panic"),
        }
        waiter
            .await
            .unwrap_or_else(|e| panic!("waiter: {e}"))
            .unwrap_or_else(|e| panic!("handoff: {e}"));
        assert!(eof.load(Ordering::SeqCst));
        let snapshot = harness.snapshot();
        assert!(snapshot.active.is_empty());
        assert_eq!(snapshot.recent_closed.len(), 1);
        let recent = &snapshot.recent_closed[0];
        assert_eq!(recent.close_cause, CloseCause::PeerEof);
        assert_eq!(recent.side, Some(vnc_relay_domain::session::Side::Viewer));
        let events = harness.events();
        let closed: Vec<_> = events
            .iter()
            .filter(|event| matches!(event, Event::SessionClosed { .. }))
            .collect();
        assert_eq!(closed.len(), 1);
        assert!(matches!(closed[0], Event::SessionClosed {
            session_id, close_cause, side, started_at, duration_ms,
            bytes_viewer_to_target, bytes_target_to_viewer, error_kind, ..
        } if session_id == &recent.session_id
            && *close_cause == recent.close_cause && *side == recent.side
            && *started_at == recent.started_at && *duration_ms == milliseconds(recent.duration)
            && *bytes_viewer_to_target == recent.bytes_viewer_to_target
            && *bytes_target_to_viewer == recent.bytes_target_to_viewer
            && error_kind.is_none()));
        assert_eq!(free_places(&harness.budget), 4);
    }
}
