//! 原版已实现的完整Esc+Delete组合语义；事件主循环与独立watchdog共用纯状态机。
use std::time::{Duration, Instant};

const WINDOW: Duration = Duration::from_millis(400);

#[derive(Default)]
pub(crate) struct Tracker {
    latched: bool,
    last: Option<Instant>,
    count: u8,
}
impl Tracker {
    pub(crate) fn state(&mut self, escape: bool, delete: bool, now: Instant) -> bool {
        if !escape && !delete {
            self.latched = false;
        }
        if !escape || !delete || self.latched {
            return false;
        }
        self.latched = true;
        self.count = if self
            .last
            .is_some_and(|last| now.saturating_duration_since(last) < WINDOW)
        {
            self.count.saturating_add(1)
        } else {
            1
        };
        self.last = Some(now);
        if self.count == 3 {
            self.count = 0;
            self.last = None;
            true
        } else {
            false
        }
    }
    pub(crate) fn message(
        &mut self,
        message: u32,
        key: usize,
        now: Instant,
        observed: impl FnOnce() -> (bool, bool),
    ) -> bool {
        if !matches!(message, 0x0100 | 0x0101 | 0x0104 | 0x0105) || !matches!(key, 0x1b | 0x2e) {
            return false;
        }
        // 局部消息可能漏收keyup或经过模态循环；不把历史事件当作仍按住的物理状态。
        // 延迟消息漏记组合是安全降级，独立watchdog仍观察完整物理键位。
        let (escape, delete) = observed();
        self.state(escape, delete, now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn triple_requires_overlap_full_release_and_accepts_either_key_order() {
        let at = Instant::now();
        let mut tracker = Tracker::default();
        for (index, (first, second)) in [(0x1b, 0x2e), (0x2e, 0x1b), (0x1b, 0x2e)]
            .into_iter()
            .enumerate()
        {
            let now = at + Duration::from_millis(index as u64 * 150);
            assert!(!tracker.message(0x100, first, now, || (first == 0x1b, first == 0x2e)));
            assert_eq!(
                tracker.message(0x100, second, now, || (true, true)),
                index == 2
            );
            assert!(!tracker.message(0x101, first, now, || (first != 0x1b, first != 0x2e)));
            assert!(!tracker.message(0x101, second, now, || (false, false)));
        }
    }
    #[test]
    fn autorepeat_partial_release_and_one_key_never_advance() {
        let now = Instant::now();
        for held in [0x1b, 0x2e] {
            let mut t = Tracker::default();
            assert!(!t.state(true, true, now));
            let tapped = if held == 0x1b { 0x2e } else { 0x1b };
            for _ in 0..20 {
                assert!(!t.message(0x100, held, now, || (true, true)));
                assert!(!t.message(0x101, tapped, now, || (held == 0x1b, held == 0x2e)));
                assert!(!t.message(0x100, tapped, now, || (true, true)));
            }
            assert!(!t.state(false, false, now));
            assert!(!t.state(true, true, now));
            assert!(!t.state(false, false, now));
            assert!(t.state(true, true, now));
        }
        let mut t = Tracker::default();
        for _ in 0..20 {
            for key in [0x1b, 0x2e] {
                assert!(!t.message(0x100, key, now, || (key == 0x1b, key == 0x2e)));
                assert!(!t.message(0x101, key, now, || (false, false)));
            }
        }
    }
    #[test]
    fn exact_400ms_resets_but_one_ns_inside_continues_and_completed_sequence_resets() {
        let at = Instant::now();
        for delta in [WINDOW, WINDOW + Duration::from_nanos(1)] {
            let mut t = Tracker::default();
            for index in 0..4 {
                assert!(!t.state(true, true, at + delta * index));
                t.state(false, false, at + delta * index);
            }
        }
        let mut t = Tracker::default();
        let delta = WINDOW - Duration::from_nanos(1);
        for index in 0..4 {
            assert_eq!(t.state(true, true, at + delta * index), index == 2);
            t.state(false, false, at + delta * index);
        }
    }
    #[test]
    fn irrelevant_messages_and_non_overlapping_events_do_not_arm_exit() {
        let at = Instant::now();
        let mut t = Tracker::default();
        for _ in 0..10 {
            assert!(!t.message(0x200, 0x1b, at, || panic!("Unrelated message sampled keys")));
            assert!(!t.message(0x100, 0x41, at, || panic!("Unrelated key sampled keys")));
            assert!(!t.message(0x104, 0x1b, at, || (true, false)));
            assert!(!t.message(0x105, 0x1b, at, || (false, false)));
            assert!(!t.message(0x100, 0x2e, at, || (false, true)));
            assert!(!t.message(0x101, 0x2e, at, || (false, false)));
        }
    }
    #[test]
    fn missing_keyup_and_delayed_messages_cannot_manufacture_overlap() {
        let at = Instant::now();
        let mut t = Tracker::default();
        assert!(!t.message(0x100, 0x1b, at, || (true, false)));
        // Escape已松开但keyup被其他窗口/嵌套泵消费；Delete单键不能算组合。
        for _ in 0..4 {
            assert!(!t.message(0x100, 0x2e, at, || (false, true)));
            assert!(!t.message(0x101, 0x2e, at, || (false, false)));
        }
        // 队列迟到的down在物理释放后处理，不累计旧组合。
        assert!(!t.message(0x100, 0x1b, at, || (false, false)));
        for index in 0..3 {
            assert_eq!(t.message(0x100, 0x2e, at, || (true, true)), index == 2);
            t.message(0x101, 0x1b, at, || (false, false));
        }
    }
}
