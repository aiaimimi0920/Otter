//! 本地 HWND 交付回归；不访问远端、不启用点击入口。
use super::*;
#[path = "surface_batch_delivery_tests.rs"]
mod batch_delivery;
#[path = "surface_decision_delivery_tests.rs"]
mod decision_delivery;
#[path = "surface_edit_delivery_tests.rs"]
mod edit_delivery;
#[path = "surface_edit_policy_tests.rs"]
mod edit_policy;
#[path = "surface_window_resource_tests.rs"]
mod resource_delivery;
#[path = "surface_result_binding_tests.rs"]
mod result_binding;
use crate::{
    desktop::surface_authorized_window::PresentationError, surface_action_ack::Ack,
    surface_click_event::ClickEvent, surface_click_route::ClickRoute,
    surface_convergence::Snapshot, surface_event_runtime::Outcome as EventOutcome,
    surface_scene::Scene,
};
fn active_window(grant: &mut crate::device_authorization_lease::Grant) -> AuthorizedWindow {
    let lease = grant.lease("https://localhost:1234", 0).unwrap();
    let mut window = AuthorizedWindow::new(&outcome(), lease, 0, [200, 140]).unwrap();
    let mut owner = window.begin_activation(0).unwrap().unwrap();
    let state = crate::surface_live_binding::Lifecycle::Active;
    let revision = owner.begin(state).unwrap();
    let binding = owner.binding();
    owner
        .acknowledge(
            &serde_json::to_vec(&serde_json::json!({"descriptor":{
        "instanceId":binding.instance(),"attachmentId":binding.attachment(),
        "hookNodeId":binding.node(),"deviceId":binding.device()},
        "lifecycle":"active","lifecycleRevision":revision}))
            .unwrap(),
            state,
            revision,
        )
        .unwrap();
    assert!(window.finish_activation(owner).is_ok());
    window.show(0).unwrap();
    window
}
fn event(window: &AuthorizedWindow) -> ClickEvent {
    let scene = Scene::parse(br#"{"id":"root","type":"view","events":{"click":"run"}}"#).unwrap();
    ClickRoute::new(&scene, Some("root"))
        .next()
        .unwrap()
        .encode(
            window.binding(),
            window.snapshot_revision(),
            Default::default(),
        )
        .unwrap()
}
#[test]
fn authorized_pointer_policy_stops_while_owner_is_inflight_and_after_revocation() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    assert!(window.sync_pointer(true, 0).unwrap());
    let events = vec![event(&window)];
    let owner = window.begin_batch(&events, 0).unwrap().unwrap();
    assert!(!window.sync_pointer(true, 0).unwrap());
    window
        .finish_batch(&events, owner)
        .unwrap_or_else(|_| panic!("restore"));
    assert!(window.sync_pointer(true, 0).unwrap());
    assert!(!window.sync_pointer(false, 0).unwrap());
    drop(grant);
    assert!(!window.sync_pointer(true, 0).unwrap());
    assert!(!window.is_alive());
}
#[test]
fn window_batch_transfer_rechecks_authority_and_restores_without_presenting() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    let mut events = vec![event(&window), event(&window)];
    let owner = window.begin_batch(&events, 0).unwrap().unwrap();
    assert!(window.begin_event(&events[0], 0).unwrap().is_none());
    let owner = *window.finish_batch(&events[..1], owner).err().unwrap();
    events[0].begin_submission();
    assert!(window.finish_batch(&events, owner).is_ok());
    assert_eq!(window.snapshot_revision(), 1);
    assert!(window.begin_batch(&events, 0).unwrap().is_none());
    let fresh = vec![event(&window)];
    drop(grant);
    assert!(window.begin_batch(&fresh, 0).unwrap().is_none());
    assert!(!window.is_alive());
    assert!(window.lifecycle().unwrap().confirmed_active());
}
fn completed(window: &mut AuthorizedWindow) -> EventOutcome {
    let mut event = event(window);
    let owner = window.begin_event(&event, 0).unwrap().unwrap();
    event.begin_submission();
    let ack = Ack::parse(
        &serde_json::to_vec(&serde_json::json!({
            "protocolVersion":"loom.surface.v1","instanceId":event.binding().instance(),
            "eventId":event.id(),"requestId":"request:test","accepted":true,"status":"succeeded"
        }))
        .unwrap(),
        &event,
    )
    .unwrap();
    EventOutcome {
        decision: None,
        confirmation: None,
        owner,
        event,
        result: Ok(ack),
        delivery: Ok(()),
        convergence: Some(Ok(Snapshot {
            resources: crate::surface_resource_snapshot::ResourceSnapshot::empty_for_test(
                window.binding(),
                2,
            ),
            commit: None,
            revision: 2,
            scene: Scene::parse(br#"{"id":"updated","type":"text","props":{"text":"fresh"}}"#)
                .unwrap(),
        })),
    }
}
#[test]
fn matching_result_updates_revision_once_and_returns_ack_receipt() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    let mut outcome = completed(&mut window);
    let raw = serde_json::value::RawValue::from_string(serde_json::json!({
        "protocolVersion":"loom.surface.v1","instanceId":"instance:test","requestId":"request:test",
        "generation":window.binding().generation(),"resultRevision":3,
        "outputs":{"value":{"kind":"value","value":"not persisted"}}
    }).to_string()).unwrap();
    outcome
        .convergence
        .as_mut()
        .unwrap()
        .as_mut()
        .unwrap()
        .commit = crate::surface_result_commit::Commit::parse_matching(
        &raw,
        &outcome.event,
        outcome.result.as_ref().unwrap(),
    )
    .unwrap();
    let id = outcome.event.id().to_owned();
    let receipt = window
        .accept_event(outcome, 0)
        .unwrap_or_else(|_| panic!("matching result rejected"));
    assert!(matches!(receipt.presentation, Ok(Some(2))));
    assert_eq!(receipt.convergence, Some(Ok(2)));
    assert_eq!(receipt.commit.as_ref().unwrap().revision(), 3);
    let metadata =
        crate::surface_event_record::encode(&[crate::surface_event_record::Record::from_receipt(
            &receipt,
        )])
        .unwrap();
    let metadata: serde_json::Value = serde_json::from_slice(&metadata).unwrap();
    assert_eq!(metadata["records"][0]["resultRevision"], 3);
    assert!(!metadata.to_string().contains("not persisted"));
    assert!(receipt.result.unwrap().succeeded());
    assert_eq!(receipt.event.id(), id);
    assert_eq!(window.snapshot_revision(), 2);
    assert!(window.lifecycle().unwrap().confirmed_active());
    assert!(window.begin_event(&receipt.event, 0).unwrap().is_none());
}
#[test]
fn foreign_event_is_returned_intact_without_releasing_pending_owner() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    let other = event(&window);
    let mut outcome = completed(&mut window);
    let original = std::mem::replace(&mut outcome.event, other);
    let mut returned = window.accept_event(outcome, 0).err().unwrap();
    assert!(window.lifecycle().is_none());
    assert!(returned.convergence.as_ref().unwrap().is_ok());
    returned.event = original;
    assert!(window.accept_event(*returned, 0).is_ok());
}
#[test]
fn revoked_expired_closed_and_denied_delivery_never_apply_snapshot() {
    for mode in 0..4 {
        let _com = Apartment::new();
        let mut grant = grant();
        let mut window = active_window(&mut grant);
        let mut outcome = completed(&mut window);
        let now = if mode == 1 { 100 } else { 0 };
        if mode == 0 {
            drop(grant);
        }
        if mode == 2 {
            window.close().unwrap();
        }
        if mode == 3 {
            outcome.delivery = Err(Error::Revoked);
        }
        let receipt = window
            .accept_event(outcome, now)
            .unwrap_or_else(|_| panic!("owner rejected"));
        assert!(receipt.presentation.is_err());
        assert!(receipt.result.unwrap().succeeded());
        assert_eq!(receipt.convergence, Some(Ok(2)));
        assert_eq!(window.snapshot_revision(), 1);
        assert!(window.lifecycle().unwrap().confirmed_active());
        if mode < 3 {
            assert!(!window.is_alive());
        }
    }
}
#[test]
fn stale_snapshot_and_non_success_ack_cannot_replace_current_scene() {
    for stale in [true, false] {
        let _com = Apartment::new();
        let mut grant = grant();
        let mut window = active_window(&mut grant);
        let mut outcome = completed(&mut window);
        if stale {
            outcome
                .convergence
                .as_mut()
                .unwrap()
                .as_mut()
                .unwrap()
                .revision = 0;
        } else {
            outcome.result = Err(crate::surface_event_runtime::Error::Stopped);
        }
        let receipt = window
            .accept_event(outcome, 0)
            .unwrap_or_else(|_| panic!("owner rejected"));
        assert!(matches!(
            receipt.presentation,
            Err(PresentationError::Revision | PresentationError::Evidence)
        ));
        assert_eq!(window.snapshot_revision(), 1);
        assert!(window.is_alive());
    }
}

