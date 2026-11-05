//! 不变时钟验证公平性/有界重试/删除；不靠sleep或放宽产品期限。
use super::*;
use crate::projection_delivery::sender::tests::{prepare, selection, setup};
fn candidates() -> Vec<(Request, Binding)> {
    let (directory, store) = setup();
    let source =
        crate::projection_delivery::sender::binding::tests::source("unit-1", &"a".repeat(64));
    (1..=3).map(|index| {
        let intent = prepare(&store,&directory,&selection(index)).unwrap();
        let response = crate::projection_delivery::sender::Response::parse(
            &serde_json::to_vec(&serde_json::json!({"envelope":intent.envelope(),"revision":1,
                "digest":intent.digest(),"linked":true,"receiverDeviceId":null,"receiverUnitId":null,
                "snapshot":null,"delivery":{"targetDeviceId":intent.target(),"status":"awaiting_confirmation"}
            })).unwrap(),&intent,false,&std::sync::atomic::AtomicBool::new(false)).unwrap();
        let binding = Binding::prepare(&source,&intent,&response,"target",
            &std::sync::atomic::AtomicBool::new(false)).unwrap().binding().clone();
        (Request::new(intent,directory.identity()),binding)
    }).collect()
}
#[test]
fn continuous_schedule_is_oldest_due_first_and_failures_cannot_starve_other_bindings() {
    let now = Instant::now();
    let values = candidates();
    let mut schedule = Schedule::default();
    schedule.reconcile(values.clone(), now);
    assert_eq!(schedule.due(now), None);
    assert_eq!(schedule.due(now + Duration::from_millis(749)), None);
    let due = now + Duration::from_millis(750);
    assert_eq!(schedule.due(due), Some(0));
    schedule.settle(&values[0].0, None, due);
    assert_eq!(schedule.due(due), Some(1));
    assert_eq!(schedule.entries[0].due, due + Duration::from_secs(1));
    for _ in 0..20 {
        schedule.settle(&values[0].0, None, due);
    }
    assert_eq!(schedule.entries[0].due, due + Duration::from_secs(30));
    let deadline = schedule.entries[0].due;
    schedule.reconcile(values.clone(), due);
    assert_eq!(
        schedule.entries[0].due, deadline,
        "reconcile must not reset backoff"
    );
    assert_eq!(schedule.entries.len(), 3);
    schedule.clear();
    assert!(schedule.entries.is_empty());
    assert!(schedule.wake.is_none());
}
#[test]
fn continuous_schedule_deduplicates_and_drops_removed_workspace_entries() {
    let now = Instant::now();
    let values = candidates();
    let mut schedule = Schedule::default();
    schedule.reconcile(vec![values[0].clone(); 1024], now);
    assert_eq!(schedule.entries.len(), 1);
    schedule.reconcile(values[1..].to_vec(), now);
    assert_eq!(schedule.entries.len(), 2);
    assert!(!schedule
        .entries
        .iter()
        .any(|entry| entry.request.same_intent(&values[0].0)));
    schedule.reconcile(Vec::new(), now);
    assert!(schedule.entries.is_empty());
    assert_eq!(schedule.due(now + Duration::from_secs(60)), None);
}
#[test]
fn source_settlement_only_advances_its_target_and_preserves_other_targets_due_and_backoff() {
    let now = Instant::now();
    let values = candidates();
    let mut schedule = Schedule::default();
    schedule.reconcile(values.clone(), now);
    let due = now + Duration::from_millis(750);
    let unit = crate::workspace_id::UnitId::parse("unit-1").unwrap();
    let first = values[0].1.envelope().projection_id();
    schedule.source_settled(&unit, Some(first), None, true, due);
    assert_eq!(schedule.due(due), Some(1));
    assert_eq!(schedule.entries[0].due, due + Duration::from_millis(750));
    schedule.source_settled(&unit, Some(first), None, false, due);
    assert_eq!(schedule.entries[0].due, due + Duration::from_secs(1));
    let first_backoff = schedule.entries[0].due;
    schedule.source_settled(
        &unit,
        Some(values[1].1.envelope().projection_id()),
        None,
        true,
        due,
    );
    assert_eq!(schedule.due(due), Some(2));
    schedule.source_settled(&unit, None, None, true, due);
    assert_eq!(schedule.entries[0].due, first_backoff);
    assert_eq!(schedule.entries[2].due, due);
}
