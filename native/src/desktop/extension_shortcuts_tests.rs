//! 注册生命周期/失败/旧事件；注入注册函数，不操作用户热键。
use super::*;
use serde_json::json;
fn snapshot(command: &str) -> Arc<Snapshot> {
    let mut v = crate::loom_extension_snapshot::tests::fixture();
    v["contributions"]["commands"][0]["commandId"] = command.into();
    v["contributions"]["shortcuts"] = json!([{"id":"ocr-key","commandId":command,
        "pluginId":"neuro.official/ocr","scopeId":"scope:ocr","payload":{"keys":"Ctrl+4","global":true}}]);
    Arc::new(Snapshot::parse(&serde_json::to_vec(&v).unwrap()).unwrap())
}
#[test]
fn unchanged_binding_keeps_registration_but_replacement_and_revocation_retire_old_message_ids() {
    let mut o = Owner::new(Default::default());
    let mut events = Vec::new();
    o.sync_with(Some(snapshot("first")), |id, _, on| {
        events.push((id, on));
        Ok(())
    })
    .unwrap();
    let old = events[0].0;
    let lp = LPARAM(((b'4' as u32) << 16 | 2) as isize);
    assert_eq!(o.global(WPARAM(old as usize), lp).unwrap().command, "first");
    o.sync_with(Some(snapshot("first")), |_, _, _| {
        panic!("unchanged binding was registered twice")
    })
    .unwrap();
    o.sync_with(Some(snapshot("second")), |id, _, on| {
        events.push((id, on));
        Ok(())
    })
    .unwrap();
    assert_eq!(events[1], (old, false));
    assert!(o.global(WPARAM(old as usize), lp).is_none());
    assert_ne!(events[2].0, old);
    o.sync_with(None, |_, _, on| {
        assert!(!on);
        Ok(())
    })
    .unwrap();
    assert!(o.bindings.is_empty());
}
#[test]
fn failed_registration_is_not_claimed_global_and_failed_unregistration_cannot_keep_a_route() {
    let mut o = Owner::new(Default::default());
    let s = snapshot("first");
    o.sync_with(Some(s.clone()), |_, _, _| {
        Err(windows::core::Error::new(E_FAIL, "occupied"))
    })
    .unwrap_err();
    assert!(o.local(b'4' as u32, || 2).is_some());
    assert!(o.local(b'4' as u32, || 4).is_none());
    assert!(o
        .local(b'5' as u32, || panic!("unbound key queried modifiers"))
        .is_none());
    assert!(o
        .global(
            WPARAM(o.bindings[0].id as usize),
            LPARAM(((b'4' as u32) << 16 | 2) as isize)
        )
        .is_none());
    o.sync_with(Some(s), |_, _, _| panic!("unexpected retry"))
        .unwrap();
    o.sync_with(None, |_, _, _| panic!("unregistered entry released OS key"))
        .unwrap();
    o.sync_with(Some(snapshot("second")), |_, _, _| Ok(()))
        .unwrap();
    o.sync_with(None, |_, _, _| {
        Err(windows::core::Error::new(E_FAIL, "release failed"))
    })
    .unwrap_err();
    assert!(o
        .local(b'4' as u32, || panic!("retired binding queried modifiers"))
        .is_none());
    assert!(o.bindings[0].definition.is_none());
    // 测试回收注入的虚拟注册，不把它交给真实 OS 释放。
    o.sync_with(Some(snapshot("third")), |_, _, _| Ok(()))
        .unwrap();
    o.sync_with(None, |_, _, _| Ok(())).unwrap();
}
