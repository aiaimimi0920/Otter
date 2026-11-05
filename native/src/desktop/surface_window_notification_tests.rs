//! 合并、忙时延后、身份撤销和通知回调重入；不依赖系统 listener 是否存在。
use super::*;
use windows::Win32::UI::Accessibility::*;
fn scene(value: bool) -> Scene {
    Scene::parse(
        serde_json::json!({"id":"root","type":"column","children":[
            {"id":"a","type":"switch","props":{"value":value},"events":{"change":"change"}},
            {"id":"b","type":"switch","props":{"value":value},"events":{"change":"change"}}
        ]})
        .to_string()
        .as_bytes(),
    )
    .unwrap()
}
fn binding() -> crate::surface_live_binding::Binding {
    crate::surface_live_binding::Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":1}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap()
}
fn mount() -> SurfaceWindow {
    let window = SurfaceWindow::new(scene(false), 240, 200).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    unsafe {
        SendMessageW(
            window.hwnd,
            WM_GETOBJECT,
            Some(WPARAM(0)),
            Some(LPARAM(UiaRootObjectId as isize)),
        );
    }
    window
}
fn toggle(window: &SurfaceWindow, id: &str) {
    window
        .state
        .run(|host| {
            assert!(host.accessibility_commit_toggle(id)?);
            Ok(())
        })
        .unwrap();
}
fn identity(provider: &IRawElementProviderSimple) -> String {
    unsafe {
        provider
            .GetPropertyValue(UIA_AutomationIdPropertyId)
            .unwrap()
            .to_string()
    }
}
#[test]
fn uia_notification_coalesces_round_trip_and_reposts_after_busy_flush() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    toggle(&window, "a");
    assert_eq!(window.state.notifications.borrow().values.len(), 1);
    assert!(window.state.notifications.borrow().posted);
    toggle(&window, "a");
    assert!(window.state.notifications.borrow().values.is_empty());
    flush_with(&window.state, |_, _, _| panic!("net-zero notification"));
    toggle(&window, "a");
    let host = window.state.host.borrow_mut().take();
    flush_with(&window.state, |_, _, _| panic!("busy notification"));
    assert!(!window.state.notifications.borrow().posted);
    assert_eq!(window.state.notifications.borrow().values.len(), 1);
    *window.state.host.borrow_mut() = host;
    window.state.run(|_| Ok(())).unwrap();
    assert!(window.state.notifications.borrow().posted);
    let mut events = Vec::new();
    flush_with(&window.state, |provider, before, after| {
        events.push((identity(provider), before, after))
    });
    assert_eq!(events, [("a".into(), false, true)]);
}
#[test]
fn uia_notification_reentrant_flush_cannot_send_stale_batch_after_new_state() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    toggle(&window, "a");
    toggle(&window, "b");
    let mut events = Vec::new();
    flush_with(&window.state, |provider, before, after| {
        let id = identity(provider);
        events.push((id.clone(), before, after));
        if id == "a" {
            toggle(&window, "b");
            flush_with(&window.state, |_, _, _| panic!("nested notification"));
        }
    });
    assert_eq!(events, [("a".into(), false, true)]);
    assert!(window.state.notifications.borrow().posted);
    flush_with(&window.state, |provider, before, after| {
        events.push((identity(provider), before, after))
    });
    assert_eq!(
        events,
        [("a".into(), false, true), ("b".into(), true, false)]
    );
}
#[test]
fn uia_notification_replacement_drops_old_identity_and_settlement_tracks_real_change() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    toggle(&window, "a");
    let events = window.prepare_edit(&binding(), 1).unwrap();
    assert_eq!(events.len(), 1);
    window.replace(scene(false)).unwrap();
    assert!(window.state.notifications.borrow().values.is_empty());
    flush_with(&window.state, |_, _, _| panic!("revoked notification"));
    window.settle_edits(&events).unwrap();
    let mut values = Vec::new();
    flush_with(&window.state, |provider, before, after| {
        values.push((identity(provider), before, after))
    });
    assert_eq!(values, [("a".into(), true, false)]);
    toggle(&window, "a");
    window.close().unwrap();
    flush_with(&window.state, |_, _, _| panic!("closed notification"));
}
