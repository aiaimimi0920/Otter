//! 真实隔离journal往返：存basis对象与原请求，重开后在新本地尺寸恢复；显式ack不是Application保存。
use super::{test_fixture::*, *};
use crate::projection_edit::{
    fixture::{context, PROJECTION, SESSION},
    journal::Store,
    Operation, Request, Route,
};
use serde_json::json;
use std::{path::PathBuf, sync::atomic::AtomicBool};
fn directory() -> PathBuf {
    let path = PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-parallel-20261029/main-projection-edit-view-1/tmp")
        .join(format!("view-{:?}",unsafe { windows::Win32::System::Com::CoCreateGuid().unwrap() }));
    std::fs::create_dir_all(&path).unwrap();
    path
}
fn initialize(store: &Store) -> Document {
    let initial = document(objects(), 1);
    let request = Request::prepare(Operation::Initialize {
        projection_id: PROJECTION.into(),
        session_id: SESSION.into(),
        expected_digest: basis().digest().into(),
        objects: objects(),
    })
    .unwrap();
    let ticket = store
        .begin(
            context("https://loom.example"),
            Route::Shared,
            request,
            None,
            objects(),
            &AtomicBool::new(false),
        )
        .unwrap();
    ticket
        .commit_after_checkpoint(initial.clone(), &AtomicBool::new(false))
        .unwrap();
    initial
}

#[test]
fn scaled_edit_original_request_and_basis_view_survive_reopen_before_explicit_checkpoint_ack() {
    let directory = directory();
    let cancel = AtomicBool::new(false);
    let store = Store::open(&directory, &cancel).unwrap();
    let previous = initialize(&store);
    let before = view();
    let resized = before.resized(Size::new(240.0, 100.0).unwrap());
    let edited = move_x(&resized, "shape-a", 40.0);
    let changes = before.changes_to(&edited).unwrap();
    let request = Request::prepare(Operation::Apply {
        projection_id: PROJECTION.into(),
        session_id: SESSION.into(),
        op_id: "op-view-original".into(),
        base_revision: 1,
        mode_revision: 1,
        changes,
    })
    .unwrap();
    let expected = json!({"operation":"apply","projectionId":PROJECTION,"sessionId":SESSION,"opId":"op-view-original",
        "baseRevision":1,"modeRevision":1,"changes":[{"objectId":"shape-a","value":{
        "id":"shape-a","type":"rect","zIndex":1.25,"x":20,"y":20,"w":30,"h":25,"style":{"color":"#123456","width":2}}}]});
    assert!(edit_equal(
        &serde_json::from_slice(request.bytes()).unwrap(),
        &expected
    ));
    let original = request.bytes().to_vec();
    let ticket = store
        .begin(
            context("https://loom.example"),
            Route::Offline,
            request,
            Some(previous.clone()),
            edited.basis_objects().clone(),
            &cancel,
        )
        .unwrap();
    let saved = std::fs::read(directory.join("projection-edit.json")).unwrap();
    drop((ticket, store));
    let reopened = Store::open(&directory, &cancel).unwrap();
    let pending = reopened.pending(&cancel).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].request().bytes(), original);
    assert_eq!(pending[0].route(), Route::Offline);
    let restored = View::from_basis(
        pending[0].previous().unwrap().basis(),
        Size::new(120.0, 300.0).unwrap(),
        pending[0].view().clone(),
    )
    .unwrap();
    assert_eq!(
        restored.local_objects().unwrap()["shape-a"]["style"]["width"],
        4.0
    );
    assert_eq!(restored.local_objects().unwrap()["shape-a"]["x"], 20.0);
    let remote = document(edited.basis_objects().clone(), 2);
    let returned = pending[0]
        .request()
        .response(&serde_json::to_vec(&remote).unwrap(), Some(&previous))
        .unwrap();
    assert_eq!(reopened.pending(&cancel).unwrap().len(), 1); // HTTP/解析成功不能清pending。
    assert_eq!(
        std::fs::read(directory.join("projection-edit.json")).unwrap(),
        saved
    );
    let current = move_x(&restored, "shape-a", 30.0);
    let result = reconcile(&previous, &edited, &returned, &current, None).unwrap();
    assert!(result.conflict().is_none());
    assert_eq!(result.changes()[0].value["x"], 30.0);
    pending[0]
        .commit_after_checkpoint(returned, &cancel)
        .unwrap();
    assert!(reopened.pending(&cancel).unwrap().is_empty());
}

#[test]
fn invalid_local_merge_or_cancelled_confirmation_cannot_change_durable_request_or_view() {
    let directory = directory();
    let cancel = AtomicBool::new(false);
    let store = Store::open(&directory, &cancel).unwrap();
    let previous = initialize(&store);
    let edited = move_x(&view(), "shape-a", 40.0);
    let request = Request::prepare(Operation::Apply {
        projection_id: PROJECTION.into(),
        session_id: SESSION.into(),
        op_id: "op-retained".into(),
        base_revision: 1,
        mode_revision: 1,
        changes: view().changes_to(&edited).unwrap(),
    })
    .unwrap();
    let ticket = store
        .begin(
            context("https://loom.example"),
            Route::Shared,
            request,
            Some(previous),
            edited.basis_objects().clone(),
            &cancel,
        )
        .unwrap();
    let saved = std::fs::read(directory.join("projection-edit.json")).unwrap();
    let mut invalid = edited.local_objects().unwrap();
    invalid.get_mut("shape-a").unwrap()["style"]["width"] = 9000.into();
    assert!(edited.edited(invalid).is_err());
    let remote = document(edited.basis_objects().clone(), 2);
    assert!(ticket
        .commit_after_checkpoint(remote, &AtomicBool::new(true))
        .is_err());
    assert_eq!(
        std::fs::read(directory.join("projection-edit.json")).unwrap(),
        saved
    );
    assert_eq!(
        store.pending(&cancel).unwrap()[0].view(),
        edited.basis_objects()
    );
}
