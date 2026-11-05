use super::*;
#[test]
fn stream_failure_and_late_cancel_remove_owned_partial_file() {
    let fixture = Fixture::new();
    let cancel = AtomicBool::new(false);
    let result = Staged::prepare_stream(&fixture.target(), &cancel, |file, _| {
        file.write_all(b"partial encoded content")?;
        Err(io::Error::other("injected stream error"))
    });
    assert!(result.is_err());
    assert_eq!(std::fs::read_dir(&fixture.0).unwrap().count(), 0);
    let result = Staged::prepare_stream(&fixture.target(), &cancel, |file, cancel| {
        file.write_all(b"complete but cancelled")?;
        cancel.store(true, Ordering::Release);
        Ok(())
    });
    assert!(result.is_err());
    assert_eq!(std::fs::read_dir(&fixture.0).unwrap().count(), 0);
}
#[test]
fn streaming_matching_bytes_do_not_impersonate_a_bound_session_candidate() {
    let fixture = Fixture::new();
    let cancel = AtomicBool::new(false);
    let candidate = candidate();
    let staged = Staged::prepare_stream(&fixture.target(), &cancel, |file, _| {
        file.write_all(&candidate.bytes)
    })
    .unwrap();
    assert!(!staged.is_for(&candidate));
    assert!(staged.bytes.is_none());
    drop(staged);
    assert_eq!(std::fs::read_dir(&fixture.0).unwrap().count(), 0);
}
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root =
            Path::new("C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-save-tests");
        std::fs::create_dir_all(root).unwrap();
        let path = root.join(format!("{:?}", unsafe { CoCreateGuid() }.unwrap()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn target(&self) -> PathBuf {
        self.0.join("会话 新建.json")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(self.target());
        let _ = std::fs::remove_dir(&self.0);
    }
}
fn candidate() -> Candidate {
    Candidate {
        bytes: br#"{"documentSchemaVersion":1,"documentRevision":1,"stickers":[],"links":[]}"#
            .to_vec()
            .into(),
        expected_revision: 0,
        revision: 1,
    }
}
#[test]
fn bound_session_candidate_cannot_use_resource_incremented_publication() {
    let fixture = Fixture::new();
    let cancel = AtomicBool::new(false);
    let staged = Staged::prepare(&fixture.target(), &candidate(), &cancel).unwrap();
    let temporary = staged.temporary.clone();
    assert_eq!(
        staged
            .commit_incremented("other", Some("json"), &cancel)
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidInput
    );
    assert!(!temporary.exists());
    assert_eq!(std::fs::read_dir(&fixture.0).unwrap().count(), 0);
}
#[test]
fn staged_file_is_not_published_until_commit_and_reads_as_complete_session() {
    let fixture = Fixture::new();
    let cancel = AtomicBool::new(false);
    let candidate = candidate();
    let staged = Staged::prepare(&fixture.target(), &candidate, &cancel).unwrap();
    assert!(!fixture.target().exists());
    assert_eq!(
        std::fs::read(&staged.temporary).unwrap(),
        candidate.bytes.as_ref()
    );
    let temporary = staged.temporary.clone();
    let target = staged.commit_new(&cancel).unwrap();
    assert!(!temporary.exists());
    let bytes = std::fs::read(target).unwrap();
    assert_eq!(bytes, candidate.bytes.as_ref());
    assert_eq!(decode(&bytes).unwrap().revision, 1);
}
#[test]
fn concurrent_existing_destination_is_never_overwritten_and_temp_is_cleaned() {
    let fixture = Fixture::new();
    let cancel = AtomicBool::new(false);
    let staged = Staged::prepare(&fixture.target(), &candidate(), &cancel).unwrap();
    let temporary = staged.temporary.clone();
    std::fs::write(fixture.target(), b"other writer").unwrap();
    assert!(staged.commit_new(&cancel).is_err());
    assert_eq!(std::fs::read(fixture.target()).unwrap(), b"other writer");
    assert!(!temporary.exists());
}
#[test]
fn cancel_drop_and_invalid_candidate_do_not_publish_files() {
    let fixture = Fixture::new();
    let cancel = AtomicBool::new(false);
    let staged = Staged::prepare(&fixture.target(), &candidate(), &cancel).unwrap();
    let temporary = staged.temporary.clone();
    drop(staged);
    assert!(!temporary.exists());
    let staged = Staged::prepare(&fixture.target(), &candidate(), &cancel).unwrap();
    let temporary = staged.temporary.clone();
    cancel.store(true, Ordering::Release);
    assert!(staged.commit_new(&cancel).is_err());
    assert!(!temporary.exists() && !fixture.target().exists());
    cancel.store(false, Ordering::Release);
    let mut invalid = candidate();
    invalid.revision = 5;
    assert!(Staged::prepare(&fixture.target(), &invalid, &cancel).is_err());
    assert!(Staged::prepare(Path::new("relative.json"), &candidate(), &cancel).is_err());
    assert_eq!(std::fs::read_dir(&fixture.0).unwrap().count(), 0);
}

#[test]
fn staged_binding_requires_same_buffer_and_revision_not_equal_bytes() {
    let fixture = Fixture::new();
    let candidate = candidate();
    let staged = Staged::prepare(&fixture.target(), &candidate, &AtomicBool::new(false)).unwrap();
    assert!(staged.is_for(&candidate));
    let mut other = Candidate {
        bytes: candidate.bytes.to_vec().into(),
        expected_revision: 0,
        revision: 1,
    };
    assert!(!staged.is_for(&other));
    other.bytes = candidate.bytes.clone();
    assert!(staged.is_for(&other));
    other.revision += 1;
    assert!(!staged.is_for(&other));
}

#[test]
fn raw_config_staging_cannot_claim_session_candidate_identity() {
    let fixture = Fixture::new();
    let candidate = candidate();
    let staged = Staged::prepare_bytes(
        &fixture.target(),
        candidate.bytes.clone(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(!staged.is_for(&candidate));
}
