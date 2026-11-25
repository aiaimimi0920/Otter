//! 测试只读写系统临时目录内自己创建的文件，不依赖真实会话或机器固定路径。
use super::*;
use std::{io::Write, path::PathBuf, sync::mpsc, time::Duration};
struct Fixture(PathBuf);
impl Fixture {
    fn new(bytes: &[u8], length: Option<u64>) -> Self {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("otter-session-{}-{unique}", std::process::id()));
        // 恢复读取会枚举父目录；独立夹具目录不受真实 TEMP 条目数量影响。
        std::fs::create_dir(&root).unwrap();
        let path = root.join("session.json");
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

    fn recovery_path(&self) -> PathBuf {
        let mut name = self.0.file_name().unwrap().to_os_string();
        name.push(".otter-recovery-01234567-89AB-CDEF-0123-456789ABCDEF.json");
        self.0.with_file_name(name)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
        let _ = std::fs::remove_file(self.recovery_path());
        let _ = std::fs::remove_dir(self.0.parent().unwrap());
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
        None,
        Arc::new(move || {
            let _ = tx.send(std::thread::current().id());
        }),
    )
    .unwrap();
    assert_ne!(rx.recv_timeout(Duration::from_secs(5)).unwrap(), caller);
    job.test_wait_finished();
    let candidate = job.poll().unwrap().unwrap().restore.vector.candidate;
    assert_eq!(candidate.original_json(), original);
    assert!(candidate.images.is_empty());
    let origin = candidate.origin.as_ref().unwrap();
    assert_eq!(origin.requested, fixture.0);
    assert!(origin.recovery.is_none());
    assert!(job.poll().is_none());
}

#[test]
fn missing_file_worker_reads_unique_recovery_and_records_actual_source_without_renaming() {
    let original = br#"{"documentRevision":4,"stickers":[],"links":[]}"#;
    let fixture = Fixture::new(original, None);
    let recovery = fixture.recovery_path();
    std::fs::rename(&fixture.0, &recovery).unwrap();
    let (tx, rx) = mpsc::channel();
    let mut job = Job::start_input(
        Input::File(fixture.0.clone()),
        None,
        None,
        Arc::new(move || {
            let _ = tx.send(());
        }),
    )
    .unwrap();
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    job.test_wait_finished();
    let candidate = job.poll().unwrap().unwrap().restore.vector.candidate;
    assert_eq!(candidate.original_json(), original);
    let origin = candidate.origin.as_ref().unwrap();
    assert_eq!(origin.requested, fixture.0);
    assert_eq!(origin.recovery.as_ref(), Some(&recovery));
    assert!(!fixture.0.exists());
    assert_eq!(std::fs::read(&recovery).unwrap(), original);
}
