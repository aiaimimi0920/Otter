//! 跨线程字节租约；预留失败不改变计数，最后一个资源 owner 释放时归还预算。
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[derive(Debug)]
pub struct Budget {
    limit: usize,
    used: AtomicUsize,
}
impl Budget {
    pub fn new(limit: usize) -> Arc<Self> {
        Arc::new(Self {
            limit,
            used: AtomicUsize::new(0),
        })
    }
    fn claim(&self, bytes: usize) -> bool {
        self.used
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                used.checked_add(bytes).filter(|total| *total <= self.limit)
            })
            .is_ok()
    }
    pub fn reserve(self: &Arc<Self>, bytes: usize) -> Option<Lease> {
        self.claim(bytes).then(|| Lease {
            budget: self.clone(),
            bytes,
        })
    }
    pub fn used(&self) -> usize {
        self.used.load(Ordering::Acquire)
    }
}

#[derive(Debug)]
pub struct Lease {
    budget: Arc<Budget>,
    bytes: usize,
}
impl Lease {
    pub fn grow_to(&mut self, bytes: usize) -> bool {
        if bytes <= self.bytes {
            return true;
        }
        if !self.budget.claim(bytes - self.bytes) {
            return false;
        }
        self.bytes = bytes;
        true
    }
    pub fn bytes(&self) -> usize {
        self.bytes
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        self.budget.used.fetch_sub(self.bytes, Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failure_growth_and_drop_keep_exact_accounting() {
        let budget = Budget::new(100);
        let mut a = budget.reserve(60).unwrap();
        assert!(budget.reserve(41).is_none());
        assert!(!a.grow_to(101));
        assert_eq!(budget.used(), 60);
        assert!(a.grow_to(80));
        assert_eq!(budget.used(), 80);
        let b = budget.reserve(20).unwrap();
        assert!(budget.reserve(usize::MAX).is_none());
        drop(a);
        assert_eq!(budget.used(), 20);
        drop(b);
        assert_eq!(budget.used(), 0);
    }
    #[test]
    fn concurrent_admission_cannot_exceed_limit() {
        let budget = Budget::new(64);
        let barrier = Arc::new(std::sync::Barrier::new(9));
        let workers: Vec<_> = (0..8)
            .map(|_| {
                let budget = budget.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let lease = budget.reserve(16);
                    barrier.wait();
                    barrier.wait();
                    lease.is_some()
                })
            })
            .collect();
        barrier.wait();
        assert_eq!(budget.used(), 64);
        barrier.wait();
        let admitted = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .filter(|admitted| *admitted)
            .count();
        assert_eq!(admitted, 4);
        assert_eq!(budget.used(), 0);
    }
}
