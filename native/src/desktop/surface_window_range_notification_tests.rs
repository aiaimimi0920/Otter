//! 数值通知合并、忙时重排、替换与关闭，不依赖系统 listener。
use super::*;
use windows::Win32::UI::Accessibility::*;
fn mount() -> SurfaceWindow {
    let window = super::super::uia::range_tests::mount();
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
fn set(window: &SurfaceWindow, value: f64) {
    window
        .state
        .run(|host| host.accessibility_set_slider_range("slider", value))
        .unwrap();
}
fn flush(state: &State, notify: impl FnMut(&IRawElementProviderSimple, f64, f64)) {
    flush_all(state, |_, _, _| panic!("unexpected Toggle"), notify);
}
#[test]
fn uia_range_notification_coalesces_net_zero_and_busy_resume() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    set(&window, 60.0);
    set(&window, 70.0);
    assert_eq!(window.state.notifications.borrow().ranges.len(), 1);
    set(&window, 50.0);
    assert!(window.state.notifications.borrow().ranges.is_empty());
    flush(&window.state, |_, _, _| panic!("net zero"));
    set(&window, 80.0);
    let host = window.state.host.borrow_mut().take();
    flush(&window.state, |_, _, _| panic!("busy"));
    assert!(!window.state.notifications.borrow().posted);
    *window.state.host.borrow_mut() = host;
    window.state.run(|_| Ok(())).unwrap();
    assert!(window.state.notifications.borrow().posted);
    let mut events = Vec::new();
    flush(&window.state, |_, before, after| {
        events.push((before, after))
    });
    assert_eq!(events, [(50.0, 80.0)]);
}
#[test]
fn uia_range_notification_reentrant_update_defers_and_close_revokes() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    set(&window, 60.0);
    let mut events = Vec::new();
    flush(&window.state, |_, before, after| {
        events.push((before, after));
        set(&window, 70.0);
        flush(&window.state, |_, _, _| panic!("nested notification"));
    });
    assert_eq!(events, [(50.0, 60.0)]);
    assert!(window.state.notifications.borrow().posted);
    flush(&window.state, |_, before, after| {
        events.push((before, after))
    });
    assert_eq!(events, [(50.0, 60.0), (60.0, 70.0)]);
    set(&window, 80.0);
    window.close().unwrap();
    flush(&window.state, |_, _, _| panic!("closed notification"));
    assert!(window.state.notifications.borrow().ranges.is_empty());
}
#[test]
fn uia_range_notification_replacement_revokes_pending_identity() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    set(&window, 60.0);
    let scene = Scene::parse(br#"{"id":"slider","type":"slider","props":{"value":30},"events":{"input":"new-move","change":"new-change"}}"#).unwrap();
    window.replace(scene).unwrap();
    assert!(window.state.notifications.borrow().ranges.is_empty());
    flush(&window.state, |_, _, _| panic!("revoked identity"));
    set(&window, 40.0);
    let mut events = Vec::new();
    flush(&window.state, |_, before, after| {
        events.push((before, after))
    });
    assert_eq!(events, [(30.0, 40.0)]);
}
#[test]
fn uia_range_readonly_coalesces_and_revokes_queued_permission() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    window.set_input_policy(false, false).unwrap();
    assert_eq!(window.state.notifications.borrow().readonly.len(), 1);
    window.set_input_policy(true, true).unwrap();
    assert!(window.state.notifications.borrow().readonly.is_empty());
    window.set_input_policy(false, false).unwrap();
    let mut events = Vec::new();
    flush_properties(
        &window.state,
        |_, _, _| {},
        |_, _, _| {},
        |_, before, after| events.push((before, after)),
    );
    assert_eq!(events, [(false, true)]);
    window.set_input_policy(true, true).unwrap();
    window.close().unwrap();
    flush_properties(
        &window.state,
        |_, _, _| {},
        |_, _, _| {},
        |_, _, _| panic!("closed readonly event"),
    );
}
