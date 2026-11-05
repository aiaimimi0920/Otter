//! 实际接收会话→菜单Apply→独立TLS oracle→精确Checkpoint→ack/重开；未冒充普通可见双端。
use super::*;
#[path = "application_projection_edit_receiver_automation_tests.rs"]
mod automation;
#[path = "application_projection_edit_resolution_tests.rs"]
mod resolution;
#[path = "application_projection_edit_submit_fixture.rs"]
mod submit_fixture;
use submit_fixture::{
    accepted_two_way, choose, completed as edit_completed, finish, journal, manual_save, poll_edit,
    server, shift,
};

unsafe fn submitted(offline: bool) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    let mut fixture = accepted_two_way(offline);
    let base = state(&fixture)
        .canvas
        .borrow()
        .editor
        .layers()
        .unwrap()
        .base
        .clone();
    shift(&fixture, 0.25);
    let rx = server(&mut fixture, vec![(200, 0.5)]);
    choose(&mut fixture, false);
    let old_receiver = fixture.app.projection_profile.entries().unwrap()[0].clone();
    fixture
        .app
        .start_projection_display_recovery(old_receiver, &mut |_| {
            panic!("edit and receiver may not share two wait handles")
        });
    fixture.app.process_projection_sync(
        &mut |_| panic!("sync competed with edit"),
        Instant::now() + Duration::from_secs(10),
    );
    fixture.app.process_projection_automatic(
        &mut |_| panic!("automatic receive competed with edit"),
        Instant::now() + Duration::from_secs(10),
    );
    assert!(!fixture.app.projection_receive.busy());
    edit_completed(&fixture);
    let request = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(request["baseRevision"], 1);
    assert_eq!(request["modeRevision"], 1);
    assert!(journal(&fixture)["entries"][0]["pending"].is_object());
    poll_edit(&mut fixture);
    assert_eq!(snapshot(&fixture).document().revision(), 2);
    assert_eq!(
        snapshot(&fixture).editor_view().unwrap().basis_objects()["remote"]["x"].as_f64(),
        Some(0.5)
    );
    // HTTP成功、当前Editor呈现、甚至新候选生成均不是磁盘提交事实。
    assert!(journal(&fixture)["entries"][0]["pending"].is_object());
    let old = crate::session_source::Source::from(
        std::fs::read(fixture.files.root().join("workspace.json")).unwrap(),
    );
    let proof = crate::session_projection_scope::Proof::capture(&old).unwrap();
    fixture
        .app
        .projection_edit
        .committed(Some(&proof), &old, &fixture.app.workspace);
    poll_edit(&mut fixture);
    assert!(
        fixture.app.projection_edit.wait_handle().is_none(),
        "old canonical must not authorize ack"
    );
    if !offline {
        manual_save(&mut fixture, false);
        poll_edit(&mut fixture);
        assert!(
            fixture.app.projection_edit.busy(),
            "manual Save must retain submission"
        );
        assert!(
            fixture.app.projection_edit.wait_handle().is_none(),
            "manual file must not authorize canonical ack"
        );
        assert!(journal(&fixture)["entries"][0]["pending"].is_object());
    }
    save(&mut fixture);
    assert!(journal(&fixture)["entries"][0]["pending"].is_object());
    finish(&mut fixture);
    joined(&mut fixture);
    assert!(journal(&fixture)["entries"][0]["pending"].is_null());
    assert_eq!(journal(&fixture)["entries"][0]["document"]["revision"], 2);
    assert!(state(&fixture)
        .canvas
        .borrow()
        .editor
        .layers()
        .unwrap()
        .base
        .same_storage(&base));
    super::super::super::fixture::reopen_workspace(&mut fixture);
    assert_eq!(snapshot(&fixture).document().revision(), 2);
    assert_eq!(
        snapshot(&fixture).editor_view().unwrap().basis_objects()["remote"]["x"].as_f64(),
        Some(0.5)
    );
    assert!(journal(&fixture)["entries"][0]["pending"].is_null());
}
#[test]
fn collaboration_unconfirmed_request_survives_checkpoint_and_reopen_without_automatic_replay() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = accepted_two_way(false);
        shift(&fixture, 0.25);
        let rx = server(&mut fixture, vec![(503, 0.5)]);
        choose(&mut fixture, false);
        edit_completed(&fixture);
        poll_edit(&mut fixture);
        joined(&mut fixture);
        let original = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        save(&mut fixture);
        let rx = submit_fixture::server_with_renew(&mut fixture, vec![(200, 0.5)], true);
        super::super::super::fixture::reopen(&mut fixture);
        assert!(!fixture.app.projection_edit.busy());
        assert_eq!(
            journal(&fixture)["entries"][0]["pending"]["request"],
            original
        );
        choose(&mut fixture, false);
        edit_completed(&fixture);
        assert_eq!(rx.recv_timeout(Duration::from_secs(5)).unwrap(), original);
        finish(&mut fixture);
        joined(&mut fixture);
        assert!(journal(&fixture)["entries"][0]["pending"].is_null());
        assert_eq!(snapshot(&fixture).document().revision(), 2);
    }
}
#[test]
fn collaboration_shared_menu_apply_requires_exact_checkpoint_then_ack_and_reopen() {
    unsafe { submitted(false) }
}
#[test]
fn collaboration_offline_menu_apply_uses_fixed_route_and_same_checkpoint_chain() {
    unsafe { submitted(true) }
}

