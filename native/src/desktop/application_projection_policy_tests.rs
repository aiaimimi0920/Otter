//! 隔离文件/真实 worker 与事务回归；所有合成输入保留在 linshi，不发现真实 profile。
use super::*;
#[path = "application_projection_pause_tests.rs"]
mod pause;
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

fn root() -> PathBuf {
    let root = PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi").join(format!(
        "otter-receive-policy-{:?}",
        unsafe { windows::Win32::System::Com::CoCreateGuid() }.unwrap()
    ));
    std::fs::create_dir(&root).unwrap();
    root
}
fn path(root: &Path) -> PathBuf {
    root.join("projection-receive-policy.json")
}
fn settle(owner: &mut Owner, consent: &mut Consent) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        owner.poll(consent);
        if owner.job.is_none() {
            break;
        }
        assert!(Instant::now() < deadline, "receive policy save deadline");
        std::thread::yield_now();
    }
}
#[test]
fn receive_policy_persists_before_granting_auto_and_reopens_all_values() {
    let root = root();
    let mut consent = Consent::default();
    let mut owner = Owner::load(&root, &AtomicBool::new(false));
    owner.poll(&mut consent);
    assert_eq!(consent.policy(), Policy::Confirm);
    assert!(!path(&root).exists());
    for policy in [Policy::Auto, Policy::Reject, Policy::Confirm] {
        let before = consent.policy();
        owner.request(policy, &mut consent, Arc::new(|| {}));
        assert_eq!(consent.policy(), before);
        assert!(consent.saving);
        assert!(consent.stamp().is_none());
        settle(&mut owner, &mut consent);
        assert!(!consent.saving);
        assert_eq!(consent.policy(), policy);
        let mut reopened = Owner::load(&root, &AtomicBool::new(false));
        let mut next = Consent::default();
        reopened.poll(&mut next);
        assert_eq!(next.policy(), policy);
    }
}
#[test]
fn receive_policy_conflict_failure_never_grants_or_overwrites_competitor() {
    let root = root();
    let mut owner = Owner::load(&root, &AtomicBool::new(false));
    let mut consent = Consent::default();
    owner.poll(&mut consent);
    let old = consent.stamp().unwrap();
    std::fs::write(path(&root), b"competing bytes").unwrap();
    owner.request(Policy::Auto, &mut consent, Arc::new(|| {}));
    settle(&mut owner, &mut consent);
    assert_eq!(consent.policy(), Policy::Confirm);
    assert_eq!(consent.status, "保存接收策略失败");
    assert!(!consent.matches(&old));
    assert_eq!(std::fs::read(path(&root)).unwrap(), b"competing bytes");
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
}
#[test]
fn receive_policy_malformed_unknown_oversized_and_locked_files_reject_without_rewrite() {
    use std::{fs::OpenOptions, os::windows::fs::OpenOptionsExt};
    for bytes in [
        b"{".to_vec(),
        br#"{"version":2,"policy":"auto"}"#.to_vec(),
        vec![b'x'; 1025],
    ] {
        let root = root();
        std::fs::write(path(&root), &bytes).unwrap();
        let mut owner = Owner::load(&root, &AtomicBool::new(false));
        let mut consent = Consent::default();
        owner.poll(&mut consent);
        assert_eq!(consent.policy(), Policy::Reject);
        owner.request(Policy::Auto, &mut consent, Arc::new(|| {}));
        assert!(owner.job.is_none());
        assert_eq!(std::fs::read(path(&root)).unwrap(), bytes);
    }
    let root = root();
    std::fs::write(path(&root), br#"{"version":1,"policy":"auto"}"#).unwrap();
    let locked = OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(path(&root))
        .unwrap();
    let file = file::File::load(&root, &AtomicBool::new(false));
    assert_eq!(file.policy(), Policy::Reject);
    assert!(!file.writable());
    drop(locked);
    assert_eq!(
        file::File::load(&root, &AtomicBool::new(false)).policy(),
        Policy::Auto
    );
}
#[test]
fn receive_policy_explicit_repair_preserves_extensions_and_dropped_stage_never_publishes() {
    let root = root();
    let bytes = br#"{"version":1,"policy":null,"future":{"keep":[3,4]}}"#;
    std::fs::write(path(&root), bytes).unwrap();
    let file = file::File::load(&root, &AtomicBool::new(false));
    assert_eq!(file.policy(), Policy::Reject);
    let staged = file
        .clone()
        .prepare(Policy::Auto, &AtomicBool::new(false))
        .unwrap();
    drop(staged);
    assert_eq!(std::fs::read(path(&root)).unwrap(), bytes);
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    assert!(file
        .clone()
        .prepare(Policy::Auto, &AtomicBool::new(true))
        .is_err());
    let (next, warning) = file
        .prepare(Policy::Confirm, &AtomicBool::new(false))
        .unwrap()
        .commit()
        .unwrap();
    assert!(!warning);
    assert_eq!(next.policy(), Policy::Confirm);
    let value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path(&root)).unwrap()).unwrap();
    assert_eq!(value["future"], serde_json::json!({"keep":[3,4]}));
}

#[test]
fn receive_policy_recovery_and_cancel_do_not_reenable_auto_or_publish_unclaimed_save() {
    let root = root();
    let recovery = root.join(format!(
        "projection-receive-policy.json.otter-recovery-{:?}.json",
        unsafe { windows::Win32::System::Com::CoCreateGuid() }.unwrap()
    ));
    let bytes = br#"{"version":1,"policy":"auto"}"#;
    std::fs::write(&recovery, bytes).unwrap();
    let file = file::File::load(&root, &AtomicBool::new(false));
    assert_eq!(file.policy(), Policy::Reject);
    assert!(!file.writable());
    assert_eq!(std::fs::read(recovery).unwrap(), bytes);
    assert!(!path(&root).exists());
    let other = super::tests::root();
    // 命名设置中的类似字段不参与设备同意解析，更不能作为 auto 的迁移输入。
    std::fs::write(other.join("app-settings.json"), bytes).unwrap();
    let mut owner = Owner::load(&other, &AtomicBool::new(false));
    let mut consent = Consent::default();
    owner.poll(&mut consent);
    assert_eq!(consent.policy(), Policy::Confirm);
    owner.request(Policy::Auto, &mut consent, Arc::new(|| {}));
    drop(owner);
    assert_eq!(consent.policy(), Policy::Confirm);
    assert!(!path(&other).exists());
    assert_eq!(std::fs::read_dir(&other).unwrap().count(), 1);
}
