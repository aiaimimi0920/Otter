//! 窗口源尺寸变化准入与帧池预算事务；不改变固定区域的目标边界。
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
    // 最小化等零内容事件只归还帧，等待下一次事件；不创建零尺寸池或主动轮询。
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
    // 保留旧池租约直到重建成功，按旧+新峰值保守准入，不把驱动瞬时占用当作已归还。
    let candidate = reserve(pool_bytes(next)?).ok_or(Error::Budget)?;
    apply()?;
    *size = next;
    *budget = Some(candidate);
    Ok(())
}

#[cfg(test)]
#[path = "live_wgc_resize_tests.rs"]
mod tests;
