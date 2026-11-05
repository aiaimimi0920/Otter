use super::*;
#[path = "composition_relay_batch_decisions_tests.rs"]
mod batch_decisions;
use crate::{
    surface_confirmation::{Choice, Decision, Request as Prompt},
    surface_decision_runtime::{Error as DecisionError, Input, Outcome as DecisionOutcome},
};
fn input() -> Input {
    let outcome = completed();
    let event = outcome.event;
    let ack = crate::surface_action_ack::Ack::parse(&serde_json::to_vec(&serde_json::json!({
        "protocolVersion":"loom.surface.v1","instanceId":event.binding().instance(),"eventId":event.id(),
        "requestId":"request:test","accepted":true,"status":"awaiting_confirmation"
    })).unwrap(), &event).unwrap();
    let action: serde_json::Value = serde_json::from_slice(event.bytes()).unwrap();
    let raw = serde_json::value::RawValue::from_string(serde_json::json!({
        "protocolVersion":"loom.surface.v1","confirmationId":"confirmation:test","instanceId":event.binding().instance(),
        "attachmentId":event.binding().attachment(),"deviceId":event.binding().device(),"hookNodeId":event.binding().node(),
        "eventId":event.id(),"requestId":ack.request_id(),"actionId":action["action"],"risk":"high","expiresAtMs":u64::MAX
    }).to_string()).unwrap();
    let prompt = Prompt::parse_matching(&raw, &event, &ack).unwrap().unwrap();
    let decision = Decision::new(prompt, &event, &ack, Choice::Reject, 0)
        .unwrap_or_else(|_| panic!("prepare"));
    Input {
        owner: outcome.owner,
        event,
        ack,
        decision,
        revision: 3,
    }
}
fn pending_receipt() -> crate::desktop::surface_authorized_window::EventReceipt {
    pending_receipt_until(u64::MAX)
}
fn pending_receipt_until(expires: u64) -> crate::desktop::surface_authorized_window::EventReceipt {
    let mut receipt = receipt(
        DecisionOutcome {
            input: input(),
            submission: Err(DecisionError::NotStarted),
            convergence: None,
            delivery: Ok(()),
        }
        .into_event(),
    );
    receipt.decision = None;
    let event = &receipt.event;
    let ack = receipt.result.as_ref().unwrap();
    let action: serde_json::Value = serde_json::from_slice(event.bytes()).unwrap();
    let raw = serde_json::value::RawValue::from_string(
        serde_json::json!({
            "protocolVersion":"loom.surface.v1","confirmationId":"confirmation:test",
            "instanceId":event.binding().instance(),"attachmentId":event.binding().attachment(),
            "deviceId":event.binding().device(),"hookNodeId":event.binding().node(),
            "eventId":event.id(),"requestId":ack.request_id(),"actionId":action["action"],
        "risk":"high","expiresAtMs":expires
        })
        .to_string(),
    )
    .unwrap();
    receipt.confirmation = Some(Ok(Prompt::parse_matching(&raw, event, ack)
        .unwrap()
        .unwrap()));
    receipt
}
#[test]
fn expired_or_denied_receipts_do_not_hide_later_valid_confirmation() {
    let mut sessions = Sessions::default();
    let expired = pending_receipt_until(50);
    let origin = expired.event.binding().origin().to_owned();
    let mut denied = pending_receipt();
    denied.delivery = Err(crate::device_authorization_lease::Error::Revoked);
    let valid = pending_receipt();
    let id = valid.event.id().to_owned();
    sessions.event_receipts.extend([expired, denied, valid]);
    assert_eq!(
        sessions.pending_surface_confirmation(&origin, 50),
        Some(id.as_str())
    );
    assert!(sessions
        .pending_surface_confirmation("https://other.example", 50)
        .is_none());
    assert_eq!(sessions.event_receipts.len(), 3);
    assert!(sessions
        .pending_surface_confirmation(&origin, u64::MAX)
        .is_none());
}
#[test]
fn stale_menu_selection_cannot_enter_confirmation_callback() {
    let mut sessions = Sessions::default();
    let expired = pending_receipt_until(1);
    let id = expired.event.id().to_owned();
    sessions.event_receipts.push(expired);
    assert!(!sessions
        .dispatch_surface_confirmation(&id, |_| panic!("expired UI"))
        .unwrap());
    assert_eq!(sessions.event_receipts.len(), 1);
    assert!(sessions.event_receipts[0]
        .confirmation
        .as_ref()
        .unwrap()
        .is_ok());
    let mut presented = pending_receipt();
    let id = presented.event.id().to_owned();
    presented.presentation = Ok(Some(3));
    sessions.event_receipts.push(presented);
    assert!(!sessions
        .dispatch_surface_confirmation(&id, |_| panic!("presented UI"))
        .unwrap());
    assert_eq!(sessions.event_receipts.len(), 2);
}
#[test]
fn unavailable_window_does_not_block_later_confirmation_or_consume_evidence() {
    let mut sessions = Sessions::default();
    let first = pending_receipt();
    let origin = first.event.binding().origin().to_owned();
    let first_id = first.event.id().to_owned();
    let second = pending_receipt();
    let second_id = second.event.id().to_owned();
    sessions
        .event_receipts
        .extend([pending_receipt_until(1), first, second]);
    let mut inspected = Vec::new();
    assert_eq!(
        sessions.pending_surface_confirmation_for(&origin, 2, |receipt| {
            inspected.push(receipt.event.id().to_owned());
            receipt.event.id() == second_id
        }),
        Some(second_id.as_str())
    );
    assert_eq!(inspected, [first_id, second_id]);
    assert_eq!(sessions.event_receipts.len(), 3);
    assert!(sessions
        .pending_surface_confirmation_for(&origin, 2, |_| false)
        .is_none());
    assert!(sessions.event_receipts.iter().all(|receipt| receipt
        .confirmation
        .as_ref()
        .unwrap()
        .is_ok()));
}
#[test]
fn selected_confirmation_cancel_keeps_receipt_and_capacity_gate_precedes_callback() {
    let mut sessions = Sessions::default();
    let pending = pending_receipt();
    let id = pending.event.id().to_owned();
    sessions.event_receipts.push(pending);
    assert!(!sessions
        .dispatch_surface_confirmation("missing", |_| panic!("unexpected UI"))
        .unwrap());
    assert!(!sessions
        .dispatch_surface_confirmation(&id, ConfirmationAction::Keep)
        .unwrap());
    assert_eq!(sessions.event_backlog(), 1);
    assert_eq!(sessions.event_receipts[0].event.id(), id);
    assert!(sessions.event_receipts[0]
        .confirmation
        .as_ref()
        .unwrap()
        .is_ok());
    for _ in 0..MAX_SESSIONS {
        sessions.event_outcomes.push(completed());
    }
    assert_eq!(
        sessions
            .dispatch_surface_confirmation(&id, |_| panic!("full UI"))
            .unwrap_err()
            .kind(),
        io::ErrorKind::WouldBlock
    );
    sessions.close();
    assert_eq!(
        sessions
            .dispatch_surface_confirmation(&id, |_| panic!("closed UI"))
            .unwrap_err()
            .kind(),
        io::ErrorKind::BrokenPipe
    );
    assert_eq!(sessions.event_receipts.len(), 1);
}
#[test]
fn decisions_use_application_wait_close_and_shared_capacity_without_losing_inputs() {
    let mut sessions = Sessions::default();
    for _ in 0..MAX_SESSIONS {
        sessions
            .start_surface_decision(surface_config(), revoked(), input())
            .unwrap_or_else(|e| panic!("{}", e.error));
    }
    assert_eq!(sessions.wait_handles().count(), MAX_SESSIONS);
    assert_eq!(sessions.surface_shutdown_handles().count(), MAX_SESSIONS);
    assert!(sessions.surface_recovery_bundle().is_err());
    let full = sessions
        .start_surface_decision(surface_config(), revoked(), input())
        .err()
        .unwrap();
    assert_eq!(full.error.kind(), io::ErrorKind::WouldBlock);
    assert!(!full.input.decision.attempted());
    sessions.close();
    while !sessions.finished() {
        assert!(sessions.poll_surfaces().is_empty());
        if sessions.finished() {
            break;
        }
        let handles: Vec<_> = sessions
            .wait_handles()
            .map(|h| HANDLE(h.as_raw_handle()))
            .collect();
        let result = unsafe { WaitForMultipleObjects(&handles, false, 5000) };
        assert!(result.0 < WAIT_OBJECT_0.0 + handles.len() as u32);
    }
    assert_eq!(sessions.occupied(), MAX_SESSIONS);
    let (attachments, events) = sessions.surface_recovery_bundle().unwrap();
    assert_eq!(attachments.len(), MAX_SESSIONS);
    assert_eq!(events.len(), MAX_SESSIONS);
    assert!(crate::surface_event_record::encode(&events).is_ok());
    for _ in 0..MAX_SESSIONS {
        let outcome = sessions.take_surface_event_outcome().unwrap();
        let evidence = outcome.decision.unwrap();
        assert_eq!(evidence.submission, Err(DecisionError::Stopped));
        assert!(outcome.delivery.is_err());
        assert!(!evidence.decision.attempted());
    }
    assert!(!sessions.surface_cleanup_pending());
    let closed = sessions
        .start_surface_decision(surface_config(), revoked(), input())
        .err()
        .unwrap();
    assert_eq!(closed.error.kind(), io::ErrorKind::BrokenPipe);
    assert_eq!(closed.input.decision.choice(), Choice::Reject);
}

