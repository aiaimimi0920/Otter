#![cfg(windows)]
//! 真实 EXE 显式配置启动；只用 linshi 合成文档，不读取用户设置或执行真实截图。
use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Output},
    time::Duration,
};
#[path = "support/process.rs"]
mod process;
struct Fixture(PathBuf);
impl Fixture {
    fn new(settings: &[u8]) -> Self {
        let root = PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi");
        fs::create_dir_all(&root).unwrap();
        let id = unsafe { windows::Win32::System::Com::CoCreateGuid().unwrap() };
        let path = root.join(format!("otter-settings-startup-{id:?}"));
        fs::create_dir(&path).unwrap();
        let fixture = Self(path);
        fixture.write("命名设置.json", settings);
        fixture.write("workspace.json", br#"{"stickers":[{"id":"one","x":10,"y":20,"w":200,"h":100}],"links":[],"otterNativeCoordinateSpace":{"version":1,"origin":[0,0],"scale":1}}"#);
        fixture
    }
    fn write(&self, name: &str, bytes: &[u8]) {
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.0.join(name))
            .unwrap()
            .write_all(bytes)
            .unwrap();
    }
    fn run(&self, args: &[&str]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_otter-native"));
        command.current_dir(&self.0).args(args);
        process::run(&mut command, Duration::from_secs(30)).unwrap()
    }
    fn unchanged_and_unlocked(&self, expected: &[u8]) {
        let source = self.0.join("命名设置.json");
        assert_eq!(fs::read(&source).unwrap(), expected);
        let moved = self.0.join("moved.json");
        fs::rename(&source, &moved).unwrap();
        fs::rename(&moved, &source).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        for name in ["命名设置.json", "workspace.json"] {
            fs::remove_file(self.0.join(name)).unwrap();
        }
        fs::remove_dir(&self.0).unwrap();
    }
}
const CONFIG: &[u8] = br#"{"schemaVersion":2,"fileNaming":{"stickerSavePattern":"custom_{shortId}_{width}"},"cache":{"tempCacheMaxBytes":33554432,"tempCacheRetentionDays":0},"unknown":{"keep":true}}"#;
fn success(output: &Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn settings_load_before_hidden_host_and_do_not_modify_source() {
    let fixture = Fixture::new(CONFIG);
    let text = success(&fixture.run(&["--settings", "命名设置.json", "--smoke-test"]));
    assert!(
        text.contains("native-settings-startup: explicit=true applied=true source-readonly=true")
    );
    assert!(text.contains("native-smoke:"));
    fixture.unchanged_and_unlocked(CONFIG);
}

#[test]
fn settings_survive_real_workspace_restore_with_relative_unicode_path() {
    let fixture = Fixture::new(CONFIG);
    for args in [
        vec![
            "--settings",
            "命名设置.json",
            "--session",
            "workspace.json",
            "--session-check",
        ],
        vec![
            "--session",
            "workspace.json",
            "--session-check",
            "--settings",
            "命名设置.json",
        ],
    ] {
        let text = success(&fixture.run(&args));
        assert!(
            text.contains("native-settings-restore: applied=true shared=true source-readonly=true")
        );
        assert!(text.contains("native-session-check: windows=1"));
        fixture.unchanged_and_unlocked(CONFIG);
    }
}

#[test]
fn invalid_settings_fail_before_window_or_capture_side_effects() {
    for bytes in [
        br#"{"schemaVersion":3}"#.to_vec(),
        br#"{"fileNaming":{"stickerSavePattern":"{bad}"}}"#.to_vec(),
        br#"{"cache":{"tempCacheRetentionDays":3651}}"#.to_vec(),
        br#"{"cache":{"recycleBinMaxEntries":501}}"#.to_vec(),
        vec![b' '; 1024 * 1024 + 1],
    ] {
        let fixture = Fixture::new(&bytes);
        for mode in ["--smoke-test", "--capture", "--capture-region"] {
            let output = fixture.run(&["--settings", "命名设置.json", mode]);
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
            assert!(String::from_utf8_lossy(&output.stderr).contains("settings"));
            fixture.unchanged_and_unlocked(&bytes);
        }
    }
}

#[test]
fn missing_file_and_duplicate_cli_options_are_not_silently_ignored() {
    let fixture = Fixture::new(CONFIG);
    for args in [
        vec!["--settings", "missing.json", "--smoke-test"],
        vec![
            "--settings",
            "命名设置.json",
            "--settings",
            "命名设置.json",
            "--smoke-test",
        ],
        vec!["--settings", "--smoke-test"],
    ] {
        let output = fixture.run(&args);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
    fixture.unchanged_and_unlocked(CONFIG);
}
