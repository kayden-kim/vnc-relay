//! Mode I policy, one absolute C, sequential candidates, and the second L place.

use std::future::{Future, poll_fn};
use std::net::{IpAddr, SocketAddr};
use std::pin::{Pin, pin};
use std::task::Poll;
use std::time::Duration;

use tokio::io::ReadBuf;
use tokio::sync::watch;
use vnc_relay_domain::handshake::{Mode, TargetAddress, TargetHost};

use super::{
    ActivePeer, OwnedConnection, RelayDispatchError, RelayEndpoint, RelayRuntime, RelayService,
    SessionPair, milliseconds,
};
use crate::events::{AbortReason, Event, RejectReason, Stage, TargetFailureReason};
use crate::ports::{
    Clock, RelayCloseAction, RelayLifecycle, RelayStreamError, TargetConnector, TargetError,
};

struct Connecting<'a, C>(&'a RelayRuntime<C>);

impl<C> Drop for Connecting<'_, C> {
    fn drop(&mut self) {
        self.0.lock().connecting -= 1;
    }
}

enum Failure {
    Rejected(RejectReason, Option<IpAddr>),
    Failed(TargetFailureReason, Option<IpAddr>),
    Shutdown,
    Aborted(AbortReason),
}

pub(super) async fn dispatch<C: TargetConnector>(
    service: &RelayService,
    runtime: &RelayRuntime<C>,
    mut viewer: OwnedConnection,
    address: TargetAddress,
) -> Result<(), RelayDispatchError> {
    let mut connect_timeout = Duration::ZERO;
    let mut policy = None;
    runtime.settings.with_current(&mut |settings| {
        connect_timeout = settings.timeouts.connect;
        policy = settings.mode1.allow_targets.clone();
    });
    let started = service.clock.monotonic();
    let deadline = started.saturating_add(connect_timeout);
    let context = viewer.context;
    let transport = viewer.stream.transport();
    let host = match &address.host {
        TargetHost::Ip(ip) => ip.to_string(),
        TargetHost::Name(name) => name.clone(),
    };
    let admitted = {
        let mut state = runtime.lock();
        if state.lifecycle == RelayLifecycle::Running {
            state.connecting += 1;
            true
        } else {
            false
        }
    };
    if !admitted {
        service
            .reject_connection(viewer, RelayCloseAction::ServerShutdown, runtime)
            .await;
        service.aborted(
            context,
            Stage::TargetConnect,
            Mode::Mode1,
            AbortReason::ServerShutdown,
        );
        return Err(RelayDispatchError::Aborted(AbortReason::ServerShutdown));
    }
    let connecting = Connecting(runtime);
    // Kw also runs during DNS/connect. Payload read-ahead uses only the unused
    // part of the same B and is handed to the active session without copying.
    let mut filled = viewer.trailing.len();
    let websocket = viewer.stream.transport().is_websocket();
    if websocket {
        viewer.trailing.resize(context.buffer_bytes, 0);
    }
    let result = {
        let lifecycle = runtime.lifecycle();
        let mut connecting = pin!(connect(
            service,
            runtime,
            &address,
            deadline,
            policy.as_ref()
        ));
        poll_fn(|cx| {
            if let Err(failure) = check(service.clock.as_ref(), deadline, &lifecycle) {
                return Poll::Ready(Err(failure));
            }
            if websocket {
                if let Poll::Ready(Err(error)) =
                    Pin::new(viewer.stream.as_mut()).poll_maintenance(cx)
                {
                    return Poll::Ready(Err(Failure::Aborted(stream_abort(&error))));
                }
                if filled < viewer.trailing.len() {
                    let mut buffer = ReadBuf::new(&mut viewer.trailing[filled..]);
                    match Pin::new(viewer.stream.as_mut()).poll_read(cx, &mut buffer) {
                        Poll::Ready(Err(error)) => {
                            return Poll::Ready(Err(Failure::Aborted(stream_abort(&error))));
                        }
                        Poll::Ready(Ok(())) if buffer.filled().is_empty() => {
                            return Poll::Ready(Err(Failure::Aborted(AbortReason::PeerEof)));
                        }
                        Poll::Ready(Ok(())) => {
                            filled += buffer.filled().len();
                            cx.waker().wake_by_ref();
                        }
                        Poll::Pending => {}
                    }
                }
            }
            connecting.as_mut().poll(cx)
        })
        .await
    };
    viewer.trailing.truncate(filled);
    drop(connecting);
    match result {
        Ok((stream, permit, target)) => {
            let other = RelayEndpoint {
                stream: Box::new(stream),
                trailing: Vec::new(),
                permit,
                buffer_bytes: context.buffer_bytes,
            };
            service
                .run_pair(
                    runtime,
                    SessionPair {
                        viewer,
                        other,
                        peer: ActivePeer::Target(target),
                        target_host: Some(host),
                        relay: None,
                    },
                )
                .await
        }
        Err(failure) => {
            let elapsed_ms = milliseconds(service.clock.monotonic().saturating_sub(started));
            let action = match &failure {
                Failure::Rejected(RejectReason::TargetNotAllowed, _) => {
                    RelayCloseAction::TargetNotAllowed
                }
                Failure::Rejected(RejectReason::RelayConnectionLimit { .. }, _) => {
                    RelayCloseAction::TargetConnectionLimit
                }
                Failure::Rejected(_, _) => RelayCloseAction::TargetConnectionLimit,
                Failure::Failed(_, _) => RelayCloseAction::TargetConnectFailed,
                Failure::Shutdown => RelayCloseAction::ServerShutdown,
                Failure::Aborted(AbortReason::DeadConnection) => RelayCloseAction::DeadConnection,
                Failure::Aborted(AbortReason::PeerEof) => RelayCloseAction::PeerEof,
                Failure::Aborted(AbortReason::ConnectionError(kind)) => {
                    RelayCloseAction::StreamError(*kind)
                }
                Failure::Aborted(_) => RelayCloseAction::ServerShutdown,
            };
            service.reject_connection(viewer, action, runtime).await;
            match failure {
                Failure::Rejected(reason, target_ip) => {
                    service.events.record(&Event::TargetRejected {
                        conn_id: context.conn_id,
                        entry_point: context.entry_point,
                        peer_ip: context.peer_ip,
                        forwarded_ip: context.forwarded_ip,
                        reason,
                        target_host: host,
                        target_port: address.port,
                        target_ip,
                    });
                    Err(RelayDispatchError::Rejected(reason))
                }
                Failure::Failed(reason, target_ip) => {
                    service.events.record(&Event::TargetConnectFailed {
                        conn_id: context.conn_id,
                        entry_point: context.entry_point,
                        viewer_transport: transport,
                        peer_ip: context.peer_ip,
                        forwarded_ip: context.forwarded_ip,
                        target_host: host,
                        target_port: address.port,
                        target_ip,
                        elapsed_ms,
                        reason,
                    });
                    Err(RelayDispatchError::Target(reason))
                }
                Failure::Shutdown => {
                    service.aborted(
                        context,
                        Stage::TargetConnect,
                        Mode::Mode1,
                        AbortReason::ServerShutdown,
                    );
                    Err(RelayDispatchError::Aborted(AbortReason::ServerShutdown))
                }
                Failure::Aborted(reason) => {
                    service.aborted(context, Stage::TargetConnect, Mode::Mode1, reason);
                    Err(RelayDispatchError::Aborted(reason))
                }
            }
        }
    }
}

