//! 已撤权批次决定通过真实线程进入应用结果队列；不访问网络或自动重放。
use super::*;
#[path = "composition_relay_batch_continuations_tests.rs"]
mod continuations;
use crate::{
    desktop::surface_authorized_window::{BatchDecisionInput, BatchReceipt, PreparedBatchDecision},
    live_relay_http::{BatchStop, BatchSubmission, ConvergenceError},
    surface_event_batch_runtime::{Observation, ObservationError, ObservedItem},
};
fn batch_input() -> BatchDecisionInput {
    let input = input();
    BatchDecisionInput {
        owner: input.owner,
        revision: input.revision,
        prepared: PreparedBatchDecision {
            index: 0,
            decision: input.decision,
            receipt: BatchReceipt {
                events: vec![input.event, event()],
                revision: input.revision,
                submission: Ok(BatchSubmission {
                    acknowledgements: vec![input.ack],
                    stop: BatchStop::Confirmation,
                }),
                observation: Some(Ok(Observation {
                    snapshot: None,
                    items: vec![ObservedItem {
                        result: Err(ObservationError::Convergence(
                            ConvergenceError::ConfirmationRequired,
                        )),
                        commit: None,
                        confirmation: None,
                        decision: None,
                    }],
                })),
                delivery: Ok(()),
                presentation: Ok(None),
            },
        },
    }
}
fn pending_batch() -> BatchReceipt {
    let receipt = pending_receipt();
    BatchReceipt {
        events: vec![receipt.event, event()],
        revision: 3,
        submission: Ok(BatchSubmission {
            acknowledgements: vec![receipt.result.unwrap()],
            stop: BatchStop::Confirmation,
        }),
        observation: Some(Ok(Observation {
            snapshot: None,
            items: vec![ObservedItem {
                result: Err(ObservationError::Convergence(
                    ConvergenceError::ConfirmationRequired,
                )),
                commit: None,
                confirmation: Some(Ok(receipt.confirmation.unwrap().unwrap())),
                decision: None,
            }],
        })),
        delivery: Ok(()),
        presentation: Ok(None),
    }
}
#[test]
fn batch_selection_skips_unavailable_windows_and_cancel_keeps_entire_receipt() {
    let mut sessions = Sessions::default();
    let first = pending_batch();
    let origin = first.events[0].binding().origin().to_owned();
    let second = pending_batch();
    let id = second.events[0].id().to_owned();
    let tail = second.events[1].id().to_owned();
    sessions.batch_receipts.extend([first, second]);
    assert_eq!(
        sessions.pending_batch_confirmation_for(&origin, 0, |receipt, index| {
            index == 0 && receipt.events[index].id() == id
        }),
        Some(id.as_str())
    );
    assert!(!sessions
        .dispatch_batch_confirmation(&id, |receipt, index| {
            assert_eq!(index, 0);
            assert_eq!(receipt.events[1].id(), tail);
            BatchConfirmationAction::Keep(Box::new(receipt))
        })
        .unwrap());
    assert_eq!(sessions.event_backlog(), 4);
    assert!(sessions.batch_receipts[1].confirmation_ready(0, 0));
    assert!(!sessions
        .dispatch_batch_confirmation("missing", |_, _| panic!("stale selection"))
        .unwrap());
}
#[test]
fn invalid_batch_prompt_or_capacity_never_enters_confirmation_callback() {
    let mut sessions = Sessions::default();
    let mut receipt = pending_batch();
    let id = receipt.events[0].id().to_owned();
    receipt.delivery = Err(crate::device_authorization_lease::Error::Revoked);
    sessions.batch_receipts.push(receipt);
    assert!(!sessions
        .dispatch_batch_confirmation(&id, |_, _| panic!("revoked"))
        .unwrap());
    sessions.batch_receipts[0].delivery = Ok(());
    for _ in 0..MAX_SESSIONS {
        sessions.event_outcomes.push(completed());
    }
    assert_eq!(
        sessions
            .dispatch_batch_confirmation(&id, |_, _| panic!("capacity"))
            .unwrap_err()
            .kind(),
        io::ErrorKind::WouldBlock
    );
    assert!(sessions.batch_receipts[0].confirmation_ready(0, 0));
}
#[test]
fn batch_dispatch_transfers_full_receipt_to_worker_without_double_counting_budget() {
    let mut sessions = Sessions::default();
    let receipt = pending_batch();
    let id = receipt.events[0].id().to_owned();
    sessions.batch_receipts.push(receipt);
    assert!(sessions
        .dispatch_batch_confirmation(&id, |mut receipt, index| {
            let request = receipt
                .observation
                .as_mut()
                .unwrap()
                .as_mut()
                .unwrap()
                .items[index]
                .confirmation
                .take()
                .unwrap()
                .unwrap();
            let ack = &receipt.submission.as_ref().unwrap().acknowledgements[index];
            let decision = Decision::new(request, &receipt.events[index], ack, Choice::Reject, 0)
                .unwrap_or_else(|_| panic!("decision"));
            BatchConfirmationAction::Start(
                surface_config(),
                revoked(),
                Box::new(BatchDecisionInput {
                    owner: surface_owner(),
                    revision: 3,
                    prepared: PreparedBatchDecision {
                        receipt,
                        index,
                        decision,
                    },
                }),
            )
        })
        .unwrap());
    assert_eq!(sessions.event_backlog(), 2);
    assert!(sessions.batch_receipts.is_empty());
    assert!(drain(&mut sessions).is_empty());
    let outcome = sessions.take_surface_batch_outcome().unwrap();
    assert_eq!(outcome.input.events[0].id(), id);
    assert!(!outcome.input.events[1].attempted());
    assert!(!outcome.observation.unwrap().unwrap().items[0]
        .decision
        .as_ref()
        .unwrap()
        .decision
        .attempted());
}
fn drain(sessions: &mut Sessions) -> Vec<(String, surfaces::Error)> {
    let mut errors = Vec::new();
    while !sessions.finished() {
        errors.extend(sessions.poll_surfaces());
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
    errors
}
#[test]
fn batch_decision_shutdown_merges_full_receipt_and_keeps_budget_and_recovery_evidence() {
    let mut sessions = Sessions::default();
    sessions
        .start_surface_batch_decision(surface_config(), revoked(), batch_input())
        .unwrap_or_else(|e| panic!("{}", e.error));
    assert_eq!(sessions.event_backlog(), 2);
    assert_eq!(sessions.occupied(), 1);
    assert_eq!(sessions.wait_handles().count(), 1);
    assert_eq!(sessions.surface_shutdown_handles().count(), 1);
    assert!(sessions.surface_recovery_bundle().is_err());
    sessions.close();
    assert!(drain(&mut sessions).is_empty());
    assert_eq!(sessions.event_backlog(), 2);
    let (attachments, records) = sessions.surface_recovery_bundle().unwrap();
    assert_eq!(attachments.len(), 1);
    assert_eq!(records.len(), 2);
    assert!(!records[0].decision().unwrap().attempted());
    assert!(crate::surface_event_record::encode(&records).is_ok());
    let outcome = sessions.take_surface_batch_outcome().unwrap();
    assert!(outcome.delivery.is_err());
    assert!(!outcome.input.events[1].attempted());
    assert!(!sessions.surface_cleanup_pending());
}
#[test]
fn malformed_decision_result_is_retained_and_cannot_produce_incomplete_exit_bundle() {
    let mut sessions = Sessions::default();
    let mut input = batch_input();
    input.prepared.index = 9;
    sessions
        .start_surface_batch_decision(surface_config(), revoked(), input)
        .unwrap_or_else(|e| panic!("{}", e.error));
    assert!(drain(&mut sessions)
        .iter()
        .any(|(_, error)| matches!(error, surfaces::Error::BatchDecisionMerge)));
    assert_eq!(sessions.event_backlog(), 2);
    assert_eq!(sessions.occupied(), 1);
    assert!(sessions.surface_recovery_bundle().is_err());
    sessions.cancel_surface_requests();
    let rejected = sessions.take_rejected_batch_decision().unwrap();
    assert_eq!(rejected.input.prepared.index, 9);
    assert_eq!(rejected.input.prepared.receipt.events.len(), 2);
    assert!(rejected.delivery.is_err());
    assert!(!sessions.surface_cleanup_pending());
}
#[test]
fn closed_or_full_collection_returns_entire_unattempted_batch_decision() {
    for closed in [false, true] {
        let mut sessions = Sessions::default();
        if closed {
            sessions.close();
        } else {
            for _ in 0..32 {
                sessions.event_outcomes.push(completed());
            }
        }
        let error = sessions
            .start_surface_batch_decision(surface_config(), revoked(), batch_input())
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
        assert_eq!(error.input.prepared.receipt.events.len(), 2);
        assert!(!error.input.prepared.decision.attempted());
        assert_eq!(sessions.wait_handles().count(), 0);
    }
}
