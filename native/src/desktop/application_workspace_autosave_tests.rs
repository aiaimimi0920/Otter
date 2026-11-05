//! 自动保存的取消/代际/冲突/显式请求和退出边界；仅自有隐藏 HWND 与 linshi 文件。
use super::*;
use std::time::{Duration, Instant};

struct Class(HINSTANCE);
impl Drop for Class {
    fn drop(&mut self) {
        unsafe {
            let _ = UnregisterClassW(w!("Otter.Native.Workspace"), Some(self.0));
        }
    }
}
unsafe fn run(test: impl FnOnce(HINSTANCE)) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let instance: HINSTANCE = windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
        .unwrap()
        .into();
    assert_ne!(
        RegisterClassW(&WNDCLASSW {
            hInstance: instance,
            lpszClassName: w!("Otter.Native.Workspace"),
            lpfnWndProc: Some(crate::desktop::procedure::window_proc),
            ..Default::default()
        }),
        0
    );
    let _class = Class(instance);
    test(instance);
}
unsafe fn ready(app: &Application) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !app.save_job.as_ref().unwrap().1.is_ready() {
        assert!(Instant::now() < deadline, "autosave staging deadline");
        std::thread::sleep(Duration::from_millis(1));
    }
}
unsafe fn revision(path: &std::path::Path) -> u64 {
    crate::session_document::decode(&std::fs::read(path).unwrap())
        .unwrap()
        .revision
}
#[test]
fn autosave_real_menu_shared_worker_disk_and_restore_product_path() {
    unsafe {
        run(|instance| smoke::run(instance).unwrap());
    }
}
#[test]
fn stale_candidate_and_disable_enable_aba_never_commit_old_snapshot() {
    unsafe {
        run(|instance| {
            for aba in [false, true] {
                let (mut app, path) = smoke::fixture(instance).unwrap();
                let before = std::fs::read(&path.0).unwrap();
                smoke::toggle(&app, 0).unwrap();
                app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.5;
                app.start_workspace_autosave().unwrap();
                assert!(app.request_save_existing().is_err(), "single save slot");
                ready(&app);
                if aba {
                    smoke::toggle(&app, 0).unwrap();
                    smoke::toggle(&app, 0).unwrap();
                } else {
                    app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.6;
                }
                app.poll_workspace_autosave().unwrap();
                assert!(app.save_job.is_none());
                assert_eq!(std::fs::read(&path.0).unwrap(), before);
                assert!(app.workspace_autosave.enabled());
                smoke::settle(&mut app).unwrap();
                assert_eq!(
                    revision(&path.0),
                    crate::session_document::decode(&before).unwrap().revision + 1
                );
                assert_eq!(app.save_status(), save_state::Status::Unchanged);
            }
        });
    }
}
#[test]
fn conflict_stops_once_and_preserves_external_file_and_memory() {
    unsafe {
        run(|instance| {
            let (mut app, path) = smoke::fixture(instance).unwrap();
            smoke::toggle(&app, 0).unwrap();
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.5;
            let before = app.restored_session.as_ref().unwrap().original.clone();
            use std::io::Write;
            std::fs::OpenOptions::new()
                .append(true)
                .open(&path.0)
                .unwrap()
                .write_all(b" ")
                .unwrap();
            let external = std::fs::read(&path.0).unwrap();
            app.start_workspace_autosave().unwrap();
            ready(&app);
            let mut warnings = Vec::new();
            for _ in 0..3 {
                app.pending_workspace_save_with(|_, _, message, warning| {
                    warnings.push((message.to_owned(), warning))
                });
            }
            assert_eq!(warnings.len(), 1);
            assert!(warnings[0].1);
            assert!(!app.workspace_autosave.enabled());
            assert!(app.save_job.is_none());
            assert!(app
                .restored_session
                .as_ref()
                .unwrap()
                .original
                .same_storage(&before));
            assert_eq!(std::fs::read(&path.0).unwrap(), external);
            assert_eq!(app.documents[0].state.canvas.borrow().normal_opacity, 0.5);
        });
    }
}
#[test]
fn manual_request_is_retained_while_auto_job_owns_the_slot() {
    unsafe {
        run(|instance| {
            let (mut app, path) = smoke::fixture(instance).unwrap();
            smoke::toggle(&app, 0).unwrap();
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.5;
            app.start_workspace_autosave().unwrap();
            let doc = &app.documents[0];
            commands::execute(doc.window.0, &doc.state, commands::SAVE_SESSION).unwrap();
            assert!(doc.state.save_existing_requested.get());
            ready(&app);
            app.pending_workspace_save_with(|_, _, _, warning| assert!(!warning));
            assert!(app.save_origin.is_some());
            assert!(app.save_job.is_some());
            assert!(app.workspace_autosave_pending.is_none());
            ready(&app);
            app.pending_workspace_save_with(|_, _, _, warning| assert!(!warning));
            assert!(app.save_origin.is_none() && app.save_job.is_none());
            assert_eq!(app.save_status(), save_state::Status::Unchanged);
            assert_eq!(revision(&path.0), 3);
        });
    }
}
#[test]
fn exit_cancels_auto_candidate_then_uses_existing_unsaved_confirmation() {
    unsafe {
        run(|instance| {
            let (mut app, path) = smoke::fixture(instance).unwrap();
            let before = std::fs::read(&path.0).unwrap();
            smoke::toggle(&app, 0).unwrap();
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.5;
            app.start_workspace_autosave().unwrap();
            ready(&app);
            app.exit_requested.set(true);
            assert!(!app.close_requested_with(|_| panic!("auto candidate must retire first")));
            app.pending_workspace_save_with(|_, _, _, _| panic!("expected silent cancellation"));
            assert!(app.exit_requested.get() && app.save_job.is_none());
            let mut confirmed = false;
            assert!(!app.close_requested_with(|_| {
                confirmed = true;
                exit::Choice::Cancel
            }));
            assert!(confirmed && !app.exit_requested.get());
            assert_eq!(std::fs::read(&path.0).unwrap(), before);
            assert!(app.documents[0].is_alive());
        });
    }
}
#[test]
fn unsaved_target_and_exhausted_generation_fail_closed() {
    unsafe {
        run(|instance| {
            let first =
                DocumentWindow::create(instance, State::new(None, false).unwrap(), false).unwrap();
            let mut app = Application::new(instance, first);
            smoke::toggle(&app, 0).unwrap();
            let mut warnings = 0;
            app.pending_workspace_save_with(|_, _, _, warning| {
                assert!(warning);
                warnings += 1;
            });
            assert_eq!(warnings, 1);
            assert!(!app.workspace_autosave.enabled() && app.save_job.is_none());
            for enabled in [false, true] {
                app.workspace_autosave.enabled.set(enabled);
                app.workspace_autosave.generation.set(u64::MAX);
                assert!(smoke::toggle(&app, 0).is_err());
                assert!(!app.workspace_autosave.enabled());
            }
        });
    }
}

