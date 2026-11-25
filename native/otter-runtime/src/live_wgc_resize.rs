//! Window-source resize admission and frame-pool budget transaction; do not change fixed-region target bounds.
use super::{pool_bytes, same_size, Error, SizeInt32};
use crate::byte_budget::Lease;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Plan {
    Same,
    Suspend,
    Recreate,
}
pub(super) fn plan(allowed: bool, current: SizeInt32, next: SizeInt32) -> Result<Plan, Error> {
    if same_size(current, next) {
        return Ok(Plan::Same);
    }
    if !allowed {
        return Err(Error::Resized);
    }
    if next.Width < 0 || next.Height < 0 {
        return Err(Error::Dimensions);
    }
    // Zero-content events such as minimization only return the frame and await the next event; no zero-sized pools or active polling.
    if next.Width == 0 || next.Height == 0 {
        return Ok(Plan::Suspend);
    }
    pool_bytes(next)?;
    Ok(Plan::Recreate)
}
pub(super) fn recreate(
    size: &mut SizeInt32,
    budget: &mut Option<Lease>,
    next: SizeInt32,
    reserve: impl FnOnce(usize) -> Option<Lease>,
    apply: impl FnOnce() -> Result<(), Error>,
) -> Result<(), Error> {
    if budget.is_none() {
        return Err(Error::Cancelled);
    }
    // Retain the old pool lease until reconstruction succeeds, conservatively admitting the old-plus-new peak rather than treating transient driver usage as returned.
    let candidate = reserve(pool_bytes(next)?).ok_or(Error::Budget)?;
    apply()?;
    *size = next;
    *budget = Some(candidate);
    Ok(())
}

#[cfg(test)]
#[path = "live_wgc_resize_tests.rs"]
mod tests;
