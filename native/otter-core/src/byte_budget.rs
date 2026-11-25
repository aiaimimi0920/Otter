//! Cross-thread byte leases; failed reservations preserve counters, and final resource-owner release returns budget/wakes waiters as needed.
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Condvar, Mutex,
    },
    time::{Duration, Instant},
};

#[derive(Debug)]
pub struct Budget {
    limit: usize,
    used: AtomicUsize,
    waiters: AtomicUsize,
    wake_lock: Mutex<()>,
    wake: Condvar,
}
impl Budget {
    pub fn new(limit: usize) -> Arc<Self> {
        Arc::new(Self {
            limit,
            used: AtomicUsize::new(0),
            waiters: AtomicUsize::new(0),
            wake_lock: Mutex::new(()),
            wake: Condvar::new(),
        })
    }
    fn claim(&self, bytes: usize) -> bool {
        self.used
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |used| {
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
    /// Wait at most `timeout` for capacity; without waiters, `Drop` retains its purely atomic fast path.
    pub fn wait_for(self: &Arc<Self>, bytes: usize, timeout: Duration) -> Option<Lease> {
        let started = Instant::now();
        if let Some(lease) = self.reserve(bytes) {
            return Some(lease);
        }
        if bytes > self.limit || timeout.is_zero() {
            return None;
        }
        struct Waiter<'a>(&'a AtomicUsize);
        impl Drop for Waiter<'_> {
            fn drop(&mut self) {
                self.0.fetch_sub(1, Ordering::SeqCst);
            }
        }
        // Share SC total ordering with failed claim reads, Drop returns, and waiter reads:
        // either registration observes capacity or the returner observes waiters/notifies under lock; neither can miss the other.
        self.waiters.fetch_add(1, Ordering::SeqCst);
        let _waiter = Waiter(&self.waiters);
        let mut guard = self.wake_lock.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            let remaining = timeout.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                return None;
            }
            if let Some(lease) = self.reserve(bytes) {
                return Some(lease);
            }
            let (next, _) = self
                .wake
                .wait_timeout(guard, remaining)
                .unwrap_or_else(|e| e.into_inner());
            guard = next;
        }
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
        self.budget.used.fetch_sub(self.bytes, Ordering::SeqCst);
        if self.budget.waiters.load(Ordering::SeqCst) != 0 {
            let _guard = self
                .budget
                .wake_lock
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            self.budget.wake.notify_all();
        }
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
    #[test]
    fn wait_for_wakes_promptly_when_lease_drops() {
        let budget = Budget::new(32);
        let held = budget.reserve(32).unwrap();
        let waiter_budget = budget.clone();
        let start = Instant::now();
        let release = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(5));
            drop(held);
        });
        let acquired = waiter_budget
            .wait_for(32, Duration::from_secs(2))
            .expect("woken on drop");
        assert!(start.elapsed() < Duration::from_millis(500));
        release.join().unwrap();
        drop(acquired);
        assert_eq!(budget.used(), 0);
    }
    #[test]
    fn release_racing_waiter_registration_does_not_lose_capacity() {
        let budget = Budget::new(1);
        for _ in 0..64 {
            let held = budget.reserve(1).unwrap();
            let barrier = Arc::new(std::sync::Barrier::new(2));
            let released = barrier.clone();
            let release = std::thread::spawn(move || {
                released.wait();
                drop(held);
            });
            barrier.wait();
            let acquired = budget
                .wait_for(1, Duration::from_secs(1))
                .expect("release racing registration must not be lost");
            release.join().unwrap();
            drop(acquired);
        }
        assert_eq!(budget.used(), 0);
        assert_eq!(budget.waiters.load(Ordering::SeqCst), 0);
    }
    #[test]
    fn wait_for_checks_expiry_after_acquiring_wake_lock() {
        let budget = Budget::new(1);
        let held = budget.reserve(1).unwrap();
        let guard = budget.wake_lock.lock().unwrap();
        let waiting = budget.clone();
        let waiter = std::thread::spawn(move || waiting.wait_for(1, Duration::from_millis(20)));
        let limit = Instant::now() + Duration::from_secs(2);
        while budget.waiters.load(Ordering::SeqCst) == 0 && Instant::now() < limit {
            std::thread::yield_now();
        }
        let registered = budget.waiters.load(Ordering::SeqCst) != 0;
        let release = std::thread::spawn(move || drop(held));
        while budget.used() != 0 && Instant::now() < limit {
            std::thread::yield_now();
        }
        std::thread::sleep(Duration::from_millis(30));
        drop(guard);
        let result = waiter.join().unwrap();
        release.join().unwrap();
        assert!(registered, "waiter did not register before test deadline");
        assert!(
            result.is_none(),
            "expired wait must not acquire returned capacity"
        );
        assert_eq!(budget.used(), 0);
    }
}
