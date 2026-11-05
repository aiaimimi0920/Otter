use super::*;
use std::{io::Write, path::PathBuf};

struct Fixture {
    directory: PathBuf,
    path: PathBuf,
}
impl Fixture {
    fn new(bytes: &[u8]) -> Self {
        let root =
            Path::new("C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-session-guard-tests");
        std::fs::create_dir_all(root).unwrap();
        let directory = root.join(format!("{:?}", unsafe {
            windows::Win32::System::Com::CoCreateGuid().unwrap()
        }));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("baseline.json");
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap()
            .write_all(bytes)
            .unwrap();
        Self { directory, path }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // 只清理本测试在 linshi 创建的唯一文件和空目录。
        let _ = std::fs::remove_file(&self.path);
        let _ = std::fs::remove_dir(&self.directory);
    }
}
const SOURCE: &[u8] = br#"{"documentRevision":1,"stickers":[],"links":[],"note":"old"}"#;

#[test]
fn movable_keeps_identity_and_write_exclusion_through_move_and_restore() {
    let fixture = Fixture::new(SOURCE);
    let guard = Movable::open(&fixture.path, SOURCE, &AtomicBool::new(false)).unwrap();
    let identity = guard.identity();
    let recovery = fixture.directory.join("recovery.json");
    guard.rename_new(&recovery).unwrap();
    assert!(!fixture.path.exists());
    assert_eq!(std::fs::read(&recovery).unwrap(), SOURCE);
    assert_eq!(guard.identity(), identity);
    assert!(OpenOptions::new().write(true).open(&recovery).is_err());
    guard.rename_new(&fixture.path).unwrap();
    assert!(!recovery.exists());
    assert_eq!(std::fs::read(&fixture.path).unwrap(), SOURCE);
    drop(guard);
    let reopened = Guard::open(&fixture.path, SOURCE, &AtomicBool::new(false)).unwrap();
    assert_eq!(reopened.identity(), identity);
}

#[test]
fn movable_never_overwrites_an_existing_destination_or_accepts_relative_paths() {
    let fixture = Fixture::new(SOURCE);
    let guard = Movable::open(&fixture.path, SOURCE, &AtomicBool::new(false)).unwrap();
    let other = fixture.directory.join("other.json");
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&other)
        .unwrap()
        .write_all(b"another writer")
        .unwrap();
    assert!(guard.rename_new(&other).is_err());
    assert!(guard.rename_new(Path::new("relative.json")).is_err());
    assert_eq!(std::fs::read(&fixture.path).unwrap(), SOURCE);
    assert_eq!(std::fs::read(&other).unwrap(), b"another writer");
    std::fs::remove_file(&other).unwrap();
}

#[test]
fn held_baseline_allows_readers_but_prevents_write_and_rename_then_releases() {
    let fixture = Fixture::new(SOURCE);
    let cancel = AtomicBool::new(false);
    let guard = Guard::open(&fixture.path, SOURCE, &cancel).unwrap();
    let second = Guard::open(&fixture.path, SOURCE, &cancel).unwrap();
    assert_eq!(guard.identity(), second.identity());
    assert_eq!(std::fs::read(&fixture.path).unwrap(), SOURCE);
    assert!(OpenOptions::new().write(true).open(&fixture.path).is_err());
    assert!(std::fs::rename(&fixture.path, fixture.directory.join("moved.json")).is_err());
    drop(second);
    assert!(OpenOptions::new().write(true).open(&fixture.path).is_err());
    drop(guard);
    assert!(OpenOptions::new().write(true).open(&fixture.path).is_ok());
    assert_eq!(std::fs::read(&fixture.path).unwrap(), SOURCE);
}
#[test]
fn same_revision_same_length_changes_are_conflicts_and_failure_releases_handle() {
    let fixture = Fixture::new(SOURCE);
    let mut different = SOURCE.to_vec();
    let index = different
        .windows(3)
        .position(|bytes| bytes == b"old")
        .unwrap();
    different[index] = b'n';
    let result = Guard::open(&fixture.path, &different, &AtomicBool::new(false));
    assert!(matches!(result, Err(ref error) if error.kind() == io::ErrorKind::InvalidData));
    assert!(OpenOptions::new().write(true).open(&fixture.path).is_ok());
    assert_eq!(std::fs::read(&fixture.path).unwrap(), SOURCE);
}
#[test]
fn existing_writer_cancellation_missing_and_relative_paths_are_rejected_without_writes() {
    let fixture = Fixture::new(SOURCE);
    let writer = OpenOptions::new().write(true).open(&fixture.path).unwrap();
    assert!(Guard::open(&fixture.path, SOURCE, &AtomicBool::new(false)).is_err());
    drop(writer);
    assert!(
        matches!(Guard::open(&fixture.path, SOURCE, &AtomicBool::new(true)), Err(ref error) if error.kind() == io::ErrorKind::Interrupted)
    );
    assert!(Guard::open(Path::new("relative.json"), SOURCE, &AtomicBool::new(false)).is_err());
    let missing = fixture.directory.join("missing.json");
    assert!(Guard::open(&missing, SOURCE, &AtomicBool::new(false)).is_err());
    assert!(!missing.exists());
    assert_eq!(std::fs::read(&fixture.path).unwrap(), SOURCE);
}
