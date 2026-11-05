use super::*;
#[test]
fn idle_has_no_sampling_and_thousands_of_events_coalesce_behind_one_worker() {
    let now = Instant::now();
    let mut schedule = Schedule::default();
    assert!(schedule.start(now));
    assert!(!schedule.needs_timer(false));
    assert_eq!(
        schedule.action(now + Duration::from_secs(1), false),
        Action::Idle
    );
    for _ in 0..10_000 {
        schedule.notify(now);
    }
    assert_eq!(schedule.action(now, false), Action::Idle);
    assert_eq!(schedule.action(now + DELAY, false), Action::Capture);
    for _ in 0..10_000 {
        schedule.notify(now + DELAY);
    }
    assert_eq!(schedule.action(now + DELAY * 2, true), Action::Idle);
    assert_eq!(schedule.attempts, 1);
    schedule.completed(now + DELAY * 2);
    assert_eq!(schedule.action(now + DELAY * 2, false), Action::Capture);
    for index in 3..=4 {
        schedule.completed(now + DELAY * (index - 1));
        assert_eq!(schedule.action(now + DELAY * index, false), Action::Capture);
    }
    schedule.completed(now + DELAY * 4);
    assert_eq!(schedule.action(now + DELAY * 5, false), Action::Idle);
    assert!(!schedule.needs_timer(false));
    assert_eq!(schedule.attempts, 4);
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
    assert_eq!(fresh.action(now + RECENT + DELAY, false), Action::Compose);
}
#[test]
fn attempts_and_total_session_time_are_not_reset_by_stop_resume() {
    let now = Instant::now();
    let mut schedule = Schedule::default();
    schedule.start(now);
    schedule.attempts = 510;
    schedule.notify(now);
    assert_eq!(schedule.action(now + DELAY, false), Action::Capture);
    schedule.stop();
    assert!(!schedule.start(now + DELAY));
    assert_eq!(schedule.attempts, 511);
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
