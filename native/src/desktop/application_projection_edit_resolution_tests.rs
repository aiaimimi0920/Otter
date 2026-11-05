//! 原生菜单→原Job→隔离TLS→Editor→默认Checkpoint→恢复；不声称普通可见双端已经验收。
use super::*;
#[path = "application_projection_edit_resolution_fixture.rs"]
mod resolution_fixture;
use super::super::super::super::fixture as app_fixture;
use resolution_fixture::{
    apply, choose_resolution, conflicted, document, read, rejected, server as steps, x,
};

#[test]
fn collaboration_rejection_preserves_request_until_checkpoint_then_remote_choice_restores() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = accepted_two_way(false);
        shift(&fixture, 0.25);
        let base = state(&fixture)
            .canvas
            .borrow()
            .editor
            .layers()
            .unwrap()
            .base
            .clone();
        let response = document(&fixture, 2, true, 0.25);
        let rx = steps(
            &mut fixture,
            vec![
                rejected(409, "projection_edit_object_conflict", "apply"),
                read(response),
            ],
            false,
        );
        choose(&mut fixture, false);
        edit_completed(&fixture);
        let original = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(5)).unwrap()["operation"],
            "read"
        );
        assert_eq!(
            journal(&fixture)["entries"][0]["pending"]["request"],
            original
        );
        assert_eq!(
            journal(&fixture)["entries"][0]["pending"]["rejected"],
            "object_conflict"
        );
        poll_edit(&mut fixture);
        assert_eq!(x(&fixture), 0.5);
        assert_eq!(
            snapshot(&fixture).conflict(),
            Some(edit::Code::ObjectConflict)
        );
        manual_save(&mut fixture, false);
        poll_edit(&mut fixture);
        assert!(fixture.app.projection_edit.busy());
        assert!(journal(&fixture)["entries"][0]["pending"].is_object());
        finish(&mut fixture);
        joined(&mut fixture);
        assert!(journal(&fixture)["entries"][0]["pending"].is_null());
        let response = document(&fixture, 3, true, 0.75);
        let rx = steps(&mut fixture, vec![read(response)], false);
        choose_resolution(&mut fixture, false).unwrap();
        edit_completed(&fixture);
        rx.recv_timeout(Duration::from_secs(5)).unwrap();
        poll_edit(&mut fixture);
        assert_eq!(x(&fixture), 0.75);
        assert_eq!(snapshot(&fixture).conflict(), None);
        finish(&mut fixture);
        joined(&mut fixture);
        assert!(state(&fixture)
            .canvas
            .borrow()
            .editor
            .layers()
            .unwrap()
            .base
            .same_storage(&base));
        app_fixture::reopen_workspace(&mut fixture);
        assert_eq!(x(&fixture), 0.75);
        assert_eq!(snapshot(&fixture).document().revision(), 3);
        assert_eq!(snapshot(&fixture).conflict(), None);
    }
}
#[test]
fn collaboration_offline_keep_local_uses_fresh_revision_then_existing_apply_and_ack() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = conflicted(true, true);
        let latest = document(&fixture, 3, true, 0.25);
        let applied = document(&fixture, 4, true, 0.5);
        let rx = steps(&mut fixture, vec![read(latest), apply(applied, 0.5)], false);
        choose_resolution(&mut fixture, true).unwrap();
        edit_completed(&fixture);
        poll_edit(&mut fixture);
        assert_eq!(x(&fixture), 0.5);
        assert_eq!(snapshot(&fixture).document().revision(), 3);
        assert_eq!(snapshot(&fixture).conflict(), None);
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(5)).unwrap()["operation"],
            "read"
        );
        assert!(fixture.app.projection_edit.wait_handle().is_none());
        finish(&mut fixture);
        joined(&mut fixture);
        let request = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(request["operation"], "apply");
        assert_eq!(request["baseRevision"], 3);
        assert_eq!(snapshot(&fixture).document().revision(), 4);
        assert!(journal(&fixture)["entries"][0]["pending"].is_null());
    }
}
#[test]
fn collaboration_remote_choice_keeps_during_read_edit_and_readonly_conflict() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = conflicted(false, false);
        let doc = fixture
            .app
            .documents
            .iter()
            .find(|doc| doc.state.unit_id.as_str() == fixture.ticket().unit())
            .unwrap();
        let menu = CreatePopupMenu().unwrap();
        crate::desktop::device_authorization_ui::append_menu(menu, &doc.state).unwrap();
        let disabled = GetMenuState(
            menu,
            crate::desktop::device_authorization_ui::editing::KEEP_LOCAL as u32,
            MF_BYCOMMAND,
        );
        assert_ne!(disabled, u32::MAX, "conflict command must actually exist");
        assert_ne!(disabled & MF_GRAYED.0, 0);
        DestroyMenu(menu).unwrap();
        // 不强行调用禁用命令；外层授权分派器按产品合同会显示错误框，私有UI叶不为测试扩权。
        let latest = document(&fixture, 3, false, 0.25);
        let rx = steps(&mut fixture, vec![read(latest)], false);
        choose_resolution(&mut fixture, false).unwrap();
        edit_completed(&fixture);
        shift(&fixture, 0.125);
        let later = x(&fixture);
        poll_edit(&mut fixture);
        assert_eq!(x(&fixture), later);
        assert_eq!(snapshot(&fixture).conflict(), Some(edit::Code::ReadOnly));
        finish(&mut fixture);
        joined(&mut fixture);
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(5)).unwrap()["operation"],
            "read"
        );
        assert!(!fixture.app.projection_edit.busy());
    }
}
#[test]
fn collaboration_fresh_oneway_mode_rejects_old_twoway_keep_choice_without_local_publish() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = conflicted(true, true);
        let before = snapshot(&fixture);
        let latest = document(&fixture, 3, false, 0.75);
        let rx = steps(&mut fixture, vec![read(latest)], false);
        choose_resolution(&mut fixture, true).unwrap();
        edit_completed(&fixture);
        poll_edit(&mut fixture);
        joined(&mut fixture);
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(5)).unwrap()["operation"],
            "read"
        );
        assert_eq!(snapshot(&fixture), before);
        assert!(!fixture.app.projection_edit.busy());
        assert!(fixture.listener.as_ref().unwrap().accept().is_err());
    }
}
#[test]
fn collaboration_unknown_apply_cannot_be_retired_by_conflict_choice_or_read() {
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
        rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let pending = journal(&fixture);
        let mut response = wire(&fixture.origin, 2, false);
        response["snapshot"] = Value::Null;
        response["editing"]["mode"] = json!("two_way");
        response["editing"]["objects"]["remote"]["value"]["x"] = json!(0.25);
        let rx = peer(&mut fixture, vec![response]);
        cycle(&mut fixture, &rx);
        joined(&mut fixture);
        let before = snapshot(&fixture);
        choose_resolution(&mut fixture, false).unwrap();
        edit_completed(&fixture);
        poll_edit(&mut fixture);
        assert!(!fixture.app.projection_edit.busy());
        assert_eq!(snapshot(&fixture), before);
        assert_eq!(journal(&fixture), pending);
        assert!(fixture.listener.as_ref().unwrap().accept().is_err());
    }
}
#[test]
fn collaboration_rejected_read_failure_reopens_and_retries_only_read_not_original_apply() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = accepted_two_way(false);
        shift(&fixture, 0.25);
        let rx = steps(
            &mut fixture,
            vec![
                rejected(403, "projection_edit_read_only", "apply"),
                rejected(503, "temporarily_unavailable", "read"),
            ],
            false,
        );
        choose(&mut fixture, false);
        edit_completed(&fixture);
        poll_edit(&mut fixture);
        joined(&mut fixture);
        let request = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(
            journal(&fixture)["entries"][0]["pending"]["rejected"],
            "read_only"
        );
        save(&mut fixture);
        let latest = document(&fixture, 2, false, 0.25);
        let rx = steps(&mut fixture, vec![read(latest)], true);
        app_fixture::reopen(&mut fixture);
        assert!(!fixture.app.projection_edit.busy());
        assert_eq!(
            journal(&fixture)["entries"][0]["pending"]["request"],
            request
        );
        choose(&mut fixture, false);
        edit_completed(&fixture);
        finish(&mut fixture);
        joined(&mut fixture);
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(5)).unwrap()["operation"],
            "read"
        );
        assert!(journal(&fixture)["entries"][0]["pending"].is_null());
        assert_eq!(snapshot(&fixture).conflict(), Some(edit::Code::ReadOnly));
        assert_eq!(x(&fixture), 0.5);
    }
}
#[test]
fn collaboration_cancel_revocation_or_basis_change_drops_resolution_without_publish() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        for case in 0..3 {
            let mut fixture = conflicted(false, true);
            let before = snapshot(&fixture);
            let latest = document(&fixture, 3, true, 0.75);
            let rx = steps(&mut fixture, vec![read(latest)], false);
            choose_resolution(&mut fixture, false).unwrap();
            edit_completed(&fixture);
            match case {
                0 => choose(&mut fixture, true),
                1 => fixture
                    .app
                    .authorization_owner
                    .invalidate(&fixture.origin)
                    .unwrap(),
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
            poll_edit(&mut fixture);
            joined(&mut fixture);
            rx.recv_timeout(Duration::from_secs(5)).unwrap();
            assert!(!fixture.app.projection_edit.busy());
            assert_eq!(snapshot(&fixture).document(), before.document());
            assert_eq!(x(&fixture), 0.5);
            if case != 2 {
                assert_eq!(snapshot(&fixture), before);
            }
        }
    }
}
