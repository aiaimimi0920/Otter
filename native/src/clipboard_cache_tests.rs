use super::*;
#[path = "clipboard_cache_cleanup_tests.rs"]
mod cleanup_tests;
#[path = "clipboard_cache_eviction_tests.rs"]
mod eviction_tests;
#[path = "clipboard_cache_hdr_tests.rs"]
mod hdr_tests;
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        Self(
            PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi")
                .join(format!("otter-cache-{:?}", unsafe {
                    windows::Win32::System::Com::CoCreateGuid().unwrap()
                })),
        )
    }
    fn cache(&self) -> PathBuf {
        self.0.join("clipboard_cache")
    }
    fn entries(&self) -> usize {
        fs::read_dir(self.cache()).unwrap().count()
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        if self.cache().exists() {
            for file in fs::read_dir(self.cache()).unwrap() {
                fs::remove_file(file.unwrap().path()).unwrap();
            }
            fs::remove_dir(self.cache()).unwrap();
        }
        if self.0.exists() {
            fs::remove_dir(&self.0).unwrap();
        }
    }
}
fn png() -> crate::pixels::Pixels {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("src-tauri/icons/32x32.png"),
    )
    .unwrap()
    .into()
}
#[test]
fn cache_lock_wait_is_cancellable_without_releasing_another_owner() {
    let directory = Directory::new();
    let first = stage(&directory.0, "held", png(), &AtomicBool::new(false)).unwrap();
    let path = directory.cache().join("cache.lock");
    let signal = std::sync::Arc::new(AtomicBool::new(false));
    let worker_signal = signal.clone();
    let worker = std::thread::spawn(move || acquire_lock(&path, &worker_signal));
    std::thread::sleep(std::time::Duration::from_millis(20));
    signal.store(true, Ordering::Release);
    assert_eq!(
        worker.join().unwrap().unwrap_err().kind(),
        io::ErrorKind::Interrupted
    );
    assert_eq!(directory.entries(), 2);
    drop(first);
    assert_eq!(directory.entries(), 1);
}
#[test]
fn canonical_cache_paths_become_ordinary_disk_or_unc_without_loss() {
    assert_eq!(
        ordinary_path(PathBuf::from(r"\\?\C:\中文🙂\a.png")).unwrap(),
        PathBuf::from(r"C:\中文🙂\a.png")
    );
    assert_eq!(
        ordinary_path(PathBuf::from(r"\\?\UNC\host\share\中文🙂.png")).unwrap(),
        PathBuf::from(r"\\host\share\中文🙂.png")
    );
    assert!(ordinary_path(PathBuf::from(r"\\.\pipe\name")).is_err());
}
#[test]
fn staging_cancel_and_drop_leave_no_published_file_and_release_lock() {
    let directory = Directory::new();
    let cancel = AtomicBool::new(false);
    let prepared = stage(&directory.0, "名称", png(), &cancel).unwrap();
    assert_eq!(directory.entries(), 2); // 排他锁文件 + 未发布暂存。
    assert!(!directory.cache().join("名称.png").exists());
    assert!(stage(&directory.0, "第二个", png(), &cancel).is_err());
    drop(prepared);
    assert_eq!(directory.entries(), 1);
    let prepared = stage(&directory.0, "名称", png(), &cancel).unwrap();
    cancel.store(true, Ordering::Release);
    assert_eq!(
        prepared.commit(&cancel).unwrap_err().kind(),
        io::ErrorKind::Interrupted
    );
    assert_eq!(directory.entries(), 1);
    cancel.store(false, Ordering::Release);
    drop(stage(&directory.0, "名称", png(), &cancel).unwrap());
}
#[test]
fn publish_reuses_staged_bytes_and_preserves_existing_files() {
    let directory = Directory::new();
    let cancel = AtomicBool::new(false);
    let bytes = png();
    let first = stage(&directory.0, "复制🙂", bytes.clone(), &cancel)
        .unwrap()
        .commit(&cancel)
        .unwrap();
    let second = stage(&directory.0, "复制🙂", bytes.clone(), &cancel)
        .unwrap()
        .commit(&cancel)
        .unwrap();
    assert_eq!(first.file_name().unwrap(), "复制🙂.png");
    assert_eq!(second.file_name().unwrap(), "复制🙂_2.png");
    assert_eq!(fs::read(first).unwrap(), &*bytes);
    assert_eq!(fs::read(second).unwrap(), &*bytes);
    assert_eq!(directory.entries(), 5); // lock + 两个 PNG + 两个所有权记录。
}

#[test]
fn published_receipt_binds_exact_filename_bytes_digest_and_file_identity() {
    use sha2::{Digest, Sha256};
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION},
    };
    let directory = Directory::new();
    let cancel = AtomicBool::new(false);
    let bytes = png();
    let path = stage(&directory.0, "所有权🙂", bytes.clone(), &cancel)
        .unwrap()
        .commit(&cancel)
        .unwrap();
    let records: Vec<_> = fs::read_dir(directory.cache())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".otter-clipboard-")
        })
        .collect();
    assert_eq!(records.len(), 1);
    let raw = fs::read(&records[0]).unwrap();
    assert!(raw.len() <= receipt::MAX_BYTES);
    let record: receipt::Receipt = serde_json::from_slice(&raw).unwrap();
    assert_eq!(record.version, 1);
    assert_eq!(record.name, "所有权🙂.png");
    assert_eq!(record.bytes, bytes.len() as u64);
    assert_eq!(record.sha256, <[u8; 32]>::from(Sha256::digest(&*bytes)));
    assert!(record.created > 0);
    let file = File::open(path).unwrap();
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    unsafe {
        GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut info).unwrap();
    }
    assert_eq!(record.volume, info.dwVolumeSerialNumber);
    assert_eq!(
        record.index,
        (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow)
    );
}

#[test]
fn cancelled_stage_has_no_receipt_and_foreign_files_are_not_claimed() {
    let directory = Directory::new();
    let cancel = AtomicBool::new(false);
    drop(stage(&directory.0, "foreign", png(), &cancel).unwrap());
    let foreign = directory.cache().join("foreign.png");
    fs::write(&foreign, b"external content").unwrap();
    let path = stage(&directory.0, "foreign", png(), &cancel)
        .unwrap()
        .commit(&cancel)
        .unwrap();
    assert_eq!(path.file_name().unwrap(), "foreign_2.png");
    assert_eq!(fs::read(foreign).unwrap(), b"external content");
    let records: Vec<_> = fs::read_dir(directory.cache())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    assert_eq!(records.len(), 1);
    let record: receipt::Receipt = serde_json::from_slice(&fs::read(&records[0]).unwrap()).unwrap();
    assert_eq!(record.name, "foreign_2.png");
}
#[test]
fn invalid_and_full_cache_are_rejected_without_deleting_existing_content() {
    let directory = Directory::new();
    let cancel = AtomicBool::new(true);
    assert!(stage(&directory.0, "x", png(), &cancel).is_err());
    assert!(!directory.0.exists());
    cancel.store(false, Ordering::Release);
    assert!(stage(&directory.0, "x", b"not PNG".to_vec().into(), &cancel).is_err());
    assert!(!directory.0.exists());
    drop(stage(&directory.0, "x", png(), &cancel).unwrap());
    let existing = directory.cache().join("existing.png");
    let file = File::create(&existing).unwrap();
    file.set_len(MAX_BYTES).unwrap();
    drop(file);
    assert!(stage(&directory.0, "x", png(), &cancel).is_err());
    assert_eq!(fs::metadata(existing).unwrap().len(), MAX_BYTES);
    assert_eq!(directory.entries(), 2);
}