#[test]
fn collection_returns_unmatched_result_and_preserves_convergence_failure_receipt() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    let mut outcome = completed(&mut window);
    let error = crate::surface_event_runtime::Error::Convergence(
        crate::live_relay_http::ConvergenceError::ConfirmationRequired,
    );
    outcome.convergence = Some(Err(error));
    let mut windows = crate::desktop::surface_windows::Windows::default();
    let returned = windows.accept_event(outcome, 0).err().unwrap();
    assert!(returned.event.attempted());
    // 同附件但没有待完成事件的窗口不能抢走另一个窗口的 owner。
    assert!(windows.insert(active_window(&mut grant)).is_ok());
    assert!(windows.insert(window).is_ok());
    let receipt = windows
        .accept_event(*returned, 0)
        .unwrap_or_else(|_| panic!("result not routed"));
    assert!(matches!(receipt.presentation, Ok(None)));
    assert_eq!(receipt.convergence, Some(Err(error)));
    assert!(receipt.result.unwrap().succeeded());
}

#[test]
fn confirmation_receipt_requires_fresh_window_authorization_and_expiry_check() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    let mut outcome = completed(&mut window);
    let ack = Ack::parse(&serde_json::to_vec(&serde_json::json!({
        "protocolVersion":"loom.surface.v1","instanceId":outcome.event.binding().instance(),
        "eventId":outcome.event.id(),"requestId":"request:test","accepted":true,"status":"awaiting_confirmation"
    })).unwrap(), &outcome.event).unwrap();
    let action: serde_json::Value = serde_json::from_slice(outcome.event.bytes()).unwrap();
    let raw = serde_json::value::RawValue::from_string(serde_json::json!({
        "protocolVersion":"loom.surface.v1","confirmationId":"confirmation:test",
        "instanceId":outcome.event.binding().instance(),"attachmentId":outcome.event.binding().attachment(),
        "deviceId":outcome.event.binding().device(),"hookNodeId":outcome.event.binding().node(),
        "eventId":outcome.event.id(),"requestId":"request:test","actionId":action["action"],
        "risk":"high","expiresAtMs":50
    }).to_string()).unwrap();
    outcome.confirmation = Some(Ok(crate::surface_confirmation::Request::parse_matching(
        &raw,
        &outcome.event,
        &ack,
    )
    .unwrap()
    .unwrap()));
    outcome.result = Ok(ack);
    outcome.convergence = Some(Err(crate::surface_event_runtime::Error::Convergence(
        crate::live_relay_http::ConvergenceError::ConfirmationRequired,
    )));
    let mut receipt = window
        .accept_event(outcome, 0)
        .unwrap_or_else(|_| panic!("owner rejected"));
    assert_eq!(
        window.confirmation_for(&receipt, 49).unwrap().id(),
        "confirmation:test"
    );
    assert!(window.confirmation_for(&receipt, 50).is_err());
    let choice = crate::surface_confirmation::Choice::Reject;
    assert!(window
        .prompt_decision_with(&mut receipt, || Ok(0), |_| Ok(None))
        .unwrap()
        .is_none());
    assert!(receipt.confirmation.as_ref().unwrap().is_ok());
    let mut times = [0, 50].into_iter();
    assert!(window
        .prompt_decision_with(
            &mut receipt,
            || Ok(times.next().unwrap()),
            |_| Ok(Some(choice))
        )
        .is_err());
    assert!(receipt.confirmation.as_ref().unwrap().is_ok());
    assert!(window.prepare_decision(&mut receipt, choice, 50).is_err());
    assert!(receipt.confirmation.as_ref().unwrap().is_ok());
    assert!(window.lifecycle().unwrap().confirmed_active());
    let decision = window
        .prompt_decision_with(
            &mut receipt,
            || Ok(0),
            |text| {
                assert!(text.contains("风险：高"));
                Ok(Some(choice))
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(decision.choice(), choice);
    assert!(!decision.attempted());
    assert!(!decision.confirmed());
    assert!(receipt.confirmation.is_none());
    assert!(window.prepare_decision(&mut receipt, choice, 0).is_err());
    assert!(window.lifecycle().unwrap().confirmed_active());
    let failed = window
        .begin_receipt_decision(receipt, decision, 50)
        .err()
        .unwrap();
    assert_eq!(failed.decision.request().id(), "confirmation:test");
    assert!(!failed.decision.attempted());
    assert!(window.lifecycle().unwrap().confirmed_active());
    let mut receipt = failed.receipt;
    receipt.presentation = Ok(Some(1));
    let failed = window
        .begin_receipt_decision(receipt, failed.decision, 0)
        .err()
        .unwrap();
    assert!(matches!(failed.receipt.presentation, Ok(Some(1))));
    let mut receipt = failed.receipt;
    receipt.presentation = Ok(None);
    let input = window
        .begin_receipt_decision(receipt, failed.decision, 0)
        .unwrap_or_else(|failure| panic!("{}", failure.error));
    assert_eq!(input.revision, 1);
    assert!(window.lifecycle().is_none());
    assert!(window
        .begin_decision(&input.event, &input.ack, &input.decision, 0)
        .unwrap()
        .is_none());
    // 模拟提交前取消的统一结果返还，证明原事件、ACK、决定和 owner 能结算回来。
    let returned = crate::surface_decision_runtime::Outcome {
        input,
        submission: Err(crate::surface_decision_runtime::Error::Stopped),
        convergence: None,
        delivery: Ok(()),
    }
    .into_event();
    let receipt = window
        .accept_event(returned, 0)
        .unwrap_or_else(|_| panic!("return owner"));
    let decision = &receipt.decision.as_ref().unwrap().decision;
    let ack = receipt.result.as_ref().unwrap();
    assert!(window.lifecycle().unwrap().confirmed_active());
    drop(grant);
    assert!(window
        .begin_decision(&receipt.event, ack, decision, 0)
        .unwrap()
        .is_none());
    assert!(window.confirmation_for(&receipt, 0).is_err());
    assert!(!window.is_alive());
    assert_eq!(
        receipt.result.unwrap().status(),
        crate::surface_action_ack::Status::AwaitingConfirmation
    );
    assert!(receipt.confirmation.is_none());
    assert_eq!(decision.request().id(), "confirmation:test");
}
