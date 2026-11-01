//! One-lock Mode II registration, bounded waiting reads, and exact-ID handoff.

use std::future::{Future, poll_fn};
use std::pin::{Pin, pin};
use std::task::{Poll, Waker};

use tokio::io::ReadBuf;
use vnc_relay_domain::budget::BudgetPermit;
use vnc_relay_domain::entry_point::Role;
use vnc_relay_domain::handshake::Mode;
use vnc_relay_domain::matching::{ArrivalOutcome, MatchClaim, MatchingError, WaitingEntry};
use vnc_relay_domain::relay_id::RelayId;

use super::{
    ActivePeer, OwnedConnection, RelayDispatchError, RelayRuntime, RelayService, SessionPair,
    milliseconds,
};
use crate::events::{AbortReason, Event, RejectReason, RelayIdRef, Stage, WaitingRemovalReason};
use crate::ports::{RelayCloseAction, RelayLifecycle, RelayStreamError, TargetConnector};

pub(super) struct WaitingConnection {
    pub(super) connection: OwnedConnection,
    pub(super) metadata: WaitingEntry,
    pub(super) reference: RelayIdRef,
    filled: usize,
    waiting_permit: Option<BudgetPermit>,
    waker: Option<Waker>,
}

impl WaitingConnection {
    fn into_connection(mut self) -> OwnedConnection {
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
        self.connection.trailing.truncate(self.filled);
        drop(self.waiting_permit);
        self.connection
    }
}

#[expect(
    clippy::large_enum_variant,
    reason = "one stack-owned registration transition avoids allocating another box for a matched connection"
)]
enum Registration {
    Waiting(WaitingEntry),
    Matched(OwnedConnection, OwnedConnection, MatchClaim),
    Rejected(OwnedConnection, MatchingError),
    Shutdown(OwnedConnection),
}

enum RemovedReason {
    Expired,
    Removed(WaitingRemovalReason),
}

struct Removed {
    entry: WaitingConnection,
    reason: RemovedReason,
}

struct WaitingGuard<'a, C> {
    runtime: &'a RelayRuntime<C>,
    id: &'a RelayId,
    conn_id: vnc_relay_domain::conn_id::ConnId,
}

impl<C> Drop for WaitingGuard<'_, C> {
    fn drop(&mut self) {
        let removed = {
            let mut state = self.runtime.lock();
            if state
                .matching
                .remove_waiting(self.id, self.conn_id)
                .is_some()
            {
                state.waiting.remove(self.id).map(|mut entry| {
                    drop(entry.waiting_permit.take());
                    entry
                })
            } else {
                None
            }
        };
        drop(removed);
    }
}

struct ClaimGuard<'a, C> {
    runtime: &'a RelayRuntime<C>,
    id: &'a RelayId,
    claim: Option<MatchClaim>,
}

impl<C> Drop for ClaimGuard<'_, C> {
    fn drop(&mut self) {
        if let Some(claim) = self.claim {
            self.runtime.lock().matching.cancel_claim(self.id, &claim);
        }
    }
}

