//! 真实 HWND/COM 焦点回归；不注入按键，不产生业务动作。
use super::*;
use windows::{core::Interface, Win32::UI::WindowsAndMessaging::*};

fn scene(disabled: bool) -> crate::surface_scene::Scene {
    crate::surface_scene::Scene::parse(
        serde_json::json!({
            "id":"button","type":"button","props":{"label":"Run","disabled":disabled},
            "events":{"click":"run"},"layout":{"width":100,"height":32}
        })
        .to_string()
        .as_bytes(),
    )
    .unwrap()
}
fn provider(window: &super::super::SurfaceWindow) -> IRawElementProviderSimple {
    unsafe {
        SendMessageW(
            window.hwnd,
            WM_GETOBJECT,
            Some(WPARAM(0)),
            Some(LPARAM(UiaRootObjectId as isize)),
        );
    }
    window.state.uia.borrow().as_ref().unwrap().provider()
}
fn property(node: &IRawElementProviderFragment, id: UIA_PROPERTY_ID) -> bool {
    unsafe {
        bool::try_from(
            &node
                .cast::<IRawElementProviderSimple>()
                .unwrap()
                .GetPropertyValue(id)
                .unwrap(),
        )
        .unwrap()
    }
}

#[test]
fn uia_focus_child_root_and_submit_pause_share_keyboard_owner() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = super::super::SurfaceWindow::new(scene(false), 240, 200).unwrap();
    window.set_input_policy(true, false).unwrap();
    unsafe {
        let _ = ShowWindow(window.hwnd, SW_SHOWNOACTIVATE);
        let simple = provider(&window);
        let root: IRawElementProviderFragmentRoot = simple.cast().unwrap();
        let fragment: IRawElementProviderFragment = simple.cast().unwrap();
        let button = fragment.Navigate(NavigateDirection_FirstChild).unwrap();
        assert!(property(&button, UIA_IsKeyboardFocusablePropertyId));
        let host = window.state.host.borrow_mut().take();
        assert_eq!(
            button.SetFocus().unwrap_err().code().0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
        *window.state.host.borrow_mut() = host;
        button.SetFocus().unwrap();
        assert!(window.take_click().is_none());
        assert_eq!(window.state.keyboard.borrow().focused(), Some("button"));
        assert!(property(&button, UIA_HasKeyboardFocusPropertyId));
        assert!(!property(&fragment, UIA_HasKeyboardFocusPropertyId));
        assert_eq!(
            root.GetFocus()
                .unwrap()
                .cast::<IRawElementProviderSimple>()
                .unwrap()
                .GetPropertyValue(UIA_AutomationIdPropertyId)
                .unwrap()
                .to_string(),
            "button"
        );
        fragment.SetFocus().unwrap();
        assert!(window.state.keyboard.borrow().focused().is_none());
        assert!(property(&fragment, UIA_HasKeyboardFocusPropertyId));
        assert!(!property(&button, UIA_HasKeyboardFocusPropertyId));
        window.set_input_policy(false, false).unwrap();
        assert!(!property(&button, UIA_IsEnabledPropertyId));
        assert!(!property(&button, UIA_IsKeyboardFocusablePropertyId));
        assert_eq!(
            button.SetFocus().unwrap_err().code().0,
            UIA_E_ELEMENTNOTENABLED as i32
        );
        window.set_input_policy(true, false).unwrap();
        let _ = ShowWindow(window.hwnd, SW_HIDE);
        assert_eq!(
            button.SetFocus().unwrap_err().code().0,
            UIA_E_ELEMENTNOTENABLED as i32
        );
        let mut output = std::ptr::dangling_mut();
        assert_eq!(
            (Interface::vtable(&root).GetFocus)(root.as_raw(), &mut output),
            S_OK
        );
        assert!(output.is_null());
        let _ = ShowWindow(window.hwnd, SW_SHOWNOACTIVATE);
        window.replace(scene(true)).unwrap();
        assert_eq!(
            button.SetFocus().unwrap_err().code().0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
        let disabled = fragment.Navigate(NavigateDirection_FirstChild).unwrap();
        assert!(!property(&disabled, UIA_IsEnabledPropertyId));
        assert_eq!(
            disabled.SetFocus().unwrap_err().code().0,
            UIA_E_ELEMENTNOTENABLED as i32
        );
        window.close().unwrap();
        assert!(fragment.SetFocus().is_err());
    }
}

#[test]
fn uia_focus_reveals_offscreen_switch_without_activation() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = crate::surface_scene::Scene::parse(br#"{"id":"scroll","type":"scroll","layout":{"width":100,"height":100},"children":[{"id":"space","type":"view","layout":{"height":200,"minHeight":200}},{"id":"slot","type":"view","layout":{"height":32,"minHeight":32},"children":[{"id":"switch","type":"switch","props":{"value":false},"events":{"change":"change"}}]}]}"#).unwrap();
    let window = super::super::SurfaceWindow::new(scene, 240, 200).unwrap();
    let binding = crate::surface_live_binding::Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":1}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap();
    window.bind_edits(&binding).unwrap();
    window.set_input_policy(true, false).unwrap();
    unsafe {
        let _ = ShowWindow(window.hwnd, SW_SHOWNOACTIVATE);
        let fragment: IRawElementProviderFragment = provider(&window).cast().unwrap();
        let scroll = fragment.Navigate(NavigateDirection_FirstChild).unwrap();
        assert_eq!(
            scroll.SetFocus().unwrap_err().code().0,
            UIA_E_NOTSUPPORTED as i32
        );
        let switch = scroll
            .Navigate(NavigateDirection_LastChild)
            .unwrap()
            .Navigate(NavigateDirection_FirstChild)
            .unwrap();
        assert_eq!(
            switch
                .cast::<IRawElementProviderSimple>()
                .unwrap()
                .GetPropertyValue(UIA_AutomationIdPropertyId)
                .unwrap()
                .to_string(),
            "switch"
        );
        assert!(property(&switch, UIA_IsOffscreenPropertyId));
        assert!(
            property(&switch, UIA_IsKeyboardFocusablePropertyId),
            "bounds={:?}, enabled={}, local={}",
            switch.BoundingRectangle().unwrap(),
            property(&switch, UIA_IsEnabledPropertyId),
            window.local_edit_enabled()
        );
        switch.SetFocus().unwrap();
        assert!(!property(&switch, UIA_IsOffscreenPropertyId));
        assert!(property(&switch, UIA_HasKeyboardFocusPropertyId));
        assert_eq!(window.state.keyboard.borrow().focused(), Some("switch"));
        let host = window.state.host.borrow();
        assert_eq!(host.as_ref().unwrap().switch_value("switch"), Some(false));
        drop(host);
        window.set_input_policy(true, true).unwrap();
        assert!(window.prepare_edit(&binding, 1).unwrap().is_empty());
        assert!(window.take_click().is_none());
    }
}
