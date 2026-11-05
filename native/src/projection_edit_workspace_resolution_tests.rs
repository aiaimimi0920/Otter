//! 独立wire数值oracle，显式选择不能复活undo或吞掉read期间编辑/单向约束。
use super::*;
use crate::projection_edit::{fixture::*, Mode};
use serde_json::json;
fn parse(value: serde_json::Value) -> Document {
    Document::parse(&serde_json::to_vec(&value).unwrap()).unwrap()
}
fn snapshot() -> Snapshot {
    let document = mode_document();
    let mut objects = document.live_objects();
    objects.get_mut("shape-1").unwrap()["x"] = json!(8);
    let view = View::from_basis(document.basis(), Size::new(32.0, 16.0).unwrap(), objects).unwrap();
    Snapshot::new(
        context("https://loom.example"),
        PROJECTION,
        Route::Shared,
        Role::Receiver,
        document,
        &view,
        Some(Code::ObjectConflict),
    )
    .unwrap()
}
fn remote(snapshot: &Snapshot, mode: Mode) -> Document {
    let mut value = serde_json::to_value(snapshot.document()).unwrap();
    value["revision"] = json!(3);
    value["receiptCount"] = json!(2);
    if mode != snapshot.document().mode() {
        value["modeRevision"] = json!(3);
    }
    value["objects"]["shape-1"]["revision"] = json!(3);
    value["objects"]["shape-1"]["value"]["x"] = json!(6);
    value["mode"] = serde_json::to_value(mode).unwrap();
    parse(value)
}
#[test]
fn explicit_keep_local_or_remote_rebases_only_the_chosen_view_not_the_raster() {
    let snapshot = snapshot();
    let current = snapshot.editor_view().unwrap();
    for keep in [false, true] {
        let next = snapshot
            .resolved(remote(&snapshot, Mode::TwoWay), &current, &current, keep)
            .unwrap();
        assert_eq!(next.document().objects()["shape-1"].value()["x"], json!(6));
        assert_eq!(
            next.editor_view().unwrap().basis_objects()["shape-1"]["x"],
            json!(if keep { 8 } else { 6 })
        );
        assert_eq!(next.conflict(), None);
        assert_eq!(next.context(), snapshot.context());
        assert_eq!(
            next.to_view().unwrap().components(),
            snapshot.to_view().unwrap().components()
        );
    }
}
#[test]
fn adopt_remote_preserves_later_edit_and_marks_real_race_instead_of_overwriting() {
    let snapshot = snapshot();
    let observed = snapshot.editor_view().unwrap();
    let mut objects = observed.basis_objects().clone();
    objects.get_mut("shape-1").unwrap()["x"] = json!(12);
    let current = View::from_basis(observed.basis(), observed.size(), objects).unwrap();
    let next = snapshot
        .resolved(remote(&snapshot, Mode::TwoWay), &observed, &current, false)
        .unwrap();
    assert_eq!(
        next.editor_view().unwrap().basis_objects()["shape-1"]["x"],
        json!(12)
    );
    assert_eq!(next.conflict(), Some(Code::ObjectConflict));
    let no_later = snapshot
        .resolved(remote(&snapshot, Mode::OneWay), &observed, &observed, false)
        .unwrap();
    assert_eq!(no_later.conflict(), None);
    let one_way = snapshot
        .resolved(remote(&snapshot, Mode::OneWay), &observed, &current, false)
        .unwrap();
    assert_eq!(one_way.conflict(), Some(Code::ReadOnly));
}
#[test]
fn keep_local_after_undo_does_not_restore_stale_conflict_payload() {
    let snapshot = snapshot();
    let observed = snapshot.editor_view().unwrap();
    let current = View::from_document(snapshot.document(), observed.size()).unwrap();
    let next = snapshot
        .resolved(remote(&snapshot, Mode::TwoWay), &observed, &current, true)
        .unwrap();
    assert_eq!(
        next.editor_view().unwrap().basis_objects()["shape-1"]["x"],
        json!(6)
    );
    assert_eq!(next.conflict(), Some(Code::ObjectConflict));
}
#[test]
fn read_only_changed_basis_or_session_and_suspended_view_cannot_be_forced() {
    let snapshot = snapshot();
    let current = snapshot.editor_view().unwrap();
    assert!(snapshot
        .resolved(remote(&snapshot, Mode::OneWay), &current, &current, true)
        .is_err());
    for (field, value) in [
        ("sessionId", json!("edit:33333333333333333333333333333333")),
        (
            "basis",
            json!({"digest":"b".repeat(64),"width":32,"height":16}),
        ),
    ] {
        let mut wire = serde_json::to_value(remote(&snapshot, Mode::TwoWay)).unwrap();
        wire[field] = value;
        assert!(snapshot
            .resolved(parse(wire), &current, &current, false)
            .is_err());
    }
    assert!(snapshot
        .suspend(Suspension::BasisChanged)
        .resolved(remote(&snapshot, Mode::TwoWay), &current, &current, false)
        .is_err());
}
