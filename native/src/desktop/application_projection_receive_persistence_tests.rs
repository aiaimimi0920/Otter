//! 真正磁盘提交与工作区交换的接收补偿竞态；不以预备索引或下一次 UI poll 冒充保存事实。
use super::*;
use std::sync::atomic::AtomicBool;

unsafe fn prepared(app: &mut Application) -> save_publish::Prepared {
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(prepared) = app.take_save_candidate().unwrap() {
            return prepared;
        }
        assert!(Instant::now() < end, "receive manual save deadline");
        std::thread::yield_now();
    }
}
unsafe fn restore(fixture: &mut Fixture) {
    let space = fixture
        .app
        .restored_session
        .as_ref()
        .unwrap()
        .space
        .unwrap();
    fixture
        .app
        .request_restore(br#"{"stickers":[],"links":[]}"#.to_vec(), None, space)
        .unwrap();
    let end = Instant::now() + Duration::from_secs(10);
    while fixture.app.restore_job.is_some() {
        fixture
            .app
            .poll_restore_with(false, |_| {
                panic!("explicit restore coordinates must not open a dialog")
            })
            .unwrap();
        assert!(Instant::now() < end, "receive workspace exchange deadline");
        std::thread::yield_now();
    }
    assert!(!fixture.unit_present());
}
fn unknown_index(fixture: &mut Fixture) {
    let host = fixture
        .app
        .documents
        .iter()
        .find(|doc| doc.state.unit_id.as_str() != fixture.ticket().unit())
        .unwrap();
    host.state.canvas.borrow_mut().pinned = true; // 使真实默认空宿主成为应保留的普通 Unit。
    let host_id = host.state.unit_id.as_str().to_owned();
    let record = fixture.app.restored_session.as_mut().unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&record.original).unwrap();
    let mut other = value["stickers"][0].clone();
    other["id"] = host_id.into();
    other["qrProjection"] = serde_json::json!({"otterNativeReceive":{}});
    value["stickers"].as_array_mut().unwrap().push(other);
    record.edit_original(serde_json::to_vec(&value).unwrap().into());
}
unsafe fn manual(case: u8) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    let mut fixture = Fixture::new();
    appended(&mut fixture);
    let journal = fixture.journal();
    if case == 3 || case == 5 {
        unknown_index(&mut fixture);
    }
    let path = fixture.files.root().join("manual-received.json");
    if case == 2 {
        fixture.app.request_save_candidate().unwrap();
    } else {
        fixture.app.request_save_new(path.clone()).unwrap();
    }
    let mut save = prepared(&mut fixture.app);
    let candidate = save.candidate.bytes.clone();
    if case == 3 || case == 5 {
        assert!(save.checkpoint_projections.is_none());
        let value: serde_json::Value = serde_json::from_slice(&candidate).unwrap();
        assert!(value["stickers"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |sticker| sticker["qrProjection"]["otterNativeReceive"]["intentId"]
                    == fixture.ticket().intent_id()
            ));
    } else {
        assert!(save
            .checkpoint_projections
            .as_ref()
            .unwrap()
            .is_for(&candidate.clone().into()));
    }
    if case == 1 || case == 5 {
        std::fs::write(&path, b"competing manual file").unwrap();
    }
    if case == 4 {
        let source = fixture
            .app
            .restored_session
            .as_ref()
            .unwrap()
            .original
            .to_vec()
            .into();
        save.checkpoint_projections = std::thread::spawn(move || {
            crate::projection_delivery::binding::checkpoint::Index::capture(&source)
        })
        .join()
        .unwrap();
    }
    if case == 2 {
        let staged = crate::session_file_write::Staged::prepare(
            &path,
            &save.candidate,
            &AtomicBool::new(false),
        )
        .unwrap();
        save.commit_new(&mut fixture.app, staged, &AtomicBool::new(false))
            .unwrap();
    } else {
        let result = save.commit_ready(&mut fixture.app, &AtomicBool::new(false));
        assert_eq!(result.is_err(), case == 1 || case == 4 || case == 5);
    }
    let marked = case == 1 || case == 4 || case == 5;
    if !marked {
        assert_eq!(std::fs::read(&path).unwrap(), candidate.as_ref());
    } else if case == 1 || case == 5 {
        assert_eq!(std::fs::read(&path).unwrap(), b"competing manual file");
    } else {
        assert!(!path.exists());
    }
    assert!(fixture.app.projection_receive.busy());
    assert!(fixture
        .app
        .projection_checkpoint_inclusion(fixture.ticket())
        .is_none());
    // 不 poll receive：覆盖 commit 后同一 UI 轮内立即 reset 的真实竞态。
    fixture.app.reset_projection_inbox();
    assert!(!fixture.app.projection_receive.busy());
    assert!(fixture.unit_present());
    assert_marker(&mut fixture, &journal, marked);
}
#[test]
fn projection_receive_lifecycle_manual_commit_before_default_checkpoint_clears_fresh_synchronously()
{
    unsafe { manual(0) }
}
#[test]
fn projection_receive_lifecycle_failed_manual_commit_cannot_clear_fresh_qualification() {
    unsafe { manual(1) }
}
#[test]
fn projection_receive_lifecycle_late_destination_commit_new_also_clears_fresh_synchronously() {
    unsafe { manual(2) }
}
#[test]
fn projection_receive_lifecycle_unknown_committed_index_degrades_to_explicit_recovery_not_unlink() {
    unsafe { manual(3) }
}
#[test]
fn projection_receive_lifecycle_mismatched_manual_index_is_rejected_before_disk_commit() {
    unsafe { manual(4) }
}
#[test]
fn projection_receive_lifecycle_unknown_index_failed_commit_retains_fresh_cleanup_qualification() {
    unsafe { manual(5) }
}

#[test]
fn projection_receive_lifecycle_unknown_checkpoint_index_never_grants_displayed_or_automatic_unlink(
) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = Fixture::new();
        appended(&mut fixture);
        let journal = fixture.journal();
        unknown_index(&mut fixture);
        checkpoint(&mut fixture.app);
        assert!(fixture
            .app
            .projection_checkpoint_inclusion(fixture.ticket())
            .is_none());
        fixture.app.process_authorization_with_projection(|_| {
            panic!("unknown index must not grant displayed")
        });
        assert!(!fixture.app.projection_receive.busy());
        assert!(fixture
            .app
            .documents
            .iter()
            .all(|doc| !IsWindowVisible(doc.window.0).as_bool()));
        assert_marker(&mut fixture, &journal, false);
    }
}

unsafe fn exchange(saved: bool) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    let mut fixture = Fixture::new();
    appended(&mut fixture);
    let journal = fixture.journal();
    if saved {
        checkpoint(&mut fixture.app);
    }
    restore(&mut fixture);
    assert!(!fixture.app.projection_receive.busy());
    assert_marker(&mut fixture, &journal, !saved);
}
#[test]
fn projection_receive_lifecycle_real_workspace_exchange_marks_unsaved_original_binding() {
    unsafe { exchange(false) }
}
#[test]
fn projection_receive_lifecycle_real_workspace_exchange_preserves_already_committed_original_binding(
) {
    unsafe { exchange(true) }
}
