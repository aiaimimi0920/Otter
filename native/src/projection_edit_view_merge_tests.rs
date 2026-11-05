//! 权威值与可见本地值分别核验；异步保存后重读，不把旧overlay叠回undo结果。
use super::{test_fixture::*, *};
use crate::projection_edit::Code;
use serde_json::json;

#[test]
fn remote_and_local_different_objects_merge_without_conflicting_or_renumbering() {
    let mut original = objects();
    original.insert("shape-b".into(), rect("shape-b"));
    let previous = document(original.clone(), 1);
    let observed = View::from_document(&previous, Size::new(120.0, 100.0).unwrap()).unwrap();
    let current = move_x(&observed, "shape-a", 40.0);
    let mut remote = original;
    remote.get_mut("shape-b").unwrap()["y"] = 60.into();
    let remote = document(remote, 2);
    let result = reconcile(&previous, &observed, &remote, &current, None).unwrap();
    assert!(result.conflict().is_none());
    assert_eq!(result.authoritative().basis_objects()["shape-a"]["x"], 10);
    assert_eq!(result.visible().basis_objects()["shape-a"]["x"], 40.0);
    assert_eq!(result.visible().basis_objects()["shape-b"]["y"], 60);
    assert_eq!(result.visible().basis_objects()["shape-b"]["zIndex"], 1.25);
    assert_eq!(result.changes().len(), 1);
    assert_eq!(previous.live_objects()["shape-b"]["y"], 20);
}

#[test]
fn same_object_race_keeps_authoritative_remote_and_local_visible_changes_separate() {
    let observed = view();
    let previous = document(objects(), 1);
    let mut remote = objects();
    remote.get_mut("shape-a").unwrap()["x"] = 60.into();
    let remote = document(remote, 2);
    let mut local = observed.local_objects().unwrap();
    local.get_mut("shape-a").unwrap()["x"] = 80.into();
    local.insert("local-new".into(), rect("local-new"));
    let current = observed.edited(local).unwrap();
    let result = reconcile(&previous, &observed, &remote, &current, None).unwrap();
    assert_eq!(result.authoritative().basis_objects()["shape-a"]["x"], 60);
    assert_eq!(result.visible().basis_objects()["shape-a"]["x"], 80.0);
    assert!(result.visible().basis_objects().contains_key("local-new"));
    let conflict = result.conflict().unwrap();
    assert_eq!(conflict.code(), Code::ObjectConflict);
    assert_eq!(conflict.changes(), result.changes());
    assert_eq!(conflict.changes().len(), 2);
    assert!(!result
        .authoritative()
        .basis_objects()
        .contains_key("local-new"));
}

#[test]
fn local_deletion_remote_tombstone_and_concurrent_creation_are_object_scoped_conflicts() {
    let previous = document(objects(), 1);
    let observed = view();
    let current = observed.edited(Objects::new()).unwrap();
    let mut remote = objects();
    remote.get_mut("shape-a").unwrap()["x"] = 60.into();
    let result = reconcile(&previous, &observed, &document(remote, 2), &current, None).unwrap();
    assert_eq!(result.conflict().unwrap().code(), Code::ObjectConflict);
    assert!(result.visible().basis_objects().is_empty());
    assert!(result.changes()[0].value.is_null());
    let mut tombstone = serde_json::to_value(&previous).unwrap();
    tombstone["revision"] = 2.into();
    tombstone["objects"]["shape-a"] = json!({"revision":2,"value":null});
    let tombstone = parse_document(tombstone);
    let moved = move_x(&observed, "shape-a", 80.0);
    let result = reconcile(&previous, &observed, &tombstone, &moved, None).unwrap();
    assert!(result.authoritative().basis_objects().is_empty());
    assert_eq!(result.visible().basis_objects()["shape-a"]["x"], 80.0);
    assert!(result.conflict().is_some());
    let result = reconcile(&previous, &observed, &tombstone, &observed, None).unwrap();
    assert!(result.visible().basis_objects().is_empty());
    assert!(result.conflict().is_none()); // 没有本地修改，不复活已删对象。
    let empty_doc = document(Objects::new(), 1);
    let empty = View::from_document(&empty_doc, observed.size()).unwrap();
    let result = reconcile(&empty_doc, &empty, &document(objects(), 2), &observed, None).unwrap();
    assert!(result.conflict().is_some()); // 同ID并发创建，即使值相同也不能冒充已确认写入。
}

#[test]
fn rereading_after_each_async_commit_keeps_newer_local_edits_without_touching_remote_baseline() {
    let previous = document(objects(), 1);
    let observed = view();
    let mut remote = objects();
    remote.get_mut("shape-a").unwrap()["x"] = 60.into();
    let remote = document(remote, 2);
    let first = reconcile(
        &previous,
        &observed,
        &remote,
        &move_x(&observed, "shape-a", 80.0),
        None,
    )
    .unwrap();
    // 模拟调用者保存权威基线期间用户又改到90，保存conflict期间再改到100。
    let second = reconcile(
        &previous,
        &observed,
        &remote,
        &move_x(&observed, "shape-a", 90.0),
        first.conflict().map(Conflict::code),
    )
    .unwrap();
    let third = reconcile(
        &previous,
        &observed,
        &remote,
        &move_x(&observed, "shape-a", 100.0),
        second.conflict().map(Conflict::code),
    )
    .unwrap();
    assert_eq!(first.visible().basis_objects()["shape-a"]["x"], 80.0);
    assert_eq!(second.visible().basis_objects()["shape-a"]["x"], 90.0);
    assert_eq!(third.visible().basis_objects()["shape-a"]["x"], 100.0);
    assert_eq!(third.authoritative().basis_objects()["shape-a"]["x"], 60);
    assert_eq!(third.conflict().unwrap().changes()[0].value["x"], 100.0);
}