pub(super) async fn dispatch<C: TargetConnector>(
    service: &RelayService,
    runtime: &RelayRuntime<C>,
    mut incoming: OwnedConnection,
    id: RelayId,
) -> Result<(), RelayDispatchError> {
    let reference = runtime.relay_ids.reference(&id);
    let context = incoming.context;
    let mut expired = None;
    let registration = {
        let mut state = runtime.lock();
        if state.lifecycle != RelayLifecycle::Running {
            Registration::Shutdown(incoming)
        } else {
            loop {
                match state.matching.arrive(
                    &id,
                    context.entry_point.role(),
                    context.conn_id,
                    service.clock.monotonic(),
                ) {
                    ArrivalOutcome::Expired(old) => {
                        if let Some(mut entry) = state.waiting.remove(&id) {
                            debug_assert_eq!(old.conn_id, entry.metadata.conn_id);
                            drop(entry.waiting_permit.take());
                            expired = Some(Removed {
                                entry,
                                reason: RemovedReason::Expired,
                            });
                        }
                    }
                    ArrivalOutcome::Waiting(metadata) => {
                        if let Some(waiting_permit) = runtime.waiting_budget.try_acquire() {
                            let filled = incoming.trailing.len();
                            incoming
                                .trailing
                                .reserve_exact(service.buffer_bytes.saturating_sub(filled));
                            incoming.trailing.resize(service.buffer_bytes, 0);
                            state.waiting.insert(
                                id.clone(),
                                WaitingConnection {
                                    connection: incoming,
                                    metadata,
                                    reference: reference.clone(),
                                    filled,
                                    waiting_permit: Some(waiting_permit),
                                    waker: None,
                                },
                            );
                            service.events.record(&Event::WaitingRegistered {
                                conn_id: context.conn_id,
                                entry_point: context.entry_point,
                                peer_ip: context.peer_ip,
                                relay_id_ref: reference.clone(),
                            });
                            break Registration::Waiting(metadata);
                        }
                        state.matching.remove_waiting(&id, context.conn_id);
                        break Registration::Rejected(incoming, MatchingError::WaitingFull);
                    }
                    ArrivalOutcome::Claimed(claim) => {
                        if let Some(entry) = state.waiting.remove(&id) {
                            // Lw is returned at this transition, before unlocking.
                            // The stream and L remain owned by the matched pair.
                            break Registration::Matched(incoming, entry.into_connection(), claim);
                        }
                        state.matching.cancel_claim(&id, &claim);
                        break Registration::Rejected(incoming, MatchingError::InvalidClaim);
                    }
                    ArrivalOutcome::Rejected(error) => {
                        break Registration::Rejected(incoming, error);
                    }
                }
            }
        }
    };
    let _waiting_guard = WaitingGuard {
        runtime,
        id: &id,
        conn_id: context.conn_id,
    };
    let _claim_guard = ClaimGuard {
        runtime,
        id: &id,
        claim: match &registration {
            Registration::Matched(_, _, claim) => Some(*claim),
            _ => None,
        },
    };
    let registration = registration;
    if let Some(expired) = expired {
        cleanup(service, runtime, expired).await;
    }
    match registration {
        Registration::Waiting(metadata) => wait(service, runtime, &id, metadata).await,
        Registration::Matched(incoming, other, claim) => {
            let (viewer, server) = if context.entry_point.role() == Role::Viewer {
                (incoming, other)
            } else {
                (other, incoming)
            };
            let (server, server_context) = server.into_endpoint();
            service
                .run_pair(
                    runtime,
                    SessionPair {
                        viewer,
                        other: server,
                        peer: ActivePeer::Server(server_context),
                        target_host: None,
                        relay: Some((id.clone(), reference, claim)),
                    },
                )
                .await
        }
        Registration::Rejected(connection, error) => {
            let (reason, action) = match error {
                MatchingError::DuplicateId => {
                    (RejectReason::DuplicateId, RelayCloseAction::RejectDuplicate)
                }
                MatchingError::WaitingFull => (
                    RejectReason::WaitingFull {
                        limit: runtime.waiting_budget.limit(),
                    },
                    RelayCloseAction::WaitingFull,
                ),
                _ => {
                    drop(connection);
                    return Err(RelayDispatchError::Matching(error));
                }
            };
            service.reject_connection(connection, action, runtime).await;
            service.events.record(&Event::RegistrationRejected {
                conn_id: context.conn_id,
                entry_point: context.entry_point,
                peer_ip: context.peer_ip,
                reason,
                relay_id_ref: reference,
            });
            Err(RelayDispatchError::Rejected(reason))
        }
        Registration::Shutdown(connection) => {
            service
                .reject_connection(connection, RelayCloseAction::ServerShutdown, runtime)
                .await;
            service.aborted(
                context,
                Stage::Register,
                Mode::Mode2,
                AbortReason::ServerShutdown,
            );
            Err(RelayDispatchError::Aborted(AbortReason::ServerShutdown))
        }
    }
}

