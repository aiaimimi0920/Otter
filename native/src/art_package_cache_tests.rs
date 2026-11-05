use super::*;
fn root() -> std::path::PathBuf {
    let root=std::path::PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi/otter-native-parallel-20261029/main-art-package-resources-1").join(format!("cache-{:?}", unsafe {windows::Win32::System::Com::CoCreateGuid()}.unwrap()));
    std::fs::create_dir(&root).unwrap();
    root
}
fn resource(bytes: &[u8]) -> Resource {
    Resource {
        id: "blob".into(),
        mime_type: "image/png".into(),
        size: bytes.len() as u64,
        digest: format!("{:x}", Sha256::digest(bytes)),
        dimensions: [1, 1],
    }
}
#[test]
fn resources_share_original_entry_and_staging_limits_and_roundtrip_large_chunks() {
    let root = root();
    let store = Store::new(&root);
    let bytes = vec![153; MAX_SOURCE + 17];
    let r = resource(&bytes);
    store.put(&[1; 32], b"source;", &|| false).unwrap();
    put(&store, &[1; 32], &r, &bytes, &|| false).unwrap();
    assert_eq!(
        load(&Store::new(&root), &[1; 32], &r).unwrap().unwrap(),
        bytes
    );
    assert!(load(&store, &[2; 32], &r).unwrap().is_none());
    for e in std::fs::read_dir(root.join("art-source-cache")).unwrap() {
        assert!(e.unwrap().metadata().unwrap().len() <= MAX_SOURCE as u64);
    }
    for i in 2..31 {
        store.put(&[i; 32], b"old;", &|| false).unwrap();
    }
    assert!(store.put(&[31; 32], b"overflow;", &|| false).is_err());
    assert_eq!(load(&store, &[1; 32], &r).unwrap().unwrap(), bytes);
}
#[test]
fn partial_cancelled_corrupt_or_mismatched_resource_is_never_restored() {
    let root = root();
    let store = Store::new(&root);
    let bytes = vec![17; MAX_SOURCE + 1];
    let r = resource(&bytes);
    let calls = std::cell::Cell::new(0);
    assert!(put(&store, &[1; 32], &r, &bytes, &|| {
        let c = calls.get();
        calls.set(c + 1);
        c >= 3
    })
    .is_err());
    assert!(load(&store, &[1; 32], &r).unwrap().is_none());
    put(&store, &[1; 32], &r, &bytes, &|| false).unwrap();
    let k = key(&[1; 32], &r, 0);
    let name = format!("{:x}.blob", Sha256::digest(k));
    std::fs::write(
        root.join("art-source-cache").join(name),
        vec![18; MAX_SOURCE],
    )
    .unwrap();
    assert!(load(&store, &[1; 32], &r).is_err());
    assert!(put(&store, &[1; 32], &r, b"wrong", &|| false).is_err());
}
