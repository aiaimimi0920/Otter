//! 显式文件自动保存与 Live 首帧/流变化/忙手势；不改变手动 dirty 或开关授权。
use super::*;
#[test]
fn file_autosave_waits_for_first_frame_then_saves_only_model_changes() {
    unsafe {
        let environment = Environment::new();
        let directory = Directory::new();
        let mut app = app(&environment);
        let path = directory.0.join("manual.json");
        save_manual(&mut app, path.clone());
        let before = std::fs::read(&path).unwrap();
        app.workspace_autosave.toggle().unwrap();
        let (hwnd, id, source) = super::super::refresh::stream(&mut app);
        settle_auto(&mut app);
        assert!(app.workspace_autosave.enabled());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        source.publish([0, 0, 255, 255]);
        tick(&mut app);
        assert!(app.automatic_save_pending());
        // worker 已捕获红帧；在交付前流变绿不能改变本次候选/基线。
        source.publish([0, 255, 0, 255]);
        ready(&app);
        settle_auto(&mut app);
        let first = std::fs::read(&path).unwrap();
        assert_eq!(revision(&first), revision(&before) + 1);
        pixels(&first, &id, [0, 0, 255, 255]);
        for _ in 0..4 {
            settle_auto(&mut app);
        }
        assert_eq!(std::fs::read(&path).unwrap(), first);
        assert_eq!(app.save_status(), save_state::Status::Changed);
        SetWindowPos(
            hwnd,
            None,
            -180,
            120,
            120,
            100,
            SWP_NOACTIVATE | SWP_NOZORDER,
        )
        .unwrap();
        settle_auto(&mut app);
        let second = std::fs::read(&path).unwrap();
        assert_eq!(revision(&second), revision(&first) + 1);
        pixels(&second, &id, [0, 255, 0, 255]);
        assert!(app.workspace_autosave.enabled());
        drop(source);
        app.shutdown_live().unwrap();
    }
}

#[test]
fn inflight_move_and_busy_live_gesture_expire_normally_then_save_the_latest_frame() {
    unsafe {
        let environment = Environment::new();
        for file in [false, true] {
            let directory = Directory::new();
            let mut app = app(&environment);
            let path = if file {
                let path = directory.0.join("manual.json");
                save_manual(&mut app, path.clone());
                app.workspace_autosave.toggle().unwrap();
                path
            } else {
                install(&mut app, &directory, false);
                directory.path()
            };
            let (hwnd, id, source) = super::super::refresh::stream(&mut app);
            source.publish([0, 0, 255, 255]);
            settle_auto(&mut app);
            let before = std::fs::read(&path).unwrap();
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.6;
            tick(&mut app);
            assert!(app.automatic_save_pending());
            ready(&app);
            // 只改线程键盘快照，驱动自有隐藏 HWND 的真实拖动，不注入全局输入。
            crate::desktop::composition::live::window::local::tests::with_keys(
                &[windows::Win32::UI::Input::KeyboardAndMouse::VK_MENU],
                || {
                    SendMessageW(hwnd, WM_LBUTTONDOWN, None, Some(LPARAM(32 | (32 << 16))));
                },
            );
            assert!(!app.live_sessions[0].persistence_idle());
            SetWindowPos(
                hwnd,
                None,
                -160,
                130,
                120,
                100,
                SWP_NOACTIVATE | SWP_NOZORDER,
            )
            .unwrap();
            tick(&mut app);
            assert!(!app.automatic_save_pending());
            assert_eq!(std::fs::read(&path).unwrap(), before);
            source.publish([255, 0, 0, 255]);
            tick(&mut app);
            assert!(app.save_job.is_none());
            SendMessageW(hwnd, WM_CANCELMODE, None, None);
            settle_auto(&mut app);
            let after = std::fs::read(&path).unwrap();
            assert_eq!(revision(&after), revision(&before) + 1);
            pixels(&after, &id, [255, 0, 0, 255]);
            if file {
                assert!(app.workspace_autosave.enabled());
            }
            drop(source);
            app.shutdown_live().unwrap();
        }
    }
}