#[test]
fn collaboration_request_manual_save_advances_only_committed_candidate_identity() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        for cancelled in [false, true] {
            let mut fixture = accepted_two_way(false);
            shift(&fixture, 0.25);
            let rx = server(&mut fixture, vec![(200, 0.5)]);
            choose(&mut fixture, false);
            edit_completed(&fixture);
            rx.recv_timeout(Duration::from_secs(5)).unwrap();
            let original = fixture
                .app
                .restored_session
                .as_ref()
                .unwrap()
                .original
                .clone();
            manual_save(&mut fixture, cancelled);
            let current = &fixture.app.restored_session.as_ref().unwrap().original;
            assert_eq!(original.same_storage(current), cancelled);
            let index = fixture.app.projection_sync_index().unwrap();
            assert!(index.for_ticket(fixture.ticket()).is_some());
            if !cancelled {
                assert!(index.is_for(current));
                let copy = crate::session_source::Source::from(current.to_vec());
                assert!(!index.is_for(&copy), "equal bytes are not commit identity");
                assert!(!index.is_for(&original));
            }
            poll_edit(&mut fixture);
            assert_eq!(snapshot(&fixture).document().revision(), 2);
            assert!(fixture.app.projection_edit.busy());
            assert!(journal(&fixture)["entries"][0]["pending"].is_object());
            finish(&mut fixture);
            joined(&mut fixture);
            assert!(journal(&fixture)["entries"][0]["pending"].is_null());
        }
    }
}

#[test]
fn collaboration_readonly_menu_preserves_local_changes_without_network_write() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = accepted(false);
        shift(&fixture, 0.25);
        let before = snapshot(&fixture);
        choose(&mut fixture, false);
        edit_completed(&fixture);
        poll_edit(&mut fixture);
        assert!(!fixture.app.projection_edit.busy());
        assert_eq!(snapshot(&fixture), before);
        assert!(!fixture.files.root().join("projection-edit.json").exists());
        assert!(fixture.listener.as_ref().unwrap().accept().is_err());
    }
}

#[test]
fn collaboration_failed_request_retries_original_op_then_submits_during_network_edit() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = accepted_two_way(false);
        shift(&fixture, 0.25);
        let rx = server(&mut fixture, vec![(503, 0.5), (200, 0.5), (200, 0.75)]);
        choose(&mut fixture, false);
        edit_completed(&fixture);
        poll_edit(&mut fixture);
        assert!(!fixture.app.projection_edit.busy());
        let first = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let frozen = journal(&fixture)["entries"][0]["pending"]["request"].clone();
        assert_eq!(first, frozen);
        assert_eq!(snapshot(&fixture).document().revision(), 1);
        choose(&mut fixture, false);
        edit_completed(&fixture);
        assert_eq!(rx.recv_timeout(Duration::from_secs(5)).unwrap(), first);
        shift(&fixture, 0.125);
        poll_edit(&mut fixture);
        assert_eq!(
            snapshot(&fixture).document().objects()["remote"].value()["x"].as_f64(),
            Some(0.5)
        );
        assert_eq!(
            snapshot(&fixture).editor_view().unwrap().basis_objects()["remote"]["x"].as_f64(),
            Some(0.75)
        );
        finish(&mut fixture);
        joined(&mut fixture);
        let last = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_ne!(last["opId"], first["opId"]);
        assert_eq!(last["baseRevision"], 2);
        assert!(journal(&fixture)["entries"][0]["pending"].is_null());
        assert_eq!(snapshot(&fixture).document().revision(), 3);
        assert_eq!(snapshot(&fixture).conflict(), None);
    }
}
#[test]
fn collaboration_cancel_revocation_and_basis_change_discard_completed_reply_without_ack() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        for case in 0..3 {
            let mut fixture = accepted_two_way(false);
            shift(&fixture, 0.25);
            let rx = server(&mut fixture, vec![(200, 0.5)]);
            choose(&mut fixture, false);
            edit_completed(&fixture);
            rx.recv_timeout(Duration::from_secs(5)).unwrap();
            match case {
                0 => choose(&mut fixture, true),
                1 => {
                    fixture
                        .app
                        .authorization_owner
                        .invalidate(&fixture.origin)
                        .unwrap();
                }
                2 => {
                    let mut canvas = state(&fixture).canvas.borrow_mut();
                    let pixels =
                        crate::pixels::Pixels::from(vec![
                            0;
                            canvas.editor.layers().unwrap().base.len()
                        ]);
                    assert!(canvas.editor.replace_source(pixels));
                }
                _ => unreachable!(),
            }
            let revision = state(&fixture).canvas.borrow().editor.revision();
            poll_edit(&mut fixture);
            joined(&mut fixture);
            assert!(!fixture.app.projection_edit.busy(), "case {case}");
            assert_eq!(snapshot(&fixture).document().revision(), 1);
            assert_eq!(state(&fixture).canvas.borrow().editor.revision(), revision);
            assert!(journal(&fixture)["entries"][0]["pending"].is_object());
        }
    }
}
