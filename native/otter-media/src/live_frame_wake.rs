//! 消费者通知合并；读取时在源锁内确认，防止读帧与重新允许通知之间丢失唤醒。
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
pub(super) struct Wake {
    state: AtomicU64,
    failures: AtomicU64,
    active: AtomicBool,
    post: Box<dyn Fn() -> bool + Send + Sync>,
}
impl Wake {
    pub fn new(post: impl Fn() -> bool + Send + Sync + 'static) -> Self {
        Self {
            state: AtomicU64::new(0),
            failures: AtomicU64::new(0),
            active: AtomicBool::new(true),
            post: Box::new(post),
        }
    }
    pub fn notify(&self) {
        if !self.active.load(Ordering::Acquire) {
            return;
        }
        let Ok(previous) = self
            .state
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                if value & 1 == 0 {
                    value.checked_add(1)
                } else {
                    None
                }
            })
        else {
            return;
        };
        let ticket = previous + 1;
        // 回调异常也不得使 worker 析构时再次 panic，或永久卡住 pending。
        let sent = self.active.load(Ordering::Acquire)
            && std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (self.post)()))
                .unwrap_or(false);
        // 失败只撤销自己的通知代次，不能覆盖期间 acknowledge 后的新一次成功投递。
        if !sent {
            let _ = self
                .failures
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                    Some(value.saturating_add(1))
                });
            if let Some(next) = ticket.checked_add(1) {
                let _ =
                    self.state
                        .compare_exchange(ticket, next, Ordering::AcqRel, Ordering::Acquire);
            }
        }
    }
    pub fn acknowledge(&self) {
        if self
            .state
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                (value | 1).checked_add(1)
            })
            .is_err()
        {
            self.disable();
        }
    }
    pub fn disable(&self) {
        self.active.store(false, Ordering::Release);
    }
    pub fn failures(&self) -> u64 {
        self.failures.load(Ordering::Relaxed)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, OnceLock, Weak};
    #[test]
    fn stale_failed_post_cannot_clear_a_new_successful_reservation() {
        let holder = Arc::new(OnceLock::<Weak<Wake>>::new());
        let reference = holder.clone();
        let calls = Arc::new(AtomicU64::new(0));
        let count = calls.clone();
        let wake = Arc::new(Wake::new(move || {
            if count.fetch_add(1, Ordering::AcqRel) == 0 {
                let current = reference.get().unwrap().upgrade().unwrap();
                current.acknowledge();
                current.notify();
                false
            } else {
                true
            }
        }));
        holder.set(Arc::downgrade(&wake)).unwrap();
        wake.notify();
        wake.notify();
        assert_eq!(calls.load(Ordering::Acquire), 2);
        wake.acknowledge();
        wake.notify();
        assert_eq!(calls.load(Ordering::Acquire), 3);
        wake.disable();
        wake.acknowledge();
        wake.notify();
        assert_eq!(calls.load(Ordering::Acquire), 3);
    }
}
