//! 真实隔离Application的协作权威/本地view保存恢复；不以此冒充网络或模式菜单已接通。
use super::*;
use crate::{
    projection_edit::{
        self as edit, fixture,
        journal::Context,
        view::{Size, View},
        workspace::{Role, Snapshot, Suspension},
    },
    viewport::Point,
};
use serde_json::{json, Value};
type EditOwner = crate::desktop::projection_edit_workspace::Owner;
fn document(path: &std::path::Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn current(app: &Application) -> Snapshot {
    let state = &app.documents[0].state;
    state
        .projection_edit
        .borrow()
        .as_ref()
        .unwrap()
        .capture(&state.canvas.borrow().editor)
}
fn metrics(
    text: &crate::text_annotation::TextAnnotation,
) -> std::result::Result<crate::session_vectors::TextMetrics, crate::session_vectors::Error> {
    unsafe {
        Ok(crate::session_vectors::TextMetrics {
            size: crate::desktop::annotation_text::measure(text)
                .map_err(|_| crate::session_vectors::Error::Geometry)?,
            baseline: crate::desktop::annotation_text::measure_flip_metrics(text)
                .map_err(|_| crate::session_vectors::Error::Geometry)?
                .baseline_from_top,
        })
    }
}
unsafe fn attach(app: &mut Application) -> Snapshot {
    // 模拟已校验权威交给宿主的接缝；不是新增产品授权/网络入口。
    let (receipt, _) = app.capture_session_snapshot().unwrap();
    assert!(receipt.matches(app));
    let scope = app
        .restored_session
        .as_ref()
        .unwrap()
        .projection_scope
        .unwrap();
    let state = &app.documents[0].state;
    let context = Context::new(
        scope,
        state.unit_id.as_str(),
        "https://loom.example",
        "device-1",
    )
    .unwrap();
    let mut raw = fixture::document_value();
    raw["objects"]["shape-1"]["value"]["x"] = json!(3.123456789123);
    raw["objects"]["shape-1"]["value"]["style"]["color"] = json!("rgba(1, 2, 3, 0.500000001)");
    let authority = edit::Document::parse(&serde_json::to_vec(&raw).unwrap()).unwrap();
    let mut canvas = state.canvas.borrow_mut();
    let frame = canvas.editor.visible_frame().unwrap();
    let view =
        View::from_document(&authority, Size::new(frame.width, frame.height).unwrap()).unwrap();
    edit::editor::prepare(&view, &canvas.editor, metrics)
        .unwrap()
        .apply(&mut canvas.editor)
        .unwrap()
        .unwrap();
    let snapshot = Snapshot::new(
        context,
        fixture::PROJECTION,
        edit::Route::Shared,
        Role::Source,
        authority,
        &view,
        Some(edit::Code::ObjectConflict),
    )
    .unwrap();
    let (editor, owner) =
        EditOwner::restore(snapshot.clone(), std::mem::take(&mut canvas.editor)).unwrap();
    canvas.editor = editor;
    *state.projection_edit.borrow_mut() = Some(owner);
    snapshot
}
fn stored(value: &Value) -> &Value {
    &value["stickers"][0][edit::workspace::FIELD]["snapshot"]
}

#[test]
fn authority_local_overlay_conflict_and_wire_values_survive_actual_save_checkpoint_and_reopen() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            let initial = attach(&mut app);
            let original = app.documents[0]
                .state
                .canvas
                .borrow()
                .image
                .as_ref()
                .unwrap()
                .clone();
            let frame = app.documents[0]
                .state
                .canvas
                .borrow()
                .editor
                .visible_frame()
                .unwrap();
            assert!(app.documents[0]
                .state
                .canvas
                .borrow_mut()
                .editor
                .translate(0, Point { x: 1.0, y: 2.0 })
                .unwrap());
            let visible = current(&app).editor_view().unwrap();
            assert!(
                (visible.basis_objects()["shape-1"]["x"].as_f64().unwrap()
                    - (3.123456789123 + 32.0 / frame.width))
                    .abs()
                    < 0.00001
            );
            settle(&mut app);
            let checkpoint = document(&directory.path());
            assert_eq!(
                stored(&checkpoint)["document"],
                serde_json::to_value(initial.document()).unwrap()
            );
            assert_eq!(
                stored(&checkpoint)["visible"],
                serde_json::to_value(visible.basis_objects()).unwrap()
            );
            assert_eq!(stored(&checkpoint)["conflict"], "object_conflict");
            assert_eq!(stored(&checkpoint)["suspended"], Value::Null);
            let manual = directory.0.join("manual-edit.json");
            crate::desktop::application::frozen_menu::reference::smoke::save(
                &mut app,
                Some(manual.clone()),
            )
            .unwrap();
            let saved = document(&manual);
            let base = crate::desktop::session_image::inline(
                saved["stickers"][0]["src"].as_str().unwrap(),
                &AtomicBool::new(false),
            )
            .unwrap();
            assert_eq!(
                base.pixels, original.pixels,
                "collaboration vectors must not be baked into source"
            );
            let fixed = std::fs::read(&manual).unwrap();
            drop(app);
            let mut reopened = new_app(instance);
            reopened
                .install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            let restored = current(&reopened);
            assert_eq!(restored.document(), initial.document());
            assert_eq!(
                restored.editor_view().unwrap().basis_objects(),
                visible.basis_objects()
            );
            assert_eq!(restored.conflict(), Some(edit::Code::ObjectConflict));
            {
                let mut canvas = reopened.documents[0].state.canvas.borrow_mut();
                assert!(
                    !canvas.editor.undo(),
                    "restore must not create synthetic undo"
                );
                assert!(canvas
                    .editor
                    .translate(0, Point { x: 2.0, y: 0.0 })
                    .unwrap());
                assert!(canvas.editor.undo());
            }
            for _ in 0..3 {
                settle(&mut reopened);
                assert_eq!(
                    current(&reopened).editor_view().unwrap().basis_objects(),
                    visible.basis_objects()
                );
            }
            assert_eq!(std::fs::read(&manual).unwrap(), fixed);
            assert!(!IsWindowVisible(reopened.documents[0].window.0).as_bool());
        });
    }
}

