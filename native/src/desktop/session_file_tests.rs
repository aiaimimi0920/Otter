//! 临时文件只放 linshi；不读取真实会话，测试结束仅删除自己 create_new 创建的文件。
use super::*;
use std::{io::Write, path::PathBuf, sync::mpsc, time::Duration};
struct Fixture(PathBuf);
impl Fixture {
    fn new(bytes: &[u8], length: Option<u64>) -> Self {
        let root = PathBuf::from(
            "C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-session-files",
        );
        std::fs::create_dir_all(&root).unwrap();
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = root.join(format!("{}-{unique}.json", std::process::id()));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        file.write_all(bytes).unwrap();
        if let Some(length) = length {
            file.set_len(length).unwrap();
        }
        drop(file);
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
fn bounded_file_read_rejects_oversize_devices_and_cancel_without_leaking_handle() {
    let cancel = AtomicBool::new(false);
    let fixture = Fixture::new(&vec![b'x'; 70_000], None);
    assert_eq!(file::read(&fixture.0, &cancel).unwrap(), vec![b'x'; 70_000]);
    // 读句柄不再占用，能够重新获得独占写访问。
    use std::os::windows::fs::OpenOptionsExt;
    assert!(std::fs::OpenOptions::new()
        .write(true)
        .share_mode(0)
        .open(&fixture.0)
        .is_ok());
    cancel.store(true, Ordering::Release);
    assert_eq!(
        file::read(&fixture.0, &cancel).unwrap_err().kind(),
        std::io::ErrorKind::Interrupted
    );
    cancel.store(false, Ordering::Release);
    let oversized = Fixture::new(
        &[],
        Some(crate::session_document::MAX_SESSION_BYTES as u64 + 1),
    );
    assert_eq!(
        file::read(&oversized.0, &cancel).unwrap_err().kind(),
        std::io::ErrorKind::InvalidData
    );
    for path in [
        std::path::Path::new("relative.json"),
        std::path::Path::new(r"\\.\NUL"),
        std::path::Path::new("C:/NUL"),
    ] {
        assert!(file::read(path, &cancel).is_err());
    }
    assert!(file::read(fixture.0.parent().unwrap(), &cancel).is_err());
}

#[test]
fn file_input_runs_in_worker_and_preserves_original_utf8() {
    let original = "{\"stickers\":[],\"links\":[],\"future\":\"保留原文\"}".as_bytes();
    let fixture = Fixture::new(original, None);
    let caller = std::thread::current().id();
    let (tx, rx) = mpsc::channel();
    let mut job = Job::start_input(
        Input::File(fixture.0.clone()),
        None,
        Arc::new(move || {
            let _ = tx.send(std::thread::current().id());
        }),
    )
    .unwrap();
    assert_ne!(rx.recv_timeout(Duration::from_secs(5)).unwrap(), caller);
    let candidate = job.poll().unwrap().unwrap();
    assert_eq!(candidate.candidate.original_json(), original);
    assert!(candidate.candidate.images.is_empty());
    let origin = candidate.candidate.origin.as_ref().unwrap();
    assert_eq!(origin.requested, fixture.0);
    assert!(origin.recovery.is_none());
    assert!(job.poll().is_none());
}

#[test]
fn missing_file_worker_reads_unique_recovery_and_records_actual_source_without_renaming() {
    let original = br#"{"documentRevision":4,"stickers":[],"links":[]}"#;
    let fixture = Fixture::new(original, None);
    let mut name = fixture.0.file_name().unwrap().to_os_string();
    name.push(".otter-recovery-01234567-89AB-CDEF-0123-456789ABCDEF.json");
    let recovery = fixture.0.with_file_name(name);
    std::fs::rename(&fixture.0, &recovery).unwrap();
    let (tx, rx) = mpsc::channel();
    let mut job = Job::start_input(
        Input::File(fixture.0.clone()),
        None,
        Arc::new(move || {
            let _ = tx.send(());
        }),
    )
    .unwrap();
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let candidate = job.poll().unwrap().unwrap();
    assert_eq!(candidate.candidate.original_json(), original);
    let origin = candidate.candidate.origin.as_ref().unwrap();
    assert_eq!(origin.requested, fixture.0);
    assert_eq!(origin.recovery.as_ref(), Some(&recovery));
    assert!(!fixture.0.exists());
    assert_eq!(std::fs::read(&recovery).unwrap(), original);
    std::fs::remove_file(recovery).unwrap();
}
