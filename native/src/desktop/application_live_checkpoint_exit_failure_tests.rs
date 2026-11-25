//! Exiting without a first frame, with stale state, or with conflicts must preserve windows; old baselines or read-only owners are not successful final commits.
use super::*;
#[test]
fn active_execution_can_be_discarded_without_profile_but_blocks_final_profile_snapshot() {
    unsafe {
        let environment = Environment::new();
        for writable_profile in [false, true] {
            let directory = Directory::new();
            let mut app = app(&environment);
            let (_, _) = live(&mut app, true);
            let unit = app.documents[0].state.unit_id.clone();
            let request = app
                .executions
                .begin(
                    unit.clone(),
                    crate::workspace_id::UnitId::parse("exit-busy-request").unwrap(),
                )
                .unwrap();
            app.executions
                .mark_preview(&unit, &request, "exit-preview".into())
                .unwrap();
            if writable_profile {
                install(&mut app, &directory, false);
            }
            let mut warnings = 0;
            app.exit_requested.set(true);
            let closed = app.close_requested_with_notify(
                |_| Choice::Discard,
                |_, message| {
                    warnings += 1;
                    assert!(message.contains("检查点"));
                },
            );
            assert_eq!(closed, !writable_profile);
            assert!(!app.checkpoint_exit_waiting() && !app.checkpoint_pending());
            assert!(!directory.path().exists() && app.save_job.is_none());
            if writable_profile {
                assert_eq!(warnings, 1);
                assert!(app.documents[0].is_alive());
                assert_eq!(app.executions.active(&unit), Some(&request));
                assert!(!app.close_requested_with(|_| panic!("no latent busy exit")));
            } else {
                assert_eq!(warnings, 0);
                assert!(app.documents.is_empty());
                assert!(app.executions.active(&unit).is_none());
                assert_eq!(app.executions.preview_bytes(), 0);
            }
            app.shutdown_live().unwrap();
        }
    }
}

#[test]
fn late_execution_cancels_final_exit_before_commit_and_before_window_destruction() {
    unsafe {
        let environment = Environment::new();
        for after_commit in [false, true] {
            let directory = Directory::new();
            let mut app = app(&environment);
            let (_, _) = live(&mut app, true);
            install(&mut app, &directory, false);
            begin(&mut app);
            ready(&app);
            if after_commit {
                tick(&mut app);
                assert!(directory.path().exists());
            }
            let unit = app.documents[0].state.unit_id.clone();
            let request = app
                .executions
                .begin(
                    unit.clone(),
                    crate::workspace_id::UnitId::parse("late-exit-busy").unwrap(),
                )
                .unwrap();
            if !after_commit {
                tick(&mut app);
                assert!(!directory.path().exists());
            }
            assert!(!app.close_requested_with(|_| panic!("late busy must cancel final exit")));
            assert!(!app.checkpoint_exit_waiting() && app.documents[0].is_alive());
            assert_eq!(app.executions.active(&unit), Some(&request));
            assert!(!app.close_requested_with(|_| panic!("no latent late-busy exit")));
            app.shutdown_live().unwrap();
        }
    }
}

#[test]
fn no_first_frame_preserves_windows_reports_once_and_does_not_leave_exit_waiting() {
    unsafe {
        let environment = Environment::new();
        let directory = Directory::new();
        let mut app = app(&environment);
        let (hwnd, _, source) = super::super::super::refresh::stream(&mut app);
        install(&mut app, &directory, false);
        let mut warnings = 0;
        app.exit_requested.set(true);
        assert!(!app.close_requested_with_notify(
            |_| Choice::Discard,
            |_, message| {
                warnings += 1;
                assert!(message.contains("Live"));
            }
        ));
        assert_eq!(warnings, 1);
        assert!(
            !app.exit_requested.get() && !app.checkpoint_exit_waiting() && app.save_job.is_none()
        );
        assert!(app.documents[0].is_alive() && IsWindow(Some(hwnd)).as_bool());
        source.publish([0, 255, 0, 255]);
        settle_auto(&mut app);
        assert!(directory.path().exists());
        assert!(!app.close_requested_with(|_| panic!("no deferred exit")));
        drop(source);
        app.shutdown_live().unwrap();
    }
}

