use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use ring::signature::{Ed25519KeyPair, KeyPair};
use windows::Win32::System::Com::CoCreateGuid;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let base = Path::new(
            "C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-device-store-tests",
        );
        std::fs::create_dir_all(base).unwrap();
        let path = base.join(format!("{:?}", unsafe { CoCreateGuid() }.unwrap()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn store(&self) -> Store {
        Store::new(&self.0).unwrap()
    }
    fn file(&self) -> PathBuf {
        self.0.join("device-identity.json")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // 只清理本测试在固定 linshi 根下创建的单层唯一目录，不递归追随链接。
        if let Ok(entries) = std::fs::read_dir(&self.0) {
            for entry in entries.flatten() {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        let _ = std::fs::remove_dir(&self.0);
    }
}
fn legacy() -> Vec<u8> {
    let seed = [7u8; 32]; // 公开、固定测试 seed，不是用户身份。
    let key = Ed25519KeyPair::from_seed_unchecked(&seed).unwrap();
    serde_json::to_vec(
        &serde_json::json!({"schemaVersion":1,"deviceId":"device:existing",
        "privateKey":STANDARD.encode(seed),"publicKey":STANDARD.encode(key.public_key().as_ref())}),
    )
    .unwrap()
}

#[test]
fn missing_renewal_never_creates_identity_and_explicit_create_is_stable() {
    let fixture = Fixture::new();
    let store = fixture.store();
    assert!(matches!(store.load_existing(), Err(Error::Missing)));
    assert!(!fixture.file().exists());
    let absent = Store::new(&fixture.0.join("absent")).unwrap();
    assert!(matches!(absent.load_existing(), Err(Error::Missing)));
    assert!(!fixture.0.join("absent").exists());
    let first = store.load_or_create().unwrap();
    let bytes = std::fs::read(fixture.file()).unwrap();
    assert!(!first.identity().needs_migration());
    assert_eq!(first.identity().device_id(), None);
    let second = store.load_or_create().unwrap();
    assert_eq!(
        first.identity().public_key().unwrap(),
        second.identity().public_key().unwrap()
    );
    assert_eq!(std::fs::read(fixture.file()).unwrap(), bytes);
}

#[test]
fn legacy_read_is_nonmutating_but_explicit_migration_preserves_identity() {
    let fixture = Fixture::new();
    let bytes = legacy();
    std::fs::write(fixture.file(), &bytes).unwrap();
    let store = fixture.store();
    let read = store.load_existing().unwrap();
    assert!(read.identity().needs_migration());
    assert_eq!(std::fs::read(fixture.file()).unwrap(), bytes);
    let migrated = store.load_or_create().unwrap();
    assert!(!migrated.identity().needs_migration());
    assert_eq!(
        migrated.identity().public_key().unwrap(),
        read.identity().public_key().unwrap()
    );
    assert_eq!(migrated.identity().device_id(), Some("device:existing"));
    let stored: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixture.file()).unwrap()).unwrap();
    assert_eq!(stored["schemaVersion"], 2);
    assert!(stored["privateKey"]
        .as_str()
        .unwrap()
        .starts_with("DPAPI1:"));
}

#[test]
fn binding_rejects_stale_or_foreign_snapshot_and_preserves_loaded_on_failure() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let mut first = store.load_or_create().unwrap();
    let mut stale = store.load_existing().unwrap();
    store.bind_and_save(&mut first, "device:paired").unwrap();
    assert_eq!(first.identity().device_id(), Some("device:paired"));
    let bytes = std::fs::read(fixture.file()).unwrap();
    assert_eq!(
        store.bind_and_save(&mut stale, "device:stale"),
        Err(Error::Changed)
    );
    assert_eq!(stale.identity().device_id(), None);
    assert_eq!(
        fixture.store().bind_and_save(&mut first, "device:foreign"),
        Err(Error::Changed)
    );
    assert!(store.bind_and_save(&mut first, "bad\nvalue").is_err());
    assert_eq!(first.identity().device_id(), Some("device:paired"));
    assert_eq!(std::fs::read(fixture.file()).unwrap(), bytes);
    let replacement = Identity::generate().unwrap().protected_document().unwrap();
    std::fs::write(fixture.file(), &replacement).unwrap();
    assert_eq!(
        store.bind_and_save(&mut first, "device:external-race"),
        Err(Error::Changed)
    );
    assert_eq!(std::fs::read(fixture.file()).unwrap(), replacement);
}

#[test]
fn malformed_or_oversized_identity_is_preserved_without_replacement() {
    let fixture = Fixture::new();
    let store = fixture.store();
    for bytes in [b"broken-json".to_vec(), vec![b'x'; MAX_DOCUMENT + 1]] {
        std::fs::write(fixture.file(), &bytes).unwrap();
        assert!(store.load_or_create().is_err());
        assert_eq!(std::fs::read(fixture.file()).unwrap(), bytes);
    }
}

#[test]
fn cooperating_lock_and_failed_commit_preserve_destination_and_remove_only_owned_temp() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let loaded = store.load_or_create().unwrap();
    let (_root, lock) = store.lock(false).unwrap();
    assert!(matches!(fixture.store().load_existing(), Err(Error::Lock)));
    drop(lock);
    let before = std::fs::read(fixture.file()).unwrap();
    let protected = loaded.identity().protected_document().unwrap();
    assert!(device_identity_file::write(&fixture.file(), &protected, false).is_err());
    let held = OpenOptions::new()
        .read(true)
        .share_mode(windows::Win32::Storage::FileSystem::FILE_SHARE_READ.0)
        .open(fixture.file())
        .unwrap();
    assert!(device_identity_file::write(&fixture.file(), &protected, true).is_err());
    drop(held);
    assert_eq!(std::fs::read(fixture.file()).unwrap(), before);
    assert!(device_identity_file::write(&fixture.file(), &legacy(), true).is_err());
    let names = std::fs::read_dir(&fixture.0)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect::<Vec<_>>();
    assert_eq!(names.len(), 2); // 仅正式身份与持久 lock 文件，失败暂存已回收。
}

#[test]
fn abandoned_encrypted_temp_is_not_promoted_or_removed_by_loading() {
    let fixture = Fixture::new();
    let store = fixture.store();
    let first = store.load_or_create().unwrap();
    let abandoned = fixture.0.join(".device-identity-abandoned.tmp");
    let other = Identity::generate().unwrap().protected_document().unwrap();
    std::fs::write(&abandoned, &other).unwrap();
    let loaded = store.load_existing().unwrap();
    assert_eq!(
        loaded.identity().public_key().unwrap(),
        first.identity().public_key().unwrap()
    );
    assert_eq!(std::fs::read(abandoned).unwrap(), other);
}
