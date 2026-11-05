//! 正常退出保存最后时间点；隐藏 HWND/合成流，不向用户桌面注入输入。
use super::*;
use crate::desktop::application::exit::Choice;
#[path = "application_live_checkpoint_exit_failure_tests.rs"]
mod failure;
#[path = "application_live_checkpoint_exit_intent_tests.rs"]
mod intent;

unsafe fn begin(app: &mut Application) {
    app.exit_requested.set(true);
    assert!(!app.close_requested_with_notify(|_| Choice::Discard, |_, text| panic!("{text}")));
    assert!(app.checkpoint_exit_waiting() && app.checkpoint_pending());
    assert!(!app.exit_requested.get());
}
unsafe fn finish(app: &mut Application) -> bool {
    ready(app);
    tick(app);
    assert!(!app.checkpoint_pending() && app.save_job.is_none());
    app.close_requested_with_notify(
        |_| panic!("must not prompt twice"),
        |_, text| panic!("{text}"),
    )
}

#[test]
fn live_discard_without_profile_accepts_video_changes_but_not_model_changes() {
    unsafe {
        let environment = Environment::new();
        let mut app = app(&environment);
        let (hwnd, _, source) = super::super::refresh::stream(&mut app);
        source.publish([0, 0, 255, 255]);
        app.exit_requested.set(true);
        assert!(!app.close_requested_with(|_| {
            SetWindowPos(
                hwnd,
                None,
                -320,
                100,
                120,
                100,
                SWP_NOACTIVATE | SWP_NOZORDER,
            )
            .unwrap();
            crate::desktop::application::exit::Choice::Discard
        }));
        assert!(IsWindow(Some(hwnd)).as_bool() && app.documents[0].is_alive());
        app.exit_requested.set(true);
        assert!(app.close_requested_with(|_| {
            source.publish([0, 255, 0, 255]);
            crate::desktop::application::exit::Choice::Discard
        }));
        assert!(app.documents.is_empty());
        drop(source);
        app.shutdown_live().unwrap();
    }
}

#[test]
fn final_checkpoint_forces_latest_frame_and_preserves_manual_file_until_window_close() {
    unsafe {
        let environment = Environment::new();
        let directory = Directory::new();
        let mut app = app(&environment);
        let (hwnd, id, source) = super::super::refresh::stream(&mut app);
        source.publish([0, 0, 255, 255]);
        let path = directory.0.join("manual.json");
        save_manual(&mut app, path.clone());
        let bytes = std::fs::read(path).unwrap();
        let original = app.restored_session.as_ref().unwrap().original.clone();
        install(&mut app, &directory, false);
        settle_auto(&mut app);
        assert_eq!(revision(&directory.bytes()), 1);
        source.publish([0, 255, 0, 255]);
        begin(&mut app);
        assert!(app.documents[0].is_alive() && IsWindow(Some(hwnd)).as_bool());
        app.exit_requested.set(true);
        assert!(!app.close_requested_with(|_| panic!("duplicate request must not re-prompt")));
        assert!(!app.exit_requested.get());
        assert_eq!(revision(&directory.bytes()), 1);
        ready(&app);
        tick(&mut app);
        assert!(app.documents[0].is_alive() && app.live_sessions[0].is_alive());
        pixels(&directory.bytes(), &id, [0, 255, 0, 255]);
        assert_eq!(revision(&directory.bytes()), 2);
        let record = app.restored_session.as_ref().unwrap();
        assert!(record.original.same_storage(&original));
        assert_eq!(
            std::fs::read(record.saved_path.as_ref().unwrap()).unwrap(),
            bytes
        );
        assert_eq!(app.save_status(), save_state::Status::Changed);
        assert!(app.close_requested_with(|_| panic!("approved final snapshot")));
        app.finish_deleted_workspace().unwrap();
        assert_eq!(
            revision(&directory.bytes()),
            2,
            "exit is not deleting all units"
        );
        drop(source);
        app.shutdown_live().unwrap();
    }
}

#[test]
fn final_worker_and_internal_messages_resume_exit_without_external_input() {
    unsafe {
        let environment = Environment::new();
        let directory = Directory::new();
        let mut app = app(&environment);
        let (_, _, source) = super::super::refresh::stream(&mut app);
        source.publish([0, 255, 0, 255]);
        install(&mut app, &directory, false);
        drain();
        begin(&mut app);
        let end = Instant::now() + Duration::from_secs(5);
        loop {
            let mut message = MSG::default();
            if !PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                assert!(Instant::now() < end, "no internal exit wake");
                std::thread::sleep(Duration::from_millis(1));
                continue;
            }
            assert_ne!(message.message, WM_QUIT);
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
            if app.close_requested_with(|_| panic!("final exit must not re-prompt")) {
                break;
            }
            app.pending_windows(false).unwrap();
            tick(&mut app);
        }
        assert!(app.documents.is_empty() && directory.path().exists());
        drop(source);
        app.shutdown_live().unwrap();
    }
}
