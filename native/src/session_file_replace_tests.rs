use super::*;
use std::io::Write;
const OLD: &[u8] = br#"{"documentRevision":1,"stickers":[],"links":[],"note":"old"}"#;
const NEW: &[u8] = br#"{"documentRevision":2,"stickers":[],"links":[],"note":"new"}"#;
struct Fixture {
    directory: PathBuf,
    target: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root =
            Path::new("C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-session-replace-tests");
        std::fs::create_dir_all(root).unwrap();
        let id = unsafe { windows::Win32::System::Com::CoCreateGuid().unwrap() };
        let directory = root.join(format!("{id:?}"));
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
    fn prepare(&self, cancel: &AtomicBool) -> Replacement {
        Replacement::prepare(&self.target, OLD, &candidate(), cancel).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.target);
        let _ = std::fs::remove_dir(&self.directory);
    }
}
fn candidate() -> Candidate {
    Candidate {
        bytes: NEW.to_vec().into(),
        expected_revision: 1,
        revision: 2,
    }
}
#[test]
fn replacement_publishes_exact_candidate_and_retires_only_old_handle() {
    let fixture = Fixture::new();
    let cancel = AtomicBool::new(false);
    let candidate = candidate();
    let replacement = Replacement::prepare(&fixture.target, OLD, &candidate, &cancel).unwrap();
    assert!(replacement.is_for(&candidate));
    assert!(!replacement.is_for(&super::tests::candidate()));
    let result = replacement.commit(&cancel).unwrap();
    assert!(result.retained_recovery.is_none());
    assert_eq!(std::fs::read(result.destination).unwrap(), NEW);
    assert_eq!(std::fs::read_dir(&fixture.directory).unwrap().count(), 1);
}
#[test]
fn drop_and_cancel_before_or_after_old_move_preserve_original() {
    let fixture = Fixture::new();
    let cancel = AtomicBool::new(false);
    drop(fixture.prepare(&cancel));
    assert_eq!(std::fs::read(&fixture.target).unwrap(), OLD);
    for after_move in [false, true] {
        cancel.store(false, Ordering::Release);
        let replacement = fixture.prepare(&cancel);
        if !after_move {
            cancel.store(true, Ordering::Release);
        }
        let failure = replacement
            .commit_with(&cancel, |_| cancel.store(true, Ordering::Release))
            .unwrap_err();
        assert!(failure.recovery.is_none() && failure.restore_error.is_none());
        assert_eq!(failure.error.kind(), io::ErrorKind::Interrupted);
        assert_eq!(std::fs::read(&fixture.target).unwrap(), OLD);
        assert_eq!(std::fs::read_dir(&fixture.directory).unwrap().count(), 1);
    }
}
#[test]
fn competing_target_is_kept_and_original_recovery_path_is_reported() {
    let fixture = Fixture::new();
    let cancel = AtomicBool::new(false);
    let failure = fixture
        .prepare(&cancel)
        .commit_with(&cancel, |path| {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .unwrap()
                .write_all(b"competing writer")
                .unwrap();
        })
        .unwrap_err();
    assert!(failure.restore_error.is_some());
    let recovery = failure.recovery.unwrap();
    assert_eq!(std::fs::read(&fixture.target).unwrap(), b"competing writer");
    assert_eq!(std::fs::read(&recovery).unwrap(), OLD);
    std::fs::remove_file(recovery).unwrap();
}
#[test]
fn conflict_or_wrong_revision_never_moves_source() {
    let fixture = Fixture::new();
    let cancel = AtomicBool::new(false);
    let mut candidate = candidate();
    candidate.expected_revision = 0;
    assert!(Replacement::prepare(&fixture.target, OLD, &candidate, &cancel).is_err());
    candidate.expected_revision = 1;
    let mut changed = OLD.to_vec();
    let index = changed
        .windows(3)
        .position(|bytes| bytes == b"old")
        .unwrap();
    changed[index] = b'x';
    assert!(Replacement::prepare(&fixture.target, &changed, &candidate, &cancel).is_err());
    assert_eq!(std::fs::read(&fixture.target).unwrap(), OLD);
    assert_eq!(std::fs::read_dir(&fixture.directory).unwrap().count(), 1);
}

#[test]
fn panic_after_move_attempts_non_overwriting_restore() {
    let fixture = Fixture::new();
    let cancel = AtomicBool::new(false);
    let replacement = fixture.prepare(&cancel);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = replacement.commit_with(&cancel, |_| panic!("replacement hook panic"));
    }));
    assert!(result.is_err());
    assert_eq!(std::fs::read(&fixture.target).unwrap(), OLD);
    assert_eq!(std::fs::read_dir(&fixture.directory).unwrap().count(), 1);
}

#[test]
fn recovery_name_collision_is_not_overwritten_or_deleted() {
    let fixture = Fixture::new();
    let cancel = AtomicBool::new(false);
    let replacement = fixture.prepare(&cancel);
    let recovery = replacement.recovery.clone();
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&recovery)
        .unwrap()
        .write_all(b"existing recovery")
        .unwrap();
    let failure = replacement.commit(&cancel).unwrap_err();
    assert!(failure.recovery.is_none());
    assert_eq!(std::fs::read(&fixture.target).unwrap(), OLD);
    assert_eq!(std::fs::read(&recovery).unwrap(), b"existing recovery");
    std::fs::remove_file(recovery).unwrap();
}

#[test]
fn old_file_cleanup_failure_is_reported_as_published_with_recovery_not_failed_save() {
    let fixture = Fixture::new();
    let original_permissions = std::fs::metadata(&fixture.target).unwrap().permissions();
    let cancel = AtomicBool::new(false);
    let replacement = fixture.prepare(&cancel);
    let recovery = replacement.recovery.clone();
    let published = replacement
        .commit_with(&cancel, |_| {
            let mut permissions = std::fs::metadata(&recovery).unwrap().permissions();
            permissions.set_readonly(true);
            std::fs::set_permissions(&recovery, permissions).unwrap();
        })
        .unwrap();
    assert!(published.retained_recovery.is_some());
    assert_eq!(std::fs::read(&fixture.target).unwrap(), NEW);
    assert_eq!(std::fs::read(&recovery).unwrap(), OLD);
    std::fs::set_permissions(&recovery, original_permissions).unwrap();
    std::fs::remove_file(recovery).unwrap();
}
