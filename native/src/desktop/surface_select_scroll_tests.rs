//! 自有真实下拉窗口的滚动/裁剪边界，不注入用户输入或访问其他应用窗口。
use super::*;
fn select_scene() -> Scene {
    Scene::parse(br#"{"id":"root","type":"scroll","layout":{"width":140,"height":100},"children":[{"id":"space","type":"view","layout":{"height":10,"minHeight":10}},{"id":"select","type":"select","props":{"value":"a","options":[{"value":"a","label":"A"},{"value":"b","label":"B"}]},"events":{"change":"choose"},"layout":{"height":32,"minHeight":32}},{"id":"tail","type":"view","layout":{"height":240,"minHeight":240}}]}"#).unwrap()
}
fn mounted() -> (SurfaceWindow, HWND) {
    let window = SurfaceWindow::new(select_scene(), 240, 200).unwrap();
    let binding = crate::surface_live_binding::Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":1}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap();
    window.bind_edits(&binding).unwrap();
    window.set_input_policy(true, true).unwrap();
    window.show().unwrap();
    let child = unsafe { FindWindowExW(Some(window.hwnd), None, w!("COMBOBOX"), None).unwrap() };
    (window, child)
}
unsafe fn open(child: HWND) {
    SendMessageW(child, CB_SHOWDROPDOWN, Some(WPARAM(1)), None);
    assert_ne!(SendMessageW(child, CB_GETDROPPEDSTATE, None, None).0, 0);
}
unsafe fn closed(child: HWND) {
    assert_eq!(SendMessageW(child, CB_GETDROPPEDSTATE, None, None).0, 0);
}
#[test]
fn surface_select_scroll_reanchors_without_orphan_popup_or_synthetic_change() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, child) = mounted();
    unsafe {
        let mut before = RECT::default();
        GetWindowRect(child, &mut before).unwrap();
        open(child);
        window.replace(select_scene()).unwrap();
        assert_ne!(
            SendMessageW(child, CB_GETDROPPEDSTATE, None, None).0,
            0,
            "unchanged geometry closed the popup"
        );
        window
            .state
            .run(|host| {
                assert!(host.scroll([5, 5], 1, 5., false)?);
                Ok(())
            })
            .unwrap();
        let mut after = RECT::default();
        GetWindowRect(child, &mut after).unwrap();
        assert!(after.top < before.top);
        closed(child);
        assert_eq!(SendMessageW(child, CB_GETCURSEL, None, None).0, 0);
        assert!(windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(child).as_bool());
        open(child);
        window
            .state
            .run(|host| {
                assert!(host.scroll([5, 5], 1, 100., false)?);
                Ok(())
            })
            .unwrap();
        closed(child);
        assert!(!windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(child).as_bool());
        window
            .state
            .run(|host| {
                assert!(host.scroll([5, 5], 1, -105., false)?);
                Ok(())
            })
            .unwrap();
        assert!(windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(child).as_bool());
        assert_eq!(SendMessageW(child, CB_GETCURSEL, None, None).0, 0);
    }
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
