//! 真实 HWND 批次交付：全部证据与授权独立核验，只替换一次最高版本场景。
use super::*;
#[path = "surface_batch_confirmation_tests.rs"]
mod confirmation;
use crate::{
    live_relay_http::{BatchStop, BatchSubmission},
    surface_event_batch_runtime::{Input, Observation, ObservedItem, Outcome},
};
fn batch(window: &mut AuthorizedWindow) -> Outcome {
    let mut events = vec![event(window), event(window)];
    let owner = window.begin_batch(&events, 0).unwrap().unwrap();
    let acknowledgements = events.iter_mut().enumerate().map(|(index, event)| {
        event.begin_submission();
        Ack::parse(&serde_json::to_vec(&serde_json::json!({
            "protocolVersion":"loom.surface.v1","instanceId":event.binding().instance(),
            "eventId":event.id(),"requestId":format!("request:{index}"),"accepted":true,"status":"succeeded"
        })).unwrap(), event).unwrap()
    }).collect();
    Outcome {
        input: Input {
            owner,
            events,
            revision: 1,
        },
        delivery: Ok(()),
        submission: Ok(BatchSubmission {
            acknowledgements,
            stop: BatchStop::Complete,
        }),
        observation: Some(Ok(Observation {
            items: [2, 3]
                .into_iter()
                .map(|revision| ObservedItem {
                    decision: None,
                    result: Ok(revision),
                    commit: None,
                    confirmation: None,
                })
                .collect(),
            snapshot: Some(Snapshot {
                resources: crate::surface_resource_snapshot::ResourceSnapshot::empty_for_test(
                    window.binding(),
                    3,
                ),
                revision: 3,
                commit: None,
                scene: Scene::parse(br#"{"id":"batch","type":"text"}"#).unwrap(),
            }),
        })),
    }
}
#[test]
fn complete_batch_updates_once_and_keeps_all_non_scene_evidence() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    let mut outcome = batch(&mut window);
    let raw = super::resource_delivery::image_raw(window.binding(), 3, "lease:batch");
    let snapshot = outcome
        .observation
        .as_mut()
        .unwrap()
        .as_mut()
        .unwrap()
        .snapshot
        .as_mut()
        .unwrap();
    snapshot.scene = super::resource_delivery::image_scene(&raw);
    snapshot.resources =
        crate::surface_resource_snapshot::ResourceSnapshot::new(&raw, window.binding(), 3).unwrap();
    let mut windows = crate::desktop::surface_windows::Windows::default();
    let outcome = windows.accept_batch(outcome, 0).err().unwrap();
    assert!(window.lifecycle().is_none());
    let receipt = window
        .accept_batch(*outcome, 0)
        .unwrap_or_else(|_| panic!("batch rejected"));
    assert!(matches!(receipt.presentation, Ok(Some(3))));
    let records = crate::surface_event_record::Record::from_batch_receipt(&receipt).unwrap();
    let bytes = crate::surface_event_record::encode(&records).unwrap();
    assert_eq!(
        crate::surface_event_record::decode(&bytes).unwrap(),
        records
    );
    assert_eq!(
        records[0].convergence(),
        &crate::surface_event_record::Stage::Succeeded(2)
    );
    assert_eq!(
        records[0].presentation(),
        &crate::surface_event_record::Stage::NotStarted
    );
    assert_eq!(
        records[1].presentation(),
        &crate::surface_event_record::Stage::Succeeded(3)
    );
    assert_eq!(window.snapshot_revision(), 3);
    assert_eq!(
        window
            .image_resource(&crate::surface_resource_snapshot::tests::id(), 0)
            .unwrap()
            .id(),
        "lease:batch"
    );
    assert_eq!(receipt.events.len(), 2);
    assert_eq!(receipt.submission.unwrap().acknowledgements.len(), 2);
    let observation = receipt.observation.unwrap().unwrap();
    assert!(observation.snapshot.is_none());
    assert_eq!(observation.items.len(), 2);
    assert!(window.lifecycle().unwrap().confirmed_active());
}
#[test]
fn batch_metadata_rejects_missing_or_reordered_members() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    let outcome = batch(&mut window);
    let mut records = crate::surface_event_record::Record::from_batch_outcome(&outcome).unwrap();
    assert!(crate::surface_event_record::encode(&records[..1]).is_err());
    records.swap(0, 1);
    assert!(crate::surface_event_record::encode(&records).is_err());
    records.swap(0, 1);
    let bytes = crate::surface_event_record::encode(&records).unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    json["records"][1]["batch"]["index"] = 0.into();
    assert!(crate::surface_event_record::decode(&serde_json::to_vec(&json).unwrap()).is_err());
}
#[test]
fn batch_metadata_keeps_unattempted_tail_without_fabricating_ack_or_convergence() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    let mut outcome = batch(&mut window);
    outcome.input.events[1] = event(&window);
    outcome
        .submission
        .as_mut()
        .unwrap()
        .acknowledgements
        .truncate(1);
    let observation = outcome.observation.as_mut().unwrap().as_mut().unwrap();
    observation.items.truncate(1);
    observation.snapshot = None;
    let records = crate::surface_event_record::Record::from_batch_outcome(&outcome).unwrap();
    assert!(records[0].attempted());
    assert!(!records[1].attempted());
    assert_eq!(records[1].status(), None);
    assert_eq!(
        records[1].convergence(),
        &crate::surface_event_record::Stage::NotStarted
    );
    assert!(crate::surface_event_record::encode(&records).is_ok());
}
#[test]
fn partial_batch_or_invalid_revision_or_revoked_delivery_never_replaces_scene() {
    for mode in 0..4 {
        let _com = Apartment::new();
        let mut grant = grant();
        let mut window = active_window(&mut grant);
        let mut outcome = batch(&mut window);
        match mode {
            0 => {
                outcome.submission.as_mut().unwrap().acknowledgements.pop();
            }
            1 => {
                outcome.submission.as_mut().unwrap().stop = BatchStop::Confirmation;
            }
            2 => {
                outcome
                    .observation
                    .as_mut()
                    .unwrap()
                    .as_mut()
                    .unwrap()
                    .snapshot
                    .as_mut()
                    .unwrap()
                    .revision = 4;
            }
            _ => {
                drop(grant);
            }
        }
        let receipt = window
            .accept_batch(outcome, 0)
            .unwrap_or_else(|_| panic!("owner rejected"));
        assert!(receipt.presentation.is_err());
        assert_eq!(window.snapshot_revision(), 1);
        assert_eq!(receipt.events.len(), 2);
        assert_eq!(receipt.observation.unwrap().unwrap().items.len(), 2);
        assert!(window.lifecycle().unwrap().confirmed_active());
    }
}
#[test]
fn reordered_batch_result_returns_entire_outcome_and_keeps_pending_owner() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    let mut outcome = batch(&mut window);
    outcome.input.events.swap(0, 1);
    let mut outcome = window.accept_batch(outcome, 0).err().unwrap();
    assert!(window.lifecycle().is_none());
    assert!(outcome
        .observation
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .snapshot
        .is_some());
    outcome.input.events.swap(0, 1);
    assert!(window.accept_batch(*outcome, 0).is_ok());
}
