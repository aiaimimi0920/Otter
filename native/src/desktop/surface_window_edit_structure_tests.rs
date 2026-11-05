//! 同场景 EDIT 挂载/rebind 改变 HWND 结构；空操作和纯同步不发送结构事件。
use super::*;
use windows::core::Interface;
fn binding(generation: u64) -> crate::surface_live_binding::Binding {
    let bytes = serde_json::json!({"instance":{"descriptor":{"instanceId":"instance:test","generation":generation},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":1}}}}).to_string();
    crate::surface_live_binding::Binding::from_mount(
        bytes.as_bytes(),
        "https://localhost:1234",
        "viewer:test",
        "node:test",
    )
    .unwrap()
}
#[test]
fn uia_edit_mount_and_rebind_notify_but_identical_binding_and_sync_do_not() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = Scene::parse(br#"{"id":"group","type":"column","children":[{"id":"field","type":"input","props":{"value":"hello"}}]}"#).unwrap();
    let window = SurfaceWindow::new(scene, 240, 200).unwrap();
    unsafe {
        SendMessageW(
            window.hwnd,
            WM_GETOBJECT,
            None,
            Some(LPARAM(UiaRootObjectId as isize)),
        );
    }
    let identity = window.accessibility_node("field").unwrap().runtime_id();
    let root = window.state.uia.borrow().as_ref().unwrap().provider();
    let mapping: IRawElementProviderHwndOverride = root.cast().unwrap();
    window.bind_edits(&binding(1)).unwrap();
    let old = window
        .state
        .host
        .borrow()
        .as_ref()
        .unwrap()
        .accessibility_edit_hwnd("field")
        .unwrap();
    let mut count = 0;
    flush_with(&window.state, |_| count += 1);
    assert_eq!(count, 1);
    window.bind_edits(&binding(1)).unwrap();
    window
        .state
        .run(|host| host.accessibility_bounds("field").map(|_| ()))
        .unwrap();
    flush_with(&window.state, |_| panic!("unchanged mount"));
    window.bind_edits(&binding(2)).unwrap();
    assert_eq!(
        window.accessibility_node("field").unwrap().runtime_id(),
        identity
    );
    let new = window
        .state
        .host
        .borrow()
        .as_ref()
        .unwrap()
        .accessibility_edit_hwnd("field")
        .unwrap();
    unsafe {
        // HWND 数值允许复用；只对不再是当前控件的数值断言 NULL。
        if old != new {
            let mut output = std::ptr::dangling_mut();
            assert_eq!(
                (mapping.vtable().GetOverrideProviderForHwnd)(mapping.as_raw(), old, &mut output),
                S_OK
            );
            assert!(output.is_null());
        }
        assert!(mapping.GetOverrideProviderForHwnd(new).is_ok());
    }
    flush_with(&window.state, |_| count += 1);
    assert_eq!(count, 2);
    window.state.run(|_| Ok(())).unwrap();
    flush_with(&window.state, |_| panic!("drained change"));
}
