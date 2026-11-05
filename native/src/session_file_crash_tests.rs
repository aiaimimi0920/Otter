//! 子进程直接退出跳过 Rust Drop，检验实际句柄回收和四个持久化边界；不模拟断电。
use super::*;
use crate::test_process as process;
use std::{io::Write, process::Command, time::Duration};
const ROOT: &str = "C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-session-crash-tests";
const OLD: &[u8] = br#"{"documentRevision":1,"stickers":[],"links":[],"note":"old"}"#;
const NEW: &[u8] = br#"{"documentRevision":2,"stickers":[],"links":[],"note":"new"}"#;
const PATH_ENV: &str = "OTTER_NATIVE_REPLACE_CRASH_TEST_PATH";
const MODE_ENV: &str = "OTTER_NATIVE_REPLACE_CRASH_TEST_MODE";

struct Fixture {
    directory: PathBuf,
    target: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        std::fs::create_dir_all(ROOT).unwrap();
        let id = unsafe { windows::Win32::System::Com::CoCreateGuid().unwrap() };
        let directory = Path::new(ROOT).join(format!("{id:?}"));
        std::fs::create_dir(&directory).unwrap();
        let target = directory.join("workspace.json");
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .unwrap()
            .write_all(OLD)
            .unwrap();
        Self { directory, target }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // 唯一 linshi 目录；中断遗留的普通暂存/恢复文件由父测试负责清理，不递归。
        assert_eq!(self.directory.parent(), Some(Path::new(ROOT)));
        for entry in std::fs::read_dir(&self.directory).unwrap().flatten() {
            if entry.file_type().unwrap().is_file() {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        let _ = std::fs::remove_dir(&self.directory);
    }
}

#[test]
fn interruption_probe() {
    let Some(path) = std::env::var_os(PATH_ENV) else {
        return;
    };
    let path = PathBuf::from(path);
    assert!(path.is_absolute() && path.starts_with(ROOT));
    assert_eq!(path.file_name().unwrap(), "workspace.json");
    let root = Path::new(ROOT).canonicalize().unwrap();
    let path = path.canonicalize().unwrap();
    assert!(path.starts_with(&root));
    assert_eq!(path.parent().unwrap().parent(), Some(root.as_path()));
    let mode = std::env::var(MODE_ENV).unwrap();
    assert!(["prepared", "moved", "published", "retired"].contains(&mode.as_str()));
    let candidate = Candidate {
        bytes: NEW.to_vec().into(),
        expected_revision: 1,
        revision: 2,
    };
    let cancel = AtomicBool::new(false);
    let replacement = Replacement::prepare(&path, OLD, &candidate, &cancel).unwrap();
    if mode == "prepared" {
        std::process::exit(80);
    }
    let published = replacement
        .commit_observed(
            &cancel,
            |_| {
                if mode == "moved" {
                    std::process::exit(81);
                }
            },
            |_| {
                if mode == "published" {
                    std::process::exit(82);
                }
            },
        )
        .unwrap();
    assert!(published.retained_recovery.is_none());
    std::process::exit(83);
}

#[test]
fn actual_process_exit_preserves_complete_source_or_published_file_at_each_boundary() {
    for (mode, code) in [
        ("prepared", 80),
        ("moved", 81),
        ("published", 82),
        ("retired", 83),
    ] {
        let fixture = Fixture::new();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "session_file_replace::crash_tests::interruption_probe",
                "--nocapture",
            ])
            .env(PATH_ENV, &fixture.target)
            .env(MODE_ENV, mode);
        let output = process::run(&mut command, Duration::from_secs(15)).unwrap();
        assert_eq!(
            output.status.code(),
            Some(code),
            "{mode}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let recovery =
            crate::session_file_recovery::find(&fixture.target, &AtomicBool::new(false)).unwrap();
        let chosen = if mode == "moved" {
            assert!(!fixture.target.exists());
            recovery.expect("interrupted move must expose one recovery candidate")
        } else {
            assert!(
                recovery.is_none(),
                "existing target must win over old recovery copy"
            );
            fixture.target.clone()
        };
        let bytes = std::fs::read(&chosen).unwrap();
        let expected = if mode == "prepared" || mode == "moved" {
            OLD
        } else {
            NEW
        };
        assert_eq!(bytes, expected, "{mode}");
        let document = crate::session_document::decode(&bytes).unwrap();
        assert_eq!(document.revision, if expected == OLD { 1 } else { 2 });
        let files: Vec<_> = std::fs::read_dir(&fixture.directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        let recovery_files: Vec<_> = files
            .iter()
            .filter(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .contains(".otter-recovery-")
            })
            .collect();
        assert_eq!(
            recovery_files.len(),
            usize::from(mode == "moved" || mode == "published")
        );
        for path in recovery_files {
            assert_eq!(std::fs::read(path).unwrap(), OLD);
        }
        let temporary: Vec<_> = files
            .iter()
            .filter(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(".otter-save-")
            })
            .collect();
        assert_eq!(
            temporary.len(),
            usize::from(mode == "prepared" || mode == "moved")
        );
        for path in temporary {
            assert_eq!(std::fs::read(path).unwrap(), NEW);
        }
        // 子进程句柄确实由系统回收，非仅内存模型改了标记。
        OpenOptions::new().write(true).open(&chosen).unwrap();
        if mode == "retired" {
            assert_eq!(std::fs::read_dir(&fixture.directory).unwrap().count(), 1);
        }
    }
}
