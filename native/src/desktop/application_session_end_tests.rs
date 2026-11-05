//! 自有隐藏HWND的系统会话消息合同；不调用ExitWindowsEx，不碰真实profile或前台。
use super::*;
use crate::desktop::session_end::with_query;

unsafe fn query(app: &Application, index: usize) -> isize {
    with_query(
        &app.session_end,
        || app.session_end_ready(),
        || {
            SendMessageW(
                app.documents[index].window.0,
                WM_QUERYENDSESSION,
                None,
                Some(LPARAM(0x80000000)),
            )
            .0
        },
    )
}
unsafe fn cancel(app: &Application) {
    for document in &app.documents {
        SendMessageW(
            document.window.0,
            WM_ENDSESSION,
            Some(WPARAM(0)),
            Some(LPARAM(0x80000000)),
        );
    }
    assert!(!app.session_end.paused());
}
#[test]
fn unsaved_query_rejects_immediately_and_cancel_preserves_pending_work() {
    unsafe {
        run(|instance| {
            let app = new_app(instance);
            let doc = &app.documents[0];
            doc.state.new_window.set(true);
            assert_eq!(query(&app, 0), 0);
            assert!(doc.state.new_window.get());
            assert!(!app.session_end.paused());
            assert!(doc.is_alive());
            assert!(doc.state.session_end_reason.get());
            let mut length = 0;
            let _ = windows::Win32::System::Shutdown::ShutdownBlockReasonQuery(
                doc.window.0,
                None,
                &mut length,
            );
            assert!(length > 0, "real Windows block reason must exist");
            cancel(&app);
            assert!(!doc.state.session_end_reason.get());
            assert!(doc.state.new_window.get());
            assert!(!drain_messages());
        });
    }
}
#[test]
fn current_checkpoint_allows_query_freezes_document_and_cancel_resumes_editing() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            settle(&mut app);
            let disk = std::fs::read(directory.path()).unwrap();
            assert_eq!(app.save_status(), save_state::Status::Unsaved);
            assert_eq!(query(&app, 0), 1);
            assert!(app.session_end.paused());
            assert!(!app.session_end.ended());
            let doc = &app.documents[0];
            SendMessageW(doc.window.0, WM_CLOSE, None, None);
            SendMessageW(
                doc.window.0,
                WM_COMMAND,
                Some(WPARAM(super::super::super::NEW_WINDOW)),
                None,
            );
            assert!(doc.is_alive());
            assert!(!doc.state.new_window.get());
            assert_eq!(std::fs::read(directory.path()).unwrap(), disk);
            cancel(&app);
            assert!(!drain_messages());
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.4;
            assert_eq!(query(&app, 0), 0);
            assert_eq!(std::fs::read(directory.path()).unwrap(), disk);
            cancel(&app);
            settle(&mut app);
            assert_eq!(query(&app, 0), 1);
            cancel(&app);
            let value: serde_json::Value =
                serde_json::from_slice(&std::fs::read(directory.path()).unwrap()).unwrap();
            assert_eq!(
                value["stickers"][0]["opacityNormal"].as_f64().unwrap() as f32,
                0.4
            );
        });
    }
}
#[test]
fn inflight_checkpoint_and_new_intents_cannot_borrow_durable_permission() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            assert_eq!(query(&app, 0), 0);
            cancel(&app);
            settle(&mut app);
            for flag in [
                &app.documents[0].state.file_dialog,
                &app.documents[0].state.new_window,
                &app.documents[0].state.save_session_requested,
                &app.documents[0].state.open_session_requested,
                &app.documents[0].state.open_session_assets_requested,
                &app.documents[0].state.naming_import_requested,
                &app.documents[0].state.naming_save_requested,
                &app.documents[0].state.naming_update_requested,
                &app.documents[0].state.capture_protection_requested,
            ] {
                flag.set(true);
                assert_eq!(query(&app, 0), 0);
                assert!(flag.get());
                cancel(&app);
                flag.set(false);
            }
            app.documents[0]
                .state
                .workflow_sync_requested
                .set(Some(true));
            assert_eq!(query(&app, 0), 0);
            assert_eq!(
                app.documents[0].state.workflow_sync_requested.get(),
                Some(true)
            );
            cancel(&app);
            app.documents[0].state.workflow_sync_requested.set(None);
            *app.documents[0].state.live_request.borrow_mut() =
                Some(crate::desktop::capture_input::live::Request::Region(RECT {
                    left: 0,
                    top: 0,
                    right: 16,
                    bottom: 16,
                }));
            assert_eq!(query(&app, 0), 0);
            assert!(app.documents[0].state.live_request.borrow().is_some());
            cancel(&app);
            app.documents[0].state.live_request.borrow_mut().take();
            assert_eq!(query(&app, 0), 1);
            cancel(&app);
        });
    }
}
#[test]
fn changed_projection_scope_cannot_borrow_same_storage_checkpoint() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            settle(&mut app);
            let original = app.restored_session.as_ref().unwrap().projection_scope;
            assert!(app.session_checkpoint_current());
            app.restored_session.as_mut().unwrap().projection_scope =
                Some(crate::session_projection_scope::Scope::generate().unwrap());
            assert!(!app.session_checkpoint_current());
            assert_eq!(query(&app, 0), 0);
            cancel(&app);
            app.restored_session.as_mut().unwrap().projection_scope = original;
            assert_eq!(query(&app, 0), 1);
            cancel(&app);
        });
    }
}
#[test]
fn published_proof_is_not_bound_to_retained_input_or_equal_bytes_in_another_allocation() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            settle(&mut app);
            let retained = app.restored_session.as_ref().unwrap().original.clone();
            let published = app
                .workspace_checkpoint
                .as_mut()
                .unwrap()
                .original
                .take()
                .unwrap();
            assert!(!retained.same_storage(&published));
            app.workspace_checkpoint.as_mut().unwrap().original = Some(retained);
            assert!(!app.session_checkpoint_current());
            let equal_bytes: crate::session_source::Source = published.to_vec().into();
            assert_eq!(&*equal_bytes, &*published);
            assert!(!equal_bytes.same_storage(&published));
            app.workspace_checkpoint.as_mut().unwrap().original = Some(equal_bytes);
            assert!(!app.session_checkpoint_current());
            app.workspace_checkpoint.as_mut().unwrap().original = Some(published);
            assert!(app.session_checkpoint_current());
            assert_eq!(query(&app, 0), 1);
            cancel(&app);
        });
    }
}
#[test]
fn all_documents_share_phase_and_late_changes_in_second_document_revoke_permission() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.add_window(false).unwrap();
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            settle(&mut app);
            assert!(std::rc::Rc::ptr_eq(
                &app.documents[1].state.session_end,
                &app.session_end
            ));
            assert_eq!(query(&app, 1), 1);
            assert_eq!(query(&app, 0), 1);
            SendMessageW(app.documents[0].window.0, WM_CLOSE, None, None);
            assert!(app.documents.iter().all(|doc| doc.is_alive()));
            cancel(&app);
            app.documents[1].state.canvas.borrow_mut().normal_opacity = 0.3;
            assert_eq!(query(&app, 0), 0);
            cancel(&app);
        });
    }
}
#[test]
fn saved_manual_file_allows_without_profile_but_unscoped_query_never_borrows_application() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            crate::desktop::application::frozen_menu::reference::smoke::save(
                &mut app,
                Some(directory.0.join("manual.json")),
            )
            .unwrap();
            assert_eq!(app.save_status(), save_state::Status::Unchanged);
            let hwnd = app.documents[0].window.0;
            assert_eq!(SendMessageW(hwnd, WM_QUERYENDSESSION, None, None).0, 0);
            cancel(&app);
            assert_eq!(query(&app, 0), 1);
            cancel(&app);
        });
    }
}
#[test]
fn confirmed_end_never_deletes_workspace_and_false_cannot_resurrect_ended_phase() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            settle(&mut app);
            let bytes = std::fs::read(directory.path()).unwrap();
            let hwnd = app.documents[0].window.0;
            assert_eq!(query(&app, 0), 1);
            SendMessageW(
                hwnd,
                WM_ENDSESSION,
                Some(WPARAM(1)),
                Some(LPARAM(0x80000001)),
            );
            assert!(app.session_end.ended());
            SendMessageW(hwnd, WM_ENDSESSION, Some(WPARAM(0)), None);
            assert!(app.session_end.ended());
            assert!(app.documents[0].is_alive());
            assert!(drain_messages());
            assert_eq!(std::fs::read(directory.path()).unwrap(), bytes);
            assert_eq!(
                crate::session_document::decode(&bytes)
                    .unwrap()
                    .workspace
                    .units()
                    .len(),
                1
            );
            drop(app);
            let mut reopened = new_app(instance);
            reopened
                .install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            assert_eq!(reopened.documents.len(), 1);
        });
    }
}
