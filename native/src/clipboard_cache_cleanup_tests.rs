use super::*;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
#[test]
fn configured_retention_and_unlimited_capacity_are_consumed_by_real_staging() {
    let directory = Directory::new();
    let cancel = AtomicBool::new(false);
    let old = stage(&directory.0, "old", png(), &cancel)
        .unwrap()
        .commit(&cancel)
        .unwrap();
    age(&directory, "old.png");
    let mut policy = crate::cache_policy::Policy {
        temp_cache_max_bytes: 0,
        temp_cache_retention_days: 0,
    };
    drop(stage_with_policy(&directory.0, "new", png(), &cancel, policy).unwrap());
    assert!(old.exists());
    policy.temp_cache_retention_days = 30;
    drop(stage_with_policy(&directory.0, "new", png(), &cancel, policy).unwrap());
    assert!(old.exists());
    policy.temp_cache_retention_days = 7;
    drop(stage_with_policy(&directory.0, "new", png(), &cancel, policy).unwrap());
    assert!(!old.exists());
    let existing = directory.cache().join("foreign.bin");
    File::create(&existing)
        .unwrap()
        .set_len(40 * 1024 * 1024)
        .unwrap();
    policy.temp_cache_max_bytes = 32 * 1024 * 1024;
    assert!(stage_with_policy(&directory.0, "new", png(), &cancel, policy).is_err());
    policy.temp_cache_max_bytes = 0;
    drop(stage_with_policy(&directory.0, "new", png(), &cancel, policy).unwrap());
    assert_eq!(fs::metadata(existing).unwrap().len(), 40 * 1024 * 1024);
}
#[test]
fn busy_or_hardlinked_files_are_retained_until_safe_to_retire() {
    let directory = Directory::new();
    let cancel = AtomicBool::new(false);
    let busy = stage(&directory.0, "busy", png(), &cancel)
        .unwrap()
        .commit(&cancel)
        .unwrap();
    let linked = stage(&directory.0, "linked", png(), &cancel)
        .unwrap()
        .commit(&cancel)
        .unwrap();
    age(&directory, "busy.png");
    age(&directory, "linked.png");
    let reader = OpenOptions::new()
        .read(true)
        .share_mode(windows::Win32::Storage::FileSystem::FILE_SHARE_READ.0)
        .open(&busy)
        .unwrap();
    let alias = directory.cache().join("alias.bin");
    fs::hard_link(&linked, &alias).unwrap();
    let report = run(&directory, &cancel).unwrap();
    assert_eq!(report.removed, 0);
    assert_eq!(report.skipped, 2);
    assert!(busy.exists() && linked.exists() && alias.exists());
    drop(reader);
    fs::remove_file(alias).unwrap();
    assert_eq!(run(&directory, &cancel).unwrap().removed, 2);
}
fn records(directory: &Directory) -> Vec<PathBuf> {
    fs::read_dir(directory.cache())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".otter-clipboard-")
        })
        .collect()
}
fn rewrite(path: &Path, update: impl FnOnce(&mut receipt::Receipt)) {
    let mut value: receipt::Receipt = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    update(&mut value);
    fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
}
fn age(directory: &Directory, name: &str) {
    for path in records(directory) {
        rewrite(&path, |record| {
            if record.name == name {
                record.created = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    - 8 * 86400;
            }
        });
    }
}
fn run(directory: &Directory, cancel: &AtomicBool) -> io::Result<cleanup::Report> {
    let _lock = OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(directory.cache().join("cache.lock"))
        .unwrap();
    cleanup::run(&directory.cache(), SystemTime::now(), cancel)
}
#[test]
fn retention_only_retires_expired_unchanged_owned_file_and_record() {
    let directory = Directory::new();
    let cancel = AtomicBool::new(false);
    for name in ["recent", "expired", "modified", "replaced"] {
        stage(&directory.0, name, png(), &cancel)
            .unwrap()
            .commit(&cancel)
            .unwrap();
    }
    for name in ["expired.png", "modified.png", "replaced.png"] {
        age(&directory, name);
    }
    let modified = directory.cache().join("modified.png");
    let mut changed = fs::read(&modified).unwrap();
    changed[0] ^= 1;
    fs::write(&modified, &changed).unwrap();
    let replaced = directory.cache().join("replaced.png");
    fs::rename(&replaced, directory.cache().join("preserved.bin")).unwrap();
    fs::write(&replaced, &*png()).unwrap();
    let foreign = directory.cache().join("foreign.png");
    fs::write(&foreign, b"foreign").unwrap();
    let report = run(&directory, &cancel).unwrap();
    assert_eq!(report.removed, 1);
    assert_eq!(report.skipped, 2);
    assert!(!directory.cache().join("expired.png").exists());
    assert!(directory.cache().join("recent.png").exists());
    assert!(replaced.exists());
    assert_eq!(fs::read(modified).unwrap(), changed);
    assert_eq!(fs::read(foreign).unwrap(), b"foreign");
    assert_eq!(records(&directory).len(), 3);
}
#[test]
fn staging_runs_retention_under_lock_and_cancellation_does_not_delete() {
    let directory = Directory::new();
    let cancel = AtomicBool::new(false);
    let old = stage(&directory.0, "old", png(), &cancel)
        .unwrap()
        .commit(&cancel)
        .unwrap();
    age(&directory, "old.png");
    cancel.store(true, Ordering::Release);
    assert!(run(&directory, &cancel).is_err());
    assert!(old.exists());
    cancel.store(false, Ordering::Release);
    drop(stage(&directory.0, "new", png(), &cancel).unwrap());
    assert!(!old.exists());
    assert!(records(&directory).is_empty());
    assert_eq!(directory.entries(), 1);
}
#[test]
fn traversal_future_version_and_clock_rollback_preserve_targets() {
    let directory = Directory::new();
    let cancel = AtomicBool::new(false);
    for name in ["traversal", "version", "future"] {
        stage(&directory.0, name, png(), &cancel)
            .unwrap()
            .commit(&cancel)
            .unwrap();
        // 只在全部创建后改时间，避免下一次 stage 自动执行本测试尚未观察的清理。
    }
    for name in ["traversal.png", "version.png"] {
        age(&directory, name);
    }
    for path in records(&directory) {
        rewrite(&path, |record| match record.name.as_str() {
            "traversal.png" => record.name = "../clipboard_cache/traversal.png".into(),
            "version.png" => record.version = 999,
            "future.png" => {
                record.created = (SystemTime::now() + Duration::from_secs(86400))
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            }
            _ => unreachable!(),
        });
    }
    let report = run(&directory, &cancel).unwrap();
    assert_eq!(report.removed, 0);
    assert_eq!(report.skipped, 2);
    for name in ["traversal", "version", "future"] {
        assert!(directory.cache().join(format!("{name}.png")).exists());
    }
}
