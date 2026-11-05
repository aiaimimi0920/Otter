#![cfg(windows)]
//! 真实 EXE 读取合成中断现场；不终止用户程序，不把此测试当作断电实验。
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
    files: Vec<(PathBuf, Vec<u8>)>,
    target_exists: bool,
}
impl Fixture {
    fn new(target: Option<&[u8]>, recoveries: &[&[u8]]) -> Self {
        let root = PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi");
        fs::create_dir_all(&root).unwrap();
        let id = unsafe { windows::Win32::System::Com::CoCreateGuid().unwrap() };
        let root = root.join(format!("otter-settings-recovery-{id:?}"));
        fs::create_dir(&root).unwrap();
        let mut fixture = Self {
            root,
            files: Vec::new(),
            target_exists: target.is_some(),
        };
        fixture.write("workspace.json", br#"{"stickers":[{"id":"one","x":0,"y":0,"w":100,"h":100}],"links":[],"otterNativeCoordinateSpace":{"version":1,"origin":[0,0],"scale":1}}"#);
        if let Some(bytes) = target {
            fixture.write("config.json", bytes);
        }
        for (index, bytes) in recoveries.iter().enumerate() {
            fixture.write(
                &format!("config.json.otter-recovery-00000000-0000-0000-0000-{index:012}.json"),
                bytes,
            );
        }
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
        self.files.push((path, bytes.to_vec()));
    }
    fn run(&self) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_otter-native"));
        command.current_dir(&self.root).args([
            "--settings",
            "config.json",
            "--session",
            "workspace.json",
            "--session-check",
        ]);
        let output = process::run(&mut command, Duration::from_secs(30)).unwrap();
        assert_eq!(self.root.join("config.json").exists(), self.target_exists);
        for (path, bytes) in &self.files {
            assert_eq!(fs::read(path).unwrap(), *bytes);
            let moved = self.root.join("probe.tmp");
            fs::rename(path, &moved).unwrap();
            fs::rename(&moved, path).unwrap();
        }
        output
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        for (path, _) in &self.files {
            fs::remove_file(path).unwrap();
        }
        fs::remove_dir(&self.root).unwrap();
    }
}
const VALID: &[u8] =
    br#"{"schemaVersion":2,"fileNaming":{"stickerSavePattern":"recovered_{shortId}"}}"#;

#[test]
fn unique_recovery_is_loaded_without_restoring_or_modifying_any_file() {
    let fixture = Fixture::new(None, &[VALID]);
    let output = fixture.run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains("native-settings-recovery: recovered=true update-blocked=true readonly=true")
    );
    assert!(text.contains("native-settings-restore: applied=true shared=true"));
}
#[test]
fn existing_original_takes_precedence_even_when_invalid() {
    for original in [VALID, br#"{"schemaVersion":999}"#] {
        let fixture = Fixture::new(Some(original), &[VALID]);
        let output = fixture.run();
        assert_eq!(output.status.success(), original == VALID);
        assert!(!String::from_utf8_lossy(&output.stdout).contains("native-settings-recovery:"));
    }
}
#[test]
fn ambiguous_invalid_future_and_oversized_recovery_files_are_rejected() {
    let oversized = vec![b' '; 1024 * 1024 + 1];
    for recoveries in [
        vec![VALID, VALID],
        vec![b"{".as_slice()],
        vec![br#"{"schemaVersion":999}"#.as_slice()],
        vec![oversized.as_slice()],
    ] {
        let fixture = Fixture::new(None, &recoveries);
        let output = fixture.run();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
}
