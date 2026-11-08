//! Administrator termination and completion waits (management-api-v1 §1.9).
//! The relay owner retains the initiating audit even when HTTP is cancelled.

use std::fmt;
use std::sync::Arc;
use std::time::SystemTime;

use tokio::sync::watch;
use vnc_relay_domain::account::AccountName;
use vnc_relay_domain::peer_ip::PeerIp;
use vnc_relay_domain::session::SessionId;

use crate::events::{AdminTerminateResult, Event};
use crate::ports::{Clock, EventSink, RelaySessionControl};
use crate::relay::ClosedSessionSnapshot;

/// A relay owner's retained completion, acquired synchronously at acceptance.
/// `initiated` is fixed under the runtime lock (management-api-v1 §1.9).
pub struct TerminationTicket {
    pub initiated: bool,
    pub completion: watch::Receiver<Option<Result<Arc<ClosedSessionSnapshot>, ()>>>,
}

/// An accepted command and its immutable audit context. No runtime lock is held.
pub struct AcceptedTermination {
    ticket: TerminationTicket,
    session_id: SessionId,
    account: AccountName,
    peer: PeerIp,
}

/// Successful termination with the actual, retained final session summary.
/// The summary does not depend on recent-closure retention (management-api-v1 §1.9).
pub struct TerminateView {
    pub as_of: SystemTime,
    pub result: AdminTerminateResult,
    pub closed_session: Arc<ClosedSessionSnapshot>,
}

/// Completion could not be confirmed. No successful summary may be returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminateError {
    Failed,
}

impl TerminateError {
    /// The management-api-v1 §2.1 error code.
    pub fn code(self) -> &'static str {
        match self {
            Self::Failed => "terminate_failed",
        }
    }
}

impl fmt::Display for TerminateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("session cleanup could not be confirmed")
    }
}

impl std::error::Error for TerminateError {}

/// Sends commands to the relay owner and audits missing targets and joins.
/// The runtime alone audits the initiating request, after `session_closed`.
pub struct ManagementTerminateService {
    relay: Arc<dyn RelaySessionControl>,
    clock: Arc<dyn Clock>,
    events: Arc<dyn EventSink>,
}

impl ManagementTerminateService {
    /// Uses the same runtime, clock and event sink as relay processing.
    pub fn new(
        relay: Arc<dyn RelaySessionControl>,
        clock: Arc<dyn Clock>,
        events: Arc<dyn EventSink>,
    ) -> Self {
        Self {
            relay,
            clock,
            events,
        }
    }

    /// Accepts a validated request synchronously, before the adapter touches
    /// authentication idle time or pauses its connection's U timer.
    /// Consumer-valid but noncanonical IDs are missing, not malformed targets.
    pub fn request_terminate(
        &self,
        session_id: &str,
        account: AccountName,
        peer: PeerIp,
    ) -> Option<AcceptedTermination> {
        if let Ok(session_id) = SessionId::parse(session_id)
            && let Some(ticket) = self.relay.request_terminate(
                session_id.as_str(),
                account.clone(),
                peer,
                self.clock.as_ref(),
                self.events.as_ref(),
            )
        {
            return Some(AcceptedTermination {
                ticket,
                session_id,
                account,
                peer,
            });
        }
        self.events.record(&Event::AdminTerminate {
            admin_account: account,
            peer_ip: peer,
            session_id: None,
            result: AdminTerminateResult::NotFound,
        });
        None
    }