#[test]
fn metadata_detach_invalidates_save_receipt_and_dirty_stamp_and_never_reinstalls_old_record() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            attach(&mut app);
            let manual = directory.0.join("metadata-edit.json");
            crate::desktop::application::frozen_menu::reference::smoke::save(
                &mut app,
                Some(manual.clone()),
            )
            .unwrap();
            assert_eq!(app.save_status(), save_state::Status::Unchanged);
            let disk = std::fs::read(&manual).unwrap();
            let revision = app.documents[0].state.canvas.borrow().editor.revision();
            let (receipt, _) = app.capture_session_snapshot().unwrap();
            let guard = restore_batch::guard::Document::capture(&app.documents[0]).unwrap();
            app.request_save_existing().unwrap();
            ready(&app);
            let owner = app.documents[0]
                .state
                .projection_edit
                .borrow_mut()
                .take()
                .unwrap();
            assert_eq!(
                app.documents[0].state.canvas.borrow().editor.revision(),
                revision
            );
            assert!(!receipt.matches(&app));
            assert_eq!(guard.mismatch(&app.documents[0]), Some("projection-edit"));
            assert_eq!(app.save_status(), save_state::Status::Changed);
            assert!(matches!(
                app.poll_save_candidate().unwrap(),
                save_control::Poll::Stale
            ));
            assert_eq!(std::fs::read(&manual).unwrap(), disk);
            drop(owner);
            crate::desktop::application::frozen_menu::reference::smoke::save(&mut app, None)
                .unwrap();
            assert!(document(&manual)["stickers"][0]
                .get(edit::workspace::FIELD)
                .is_none());
        });
    }
}

