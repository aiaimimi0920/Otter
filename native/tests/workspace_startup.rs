#![cfg(windows)]
//! 真实 EXE 默认发现/恢复屏障；只读检查必须显式隔离，不读取真实用户 profile。
use std::{path::PathBuf, process::Command, time::Duration};
#[path = "support/process.rs"]
mod process;
const ROOT: &str = "C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-workspace-startup-tests";
const SESSION: &[u8] = br#"{"documentSchemaVersion":1,"documentRevision":7,"otterNativeCoordinateSpace":{"version":1,"origin":[0,0],"scale":1},"stickers":[],"links":[],"groups":[],"extension":{"keep":true}}"#;
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        std::fs::create_dir_all(ROOT).unwrap();
        let path = PathBuf::from(ROOT).join(format!("{:?}", unsafe {
            windows::Win32::System::Com::CoCreateGuid().unwrap()
        }));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self) -> PathBuf {
        self.0.join("workspace.json")
    }
    fn run(&self) -> std::process::Output {
        process::run(
            Command::new(env!("CARGO_BIN_EXE_otter-native"))
                .arg("--workspace-check")
                .env("OTTER_NATIVE_PROFILE_DIR", &self.0),
            Duration::from_secs(30),
        )
        .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        assert_eq!(self.0.parent(), Some(std::path::Path::new(ROOT)));
        for entry in std::fs::read_dir(&self.0).unwrap().flatten() {
            if entry.file_type().unwrap().is_file() {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        let _ = std::fs::remove_dir(&self.0);
    }
}
#[test]
fn check_never_discovers_real_profile_or_combines_with_other_inputs() {
    for args in [
        vec!["--workspace-check"],
        vec!["--workspace-check", "--capture"],
        vec!["--workspace-check", "--session", "missing.json"],
    ] {
        let output = process::run(
            Command::new(env!("CARGO_BIN_EXE_otter-native")).args(args),
            Duration::from_secs(15),
        )
        .unwrap();
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("requires OTTER_NATIVE_PROFILE_DIR")
                || stderr.contains("cannot be combined"),
            "{stderr}"
        );
        assert!(!String::from_utf8_lossy(&output.stdout).contains("native-session-check:"));
    }
}
#[test]
fn missing_and_valid_default_workspace_are_read_without_writing_or_manual_target() {
    let fixture = Fixture::new();
    let absent = fixture.run();
    assert!(
        absent.status.success(),
        "{}",
        String::from_utf8_lossy(&absent.stderr)
    );
    assert!(String::from_utf8_lossy(&absent.stdout)
        .contains("loaded=false readonly=false manual-target=false"));
    assert!(!fixture.path().exists());
    std::fs::write(fixture.path(), SESSION).unwrap();
    let loaded = fixture.run();
    assert!(
        loaded.status.success(),
        "{}",
        String::from_utf8_lossy(&loaded.stderr)
    );
    let stdout = String::from_utf8_lossy(&loaded.stdout);
    assert!(
        stdout.contains("loaded=true readonly=false manual-target=false"),
        "{stdout}"
    );
    assert!(stdout.contains("native-session-save-state: Unsaved"));
    assert!(stdout.contains("hidden=true"));
    assert_eq!(std::fs::read(fixture.path()).unwrap(), SESSION);
}
#[test]
fn malformed_unknown_schema_and_missing_coordinates_fail_closed_without_overwrite() {
    for bytes in [
        b"broken".as_slice(),
        br#"{"documentSchemaVersion":2,"stickers":[],"links":[]}"#,
        br#"{"stickers":[],"links":[]}"#,
    ] {
        let fixture = Fixture::new();
        std::fs::write(fixture.path(), bytes).unwrap();
        let output = fixture.run();
        assert!(
            !output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert_eq!(std::fs::read(fixture.path()).unwrap(), bytes);
    }
}
#[test]
fn only_unique_recovery_is_read_only_and_multiple_recoveries_are_not_guessed() {
    let fixture = Fixture::new();
    let first = fixture
        .0
        .join("workspace.json.otter-recovery-00000000-0000-0000-0000-000000000001.json");
    std::fs::write(&first, SESSION).unwrap();
    let output = fixture.run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout)
        .contains("loaded=true readonly=true manual-target=false"));
    assert!(!fixture.path().exists());
    assert_eq!(std::fs::read(&first).unwrap(), SESSION);
    std::fs::write(
        fixture
            .0
            .join("workspace.json.otter-recovery-00000000-0000-0000-0000-000000000002.json"),
        SESSION,
    )
    .unwrap();
    assert!(!fixture.run().status.success());
    assert!(!fixture.path().exists());
}
#[test]
fn other_process_cannot_start_a_second_default_workspace_writer() {
    use std::os::windows::fs::OpenOptionsExt;
    let fixture = Fixture::new();
    std::fs::write(fixture.path(), SESSION).unwrap();
    let _lease = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .share_mode(0)
        .open(fixture.0.join("workspace.lock"))
        .unwrap();
    let output = fixture.run();
    assert!(!output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("native-session-check:"));
    assert_eq!(std::fs::read(fixture.path()).unwrap(), SESSION);
}
