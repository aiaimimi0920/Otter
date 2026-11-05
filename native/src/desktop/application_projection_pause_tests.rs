//! 独立暂停字段沿原事务保存；旧文档兼容、失败减权与重新打开都不改设备 NEW 同意。
use super::*;

#[test]
fn receive_policy_pause_persists_reopens_and_preserves_consent_and_extensions() {
    let root = root();
    std::fs::write(
        path(&root),
        br#"{"version":1,"policy":"auto","future":{"keep":1}}"#,
    )
    .unwrap();
    let mut owner = Owner::load(&root, &AtomicBool::new(false));
    let mut consent = Consent::default();
    owner.poll(&mut consent);
    assert!(!consent.paused());
    assert_eq!(consent.policy(), Policy::Auto);
    for paused in [true, false] {
        let old = consent.participation();
        owner.request(Change::Paused(paused), &mut consent, Arc::new(|| {}));
        assert!(consent.saving);
        assert_eq!(consent.paused(), !paused);
        if let Some(old) = old {
            assert!(!consent.participates(&old));
        }
        settle(&mut owner, &mut consent);
        assert_eq!(consent.paused(), paused);
        assert_eq!(consent.policy(), Policy::Auto);
        let mut reopened = Owner::load(&root, &AtomicBool::new(false));
        let mut next = Consent::default();
        reopened.poll(&mut next);
        assert_eq!(next.paused(), paused);
        assert_eq!(next.policy(), Policy::Auto);
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path(&root)).unwrap()).unwrap();
        assert_eq!(value["future"], serde_json::json!({"keep":1}));
    }
}

#[test]
fn receive_policy_pause_write_conflict_invalidates_old_work_without_overwriting_competitor() {
    let root = root();
    let mut owner = Owner::load(&root, &AtomicBool::new(false));
    let mut consent = Consent::default();
    owner.poll(&mut consent);
    let old = consent.participation().unwrap();
    std::fs::write(path(&root), b"competitor").unwrap();
    owner.request(Change::Paused(true), &mut consent, Arc::new(|| {}));
    settle(&mut owner, &mut consent);
    assert!(!consent.paused());
    assert!(!consent.participates(&old));
    assert_eq!(consent.policy(), Policy::Confirm);
    assert_eq!(std::fs::read(path(&root)).unwrap(), b"competitor");
    assert_eq!(consent.status, "保存接收策略失败");
    let mut reopened = Owner::load(&root, &AtomicBool::new(false));
    reopened.poll(&mut consent);
    assert!(consent.paused());
    assert_eq!(consent.policy(), Policy::Reject);
}
