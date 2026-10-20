//! Connection ID (`conn_id`, ops-info-v1 §1.4).
//! Design: docs/architectures/tcp-entry-point.md.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

/// Sequence number of the next connection ID in this process.
static NEXT: AtomicU64 = AtomicU64::new(1);

/// ID that a relay listener gives a connection when it accepts it. It is
/// unique within the process and differs from the session ID. `Display`
/// writes `c` and the decimal sequence number, such as `c1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnId(u64);

impl ConnId {
    /// The next ID of this process. Every entry point shares one sequence,
    /// so no two connections get the same ID before the process ends.
    pub fn next() -> Self {
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

impl fmt::Display for ConnId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "c{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hands_out_distinct_ids_written_with_a_c_prefix() {
        let first = ConnId::next();
        let second = ConnId::next();
        assert_ne!(first, second);
        for id in [first, second] {
            let text = id.to_string();
            let digits = text
                .strip_prefix('c')
                .unwrap_or_else(|| panic!("no prefix: {text}"));
            assert!(
                !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()),
                "{text}"
            );
        }
        assert_eq!(ConnId(7).to_string(), "c7");
    }
}
