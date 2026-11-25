use super::*;

#[test]
fn explicit_shell_activation_and_foreground_taskbar_toggle_are_single_use() {
    let mut tracker = IntentTracker::default();
    let mut state = Activation::new(None, 2);
    assert!(tracker.record(10, 2));
    assert!(state.observe(1, 1, false, tracker.snapshot(10), 10));
    assert!(!state.observe(1, 1, true, tracker.snapshot(11), 11));
    // 同一毫秒也必须生成不同序号，且不制造未来时间戳。
    assert!(tracker.record(10, 1));
    assert!(state.observe(1, 1, true, tracker.snapshot(10), 10));
}

#[test]
fn initial_intent_late_activation_and_cancel_do_not_reclaim() {
    let mut tracker = IntentTracker::default();
    tracker.record(10, 2);
    let mut state = Activation::new(tracker.snapshot(10), 2);
    assert!(!state.observe(1, 1, false, tracker.snapshot(11), 11));
    assert!(!state.observe(2, 1, false, tracker.snapshot(12), 12));
    assert!(!state.observe(1, 1, false, tracker.snapshot(1000), 1000));
    tracker.record(1001, 1);
    tracker.cancel();
    assert!(!state.observe(1, 1, true, tracker.snapshot(1002), 1002));
    assert!(!state.observe(2, 1, false, None, 1003));
    assert!(!state.observe(1, 1, false, None, 1004));
}

#[test]
fn expiration_future_clock_and_out_of_order_tokens_fail_closed() {
    let mut tracker = IntentTracker::default();
    tracker.record(100, 1);
    let old = tracker.snapshot(100);
    assert!(tracker.snapshot(99).is_none());
    assert!(tracker.snapshot(2100).is_some());
    assert!(tracker.snapshot(2101).is_none());
    let mut state = Activation::new(None, 1);
    assert!(!state.observe(1, 1, true, old, 99));
    assert!(!state.observe(1, 1, true, old, 100));
    tracker.record(200, 1);
    assert!(state.observe(1, 1, true, tracker.snapshot(200), 200));
    assert!(!state.observe(1, 1, true, old, 201));
    let mut expired = Activation::new(None, 1);
    assert!(!expired.observe(1, 1, true, old, 2101));
}

#[test]
fn unrelated_iconic_source_cannot_consume_intent_or_restore_null_target() {
    let mut tracker = IntentTracker::default();
    tracker.record(10, 2);
    let mut state = Activation::new(None, 2);
    assert!(!state.observe(2, 1, true, tracker.snapshot(10), 10));
    assert!(!state.observe(0, 0, false, tracker.snapshot(10), 10));
    assert!(state.observe(1, 1, false, tracker.snapshot(11), 11));
}

#[test]
fn sequence_exhaustion_cancels_existing_authority_without_wrapping() {
    let mut tracker = IntentTracker {
        sequence: u64::MAX - 1,
        current: None,
    };
    assert!(tracker.record(10, 1));
    assert!(tracker.snapshot(10).is_some());
    assert!(!tracker.record(11, 1));
    assert!(tracker.snapshot(11).is_none());
    assert_eq!(tracker.sequence, u64::MAX);
}