    /// Waits without a runtime or authentication lock. Already-completed and
    /// closed watch channels retain their last result; a missing result fails.
    /// Cancelling a join drops only this request's audit context.
    pub async fn complete(
        &self,
        accepted: AcceptedTermination,
    ) -> Result<TerminateView, TerminateError> {
        let AcceptedTermination {
            mut ticket,
            session_id,
            account,
            peer,
        } = accepted;
        let completion = loop {
            let completed = ticket.completion.borrow_and_update().clone();
            if let Some(result) = completed {
                break result;
            }
            if ticket.completion.changed().await.is_err() {
                break Err(());
            }
        };
        if !ticket.initiated {
            self.events.record(&Event::AdminTerminate {
                admin_account: account,
                peer_ip: peer,
                session_id: Some(session_id),
                result: if completion.is_ok() {
                    AdminTerminateResult::AlreadyClosed
                } else {
                    AdminTerminateResult::Failed
                },
            });
        }
        let closed_session = completion.map_err(|()| TerminateError::Failed)?;
        Ok(TerminateView {
            as_of: self.clock.wall(),
            result: if ticket.initiated {
                AdminTerminateResult::Terminated
            } else {
                AdminTerminateResult::AlreadyClosed
            },
            closed_session,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::future::{Future, poll_fn};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::Poll;
    use std::time::Duration;

    use vnc_relay_domain::handshake::Mode;
    use vnc_relay_domain::session::{CloseCause, Transport};

    use super::*;
    use crate::management::test_support::{FakeClock, FakeEvents, base_wall, peer};

    struct Control {
        initiated: bool,
        completion: watch::Receiver<Option<Result<Arc<ClosedSessionSnapshot>, ()>>>,
        requests: AtomicUsize,
    }

    impl RelaySessionControl for Control {
        fn request_terminate(
            &self,
            _session_id: &str,
            _account: AccountName,
            _peer: PeerIp,
            _clock: &dyn Clock,
            _events: &dyn EventSink,
        ) -> Option<TerminationTicket> {
            self.requests.fetch_add(1, Ordering::Relaxed);
            Some(TerminationTicket {
                initiated: self.initiated,
                completion: self.completion.clone(),
            })
        }
    }

    fn account() -> AccountName {
        AccountName::parse("alice").unwrap_or_else(|error| panic!("{error}"))
    }

    fn closed() -> Arc<ClosedSessionSnapshot> {
        Arc::new(ClosedSessionSnapshot {
            session_id: SessionId::from_block([1; 16]),
            mode: Mode::Mode2,
            viewer_transport: Transport::Tcp,
            target_transport: Transport::Tcp,
            started_at: base_wall(),
            ended_at: base_wall() + Duration::from_secs(1),
            duration: Duration::from_secs(1),
            bytes_viewer_to_target: 7,
            bytes_target_to_viewer: 11,
            close_cause: CloseCause::AdminTerminated,
            relay_id_ref: None,
            side: None,
        })
    }

    fn assert_audit(events: &FakeEvents, result: AdminTerminateResult, id: Option<SessionId>) {
        let recorded = events.all();
        assert_eq!(recorded.len(), 1);
        assert!(matches!(
            &recorded[0],
            Event::AdminTerminate { admin_account, peer_ip, session_id, result: actual }
                if admin_account == &account() && peer_ip == &peer(1)
                    && session_id == &id && actual == &result
        ));
    }

    #[tokio::test]
    async fn retained_completion_survives_sender_loss_without_duplicate_primary_audit() {
        let summary = closed();
        let (sender, completion) = watch::channel(Some(Ok(Arc::clone(&summary))));
        let control = Arc::new(Control {
            initiated: true,
            completion,
            requests: AtomicUsize::new(0),
        });
        let events = Arc::new(FakeEvents::default());
        let clock = Arc::new(FakeClock::default());
        let service = ManagementTerminateService::new(control, clock.clone(), events.clone());
        let accepted = service
            .request_terminate(summary.session_id.as_str(), account(), peer(1))
            .unwrap_or_else(|| panic!("accepted"));
        drop(sender);
        clock.set(Duration::from_secs(9));
        let view = service
            .complete(accepted)
            .await
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(Arc::ptr_eq(&view.closed_session, &summary));
        assert_eq!(view.as_of, base_wall() + Duration::from_secs(9));
        assert_eq!(view.result, AdminTerminateResult::Terminated);
        assert!(events.all().is_empty());
    }

    /// spec: management.md:MGT-011 partial
    #[tokio::test]
    async fn join_waits_then_audits_already_closed_with_the_same_summary() {
        let summary = closed();
        let (sender, completion) = watch::channel(None);
        let control = Arc::new(Control {
            initiated: false,
            completion,
            requests: AtomicUsize::new(0),
        });
        let events = Arc::new(FakeEvents::default());
        let service = ManagementTerminateService::new(
            control,
            Arc::new(FakeClock::default()),
            events.clone(),
        );
        let accepted = service
            .request_terminate(summary.session_id.as_str(), account(), peer(1))
            .unwrap_or_else(|| panic!("accepted"));
        let mut wait = std::pin::pin!(service.complete(accepted));
        poll_fn(|cx| {
            assert!(wait.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        assert!(events.all().is_empty());
        sender.send_replace(Some(Ok(Arc::clone(&summary))));
        let view = wait.await.unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(view.result, AdminTerminateResult::AlreadyClosed);
        assert!(Arc::ptr_eq(&view.closed_session, &summary));
        assert_audit(
            &events,
            AdminTerminateResult::AlreadyClosed,
            Some(summary.session_id.clone()),
        );
    }

    /// spec: management.md:MGT-012 partial
    #[tokio::test]
    async fn joined_failure_and_lost_sender_fail_without_a_success_summary() {
        for explicit_failure in [false, true] {
            let (sender, completion) = watch::channel(None);
            let control = Arc::new(Control {
                initiated: false,
                completion,
                requests: AtomicUsize::new(0),
            });
            let events = Arc::new(FakeEvents::default());
            let service = ManagementTerminateService::new(
                control,
                Arc::new(FakeClock::default()),
                events.clone(),
            );
            let id = SessionId::from_block([2; 16]);
            let accepted = service
                .request_terminate(id.as_str(), account(), peer(1))
                .unwrap_or_else(|| panic!("accepted"));
            if explicit_failure {
                sender.send_replace(Some(Err(())));
            }
            drop(sender);
            let error = service.complete(accepted).await.err();
            assert_eq!(error, Some(TerminateError::Failed));
            assert_eq!(TerminateError::Failed.code(), "terminate_failed");
            assert_audit(&events, AdminTerminateResult::Failed, Some(id));
        }
    }

    #[tokio::test]
    async fn failed_initiator_is_not_audited_twice() {
        let (_sender, completion) = watch::channel(Some(Err(())));
        let control = Arc::new(Control {
            initiated: true,
            completion,
            requests: AtomicUsize::new(0),
        });
        let events = Arc::new(FakeEvents::default());
        let service = ManagementTerminateService::new(
            control,
            Arc::new(FakeClock::default()),
            events.clone(),
        );
        let id = SessionId::from_block([3; 16]);
        let accepted = service
            .request_terminate(id.as_str(), account(), peer(1))
            .unwrap_or_else(|| panic!("accepted"));
        assert_eq!(
            service.complete(accepted).await.err(),
            Some(TerminateError::Failed)
        );
        assert!(events.all().is_empty());
    }

    #[test]
    fn noncanonical_consumer_id_is_missing_and_not_recorded_as_an_identifier() {
        let (_sender, completion) = watch::channel(None);
        let control = Arc::new(Control {
            initiated: false,
            completion,
            requests: AtomicUsize::new(0),
        });
        let events = Arc::new(FakeEvents::default());
        let service = ManagementTerminateService::new(
            control.clone(),
            Arc::new(FakeClock::default()),
            events.clone(),
        );
        assert!(
            service
                .request_terminate("unknown", account(), peer(1))
                .is_none()
        );
        assert_eq!(control.requests.load(Ordering::Relaxed), 0);
        assert_audit(&events, AdminTerminateResult::NotFound, None);
    }
}
