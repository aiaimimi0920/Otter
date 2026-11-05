//! 决定结果经统一事件交付到真实 HWND；模拟协议证据，不连接真实服务。
use super::*;
use crate::{
    surface_confirmation::{Choice, Decision, Request as Prompt},
    surface_decision_runtime::{Input, Outcome as DecisionOutcome},
};
fn outcome(window: &mut AuthorizedWindow, confirmed: bool) -> EventOutcome {
    let completion = completed(window);
    let event = &completion.event;
    let pending = Ack::parse(&serde_json::to_vec(&serde_json::json!({
        "protocolVersion":"loom.surface.v1","instanceId":event.binding().instance(),"eventId":event.id(),
        "requestId":"request:test","accepted":true,"status":"awaiting_confirmation"
    })).unwrap(), event).unwrap();
    let action: serde_json::Value = serde_json::from_slice(event.bytes()).unwrap();
    let raw = serde_json::value::RawValue::from_string(serde_json::json!({
        "protocolVersion":"loom.surface.v1","confirmationId":"confirmation:test","instanceId":event.binding().instance(),
        "attachmentId":event.binding().attachment(),"deviceId":event.binding().device(),"hookNodeId":event.binding().node(),
        "eventId":event.id(),"requestId":pending.request_id(),"actionId":action["action"],"risk":"high","expiresAtMs":u64::MAX
    }).to_string()).unwrap();
    let prompt = Prompt::parse_matching(&raw, event, &pending)
        .unwrap()
        .unwrap();
    let mut decision = Decision::new(prompt, event, &pending, Choice::Approve, 0)
        .unwrap_or_else(|_| panic!("prepare"));
    decision.begin_submission();
    if confirmed {
        decision.acknowledge();
    }
    DecisionOutcome {
        input: Input {
            owner: completion.owner,
            event: completion.event,
            ack: completion.result.unwrap(),
            decision,
            revision: 1,
        },
        submission: Ok(()),
        convergence: completion
            .convergence
            .map(|result| result.map_err(|_| unreachable!())),
        delivery: Ok(()),
    }
    .into_event()
}
#[test]
fn confirmed_decision_applies_snapshot_and_preserves_decision_in_receipt_and_metadata() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    let result = outcome(&mut window, true);
    let receipt = window
        .accept_event(result, 0)
        .unwrap_or_else(|_| panic!("rejected"));
    assert!(matches!(receipt.presentation, Ok(Some(2))));
    let evidence = receipt.decision.as_ref().unwrap();
    assert!(evidence.submission.is_ok() && evidence.decision.confirmed());
    let metadata = crate::surface_event_record::Record::from_receipt(&receipt);
    assert!(metadata.decision().unwrap().approved());
    assert!(metadata.decision().unwrap().confirmed());
    assert!(crate::surface_event_record::encode(&[metadata]).is_ok());
    assert_eq!(window.snapshot_revision(), 2);
}
#[test]
fn unconfirmed_or_revoked_decision_cannot_apply_snapshot_but_keeps_evidence() {
    for confirmed in [false, true] {
        let _com = Apartment::new();
        let mut grant = grant();
        let mut window = active_window(&mut grant);
        let result = outcome(&mut window, confirmed);
        if confirmed {
            drop(grant);
        }
        let receipt = window
            .accept_event(result, 0)
            .unwrap_or_else(|_| panic!("rejected owner"));
        assert!(receipt.presentation.is_err());
        assert_eq!(window.snapshot_revision(), 1);
        assert!(receipt.result.unwrap().succeeded());
        assert_eq!(receipt.decision.unwrap().decision.confirmed(), confirmed);
    }
}
#[test]
fn foreign_event_rejection_returns_decision_evidence_intact() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    let mut result = outcome(&mut window, true);
    let original = std::mem::replace(&mut result.event, event(&window));
    let mut returned = window.accept_event(result, 0).err().unwrap();
    assert!(returned.decision.as_ref().unwrap().decision.confirmed());
    assert!(window.lifecycle().is_none());
    returned.event = original;
    assert!(window.accept_event(*returned, 0).is_ok());
}
