//! 隐藏 HWND 的语义句柄生命周期；不是 UIA 客户端验收。
use super::*;

fn scene() -> Scene {
    Scene::parse(br#"{"id":"root","type":"text","props":{"text":"Text"}}"#).unwrap()
}

#[test]
fn accessibility_window_replace_close_and_drop_revoke_handles() {
    let _com = super::super::Apartment::new().unwrap();
    let window = SurfaceWindow::new(scene(), 200, 140).unwrap();
    let old = window.accessibility_node("root").unwrap();
    window.replace(scene()).unwrap();
    assert!(old.read().is_none());
    let current = window.accessibility_node("root").unwrap();
    assert!(current.read().is_some());
    window.close().unwrap();
    assert!(current.read().is_none());
    assert!(window.accessibility_node("root").is_none());
    let window = SurfaceWindow::new(scene(), 200, 140).unwrap();
    let dropped = window.accessibility_node("root").unwrap();
    drop(window);
    assert!(dropped.read().is_none());
}

#[test]
fn accessibility_busy_replace_fails_closed_then_recovers() {
    let _com = super::super::Apartment::new().unwrap();
    let window = SurfaceWindow::new(scene(), 200, 140).unwrap();
    let old = window.accessibility_node("root").unwrap();
    let host = window.state.host.borrow_mut().take();
    assert!(window.replace(scene()).is_err());
    assert!(old.read().is_none());
    assert!(window.accessibility_node("root").is_none());
    *window.state.host.borrow_mut() = host;
    window.replace(scene()).unwrap();
    assert!(window.accessibility_node("root").unwrap().read().is_some());
    unsafe { DestroyWindow(window.hwnd).unwrap() };
    assert!(window.accessibility_node("root").is_none());
}
