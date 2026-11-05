//! 独立wire/坐标oracle：mode/checkpoint不误确认本地编辑；自动申请只产生新的有界请求。
use super::*;
use crate::projection_edit::{fixture::*, view::Size, workspace::Suspension};
use serde_json::json;
fn state(role: Role, document: Document, local: bool) -> Snapshot {
    let mut objects = document.live_objects();
    if local {
        objects.get_mut("shape-1").unwrap()["x"] = json!(9);
    }
    let view = View::from_basis(document.basis(), Size::new(32.0, 16.0).unwrap(), objects).unwrap();
    Snapshot::new(
        context("https://loom.example"),
        PROJECTION,
        protocol::Route::Shared,
        role,
        document,
        &view,
        None,
    )
    .unwrap()
}
#[test]
fn mode_uses_source_authority_base_but_does_not_acknowledge_unsent_objects() {
    let snapshot = state(Role::Source, mode_document(), true);
    let (prepared, acknowledged) =
        request(&snapshot, "mode-fixed".into(), Action::Mode(Mode::OneWay)).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(prepared.bytes()).unwrap(),
        json!({"operation":"mode","projectionId":PROJECTION,"sessionId":SESSION,"opId":"mode-fixed","baseModeRevision":2,"mode":"one_way"})
    );
    assert_eq!(acknowledged["shape-1"]["x"], 3);
    let mut value = serde_json::to_value(snapshot.document()).unwrap();
    value["revision"] = json!(3);
    value["modeRevision"] = json!(3);
    value["mode"] = json!("one_way");
    let document = Document::parse(&serde_json::to_vec(&value).unwrap()).unwrap();
    let current = snapshot.editor_view().unwrap();
    let acknowledged = View::from_basis(document.basis(), current.size(), acknowledged).unwrap();
    let next = snapshot
        .applied(document, snapshot.document(), &acknowledged, &current)
        .unwrap();
    assert_eq!(
        next.editor_view().unwrap().basis_objects()["shape-1"]["x"],
        9
    );
    assert!(request(
        &state(Role::Receiver, mode_document(), true),
        "forbidden".into(),
        Action::Mode(Mode::OneWay)
    )
    .is_err());
}
#[test]
fn automatic_checks_role_conflict_suspension_and_sends_only_32_new_changes() {
    assert!(!automatic(&state(Role::Receiver, document(), true)));
    assert!(!automatic(&state(Role::Source, document(), false)));
    let source = state(Role::Source, document(), true);
    assert!(automatic(&source));
    assert!(!automatic(&source.suspend(Suspension::BasisChanged)));
    let conflict = source.rejected(protocol::Code::ObjectConflict).unwrap();
    assert!(!automatic(&conflict));
    let receiver = state(Role::Receiver, mode_document(), true);
    assert!(automatic(&receiver));
    let mut objects = receiver.editor_view().unwrap().basis_objects().clone();
    for i in 0..40 {
        let id = format!("local-{i:02}");
        objects.insert(id.clone(), annotation(&id));
    }
    let view = View::from_basis(
        receiver.document().basis(),
        Size::new(32.0, 16.0).unwrap(),
        objects,
    )
    .unwrap();
    let snapshot = receiver.with_view(&view).unwrap();
    let (request, acknowledged) =
        request(&snapshot, "automatic-fixed".into(), Action::Automatic).unwrap();
    let Operation::Apply { changes, .. } = request.operation() else {
        panic!()
    };
    assert_eq!(changes.len(), 32);
    assert!(!acknowledged.contains_key("local-32"));
}
#[test]
fn automatic_source_checkpoint_uses_original_threshold_and_keeps_local_edits_unacknowledged() {
    let mut value = serde_json::to_value(document()).unwrap();
    value["revision"] = json!(225);
    for (receipts, expected) in [(223, false), (224, true)] {
        value["receiptCount"] = json!(receipts);
        let document = Document::parse(&serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(automatic(&state(Role::Source, document, false)), expected);
    }
    let document = Document::parse(&serde_json::to_vec(&value).unwrap()).unwrap();
    let source = state(Role::Source, document, true);
    let (request, acknowledged) =
        request(&source, "checkpoint-fixed".into(), Action::Automatic).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(request.bytes()).unwrap(),
        json!({"operation":"checkpoint","projectionId":PROJECTION,"sessionId":SESSION,"expectedRevision":225})
    );
    assert_eq!(acknowledged["shape-1"]["x"], 3);
    assert!(!automatic(&state(
        Role::Receiver,
        source.document().clone(),
        false
    )));
}
#[test]
fn automatic_tombstone_reclaim_requires_240_objects_and_never_runs_on_receiver() {
    for (count, tombstone, expected) in [(239, true, false), (240, false, false), (240, true, true)]
    {
        let mut value = serde_json::to_value(document()).unwrap();
        let objects = value["objects"].as_object_mut().unwrap();
        objects.clear();
        for i in 0..count {
            let id = format!("shape-{i}");
            objects.insert(id.clone(),json!({"revision":1,"value":if i==0 && tombstone {serde_json::Value::Null} else {annotation(&id)}}));
        }
        let document = Document::parse(&serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(
            automatic(&state(Role::Source, document.clone(), false)),
            expected
        );
        assert!(!automatic(&state(Role::Receiver, document, false)));
    }
}
