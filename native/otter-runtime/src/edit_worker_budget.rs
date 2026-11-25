//! Bounds live and retiring document workers together. A permit belongs to
//! the join owner, not the compute body, so uncollected work remains charged.
use std::sync::atomic::{AtomicUsize, Ordering};

// Match the previous maximum of six job slots per workspace unit.
const LIMIT: usize = 6 * crate::workspace::MAX_UNITS;
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
static OUTPUT: AtomicUsize = AtomicUsize::new(0);
static OPEN: AtomicUsize = AtomicUsize::new(0);

pub struct Permit<'a>(&'a AtomicUsize);

impl Permit<'static> {
    pub fn acquire() -> Option<Self> {
        reserve(&ACTIVE, LIMIT)
    }

    /// Separate from raster slots so long file writes cannot starve edits.
    pub fn acquire_output() -> Option<Self> {
        reserve(&OUTPUT, 3 * crate::workspace::MAX_UNITS)
    }

    /// One decode slot per unit, separate from raster edits and outputs.
    pub fn acquire_open() -> Option<Self> {
        reserve(&OPEN, crate::workspace::MAX_UNITS)
    }
}

fn reserve(active: &AtomicUsize, limit: usize) -> Option<Permit<'_>> {
    active
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
            (n < limit).then_some(n + 1)
        })
        .ok()?;
    Some(Permit(active))
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Holds a real test worker in flight until the application has retired its window.
#[cfg(any(test, feature = "test-support"))]
pub mod test_gate {
    use std::{
        sync::{
            atomic::{AtomicBool, Ordering},
            mpsc::{self, Receiver, Sender},
        },
        time::Duration,
    };

    pub struct Gate {
        release: Receiver<()>,
        observed: Sender<bool>,
    }

    impl Gate {
        pub fn pair() -> (Self, Sender<()>, Receiver<bool>) {
            let (release, wait) = mpsc::channel();
            let (observed, result) = mpsc::channel();
            (
                Self {
                    release: wait,
                    observed,
                },
                release,
                result,
            )
        }

        pub fn wait(self, cancel: &AtomicBool) {
            // Do not strand a join if the host assertion fails before release.
            if self.release.recv_timeout(Duration::from_secs(8)).is_ok() {
                let _ = self.observed.send(cancel.load(Ordering::Acquire));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saturated_budget_reopens_only_after_owner_releases_slot() {
        let active = AtomicUsize::new(0);
        let old = reserve(&active, 2).unwrap();
        let current = reserve(&active, 2).unwrap();
        assert!(reserve(&active, 2).is_none());
        drop(old);
        let replacement = reserve(&active, 2).unwrap();
        assert!(reserve(&active, 2).is_none());
        drop((current, replacement));
        assert_eq!(active.load(Ordering::Acquire), 0);
    }
}
