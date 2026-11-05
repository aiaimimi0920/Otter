//! 退出批准是一次性的；晚到文件/Live 请求不被最终保存或自动续退越过。
use super::*;
#[test]
fn late_save_open_freeze_and_recycle_cancel_exit_without_consuming_requests() {
    unsafe {
        let environment = Environment::new();
        for mode in 0..4 {
            let directory = Directory::new();
            let mut app = app(&environment);
            let (hwnd, _) = live(&mut app, true);
            install(&mut app, &directory, false);
            begin(&mut app);
            ready(&app);
            match mode {
                0 => app.documents[0].state.save_session_requested.set(true),
                1 => app.documents[0].state.open_session_requested.set(true),
                2 => queue(hwnd),
                _ => {
                    SendMessageW(hwnd, WM_CLOSE, None, None);
                }
            }
            tick(&mut app);
            app.pending_open_session_with(|_, _, _| panic!("defer until exit is cancelled"))
                .unwrap();
            assert!(!app.close_requested_with(|_| panic!("cancel stale exit")));
            assert!(!app.checkpoint_exit_waiting() && app.documents[0].is_alive());
            assert!(IsWindow(Some(hwnd)).as_bool() && !directory.path().exists());
            match mode {
                0 => assert!(app.documents[0].state.save_session_requested.replace(false)),
                1 => assert!(app.documents[0].state.open_session_requested.replace(false)),
                2 => assert!(app.live_sessions[0].take_freeze_request()),
                _ => assert!(app.live_sessions[0].take_recycle_request()),
            }
            settle_auto(&mut app);
            assert!(
                directory.path().exists(),
                "normal stale must not disable owner"
            );
            assert!(!app.close_requested_with(|_| panic!("no latent exit")));
            app.shutdown_live().unwrap();
        }
    }
}

#[test]
fn post_commit_new_window_and_save_as_are_checked_before_destroying_sources() {
    unsafe {
        let environment = Environment::new();
        for mode in 0..5 {
            let directory = Directory::new();
            let mut app = app(&environment);
            let (hwnd, _) = live(&mut app, true);
            install(&mut app, &directory, false);
            begin(&mut app);
            ready(&app);
            tick(&mut app);
            match mode {
                0 => app.documents[0].state.new_window.set(true),
                1 => app.documents[0].state.save_session_requested.set(true),
                2 => app.documents[0].state.open_session_requested.set(true),
                3 => queue(hwnd),
                _ => {
                    SendMessageW(hwnd, WM_CLOSE, None, None);
                }
            }
            assert!(!app.close_requested_with(|_| panic!("late intent cancels final exit")));
            assert!(app.documents[0].is_alive() && IsWindow(Some(hwnd)).as_bool());
            match mode {
                0 => assert!(app.documents[0].state.new_window.replace(false)),
                1 => assert!(app.documents[0].state.save_session_requested.replace(false)),
                2 => assert!(app.documents[0].state.open_session_requested.replace(false)),
                3 => assert!(app.live_sessions[0].take_freeze_request()),
                _ => assert!(app.live_sessions[0].take_recycle_request()),
            }
            app.shutdown_live().unwrap();
        }
    }
}

#[test]
fn successful_live_save_resumes_once_without_marking_stream_clean_and_late_edit_cancels() {
    unsafe {
        let environment = Environment::new();
        for mode in 0..3 {
            let changed = mode == 1;
            let directory = Directory::new();
            let mut app = app(&environment);
            let (_, _) = live(&mut app, true);
            if mode == 2 {
                install(&mut app, &directory, false);
            }
            app.request_save_new(directory.0.join("manual.json"))
                .unwrap();
            app.save_origin = Some(app.documents[0].state.unit_id.clone());
            app.exit_after_save = true;
            ready(&app);
            app.pending_save_ui_with(|_, _, message, warning| assert!(!warning, "{message}"));
            assert!(app.exit_requested.get() && !app.exit_after_save);
            assert_eq!(app.save_status(), save_state::Status::Changed);
            if changed {
                app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.5;
            }
            let closed = app.close_requested_with(|_| panic!("save approval must not re-prompt"));
            if mode == 2 {
                assert!(!closed && app.checkpoint_exit_waiting());
                assert!(finish(&mut app));
                assert_eq!(revision(&directory.bytes()), 1);
            } else {
                assert_eq!(closed, !changed);
            }
            assert!(app.exit_saved.is_none());
            if changed {
                assert!(app.documents[0].is_alive());
                assert!(!app.close_requested_with(|_| panic!("no latent exit")));
            }
            app.shutdown_live().unwrap();
        }
    }
}
