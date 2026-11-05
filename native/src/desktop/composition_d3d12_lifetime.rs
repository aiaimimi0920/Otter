//! 两后端共享的窗口图保留预算；文件名保留历史路径，逻辑 owner 为 desktop::composition_lifetime。
//! 两种后端合计 MAX_UNITS 个窗口图；另限制 D3D12 两代设备。未确认关闭不归还配额。
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
        // 测试模型没有 GPU 在途，允许明确确认后回收，避免污染测试计数。
        drop(std::mem::ManuallyDrop::into_inner(retained));
        assert_eq!(COUNT.load(Ordering::Acquire), 0);
    }
}
