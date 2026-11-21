use super::*;
use crate::surface_javascript::fixture::{self, binding, resources, snapshot, text};
use serde_json::json;
const SOURCE: &str = include_str!("../tests/fixtures/javascript-view.js");

#[test]
fn view_change_updates_existing_child_without_resetting_state_resources_or_authority_revision() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let mut value = snapshot(3, 42);
    value["state"]["emit"] = json!(true);
    let (mut session, first) =
        Session::mount(binding(), SOURCE.into(), value, resources(), &|| false).unwrap();
    let process = session.child.as_ref().unwrap().probe().unwrap();
    let button = first.scene.root().children()[1].id().to_owned();
    assert_eq!(text(&first.scene), "none:0:0:3:42:1");
    assert_eq!(
        text(&session.click(&button, &|| false).unwrap().scene),
        "none:1:0:3:42:1"
    );
    for (index, view) in ["main", "vendor/detail.v2:wide-mode", "main"]
        .iter()
        .enumerate()
    {
        let frame = session.select_view(view, &|| false).unwrap();
        assert_eq!(
            text(&frame.scene),
            format!("{view}:{}:{}:3:42:1", index + 1, index + 1)
        );
        let frame = session.click(&button, &|| false).unwrap();
        let event: Value = serde_json::from_slice(frame.events[0].bytes()).unwrap();
        assert_eq!(event["baseRevision"], 3);
        assert_eq!(event["payload"]["viewId"], *view);
        assert_eq!(event["payload"]["count"], index + 2);
        assert_eq!(event["generation"], 7);
    }
    assert_eq!(session.revision, 3);
    assert!(session
        .update(snapshot(3, 99), resources(), &|| false)
        .is_err());
    session.dispose(&|| false).unwrap();
    assert!(process.exited());
    assert_eq!(CAPACITY.used(), 0);
}

#[test]
fn invalid_view_does_not_poison_session_and_local_switch_does_not_resume_suspension() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (mut session, first) = Session::mount(
        binding(),
        SOURCE.into(),
        snapshot(4, 9),
        resources(),
        &|| false,
    )
    .unwrap();
    let button = first.scene.root().children()[1].id().to_owned();
    for id in ["", "bad space", "bad\n", &"x".repeat(161)] {
        assert!(session.select_view(id, &|| false).is_err());
    }
    assert_eq!(
        text(&session.select_view("valid", &|| false).unwrap().scene),
        "valid:0:1:4:9:1"
    );
    session.suspend(&|| false).unwrap();
    assert_eq!(
        text(&session.select_view("paused", &|| false).unwrap().scene),
        "paused:0:2:4:9:1"
    );
    assert!(session.suspended);
    assert!(session.click(&button, &|| false).is_err());
    session.resume(&|| false).unwrap();
    assert_eq!(
        text(&session.click(&button, &|| false).unwrap().scene),
        "paused:1:2:4:9:1"
    );
    session.dispose(&|| false).unwrap();
}

#[test]
fn local_view_update_never_calls_inherited_snapshot_setter_or_to_json() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let source = r#"let label;
      NeuroSurface.define({mount({root}){label=document.createElement('span');label.textContent='ready';root.replaceChildren(label);
        Object.defineProperty(Object.prototype,'viewId',{set(){throw Error('PRIVATE_SETTER')}});
        Object.prototype.toJSON=()=>{throw Error('PRIVATE_JSON')};
      },update({snapshot}){label.textContent=snapshot.viewId+':'+snapshot.revision;}});"#;
    let (mut session, _) = Session::mount(
        binding(),
        source.into(),
        snapshot(6, 1),
        resources(),
        &|| false,
    )
    .unwrap();
    assert_eq!(
        text(&session.select_view("safe", &|| false).unwrap().scene),
        "safe:6"
    );
    session.dispose(&|| false).unwrap();
}

#[test]
fn cancelled_or_failed_view_update_reclaims_child_without_private_error_leak() {
    let _serial = fixture::SERIAL.lock().unwrap();
    for cancelled in [false, true] {
        let source = "NeuroSurface.define({mount({root}){root.textContent='ready'},update(){throw Error('PRIVATE_VIEW')}});";
        let (mut session, _) = Session::mount(
            binding(),
            source.into(),
            snapshot(1, 1),
            resources(),
            &|| false,
        )
        .unwrap();
        let process = session.child.as_ref().unwrap().probe().unwrap();
        let error = session.select_view("main", &|| cancelled).err().unwrap();
        assert!(!error.to_string().contains("PRIVATE_VIEW"));
        assert!(process.exited());
        assert!(session.child.is_none());
        assert_eq!(session.revision, 1);
    }
    assert_eq!(CAPACITY.used(), 0);
}

#[test]
fn local_view_preserves_remote_update_monotonicity_and_next_authority() {
    let _serial = fixture::SERIAL.lock().unwrap();
    let (mut session, first) = Session::mount(
        binding(),
        fixture::SOURCE.into(),
        snapshot(8, 42),
        resources(),
        &|| false,
    )
    .unwrap();
    let button = first.scene.root().children()[1].id().to_owned();
    session.select_view("main", &|| false).unwrap();
    assert!(session
        .update(snapshot(8, 99), resources(), &|| false)
        .is_err());
    let mut next = snapshot(9, 43);
    next["viewId"] = json!("main");
    session.update(next, resources(), &|| false).unwrap();
    session.select_view("detail", &|| false).unwrap();
    let frame = session.click(&button, &|| false).unwrap();
    assert_eq!(text(&frame.scene), "原生🙂:43:1");
    let event: Value = serde_json::from_slice(frame.events[0].bytes()).unwrap();
    assert_eq!(event["baseRevision"], 9);
    assert_eq!(event["payload"]["quote"], 43);
    session.dispose(&|| false).unwrap();
}
