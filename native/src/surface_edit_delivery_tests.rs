//! 输入从真实 EDIT 到授权批次，再按完整证据回填；网络 ACK 使用合成夹具，不宣称跨机验收。
use super::*;
#[path = "surface_edit_dependency_tests.rs"]
mod dependencies;
use crate::{
    live_relay_http::{BatchStop, BatchSubmission},
    surface_event_batch_runtime::{Input, Observation, ObservedItem, Outcome},
};
use windows::{
    core::HSTRING,
    Win32::{
        Foundation::*,
        UI::{Controls::*, WindowsAndMessaging::*},
    },
};
fn form(value: &str) -> Scene {
    Scene::parse(serde_json::json!({"id":"field","type":"input","props":{"value":value},"events":{"input":"typing","change":"commit"}}).to_string().as_bytes()).unwrap()
}
fn field(window: &AuthorizedWindow) -> HWND {
    unsafe { GetWindow(window.edit_test_parent(), GW_CHILD).unwrap() }
}
fn type_text(hwnd: HWND, text: &str) {
    unsafe {
        SendMessageW(hwnd, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
        let text = HSTRING::from(text);
        SendMessageW(
            hwnd,
            EM_REPLACESEL,
            Some(WPARAM(1)),
            Some(LPARAM(text.as_ptr() as isize)),
        );
    }
}
fn text(hwnd: HWND) -> String {
    let mut buffer = [0u16; 128];
    let count = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    String::from_utf16(&buffer[..count as usize]).unwrap()
}
fn install(window: &mut AuthorizedWindow) {
    let mut update = completed(window);
    update.convergence.as_mut().unwrap().as_mut().unwrap().scene = form("source");
    let receipt = window
        .accept_event(update, 0)
        .unwrap_or_else(|_| panic!("initial owner rejected"));
    assert!(matches!(receipt.presentation, Ok(Some(2))));
    assert!(window.sync_input(true, true, 0).unwrap());
}
fn input(window: &mut AuthorizedWindow, value: &str) -> Input {
    type_text(field(window), value);
    assert!(window.has_pending_click());
    let events = window.take_prepared_click(0).unwrap().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(events[0].bytes()).unwrap()["payload"]["value"],
        value
    );
    let revision = window.snapshot_revision();
    let owner = window.begin_batch(&events, 0).unwrap().unwrap();
    Input {
        owner,
        events,
        revision,
    }
}
fn success(mut input: Input, value: &str) -> Outcome {
    let acknowledgements = input.events.iter_mut().map(|event| {
        event.begin_submission();
        Ack::parse(&serde_json::to_vec(&serde_json::json!({"protocolVersion":"loom.surface.v1", "instanceId":event.binding().instance(), "eventId":event.id(),"requestId":"request:edit","accepted":true,"status":"succeeded"})).unwrap(), event).unwrap()
    }).collect();
    let revision = input.revision + 1;
    let resources = crate::surface_resource_snapshot::ResourceSnapshot::empty_for_test(
        input.owner.binding(),
        revision,
    );
    Outcome {
        input,
        delivery: Ok(()),
        submission: Ok(BatchSubmission {
            acknowledgements,
            stop: BatchStop::Complete,
        }),
        observation: Some(Ok(Observation {
            items: vec![ObservedItem {
                decision: None,
                result: Ok(revision),
                commit: None,
                confirmation: None,
            }],
            snapshot: Some(Snapshot {
                resources,
                revision,
                commit: None,
                scene: form(value),
            }),
        })),
    }
}
#[test]
fn surface_edit_delivery_preserves_typing_during_owner_transfer_then_settles_verified_result() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    install(&mut window);
    let first = input(&mut window, "first");
    assert!(window.lifecycle().is_none());
    assert!(!window.sync_input(true, true, 0).unwrap());
    type_text(field(&window), "second");
    assert!(window.take_prepared_click(0).is_err());
    let receipt = window
        .accept_batch(success(first, "first result"), 0)
        .unwrap_or_else(|_| panic!("owner rejected"));
    assert!(matches!(receipt.presentation, Ok(Some(3))));
    assert_eq!(text(field(&window)), "second");
    window.sync_input(true, true, 0).unwrap();
    let events = window.take_prepared_click(0).unwrap().unwrap();
    assert_eq!(events.len(), 1);
    let owner = window.begin_batch(&events, 0).unwrap().unwrap();
    let next = Input {
        owner,
        events,
        revision: 3,
    };
    let receipt = window
        .accept_batch(success(next, "accepted"), 0)
        .unwrap_or_else(|_| panic!("owner rejected"));
    assert!(matches!(receipt.presentation, Ok(Some(4))));
    assert_eq!(text(field(&window)), "accepted");
    assert!(!window.has_pending_click());
}
#[test]
fn surface_edit_delivery_not_started_restores_draft_and_revocation_never_resurrects_control() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    install(&mut window);
    let pending = input(&mut window, "local");
    let outcome = Outcome {
        input: pending,
        delivery: Ok(()),
        submission: Err(crate::surface_event_batch_runtime::Error::NotStarted),
        observation: None,
    };
    let receipt = window
        .accept_batch(outcome, 0)
        .unwrap_or_else(|_| panic!("owner rejected"));
    assert!(matches!(receipt.presentation, Ok(None)));
    assert_eq!(text(field(&window)), "local");
    window.sync_input(true, true, 0).unwrap();
    assert!(window.has_pending_click());
    let pending = input(&mut window, "later");
    let edit = field(&window);
    drop(grant);
    assert!(!window.sync_input(true, true, 0).unwrap());
    assert!(!unsafe { IsWindow(Some(edit)) }.as_bool());
    let receipt = window
        .accept_batch(success(pending, "late"), 0)
        .unwrap_or_else(|_| panic!("owner rejected"));
    assert!(receipt.presentation.is_err());
    assert!(!window.is_alive());
}
#[test]
fn surface_edit_delivery_invalid_evidence_does_not_apply_authority_or_replay_input() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    install(&mut window);
    let pending = input(&mut window, "local");
    let mut outcome = success(pending, "unverified");
    outcome
        .submission
        .as_mut()
        .unwrap()
        .acknowledgements
        .clear();
    let receipt = window
        .accept_batch(outcome, 0)
        .unwrap_or_else(|_| panic!("owner rejected"));
    assert!(matches!(
        receipt.presentation,
        Err(PresentationError::Evidence)
    ));
    assert_eq!(text(field(&window)), "local");
    assert!(!window.has_pending_click());
}
#[test]
fn surface_edit_delivery_failed_predecessor_discards_dependents_but_keeps_visible_draft() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    install(&mut window);
    let pending = input(&mut window, "first");
    window.sync_input(true, true, 0).unwrap();
    type_text(field(&window), "dependent");
    assert!(window.has_pending_click());
    let mut outcome = success(pending, "must not apply");
    outcome
        .submission
        .as_mut()
        .unwrap()
        .acknowledgements
        .clear();
    let receipt = window
        .accept_batch(outcome, 0)
        .unwrap_or_else(|_| panic!("owner rejected"));
    assert!(matches!(
        receipt.presentation,
        Err(PresentationError::Evidence)
    ));
    assert_eq!(text(field(&window)), "dependent");
    assert!(!window.has_pending_click());
    window.sync_input(true, true, 0).unwrap();
    unsafe {
        SendMessageW(
            window.edit_test_parent(),
            WM_COMMAND,
            Some(WPARAM((EN_KILLFOCUS as usize) << 16)),
            Some(LPARAM(field(&window).0 as isize)),
        );
    }
    assert!(!window.has_pending_click());
    type_text(field(&window), "explicit new edit");
    let events = window.take_prepared_click(0).unwrap().unwrap();
    let wire: serde_json::Value = serde_json::from_slice(events[0].bytes()).unwrap();
    assert_eq!(wire["payload"]["value"], "explicit new edit");
}
#[test]
fn surface_edit_delivery_explicit_confirmation_wait_preserves_dependents() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = active_window(&mut grant);
    install(&mut window);
    let mut pending = input(&mut window, "first");
    window.sync_input(true, true, 0).unwrap();
    type_text(field(&window), "dependent");
    let event = &mut pending.events[0];
    event.begin_submission();
    let ack = Ack::parse(&serde_json::to_vec(&serde_json::json!({
        "protocolVersion":"loom.surface.v1", "instanceId":event.binding().instance(),
        "eventId":event.id(),"requestId":"request:confirmation","accepted":true,"status":"awaiting_confirmation"
    })).unwrap(), event).unwrap();
    let outcome = Outcome {
        input: pending,
        delivery: Ok(()),
        submission: Ok(BatchSubmission {
            acknowledgements: vec![ack],
            stop: BatchStop::Confirmation,
        }),
        observation: None,
    };
    let receipt = window
        .accept_batch(outcome, 0)
        .unwrap_or_else(|_| panic!("owner rejected"));
    assert!(matches!(receipt.presentation, Ok(None)));
    assert!(window.has_pending_click());
    assert_eq!(text(field(&window)), "dependent");
}