#[test]
fn stale_final_model_keeps_old_checkpoint_and_never_closes_on_later_autosave() {
    unsafe {
        let environment = Environment::new();
        let directory = Directory::new();
        let mut app = app(&environment);
        let (hwnd, _) = live(&mut app, true);
        install(&mut app, &directory, false);
        settle_auto(&mut app);
        let before = directory.bytes();
        begin(&mut app);
        SetWindowPos(
            hwnd,
            None,
            -380,
            140,
            120,
            100,
            SWP_NOACTIVATE | SWP_NOZORDER,
        )
        .unwrap();
        assert!(!finish(&mut app));
        assert_eq!(directory.bytes(), before);
        assert!(app.documents[0].is_alive() && app.live_sessions[0].is_alive());
        settle_auto(&mut app);
        assert_ne!(directory.bytes(), before);
        assert!(!app.close_requested_with(|_| panic!("ordinary save cannot revive exit")));
        app.shutdown_live().unwrap();
    }
}

#[test]
fn external_conflict_stops_once_preserves_external_file_and_explicit_discard_can_exit_readonly() {
    unsafe {
        let environment = Environment::new();
        let directory = Directory::new();
        let mut app = app(&environment);
        let (_, _) = live(&mut app, true);
        install(&mut app, &directory, false);
        settle_auto(&mut app);
        let external = b"external-conflict-must-not-be-overwritten";
        std::fs::write(directory.path(), external).unwrap();
        begin(&mut app); // Make the change before the worker acquires the original transaction's shared lock; do not bypass the OS lock held during saving.
        ready(&app);
        let mut warnings = 0;
        for _ in 0..3 {
            app.pending_workspace_save_with(|_, _, message, warning| {
                assert!(warning && message.contains("检查点"));
                warnings += 1;
            });
        }
        assert_eq!(warnings, 1);
        assert!(!app.checkpoint_exit_waiting() && !app.checkpoint_pending());
        assert_eq!(directory.bytes(), external);
        assert!(app.documents[0].is_alive());
        assert!(!app.close_requested_with(|_| panic!("failed exit is consumed")));
        app.exit_requested.set(true);
        assert!(app.close_requested_with(|_| Choice::Discard));
        assert_eq!(directory.bytes(), external);
        app.shutdown_live().unwrap();
    }
}

#[test]
fn stopped_profile_live_manual_snapshot_allows_exit_only_with_unchanged_layout() {
    unsafe {
        let environment = Environment::new();
        let directory = Directory::new();
        let mut app = app(&environment);
        let (hwnd, _, source) = super::super::super::refresh::stream(&mut app);
        source.publish([0, 0, 255, 255]);
        install(&mut app, &directory, false);
        settle_auto(&mut app);
        let before = b"external-conflict-must-not-be-overwritten";
        std::fs::write(directory.path(), before).unwrap();
        begin(&mut app);
        ready(&app);
        app.pending_workspace_save_with(|_, _, _, warning| assert!(warning));
        assert!(app.checkpoint_writes_stopped());
        save_manual(&mut app, directory.0.join("rescue-before.json"));
        assert_ne!(app.save_status(), save_state::Status::Unchanged);
        SetWindowPos(
            hwnd,
            None,
            -380,
            140,
            120,
            100,
            SWP_NOACTIVATE | SWP_NOZORDER,
        )
        .unwrap();
        app.exit_requested.set(true);
        let mut notices = 0;
        assert!(!app.close_requested_automatically_with_notify(|_, _| notices += 1));
        assert_eq!(notices, 1);
        assert!(!app.exit_requested.get());
        assert!(app.documents[0].is_alive() && IsWindow(Some(hwnd)).as_bool());
        save_manual(&mut app, directory.0.join("rescue-after.json"));
        assert_ne!(app.save_status(), save_state::Status::Unchanged);
        app.exit_requested.set(true);
        assert!(app.close_requested_automatically_with_notify(|_, text| panic!("{text}")));
        assert!(app.documents.is_empty());
        assert_eq!(directory.bytes(), before);
        drop(source);
        app.shutdown_live().unwrap();
    }
}
