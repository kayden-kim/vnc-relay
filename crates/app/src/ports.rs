//! Ports: traits for outside functions that app use cases call. adapters
//! implements them (docs/architectures/code-map.md, layer "app").

use crate::events::Event;

/// Records operation log events (ops-info-v1). Recording does not fail for
/// the caller: a sink that cannot write drops the line (ops-info-v1 §1.1),
/// so a log failure does not change the flow of a use case.
pub trait EventSink: Send + Sync {
    fn record(&self, event: &Event);
}
