#![cfg(windows)]
//! 子进程显式覆盖 profile 到 linshi；不读取真实 LOCALAPPDATA，不在启动时创建目录。
use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Output},
    time::Duration,
};
#[path = "support/process.rs"]
mod process;
struct Fixture {
    root: PathBuf,
    files: Vec<PathBuf>,
}
impl Fixture {
    fn new() -> Self {
        let root = PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi");
        fs::create_dir_all(&root).unwrap();
        let id = unsafe { windows::Win32::System::Com::CoCreateGuid().unwrap() };
        let root = root.join(format!("otter-profile-startup-{id:?}"));
        fs::create_dir(&root).unwrap();
        let mut fixture = Self {
            root,
            files: Vec::new(),
        };
        fixture.write("workspace.json", br#"{"stickers":[{"id":"one","x":0,"y":0,"w":100,"h":100}],"links":[],"otterNativeCoordinateSpace":{"version":1,"origin":[0,0],"scale":1}}"#);
        fixture
    }
    fn write(&mut self, name: &str, bytes: &[u8]) {
        let path = self.root.join(name);
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap()
            .write_all(bytes)
            .unwrap();
        self.files.push(path);
    }
    fn run(&self, profile: &std::path::Path, explicit: Option<&str>) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_otter-native"));
        command
            .current_dir(&self.root)
            .env("OTTER_NATIVE_PROFILE_DIR", profile)
            .args(["--session", "workspace.json", "--session-check"]);
        if let Some(path) = explicit {
            command.args(["--settings", path]);
        }
        process::run(&mut command, Duration::from_secs(30)).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        for path in &self.files {
            fs::remove_file(path).unwrap();
        }
        fs::remove_dir(&self.root).unwrap();
    }
}
const CONFIG: &[u8] =
    br#"{"schemaVersion":2,"fileNaming":{"stickerSavePattern":"profile_{shortId}"}}"#;
#[test]
fn missing_profile_uses_defaults_without_creating_directories_or_files() {
    let fixture = Fixture::new();
    let root = fixture.root.join("not-created");
    let output = fixture.run(&root, None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout)
        .contains("native-settings-profile: configured=true loaded=false"));
    assert!(!root.exists());
}
#[test]
fn existing_profile_is_loaded_readonly_and_explicit_settings_take_precedence() {
    let mut fixture = Fixture::new();
    fixture.write("app-settings.json", CONFIG);
    let output = fixture.run(&fixture.root, None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout)
        .contains("native-settings-profile: configured=true loaded=true"));
    assert_eq!(
        fs::read(fixture.root.join("app-settings.json")).unwrap(),
        CONFIG
    );
    assert!(!fixture
        .run(&fixture.root.join("app-settings.json"), None)
        .status
        .success());
    fixture.write("explicit.json", CONFIG);
    let output = fixture.run(
        std::path::Path::new("relative-invalid-profile"),
        Some("explicit.json"),
    );
    assert!(output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("native-settings-profile:"));
    assert!(!fixture
        .run(&fixture.root, Some("missing.json"))
        .status
        .success());
}
#[test]
fn invalid_profile_and_ambiguous_recovery_do_not_become_default_settings() {
    let mut fixture = Fixture::new();
    fixture.write("app-settings.json", br#"{"schemaVersion":999}"#);
    assert!(!fixture.run(&fixture.root, None).status.success());
    let mut ambiguous = Fixture::new();
    for index in 0..2 {
        ambiguous.write(
            &format!("app-settings.json.otter-recovery-00000000-0000-0000-0000-{index:012}.json"),
            CONFIG,
        );
    }
    assert!(!ambiguous.run(&ambiguous.root, None).status.success());
    assert!(!ambiguous.root.join("app-settings.json").exists());
}