#[test]
fn apply_reply_uses_original_pending_view_as_acknowledged_baseline_not_old_document() {
    let previous = document(objects(), 1);
    let original = view();
    let acknowledged = move_x(&original, "shape-a", 20.0);
    let remote = document(acknowledged.basis_objects().clone(), 2);
    let resized = acknowledged.resized(Size::new(240.0, 100.0).unwrap());
    let current = move_x(&resized, "shape-a", 50.0); // basis x=25，只保留请求后另改的5。
    let result = reconcile(&previous, &acknowledged, &remote, &current, None).unwrap();
    assert!(result.conflict().is_none());
    assert_eq!(result.changes().len(), 1);
    assert_eq!(result.changes()[0].value["x"], 25.0);
    assert_eq!(
        result.authoritative().local_objects().unwrap()["shape-a"]["x"],
        40.0
    );
    assert_eq!(
        result.visible().local_objects().unwrap()["shape-a"]["x"],
        50.0
    );
}

#[test]
fn undo_clears_outdated_conflict_payload_but_never_silently_clears_sticky_conflict_status() {
    let previous = document(objects(), 1);
    let observed = view();
    let mut remote = objects();
    remote.get_mut("shape-a").unwrap()["x"] = 60.into();
    let remote = document(remote, 2);
    let first = reconcile(
        &previous,
        &observed,
        &remote,
        &move_x(&observed, "shape-a", 80.0),
        None,
    )
    .unwrap();
    let undone = reconcile(
        &previous,
        &observed,
        &remote,
        &observed,
        first.conflict().map(Conflict::code),
    )
    .unwrap();
    assert!(undone.changes().is_empty());
    assert!(undone.conflict().unwrap().changes().is_empty());
    assert_eq!(undone.conflict().unwrap().code(), Code::ObjectConflict);
    assert_eq!(undone.visible().basis_objects()["shape-a"]["x"], 60);
    for code in [Code::ReadOnly, Code::ModeConflict, Code::ObjectBudget] {
        assert_eq!(
            reconcile(&previous, &observed, &previous, &observed, Some(code))
                .unwrap()
                .conflict()
                .unwrap()
                .code(),
            code
        );
    }
}

#[test]
fn stale_revision_changed_session_basis_or_invalid_document_never_produce_a_merge() {
    let previous = document(objects(), 2);
    let observed = view();
    assert!(reconcile(
        &previous,
        &observed,
        &document(objects(), 1),
        &observed,
        None
    )
    .is_err());
    let mut changed = serde_json::to_value(&previous).unwrap();
    changed["objects"]["shape-a"]["value"]["x"] = 80.into();
    assert!(reconcile(
        &previous,
        &observed,
        &parse_document(changed),
        &observed,
        None
    )
    .is_err());
    for (key, value) in [
        ("sessionId", json!("edit:33333333333333333333333333333333")),
        (
            "basis",
            json!({"digest":"b".repeat(64),"width":120,"height":100}),
        ),
    ] {
        let mut changed = serde_json::to_value(&previous).unwrap();
        changed[key] = value;
        assert!(reconcile(
            &previous,
            &observed,
            &parse_document(changed),
            &observed,
            None
        )
        .is_err());
    }
    let mut invalid = serde_json::to_value(&previous).unwrap();
    invalid["modeRevision"] = 0.into();
    let invalid: Document = serde_json::from_value(invalid).unwrap();
    assert!(reconcile(&invalid, &observed, &previous, &observed, None).is_err());
    assert!(reconcile(&previous, &observed, &invalid, &observed, None).is_err());
    assert_eq!(observed.basis_objects(), &objects());
}

#[test]
fn combined_object_budget_failure_retains_both_inputs_instead_of_dropping_any_object() {
    let build = |prefix: &str, x: i32| {
        (0..256)
            .map(|index| {
                let id = format!("{prefix}-{index:03}");
                let mut value = rect(&id);
                value["x"] = x.into();
                (id, value)
            })
            .collect()
    };
    let previous = document(build("old", 10), 1);
    let observed = View::from_document(&previous, Size::new(120.0, 100.0).unwrap()).unwrap();
    let current = View::from_basis(&basis(), observed.size(), build("old", 80)).unwrap();
    let remote = document(build("new", 60), 2);
    assert_eq!(
        reconcile(&previous, &observed, &remote, &current, None),
        Err(Error::Limit)
    );
    assert_eq!(current.basis_objects().len(), 256);
    assert_eq!(remote.live_objects().len(), 256);
    assert_eq!(current.basis_objects()["old-000"]["x"], 80);
}
