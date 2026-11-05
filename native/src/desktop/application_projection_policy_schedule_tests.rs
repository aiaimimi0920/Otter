//! 真实 Invitation 的有界、公平、NEW-only 和失败退避；不启动网络或修改墙钟。
use super::*;
use crate::projection_delivery::{tests::invitation, Inbox};
const ORIGIN: &str = "https://loom.example";
fn entries(count: usize, accepted: bool) -> Vec<Arc<Invitation>> {
    let values = (0..count)
        .map(|index| {
            let mut value = invitation(ORIGIN, accepted);
            value["envelope"]["projectionId"] = format!("projection:{:032x}", index + 1).into();
            value
        })
        .collect::<Vec<_>>();
    Inbox::parse(
        &serde_json::to_vec(&serde_json::json!({"invitations":values})).unwrap(),
        ORIGIN,
    )
    .unwrap()
    .into_entries()
    .into_iter()
    .map(Arc::new)
    .collect()
}
#[test]
fn receive_policy_schedule_fair_bounded_and_does_not_repeat_failed_accepts() {
    let entries = entries(64, false);
    let mut schedule = Schedule::default();
    let start = Instant::now();
    for (index, entry) in entries.iter().enumerate() {
        let now = start + Duration::from_secs(index as u64 * 3);
        let Some(Request::Accept(selected)) = schedule.plan(ORIGIN, &entries, Policy::Auto, now)
        else {
            panic!("missing fair candidate")
        };
        assert!(Arc::ptr_eq(entry, &selected));
        assert!(schedule.plan(ORIGIN, &entries, Policy::Auto, now).is_none());
        // 同一快照回传不能重新授予自动尝试；成功与失败都不能热循环。
        schedule.inbox(&entries);
    }
    assert_eq!(schedule.attempted.len(), 64);
    assert!(matches!(
        schedule.plan(
            ORIGIN,
            &entries,
            Policy::Auto,
            start + Duration::from_secs(192)
        ),
        Some(Request::Refresh)
    ));
    schedule.reset();
    assert!(matches!(
        schedule.plan(ORIGIN, &entries, Policy::Reject, start),
        Some(Request::Reject(_))
    ));
}
#[test]
fn receive_policy_schedule_accepted_uses_reattach_not_reject_and_caps_network_failure_backoff() {
    let entries = entries(1, true);
    let mut schedule = Schedule::default();
    let now = Instant::now();
    assert!(matches!(
        schedule.plan(ORIGIN, &entries, Policy::Confirm, now),
        Some(Request::Reattach(_))
    ));
    assert_eq!(schedule.attempted.len(), 1);
    assert_eq!(schedule.wake, Some(now + Duration::from_secs(3)));
    for _ in 0..12 {
        schedule.failure(ORIGIN, now);
    }
    assert!(schedule
        .plan(
            ORIGIN,
            &entries,
            Policy::Reject,
            now + Duration::from_secs(29)
        )
        .is_none());
    assert_eq!(schedule.wake, Some(now + Duration::from_secs(30)));
    assert!(matches!(
        schedule.plan(
            ORIGIN,
            &entries,
            Policy::Reject,
            now + Duration::from_secs(30)
        ),
        Some(Request::Refresh)
    ));
}

#[test]
fn projection_reattach_schedule_never_automatically_migrates_existing_record_or_unit() {
    use crate::projection_delivery::{
        accept::tests as png,
        journal::{tests::Directory, Store},
    };
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let entry = png::accepted(ORIGIN);
    assert!(missing(&entry, &[], &[]));
    let ticket = Arc::new(store.begin(&entry, "device-1").unwrap());
    assert!(!missing(&entry, &[ticket], &[]));
    let unit = crate::workspace_id::UnitId::parse(&entry.receiver_unit_id()).unwrap();
    assert!(!missing(&entry, &[], &[unit]));
}

#[test]
fn projection_reattach_schedule_early_wakes_keep_deadline_and_do_not_consume_candidates() {
    let mut schedule = Schedule::default();
    let now = Instant::now();
    let entries = entries(1, true);
    assert!(matches!(
        schedule.plan(ORIGIN, &[], Policy::Reject, now),
        Some(Request::Refresh)
    ));
    for offset in 0..1000 {
        assert!(!schedule.due(ORIGIN, now + Duration::from_millis(offset)));
        assert!(schedule.attempted.is_empty());
        assert_eq!(schedule.wake, Some(now + Duration::from_secs(3)));
    }
    assert!(matches!(
        schedule.plan(
            ORIGIN,
            &entries,
            Policy::Reject,
            now + Duration::from_secs(3)
        ),
        Some(Request::Reattach(_))
    ));
}

