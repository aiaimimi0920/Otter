#![cfg(windows)]
//! Default discovery/recovery barrier through the real EXE; read-only checks require explicit isolation and never read the real user profile.
use std::{path::PathBuf, process::Command, time::Duration};
#[path = "../support/process.rs"]
mod process;
#[path = "workspace_restart_checkpoint.rs"]
mod restart_checkpoint;
fn root_dir() -> PathBuf {
    crate::test_temp_root().join("otter-workspace-startup-tests")
}
const SESSION: &[u8] = br#"{"documentSchemaVersion":1,"documentRevision":7,"otterNativeCoordinateSpace":{"version":1,"origin":[0,0],"scale":1},"stickers":[],"links":[],"groups":[],"extension":{"keep":true}}"#;
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let base = root_dir();
        std::fs::create_dir_all(&base).unwrap();
        let path = base.join(format!("{:?}", unsafe {
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
        assert_eq!(self.0.parent(), Some(root_dir().as_path()));
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
fn login_startup_readback_does_not_acquire_profile_or_read_settings() {
    let fixture = Fixture::new();
    let missing = fixture.0.join("not-a-profile");
    let result = process::run(
        Command::new(env!("CARGO_BIN_EXE_otter-native"))
            .arg("--check-login-startup")
            .env("OTTER_NATIVE_PROFILE_DIR", &missing),
        Duration::from_secs(15),
    )
    .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let stdout = String::from_utf8(result.stdout).unwrap();
    assert!(stdout.starts_with("native-login-startup: ordinary="));
    assert_eq!(stdout.split_whitespace().count(), 3);
    assert!(stdout.trim_end().ends_with(" read-only=true"));
    assert!(!missing.exists());
}
#[test]
#[ignore = "Run from a manually elevated Windows shell; this test never requests UAC"]
fn manually_elevated_desktop_restores_workspace_without_changing_login() {
    assert!(otter_media::elevation_status::elevated().unwrap());
    assert!(!otter_media::elevation_status::allows_desktop_integrations());
    let fixture = Fixture::new();
    let read_login = || {
        let result = process::run(
            Command::new(env!("CARGO_BIN_EXE_otter-native"))
                .arg("--check-login-startup")
                .env("OTTER_NATIVE_PROFILE_DIR", &fixture.0),
            Duration::from_secs(15),
        )
        .unwrap();
        assert!(result.status.success());
        result.stdout
    };
    let before = read_login();
    std::fs::write(fixture.path(), SESSION).unwrap();
    let result = fixture.run();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        String::from_utf8_lossy(&result.stdout).contains("native-workspace-startup: loaded=true")
    );
    assert_eq!(std::fs::read(fixture.path()).unwrap(), SESSION);
    assert_eq!(read_login(), before);
}
#[test]
fn occupied_workspace_is_rejected_before_reading_stale_or_corrupt_policy() {
    use std::os::windows::fs::OpenOptionsExt;
    let fixture = Fixture::new();
    let settings = fixture.0.join("app-settings.json");
    std::fs::write(&settings, b"unfinished policy").unwrap();
    let lease = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .share_mode(0)
        .open(fixture.0.join("workspace.lock"))
        .unwrap();
    let denied = fixture.run();
    let stderr = String::from_utf8_lossy(&denied.stderr);
    assert!(!denied.status.success());
    assert!(stderr.contains("Cannot open workspace profile"), "{stderr}");
    assert!(!String::from_utf8_lossy(&denied.stdout).contains("native-settings-profile"));
    drop(lease);
    let invalid = fixture.run();
    assert!(!invalid.status.success());
    let stderr = String::from_utf8_lossy(&invalid.stderr);
    assert!(stderr.contains("Invalid settings"), "{stderr}");
    assert_eq!(std::fs::read(&settings).unwrap(), b"unfinished policy");
    assert!(!fixture.path().exists());
}
#[test]
fn session_named_like_workspace_check_uses_the_session_mode() {
    let fixture = Fixture::new();
    let name = "--workspace-check";
    let path = fixture.0.join(name);
    std::fs::write(&path, SESSION).unwrap();
    let output = process::run(
        Command::new(env!("CARGO_BIN_EXE_otter-native"))
            .current_dir(&fixture.0)
            .args(["--session", name, "--session-check"])
            .env("OTTER_NATIVE_PROFILE_DIR", &fixture.0),
        Duration::from_secs(30),
    )
    .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("native-session-check:"), "{stdout}");
    assert_eq!(std::fs::read(path).unwrap(), SESSION);
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
    let stdout = String::from_utf8_lossy(&absent.stdout);
    assert!(stdout.contains("native-session-check: windows=1"));
    assert!(stdout.contains("hidden=true"));
    assert!(stdout.contains("native-session-render-check: paints=2 device-recreated=true"));
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
    assert!(stdout.contains("native-session-render-check: paints=2 device-recreated=true"));
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
