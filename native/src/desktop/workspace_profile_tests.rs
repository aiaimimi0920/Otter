//! 只用独占 linshi 目录；租约在跨进程失败和异常退出后仍保持安全。
use super::*;
use crate::test_process as process;
use std::{process::Command, time::Duration};
pub(super) const ROOT: &str =
    "C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-workspace-profile-tests";
pub(super) struct Directory(pub PathBuf);
impl Directory {
    pub(super) fn new() -> Self {
        std::fs::create_dir_all(ROOT).unwrap();
        let path = Path::new(ROOT).join(format!("{:?}", unsafe {
            windows::Win32::System::Com::CoCreateGuid().unwrap()
        }));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        assert_eq!(self.0.parent(), Some(Path::new(ROOT)));
        // 只删除本次 GUID 目录的普通文件；不跟随重解析点、不递归清理。
        for entry in std::fs::read_dir(&self.0).unwrap().flatten() {
            if entry.file_type().unwrap().is_file() {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        let _ = std::fs::remove_dir(&self.0);
    }
}
#[test]
fn lease_probe() {
    let Some(root) = std::env::var_os("OTTER_NATIVE_WORKSPACE_LEASE_TEST") else {
        return;
    };
    let root = PathBuf::from(root);
    assert!(root.starts_with(ROOT) && root.parent() == Some(Path::new(ROOT)));
    let result = Profile::open(&root);
    if std::env::var_os("OTTER_NATIVE_WORKSPACE_LEASE_EXIT").is_some() {
        let _profile = result.unwrap();
        std::process::exit(81); // 跳过 Drop，验证 OS 回收真实句柄。
    }
    assert!(
        result.is_err(),
        "second process must not share a workspace lease"
    );
}
#[test]
fn lease_is_exclusive_across_processes_and_released_after_abrupt_exit() {
    let directory = Directory::new();
    let profile = Profile::open(&directory.0).unwrap();
    assert!(!profile.restore);
    assert!(Profile::open(&directory.0).is_err());
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "desktop::workspace_profile::tests::lease_probe",
            "--nocapture",
        ])
        .env("OTTER_NATIVE_WORKSPACE_LEASE_TEST", &directory.0);
    let output = process::run(&mut command, Duration::from_secs(15)).unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(std::fs::rename(&directory.0, directory.0.with_extension("moved")).is_err());
    assert!(std::fs::rename(
        directory.0.join("workspace.lock"),
        directory.0.join("other.lock")
    )
    .is_err());
    drop(profile);
    command.env("OTTER_NATIVE_WORKSPACE_LEASE_EXIT", "1");
    assert_eq!(
        process::run(&mut command, Duration::from_secs(15))
            .unwrap()
            .status
            .code(),
        Some(81)
    );
    assert!(Profile::open(&directory.0).is_ok());
}
#[test]
fn file_roots_and_parent_components_are_rejected_and_unique_recovery_is_found() {
    let directory = Directory::new();
    let sentinel = directory.0.join("sentinel");
    std::fs::write(&sentinel, b"keep").unwrap();
    assert!(Profile::open(&sentinel).is_err());
    assert!(Profile::open(&directory.0.join("..")).is_err());
    let recovery = directory
        .0
        .join("workspace.json.otter-recovery-00000000-0000-0000-0000-000000000001.json");
    std::fs::write(&recovery, b"keep").unwrap();
    let profile = Profile::open(&directory.0).unwrap();
    assert!(profile.restore);
    assert_eq!(std::fs::read(recovery).unwrap(), b"keep");
}
