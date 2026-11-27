//! Reopen the scene saved by default exit; no image export, user-profile access, or confirmation dialogs.
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
                // Call the real default entry point; the old implementation would enter MessageBox and prevent this test from finishing.
                assert!(!app.close_requested());
                assert!(app.checkpoint_exit_waiting() && app.checkpoint_pending());
                assert!(IsWindow(Some(hwnd)).as_bool());
                assert!(!app.documents[0].state.save_existing_requested.get());
                ready(&app);
                app.pending_workspace_save_with(|_, _, message, warning| {
                    assert!(!warning, "{message}")
                });
                // Compute the receiving test's receipt from the actual published
                // candidate, not by hashing the file we later ask it to verify.
                use sha2::{Digest, Sha256};
                let owner = app.workspace_checkpoint.as_ref().unwrap();
                let expected = crate::desktop::workspace_profile::restart::Expected::parse(
                    &owner.revision.to_string(),
                    &format!(
                        "{:x}",
                        Sha256::digest(owner.original.as_ref().unwrap().as_slice())
                    ),
                )
                .unwrap();
                assert!(Profile::claim(&directory.0, expected).is_err());
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
                    .install_workspace_checkpoint(
                        Some(Profile::claim(&directory.0, expected).unwrap()),
                        true,
                    )
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
                    assert!(message.contains(crate::ui_text::text("app-exit-autosave-unavailable")));
                    notices += 1;
                })
            );
            assert_eq!(notices, 1);
            assert!(!app.exit_requested.get());
            assert!(app.documents[0].is_alive());
            assert_eq!(std::fs::read(directory.path()).unwrap(), before);
            assert!(!app.close_requested_automatically_with_notify(|_, _| panic!("no latent exit")));
            // Exit remains possible after explicit save-as succeeds; a failure state must not become a permanent no-exit gate.
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
