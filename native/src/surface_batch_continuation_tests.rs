//! 显式接续的 HWND/owner 边界；完成前缀与未尝试后缀不可拆散或重放。
use super::*;
fn ready(window: &mut AuthorizedWindow) -> crate::desktop::surface_authorized_window::BatchReceipt {
    let receipt = waiting(window);
    let mut prepared = window
        .prepare_batch_decision(receipt, 0, Choice::Approve, 0)
        .unwrap_or_else(|_| panic!("prepare"));
    prepared.decision.begin_submission();
    prepared.decision.acknowledge();
    let mut receipt = prepared.receipt;
    let event = &receipt.events[0];
    let ack = Ack::parse(&serde_json::to_vec(&serde_json::json!({
        "protocolVersion":"loom.surface.v1","instanceId":event.binding().instance(),"eventId":event.id(),
        "requestId":"request:0","accepted":true,"status":"succeeded"
    })).unwrap(), event).unwrap();
    let submitted = receipt.submission.as_mut().unwrap();
    submitted.acknowledgements = vec![ack];
    submitted.stop = crate::live_relay_http::BatchStop::Confirmation;
    receipt.events.truncate(1);
    receipt.events.push(super::super::event(window));
    let observed = receipt.observation.as_mut().unwrap().as_mut().unwrap();
    observed.items.truncate(1);
    observed.items[0].result = Ok(receipt.revision + 1);
    observed.items[0].decision = Some(crate::surface_decision_runtime::Evidence {
        decision: prepared.decision,
        revision: receipt.revision,
        submission: Ok(()),
    });
    receipt
}
#[test]
fn continuation_transfers_whole_batch_and_never_marks_tail_attempted() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    let receipt = ready(&mut window);
    assert_eq!(receipt.continuation_boundary(), Some(1));
    assert_eq!(window.batch_continuation_for(&receipt, 0).unwrap(), 1);
    assert!(window.lifecycle().unwrap().confirmed_active());
    let input = window
        .begin_batch_continuation(receipt, 0)
        .unwrap_or_else(|_| panic!("transfer"));
    assert_eq!(input.next, 1);
    assert!(window.lifecycle().is_none());
    assert!(input.receipt.events[0].attempted());
    assert!(!input.receipt.events[1].attempted());
    window
        .finish_batch(&input.receipt.events, input.owner)
        .unwrap_or_else(|_| panic!("restore"));
    assert!(window.lifecycle().unwrap().confirmed_active());
}
#[test]
fn continuation_rejects_uncertain_prefix_attempted_tail_and_late_revocation() {
    let _com = Apartment::new();
    for case in 0..3 {
        let mut grant = grant();
        let mut window = active_window(&mut grant);
        let mut receipt = ready(&mut window);
        match case {
            0 => {
                receipt
                    .observation
                    .as_mut()
                    .unwrap()
                    .as_mut()
                    .unwrap()
                    .items[0]
                    .result = Err(crate::surface_event_batch_runtime::ObservationError::Panicked)
            }
            1 => receipt.events[1].begin_submission(),
            _ => drop(grant),
        }
        let returned = window.begin_batch_continuation(receipt, 0).err().unwrap();
        assert_eq!(returned.receipt.events.len(), 2);
        assert!(window.lifecycle().unwrap().confirmed_active());
        if case == 2 {
            assert!(!window.is_alive());
        }
    }
}
