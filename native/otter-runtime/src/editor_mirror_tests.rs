//! Mirror metadata/materialized pixels share transactions, identities, and history; snapshots never flip pixels again.
use super::*;
use crate::{erase_layers::Layers, raster_flip::Axis};

fn document() -> Editor {
    Editor::with_layers(Layers {
        size: (2, 2),
        base: [1, 1, 1, 255, 2, 2, 2, 255, 3, 3, 3, 255, 4, 4, 4, 255]
            .to_vec()
            .into(),
        annotation_size: None,
        annotations: None,
    })
    .unwrap()
}
fn flip(editor: &mut Editor, axis: Axis) {
    let plan = editor
        .prepare_flip(axis, |_| unreachable!())
        .unwrap()
        .unwrap();
    assert!(editor
        .commit_raster_refresh(plan.compute(|| false).unwrap())
        .unwrap());
}
#[test]
fn both_axes_pixels_pressed_and_history_move_in_one_step() {
    let mut editor = document();
    let initial = editor.layers().unwrap().base.clone();
    let mut states = vec![(editor.mirror_state(), initial.clone())];
    for axis in [
        Axis::Horizontal,
        Axis::Vertical,
        Axis::Horizontal,
        Axis::Vertical,
    ] {
        let expected = editor.mirror_state().toggled(axis);
        flip(&mut editor, axis);
        assert_eq!(editor.mirror_state(), expected);
        states.push((expected, editor.layers().unwrap().base.clone()));
    }
    assert_eq!(editor.mirror_state(), MirrorState::default());
    assert_eq!(editor.layers().unwrap().base, initial);
    for (mirror, base) in states[..4].iter().rev() {
        assert!(editor.undo());
        assert_eq!(editor.mirror_state(), *mirror);
        assert_eq!(editor.layers().unwrap().base, *base);
    }
    assert!(!editor.undo());
    for (mirror, base) in &states[1..] {
        assert!(editor.redo());
        assert_eq!(editor.mirror_state(), *mirror);
        assert_eq!(editor.layers().unwrap().base, *base);
    }
    assert!(!editor.redo());
    assert!(editor.undo());
    flip(&mut editor, Axis::Horizontal);
    assert!(!editor.redo());
    assert_eq!(editor.mirror_state(), MirrorState { x: true, y: true });
}
#[test]
fn cancelled_stale_and_other_document_plans_cannot_change_pressed_state() {
    let mut editor = document();
    let plan = editor
        .prepare_flip(Axis::Horizontal, |_| unreachable!())
        .unwrap()
        .unwrap();
    assert!(plan.compute(|| true).is_err());
    assert_eq!(editor.mirror_state(), MirrorState::default());
    let plan = editor
        .prepare_flip(Axis::Horizontal, |_| unreachable!())
        .unwrap()
        .unwrap()
        .compute(|| false)
        .unwrap();
    let mut other = document();
    assert!(!other.commit_raster_refresh(plan).unwrap());
    assert_eq!(other.mirror_state(), MirrorState::default());
    let stale = editor
        .prepare_flip(Axis::Vertical, |_| unreachable!())
        .unwrap()
        .unwrap()
        .compute(|| false)
        .unwrap();
    flip(&mut editor, Axis::Horizontal);
    assert!(!editor.commit_raster_refresh(stale).unwrap());
    assert_eq!(editor.mirror_state(), MirrorState { x: true, y: false });
    assert!(editor.with_restored_mirror(MirrorState::default()).is_err());
}
#[test]
fn snapshots_adoption_and_undo_keep_orientation_without_reflecting_pixels_again() {
    let mut source = document();
    flip(&mut source, Axis::Horizontal);
    let base = source.layers().unwrap().base.clone();
    let snapshot = source.export_snapshot();
    let mut pasted = source.duplicate_snapshot().unwrap();
    for editor in [&snapshot, &pasted] {
        assert_eq!(editor.mirror_state(), source.mirror_state());
        assert!(editor.layers().unwrap().base.same_storage(&base));
    }
    assert!(!pasted.undo());
    assert_eq!(
        source.export_overlay_snapshot().mirror_state(),
        MirrorState::default()
    );
    assert_eq!(
        source.export_without_annotations().mirror_state(),
        source.mirror_state()
    );
    let mut target = document();
    let original = target.layers().unwrap().base.clone();
    assert!(target.adopt_snapshot(target.revision(), &source).unwrap());
    assert_eq!(target.mirror_state(), source.mirror_state());
    assert!(target.undo());
    assert_eq!(target.mirror_state(), MirrorState::default());
    assert_eq!(target.layers().unwrap().base, original);
    assert!(target.redo());
    assert_eq!(target.mirror_state(), source.mirror_state());
    assert_eq!(target.layers().unwrap().base, base);
}
#[test]
fn annotation_free_basis_preserves_raster_geometry_but_owns_no_annotations_or_history() {
    let mut source = document();
    source.begin(Tool::Brush, Point { x: 0.0, y: 0.0 }).unwrap();
    source.extend(Point { x: 1.0, y: 1.0 }).unwrap();
    assert!(source.commit());
    flip(&mut source, Axis::Horizontal);
    let stamp = source.document_stamp();
    let mut basis = source.export_without_annotations();
    assert!(basis.elements().is_empty());
    assert!(!basis.has_edit_state());
    assert!(!basis.undo() && !basis.redo());
    assert!(!stamp.same_document(&basis));
    assert_eq!(basis.revision(), source.revision());
    assert_eq!(basis.mirror_state(), source.mirror_state());
    assert_eq!(basis.crop_view(), source.crop_view());
    assert_eq!(basis.visible_frame(), source.visible_frame());
    assert!(basis
        .layers()
        .unwrap()
        .base
        .same_storage(&source.layers().unwrap().base));
    assert_eq!(source.elements().len(), 1);
    assert!(stamp.matches(&source));
}
#[test]
fn stored_metadata_overrides_legacy_flags_and_rejects_invalid_types() {
    let read = |text: &str| {
        let raw = serde_json::value::RawValue::from_string(text.into()).unwrap();
        MirrorState::read_raw(Some(&raw))
    };
    assert_eq!(MirrorState::read_raw(None).unwrap(), MirrorState::default());
    assert_eq!(
        read(r#"{"flippedX":true}"#).unwrap(),
        MirrorState { x: true, y: false }
    );
    assert_eq!(
        read(r#"{"flippedX":true,"otterNativeMaterializedMirror":{"x":false,"y":true}}"#).unwrap(),
        MirrorState { x: false, y: true }
    );
    for text in [
        r#"{"otterNativeMaterializedMirror":{"x":1,"y":false}}"#,
        r#"{"otterNativeMaterializedMirror":{"x":true}}"#,
        r#"{"flippedX":"true"}"#,
        "[]",
    ] {
        assert!(read(text).is_err(), "{text}");
    }
    let mut stored = serde_json::json!({"future":42,"flippedX":false});
    MirrorState { x: true, y: false }
        .write(&mut stored)
        .unwrap();
    assert_eq!(stored["future"], 42);
    assert_eq!(stored["flippedX"], false);
    assert_eq!(
        read(&stored.to_string()).unwrap(),
        MirrorState { x: true, y: false }
    );
}
