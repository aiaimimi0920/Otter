//! 焦点通知的 owner/身份/重入回归；不依赖全局系统订阅者。
use super::*;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetFocus, SetFocus};
fn scene() -> Scene {
    Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"a","type":"button","props":{"label":"A"}},{"id":"b","type":"button","props":{"label":"B"}}]}"#).unwrap()
}
fn mount() -> SurfaceWindow {
    let window = SurfaceWindow::new(scene(), 240, 200).unwrap();
    window.set_input_policy(true, false).unwrap();
    window.show().unwrap();
    unsafe {
        SendMessageW(
            window.hwnd,
            WM_GETOBJECT,
            None,
            Some(LPARAM(UiaRootObjectId as isize)),
        );
        let _ = SetFocus(Some(window.hwnd));
        assert_eq!(GetFocus(), window.hwnd);
    }
    window
}
fn target(window: &SurfaceWindow, id: &str) {
    keyboard::set_focus(&window.state, Some(id.into()));
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
fn uia_focus_notifications_coalesce_and_requeue_busy_owner() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    target(&window, "a");
    target(&window, "b");
    let host = window.state.host.borrow_mut().take();
    flush_with(window.hwnd, &window.state, |_| panic!("busy"));
    assert!(window.state.focus_notifications.dirty.get());
    *window.state.host.borrow_mut() = host;
    window.state.run(|_| Ok(())).unwrap();
    assert!(window.state.focus_notifications.posted.get());
    let mut events = Vec::new();
    flush_with(window.hwnd, &window.state, |p| events.push(identity(p)));
    assert_eq!(events, ["b"]);
    target(&window, "b");
    flush_with(window.hwnd, &window.state, |_| panic!("duplicate"));
    unsafe {
        let _ = SetFocus(None);
        assert_eq!(GetFocus(), HWND::default());
    }
    flush_with(window.hwnd, &window.state, |_| panic!("lost focus"));
    unsafe {
        let _ = SetFocus(Some(window.hwnd));
        assert_eq!(GetFocus(), window.hwnd);
    }
    target(&window, "b");
    flush_with(window.hwnd, &window.state, |p| events.push(identity(p)));
    assert_eq!(events, ["b", "b"]);
}
#[test]
fn uia_focus_notifications_reentrant_replace_and_close_use_current_identity() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    target(&window, "a");
    let mut events = Vec::new();
    flush_with(window.hwnd, &window.state, |p| {
        events.push(identity(p));
        target(&window, "b");
        flush_with(window.hwnd, &window.state, |_| panic!("nested"));
    });
    flush_with(window.hwnd, &window.state, |p| events.push(identity(p)));
    assert_eq!(events, ["a", "b"]);
    window.replace(scene()).unwrap();
    flush_with(window.hwnd, &window.state, |p| events.push(identity(p)));
    assert_eq!(events, ["a", "b", "b"]);
    target(&window, "a");
    window.close().unwrap();
    flush_with(window.hwnd, &window.state, |_| panic!("closed"));
}