fn stream_abort(error: &std::io::Error) -> AbortReason {
    if RelayStreamError::is_dead_connection(error) {
        AbortReason::DeadConnection
    } else {
        AbortReason::ConnectionError(RelayStreamError::classify(error))
    }
}

async fn connect<C: TargetConnector>(
    service: &RelayService,
    runtime: &RelayRuntime<C>,
    address: &TargetAddress,
    deadline: Duration,
    policy: Option<&vnc_relay_domain::target_policy::TargetPolicy>,
) -> Result<
    (
        C::Stream,
        vnc_relay_domain::budget::BudgetPermit,
        SocketAddr,
    ),
    Failure,
> {
    let resolved;
    let candidates: &[IpAddr] = match &address.host {
        TargetHost::Ip(ip) => std::slice::from_ref(ip),
        TargetHost::Name(name) => {
            if !policy.is_some_and(|policy| policy.permits_name(name, address.port)) {
                return Err(Failure::Rejected(RejectReason::TargetNotAllowed, None));
            }
            resolved = match bounded(
                runtime.connector.resolve(name),
                service.clock.as_ref(),
                deadline,
                runtime.lifecycle(),
            )
            .await
            {
                Ok(Ok(ips)) if !ips.is_empty() => ips,
                Ok(_) => return Err(Failure::Failed(TargetFailureReason::ResolveFailed, None)),
                Err(failure) => return Err(failure),
            };
            &resolved
        }
    };
    let mut permit = None;
    let mut last_ip = None;
    let mut last_checked = None;
    let mut all_refused = true;
    for ip in candidates {
        let ip = ip.to_canonical();
        last_checked = Some(ip);
        if !policy.is_some_and(|policy| policy.permits_ip(ip, address.port)) {
            continue;
        }
        check(service.clock.as_ref(), deadline, &runtime.lifecycle())?;
        if permit.is_none() {
            // The runtime source calls its reader once; an unread limit stays 0 and rejects.
            let mut limit = 0;
            let mut admission = None;
            runtime.settings.with_current(&mut |settings| {
                limit = settings.limits.relay_connections;
                admission = runtime.relay_budget.try_acquire(limit);
            });
            permit = Some(admission.ok_or(Failure::Rejected(
                RejectReason::RelayConnectionLimit { limit },
                None,
            ))?);
        }
        last_ip = Some(ip);
        let target = SocketAddr::new(ip, address.port);
        match bounded(
            runtime.connector.connect(target),
            service.clock.as_ref(),
            deadline,
            runtime.lifecycle(),
        )
        .await
        {
            Ok(Ok(stream)) => {
                // bounded checks C and shutdown after the connector resolves.
                // The target is not activated until run_pair's shared-lock gate.
                if let Some(permit) = permit.take() {
                    return Ok((stream, permit, target));
                }
                return Err(Failure::Failed(TargetFailureReason::ConnectError, last_ip));
            }
            Ok(Err(TargetError::KeepaliveFailed)) => {
                return Err(Failure::Failed(TargetFailureReason::ConnectError, last_ip));
            }
            Ok(Err(error)) => all_refused &= error == TargetError::ConnectRefused,
            Err(Failure::Failed(reason, _)) => return Err(Failure::Failed(reason, last_ip)),
            Err(failure) => return Err(failure),
        }
    }
    if last_ip.is_none() {
        Err(Failure::Rejected(
            RejectReason::TargetNotAllowed,
            last_checked,
        ))
    } else {
        Err(Failure::Failed(
            if all_refused {
                TargetFailureReason::ConnectRefused
            } else {
                TargetFailureReason::ConnectError
            },
            last_ip,
        ))
    }
}

