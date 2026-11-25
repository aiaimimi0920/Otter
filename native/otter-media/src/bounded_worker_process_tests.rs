//! 容量等待不依赖实际进程：覆盖截止、取消及每轮 idle 回收。
use super::*;
use std::{
    cell::{Cell, RefCell},
    time::Duration,
};

#[test]
fn expired_or_cancelled_capacity_never_reserves_idle_pool() {
    let pool = crate::byte_budget::Budget::new(4);
    for (deadline, stopped) in [
        (Instant::now(), false),
        (Instant::now() + Duration::from_secs(1), true),
    ] {
        let reclaimed = Cell::new(false);
        let result = capacity_reclaiming(&pool, 4, deadline, &|| stopped, &|| {
            reclaimed.set(true);
        });
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::TimedOut);
        assert!(!reclaimed.get());
        assert_eq!(pool.used(), 0);
    }
}

#[test]
fn capacity_reclaims_idle_owner_again_after_timed_wait() {
    let pool = crate::byte_budget::Budget::new(4);
    let held = RefCell::new(pool.reserve(4));
    let calls = Cell::new(0);
    let lease = capacity_reclaiming(
        &pool,
        4,
        Instant::now() + Duration::from_secs(2),
        &|| false,
        &|| {
            calls.set(calls.get() + 1);
            if calls.get() == 2 {
                held.borrow_mut().take();
            }
        },
    )
    .unwrap();
    assert_eq!(calls.get(), 2);
    assert_eq!(pool.used(), 4);
    drop(lease);
    assert_eq!(pool.used(), 0);
}

#[test]
fn cancellation_during_reclaim_does_not_admit_available_capacity() {
    let pool = crate::byte_budget::Budget::new(4);
    let stopped = Cell::new(false);
    let result = capacity_reclaiming(
        &pool,
        4,
        Instant::now() + Duration::from_secs(1),
        &|| stopped.get(),
        &|| stopped.set(true),
    );
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::TimedOut);
    assert_eq!(pool.used(), 0);
}
