//! 批次准备保留其他事件证据；允许因部分完成而未呈现的有效确认项。
use super::*;
use crate::surface_confirmation::{Choice, Request};
#[path = "surface_batch_continuation_tests.rs"]
mod continuation;
#[test]
fn batch_dialog_choice_and_cancel_preserve_prompt_and_owner_until_explicit_preparation() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    let receipt = waiting(&mut window);
    for choice in [None, Some(Choice::Approve), Some(Choice::Reject)] {
        assert_eq!(
            window
                .prompt_batch_choice_with(
                    &receipt,
                    0,
                    || Ok(0),
                    |text| {
                        assert!(text.contains("风险：高"));
                        Ok(choice)
                    }
                )
                .unwrap(),
            choice
        );
        assert!(receipt.confirmation_ready(0, 0));
        assert_eq!(receipt.events.len(), 2);
        assert!(window.lifecycle().unwrap().confirmed_active());
    }
}
#[test]
fn batch_dialog_rechecks_expiry_and_revocation_after_choice() {
    let _com = Apartment::new();
    for revoke in [false, true] {
        let mut grant = grant();
        let mut window = active_window(&mut grant);
        let receipt = waiting(&mut window);
        let mut calls = 0;
        assert!(window
            .prompt_batch_choice_with(
                &receipt,
                0,
                || {
                    calls += 1;
                    Ok(if calls == 1 || revoke { 0 } else { 50 })
                },
                |_| {
                    if revoke {
                        drop(grant);
                    }
                    Ok(Some(Choice::Approve))
                }
            )
            .is_err());
        assert!(receipt.confirmation_ready(0, 0));
        assert!(window.lifecycle().unwrap().confirmed_active());
        if revoke {
            assert!(!window.is_alive());
        }
    }
}
fn waiting(
    window: &mut AuthorizedWindow,
) -> crate::desktop::surface_authorized_window::BatchReceipt {
    let mut outcome = batch(window);
    let event = &outcome.input.events[0];
    let ack = Ack::parse(&serde_json::to_vec(&serde_json::json!({
        "protocolVersion":"loom.surface.v1","instanceId":event.binding().instance(),"eventId":event.id(),
        "requestId":"request:0","accepted":true,"status":"awaiting_confirmation"
    })).unwrap(), event).unwrap();
    let action: serde_json::Value = serde_json::from_slice(event.bytes()).unwrap();
    let raw = serde_json::value::RawValue::from_string(serde_json::json!({
        "protocolVersion":"loom.surface.v1","confirmationId":"confirmation:batch",
        "instanceId":event.binding().instance(),"attachmentId":event.binding().attachment(),
        "deviceId":event.binding().device(),"hookNodeId":event.binding().node(),"eventId":event.id(),
        "requestId":"request:0","actionId":action["action"],"risk":"high","expiresAtMs":50
    }).to_string()).unwrap();
    let prompt = Request::parse_matching(&raw, event, &ack).unwrap().unwrap();
    outcome.submission.as_mut().unwrap().acknowledgements[0] = ack;
    let item = &mut outcome
        .observation
        .as_mut()
        .unwrap()
        .as_mut()
        .unwrap()
        .items[0];
    item.result = Err(
        crate::surface_event_batch_runtime::ObservationError::Convergence(
            crate::live_relay_http::ConvergenceError::ConfirmationRequired,
        ),
    );
    item.confirmation = Some(Ok(prompt));
    window
        .accept_batch(outcome, 0)
        .unwrap_or_else(|_| panic!("batch rejected"))
}
#[test]
fn preparing_one_batch_choice_preserves_other_events_and_does_not_take_owner() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    let receipt = waiting(&mut window);
    assert!(matches!(
        receipt.presentation,
        Err(PresentationError::Evidence)
    ));
    assert_eq!(
        window.batch_confirmation_for(&receipt, 0, 49).unwrap().id(),
        "confirmation:batch"
    );
    let failed = window
        .prepare_batch_decision(receipt, 2, Choice::Reject, 0)
        .err()
        .unwrap();
    assert_eq!(failed.receipt.events.len(), 2);
    let failed = window
        .prepare_batch_decision(failed.receipt, 0, Choice::Reject, 50)
        .err()
        .unwrap();
    assert!(window.batch_confirmation_for(&failed.receipt, 0, 0).is_ok());
    let prepared = window
        .prepare_batch_decision(failed.receipt, 0, Choice::Reject, 0)
        .unwrap_or_else(|failure| panic!("{}", failure.error));
    assert_eq!(prepared.index, 0);
    assert_eq!(prepared.decision.choice(), Choice::Reject);
    assert!(!prepared.decision.attempted());
    assert_eq!(prepared.receipt.events.len(), 2);
    assert_eq!(
        prepared
            .receipt
            .submission
            .as_ref()
            .unwrap()
            .acknowledgements
            .len(),
        2
    );
    let items = &prepared
        .receipt
        .observation
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .items;
    assert!(items[0].confirmation.is_none());
    assert_eq!(items[1].result, Ok(3));
    assert!(window
        .batch_confirmation_for(&prepared.receipt, 0, 0)
        .is_err());
    assert!(window.lifecycle().unwrap().confirmed_active());
    assert_eq!(window.snapshot_revision(), 1);
    let failed = window
        .begin_prepared_batch_decision(prepared, 50)
        .err()
        .unwrap();
    assert_eq!(failed.prepared.receipt.events.len(), 2);
    assert!(window.lifecycle().unwrap().confirmed_active());
    let input = window
        .begin_prepared_batch_decision(failed.prepared, 0)
        .unwrap_or_else(|failure| panic!("{}", failure.error));
    assert_eq!(input.revision, 1);
    assert!(window.lifecycle().is_none());
    assert!(window
        .begin_event(&input.prepared.receipt.events[0], 0)
        .unwrap()
        .is_none());
    assert!(window
        .finish_batch(&input.prepared.receipt.events, input.owner)
        .is_ok());
    assert!(window.lifecycle().unwrap().confirmed_active());
    assert_eq!(input.prepared.decision.request().id(), "confirmation:batch");
}
#[test]
fn batch_choice_cannot_transfer_after_revocation_or_network_attempt() {
    for revoke in [false, true] {
        let _com = Apartment::new();
        let mut grant = grant();
        let mut window = active_window(&mut grant);
        let receipt = waiting(&mut window);
        let mut prepared = window
            .prepare_batch_decision(receipt, 0, Choice::Approve, 0)
            .unwrap_or_else(|failure| panic!("{}", failure.error));
        if revoke {
            drop(grant);
        } else {
            prepared.decision.begin_submission();
        }
        let failed = window
            .begin_prepared_batch_decision(prepared, 0)
            .err()
            .unwrap();
        assert_eq!(failed.prepared.receipt.events.len(), 2);
        assert_eq!(failed.prepared.decision.attempted(), !revoke);
        assert!(window.lifecycle().unwrap().confirmed_active());
    }
}
#[test]
fn revocation_before_batch_choice_keeps_prompt_and_closes_window() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    let receipt = waiting(&mut window);
    drop(grant);
    let failed = window
        .prepare_batch_decision(receipt, 0, Choice::Approve, 0)
        .err()
        .unwrap();
    assert!(!window.is_alive());
    assert!(failed.receipt.observation.unwrap().unwrap().items[0]
        .confirmation
        .as_ref()
        .unwrap()
        .is_ok());
    assert!(window.lifecycle().unwrap().confirmed_active());
}
#[test]
fn confirmed_batch_decision_merges_evidence_and_updates_scene_through_batch_delivery() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    let receipt = waiting(&mut window);
    let prepared = window
        .prepare_batch_decision(receipt, 0, Choice::Approve, 0)
        .unwrap_or_else(|_| panic!("prepare"));
    let mut input = window
        .begin_prepared_batch_decision(prepared, 0)
        .unwrap_or_else(|_| panic!("transfer"));
    input.prepared.decision.begin_submission();
    input.prepared.decision.acknowledge();
    let event = &input.prepared.receipt.events[0];
    let ack = Ack::parse(&serde_json::to_vec(&serde_json::json!({
        "protocolVersion":"loom.surface.v1","instanceId":event.binding().instance(),"eventId":event.id(),
        "requestId":"request:0","accepted":true,"status":"succeeded"
    })).unwrap(), event).unwrap();
    input
        .prepared
        .receipt
        .submission
        .as_mut()
        .unwrap()
        .acknowledgements[0] = ack;
    let outcome = crate::surface_batch_decision_runtime::Outcome {
        input,
        submission: Ok(()),
        delivery: Ok(()),
        convergence: Some(Ok(Snapshot {
            resources: crate::surface_resource_snapshot::ResourceSnapshot::empty_for_test(
                window.binding(),
                4,
            ),
            revision: 4,
            commit: None,
            scene: Scene::parse(br#"{"id":"confirmed","type":"text"}"#).unwrap(),
        })),
    };
    let merged = outcome.into_batch().unwrap_or_else(|_| panic!("merge"));
    let receipt = window
        .accept_batch(merged, 0)
        .unwrap_or_else(|_| panic!("delivery"));
    assert!(matches!(receipt.presentation, Ok(Some(4))));
    assert_eq!(
        receipt
            .observation
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .items[1]
            .result,
        Ok(3)
    );
    let records = crate::surface_event_record::Record::from_batch_receipt(&receipt).unwrap();
    assert!(records[0].decision().unwrap().confirmed());
    assert!(records[0].decision().unwrap().approved());
    assert!(records[1].decision().is_none());
    assert!(crate::surface_event_record::encode(&records).is_ok());
}
