//! 批次应用等待/预算/收据测试：已撤权租约，不访问远端。
use super::*;
use crate::surface_event_batch_runtime::{Error as BatchError, Input, Outcome as BatchOutcome};
#[test]
fn click_dispatch_capacity_gate_precedes_owner_transfer() {
    let mut sessions = Sessions::default();
    for _ in 0..MAX_SESSIONS {
        sessions.event_outcomes.push(completed());
    }
    assert_eq!(
        sessions
            .dispatch_surface_batch(surface_config(), revoked(), |_| panic!(
                "capacity must precede callback"
            ))
            .unwrap_err()
            .kind(),
        io::ErrorKind::WouldBlock
    );
    sessions.close();
    assert_eq!(
        sessions
            .dispatch_surface_batch(surface_config(), revoked(), |_| panic!("closed"))
            .unwrap_err()
            .kind(),
        io::ErrorKind::BrokenPipe
    );
}
#[test]
fn click_dispatch_preserves_failed_start_input_as_unattempted_outcome() {
    let mut sessions = Sessions::default();
    let error = sessions
        .dispatch_surface_batch(surface_config(), revoked(), |limit| {
            assert_eq!(limit, 32);
            Ok(Some(input(0)))
        })
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    let outcome = sessions.take_surface_batch_outcome().unwrap();
    assert!(matches!(outcome.submission, Err(BatchError::NotStarted)));
    assert_eq!(outcome.input.owner.binding(), surface_owner().binding());
    assert!(outcome.observation.is_none());
}
#[test]
fn click_dispatch_without_candidate_starts_no_worker() {
    let mut sessions = Sessions::default();
    assert!(!sessions
        .dispatch_surface_batch(surface_config(), revoked(), |_| Ok(None))
        .unwrap());
    assert_eq!(sessions.event_backlog(), 0);
    assert_eq!(sessions.wait_handles().count(), 0);
}
fn input(count: usize) -> Input {
    Input {
        owner: surface_owner(),
        revision: 3,
        events: (0..count).map(|_| event()).collect(),
    }
}
#[test]
fn unresolved_batch_receipt_blocks_new_input_until_evidence_is_taken() {
    let mut sessions = Sessions::default();
    let input = input(2);
    assert!(sessions.surface_input_ready(input.owner.binding()));
    sessions
        .batch_receipts
        .push(crate::desktop::surface_authorized_window::BatchReceipt {
            events: input.events,
            revision: 3,
            submission: Err(BatchError::Stopped),
            observation: None,
            delivery: Ok(()),
            presentation: Ok(None),
        });
    assert!(!sessions.surface_input_ready(input.owner.binding()));
    let receipt = sessions.take_surface_batch_receipt().unwrap();
    assert_eq!(receipt.events.len(), 2);
    assert!(sessions.surface_input_ready(input.owner.binding()));
    sessions.close();
    assert!(!sessions.surface_input_ready(input.owner.binding()));
}
#[test]
fn batch_counts_each_event_and_joins_through_application_shutdown_handles() {
    let mut sessions = Sessions::default();
    sessions
        .start_surface_batch(surface_config(), revoked(), input(32))
        .unwrap_or_else(|error| panic!("{}", error.error));
    assert_eq!(sessions.event_backlog(), 32);
    assert_eq!(sessions.wait_handles().count(), 1);
    assert_eq!(sessions.surface_shutdown_handles().count(), 1);
    assert!(sessions.surface_recovery_bundle().is_err());
    let error = sessions
        .start_surface_event(surface_config(), revoked(), surface_owner(), 3, event())
        .err()
        .unwrap();
    assert_eq!(error.error.kind(), io::ErrorKind::WouldBlock);
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
    assert_eq!(sessions.event_backlog(), 32);
    assert_eq!(sessions.occupied(), 1);
    let (attachments, records) = sessions.surface_recovery_bundle().unwrap();
    assert_eq!(attachments.len(), 1);
    assert_eq!(records.len(), 32);
    assert!(crate::surface_event_record::encode(&records).is_ok());
    let outcome = sessions.take_surface_batch_outcome().unwrap();
    assert!(matches!(outcome.submission, Err(BatchError::Stopped)));
    assert_eq!(outcome.input.events.len(), 32);
    assert!(outcome.input.events.iter().all(|event| !event.attempted()));
    assert!(outcome.delivery.is_err());
    assert!(!sessions.surface_cleanup_pending());
}
#[test]
fn rejected_batches_and_settled_receipts_retain_evidence_without_network_slots() {
    let mut sessions = Sessions::default();
    sessions.batch_outcomes.push(BatchOutcome {
        input: input(2),
        submission: Err(BatchError::Stopped),
        observation: None,
        delivery: Ok(()),
    });
    assert_eq!(
        sessions
            .settle_surface_batches(|outcome| Err(Box::new(outcome)))
            .unwrap(),
        0
    );
    sessions.cancel_surface_requests();
    assert_eq!(
        sessions
            .settle_surface_batches(|outcome| {
                assert!(outcome.delivery.is_err());
                Ok(crate::desktop::surface_authorized_window::BatchReceipt {
                    events: outcome.input.events,
                    revision: outcome.input.revision,
                    submission: outcome.submission,
                    observation: outcome.observation,
                    delivery: outcome.delivery,
                    presentation: Ok(None),
                })
            })
            .unwrap(),
        1
    );
    assert_eq!(sessions.occupied(), 0);
    assert_eq!(sessions.event_backlog(), 2);
    assert!(sessions.surface_recovery_records().is_err());
    let (attachments, records) = sessions.surface_recovery_bundle().unwrap();
    assert!(attachments.is_empty());
    assert_eq!(records.len(), 2);
    assert!(crate::surface_event_record::encode(&records).is_ok());
    let receipt = sessions.take_surface_batch_receipt().unwrap();
    assert_eq!(receipt.events.len(), 2);
    assert!(!sessions.surface_cleanup_pending());
}
