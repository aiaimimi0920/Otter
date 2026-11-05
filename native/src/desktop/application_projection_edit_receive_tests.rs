//! 实际协作接收/metadata read/Editor/原Checkpoint重开；独立wire与原始源像素oracle。
use super::*;
use crate::{
    projection_edit::{self as edit, workspace::Role},
    viewport::Point,
};
use serde_json::{json, Value};
#[path = "application_projection_edit_receive_fixture.rs"]
mod fixture;
use fixture::{accepted, peer, save, snapshot, state, wire};
#[path = "application_projection_edit_attach_tests.rs"]
mod attach;
#[path = "application_projection_edit_submit_tests.rs"]
mod submit;

unsafe fn first(offline: bool) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    let mut fixture = accepted(offline);
    let original = snapshot(&fixture);
    assert_eq!(original.role(), Role::Receiver);
    assert_eq!(
        original.route(),
        if offline {
            edit::Route::Offline
        } else {
            edit::Route::Shared
        }
    );
    assert_eq!(
        original.editor_view().unwrap().basis_objects()["remote"]["w"],
        json!(1)
    );
    let base = state(&fixture)
        .canvas
        .borrow()
        .minified_source
        .as_ref()
        .unwrap()
        .image()
        .pixels
        .clone();
    assert_eq!(&*base, &[0, 0, 255, 255, 0, 128, 0, 128]);
    assert_eq!(state(&fixture).canvas.borrow().editor.elements().len(), 1);
    {
        let canvas = state(&fixture).canvas.borrow();
        let (png, _) = crate::desktop::export::png_with_opacity(
            canvas.minified_source.as_ref().unwrap().image(),
            &canvas.editor,
            1.0,
        )
        .unwrap();
        let image = crate::desktop::export::decode_png(png.to_vec());
        let offset = ((image.height / 2 * image.width + image.width / 4) * 4) as usize;
        assert_eq!(
            &image.pixels[offset..offset + 4],
            &[255, 0, 0, 255],
            "receiver vector must be painted blue by actual D2D/WIC"
        );
    }
    assert!(fixture
        .app
        .documents
        .iter()
        .all(|doc| !IsWindowVisible(doc.window.0).as_bool()));
    let journal = fixture.journal();
    super::super::fixture::reopen_workspace(&mut fixture);
    let restored = snapshot(&fixture);
    assert_eq!(restored.document(), original.document());
    assert_eq!(
        restored.editor_view().unwrap().basis_objects(),
        original.editor_view().unwrap().basis_objects()
    );
    assert!(
        !state(&fixture).canvas.borrow_mut().editor.undo(),
        "initial receive/restore must not invent undo"
    );
    assert_eq!(fixture.journal(), journal);
    assert_eq!(
        &*state(&fixture)
            .canvas
            .borrow()
            .minified_source
            .as_ref()
            .unwrap()
            .image()
            .pixels,
        &*base
    );
}
#[test]
fn collaboration_shared_accept_is_separate_vectors_and_restores_original_source_and_authority() {
    unsafe { first(false) }
}
#[test]
fn collaboration_offline_accept_keeps_transport_context_and_restores_authority() {
    unsafe { first(true) }
}

