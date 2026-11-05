//! Live 生产关闭/系统菜单到回收、真实保存和恢复；只使用隐藏窗口及合成 WARP 帧。
use super::*;
use crate::desktop::composition::live::window::local::tests::wheel;
use crate::session_library::Kind;
use windows::Win32::UI::Input::KeyboardAndMouse::VK_MENU;
#[test]
fn live_recycle_explicit_discard_can_close_unframed_old_workspace_without_library_write() {
    unsafe {
        let environment = Environment::new();
        let mut app = environment.app();
        let (hwnd, _) = live(&mut app, false);
        let source = app.restored_session.as_ref().unwrap().original.clone();
        app.workspace =
            crate::workspace::Workspace::with_initial(app.documents[0].state.unit_id.clone());
        SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(0x1fc0)), Some(LPARAM(0)));
        assert!(!IsWindow(Some(hwnd)).as_bool());
        assert!(app
            .restored_session
            .as_ref()
            .unwrap()
            .original
            .same_storage(&source));
        assert!(app.save_job.is_none());
        app.shutdown_live().unwrap();
    }
}
unsafe fn request_close(hwnd: HWND, menu: bool) {
    if menu {
        SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(0x1fd0)), Some(LPARAM(0)));
    } else {
        SendMessageW(hwnd, WM_CLOSE, None, None);
    }
}
pub(super) unsafe fn settle_recycle(app: &mut Application) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.save_job.is_some() {
        app.process_reference(false)?;
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}
#[test]
fn live_recycle_close_and_menu_preserve_frame_until_commit_and_restore_from_library() {
    unsafe {
        let environment = Environment::new();
        for menu in [false, true] {
            let mut app = environment.app();
            let (hwnd, id) = live(&mut app, true);
            wheel(hwnd, &[VK_MENU], -650, (0, 0));
            let host = app.documents[0].window.0;
            request_close(hwnd, menu);
            assert!(IsWindow(Some(hwnd)).as_bool());
            app.process_live(false).unwrap();
            assert!(app.live_sessions[0].is_alive());
            assert!(app.save_job.is_some());
            // 普通磁盘保存消费者不能领取会关闭窗口的结果。
            assert!(app.take_save_candidate().is_err());
            settle_recycle(&mut app).unwrap();
            assert!(!IsWindow(Some(hwnd)).as_bool());
            assert!(IsWindow(Some(host)).as_bool());
            app.shutdown_live().unwrap();
            let path = save_publish::smoke_destination().unwrap();
            app.request_save_new(path.clone()).unwrap();
            super::save::prepared(&mut app)
                .unwrap()
                .commit_ready(&mut app, &AtomicBool::new(false))
                .unwrap();
            let bytes = std::fs::read(&path).unwrap();
            let doc = crate::session_document::decode(&bytes).unwrap();
            assert!(doc.windows.is_empty());
            let library = doc.library(Kind::RecycleBin).unwrap();
            assert_eq!(library.entries().len(), 1);
            let entry_id = library.entries()[0].entry_id.to_string();
            let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(json["recycleBin"][0]["sourceStickerId"], id.as_str());
            let source = app.restored_session.as_ref().unwrap().original.clone();
            app.request_frozen(source, Kind::RecycleBin, entry_id, POINT { x: 0, y: 0 })
                .unwrap();
            let deadline = Instant::now() + Duration::from_secs(5);
            while app.restore_job.is_some() {
                app.pending_windows(false).unwrap();
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(1));
            }
            assert_eq!(app.documents.len(), 2);
            let canvas = app.documents[1].state.canvas.borrow();
            assert!((canvas.normal_opacity - 0.35).abs() < 0.00001);
            assert!((canvas.window_opacity - 0.35).abs() < 0.00001);
            // 恢复器的显示栅格按贴纸几何生成；原帧由独立 source owner 保留。
            let image = canvas.minified_source.as_ref().unwrap().image();
            assert_eq!([image.width, image.height], [2, 2]);
            assert_eq!(&*image.pixels, &[0, 0, 255, 255].repeat(4));
            assert!(canvas
                .image
                .as_ref()
                .unwrap()
                .pixels
                .chunks_exact(4)
                .all(|pixel| pixel == [0, 0, 255, 255]));
            drop(canvas);
            assert!(crate::session_library::decode(
                &app.restored_session.as_ref().unwrap().original,
                Kind::RecycleBin
            )
            .unwrap()
            .entries()
            .is_empty());
            std::fs::remove_file(path).unwrap();
        }
    }
}
#[test]
fn live_recycle_failed_or_stale_requests_never_close_or_publish() {
    unsafe {
        let environment = Environment::new();
        for mode in 0..6 {
            let mut app = environment.app();
            let (hwnd, _) = live(&mut app, mode != 0);
            let source = app.restored_session.as_ref().unwrap().original.clone();
            request_close(hwnd, false);
            if mode == 0 {
                assert!(app.process_live_recycle().is_err());
            } else if mode == 1 {
                app.workspace = crate::workspace::Workspace::with_initial(
                    app.documents[0].state.unit_id.clone(),
                );
                assert!(app.process_live_recycle().is_err());
            } else {
                app.process_live_recycle().unwrap();
                match mode {
                    2 => app.cancel_save_candidate(),
                    3 => {
                        SetWindowPos(hwnd, None, 30, 40, 300, 200, SWP_NOACTIVATE | SWP_NOZORDER)
                            .unwrap();
                    }
                    4 => {
                        app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.5;
                    }
                    5 => wheel(hwnd, &[VK_MENU], -120, (0, 0)),
                    _ => unreachable!(),
                }
                assert!(settle_recycle(&mut app).is_err());
            }
            assert!(app.save_job.is_none());
            assert!(IsWindow(Some(hwnd)).as_bool());
            assert!(app.live_sessions[0].is_alive());
            assert!(app
                .restored_session
                .as_ref()
                .unwrap()
                .original
                .same_storage(&source));
            app.shutdown_live().unwrap();
        }
    }
}
