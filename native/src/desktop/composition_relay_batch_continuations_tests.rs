//! 接续集合的完整预算与退出收割；使用已撤权租约，不访问网络。
use super::*;
use crate::desktop::surface_authorized_window::BatchContinuationInput;
#[test]
fn explicit_resume_selection_and_cancel_keep_original_batch_order() {
    let mut sessions = Sessions::default();
    let first = input().receipt;
    let origin = first.events[0].binding().origin().to_owned();
    let second = input().receipt;
    let id = second.events[0].id().to_owned();
    sessions.batch_receipts.extend([first, second]);
    assert_eq!(
        sessions.pending_batch_resume_for(&origin, |receipt| receipt.events[0].id() == id),
        Some(id.as_str())
    );
    assert!(!sessions
        .dispatch_batch_resume(&id, |receipt| BatchResumeAction::Keep(Box::new(receipt)))
        .unwrap());
    assert_eq!(sessions.batch_receipts[1].events[0].id(), id);
    assert_eq!(sessions.event_backlog(), 4);
    assert!(!sessions
        .dispatch_batch_resume("missing", |_| panic!("missing batch"))
        .unwrap());
}
#[test]
fn explicit_resume_transfers_existing_receipt_without_double_counting() {
    let mut sessions = Sessions::default();
    let receipt = input().receipt;
    let id = receipt.events[0].id().to_owned();
    sessions.batch_receipts.push(receipt);
    assert!(sessions
        .dispatch_batch_resume(&id, |receipt| BatchResumeAction::Start(
            surface_config(),
            revoked(),
            Box::new(BatchContinuationInput {
                owner: surface_owner(),
                revision: 3,
                receipt,
                next: 1,
            }),
        ))
        .unwrap());
    assert!(sessions.batch_receipts.is_empty());
    assert_eq!(sessions.event_backlog(), 2);
    assert!(drain(&mut sessions).is_empty());
    let outcome = sessions.take_surface_batch_outcome().unwrap();
    assert_eq!(outcome.input.events[0].id(), id);
    assert!(!outcome.input.events[1].attempted());
}
#[test]
fn resume_start_failure_preserves_owner_and_full_batch_for_delivery() {
    let mut sessions = Sessions::default();
    let owner = surface_owner();
    let was_active = owner.confirmed_active();
    let receipt = input().receipt;
    let id = receipt.events[0].id().to_owned();
    sessions.batch_receipts.push(receipt);
    let error = sessions
        .dispatch_batch_resume(&id, |receipt| {
            BatchResumeAction::Start(
                surface_config(),
                revoked(),
                Box::new(BatchContinuationInput {
                    owner,
                    revision: 3,
                    receipt,
                    next: 9,
                }),
            )
        })
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    assert_eq!(sessions.event_backlog(), 2);
    assert_eq!(sessions.wait_handles().count(), 0);
    let outcome = sessions.take_surface_batch_outcome().unwrap();
    assert_eq!(outcome.input.owner.confirmed_active(), was_active);
    assert_eq!(outcome.input.events[0].id(), id);
    assert!(!outcome.input.events[1].attempted());
}
fn input() -> BatchContinuationInput {
    let mut receipt = pending_batch();
    let observed = receipt.observation.as_mut().unwrap().as_mut().unwrap();
    let prompt = observed.items[0].confirmation.take().unwrap().unwrap();
    let event = &receipt.events[0];
    let submitted = receipt.submission.as_mut().unwrap();
    let mut decision = Decision::new(
        prompt,
        event,
        &submitted.acknowledgements[0],
        Choice::Approve,
        0,
    )
    .unwrap_or_else(|_| panic!("prepare"));
    decision.begin_submission();
    decision.acknowledge();
    submitted.acknowledgements[0] = crate::surface_action_ack::Ack::parse(&serde_json::to_vec(&serde_json::json!({
        "protocolVersion":"loom.surface.v1","instanceId":event.binding().instance(),"eventId":event.id(),
        "requestId":"request:test","accepted":true,"status":"succeeded"
    })).unwrap(), event).unwrap();
    observed.items[0].result = Ok(3);
    observed.items[0].decision = Some(crate::surface_decision_runtime::Evidence {
        decision,
        revision: 3,
        submission: Ok(()),
    });
    BatchContinuationInput {
        owner: surface_owner(),
        revision: 3,
        receipt,
        next: 1,
    }
}
#[test]
fn continuation_counts_full_batch_and_shutdown_keeps_recovery_evidence() {
    let mut sessions = Sessions::default();
    sessions
        .start_surface_batch_continuation(surface_config(), revoked(), input())
        .unwrap_or_else(|e| panic!("{}", e.error));
    assert_eq!(sessions.event_backlog(), 2);
    assert_eq!(sessions.occupied(), 1);
    assert_eq!(sessions.wait_handles().count(), 1);
    assert_eq!(sessions.surface_shutdown_handles().count(), 1);
    assert!(sessions.surface_cleanup_pending());
    assert!(sessions.surface_recovery_bundle().is_err());
    sessions.close();
    assert!(drain(&mut sessions).is_empty());
    let (attachments, records) = sessions.surface_recovery_bundle().unwrap();
    assert_eq!(attachments.len(), 1);
    assert_eq!(records.len(), 2);
    assert!(records[0].decision().unwrap().confirmed());
    assert!(crate::surface_event_record::encode(&records).is_ok());
    let outcome = sessions.take_surface_batch_outcome().unwrap();
    assert!(outcome.delivery.is_err());
    assert!(!outcome.input.events[1].attempted());
    assert!(!sessions.surface_cleanup_pending());
}
#[test]
fn continuation_start_failure_returns_prefix_and_unattempted_tail() {
    for closed in [false, true] {
        let mut sessions = Sessions::default();
        if closed {
            sessions.close();
        } else {
            for _ in 0..MAX_SESSIONS {
                sessions.event_outcomes.push(completed());
            }
        }
        let error = sessions
            .start_surface_batch_continuation(surface_config(), revoked(), input())
            .err()
            .unwrap();
        assert_eq!(
            error.error.kind(),
            if closed {
                io::ErrorKind::BrokenPipe
            } else {
                io::ErrorKind::WouldBlock
            }
        );
        assert_eq!(error.input.receipt.continuation_boundary(), Some(1));
        assert_eq!(sessions.wait_handles().count(), 0);
    }
}
#[test]
fn mixed_batch_jobs_preallocate_all_completion_slots() {
    let mut sessions = Sessions::default();
    sessions
        .start_surface_batch_continuation(surface_config(), revoked(), input())
        .unwrap_or_else(|e| panic!("{}", e.error));
    sessions
        .start_surface_batch(
            surface_config(),
            revoked(),
            crate::surface_event_batch_runtime::Input {
                owner: surface_owner(),
                revision: 3,
                events: vec![event()],
            },
        )
        .unwrap_or_else(|e| panic!("{}", e.error));
    sessions
        .start_surface_batch_decision(surface_config(), revoked(), batch_input())
        .unwrap_or_else(|e| panic!("{}", e.error));
    assert!(sessions.batch_outcomes.capacity() >= 3);
    assert_eq!(sessions.event_backlog(), 5);
    sessions.close();
    assert!(drain(&mut sessions).is_empty());
    assert_eq!(sessions.batch_outcomes.len(), 3);
    assert_eq!(sessions.event_backlog(), 5);
}
