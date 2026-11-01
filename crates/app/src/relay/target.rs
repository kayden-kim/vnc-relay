//! Mode I policy, one absolute C, sequential candidates, and the second L place.

use std::future::{Future, poll_fn};
use std::net::{IpAddr, SocketAddr};
use std::pin::pin;
use std::task::Poll;
use std::time::Duration;

use tokio::sync::watch;
use vnc_relay_domain::handshake::{Mode, TargetAddress, TargetHost};

use super::{
    OwnedConnection, RelayDispatchError, RelayEndpoint, RelayRuntime, RelayService, SessionPair,
    milliseconds,
};
use crate::events::{AbortReason, Event, RejectReason, Stage, TargetFailureReason};
use crate::ports::{Clock, RelayCloseAction, RelayLifecycle, TargetConnector, TargetError};

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
}

pub(super) async fn dispatch<C: TargetConnector>(
    service: &RelayService,
    runtime: &RelayRuntime<C>,
    viewer: OwnedConnection,
    address: TargetAddress,
) -> Result<(), RelayDispatchError> {
    let started = service.clock.monotonic();
    let deadline = started.saturating_add(runtime.config.connect_timeout);
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
    let result = connect(service, runtime, &address, deadline).await;
    drop(connecting);
    match result {
        Ok((stream, permit, target)) => {
            let other = RelayEndpoint {
                stream: Box::new(stream),
                trailing: Vec::new(),
                permit,
            };
            service
                .run_pair(
                    runtime,
                    SessionPair {
                        viewer,
                        other,
                        server_context: None,
                        target: Some(target),
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
                Failure::Rejected(_, _) => RelayCloseAction::Reject,
                Failure::Failed(_, _) => RelayCloseAction::TargetConnectFailed,
                Failure::Shutdown => RelayCloseAction::ServerShutdown,
            };
            service.reject_connection(viewer, action, runtime).await;
            match failure {
                Failure::Rejected(reason, target_ip) => {
                    service.events.record(&Event::TargetRejected {
                        conn_id: context.conn_id,
                        entry_point: context.entry_point,
                        peer_ip: context.peer_ip,
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
            }
        }
    }
}

async fn connect<C: TargetConnector>(
    service: &RelayService,
    runtime: &RelayRuntime<C>,
    address: &TargetAddress,
    deadline: Duration,
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
            if !runtime
                .config
                .target_policy
                .permits_name(name, address.port)
            {
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
        if !runtime.config.target_policy.permits_ip(ip, address.port) {
            continue;
        }
        check(service.clock.as_ref(), deadline, &runtime.lifecycle())?;
        if permit.is_none() {
            permit = Some(runtime.relay_budget.try_acquire().ok_or(Failure::Rejected(
                RejectReason::RelayConnectionLimit {
                    limit: runtime.relay_budget.limit(),
                },
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
