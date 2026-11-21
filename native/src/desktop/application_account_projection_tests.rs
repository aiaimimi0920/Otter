//! 实际 Application 的 v2 接收追加、落盘包含关系和重开；不把隐藏窗口当作已显示。
use super::*;
use crate::desktop::application::account_projection::{Destination, Prepared};
use crate::projection_account::{scene::Binding, tests::scene_tests::fixture};

unsafe fn prepare(app: &mut Application) -> (Destination, Prepared, Binding) {
    app.capture_session_snapshot().unwrap();
    let destination = Destination::capture(app).unwrap();
    let (content, binding) = fixture(destination.scope, "receive-1");
    let candidate_binding = binding.clone();
    let space = destination.space;
    let vector = std::thread::spawn(move || {
        let _com = crate::desktop::Apartment::new().unwrap();
        crate::desktop::session_candidate::account_projection::prepare(
            content,
            &candidate_binding,
            space,
            crate::session_window::Rect {
                x: 20.0,
                y: 30.0,
                w: 160.0,
                h: 80.0,
            },
            &AtomicBool::new(false),
        )
        .unwrap()
    })
    .join()
    .unwrap();
    let prepared = destination
        .prepare(
            vector,
            binding.clone(),
            app.workspace.units(),
            &AtomicBool::new(false),
        )
        .unwrap();
    (destination, prepared, binding)
}
#[test]
fn account_receive_is_not_saved_until_real_checkpoint_and_reopens_with_same_pixels_and_binding() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            let original_unit = app.documents[0].state.unit_id.clone();
            let original_window = app.documents[0].window.0;
            let (destination, prepared, binding) = prepare(&mut app);
            let receipt = app
                .publish_account_projection(&destination, prepared)
                .unwrap();
            assert!(!receipt.saved(&app));
            let received = crate::workspace_id::UnitId::parse(binding.unit()).unwrap();
            assert_eq!(app.documents.len(), 2);
            assert_eq!(app.source_window(&original_unit), Some(original_window));
            assert!(!IsWindowVisible(app.source_window(&received).unwrap()).as_bool());
            settle(&mut app);
            assert!(receipt.saved(&app));
            let bytes = std::fs::read(directory.path()).unwrap();
            let source = bytes.clone().into();
            assert!(crate::projection_account::scene::Index::capture(&source)
                .unwrap()
                .includes(&binding));
            let saved: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            let row = saved["stickers"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["id"] == binding.unit())
                .unwrap();
            let image = crate::desktop::session_image::inline(
                row["src"].as_str().unwrap(),
                &AtomicBool::new(false),
            )
            .unwrap();
            assert_eq!((image.width, image.height), (2, 1));
            assert_eq!(&image.pixels[..], &[211, 83, 17, 255, 9, 8, 7, 255]);
            drop(app);
            let mut reopened = new_app(instance);
            reopened
                .install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            assert_eq!(reopened.documents.len(), 2);
            assert!(reopened
                .workspace_checkpoint
                .as_ref()
                .unwrap()
                .includes_account(&binding, &reopened.workspace));
        });
    }
}
#[test]
fn account_receive_rejects_late_destination_and_failed_checkpoint_never_grants_saved_receipt() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            let (destination, prepared, _) = prepare(&mut app);
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.5;
            assert!(app
                .publish_account_projection(&destination, prepared)
                .is_err());
            assert_eq!(app.documents.len(), 1);
            let (destination, prepared, _) = prepare(&mut app);
            let receipt = app
                .publish_account_projection(&destination, prepared)
                .unwrap();
            std::fs::write(directory.path(), b"external sentinel").unwrap();
            let end = Instant::now() + Duration::from_secs(10);
            while !app.workspace_checkpoint.as_ref().unwrap().stopped {
                app.pending_workspace_save_with(|_, _, _, _| {});
                assert!(Instant::now() < end);
                std::thread::sleep(Duration::from_millis(1));
            }
            assert!(!receipt.saved(&app));
            assert_eq!(
                std::fs::read(directory.path()).unwrap(),
                b"external sentinel"
            );
        });
    }
}

#[test]
fn account_receive_consent_change_invalidates_inflight_even_when_reenabled() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            let (destination, prepared, _) = prepare(&mut app);
            app.authorization
                .borrow_mut()
                .consent
                .install(crate::projection_receive_policy::Policy::Reject);
            app.authorization
                .borrow_mut()
                .consent
                .install(crate::projection_receive_policy::Policy::Confirm);
            assert!(app
                .publish_account_projection(&destination, prepared)
                .is_err());
            assert_eq!(app.documents.len(), 1);
            let (destination, prepared, _) = prepare(&mut app);
            app.authorization.borrow_mut().consent.install_paused(true);
            app.authorization.borrow_mut().consent.install_paused(false);
            assert!(app
                .publish_account_projection(&destination, prepared)
                .is_err());
            assert_eq!(app.documents.len(), 1);
        });
    }
}

#[test]
fn account_existing_recovery_survives_new_receive_rejection_but_not_pause() {
    unsafe {
        run(|instance| {
            let directory = Directory::new();
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                .unwrap();
            app.capture_session_snapshot().unwrap();
            app.authorization
                .borrow_mut()
                .consent
                .install(crate::projection_receive_policy::Policy::Reject);
            assert!(Destination::capture(&app).is_err());
            let destination = Destination::capture_existing(&app).unwrap();
            assert!(destination.matches(&app));
            app.authorization.borrow_mut().consent.install_paused(true);
            app.authorization.borrow_mut().consent.install_paused(false);
            assert!(!destination.matches(&app));
        });
    }
}

#[test]
fn account_saved_receive_does_not_present_after_cancel_pause_policy_or_origin_change() {
    unsafe {
        run(|instance| {
            for change in ["cancel", "pause", "policy", "origin", "none"] {
                let directory = Directory::new();
                let mut app = new_app(instance);
                app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                    .unwrap();
                let (destination, prepared, binding) = prepare(&mut app);
                app.authorization.borrow_mut().origin = Some(binding.origin().into());
                let receipt = app
                    .publish_account_projection(&destination, prepared)
                    .unwrap();
                let cancel = AtomicBool::new(false);
                match change {
                    "cancel" => cancel.store(true, std::sync::atomic::Ordering::Release),
                    "pause" => {
                        app.authorization.borrow_mut().consent.install_paused(true);
                        app.authorization.borrow_mut().consent.install_paused(false);
                    }
                    "policy" => {
                        app.authorization
                            .borrow_mut()
                            .consent
                            .install(crate::projection_receive_policy::Policy::Reject);
                        app.authorization
                            .borrow_mut()
                            .consent
                            .install(crate::projection_receive_policy::Policy::Confirm);
                    }
                    "origin" => {
                        app.authorization.borrow_mut().origin = Some("https://other.example".into())
                    }
                    "none" => {}
                    _ => unreachable!(),
                }
                settle(&mut app);
                assert!(receipt.saved(&app), "{change}");
                let unit = crate::workspace_id::UnitId::parse(binding.unit()).unwrap();
                let hwnd = app.source_window(&unit).unwrap();
                assert!(!IsWindowVisible(hwnd).as_bool());
                assert_eq!(
                    receipt.present(&app, Some(&cancel)).unwrap(),
                    change == "none"
                );
                assert_eq!(IsWindowVisible(hwnd).as_bool(), change == "none");
            }
        });
    }
}
