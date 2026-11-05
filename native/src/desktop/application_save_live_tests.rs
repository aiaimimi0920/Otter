//! 复用隐藏 Live/WARP fixture，验证生产保存 worker、磁盘提交和实际 PNG 恢复。
use super::*;
use crate::desktop::composition::live::window::local::tests::wheel;
use windows::Win32::UI::Input::KeyboardAndMouse::VK_MENU;
pub(super) unsafe fn prepared(app: &mut Application) -> Result<save_publish::Prepared> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(value) = app.take_save_candidate()? {
            return Ok(value);
        }
        assert!(Instant::now() < deadline, "Live save worker did not settle");
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn live_save_publishes_same_id_pixels_without_stopping_capture_and_roundtrips() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let (hwnd, id) = live(&mut app, true);
        wheel(hwnd, &[VK_MENU], -650, (0, 0));
        let path = save_publish::smoke_destination().unwrap();
        let cancel = AtomicBool::new(false);
        app.request_save_new(path.clone()).unwrap();
        let output = prepared(&mut app).unwrap();
        let bytes = output.candidate.bytes.clone();
        let decoded = crate::session_document::decode(&bytes).unwrap();
        assert!(decoded.workspace.contains(&id));
        output.commit_ready(&mut app, &cancel).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), bytes.as_ref());
        assert!(IsWindow(Some(hwnd)).as_bool());
        assert!(app.live_sessions[0].is_alive());
        assert_eq!(app.save_status(), save_state::Status::Changed);
        let restored =
            crate::desktop::session_candidate::prepare(bytes.to_vec(), None, &cancel).unwrap();
        assert_eq!(restored.images.len(), 1);
        let image = restored.images[0].source.as_ref().unwrap();
        assert_eq!([image.width, image.height], [2, 2]);
        assert_eq!(&*image.pixels, &[0, 0, 255, 255].repeat(4));
        // 同一 Live 再保存复用身份/磁盘基线，不追加重复 sticker。
        app.request_save_existing().unwrap();
        let second = prepared(&mut app).unwrap();
        assert_eq!(second.candidate.revision, 2);
        second.commit_ready(&mut app, &cancel).unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(json["stickers"].as_array().unwrap().len(), 1);
        assert_eq!(json["stickers"][0]["id"], id.as_str());
        assert!((json["stickers"][0]["opacityNormal"].as_f64().unwrap() - 0.35).abs() < 0.00001);
        assert!(json["stickers"][0].get("target").is_none());
        assert!(app.live_sessions[0].is_alive());
        app.shutdown_live().unwrap();
        assert_eq!(app.save_status(), save_state::Status::Changed);
        std::fs::remove_file(path).unwrap();
    }
}
#[test]
fn live_save_rejects_no_frame_cancel_closed_moved_and_added_sessions() {
    unsafe {
        let environment = Environment::new();
        for mode in 0..6 {
            let mut app = environment.app();
            let (hwnd, _) = live(&mut app, mode != 0);
            let original = app.restored_session.as_ref().unwrap().original.clone();
            if mode == 0 {
                assert!(app.request_save_candidate().is_err());
                assert!(app.save_job.is_none());
            } else {
                app.request_save_candidate().unwrap();
                match mode {
                    1 => app.cancel_save_candidate(),
                    2 => {
                        DestroyWindow(hwnd).unwrap();
                    }
                    3 => {
                        SetWindowPos(hwnd, None, 10, 20, 300, 200, SWP_NOACTIVATE | SWP_NOZORDER)
                            .unwrap();
                    }
                    4 => {
                        live(&mut app, true);
                    }
                    5 => wheel(hwnd, &[VK_MENU], -120, (0, 0)),
                    _ => unreachable!(),
                }
                assert!(prepared(&mut app).is_err());
                assert!(app.save_job.is_none());
            }
            assert!(app
                .restored_session
                .as_ref()
                .unwrap()
                .original
                .same_storage(&original));
            if mode != 2 {
                assert!(app.live_sessions[0].is_alive());
            }
            app.shutdown_live().unwrap();
        }
    }
}

#[test]
fn live_save_receipt_rechecks_opacity_after_worker_delivery() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let (hwnd, _) = live(&mut app, true);
        let path = save_publish::smoke_destination().unwrap();
        app.request_save_new(path.clone()).unwrap();
        let output = prepared(&mut app).unwrap();
        wheel(hwnd, &[VK_MENU], -120, (0, 0));
        assert!(output
            .commit_ready(&mut app, &AtomicBool::new(false))
            .is_err());
        assert!(!path.exists());
        assert!(app.live_sessions[0].is_alive());
        app.shutdown_live().unwrap();
    }
}
#[test]
fn live_save_receipt_rechecks_geometry_after_worker_delivery() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let (hwnd, _) = live(&mut app, true);
        let path = save_publish::smoke_destination().unwrap();
        app.request_save_new(path.clone()).unwrap();
        let output = prepared(&mut app).unwrap();
        SetWindowPos(hwnd, None, 25, 35, 300, 200, SWP_NOACTIVATE | SWP_NOZORDER).unwrap();
        assert!(output
            .commit_ready(&mut app, &AtomicBool::new(false))
            .is_err());
        assert!(!path.exists());
        assert!(app.live_sessions[0].is_alive());
        app.shutdown_live().unwrap();
    }
}
