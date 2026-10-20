//! TCP listeners. Startup binding: docs/architectures/serve-startup.md.

use std::io::{self, ErrorKind};

use tokio::net::TcpListener;
use vnc_relay_app::settings::ListenerPlan;

/// A listener bound for its plan.
#[derive(Debug)]
pub struct BoundListener {
    pub plan: ListenerPlan,
    pub listener: TcpListener,
}

/// A plan that could not be bound. `message` names the address and the cause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindFailure {
    pub plan: ListenerPlan,
    pub message: String,
}

/// Binds every plan in order. After a failure it still tries the rest so
/// that every failure is reported (cli-v1 §1.5). When any plan fails, the
/// listeners already bound are closed before returning.
pub async fn bind(plans: &[ListenerPlan]) -> Result<Vec<BoundListener>, Vec<BindFailure>> {
    let mut bound = Vec::with_capacity(plans.len());
    let mut failures = Vec::new();
    for plan in plans {
        match TcpListener::bind(plan.addr).await {
            Ok(listener) => bound.push(BoundListener {
                plan: *plan,
                listener,
            }),
            Err(error) => failures.push(BindFailure {
                plan: *plan,
                message: bind_message(plan, &error),
            }),
        }
    }
    if failures.is_empty() {
        Ok(bound)
    } else {
        drop(bound);
        Err(failures)
    }
}

fn bind_message(plan: &ListenerPlan, error: &io::Error) -> String {
    let addr = plan.addr;
    match error.kind() {
        ErrorKind::AddrInUse => format!("{addr}에 바인드할 수 없다. 이미 사용 중이다."),
        ErrorKind::AddrNotAvailable => {
            format!("{addr}에 바인드할 수 없다. 이 호스트의 주소가 아니다.")
        }
        ErrorKind::PermissionDenied => format!("{addr}에 바인드할 수 없다. 권한이 없다."),
        _ => format!("{addr}에 바인드할 수 없다({error})."),
    }
}
