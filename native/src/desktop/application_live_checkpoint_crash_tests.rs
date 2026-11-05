//! 真子进程绕过所有 Drop；仅最近成功 Live 时间点恢复为静态，租约由 OS 释放。
use super::*;
use crate::test_process;
use std::process::Command;
const ROOT: &str = "C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-live-checkpoint-tests";
const PATH_ENV: &str = "OTTER_NATIVE_LIVE_CHECKPOINT_CRASH_ROOT";
const MODE_ENV: &str = "OTTER_NATIVE_LIVE_CHECKPOINT_CRASH_MODE";
#[test]
fn interruption_probe() {
    let Some(root) = std::env::var_os(PATH_ENV) else {
        return;
    };
    let root = PathBuf::from(root);
    assert_eq!(root.parent(), Some(std::path::Path::new(ROOT)));
    assert_eq!(
        root.canonicalize().unwrap().parent(),
        Some(std::path::Path::new(ROOT).canonicalize().unwrap().as_path())
    );
    let mode = std::env::var(MODE_ENV).unwrap();
    assert!(mode == "staged" || mode == "committed");
    unsafe {
        let environment = Environment::new();
        let mut app = app(&environment);
        let directory = Directory(root);
        install(&mut app, &directory, true);
        let (_, id) = live(&mut app, true);
        std::fs::write(directory.0.join("new-live-id.txt"), id.as_str()).unwrap();
        tick(&mut app);
        ready(&app);
        if mode == "committed" {
            settle_auto(&mut app);
        }
        std::process::exit(if mode == "staged" { 83 } else { 84 });
    }
}
#[test]
fn abrupt_exit_restores_only_successfully_committed_live_snapshot_not_capture_permissions() {
    unsafe {
        let environment = Environment::new();
        for (mode, code) in [("staged", 83), ("committed", 84)] {
            let directory = Directory::new();
            let mut app = app(&environment);
            let (_, id) = live(&mut app, true);
            install(&mut app, &directory, false);
            settle_auto(&mut app);
            let before = directory.bytes();
            app.shutdown_live().unwrap();
            drop(app);
            let mut command = Command::new(std::env::current_exe().unwrap());
            command.args(["--exact", "desktop::application::live::freeze::tests::checkpoint::crash::interruption_probe", "--nocapture"])
                .env(PATH_ENV, &directory.0).env(MODE_ENV, mode);
            let output = test_process::run(&mut command, Duration::from_secs(15)).unwrap();
            assert_eq!(
                output.status.code(),
                Some(code),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let after = directory.bytes();
            if mode == "staged" {
                assert_eq!(after, before);
            } else {
                assert_eq!(revision(&after), revision(&before) + 1);
            }
            pixels(&after, &id, [0, 0, 255, 255]);
            let new_id = crate::workspace_id::UnitId::parse(
                &std::fs::read_to_string(directory.0.join("new-live-id.txt")).unwrap(),
            )
            .unwrap();
            assert_eq!(
                crate::session_document::decode(&after)
                    .unwrap()
                    .workspace
                    .contains(&new_id),
                mode == "committed"
            );
            let mut restored = self::super::app(&environment);
            install(&mut restored, &directory, true);
            assert!(restored.live_sessions.is_empty());
            assert!(restored
                .documents
                .iter()
                .all(|doc| !doc.state.global_capture.get()));
            assert!(restored
                .restored_session
                .as_ref()
                .unwrap()
                .saved_path
                .is_none());
            drop(restored);
        }
    }
}