async fn wait<C: TargetConnector>(
    service: &RelayService,
    runtime: &RelayRuntime<C>,
    id: &RelayId,
    metadata: WaitingEntry,
) -> Result<(), RelayDispatchError> {
    let mut lifecycle = runtime.lifecycle();
    let mut stopped = pin!(lifecycle.wait_for(|phase| *phase != RelayLifecycle::Running));
    let mut timer = pin!(tokio::time::sleep(
        metadata.deadline.saturating_sub(service.clock.monotonic())
    ));
    let removed = poll_fn(|cx| {
        let shutdown = stopped.as_mut().poll(cx).is_ready();
        let expired =
            service.clock.monotonic() >= metadata.deadline || timer.as_mut().poll(cx).is_ready();
        let mut state = runtime.lock();
        let Some(entry) = state.waiting.get_mut(id) else {
            return Poll::Ready(None);
        };
        if entry.metadata.conn_id != metadata.conn_id {
            return Poll::Ready(None);
        }
        let mut reason = if state.lifecycle != RelayLifecycle::Running || shutdown {
            Some(RemovedReason::Removed(WaitingRemovalReason::ServerShutdown))
        } else if expired {
            Some(RemovedReason::Expired)
        } else {
            None
        };
        if reason.is_none() {
            let entry = match state.waiting.get_mut(id) {
                Some(entry) => entry,
                None => return Poll::Ready(None),
            };
            entry.waker = Some(cx.waker().clone());
            if let Poll::Ready(Err(error)) =
                Pin::new(entry.connection.stream.as_mut()).poll_maintenance(cx)
            {
                reason = Some(RemovedReason::Removed(
                    if RelayStreamError::is_dead_connection(&error) {
                        WaitingRemovalReason::DeadConnection
                    } else {
                        WaitingRemovalReason::ConnectionError(RelayStreamError::classify(&error))
                    },
                ));
            }
            if reason.is_none() && entry.filled < entry.connection.trailing.len() {
                let mut buffer = ReadBuf::new(&mut entry.connection.trailing[entry.filled..]);
                match Pin::new(entry.connection.stream.as_mut()).poll_read(cx, &mut buffer) {
                    Poll::Ready(Ok(())) if buffer.filled().is_empty() => {
                        reason = Some(RemovedReason::Removed(WaitingRemovalReason::PeerEof));
                    }
                    Poll::Ready(Ok(())) => {
                        entry.filled += buffer.filled().len();
                        cx.waker().wake_by_ref();
                    }
                    Poll::Ready(Err(error)) => {
                        reason = Some(RemovedReason::Removed(
                            if RelayStreamError::is_dead_connection(&error) {
                                WaitingRemovalReason::DeadConnection
                            } else {
                                WaitingRemovalReason::ConnectionError(RelayStreamError::classify(
                                    &error,
                                ))
                            },
                        ));
                    }
                    Poll::Pending => {}
                }
            }
        }
        if state.lifecycle == RelayLifecycle::Running
            && !shutdown
            && service.clock.monotonic() >= metadata.deadline
        {
            reason = Some(RemovedReason::Expired);
        }
        if let Some(reason) = reason {
            state.matching.remove_waiting(id, metadata.conn_id);
            Poll::Ready(state.waiting.remove(id).map(|mut entry| {
                drop(entry.waiting_permit.take());
                Removed { entry, reason }
            }))
        } else {
            Poll::Pending
        }
    })
    .await;
    if let Some(removed) = removed {
        cleanup(service, runtime, removed).await;
    }
    Ok(())
}

async fn cleanup<C: TargetConnector>(
    service: &RelayService,
    runtime: &RelayRuntime<C>,
    removed: Removed,
) {
    let metadata = removed.entry.metadata;
    let reference = removed.entry.reference.clone();
    let waited_ms = milliseconds(
        service
            .clock
            .monotonic()
            .saturating_sub(metadata.registered_at),
    );
    let connection = removed.entry.into_connection();
    let action = match removed.reason {
        RemovedReason::Expired => RelayCloseAction::WaitingExpired,
        RemovedReason::Removed(WaitingRemovalReason::ServerShutdown) => {
            RelayCloseAction::ServerShutdown
        }
        RemovedReason::Removed(WaitingRemovalReason::PeerEof) => RelayCloseAction::PeerEof,
        RemovedReason::Removed(WaitingRemovalReason::ConnectionError(kind)) => {
            RelayCloseAction::StreamError(kind)
        }
        RemovedReason::Removed(WaitingRemovalReason::DeadConnection) => {
            RelayCloseAction::DeadConnection
        }
    };
    service.reject_connection(connection, action, runtime).await;
    match removed.reason {
        RemovedReason::Expired => service.events.record(&Event::WaitingExpired {
            conn_id: metadata.conn_id,
            role: metadata.role,
            relay_id_ref: reference,
            waited_ms,
        }),
        RemovedReason::Removed(reason) => service.events.record(&Event::WaitingRemoved {
            conn_id: metadata.conn_id,
            role: metadata.role,
            relay_id_ref: reference,
            waited_ms,
            reason,
        }),
    }
}