#[test]
fn unsupported_basis_change_saves_current_crop_as_suspended_and_reopen_does_not_replay_old_basis() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            let authority = attach(&mut app).document().clone();
            let full = app.documents[0]
                .state
                .canvas
                .borrow()
                .editor
                .visible_frame()
                .unwrap();
            assert!(app.documents[0]
                .state
                .canvas
                .borrow_mut()
                .editor
                .crop_to(crate::crop_geometry::Rect {
                    x: 1.0,
                    y: 1.0,
                    width: full.width / 2.0,
                    height: full.height / 2.0
                })
                .unwrap());
            assert_eq!(current(&app).suspended(), Some(Suspension::BasisChanged));
            settle(&mut app);
            let value = document(&directory.path());
            assert_eq!(stored(&value)["suspended"], "basis_changed");
            let crop = value["stickers"][0]["imageEditState"].clone();
            drop(app);
            let mut reopened = new_app(instance);
            reopened
                .install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            assert!(reopened.documents[0]
                .state
                .canvas
                .borrow()
                .editor
                .crop_view()
                .is_some());
            assert_eq!(current(&reopened).document(), &authority);
            assert_eq!(
                current(&reopened).suspended(),
                Some(Suspension::BasisChanged)
            );
            settle(&mut reopened);
            assert_eq!(
                document(&directory.path())["stickers"][0]["imageEditState"],
                crop
            );
        });
    }
}

#[test]
fn actual_checkpoint_and_cancelled_save_never_acknowledge_unknown_network_pending() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            let snapshot = attach(&mut app);
            let store = edit::journal::Store::open(&directory.0, &AtomicBool::new(false)).unwrap();
            let ticket = store
                .begin(
                    snapshot.context().clone(),
                    edit::Route::Shared,
                    fixture::initialize(),
                    None,
                    fixture::view(),
                    &AtomicBool::new(false),
                )
                .unwrap();
            let request = ticket.request().bytes().to_vec();
            let journal = std::fs::read(directory.0.join("projection-edit.json")).unwrap();
            settle(&mut app);
            assert_eq!(
                store.pending(&AtomicBool::new(false)).unwrap()[0]
                    .request()
                    .bytes(),
                request
            );
            app.request_save_candidate().unwrap();
            app.cancel_save_candidate();
            ready(&app);
            assert!(app.take_save_candidate().is_err());
            assert_eq!(
                std::fs::read(directory.0.join("projection-edit.json")).unwrap(),
                journal
            );
            drop((ticket, store, app));
            let store = edit::journal::Store::open(&directory.0, &AtomicBool::new(false)).unwrap();
            assert_eq!(
                store.pending(&AtomicBool::new(false)).unwrap()[0]
                    .request()
                    .bytes(),
                request
            );
            drop(store);
        });
    }
}

#[test]
fn stale_or_cancelled_collaboration_restore_preserves_existing_application_and_source() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            attach(&mut app);
            let manual = directory.0.join("restore-edit.json");
            crate::desktop::application::frozen_menu::reference::smoke::save(
                &mut app,
                Some(manual.clone()),
            )
            .unwrap();
            let saved = document(&manual);
            let original = app.restored_session.as_ref().unwrap().original.clone();
            let window = app.documents[0].window.0;
            let revision = app.documents[0].state.canvas.borrow().editor.revision();
            for cancelled in [false, true] {
                let mut incoming = saved.clone();
                if !cancelled {
                    incoming["stickers"][0]["annotationState"]["elements"] = json!([]);
                }
                let space = app.restored_session.as_ref().unwrap().space.unwrap();
                app.request_restore(serde_json::to_vec(&incoming).unwrap(), None, space)
                    .unwrap();
                if cancelled {
                    app.cancel_restore();
                }
                let deadline = Instant::now() + Duration::from_secs(10);
                let mut rejected = false;
                loop {
                    if app.process_restore(false).is_err() {
                        rejected = true;
                        break;
                    }
                    if app.restore_job.is_none() {
                        break;
                    }
                    assert!(Instant::now() < deadline, "collaboration restore deadline");
                    std::thread::sleep(Duration::from_millis(1));
                }
                assert!(
                    cancelled || rejected,
                    "stale canonical view must be rejected"
                );
                assert!(app.restore_job.is_none());
                assert_eq!(app.documents[0].window.0, window);
                assert_eq!(
                    app.documents[0].state.canvas.borrow().editor.revision(),
                    revision
                );
                assert!(app
                    .restored_session
                    .as_ref()
                    .unwrap()
                    .original
                    .same_storage(&original));
            }
            assert_eq!(document(&manual), saved);
        });
    }
}
