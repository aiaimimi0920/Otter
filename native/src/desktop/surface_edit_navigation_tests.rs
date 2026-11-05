//! EDIT 和按钮共享焦点序列；组合中的按键仍留给系统输入法。
use super::*;
#[test]
fn surface_focus_reveal_real_tab_scrolls_between_edit_and_button_preserving_draft() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = Scene::parse(br#"{"id":"root","type":"scroll","layout":{"height":100},"children":[{"id":"field","type":"input","layout":{"height":32,"minHeight":32}},{"id":"gap","type":"view","layout":{"height":180,"minHeight":180}},{"id":"button","type":"button","layout":{"height":40,"minHeight":40},"props":{"label":"Run"},"events":{"click":"run"}}]}"#).unwrap();
    let window = SurfaceWindow::new(scene, 240, 180).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, false).unwrap();
    let edit = child(&window);
    type_text(edit, "keep draft");
    let eligible = |id| {
        window
            .state
            .host
            .borrow()
            .as_ref()
            .unwrap()
            .presentation()
            .focusable(id)
    };
    assert!(!eligible("button"));
    unsafe {
        SetFocus(Some(edit)).unwrap();
        SendMessageW(edit, WM_KEYDOWN, Some(WPARAM(VK_TAB.0 as usize)), None);
        assert_eq!(GetFocus(), window.hwnd);
        assert_eq!(window.state.keyboard.borrow().focused(), Some("button"));
        assert!(eligible("button"));
        // 本地导航不授予提交权，即使目标已滚入视野也不能激活动作。
        SendMessageW(
            window.hwnd,
            WM_KEYDOWN,
            Some(WPARAM(VK_RETURN.0 as usize)),
            None,
        );
        assert!(!window.has_pending_click());
        SendMessageW(
            window.hwnd,
            WM_KEYDOWN,
            Some(WPARAM(VK_TAB.0 as usize)),
            None,
        );
        assert_eq!(GetFocus(), edit);
        assert!(eligible("field"));
    }
    assert_eq!(child(&window), edit);
    assert_eq!(text(edit), "keep draft");
    window.set_input_policy(false, false).unwrap();
    unsafe {
        SendMessageW(edit, WM_KEYDOWN, Some(WPARAM(VK_TAB.0 as usize)), None);
    }
    assert!(!eligible("button"));
    assert!(window.state.keyboard.borrow().focused().is_none());
    assert!(window.take_error().is_none());
}
#[test]
fn surface_edit_textarea_native_crlf_is_encoded_as_html_lf() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = Scene::parse(br#"{"id":"field","type":"textarea","props":{"value":"first\nsecond"},"events":{"input":"typing"}}"#).unwrap();
    let window = SurfaceWindow::new(scene, 240, 160).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    let edit = child(&window);
    assert_eq!(text(edit), "first\r\nsecond");
    type_text(edit, "中文\r\nnext");
    let events = window.prepare_edit(&binding(), 1).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(events[0].bytes()).unwrap()["payload"]["value"],
        "中文\nnext"
    );
    assert!(window.take_error().is_none());
}
#[test]
fn surface_edit_navigation_tab_moves_to_button_and_back_without_typing_tab() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"field","type":"input"},{"id":"button","type":"button","props":{"label":"Run"},"events":{"click":"run"}}]}"#).unwrap();
    let window = SurfaceWindow::new(scene, 240, 160).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    let edit = child(&window);
    unsafe {
        SetFocus(Some(edit)).unwrap();
        SendMessageW(edit, WM_KEYDOWN, Some(WPARAM(VK_TAB.0 as usize)), None);
        assert_eq!(GetFocus(), window.hwnd);
        assert_eq!(window.state.keyboard.borrow().focused(), Some("button"));
        SendMessageW(
            window.hwnd,
            WM_KEYDOWN,
            Some(WPARAM(VK_TAB.0 as usize)),
            None,
        );
        assert_eq!(GetFocus(), edit);
        assert_eq!(text(edit), "");
    }
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
#[test]
fn surface_edit_navigation_enter_commits_only_changed_single_line_value() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    let edit = child(&window);
    unsafe {
        SendMessageW(edit, WM_KEYDOWN, Some(WPARAM(VK_RETURN.0 as usize)), None);
    }
    assert!(!window.has_pending_edit());
    type_text(edit, "new value");
    let _input = window.prepare_edit(&binding(), 1).unwrap();
    unsafe {
        SendMessageW(edit, WM_KEYDOWN, Some(WPARAM(VK_RETURN.0 as usize)), None);
    }
    let commit = window.prepare_edit(&binding(), 1).unwrap();
    assert_eq!(commit.len(), 1);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(commit[0].bytes()).unwrap()["event"],
        "change"
    );
    assert_eq!(text(edit), "new value");
}
#[test]
fn surface_edit_navigation_later_overlay_cannot_be_intercepted_by_child_window() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = Scene::parse(br#"{"id":"root","type":"stack","layout":{"width":180,"height":80},"children":[{"id":"field","type":"input","layout":{"width":180,"height":60}},{"id":"cover","type":"view","layout":{"width":50,"height":40}}]}"#).unwrap();
    let window = SurfaceWindow::new(scene, 240, 160).unwrap();
    window.bind_edits(&binding()).unwrap();
    let edit = child(&window);
    let region = unsafe { CreateRectRgn(0, 0, 0, 0) };
    unsafe {
        GetWindowRgn(edit, region);
        assert!(!PtInRegion(region, 2, 2).as_bool());
        let scale = windows::Win32::UI::HiDpi::GetDpiForWindow(window.hwnd) as f32 / 96.;
        assert!(PtInRegion(region, (80. * scale) as i32, (10. * scale) as i32).as_bool());
        let _ = DeleteObject(region.into());
    }
    assert!(window.take_error().is_none());
}
