use super::*;
#[test]
fn cover_retries_at_bounded_cadence_and_finishing_uses_trusted_frames() {
    let now = Instant::now();
    let mut schedule = Schedule::default();
    assert!(schedule.start(now));
    schedule.notify(now);
    schedule.covered(now);
    assert_eq!(schedule.wake_after(now, false), Some(COVER_CHECK));
    assert_eq!(schedule.action(now + POLL, false), Action::Idle);
    assert_eq!(schedule.action(now + COVER_CHECK, false), Action::Capture);

    schedule.finish(now + COVER_CHECK);
    schedule.covered(now + COVER_CHECK);
    assert_eq!(schedule.action(now + COVER_CHECK, false), Action::Compose);
    assert_eq!(schedule.wake_after(now + COVER_CHECK, false), None);
}

#[test]
fn recording_polls_without_wheel_and_thousands_of_hints_coalesce_behind_one_worker() {
    let now = Instant::now();
    let mut schedule = Schedule::default();
    assert!(schedule.start(now));
    assert!(!schedule.needs_timer(false));
    assert_eq!(schedule.wake_after(now, false), Some(POLL));
    assert_eq!(schedule.action(now + POLL, false), Action::Capture);
    schedule.completed(now + POLL);
    assert_eq!(schedule.action(now + POLL * 2, false), Action::Capture);
    schedule.completed(now + POLL * 2);
    let now = now + POLL * 2;
    for _ in 0..10_000 {
        schedule.notify(now);
    }
    assert_eq!(schedule.action(now, false), Action::Idle);
    assert_eq!(schedule.action(now + DELAY, false), Action::Capture);
    for _ in 0..10_000 {
        schedule.notify(now + DELAY);
    }
    assert_eq!(schedule.action(now + DELAY * 2, true), Action::Idle);
    schedule.completed(now + DELAY * 2);
    assert_eq!(schedule.action(now + DELAY * 2, false), Action::Capture);
    for index in 3..=4 {
        schedule.completed(now + DELAY * (index - 1));
        assert_eq!(schedule.action(now + DELAY * index, false), Action::Capture);
    }
    schedule.completed(now + DELAY * 4);
    assert_eq!(schedule.action(now + DELAY * 5, false), Action::Idle);
    assert!(!schedule.needs_timer(false));
    assert_eq!(
        schedule.action(now + DELAY * 4 + POLL, false),
        Action::Capture
    );
}
#[test]
fn stop_discards_pending_tail_and_late_completion_cannot_restart_it() {
    let now = Instant::now();
    let mut schedule = Schedule::default();
    schedule.start(now);
    schedule.notify(now);
    assert_eq!(schedule.action(now + DELAY, false), Action::Capture);
    schedule.notify(now + DELAY);
    schedule.stop();
    schedule.completed(now + DELAY * 2);
    assert!(!schedule.needs_timer(false));
    assert_eq!(
        schedule.action(now + Duration::from_secs(1), false),
        Action::Idle
    );
    schedule.notify(now);
    assert!(!schedule.dirty);
}
#[test]
fn finish_drains_recent_hint_once_then_composes_and_never_authorizes_new_events() {
    let now = Instant::now();
    let mut schedule = Schedule::default();
    schedule.start(now);
    schedule.notify(now);
    schedule.finish(now);
    schedule.notify(now + DELAY); // listener 已卸载，迟到通知也不能延长 drain。
    assert_eq!(schedule.action(now + DELAY, false), Action::Capture);
    assert_eq!(schedule.action(now + DRAIN, true), Action::Idle);
    schedule.completed(now + DRAIN);
    assert_eq!(schedule.action(now + DRAIN, false), Action::Compose);
    assert_eq!(schedule.action(now + DRAIN, false), Action::Idle);
    assert_eq!(schedule.phase(), Phase::Finished);
}
#[test]
fn old_hint_cancel_and_restart_do_not_leak_across_sessions() {
    let now = Instant::now();
    let mut old = Schedule::default();
    old.start(now);
    old.notify(now);
    old.cancel();
    old.completed(now);
    old.notify(now);
    assert!(!old.start(now));
    assert!(!old.needs_timer(false));
    let mut fresh = Schedule::default();
    fresh.start(now);
    assert_eq!(fresh.action(now + DELAY, false), Action::Idle);
    fresh.notify(now);
    fresh.finish(now + RECENT + DELAY);
    assert_eq!(
        fresh.action(now + RECENT + DELAY * 2, false),
        Action::Capture
    );
    fresh.completed(now + RECENT + DELAY * 2);
    assert_eq!(
        fresh.action(now + RECENT + DELAY * 2, false),
        Action::Compose
    );
}
#[test]
fn repeated_polling_does_not_exhaust_frames_and_total_time_survives_stop_resume() {
    let now = Instant::now();
    let mut schedule = Schedule::default();
    schedule.start(now);
    for index in 1..=1000 {
        let at = now + POLL * index;
        assert_eq!(schedule.action(at, false), Action::Capture);
        schedule.completed(at);
    }
    assert_eq!(schedule.phase(), Phase::Recording);
    let mut timed = Schedule::default();
    timed.start(now);
    timed.stop();
    assert!(!timed.start(now + Duration::from_secs(300)));
    let mut active = Schedule::default();
    active.start(now);
    active.notify(now + Duration::from_secs(300));
    assert_eq!(
        active.action(now + Duration::from_secs(301), false),
        Action::Limit
    );
    assert!(!active.needs_timer(false));
}

#[test]
fn finishing_after_drag_without_any_wheel_takes_a_final_frame_and_stops_polling() {
    let now = Instant::now();
    let mut schedule = Schedule::default();
    schedule.start(now);
    schedule.finish(now);
    assert_eq!(schedule.action(now, false), Action::Idle);
    assert_eq!(schedule.action(now + DELAY, false), Action::Capture);
    assert_eq!(schedule.action(now + POLL, true), Action::Idle);
    schedule.completed(now + POLL);
    assert_eq!(schedule.action(now + POLL, false), Action::Compose);
    assert_eq!(schedule.wake_after(now + POLL, false), None);
}

#[test]
fn slow_inflight_job_cannot_consume_the_completion_frame_budget() {
    let now = Instant::now();
    let mut schedule = Schedule::default();
    schedule.start(now);
    schedule.finish(now);
    let ready = now + DRAIN * 2;
    assert_eq!(schedule.action(ready, true), Action::Idle);
    schedule.completed(ready);
    assert_eq!(schedule.action(ready, false), Action::Capture);
    schedule.completed(ready + POLL);
    assert_eq!(schedule.action(ready + POLL, false), Action::Compose);
    assert_eq!(schedule.wake_after(ready + POLL, false), None);
}

#[test]
fn slow_matching_does_not_add_another_poll_period_after_the_reply() {
    let now = Instant::now();
    let mut schedule = Schedule::default();
    schedule.start(now);
    assert_eq!(schedule.action(now + POLL, false), Action::Capture);
    let ready = now + POLL * 5;
    assert_eq!(schedule.action(ready, true), Action::Idle);
    schedule.completed(ready);
    assert_eq!(schedule.action(ready, false), Action::Idle); // 预览有机会使用同一槽。
    assert_eq!(
        schedule.action(ready + Duration::from_millis(1), false),
        Action::Capture
    );
    assert_eq!(schedule.wake_after(ready, true), Some(REPLY));
}