#[test]
fn open_request_waits_for_auto_owner_without_consuming_selected_intent() {
    unsafe {
        run(|instance| {
            let (mut app, path) = smoke::fixture(instance).unwrap();
            smoke::toggle(&app, 0).unwrap();
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.5;
            app.start_workspace_autosave().unwrap();
            let doc = &app.documents[0];
            commands::execute(doc.window.0, &doc.state, commands::OPEN_SESSION).unwrap();
            app.pending_open_session_with(|_, _, _| panic!("auto owner must defer chooser"))
                .unwrap();
            assert!(app.documents[0].state.open_session_requested.get());
            ready(&app);
            app.pending_workspace_save_with(|_, _, _, warning| assert!(!warning));
            assert!(app.documents[0].state.open_session_requested.get());
            assert!(app.save_job.is_none());
            app.pending_open_session_with(|_, _, _| Ok(Some(path.0.clone().into())))
                .unwrap();
            assert!(app.restore_job.is_some());
            assert!(!app.documents[0].state.open_session_requested.get());
            app.cancel_restore();
        });
    }
}

#[test]
fn disabling_pending_save_and_saving_as_new_target_do_not_reuse_consent() {
    unsafe {
        run(|instance| {
            let (mut app, path) = smoke::fixture(instance).unwrap();
            let before = std::fs::read(&path.0).unwrap();
            smoke::toggle(&app, 0).unwrap();
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.5;
            app.start_workspace_autosave().unwrap();
            ready(&app);
            smoke::toggle(&app, 0).unwrap();
            smoke::settle(&mut app).unwrap();
            assert!(!app.workspace_autosave.enabled());
            assert_eq!(std::fs::read(&path.0).unwrap(), before);
            smoke::toggle(&app, 0).unwrap();
            let new = smoke::File(save_publish::smoke_destination().unwrap());
            crate::desktop::application::frozen_menu::reference::smoke::save(
                &mut app,
                Some(new.0.clone()),
            )
            .unwrap();
            assert!(!app.workspace_autosave.enabled());
            assert_eq!(std::fs::read(&path.0).unwrap(), before);
            assert!(new.0.is_file());
        });
    }
}
