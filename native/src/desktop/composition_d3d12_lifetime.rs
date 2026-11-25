//! Window-graph retention budget shared by both backends; the filename preserves its historical path, while the logical owner is desktop::composition_lifetime.
//! Both backends together allow MAX_UNITS window graphs, with D3D12 additionally limited to two device generations. Unconfirmed closure returns no quota.
use std::sync::atomic::{AtomicUsize, Ordering};
use windows::{
    core::{Error, Result},
    Win32::Foundation::E_FAIL,
};
static CANDIDATES: AtomicUsize = AtomicUsize::new(0);
static GENERATIONS: AtomicUsize = AtomicUsize::new(0);
const LIMIT: usize = 2;
pub(super) struct Lease(&'static AtomicUsize);
impl Lease {
    #[cfg(test)]
    pub(super) fn used_for_test() -> usize {
        CANDIDATES.load(Ordering::Acquire)
    }
    pub fn new() -> Result<Self> {
        Self::reserve(&CANDIDATES, crate::workspace::MAX_UNITS)
    }
    pub fn generation() -> Result<Self> {
        Self::reserve(&GENERATIONS, LIMIT)
    }
    fn reserve(counter: &'static AtomicUsize, limit: usize) -> Result<Self> {
        counter
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < limit).then_some(n + 1)
            })
            .map_err(|_| Error::new(E_FAIL, "Composition ownership budget exhausted"))?;
        Ok(Self(counter))
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unconfirmed_owner_keeps_budget_and_normal_drop_returns_it() {
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        let retained = std::mem::ManuallyDrop::new(Lease::reserve(&COUNT, 2).unwrap());
        let normal = Lease::reserve(&COUNT, 2).unwrap();
        assert!(Lease::reserve(&COUNT, 2).is_err());
        drop(normal);
        let replacement = Lease::reserve(&COUNT, 2).unwrap();
        assert!(Lease::reserve(&COUNT, 2).is_err());
        drop(replacement);
        assert_eq!(COUNT.load(Ordering::Acquire), 1);
        // The test model has no in-flight GPU work; allow reclamation after explicit confirmation to avoid contaminating test counters.
        drop(std::mem::ManuallyDrop::into_inner(retained));
        assert_eq!(COUNT.load(Ordering::Acquire), 0);
    }
}
