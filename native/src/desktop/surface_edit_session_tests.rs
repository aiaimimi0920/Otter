//! 使用真实隐藏 EDIT 验证会话；不发送网络请求或模拟已完成人工 IME 验收。
use super::*;
use crate::{desktop::surface_window::SurfaceWindow, surface_scene::Scene};
#[path = "surface_edit_unbound_tests.rs"]
mod unbound;

fn binding() -> Binding {
    Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":7},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":3}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap()
}
fn scene(value: &str) -> Scene {
    Scene::parse(
        serde_json::json!({"id":"field", "type":"input", "props":{"value":value},
        "events":{"input":"typing", "change":"commit"}})
        .to_string()
        .as_bytes(),
    )
    .unwrap()
}
fn setup() -> (SurfaceWindow, Session) {
    let parent = SurfaceWindow::new(
        Scene::parse(br#"{"id":"root","type":"view"}"#).unwrap(),
        240,
        160,
    )
    .unwrap();
    let session = Session::new(
        parent.dialog_owner().unwrap(),
        scene("source").root(),
        &binding(),
        RECT {
            left: 4,
            top: 4,
            right: 204,
            bottom: 40,
        },
    )
    .unwrap();
    (parent, session)
}
#[test]
fn surface_edit_session_pending_result_preserves_newer_unsent_text() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let (_parent, mut session) = setup();
    session.control().set_text("中文🙂").unwrap();
    let first = session
        .prepare(scene("source").root(), &binding(), 1, Phase::Input)
        .unwrap()
        .unwrap();
    session.control().set_text("later").unwrap();
    assert!(session.capture().unwrap());
    session
        .synchronize(scene("first result").root(), &binding())
        .unwrap();
    assert!(session.settle(first.id()).unwrap());
    assert_eq!(session.control().text().unwrap(), "later");
    assert!(!session.settle(first.id()).unwrap());
    let second = session
        .prepare(scene("first result").root(), &binding(), 2, Phase::Change)
        .unwrap()
        .unwrap();
    session
        .synchronize(scene("final result").root(), &binding())
        .unwrap();
    assert!(session.settle(second.id()).unwrap());
    assert_eq!(session.control().text().unwrap(), "final result");
}
#[test]
fn surface_edit_session_cancel_restores_local_reservation() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let (_parent, mut session) = setup();
    session.control().set_text("local").unwrap();
    let event = session
        .prepare(scene("source").root(), &binding(), 1, Phase::Input)
        .unwrap()
        .unwrap();
    assert!(session.cancel(event.id()));
    assert!(!session.cancel(event.id()));
    session
        .synchronize(scene("remote").root(), &binding())
        .unwrap();
    assert_eq!(session.value(), "local");
    assert!(session.has_unsent());
    assert_eq!(session.pending(), 0);
}
#[test]
fn surface_edit_session_composition_defers_updates_and_submission() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let (_parent, mut session) = setup();
    session.begin_composition().unwrap();
    session.control().set_text("组合🙂").unwrap();
    assert!(!session.capture().unwrap());
    session
        .synchronize(scene("remote").root(), &binding())
        .unwrap();
    assert!(session
        .prepare(scene("remote").root(), &binding(), 2, Phase::Input)
        .is_err());
    assert_eq!(session.control().text().unwrap(), "组合🙂");
    assert!(session.end_composition().unwrap());
    assert_eq!(session.value(), "组合🙂");
    assert!(session.has_unsent());
}
#[test]
fn surface_edit_session_cancelled_composition_accepts_deferred_authority() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let (_parent, mut session) = setup();
    session.begin_composition().unwrap();
    session
        .synchronize(scene("remote").root(), &binding())
        .unwrap();
    assert!(!session.end_composition().unwrap());
    assert_eq!(session.control().text().unwrap(), "remote");
    assert!(!session.has_unsent());
}
#[test]
fn surface_edit_session_scope_and_destroyed_parent_fail_closed() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let (parent, mut session) = setup();
    let other = Scene::parse(br#"{"id":"other","type":"input"}"#).unwrap();
    assert!(session.synchronize(other.root(), &binding()).is_err());
    let disabled =
        Scene::parse(br#"{"id":"field","type":"input","props":{"disabled":true}}"#).unwrap();
    assert!(session
        .prepare(disabled.root(), &binding(), 1, Phase::Input)
        .is_err());
    parent.close().unwrap();
    assert!(session.capture().is_err());
    assert!(session.begin_composition().is_err());
    assert!(session.settle("foreign").is_err());
}

#[test]
fn surface_edit_session_missing_phase_cannot_release_newer_input() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let (_parent, mut session) = setup();
    let input_only =
        Scene::parse(br#"{"id":"field","type":"input","events":{"input":"typing"}}"#).unwrap();
    session.control().set_text("A").unwrap();
    let event = session
        .prepare(input_only.root(), &binding(), 1, Phase::Input)
        .unwrap()
        .unwrap();
    session.control().set_text("AB").unwrap();
    assert!(session
        .prepare(input_only.root(), &binding(), 1, Phase::Change)
        .unwrap()
        .is_none());
    session.synchronize(scene("A").root(), &binding()).unwrap();
    assert!(session.settle(event.id()).unwrap());
    assert_eq!(session.control().text().unwrap(), "AB");
    assert!(session.has_unsent());
}

#[test]
fn surface_edit_session_cancelled_composition_preserves_handlerless_local_value() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let (_parent, mut session) = setup();
    let handlerless =
        Scene::parse(br#"{"id":"field","type":"input","props":{"value":"source"}}"#).unwrap();
    session.control().set_text("local").unwrap();
    assert!(session
        .prepare(handlerless.root(), &binding(), 1, Phase::Input)
        .unwrap()
        .is_none());
    assert!(!session.has_unsent());
    session.begin_composition().unwrap();
    session.synchronize(handlerless.root(), &binding()).unwrap();
    assert!(!session.end_composition().unwrap());
    assert_eq!(session.control().text().unwrap(), "local");
    assert!(!session.has_unsent());
}

#[test]
fn surface_edit_session_capacity_keeps_one_local_slot_and_settles_out_of_order() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let (_parent, mut session) = setup();
    let node = scene("source");
    let mut events = Vec::new();
    for index in 0..MAX_PENDING - 1 {
        session.control().set_text(&index.to_string()).unwrap();
        events.push(
            session
                .prepare(node.root(), &binding(), 1, Phase::Input)
                .unwrap()
                .unwrap(),
        );
    }
    session.control().set_text("newest").unwrap();
    assert!(session.capture().unwrap());
    assert!(session
        .prepare(node.root(), &binding(), 1, Phase::Input)
        .is_err());
    session
        .synchronize(scene("remote").root(), &binding())
        .unwrap();
    for event in events.iter().rev() {
        assert!(session.settle(event.id()).unwrap());
        assert!(!session.settle(event.id()).unwrap());
    }
    assert_eq!(session.value(), "newest");
    assert_eq!(session.pending(), 0);
    assert!(session.has_unsent());
}

#[test]
fn surface_edit_session_result_during_cancelled_composition_reconciles_after_end() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let (_parent, mut session) = setup();
    session.control().set_text("submitted").unwrap();
    let event = session
        .prepare(scene("source").root(), &binding(), 1, Phase::Input)
        .unwrap()
        .unwrap();
    session.begin_composition().unwrap();
    assert!(session.settle(event.id()).unwrap());
    assert_eq!(session.control().text().unwrap(), "submitted");
    assert!(!session.end_composition().unwrap());
    assert_eq!(session.control().text().unwrap(), "source");
}
