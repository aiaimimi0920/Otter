//! 显式配置、真实设置事务与原租约撤销；不读默认 profile，也不信任网络 key。
use super::*;
use crate::{
    device_authorization_lease::Grant, device_identity::Identity, device_session_attempt::Attempt,
};
use std::sync::atomic::AtomicBool;
#[path = "art_issuer_resume_tests.rs"]
mod resume_tests;
const ORIGIN: &str = "https://loom.example";
fn document() -> Document {
    Document {
        issuers: vec![Entry {
            origin: ORIGIN.into(),
            key_id: "issuer:1".into(),
            public_key: STANDARD.encode([7; 32]),
        }],
    }
}
fn lease(origin: &str) -> (Grant, Arc<Lease>) {
    let mut identity = Identity::generate().unwrap();
    identity.bind("device:1").unwrap();
    let session = Attempt::new(&identity, origin)
        .unwrap()
        .challenge(
            200,
            br#"{"deviceId":"device:1","challengeId":"id","challenge":"value"}"#,
        )
        .unwrap()
        .finish(
            200,
            br#"{"deviceId":"device:1","token":"test-only","expiresAtMs":9007199254740991}"#,
            1,
        )
        .unwrap();
    let mut owner = Grant::new(session);
    let lease = Arc::new(owner.lease(origin, 1).unwrap());
    (owner, lease)
}
#[test]
fn configuration_is_exact_bounded_and_never_accepts_aliases() {
    assert!(document().validate().is_ok());
    for origin in [
        "https://loom.example/",
        "https://loom.example/path",
        "https://user@loom.example",
        "http://remote.example",
    ] {
        let mut d = document();
        d.issuers[0].origin = origin.into();
        assert!(d.validate().is_err(), "{origin}");
    }
    let mut d = document();
    d.issuers.push(d.issuers[0].clone());
    assert!(d.validate().is_err());
    let mut d = document();
    d.issuers[0].public_key = STANDARD.encode([7; 31]);
    assert!(d.validate().is_err());
    let mut d = document();
    d.issuers[0].key_id = "bad\nkey".into();
    assert!(d.validate().is_err());
    let mut d = document();
    d.issuers = vec![d.issuers[0].clone(); 33];
    assert!(d.validate().is_err());
}
#[test]
fn rotation_revocation_and_drop_stop_only_owned_origin_leases() {
    let mut d = document();
    let mut other = d.issuers[0].clone();
    other.origin = "https://other.example".into();
    d.issuers.push(other);
    let mut trust = Authority::new(d.clone()).unwrap();
    let (_a, a) = lease(ORIGIN);
    let (_b, b) = lease("https://other.example");
    let _auth_a = trust.authorize(ORIGIN, a.clone()).unwrap();
    let _auth_b = trust.authorize("https://other.example", b.clone()).unwrap();
    trust.replace(d.clone()).unwrap();
    assert!(!a.is_revoked());
    let mut invalid = d.clone();
    invalid.issuers[0].public_key = "broken".into();
    assert!(trust.replace(invalid).is_err());
    assert!(!a.is_revoked());
    d.issuers[0].public_key = STANDARD.encode([8; 32]);
    trust.replace(d).unwrap();
    assert!(a.is_revoked());
    assert!(!b.is_revoked());
    drop(trust);
    assert!(b.is_revoked());
}
#[test]
fn absent_origin_and_cross_origin_lease_cannot_be_authorized() {
    let mut trust = Authority::new(document()).unwrap();
    let (_owner, lease) = lease("https://other.example");
    assert!(trust.authorize(ORIGIN, lease.clone()).is_err());
    assert!(trust.authorize("https://other.example", lease).is_err());
}
#[test]
fn settings_commit_reload_and_stale_write_preserve_existing_configuration() {
    let root = std::env::temp_dir().join(format!(
        "otter-art-trust-{:?}",
        unsafe { windows::Win32::System::Com::CoCreateGuid() }.unwrap()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("settings.json");
    let bytes = br#"{"schemaVersion":2,"extension":{"keep":true}}"#.to_vec();
    std::fs::write(&path, &bytes).unwrap();
    let source = crate::settings_source::Source::new(path.clone(), bytes).unwrap();
    assert!(source
        .document()
        .art_issuer_trust()
        .unwrap()
        .issuers
        .is_empty());
    let stop = AtomicBool::new(false);
    let transaction =
        crate::settings_update::Prepared::prepare_issuer_trust(&source, document(), &stop).unwrap();
    let (next, _published) = transaction.commit(&stop).unwrap();
    let loaded =
        crate::settings_source::Source::new(path.clone(), std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(loaded.document().art_issuer_trust().unwrap(), document());
    assert!(Authority::from_settings(&loaded).is_ok());
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(value["extension"]["keep"], true);
    assert!(crate::settings_update::Prepared::prepare_issuer_trust(
        &source,
        Document::default(),
        &stop
    )
    .is_err());
    let mut trust = Authority::from_settings(&next).unwrap();
    let (_owner, lease) = lease(ORIGIN);
    let _authorization = trust.authorize(ORIGIN, lease.clone()).unwrap();
    let transaction =
        crate::settings_update::Prepared::prepare_issuer_trust(&next, Document::default(), &stop)
            .unwrap();
    let (removed, _) = transaction.commit(&stop).unwrap();
    trust
        .replace(removed.document().art_issuer_trust().unwrap())
        .unwrap();
    assert!(lease.is_revoked());
    // 测试环境 TEMP/TMP 必须由本批 gate 指向 linshi；只回收本测试自建目录。
    std::fs::remove_dir_all(root).unwrap();
}