#[test]
fn collaboration_same_png_revision_updates_remote_objects_keeps_local_undo_and_survives_reopen() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = accepted(false);
        let base = state(&fixture)
            .canvas
            .borrow()
            .editor
            .layers()
            .unwrap()
            .base
            .clone();
        let journal = fixture.journal();
        let mut response = wire(&fixture.origin, 2, false);
        response["snapshot"] = Value::Null;
        response["editing"]["objects"]["remote"]["value"]["w"] = json!(0.5);
        let first = response.clone();
        let rx = peer(
            &mut fixture,
            vec![response.clone(), response.clone(), response.clone()],
        );
        cycle(&mut fixture, &rx);
        assert_eq!(snapshot(&fixture).document().revision(), 2);
        assert_eq!(binding(&fixture)["revision"], 5);
        assert!(state(&fixture)
            .canvas
            .borrow()
            .editor
            .layers()
            .unwrap()
            .base
            .same_storage(&base));
        let unchanged_revision = state(&fixture).canvas.borrow().editor.revision();
        cycle(&mut fixture, &rx);
        assert_eq!(
            state(&fixture).canvas.borrow().editor.revision(),
            unchanged_revision
        );
        let frame = state(&fixture)
            .canvas
            .borrow()
            .editor
            .full_logical_frame()
            .unwrap();
        state(&fixture)
            .canvas
            .borrow_mut()
            .editor
            .translate(
                0,
                Point {
                    x: (frame.width / 4.0) as f32,
                    y: 0.0,
                },
            )
            .unwrap();
        cycle(&mut fixture, &rx);
        joined(&mut fixture);
        let changed = snapshot(&fixture);
        assert_eq!(changed.conflict(), Some(edit::Code::ReadOnly));
        assert!(
            (changed.editor_view().unwrap().basis_objects()["remote"]["x"]
                .as_f64()
                .unwrap()
                - 0.5)
                .abs()
                < 0.00001
        );
        assert_eq!(
            serde_json::to_value(changed.document()).unwrap(),
            first["editing"]
        );
        assert!(state(&fixture).canvas.borrow_mut().editor.undo());
        let undone = snapshot(&fixture);
        assert_eq!(
            undone.editor_view().unwrap().basis_objects()["remote"]["x"].as_f64(),
            Some(0.0)
        );
        save(&mut fixture);
        let mut next = wire(&fixture.origin, 3, false);
        next["snapshot"] = Value::Null;
        next["editing"]["objects"]["remote"]["value"]["x"] = json!(0.25);
        let rx = fixture::peer_with_renew(&mut fixture, vec![next], true);
        super::super::fixture::reopen(&mut fixture);
        assert_eq!(
            snapshot(&fixture).editor_view().unwrap().basis_objects()["remote"]["x"].as_f64(),
            Some(0.0)
        );
        assert_eq!(snapshot(&fixture).document().revision(), 2);
        cycle(&mut fixture, &rx);
        joined(&mut fixture);
        assert_eq!(snapshot(&fixture).document().revision(), 3);
        assert_eq!(
            snapshot(&fixture).editor_view().unwrap().basis_objects()["remote"]["x"],
            json!(0.25)
        );
        assert_eq!(fixture.journal(), journal);
    }
}

#[test]
fn collaboration_completed_metadata_is_discarded_after_local_edit_and_original_permit_is_reaped() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = accepted(false);
        let old = source(&fixture);
        let mut response = wire(&fixture.origin, 2, false);
        response["snapshot"] = Value::Null;
        response["editing"]["objects"]["remote"]["value"]["w"] = json!(0.5);
        let rx = peer(&mut fixture, vec![response]);
        completed(&mut fixture, &rx);
        let mut retry = fixture.ticket().retry();
        assert_eq!(
            retry.activate(),
            Err(crate::projection_delivery::journal::Error::Busy)
        );
        state(&fixture)
            .canvas
            .borrow_mut()
            .editor
            .translate(0, Point { x: 1.0, y: 0.0 })
            .unwrap();
        let revision = state(&fixture).canvas.borrow().editor.revision();
        poll(&mut fixture);
        joined(&mut fixture);
        assert_eq!(snapshot(&fixture).document().revision(), 1);
        assert_eq!(state(&fixture).canvas.borrow().editor.revision(), revision);
        assert!(source(&fixture).same_storage(&old));
        retry.activate().unwrap();
        assert!(!fixture.app.projection_cleanup.pending_local());
    }
}