#[test]
fn receive_policy_slow_success_preserves_three_second_sync_window() {
    let mut schedule = Schedule::default();
    let start = Instant::now();
    assert!(matches!(
        schedule.plan(ORIGIN, &[], Policy::Confirm, start),
        Some(Request::Refresh)
    ));
    let finished = start + Duration::from_secs(12);
    schedule.completed(ORIGIN, finished);
    assert!(schedule
        .plan(
            ORIGIN,
            &[],
            Policy::Confirm,
            finished + Duration::from_millis(750)
        )
        .is_none());
    assert_eq!(schedule.wake, Some(finished + Duration::from_secs(3)));
    assert!(matches!(
        schedule.plan(
            ORIGIN,
            &[],
            Policy::Confirm,
            finished + Duration::from_secs(3)
        ),
        Some(Request::Refresh)
    ));
}

#[test]
fn receive_policy_explicit_completion_and_failure_initialize_origin_without_losing_deadlines() {
    let now = Instant::now();
    let mut completed = Schedule::default();
    completed.completed(ORIGIN, now);
    assert!(!completed.due(ORIGIN, now));
    assert_eq!(completed.wake, Some(now + Duration::from_secs(3)));
    assert!(completed.due(ORIGIN, now + Duration::from_secs(3)));

    let mut failed = Schedule::default();
    failed.failure(ORIGIN, now);
    assert!(!failed.due(ORIGIN, now + Duration::from_secs(3)));
    assert_eq!(failed.wake, Some(now + Duration::from_secs(6)));
    assert!(failed.due(ORIGIN, now + Duration::from_secs(6)));
}

#[test]
fn receive_policy_same_origin_results_preserve_attempts_and_only_new_origin_resets() {
    let now = Instant::now();
    let entries = entries(1, false);
    let mut schedule = Schedule::default();
    assert!(matches!(
        schedule.plan(ORIGIN, &entries, Policy::Auto, now),
        Some(Request::Accept(_))
    ));
    schedule.completed(ORIGIN, now);
    schedule.inbox(&entries);
    schedule.failure(ORIGIN, now);
    assert_eq!(schedule.attempted.len(), 1);
    assert!(matches!(
        schedule.plan(ORIGIN, &entries, Policy::Auto, now + Duration::from_secs(6)),
        Some(Request::Refresh)
    ));
    schedule.completed("https://other.example", now);
    assert!(schedule.attempted.is_empty());
    assert_eq!(schedule.failures, 0);
    assert!(!schedule.due("https://other.example", now));
    schedule.failure(ORIGIN, now);
    assert_eq!(schedule.failures, 1);
    assert!(!schedule.due(ORIGIN, now));
    assert_eq!(schedule.wake, Some(now + Duration::from_secs(6)));
}
#[test]
fn projection_offline_schedule_attempt_identity_includes_transport_and_peer_and_missing_record() {
    use crate::projection_delivery::{
        journal::{tests::Directory, Store},
        tests::offline,
    };
    let now = Instant::now();
    let mut schedule = Schedule::default();
    let old = Arc::new(offline::invitation(ORIGIN, true));
    assert!(matches!(
        schedule.plan(ORIGIN, std::slice::from_ref(&old), Policy::Confirm, now),
        Some(Request::Reattach(_))
    ));
    let mut changed = offline::entry(true);
    changed["peerId"] = format!("loom-{}", "c".repeat(64)).into();
    let changed = Arc::new(
        Inbox::parse_offline(
            &serde_json::to_vec(&serde_json::json!({"invitations":[changed]})).unwrap(),
            ORIGIN,
        )
        .unwrap()
        .into_entries()
        .pop()
        .unwrap(),
    );
    schedule.inbox(std::slice::from_ref(&changed));
    assert!(schedule.attempted.is_empty());
    assert!(matches!(
        schedule.plan(
            ORIGIN,
            std::slice::from_ref(&changed),
            Policy::Confirm,
            now + Duration::from_secs(3)
        ),
        Some(Request::Reattach(_))
    ));
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let ticket = Arc::new(store.begin(&old, "device-1").unwrap());
    assert!(!missing(&old, std::slice::from_ref(&ticket), &[]));
    assert!(missing(&changed, std::slice::from_ref(&ticket), &[]));
    let other = offline::invitation("https://other.example", true);
    assert!(missing(&other, &[ticket], &[]));
}
