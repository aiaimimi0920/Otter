//! 默认退出保存场景后重开；不导出图片、不触碰用户 profile 或显示确认框。
use super::*;

#[test]
fn default_exit_saves_last_layout_and_reopens_without_manual_file_confirmation() {
    unsafe {
        run(|instance| {
            for manual in [false, true] {
                let directory = Directory::new();
                let mut app = new_app(instance);
                let path = directory.0.join("manual.json");
                let manual_bytes = if manual {
                    crate::desktop::application::frozen_menu::reference::smoke::save(
                        &mut app,
                        Some(path.clone()),
                    )
                    .unwrap();
                    Some(std::fs::read(&path).unwrap())
                } else {
                    None
                };
                app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                    .unwrap();
                settle(&mut app);
                let hwnd = app.documents[0].window.0;
                SetWindowPos(
                    hwnd,
                    None,
                    280,
                    240,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                )
                .unwrap();
                app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.55;
                assert_ne!(app.save_status(), save_state::Status::Unchanged);
                app.exit_requested.set(true);
                // 调用真正默认入口；旧实现会进入 MessageBox 而无法完成此测试。
                assert!(!app.close_requested());
                assert!(app.checkpoint_exit_waiting() && app.checkpoint_pending());
                assert!(IsWindow(Some(hwnd)).as_bool());
                assert!(!app.documents[0].state.save_existing_requested.get());
                ready(&app);
                app.pending_workspace_save_with(|_, _, message, warning| {
                    assert!(!warning, "{message}")
                });
                assert!(app.close_requested());
                assert!(app.documents.is_empty());
                assert!(drain_messages());
                app.finish_deleted_workspace().unwrap();
                let bytes = std::fs::read(directory.path()).unwrap();
                let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(value["stickers"][0]["x"].as_f64(), Some(280.0));
                assert_eq!(value["stickers"][0]["y"].as_f64(), Some(240.0));
                assert!(value["stickers"][0]["src"]
                    .as_str()
                    .unwrap()
                    .starts_with("data:image/png;base64,"));
                if let Some(original) = manual_bytes {
                    assert_eq!(std::fs::read(&path).unwrap(), original);
                } else {
                    assert!(!path.exists());
                }
                drop(app);
                let mut restored = new_app(instance);
                restored
                    .install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                    .unwrap();
                assert_eq!(restored.documents.len(), 1);
                assert_eq!(
                    restored.documents[0].state.canvas.borrow().normal_opacity,
                    0.55
                );
                let mut rect = RECT::default();
                GetWindowRect(restored.documents[0].window.0, &mut rect).unwrap();
                assert_eq!((rect.left, rect.top), (280, 240));
                assert!(restored
                    .restored_session
                    .as_ref()
                    .unwrap()
                    .saved_path
                    .is_none());
                assert_eq!(restored.save_status(), save_state::Status::Unsaved);
                drop(restored);
            }
        });
    }
}

#[test]
fn stopped_scene_save_keeps_unsaved_windows_and_consumes_default_exit() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            settle(&mut app);
            let before = std::fs::read(directory.path()).unwrap();
            app.workspace_checkpoint.as_mut().unwrap().stopped = true;
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.4;
            app.exit_requested.set(true);
            let mut notices = 0;
            assert!(
                !app.close_requested_automatically_with_notify(|_, message| {
                    assert!(message.contains("场景自动保存不可用"));
                    notices += 1;
                })
            );
            assert_eq!(notices, 1);
            assert!(!app.exit_requested.get());
            assert!(app.documents[0].is_alive());
            assert_eq!(std::fs::read(directory.path()).unwrap(), before);
            assert!(!app.close_requested_automatically_with_notify(|_, _| panic!("no latent exit")));
            // 明确另存成功后仍可退出，不把故障状态变成无法退出的永久门禁。
            crate::desktop::application::frozen_menu::reference::smoke::save(
                &mut app,
                Some(directory.0.join("rescue.json")),
            )
            .unwrap();
            app.exit_requested.set(true);
            assert!(app.close_requested_automatically_with_notify(|_, text| panic!("{text}")));
            assert!(app.documents.is_empty());
            drop(app);
        });
    }
}
