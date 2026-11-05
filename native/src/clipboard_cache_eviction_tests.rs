use super::*;
use std::time::{SystemTime, UNIX_EPOCH};
#[test]
fn changed_receipt_cannot_be_retired_using_an_old_sorted_candidate() {
    let directory = Directory::new();
    let cancel = AtomicBool::new(false);
    let target = stage(&directory.0, "old", png(), &cancel)
        .unwrap()
        .commit(&cancel)
        .unwrap();
    let path = fs::read_dir(directory.cache())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.extension().is_some_and(|ext| ext == "json"))
        .unwrap();
    let expected = {
        let (_, record) = cleanup::read_record(&path).unwrap();
        record
    };
    let mut changed: receipt::Receipt = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    changed.created += 1;
    fs::write(&path, serde_json::to_vec(&changed).unwrap()).unwrap();
    let _lock = OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(directory.cache().join("cache.lock"))
        .unwrap();
    assert!(cleanup::retire_matching(
        &directory.cache(),
        &path,
        |current| current == &expected,
        &mut 0,
        &cancel
    )
    .unwrap()
    .is_none());
    assert!(target.exists() && path.exists());
}
fn record_times(directory: &Directory) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    for entry in fs::read_dir(directory.cache()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|ext| ext == "json") {
            let mut record: receipt::Receipt =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            record.created = now
                - match record.name.as_str() {
                    "old.png" => 3,
                    "middle.png" => 2,
                    _ => 1,
                };
            fs::write(path, serde_json::to_vec(&record).unwrap()).unwrap();
        }
    }
}
#[test]
fn actual_staging_evicts_oldest_owned_files_to_half_quota_before_new_output() {
    let directory = Directory::new();
    let cancel = AtomicBool::new(false);
    let mut bytes = png().to_vec();
    bytes.resize(12 * 1024 * 1024, 0);
    let bytes: crate::pixels::Pixels = bytes.into();
    for name in ["newest", "old", "middle"] {
        stage(&directory.0, name, bytes.clone(), &cancel)
            .unwrap()
            .commit(&cancel)
            .unwrap();
    }
    record_times(&directory);
    let policy = crate::cache_policy::Policy {
        temp_cache_max_bytes: 32 * 1024 * 1024,
        temp_cache_retention_days: 0,
    };
    let output = stage_with_policy(&directory.0, "incoming", png(), &cancel, policy)
        .unwrap()
        .commit(&cancel)
        .unwrap();
    assert!(output.exists());
    assert!(directory.cache().join("newest.png").exists());
    assert!(!directory.cache().join("old.png").exists());
    assert!(!directory.cache().join("middle.png").exists());
    let total: u64 = fs::read_dir(directory.cache())
        .unwrap()
        .map(|entry| entry.unwrap().metadata().unwrap().len())
        .sum();
    assert!(total < 16 * 1024 * 1024);
}
#[test]
fn unknown_modified_busy_and_hash_budget_limited_candidates_are_not_blindly_evicted() {
    let directory = Directory::new();
    let cancel = AtomicBool::new(false);
    let target = stage(&directory.0, "old", png(), &cancel)
        .unwrap()
        .commit(&cancel)
        .unwrap();
    let total: u64 = fs::read_dir(directory.cache())
        .unwrap()
        .map(|entry| entry.unwrap().metadata().unwrap().len())
        .sum();
    let _lock = OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(directory.cache().join("cache.lock"))
        .unwrap();
    let reader = OpenOptions::new()
        .read(true)
        .share_mode(windows::Win32::Storage::FileSystem::FILE_SHARE_READ.0)
        .open(&target)
        .unwrap();
    assert!(eviction::run(&directory.cache(), 1, total, &mut 0, &cancel).is_err());
    assert!(target.exists());
    drop(reader);
    let mut exhausted = MAX_BYTES;
    assert!(eviction::run(&directory.cache(), 1, total, &mut exhausted, &cancel).is_err());
    assert!(target.exists());
    let mut bytes = fs::read(&target).unwrap();
    bytes[0] ^= 1;
    fs::write(&target, &bytes).unwrap();
    assert!(eviction::run(&directory.cache(), 1, total, &mut 0, &cancel).is_err());
    assert_eq!(fs::read(target).unwrap(), bytes);
    let unknown = directory.cache().join("unknown.png");
    fs::write(&unknown, b"external").unwrap();
    assert!(eviction::run(&directory.cache(), 1, 1, &mut 0, &cancel).is_err());
    assert_eq!(fs::read(unknown).unwrap(), b"external");
}
#[test]
fn disabled_quota_cancel_and_impossible_incoming_preserve_cache() {
    let directory = Directory::new();
    let cancel = AtomicBool::new(false);
    let target = stage(&directory.0, "old", png(), &cancel)
        .unwrap()
        .commit(&cancel)
        .unwrap();
    let _lock = OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(directory.cache().join("cache.lock"))
        .unwrap();
    eviction::run(&directory.cache(), u64::MAX, 0, &mut 0, &cancel).unwrap();
    assert!(eviction::run(&directory.cache(), 11, 10, &mut 0, &cancel).is_err());
    assert!(target.exists());
    cancel.store(true, Ordering::Release);
    assert!(eviction::run(&directory.cache(), 1, 1, &mut 0, &cancel).is_err());
    assert!(target.exists());
}
