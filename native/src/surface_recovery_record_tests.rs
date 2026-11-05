//! 磁盘提示不包含凭据/场景，不可直接恢复成授权 owner；文件测试仅使用 linshi。
use super::*;
use std::path::PathBuf;

fn record() -> Record {
    Record {
        origin: "https://localhost:1234".into(),
        device: "viewer:test".into(),
        node: "node:test".into(),
        art: Some("art:test".into()),
        binding: Some(Hint {
            instance: "instance:test".into(),
            attachment: "attachment:test".into(),
            generation: 1,
            lifecycle: Some(Lifecycle::Mounted),
            revision: Some(2),
        }),
        uncertain: true,
        in_flight: false,
    }
}
fn directory() -> PathBuf {
    let guid = unsafe { windows::Win32::System::Com::CoCreateGuid() }.unwrap();
    PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi")
        .join(format!("surface-journal-{guid:?}"))
}
#[test]
fn recovery_file_reader_rejects_device_paths_relative_paths_and_alternate_streams() {
    for path in [
        r"relative.json",
        r"\\.\pipe\otter-recovery-fixture",
        r"C:\file.json:stream",
    ] {
        assert!(crate::surface_recovery_store::load(std::path::Path::new(path)).is_err());
    }
}
#[test]
fn roundtrip_is_metadata_only_and_preserves_unknown() {
    let record = record();
    let bytes = encode(std::slice::from_ref(&record)).unwrap();
    assert_eq!(decode(&bytes).unwrap(), vec![record]);
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let fields = json["records"][0].as_object().unwrap();
    assert_eq!(fields.len(), 7);
    for forbidden in ["token", "scene", "snapshot", "params", "password"] {
        assert!(!fields.contains_key(forbidden));
    }
    assert!(!bytes.starts_with(&[0xef, 0xbb, 0xbf]));
}
#[test]
fn malformed_extended_noncanonical_and_oversized_metadata_fail_closed() {
    let mut json: serde_json::Value =
        serde_json::from_slice(&encode(&[record()]).unwrap()).unwrap();
    json["records"][0]["token"] = "secret".into();
    assert!(decode(&serde_json::to_vec(&json).unwrap()).is_err());
    assert!(decode(br#"{"version":2,"records":[]}"#).is_err());
    assert!(decode(&vec![b' '; MAX_BYTES + 1]).is_err());
    assert!(encode(&vec![record(); MAX_RECORDS + 1]).is_err());
    let mut invalid = record();
    invalid.origin = "https://localhost:1234/path".into();
    assert!(encode(&[invalid]).is_err());
    let mut invalid = record();
    invalid.binding.as_mut().unwrap().revision = None;
    assert!(encode(&[invalid]).is_err());
}
#[test]
fn store_roundtrips_without_overwriting_previous_or_partial_files() {
    let root = directory();
    let first = crate::surface_recovery_store::save(&root, &[record()]).unwrap();
    let original = std::fs::read(&first).unwrap();
    let second = crate::surface_recovery_store::save(&root, &[record()]).unwrap();
    assert_ne!(first, second);
    assert_eq!(std::fs::read(&first).unwrap(), original);
    assert_eq!(
        crate::surface_recovery_store::load(&second).unwrap(),
        vec![record()]
    );
    let partial = root.join("surface-recovery/surface-002.json");
    std::fs::write(&partial, b"{\"version\":1").unwrap();
    assert!(crate::surface_recovery_store::load(&partial).is_err());
    let third = crate::surface_recovery_store::save(&root, &[record()]).unwrap();
    assert!(third.ends_with("surface-003.json"));
    assert_eq!(std::fs::read(partial).unwrap(), b"{\"version\":1");
}
#[test]
fn store_quota_is_shared_across_concurrent_writers_and_never_evicts() {
    let root = directory();
    std::fs::create_dir_all(root.join("surface-recovery")).unwrap();
    for index in 0..127 {
        std::fs::write(
            root.join(format!("surface-recovery/surface-{index:03}.json")),
            b"old",
        )
        .unwrap();
    }
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let root = root.clone();
            std::thread::spawn(move || crate::surface_recovery_store::save(&root, &[record()]))
        })
        .collect();
    let results: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        std::fs::read(root.join("surface-recovery/surface-000.json")).unwrap(),
        b"old"
    );
    assert!(crate::surface_recovery_store::save(&root, &[record()]).is_err());
}
