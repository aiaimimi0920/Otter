//! 调度容量/公平/指数退避与无候选静默；注入 Instant 不改产品期限或系统时间。
use super::*;
use crate::projection_delivery::{
    accept::tests as png,
    journal::{tests::Directory, Store},
    tests as wire, Inbox,
};
fn candidates(count: usize) -> (Directory, Vec<(Arc<Ticket>, Binding)>) {
    let root = Directory::new();
    let store = Store::open(&root.0).unwrap();
    let origin = "https://loom.example";
    let first = store.begin(&png::invitation(origin), "device-1").unwrap();
    let sample = Binding::new(&png::verified(origin).bind_intent(&first).unwrap(), &first).unwrap();
    let mut rows = Vec::new();
    for number in 1..=count {
        let mut raw = wire::invitation(origin, false);
        raw["envelope"]["projectionId"] = format!("projection:{number:032x}").into();
        let body = serde_json::to_vec(&serde_json::json!({"invitations":[raw]})).unwrap();
        let invitation = Inbox::parse(&body, origin)
            .unwrap()
            .into_entries()
            .pop()
            .unwrap();
        let ticket = store.begin(&invitation, "device-1").unwrap();
        let mut binding = serde_json::to_value(&sample).unwrap();
        binding["envelope"] = serde_json::to_value(ticket.envelope()).unwrap();
        binding["localUnitId"] = ticket.unit().into();
        binding["otterNativeReceive"]["intentId"] = ticket.intent_id().into();
        let binding: Binding = serde_json::from_value(binding).unwrap();
        assert!(binding.matches_ticket(&ticket));
        rows.push((Arc::new(ticket), binding));
    }
    (root, rows)
}
#[test]
fn projection_sync_schedule_capacity_remains_64_when_candidate_order_changes() {
    let (_root, mut rows) = candidates(65);
    let now = Instant::now();
    let mut schedule = Schedule::default();
    schedule.reconcile(rows.clone(), now);
    assert_eq!(schedule.entries.len(), 64);
    assert!(schedule.due(now).is_none());
    rows.rotate_left(1);
    schedule.reconcile(rows, now);
    assert_eq!(schedule.entries.len(), 64);
    schedule.reconcile(Vec::new(), now);
    assert!(schedule.entries.is_empty());
    assert!(schedule.due(now + Duration::from_secs(60)).is_none());
}
#[test]
fn projection_sync_schedule_oldest_due_is_fair_and_backoff_is_bounded_then_resets() {
    let (_root, rows) = candidates(3);
    let now = Instant::now();
    let mut schedule = Schedule::default();
    schedule.reconcile(rows.clone(), now);
    let due = now + Duration::from_millis(750);
    assert_eq!(schedule.due(due), Some(0));
    schedule.settle(&rows[0].0, due, false);
    assert_eq!(schedule.due(due), Some(1));
    assert_eq!(schedule.entries[0].due, due + Duration::from_millis(1000));
    for _ in 0..20 {
        schedule.settle(&rows[0].0, due, false);
    }
    assert_eq!(schedule.entries[0].due, due + Duration::from_secs(30));
    schedule.settle(&rows[0].0, due, true);
    assert_eq!(schedule.entries[0].due, due + Duration::from_millis(750));
}
#[test]
fn projection_sync_schedule_stopped_entries_do_not_hide_later_active_candidates() {
    let (_root, mut rows) = candidates(65);
    for (_, binding) in &mut rows[..64] {
        *binding =
            binding.stop(crate::projection_delivery::binding::StopReason::ProjectionUnlinked);
    }
    let now = Instant::now();
    let mut schedule = Schedule::default();
    schedule.reconcile(rows.clone(), now);
    assert_eq!(schedule.entries.len(), 1);
    assert!(schedule.entries[0].ticket.same_record(&rows[64].0));
}
#[test]
fn projection_sync_schedule_foreground_completion_defers_due_reads_without_shortening_backoff() {
    let (_root, rows) = candidates(2);
    let now = Instant::now();
    let mut schedule = Schedule::default();
    schedule.reconcile(rows.clone(), now);
    let due = now + Duration::from_millis(750);
    schedule.settle(&rows[0].0, due, false);
    schedule.defer_foreground(due);
    assert!(schedule.due(due).is_none());
    assert_eq!(schedule.due(due + Duration::from_millis(750)), Some(1));
    assert_eq!(schedule.entries[0].due, due + Duration::from_millis(1000));
}
