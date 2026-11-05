//! 场景通知合并、忙时重排、回调再替换与关闭生命周期。
use super::*;
fn scene(id: &str) -> Scene {
    Scene::parse(
        serde_json::json!({"id":id,"type":"text","props":{"text":id}})
            .to_string()
            .as_bytes(),
    )
    .unwrap()
}
fn mount() -> SurfaceWindow {
    let window = SurfaceWindow::new(scene("old"), 240, 200).unwrap();
    unsafe {
        SendMessageW(
            window.hwnd,
            WM_GETOBJECT,
            None,
            Some(LPARAM(UiaRootObjectId as isize)),
        );
    }
    window
}
#[test]
fn uia_structure_notifications_coalesce_busy_and_revoke_old_nodes() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    let old = window.accessibility_node("old").unwrap();
    window.replace(scene("middle")).unwrap();
    window.replace(scene("new")).unwrap();
    assert!(old.read().is_none());
    let host = window.state.host.borrow_mut().take();
    flush_with(&window.state, |_| panic!("busy"));
    assert!(window.state.structure_notifications.dirty.get());
    *window.state.host.borrow_mut() = host;
    window.state.run(|_| Ok(())).unwrap();
    assert!(window.state.structure_notifications.posted.get());
    let mut count = 0;
    flush_with(&window.state, |_| {
        count += 1;
        assert!(window.accessibility_node("new").is_some());
        assert!(window.accessibility_node("middle").is_none());
    });
    flush_with(&window.state, |_| panic!("duplicate"));
    assert_eq!(count, 1);
}
#[test]
fn uia_structure_notifications_reentrant_replace_delays_and_close_suppresses() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    window.replace(scene("first")).unwrap();
    flush_with(&window.state, |_| {
        window.replace(scene("second")).unwrap();
        flush_with(&window.state, |_| panic!("nested"));
    });
    assert!(window.state.structure_notifications.posted.get());
    let mut count = 0;
    flush_with(&window.state, |_| {
        count += 1;
        assert!(window.accessibility_node("second").is_some());
    });
    assert_eq!(count, 1);
    window.replace(scene("closed")).unwrap();
    window.close().unwrap();
    flush_with(&window.state, |_| panic!("closed"));
}
#[test]
fn uia_structure_failed_replace_still_reports_revoked_child_identity() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    let old = window.accessibility_node("old").unwrap();
    let host = window.state.host.borrow_mut().take();
    assert!(window.replace(scene("rejected")).is_err());
    assert!(old.read().is_none());
    assert!(window.state.structure_notifications.dirty.get());
    *window.state.host.borrow_mut() = host;
    window.state.run(|_| Ok(())).unwrap();
    let mut count = 0;
    flush_with(&window.state, |_| count += 1);
    assert_eq!(count, 1);
    // 失败保持不可用，不把原 Host 内容伪装成仍有效的旧 provider 树。
    assert!(window.accessibility_node("old").is_none());
    assert!(window.accessibility_node("rejected").is_none());
}
