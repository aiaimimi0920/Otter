//! 独立对象/数值oracle：32项确认边界、期间编辑、日志基线推进与原请求围栏。
use super::*;
use crate::projection_edit::{fixture::*, view::Size, workspace::Role, Operation};
use serde_json::json;
use std::sync::atomic::AtomicBool;

fn snapshot() -> Snapshot {
    let document = mode_document();
    let view = View::from_document(&document, Size::new(32.0, 16.0).unwrap()).unwrap();
    Snapshot::new(
        context("https://loom.example"),
        PROJECTION,
        protocol::Route::Shared,
        Role::Receiver,
        document,
        &view,
        None,
    )
    .unwrap()
}
#[test]
fn submission_freezes_only_first_32_changes_and_leaves_unsent_objects_pending() {
    let snapshot = snapshot();
    let mut objects = snapshot.document().live_objects();
    for i in 0..40 {
        let id = format!("local-{i:02}");
        objects.insert(id.clone(), annotation(&id));
    }
    let current = View::from_basis(
        snapshot.document().basis(),
        Size::new(32.0, 16.0).unwrap(),
        objects,
    )
    .unwrap();
    let snapshot = snapshot.with_view(&current).unwrap();
    let (request, acknowledged) = prepare(&snapshot, "op-original".into()).unwrap();
    let Operation::Apply {
        changes,
        base_revision,
        mode_revision,
        op_id,
        ..
    } = request.operation()
    else {
        panic!()
    };
    assert_eq!(
        (*base_revision, *mode_revision, op_id.as_str()),
        (2, 2, "op-original")
    );
    assert_eq!(changes.len(), 32);
    assert_eq!(changes[0].object_id, "local-00");
    assert_eq!(changes[31].object_id, "local-31");
    assert_eq!(acknowledged.len(), 33);
    assert!(!acknowledged.contains_key("local-32"));
    assert!(acknowledged.contains_key("shape-1"));
}
#[test]
fn response_keeps_during_network_edit_and_undo_does_not_revive_old_pending() {
    let snapshot = snapshot();
    let mut sent = snapshot.document().live_objects();
    sent.get_mut("shape-1").unwrap()["x"] = json!(8);
    let acknowledged = View::from_basis(
        snapshot.document().basis(),
        Size::new(32.0, 16.0).unwrap(),
        sent.clone(),
    )
    .unwrap();
    let mut later = sent;
    later.get_mut("shape-1").unwrap()["x"] = json!(12);
    let current =
        View::from_basis(snapshot.document().basis(), acknowledged.size(), later).unwrap();
    let mut response = serde_json::to_value(snapshot.document()).unwrap();
    response["revision"] = json!(3);
    response["receiptCount"] = json!(2);
    response["objects"]["shape-1"]["revision"] = json!(3);
    response["objects"]["shape-1"]["value"]["x"] = json!(8);
    let document = Document::parse(&serde_json::to_vec(&response).unwrap()).unwrap();
    let next = snapshot
        .applied(document, snapshot.document(), &acknowledged, &current)
        .unwrap();
    assert_eq!(next.document().objects()["shape-1"].value()["x"], json!(8));
    assert_eq!(
        next.editor_view().unwrap().basis_objects()["shape-1"]["x"],
        json!(12)
    );
    assert_eq!(next.conflict(), None);
    let undone = next.with_view(&acknowledged).unwrap();
    assert!(View::from_document(undone.document(), acknowledged.size())
        .unwrap()
        .changes_to(&undone.editor_view().unwrap())
        .unwrap()
        .is_empty());
}
#[test]
fn readonly_conflict_and_suspended_or_empty_changes_do_not_create_write_request() {
    let snapshot = snapshot();
    assert!(prepare(&snapshot, "op-empty".into()).is_err());
    let local = View::from_basis(
        snapshot.document().basis(),
        Size::new(32.0, 16.0).unwrap(),
        Default::default(),
    )
    .unwrap();
    let changed = snapshot.with_view(&local).unwrap();
    assert!(prepare(&changed, "op-delete".into()).is_ok());
    assert!(prepare(
        &changed.suspend(protocol::workspace::Suspension::BasisChanged),
        "op-stop".into()
    )
    .is_err());
    let readonly = Snapshot::new(
        context("https://loom.example"),
        PROJECTION,
        protocol::Route::Shared,
        Role::Receiver,
        document(),
        &View::from_basis(document().basis(), local.size(), Default::default()).unwrap(),
        None,
    )
    .unwrap();
    assert!(prepare(&readonly, "op-readonly".into()).is_err());
    let conflict = Snapshot::new(
        changed.context().clone(),
        PROJECTION,
        changed.route(),
        Role::Receiver,
        changed.document().clone(),
        &local,
        Some(protocol::Code::ObjectConflict),
    )
    .unwrap();
    assert!(prepare(&conflict, "op-conflict".into()).is_err());
}
#[test]
fn observed_baseline_can_advance_only_same_session_and_never_overwrite_pending() {
    let directory = Directory::new();
    let store = directory.store();
    let cancel = AtomicBool::new(false);
    store
        .begin(
            context("https://loom.example"),
            protocol::Route::Shared,
            initialize(),
            None,
            view(),
            &cancel,
        )
        .unwrap()
        .commit_after_checkpoint(document(), &cancel)
        .unwrap();
    let snapshot = snapshot();
    let local = View::from_basis(
        snapshot.document().basis(),
        Size::new(32.0, 16.0).unwrap(),
        Default::default(),
    )
    .unwrap();
    let snapshot = snapshot.with_view(&local).unwrap();
    let (request, acknowledged) = prepare(&snapshot, "op-advance".into()).unwrap();
    assert!(matches!(
        store.begin(
            snapshot.context().clone(),
            snapshot.route(),
            request.clone(),
            Some(snapshot.document().clone()),
            acknowledged.clone(),
            &cancel
        ),
        Err(protocol::journal::Error::Changed)
    ));
    let ticket = store
        .begin_observed(
            snapshot.context().clone(),
            snapshot.route(),
            request.clone(),
            snapshot.document().clone(),
            acknowledged.clone(),
            &cancel,
        )
        .unwrap();
    let original = std::fs::read(directory.path()).unwrap();
    assert!(matches!(
        store.begin_observed(
            snapshot.context().clone(),
            snapshot.route(),
            request,
            snapshot.document().clone(),
            view(),
            &cancel
        ),
        Err(protocol::journal::Error::Busy)
    ));
    assert_eq!(std::fs::read(directory.path()).unwrap(), original);
    assert_eq!(ticket.previous(), Some(snapshot.document()));
}
