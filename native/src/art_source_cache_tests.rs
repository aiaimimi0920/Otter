use super::*;
fn root() -> PathBuf {
    let id = unsafe { windows::Win32::System::Com::CoCreateGuid() }.unwrap();
    let root = PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-parallel-20261029/main-art-source-cache-1").join(format!("store-{id:?}"));
    std::fs::create_dir(&root).unwrap();
    root
}
#[test]
fn cache_reopens_exact_bytes_without_grant_or_credentials() {
    let root = root();
    let store = Store::new(&root);
    assert!(store.load(&[1; 32]).unwrap().is_none());
    store.put(&[1; 32], b"let value=42;", &|| false).unwrap();
    let cached = Store::new(&root).load(&[1; 32]).unwrap().unwrap();
    assert_eq!(cached.source, b"let value=42;");
    assert_eq!(
        cached.digest,
        format!("{:x}", Sha256::digest(b"let value=42;"))
    );
    assert!(Store::new(&root).load(&[2; 32]).unwrap().is_none());
    assert_eq!(
        std::fs::read_dir(root.join("art-source-cache"))
            .unwrap()
            .count(),
        2
    );
}
#[test]
fn cache_rejects_invalid_source_and_replaces_corruption_atomically() {
    let root = root();
    let store = Store::new(&root);
    for invalid in [vec![], vec![0], vec![255], vec![b'a'; MAX_SOURCE + 1]] {
        assert!(store.put(&[1; 32], &invalid, &|| false).is_err());
    }
    store.put(&[1; 32], b"old;", &|| false).unwrap();
    let file = root.join("art-source-cache").join(name(&[1; 32]));
    for invalid in [vec![], vec![0], vec![255], vec![b'a'; MAX_SOURCE + 1]] {
        std::fs::write(&file, invalid).unwrap();
        assert!(store.load(&[1; 32]).is_err());
    }
    store.put(&[1; 32], b"repaired;", &|| false).unwrap();
    assert_eq!(store.load(&[1; 32]).unwrap().unwrap().source, b"repaired;");
}
#[test]
fn unknown_disk_bytes_and_abandoned_staging_count_against_capacity() {
    let root = root();
    let store = Store::new(&root);
    store.put(&[1; 32], b"keep;", &|| false).unwrap();
    let unknown = root.join("art-source-cache/.abandoned.tmp");
    let file = File::create(&unknown).unwrap();
    file.set_len(MAX_BYTES).unwrap();
    drop(file);
    assert!(store.put(&[2; 32], b"new;", &|| false).is_err());
    assert_eq!(std::fs::metadata(unknown).unwrap().len(), MAX_BYTES);
    assert_eq!(store.load(&[1; 32]).unwrap().unwrap().source, b"keep;");
}
#[test]
fn cache_capacity_lock_and_path_failures_never_evict_existing_content() {
    let root = root();
    let store = Store::new(&root);
    for i in 0..MAX_ENTRIES {
        store.put(&[i as u8; 32], b"0", &|| false).unwrap();
    }
    assert!(store.put(&[100; 32], b"1", &|| false).is_err());
    store.put(&[0; 32], b"replacement;", &|| false).unwrap();
    assert_eq!(
        store.load(&[0; 32]).unwrap().unwrap().source,
        b"replacement;"
    );
    let (_, lock) = store.lock(false).unwrap();
    assert!(store.load(&[0; 32]).is_err());
    drop(lock);
    assert!(Store::new(Path::new("relative"))
        .put(&[1; 32], b"0", &|| false)
        .is_err());
    let file = root.join("not-a-profile");
    std::fs::write(&file, b"keep").unwrap();
    assert!(Store::new(&file).put(&[1; 32], b"0", &|| false).is_err());
    assert_eq!(std::fs::read(file).unwrap(), b"keep");
}
#[test]
fn cache_cancelled_staging_preserves_old_source_and_cleans_owned_temp() {
    let root = root();
    let store = Store::new(&root);
    store.put(&[1; 32], b"old;", &|| false).unwrap();
    let observations = std::cell::Cell::new(0);
    let stopped = || {
        let n = observations.get();
        observations.set(n + 1);
        n >= 2
    };
    assert!(store.put(&[1; 32], b"new;", &stopped).is_err());
    assert_eq!(store.load(&[1; 32]).unwrap().unwrap().source, b"old;");
    assert_eq!(
        std::fs::read_dir(root.join("art-source-cache"))
            .unwrap()
            .count(),
        2
    );
}
#[test]
fn cache_persists_into_an_independent_process() {
    let root = root();
    Store::new(&root)
        .put(&[7; 32], b"independent;", &|| false)
        .unwrap();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "art_source_cache::tests::cache_child_reader",
            "--exact",
            "--ignored",
            "--test-threads=1",
        ])
        .env("OTTER_ART_CACHE_TEST_ROOT", &root)
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("cache child timeout");
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
#[test]
#[ignore = "private independent-process cache reader; invoked by parent gate"]
fn cache_child_reader() {
    let root = PathBuf::from(std::env::var_os("OTTER_ART_CACHE_TEST_ROOT").unwrap());
    assert_eq!(
        Store::new(&root).load(&[7; 32]).unwrap().unwrap().source,
        b"independent;"
    );
}