#[test]
fn confirmation_start_failure_keeps_original_event_choice_and_owner_in_completion_queue() {
    let slots: Vec<_> = (0..MAX_SESSIONS)
        .map(|_| crate::live_relay_control_runtime::Slot::acquire().unwrap())
        .collect();
    let mut sessions = Sessions::default();
    let pending = pending_receipt();
    let id = pending.event.id().to_owned();
    sessions.event_receipts.push(pending);
    let error = sessions
        .dispatch_surface_confirmation(&id, |mut receipt| {
            let ack = receipt.result.unwrap();
            let prompt = receipt.confirmation.take().unwrap().unwrap();
            let decision = Decision::new(prompt, &receipt.event, &ack, Choice::Reject, 0)
                .unwrap_or_else(|_| panic!("prepare"));
            ConfirmationAction::Start(
                surface_config(),
                revoked(),
                Input {
                    owner: input().owner,
                    event: receipt.event,
                    ack,
                    decision,
                    revision: 3,
                },
            )
        })
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
    assert!(sessions.event_receipts.is_empty());
    assert_eq!(sessions.event_backlog(), 1);
    assert_eq!(sessions.wait_handles().count(), 0);
    let (_, records) = sessions.surface_recovery_bundle().unwrap();
    assert_eq!(records.len(), 1);
    let outcome = sessions.take_surface_event_outcome().unwrap();
    assert_eq!(outcome.event.id(), id);
    assert_eq!(
        outcome.result.unwrap().status(),
        crate::surface_action_ack::Status::AwaitingConfirmation
    );
    let evidence = outcome.decision.unwrap();
    assert_eq!(evidence.submission, Err(DecisionError::NotStarted));
    assert_eq!(evidence.decision.choice(), Choice::Reject);
    assert!(!evidence.decision.attempted());
    drop(slots);
}
#[test]
fn cached_decisions_keep_recovery_evidence_when_delivery_is_rejected_or_cancelled() {
    let mut sessions = Sessions::default();
    sessions.event_outcomes.push(
        DecisionOutcome {
            input: input(),
            submission: Err(DecisionError::Stopped),
            convergence: None,
            delivery: Ok(()),
        }
        .into_event(),
    );
    assert!(!sessions.deliver_surface_event(|outcome| Err(Box::new(outcome))));
    assert!(sessions.surface_recovery_records().is_err());
    let (_, records) = sessions.surface_recovery_bundle().unwrap();
    let decision = records[0].decision().unwrap();
    assert_eq!(decision.confirmation_id(), "confirmation:test");
    assert!(!decision.approved() && !decision.attempted() && !decision.confirmed());
    sessions.cancel_surface_requests();
    assert!(sessions.deliver_surface_event(|outcome| {
        assert!(outcome.delivery.is_err());
        assert_eq!(
            outcome.result.unwrap().status(),
            crate::surface_action_ack::Status::AwaitingConfirmation
        );
        assert!(!outcome.decision.unwrap().decision.attempted());
        Ok(())
    }));
    assert!(!sessions.surface_cleanup_pending());
}
#[test]
fn receipt_backlog_also_prevents_new_decision_tasks() {
    let mut sessions = Sessions::default();
    for _ in 0..32 {
        sessions.event_receipts.push(receipt(completed()));
    }
    let error = sessions
        .start_surface_decision(surface_config(), revoked(), input())
        .err()
        .unwrap();
    assert_eq!(error.error.kind(), io::ErrorKind::WouldBlock);
    assert_eq!(sessions.wait_handles().count(), 0);
    assert_eq!(error.input.decision.request().id(), "confirmation:test");
}

#[test]
fn decision_evidence_survives_unified_settlement_and_receipt_recovery() {
    let mut sessions = Sessions::default();
    sessions.event_outcomes.push(
        DecisionOutcome {
            input: input(),
            submission: Err(DecisionError::Stopped),
            convergence: None,
            delivery: Ok(()),
        }
        .into_event(),
    );
    assert_eq!(
        sessions
            .settle_surface_events(|outcome| Ok(receipt(outcome)))
            .unwrap(),
        1
    );
    assert_eq!(sessions.occupied(), 0);
    assert_eq!(sessions.event_backlog(), 1);
    assert!(sessions.surface_cleanup_pending());
    let (attachments, records) = sessions.surface_recovery_bundle().unwrap();
    assert!(attachments.is_empty());
    assert!(records[0].decision().is_some());
    assert!(crate::surface_event_record::encode(&records).is_ok());
    let receipt = sessions.take_surface_event_receipt().unwrap();
    assert_eq!(
        receipt.decision.unwrap().submission,
        Err(DecisionError::Stopped)
    );
    assert!(!sessions.surface_cleanup_pending());
}
