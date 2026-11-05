//! 完整 Application 检查点/重开/文件隔离；真实隐藏 HWND，不发全局输入。
use super::*;
#[path = "application_workspace_checkpoint_annotation_domain_tests.rs"]
mod annotation_domain;
#[path = "application_workspace_checkpoint_edit_tests.rs"]
mod collaboration;
#[path = "application_workspace_checkpoint_crash_tests.rs"]
mod crash;
#[path = "application_workspace_checkpoint_geometry_tests.rs"]
mod geometry;
#[path = "application_workspace_checkpoint_maintenance_tests.rs"]
mod maintenance;
#[path = "application_workspace_checkpoint_output_tests.rs"]
mod output;
#[path = "application_workspace_checkpoint_paint_tests.rs"]
mod paint;
#[path = "application_workspace_checkpoint_path_tests.rs"]
mod paths;
#[path = "application_projection_profile_tests.rs"]
mod projection_profile;
#[path = "application_projection_receive_tests.rs"]
mod projection_receive;
#[path = "application_workspace_scope_tests.rs"]
mod scope;
#[path = "application_workspace_sender_tests.rs"]
mod sender;
#[path = "application_session_end_tests.rs"]
mod session_end;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
struct Class(HINSTANCE);
impl Drop for Class {
    fn drop(&mut self) {
        unsafe {
            drain_messages();
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
unsafe fn drain_messages() -> bool {
    // WM_QUIT 为低优先级生成消息；沿原 smoke 方式有界排空，不能只过滤退出消息。
    let mut quit = false;
    let mut message = MSG::default();
    for _ in 0..1024 {
        if !PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
            return quit;
        }
        if message.message == WM_QUIT {
            assert_eq!(message.wParam.0, 0);
            quit = true;
        } else {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    panic!("test message drain exceeded existing smoke bound");
}
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let root = std::path::Path::new(
            "C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-workspace-checkpoint-tests",
        );
        std::fs::create_dir_all(root).unwrap();
        let path = root.join(format!("{:?}", unsafe {
            windows::Win32::System::Com::CoCreateGuid().unwrap()
        }));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self) -> PathBuf {
        self.0.join("workspace.json")
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!(
                "workspace-checkpoint-failure-evidence: {}",
                self.0.display()
            );
            return;
        }
        assert!(self.0.starts_with(
            "C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-workspace-checkpoint-tests"
        ));
        for entry in std::fs::read_dir(&self.0).unwrap().flatten() {
            if entry.file_type().unwrap().is_file() {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        let _ = std::fs::remove_dir(&self.0);
    }
}
unsafe fn new_app(instance: HINSTANCE) -> Application {
    drain_messages();
    let image = crate::desktop::image::Image::from_encoded(
        crate::desktop::application::smoke::image_fixture::PNG,
    )
    .unwrap();
    Application::new(
        instance,
        DocumentWindow::create(instance, State::new(Some(image), false).unwrap(), false).unwrap(),
    )
}
unsafe fn settle(app: &mut Application) {
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        app.pending_workspace_save_with(|_, _, message, warning| assert!(!warning, "{message}"));
        if app.save_job.is_none() {
            break;
        }
        assert!(Instant::now() < end, "checkpoint deadline");
        std::thread::sleep(Duration::from_millis(1));
    }
}
unsafe fn ready(app: &Application) {
    let end = Instant::now() + Duration::from_secs(10);
    while !app.save_job.as_ref().unwrap().1.is_ready() {
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn checkpoint_roundtrip_is_self_contained_and_never_marks_manual_file_saved() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            let manual = directory.0.join("manual.json");
            let record = app.restored_session.as_mut().unwrap();
            let mut value: serde_json::Value = serde_json::from_slice(&record.original).unwrap();
            value["extension"] = serde_json::json!({"autosave-test":"preserved"});
            record.edit_original(serde_json::to_vec(&value).unwrap().into());
            crate::desktop::application::frozen_menu::reference::smoke::save(
                &mut app,
                Some(manual.clone()),
            )
            .unwrap();
            let manual_bytes = std::fs::read(&manual).unwrap();
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            let original = app.restored_session.as_ref().unwrap().original.clone();
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.5;
            settle(&mut app);
            let first = std::fs::read(directory.path()).unwrap();
            assert_eq!(crate::session_document::decode(&first).unwrap().revision, 1);
            assert_eq!(app.save_status(), save_state::Status::Changed);
            assert!(original.same_storage(&app.restored_session.as_ref().unwrap().original));
            assert_eq!(std::fs::read(&manual).unwrap(), manual_bytes);
            settle(&mut app);
            assert_eq!(
                std::fs::read(directory.path()).unwrap(),
                first,
                "unchanged must not write"
            );
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.6;
            settle(&mut app);
            let second = std::fs::read(directory.path()).unwrap();
            assert_eq!(
                crate::session_document::decode(&second).unwrap().revision,
                2
            );
            let value: serde_json::Value = serde_json::from_slice(&second).unwrap();
            assert!(value["stickers"][0]["src"]
                .as_str()
                .unwrap()
                .starts_with("data:image/png;base64,"));
            assert_eq!(value["extension"]["autosave-test"], "preserved");
            app.request_save_existing().unwrap();
            ready(&app);
            app.take_save_candidate()
                .unwrap()
                .unwrap()
                .commit_ready(&mut app, &AtomicBool::new(false))
                .unwrap();
            assert_eq!(app.save_status(), save_state::Status::Unchanged);
            drop(app);
            let mut restored = new_app(instance);
            restored
                .install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            assert_eq!(restored.documents.len(), 1);
            assert_eq!(
                restored.documents[0].state.canvas.borrow().normal_opacity,
                0.6
            );
            assert!(restored
                .restored_session
                .as_ref()
                .unwrap()
                .saved_path
                .is_none());
            assert_eq!(restored.save_status(), save_state::Status::Unsaved);
            assert!(!restored.documents[0].state.global_capture.get());
            assert!(!IsWindowVisible(restored.documents[0].window.0).as_bool());
            restored.documents[0]
                .state
                .canvas
                .borrow_mut()
                .normal_opacity = 0.4;
            settle(&mut restored);
            assert_eq!(
                crate::session_document::decode(&std::fs::read(directory.path()).unwrap())
                    .unwrap()
                    .revision,
                3,
                "restored checkpoint revision remains independent of manual saves"
            );
            assert_eq!(restored.save_status(), save_state::Status::Unsaved);
            drop(restored);
        });
    }
}
#[test]
fn busy_defers_stale_retries_latest_and_external_conflict_stops_once() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            app.documents[0].state.file_dialog.set(true);
            app.pending_workspace_save_with(|_, _, _, _| panic!("modal"));
            assert!(app.save_job.is_none());
            app.documents[0].state.file_dialog.set(false);
            app.start_checkpoint().unwrap();
            ready(&app);
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.5;
            app.poll_checkpoint().unwrap();
            assert!(!directory.path().exists());
            settle(&mut app);
            let original = app.restored_session.as_ref().unwrap().original.clone();
            std::fs::write(directory.path(), b"external sentinel").unwrap();
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.6;
            let mut warnings = 0;
            let end = Instant::now() + Duration::from_secs(10);
            while !app.workspace_checkpoint.as_ref().unwrap().stopped {
                app.pending_workspace_save_with(|_, _, _, warning| {
                    warnings += usize::from(warning)
                });
                assert!(Instant::now() < end);
                std::thread::sleep(Duration::from_millis(1));
            }
            for _ in 0..3 {
                app.pending_workspace_save_with(|_, _, _, _| panic!("duplicate error"));
            }
            assert_eq!(warnings, 1);
            assert_eq!(
                std::fs::read(directory.path()).unwrap(),
                b"external sentinel"
            );
            assert!(original.same_storage(&app.restored_session.as_ref().unwrap().original));
            drop(app);
        });
    }
}
#[test]
fn queued_open_manual_and_exit_are_not_consumed_by_checkpoint() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            app.start_checkpoint().unwrap();
            app.documents[0].state.save_existing_requested.set(true);
            app.documents[0].state.open_session_requested.set(true);
            app.pending_open_session_with(|_, _, _| panic!("must defer"))
                .unwrap();
            assert!(app.documents[0].state.save_existing_requested.get());
            assert!(app.documents[0].state.open_session_requested.get());
            app.exit_requested.set(true);
            assert!(!app.close_requested_with(|_| panic!("must defer")));
            ready(&app);
            app.poll_checkpoint().unwrap();
            assert!(app.documents[0].state.save_existing_requested.get());
            assert!(app.documents[0].state.open_session_requested.get());
            assert!(app.exit_requested.get());
            assert!(!directory.path().exists());
            assert!(!app.close_requested_with(|_| exit::Choice::Cancel));
            assert!(!app.exit_requested.get());
            assert!(!app.workspace_checkpoint.as_ref().unwrap().stopped);
            drop(app);
        });
    }
}
#[test]
fn deleting_last_unit_commits_empty_but_application_exit_preserves_workspace() {
    unsafe {
        run(|instance| {
            for mode in 0..4 {
                let exiting = mode == 3;
                let directory = Directory::new();
                let mut app = new_app(instance);
                app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                    .unwrap();
                settle(&mut app);
                let before = std::fs::read(directory.path()).unwrap();
                if mode == 1 {
                    app.request_save_candidate().unwrap();
                }
                if mode == 2 {
                    app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.8;
                    app.start_checkpoint().unwrap();
                }
                if exiting {
                    app.exit_requested.set(true);
                    assert!(!app.close_requested_with(|_| {
                        crate::desktop::application::exit::Choice::Discard
                    }));
                    ready(&app);
                    app.pending_workspace_save_with(|_, _, message, warning| {
                        assert!(!warning, "{message}")
                    });
                    assert!(
                        app.close_requested_with(|_| panic!("final checkpoint already approved"))
                    );
                } else {
                    SendMessageW(app.documents[0].window.0, WM_CLOSE, None, None);
                }
                // 真实最后窗口消息已排队；复用 run_loop 在 WM_QUIT 之后调用的同一收尾入口。
                assert!(drain_messages());
                app.finish_deleted_workspace().unwrap();
                let after = std::fs::read(directory.path()).unwrap();
                if exiting {
                    let old = crate::session_document::decode(&before).unwrap();
                    let new = crate::session_document::decode(&after).unwrap();
                    assert_eq!(new.revision, old.revision + 1);
                    assert_eq!(new.workspace.units(), old.workspace.units());
                } else {
                    assert!(crate::session_document::decode(&after)
                        .unwrap()
                        .workspace
                        .units()
                        .is_empty());
                }
                drop(app);
            }
        });
    }
}
