//! 隔离profile存储测试；保留失败现场，不操作真实用户配置。
use super::*;
fn root() -> PathBuf {
    let root=PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-parallel-20261029/main-art-offline-resume-1/runtime")
        .join(format!("store-{:?}",unsafe {windows::Win32::System::Com::CoCreateGuid()}.unwrap()));
    std::fs::create_dir_all(&root).unwrap();
    root
}
fn slot(root: &Path) -> Slot {
    Slot::open(
        root,
        "https://loom.example",
        "vendor/demo",
        &crate::live_relay_http::art_prepare::Pin {
            key_id: "fixture".into(),
            public_key: [7; 32],
        },
        "device-public-key",
    )
    .unwrap()
}
fn envelope() -> crate::art_execution_grant::wire::Envelope {
    crate::art_execution_grant::wire::Envelope {
        algorithm: "ed25519".into(),
        key_id: "fixture".into(),
        payload: "synthetic-storage-only".into(),
        signature: "fixture".into(),
    }
}
#[test]
fn persisted_resume_advances_startup_high_water_and_clock_rollback_is_terminal() {
    let root = root();
    {
        let slot = slot(&root);
        slot.save(envelope(), 1000, &|| false).unwrap();
    }
    {
        let slot = slot(&root);
        assert!(!slot.take(2000, &|| false).unwrap().is_empty());
    }
    {
        let slot = slot(&root);
        assert!(slot.take(1999, &|| false).is_err());
    }
    assert!(slot(&root).take(3000, &|| false).is_err());
}
#[test]
fn explicit_invalidation_and_origin_cutoff_survive_owner_recreation() {
    let root = root();
    {
        let slot = slot(&root);
        slot.save(envelope(), 1000, &|| false).unwrap();
    }
    Slot::invalidate_origin(&root, "https://loom.example", 2000, &|| false).unwrap();
    assert_eq!(Slot::cutoff(&root, "https://loom.example").unwrap(), 2000);
    assert_eq!(Slot::cutoff(&root, "https://other.example").unwrap(), 0);
    {
        let slot = slot(&root);
        slot.invalidate(3000, &|| false).unwrap();
    }
    assert!(slot(&root).take(4000, &|| false).is_err());
}
#[test]
fn offline_slots_are_exclusive_bounded_cancelled_and_do_not_delete_unknown_files() {
    let root = root();
    let held = slot(&root);
    assert!(Slot::open(
        &root,
        "https://loom.example",
        "vendor/demo",
        &crate::live_relay_http::art_prepare::Pin {
            key_id: "fixture".into(),
            public_key: [7; 32]
        },
        "device-public-key"
    )
    .is_err());
    held.save(envelope(), 1000, &|| false).unwrap();
    assert!(held.invalidate(2000, &|| true).is_err());
    assert!(held.take(1001, &|| false).is_ok());
    for i in 0..32 {
        std::fs::write(held.root.join(format!("unknown-{i}")), b"retained").unwrap();
    }
    assert!(held.invalidate(3000, &|| false).is_err());
    assert_eq!(
        std::fs::read(held.root.join("unknown-0")).unwrap(),
        b"retained"
    );
}
