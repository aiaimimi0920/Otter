//! 应用任务集合真实线程收割及结果缓存边界；使用已撤销租约，不连接远端。
use super::*;
#[path = "composition_relay_surface_batches_tests.rs"]
mod batches;
#[path = "composition_relay_surface_decisions_tests.rs"]
mod decisions;
#[path = "composition_relay_surface_outputs_tests.rs"]
mod outputs;
use crate::{
    surface_click_event::{ClickEvent, Modifiers},
    surface_click_route::ClickRoute,
    surface_event_runtime::{Error as EventError, Outcome},
    surface_scene::Scene,
};
fn event() -> ClickEvent {
    let scene = Scene::parse(br#"{"id":"root","type":"view","events":{"click":"run"}}"#).unwrap();
    ClickRoute::new(&scene, Some("root"))
        .next()
        .unwrap()
        .encode(surface_owner().binding(), 3, Modifiers::default())
        .unwrap()
}
fn completed() -> Outcome {
    let mut event = event();
    event.begin_submission();
    let ack = crate::surface_action_ack::Ack::parse(
        &serde_json::to_vec(&serde_json::json!({
            "protocolVersion":"loom.surface.v1","instanceId":"surface:test","eventId":event.id(),
            "requestId":"request:test","accepted":true,"status":"succeeded"
        }))
        .unwrap(),
        &event,
    )
    .unwrap();
    Outcome {
        decision: None,
        owner: surface_owner(),
        event,
        result: Ok(ack),
        convergence: None,
        confirmation: None,
        delivery: Ok(()),
    }
}
#[test]
fn event_jobs_join_through_existing_poll_wait_and_hold_shared_capacity() {
    let mut sessions = Sessions::default();
    for _ in 0..MAX_SESSIONS {
        sessions
            .start_surface_event(surface_config(), revoked(), surface_owner(), 3, event())
            .unwrap_or_else(|e| panic!("{}", e.error));
    }
    assert_eq!(sessions.wait_handles().count(), MAX_SESSIONS);
    assert_eq!(sessions.surface_shutdown_handles().count(), MAX_SESSIONS);
    assert!(sessions.surface_recovery_records().is_err());
    assert!(
        matches!(sessions.discover(surface_config(), revoked()), Err(e) if e.kind() == io::ErrorKind::WouldBlock)
    );
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
    assert_eq!(sessions.wait_handles().count(), 0);
    assert_eq!(sessions.occupied(), MAX_SESSIONS);
    assert!(sessions.surface_cleanup_pending());
    for _ in 0..MAX_SESSIONS {
        let outcome = sessions.take_surface_event_outcome().unwrap();
        assert!(matches!(outcome.result, Err(EventError::Stopped)));
        assert!(outcome.delivery.is_err());
        assert!(!outcome.event.attempted());
    }
    assert!(!sessions.surface_cleanup_pending());
    let pending = event();
    let id = pending.id().to_owned();
    let Err(error) =
        sessions.start_surface_event(surface_config(), revoked(), surface_owner(), 3, pending)
    else {
        panic!("closed collection accepted event")
    };
    assert_eq!(error.error.kind(), io::ErrorKind::BrokenPipe);
    assert_eq!(error.event.id(), id);
    assert_eq!(error.owner.binding().instance(), "surface:test");
}
#[test]
fn rejected_event_delivery_preserves_capacity_and_cancel_keeps_ack_evidence() {
    let mut sessions = Sessions::default();
    let outcome = completed();
    let id = outcome.event.id().to_owned();
    sessions.event_outcomes.push(outcome);
    assert!(!sessions.deliver_surface_event(|outcome| Err(Box::new(outcome))));
    assert_eq!(sessions.occupied(), 1);
    // 旧附件恢复记录不能冒充保存了待处理动作证据。
    assert!(sessions.surface_recovery_records().is_err());
    sessions.cancel_surface_requests();
    assert!(sessions.deliver_surface_event(|outcome| {
        assert_eq!(outcome.event.id(), id);
        assert!(outcome.result.unwrap().succeeded());
        assert!(outcome.delivery.is_err());
        Ok(())
    }));
    assert_eq!(sessions.occupied(), 0);
    assert!(sessions.surface_recovery_records().unwrap().is_empty());
}
#[test]
fn unmatched_event_does_not_block_another_completed_result() {
    let mut sessions = Sessions::default();
    sessions.event_outcomes.push(completed());
    let expected = sessions.event_outcomes[0].event.id().to_owned();
    sessions.event_outcomes.push(completed());
    assert!(sessions.deliver_surface_event(|outcome| {
        if outcome.event.id() == expected {
            Ok(())
        } else {
            Err(Box::new(outcome))
        }
    }));
    assert_eq!(sessions.occupied(), 1);
    assert_ne!(
        sessions.take_surface_event_outcome().unwrap().event.id(),
        expected
    );
}

fn receipt(outcome: Outcome) -> crate::desktop::surface_authorized_window::EventReceipt {
    crate::desktop::surface_authorized_window::EventReceipt {
        decision: outcome.decision,
        commit: None,
        confirmation: outcome.confirmation,
        event: outcome.event,
        result: outcome.result,
        convergence: None,
        delivery: outcome.delivery,
        presentation: Ok(None),
    }
}
#[test]
fn settlement_caches_receipts_and_preserves_rejected_results_and_shutdown_evidence() {
    let mut sessions = Sessions::default();
    sessions.event_outcomes.push(completed());
    let expected = sessions.event_outcomes[0].event.id().to_owned();
    sessions.event_outcomes.push(completed());
    assert_eq!(
        sessions
            .settle_surface_events(|outcome| {
                if outcome.event.id() == expected {
                    Ok(receipt(outcome))
                } else {
                    Err(Box::new(outcome))
                }
            })
            .unwrap(),
        1
    );
    assert_eq!(sessions.occupied(), 1);
    assert_eq!(sessions.event_receipts.len(), 1);
    assert!(sessions.surface_recovery_records().is_err());
    sessions.close();
    assert!(sessions.finished());
    assert_eq!(sessions.wait_handles().count(), 0);
    assert!(sessions.surface_cleanup_pending());
    let receipt = sessions.take_surface_event_receipt().unwrap();
    assert_eq!(receipt.event.id(), expected);
    assert!(receipt.result.unwrap().succeeded());
    // 已完成的收割/呈现检查是历史证据，关闭集合不能改写历史。
    assert!(receipt.delivery.is_ok());
    sessions.take_surface_event_outcome().unwrap();
    assert!(!sessions.surface_cleanup_pending());
}
#[test]
fn receipt_backlog_blocks_new_actions_without_consuming_results_or_network_slots() {
    let mut sessions = Sessions::default();
    for _ in 0..32 {
        sessions.event_receipts.push(receipt(completed()));
    }
    assert_eq!(sessions.occupied(), 0);
    assert!(sessions.surface_recovery_records().is_err());
    let error = sessions
        .start_surface_event(surface_config(), revoked(), surface_owner(), 3, event())
        .err()
        .unwrap();
    assert_eq!(error.error.kind(), io::ErrorKind::WouldBlock);
    assert!(!error.event.attempted());
    assert_eq!(error.owner.binding(), surface_owner().binding());
    sessions.event_outcomes.push(completed());
    assert_eq!(
        sessions
            .settle_surface_events(|_| panic!("full receipt queue consumed result"))
            .unwrap(),
        0
    );
    assert_eq!(sessions.event_outcomes.len(), 1);
    sessions.take_surface_event_receipt().unwrap();
    assert_eq!(
        sessions
            .settle_surface_events(|outcome| Ok(receipt(outcome)))
            .unwrap(),
        1
    );
    assert_eq!(sessions.event_receipts.len(), 32);
    assert!(sessions.event_outcomes.is_empty());
}

#[test]
fn recovery_bundle_preserves_pending_owners_and_receipts_without_consuming_evidence() {
    let mut sessions = Sessions::default();
    sessions.event_outcomes.push(completed());
    sessions.event_receipts.push(receipt(completed()));
    let (attachments, events) = sessions.surface_recovery_bundle().unwrap();
    assert_eq!(attachments.len(), 1);
    assert_eq!(events.len(), 2);
    assert!(crate::surface_event_record::encode(&events).is_ok());
    assert!(events.iter().all(|event| event.attempted()));
    assert_eq!(sessions.event_outcomes.len(), 1);
    assert_eq!(sessions.event_receipts.len(), 1);
    assert!(sessions.surface_recovery_records().is_err());
}
