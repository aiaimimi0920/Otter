//! Identity-read failure before privilege reduction must block the online chain; distinguish Missing compatibility from exclusive-lock/corruption failures.
use super::*;
use std::os::windows::fs::OpenOptionsExt;

#[test]
fn identity_lock_or_corruption_cannot_skip_offline_invalidation() {
    let root = std::env::temp_dir().join(format!(
        "issuer-resume-{:?}",
        unsafe { windows::Win32::System::Com::CoCreateGuid() }.unwrap()
    ));
    std::fs::create_dir(&root).unwrap();
    let mut authority = Authority::new(document()).unwrap();
    let (_grant, lease) = lease(ORIGIN);
    let authorization = authority.authorize(ORIGIN, lease).unwrap();
    assert!(authorization
        .invalidate_offline(&root, "vendor/demo")
        .is_ok());
    let store = crate::device_identity_store::Store::new(&root).unwrap();
    let mut loaded = store.load_or_create().unwrap();
    store.bind_and_save(&mut loaded, "device:1").unwrap();
    let lock = std::fs::OpenOptions::new()
        .write(true)
        .share_mode(0)
        .open(root.join(".device-identity.lock"))
        .unwrap();
    assert!(matches!(
        store.load_existing(),
        Err(crate::device_identity_store::Error::Lock)
    ));
    assert_eq!(
        authorization.invalidate_offline(&root, "vendor/demo"),
        Err(Error::Trust)
    );
    drop(lock);
    assert!(authorization
        .invalidate_offline(&root, "vendor/demo")
        .is_ok());
    std::fs::write(root.join("device-identity.json"), b"corrupt test identity").unwrap();
    assert_eq!(
        authorization.invalidate_offline(&root, "vendor/demo"),
        Err(Error::Trust)
    );
}