fn check(
    clock: &dyn Clock,
    deadline: Duration,
    lifecycle: &watch::Receiver<RelayLifecycle>,
) -> Result<(), Failure> {
    if *lifecycle.borrow() != RelayLifecycle::Running {
        Err(Failure::Shutdown)
    } else if clock.monotonic() >= deadline {
        Err(Failure::Failed(TargetFailureReason::ConnectTimeout, None))
    } else {
        Ok(())
    }
}

async fn bounded<T>(
    future: impl Future<Output = T>,
    clock: &dyn Clock,
    deadline: Duration,
    mut lifecycle: watch::Receiver<RelayLifecycle>,
) -> Result<T, Failure> {
    let mut stopped = pin!(lifecycle.wait_for(|phase| *phase != RelayLifecycle::Running));
    let mut timer = pin!(tokio::time::sleep(
        deadline.saturating_sub(clock.monotonic())
    ));
    let mut future = pin!(future);
    poll_fn(|cx| {
        if stopped.as_mut().poll(cx).is_ready() {
            return Poll::Ready(Err(Failure::Shutdown));
        }
        if clock.monotonic() >= deadline || timer.as_mut().poll(cx).is_ready() {
            return Poll::Ready(Err(Failure::Failed(
                TargetFailureReason::ConnectTimeout,
                None,
            )));
        }
        match future.as_mut().poll(cx) {
            Poll::Ready(value) => {
                if stopped.as_mut().poll(cx).is_ready() {
                    drop(value);
                    Poll::Ready(Err(Failure::Shutdown))
                } else if clock.monotonic() >= deadline || timer.as_mut().poll(cx).is_ready() {
                    drop(value);
                    Poll::Ready(Err(Failure::Failed(
                        TargetFailureReason::ConnectTimeout,
                        None,
                    )))
                } else {
                    Poll::Ready(Ok(value))
                }
            }
            Poll::Pending => Poll::Pending,
        }
    })
    .await
}
