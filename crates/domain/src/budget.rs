//! Connection budgets: the relay connection limit L, the management and
//! status connection limit Lm, and the waiting limit Lw (operations spec §2).
//! Design: docs/architectures/connection-limits.md.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

/// Counts held places independently of the current configured limit.
/// Callers supply the limit from their runtime snapshot for each admission.
#[derive(Debug)]
pub struct ConnectionBudget {
    used: AtomicU32,
}

impl ConnectionBudget {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            used: AtomicU32::new(0),
        })
    }

    /// The current number of held permits.
    pub fn current(&self) -> u32 {
        self.used.load(Ordering::Acquire)
    }

    /// Takes one place, or `None` when all `limit` places are held. Takes
    /// never exceed the limit, even when tasks take places at once.
    pub fn try_acquire(self: &Arc<Self>, limit: u32) -> Option<BudgetPermit> {
        self.used
            .try_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                (used < limit).then(|| used + 1)
            })
            .ok()?;
        Some(BudgetPermit {
            budget: Arc::clone(self),
        })
    }
}

/// One held place. Dropping it gives the place back.
#[derive(Debug)]
#[must_use = "dropping the permit gives the place back at once"]
pub struct BudgetPermit {
    budget: Arc<ConnectionBudget>,
}

impl Drop for BudgetPermit {
    fn drop(&mut self) {
        self.budget.used.fetch_sub(1, Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    /// spec: operations.md:OPS-051 partial
    /// spec: operations.md:OPS-052 partial
    /// spec: operations.md:OPS-053 partial
    #[test]
    fn lower_limit_does_not_revoke_permits_and_recovers_after_release() {
        let budget = ConnectionBudget::new();
        let first = budget.try_acquire(3).expect("first");
        let second = budget.try_acquire(3).expect("second");
        let third = budget.try_acquire(3).expect("third");
        assert!(budget.try_acquire(1).is_none());
        assert_eq!(budget.current(), 3);
        drop(first);
        drop(second);
        assert!(budget.try_acquire(1).is_none());
        drop(third);
        let held = budget.try_acquire(1).expect("released below new limit");
        assert!(budget.try_acquire(1).is_none());
        let raised = budget.try_acquire(2).expect("raised limit");
        assert_eq!(budget.current(), 2);
        drop((held, raised));
    }

    /// spec: operations.md:OPS-004 partial
    #[test]
    fn holds_at_most_limit_places_and_takes_a_released_one_back() {
        let budget = ConnectionBudget::new();
        let first = budget.try_acquire(2);
        let second = budget.try_acquire(2);
        assert!(first.is_some() && second.is_some());
        assert!(budget.try_acquire(2).is_none());
        assert!(budget.try_acquire(2).is_none());

        drop(first);
        let third = budget.try_acquire(2);
        assert!(third.is_some());
        assert!(budget.try_acquire(2).is_none());
    }

    /// spec: operations.md:OPS-033 partial
    #[test]
    fn an_exhausted_budget_does_not_block_another() {
        let relay = ConnectionBudget::new();
        let admin = ConnectionBudget::new();
        let held: Vec<_> = (0..2).filter_map(|_| relay.try_acquire(2)).collect();
        assert_eq!(held.len(), 2);
        assert!(relay.try_acquire(2).is_none());

        let admin_place = admin.try_acquire(1);
        assert!(admin_place.is_some());
        assert!(admin.try_acquire(1).is_none());
        // Releasing an admin place does not free a relay place.
        drop(admin_place);
        assert!(relay.try_acquire(2).is_none());
    }

    #[test]
    fn concurrent_takes_never_exceed_the_limit() {
        let budget = ConnectionBudget::new();
        let workers: Vec<_> = (0..8)
            .map(|_| {
                let budget = Arc::clone(&budget);
                thread::spawn(move || {
                    (0..1000)
                        .filter_map(|_| budget.try_acquire(100))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        let held: Vec<BudgetPermit> = workers
            .into_iter()
            .flat_map(|worker| worker.join().unwrap_or_else(|_| panic!("worker panicked")))
            .collect();
        assert_eq!(held.len(), 100);
        drop(held);
        let again: Vec<_> = (0..100).filter_map(|_| budget.try_acquire(100)).collect();
        assert_eq!(again.len(), 100);
    }

    /// spec: management.md:MGT-026 partial
    /// spec: operations.md:OPS-033 partial
    #[test]
    fn reports_current_held_places() {
        let budget = ConnectionBudget::new();
        assert_eq!(budget.current(), 0);

        let p1 = budget.try_acquire(3);
        assert_eq!(budget.current(), 1);

        let p2 = budget.try_acquire(3);
        assert_eq!(budget.current(), 2);

        let p3 = budget.try_acquire(3);
        assert_eq!(budget.current(), 3);

        drop(p2);
        assert_eq!(budget.current(), 2);

        drop(p1);
        drop(p3);
        assert_eq!(budget.current(), 0);
    }
}
