//! 独立JSON常量核记录与canonical绑定，不以新映射器生成预期。
use super::*;
use crate::projection_edit::fixture;
use serde_json::json;
pub(super) fn snapshot() -> Snapshot {
    let mut document = fixture::document_value();
    document["objects"]["shape-1"]["value"]["x"] = json!(3.123456789123);
    document["objects"]["shape-1"]["value"]["style"]["color"] = json!("rgba(1, 2, 3, 0.500000001)");
    let document = Document::parse(&serde_json::to_vec(&document).unwrap()).unwrap();
    let view = View::from_document(&document, Size::new(64.0, 32.0).unwrap()).unwrap();
    Snapshot::new(
        fixture::context("https://loom.example"),
        fixture::PROJECTION,
        Route::Offline,
        Role::Receiver,
        document,
        &view,
        Some(Code::ObjectConflict),
    )
    .unwrap()
}
fn window() -> Map<String, Value> {
    json!({"id":"unit-1","x":0,"y":0,"w":64,"h":32,"src":"data:image/png;base64,AQ==",
        "annotationState":{"elements":[{"id":"shape-1","x":6.246913578246}],"serialCounter":0}})
    .as_object()
    .unwrap()
    .clone()
}
fn restore(fields: Map<String, Value>, scope: Option<Scope>) -> Result<Option<Snapshot>, Error> {
    let raw = serde_json::to_vec(&json!({"stickers":[fields],"links":[]})).unwrap();
    let document = crate::session_document::decode(&raw).unwrap();
    Snapshot::restore(&document.windows[0], scope)
}
fn assert_invalid_snapshot_is_rejected(invalid: &Snapshot) {
    let mut fields = window();
    let before = fields.clone();
    assert_eq!(invalid.install(&mut fields), Err(Error::Format));
    assert_eq!(
        fields, before,
        "failed validation must not publish a record"
    );
    let valid = snapshot();
    valid.install(&mut fields).unwrap();
    // 内容摘要不变，只破坏持久化 snapshot，确保 restore 也执行同一校验。
    fields.get_mut(FIELD).unwrap()["snapshot"] = serde_json::to_value(invalid).unwrap();
    assert_eq!(
        restore(fields, Some(valid.context().scope())),
        Err(Error::Format)
    );
}
#[test]
fn visible_status_comparison_matches_view_and_ignores_authority_tombstones() {
    let mut state = snapshot();
    let mut wire = serde_json::to_value(state.document()).unwrap();
    wire["objects"]["deleted"] = json!({"revision":1,"value":null});
    state.document = Document::parse(&serde_json::to_vec(&wire).unwrap()).unwrap();
    let check = |state: &Snapshot, changed| {
        assert_eq!(state.visible_differs_from_authority(), changed);
        assert_eq!(
            state.editor_view().unwrap().basis_objects() != &state.document().live_objects(),
            changed
        );
    };
    check(&state, false);
    let original = state.visible.get("shape-1").unwrap().clone();
    state.visible.get_mut("shape-1").unwrap()["x"] = json!(8);
    check(&state, true);
    state.visible.clear();
    check(&state, true);
    state.visible.insert("shape-1".into(), original);
    let deleted = fixture::annotation("deleted");
    state.visible.insert("deleted".into(), deleted);
    check(&state, true);
}
#[test]
fn snapshot_validation_keeps_real_layout_mapping_and_visible_boundaries() {
    for (field, value) in [
        ("width", json!(0)),
        ("height", json!(1_000_001)),
        ("scale", json!(0)),
        ("offsetX", json!(f32::MAX)),
    ] {
        let mut raw = serde_json::to_value(snapshot()).unwrap();
        raw["layout"][field] = value;
        let invalid = serde_json::from_value(raw).unwrap();
        assert_invalid_snapshot_is_rejected(&invalid);
    }
    let mut invalid = snapshot();
    invalid.visible.get_mut("shape-1").unwrap()["type"] = json!("unknown");
    assert_invalid_snapshot_is_rejected(&invalid);
}
#[test]
fn snapshot_validation_keeps_independent_document_and_visible_byte_budgets() {
    let mut objects = Map::new();
    let mut visible = Objects::new();
    for index in 0..64 {
        let id = format!("text-{index}");
        let value = json!({"id":id,"type":"text","zIndex":0,"x":0,"y":0,
            "style":{"color":"#000","width":1},"text":"x".repeat(4096)});
        assert!(crate::projection_edit::objects::annotation(&id, &value));
        visible.insert(id.clone(), value.clone());
        objects.insert(id, json!({"revision":1,"value":value}));
    }
    let mut invalid = snapshot();
    let mut document = serde_json::to_value(&invalid.document).unwrap();
    document["objects"] = objects.into();
    invalid.document = serde_json::from_value(document).unwrap();
    assert_eq!(
        invalid.document.validate(),
        Err(crate::projection_edit::Error::Limit)
    );
    assert!(
        invalid.editor_view().is_ok(),
        "visible remains within budget"
    );
    assert_invalid_snapshot_is_rejected(&invalid);

    let mut invalid = snapshot();
    invalid.visible = visible;
    assert!(invalid.document.validate().is_ok());
    assert_eq!(
        View::from_basis(
            invalid.document.basis(),
            Size::new(64.0, 32.0).unwrap(),
            invalid.visible.clone()
        ),
        Err(crate::projection_edit::view::Error::Limit)
    );
    assert_eq!(invalid.editor_view(), Err(Error::Format));
    assert_invalid_snapshot_is_rejected(&invalid);
}
#[test]
fn authoritative_and_visible_basis_survive_exact_record_roundtrip_without_lexical_loss() {
    let original = snapshot();
    let mut visible = original
        .view(Size::new(64.0, 32.0).unwrap())
        .unwrap()
        .local_objects()
        .unwrap();
    visible.get_mut("shape-1").unwrap()["y"] = json!(18.0);
    let edited = original
        .view(Size::new(64.0, 32.0).unwrap())
        .unwrap()
        .edited(visible)
        .unwrap();
    let original = original.with_view(&edited).unwrap();
    let mut fields = window();
    original.install(&mut fields).unwrap();
    let restored = restore(fields, Some(original.context().scope()))
        .unwrap()
        .unwrap();
    assert_eq!(restored, original);
    assert_eq!(
        restored
            .view(Size::new(32.0, 16.0).unwrap())
            .unwrap()
            .basis_objects()["shape-1"]["y"],
        9.0
    );
    assert_eq!(restored.document().live_objects()["shape-1"]["y"], 4);
    assert_eq!(
        restored.document().live_objects()["shape-1"]["x"],
        json!(3.123456789123)
    );
    assert_eq!(restored.conflict(), Some(Code::ObjectConflict));
    assert!(!restored.document().live_objects()["shape-1"]
        .as_object()
        .unwrap()
        .contains_key("opacity"));
}
#[test]
fn changed_canonical_annotations_image_or_geometry_and_wrong_scope_cannot_revive_old_view() {
    let snapshot = snapshot();
    let scope = snapshot.context().scope();
    let mut fields = window();
    snapshot.install(&mut fields).unwrap();
    for (key, value) in [
        ("annotationState", json!({"elements":[]})),
        ("src", json!("different")),
        ("w", json!(65)),
        ("imageEditState", json!({"flippedX":true})),
    ] {
        let mut changed = fields.clone();
        changed.insert(key.into(), value);
        assert_eq!(
            restore(changed, Some(scope)),
            Err(Error::ContentChanged),
            "{key}"
        );
    }
    assert_eq!(restore(fields.clone(), None), Err(Error::Identity));
    let other = serde_json::from_value(json!("b".repeat(64))).unwrap();
    assert_eq!(restore(fields.clone(), Some(other)), Err(Error::Identity));
    fields.insert("id".into(), json!("unit-2"));
    assert_eq!(restore(fields, Some(scope)), Err(Error::Identity));
}
#[test]
fn suspended_state_is_explicit_and_null_future_or_overbudget_records_are_not_legacy_absence() {
    let snapshot = snapshot().suspend(Suspension::BasisChanged);
    let scope = snapshot.context().scope();
    assert_eq!(restore(window(), None), Ok(None));
    let mut fields = window();
    snapshot.install(&mut fields).unwrap();
    assert_eq!(
        restore(fields.clone(), Some(scope)).unwrap().unwrap(),
        snapshot
    );
    for value in [
        Value::Null,
        json!({"snapshot":{"schema":"future"}}),
        json!("x".repeat(MAX_RECORD)),
    ] {
        fields.insert(FIELD.into(), value);
        assert!(restore(fields.clone(), Some(scope)).is_err());
    }
}
#[test]
fn canonical_key_order_whitespace_and_minified_reference_do_not_change_content_identity() {
    let snapshot = snapshot();
    let mut fields = window();
    fields.insert("minified".into(), json!(true));
    fields.insert("savedRect".into(), json!({"x":7,"y":8,"w":64,"h":32}));
    fields.insert("w".into(), json!(10));
    fields.insert("h".into(), json!(10));
    snapshot.install(&mut fields).unwrap();
    let bytes = serde_json::to_vec_pretty(&json!({"stickers":[fields],"links":[]})).unwrap();
    let document = crate::session_document::decode(&bytes).unwrap();
    assert_eq!(
        Snapshot::restore(&document.windows[0], Some(snapshot.context().scope())).unwrap(),
        Some(snapshot)
    );
}

#[test]
fn checkpoint_mapping_composes_inverse_viewport_without_accumulating_basis_or_inset() {
    use crate::{session_vector_write::Mapping, viewport::Point};
    let snapshot = snapshot();
    let mapping = Mapping::from_components(2.0, Point { x: 20.0, y: 10.0 }).unwrap();
    let saved = snapshot.for_save(mapping).unwrap();
    assert_eq!(
        saved
            .to_view()
            .unwrap()
            .point(Point { x: 28.0, y: 22.0 })
            .unwrap(),
        Point { x: 4.0, y: 6.0 }
    );
    let second = saved
        .for_save(Mapping::from_components(0.5, Point { x: 7.0, y: 3.0 }).unwrap())
        .unwrap();
    assert_eq!(
        second
            .to_view()
            .unwrap()
            .point(Point { x: 21.0, y: 14.0 })
            .unwrap(),
        Point { x: 4.0, y: 6.0 }
    );
    assert_eq!(second.document(), snapshot.document());
    assert_eq!(
        second.editor_view().unwrap(),
        snapshot.editor_view().unwrap()
    );
    assert!(Mapping::from_components(0.0, Point::default()).is_err());
    assert!(Mapping::from_components(f32::INFINITY, Point::default()).is_err());
}