unsafe fn local_and_remote(two_way: bool) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    let mut fixture = accepted(false);
    if two_way {
        // 此测试专注只读合并/保存；自动提交另由原read→TLS→Checkpoint闭环测试覆盖。
        let unit = state(&fixture).unit_id.clone();
        fixture
            .app
            .projection_edit
            .pause_automatic_for_test(&fixture.app.workspace, &unit);
    }
    let local = {
        let mut canvas = state(&fixture).canvas.borrow_mut();
        canvas
            .editor
            .begin(crate::editor::Tool::Brush, Point { x: 20.0, y: 20.0 })
            .unwrap();
        assert!(canvas.editor.commit());
        canvas.editor.elements()[1].annotation_id().to_owned()
    };
    let mut response = wire(&fixture.origin, 2, false);
    response["snapshot"] = Value::Null;
    response["editing"]["objects"]["remote"]["value"]["w"] = json!(0.5);
    if two_way {
        response["editing"]["mode"] = json!("two_way");
        response["editing"]["modeRevision"] = json!(2);
    }
    let rx = peer(&mut fixture, vec![response]);
    cycle(&mut fixture, &rx);
    joined(&mut fixture);
    let current = snapshot(&fixture);
    let objects = current.editor_view().unwrap();
    assert_eq!(objects.basis_objects()["remote"]["w"], json!(0.5));
    assert!(objects.basis_objects().contains_key(&local));
    assert_eq!(
        current.conflict(),
        if two_way {
            None
        } else {
            Some(edit::Code::ReadOnly)
        }
    );
    assert!(
        !current.document().objects().contains_key(&local),
        "read must not pretend local edits are committed remotely"
    );
    assert_eq!(
        fixture.app.authorization.borrow().projection.status,
        if two_way {
            "已接收协作，本地修改待提交"
        } else {
            "协作冲突，保留本地修改"
        }
    );
    save(&mut fixture);
    super::super::fixture::reopen_workspace(&mut fixture);
    assert_eq!(
        snapshot(&fixture).editor_view().unwrap().basis_objects(),
        objects.basis_objects()
    );
    assert_eq!(snapshot(&fixture).conflict(), current.conflict());
}
#[test]
fn collaboration_one_way_remote_update_preserves_independent_local_object_and_readonly_conflict() {
    unsafe { local_and_remote(false) }
}
#[test]
fn collaboration_two_way_receive_while_automatic_paused_preserves_local_object_as_pending() {
    unsafe { local_and_remote(true) }
}

#[test]
fn collaboration_stale_session_basis_and_missing_metadata_never_replace_current_editor_or_source() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        // 各反例独立owner；不把产品指数退避串到合成peer的单次接入期限中。
        for case in 0..5 {
            let mut fixture = accepted(false);
            let mut good = wire(&fixture.origin, 2, false);
            good["snapshot"] = Value::Null;
            let mut bad = good.clone();
            match case {
                0 => {
                    bad.as_object_mut().unwrap().remove("editing");
                }
                1 => bad["editing"]["sessionId"] = json!("edit:33333333333333333333333333333333"),
                2 => bad["editing"]["basis"]["width"] = json!(3),
                3 => {
                    bad = wire(&fixture.origin, 1, false);
                    bad["snapshot"] = Value::Null;
                }
                4 => {
                    bad["width"] = json!(3);
                    bad["editing"]["basis"]["width"] = json!(3);
                }
                _ => unreachable!(),
            }
            let rx = peer(&mut fixture, vec![good, bad]);
            cycle(&mut fixture, &rx);
            let before = snapshot(&fixture);
            let before_source = source(&fixture);
            let revision = state(&fixture).canvas.borrow().editor.revision();
            cycle(&mut fixture, &rx);
            assert_eq!(snapshot(&fixture), before, "case {case}");
            assert!(source(&fixture).same_storage(&before_source), "case {case}");
            assert_eq!(
                state(&fixture).canvas.borrow().editor.revision(),
                revision,
                "case {case}"
            );
            joined(&mut fixture);
        }
    }
}
