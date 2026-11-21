//! 本机管理通道和自己的公钥授权边界；不访问真实用户 profile。
use super::*;
#[test]
fn local_loom_renewal_preserves_inflight_lease_but_explicit_revoke_cancels_both() {
    use crate::device_session_client::tests::session;
    let mut owner = crate::device_authorization_owner::Owner::new();
    let origin = "https://localhost:443";
    owner.install(origin, session(origin, 10_000), 0).unwrap();
    let old = owner.lease(origin, 0).unwrap();
    owner.install(origin, session(origin, 20_000), 0).unwrap();
    let new = owner.lease(origin, 0).unwrap();
    assert!(!old.is_revoked());
    assert!(old.validate(origin, 10_000).is_err());
    assert_eq!(new.expires_at_ms(), 20_000);
    owner.invalidate(origin).unwrap();
    assert!(old.is_revoked() && new.is_revoked());
}
#[test]
fn local_loom_never_accepts_remote_administrator_target() {
    for origin in [
        "http://192.168.1.2:8765",
        "https://127.0.0.1:8765",
        "http://example.test",
    ] {
        assert!(matches!(
            http::Local::new(crate::loom_media_target::Target::new(
                origin.into(),
                Some("secret".into())
            )),
            Err(Error::LocalOnly)
        ));
    }
    assert!(http::Local::new(crate::loom_media_target::Target::new(
        "http://localhost:8765".into(),
        Some("secret".into())
    ))
    .is_ok());
}
fn registry(approval: &str, enabled: bool, duplicate: bool) -> Value {
    let device = json!({"id":"device:own","publicKey":"own","approval":approval,"enabled":enabled});
    json!({"devices":[device.clone()],"pending":if duplicate { vec![device] } else { vec![] }})
}
#[test]
fn local_loom_does_not_revive_revoked_or_ambiguous_identity() {
    for (approval, enabled, duplicate) in [
        ("rejected", true, false),
        ("approved", false, false),
        ("approved", true, true),
    ] {
        assert!(matches!(
            own_device(registry(approval, enabled, duplicate), "own"),
            Err(Error::DeviceDenied)
        ));
    }
    assert!(own_device(registry("approved", true, false), "other")
        .unwrap()
        .is_none());
    assert_eq!(
        own_device(registry("approved", true, false), "own")
            .unwrap()
            .unwrap()
            .approval,
        "approved"
    );
}
