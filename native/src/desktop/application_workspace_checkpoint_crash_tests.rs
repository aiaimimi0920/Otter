//! 中断完整 Application 管线而非模拟 Drop；最近成功检查点可恢复，未发布候选不替代它。
use super::*;
use crate::test_process as process;
use std::process::Command;
const PATH_ENV: &str = "OTTER_NATIVE_CHECKPOINT_CRASH_TEST_ROOT";
const MODE_ENV: &str = "OTTER_NATIVE_CHECKPOINT_CRASH_TEST_MODE";
const ROOT: &str = "C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-workspace-checkpoint-tests";
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
        run(|instance| {
            let mut app = new_app(instance);
            app.install_workspace_checkpoint(Some(Profile::open(&root).unwrap()), true)
                .unwrap();
            app.documents[0].state.canvas.borrow_mut().normal_opacity = 0.7;
            if mode == "staged" {
                app.start_checkpoint().unwrap();
                ready(&app);
            } else {
                settle(&mut app);
            }
            std::process::exit(if mode == "staged" { 81 } else { 82 });
        });
    }
}
#[test]
fn abrupt_process_exit_restores_only_last_committed_snapshot_and_releases_lease() {
    unsafe {
        run(|instance| {
            for (mode, code) in [("staged", 81), ("committed", 82)] {
                let directory = Directory::new();
                let mut app = new_app(instance);
                app.install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                    .unwrap();
                settle(&mut app);
                let before = std::fs::read(directory.path()).unwrap();
                drop(app);
                let mut command = Command::new(std::env::current_exe().unwrap());
                command.args(["--exact", "desktop::application::workspace_checkpoint::tests::crash::interruption_probe", "--nocapture"])
                .env(PATH_ENV, &directory.0).env(MODE_ENV, mode);
                let output = process::run(&mut command, Duration::from_secs(15)).unwrap();
                assert_eq!(
                    output.status.code(),
                    Some(code),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                let after = std::fs::read(directory.path()).unwrap();
                if mode == "staged" {
                    assert_eq!(after, before);
                } else {
                    assert_ne!(after, before);
                }
                let mut restored = new_app(instance);
                restored
                    .install_workspace_checkpoint(Some(Profile::open(&directory.0).unwrap()), true)
                    .unwrap();
                assert_eq!(
                    restored.documents[0].state.canvas.borrow().normal_opacity,
                    if mode == "staged" { 1.0 } else { 0.7 }
                );
                assert!(restored.live_sessions.is_empty());
                assert!(restored
                    .restored_session
                    .as_ref()
                    .unwrap()
                    .saved_path
                    .is_none());
                drop(restored);
            }
        });
    }
}
