//! Invoke 的 COM 身份、单槽、权限和消费路径；不发送远端动作。
use super::*;
use windows::{
    core::{IUnknown, Interface},
    Win32::UI::{Input::KeyboardAndMouse::GetFocus, WindowsAndMessaging::*},
};

fn binding() -> crate::surface_live_binding::Binding {
    crate::surface_live_binding::Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":1}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap()
}
fn scene(disabled: bool) -> crate::surface_scene::Scene {
    crate::surface_scene::Scene::parse(serde_json::json!({"id":"button","type":"button","props":{"label":"Run","disabled":disabled},"events":{"click":"run"}}).to_string().as_bytes()).unwrap()
}
unsafe fn providers(
    window: &super::super::SurfaceWindow,
) -> (IRawElementProviderSimple, IRawElementProviderSimple) {
    SendMessageW(
        window.hwnd,
        WM_GETOBJECT,
        Some(WPARAM(0)),
        Some(LPARAM(UiaRootObjectId as isize)),
    );
    let root = window.state.uia.borrow().as_ref().unwrap().provider();
    let child = root
        .cast::<IRawElementProviderFragment>()
        .unwrap()
        .Navigate(NavigateDirection_FirstChild)
        .unwrap()
        .cast()
        .unwrap();
    (root, child)
}

#[test]
fn uia_invoke_queues_single_candidate_without_focus_or_direct_action() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = super::super::SurfaceWindow::new(scene(false), 240, 200).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    unsafe {
        let _ = ShowWindow(window.hwnd, SW_SHOWNOACTIVATE);
        let focus = GetFocus();
        let (root, button) = providers(&window);
        let mut output = std::ptr::dangling_mut();
        assert_eq!(
            (Interface::vtable(&root).GetPatternProvider)(
                root.as_raw(),
                UIA_InvokePatternId,
                &mut output
            ),
            S_OK
        );
        assert!(output.is_null());
        let invoke: IInvokeProvider = button
            .GetPatternProvider(UIA_InvokePatternId)
            .unwrap()
            .cast()
            .unwrap();
        assert_eq!(
            invoke.cast::<IUnknown>().unwrap(),
            button.cast::<IUnknown>().unwrap()
        );
        assert!(bool::try_from(
            &button
                .GetPropertyValue(UIA_IsInvokePatternAvailablePropertyId)
                .unwrap()
        )
        .unwrap());
        invoke.Invoke().unwrap();
        assert_eq!(GetFocus(), focus);
        assert!(window.state.keyboard.borrow().focused().is_none());
        assert_eq!(
            invoke.Invoke().unwrap_err().code().0,
            UIA_E_INVALIDOPERATION as i32
        );
        let click = window.take_click().unwrap();
        assert!(matches!(click.source, super::super::Activation::Automation));
        let deferred = window.defer_click(click, &binding(), 1).unwrap();
        let events = window.prepare_deferred(&deferred, &binding(), 1).unwrap();
        assert_eq!(events.len(), 1);
        let wire: serde_json::Value = serde_json::from_slice(events[0].bytes()).unwrap();
        assert_eq!(wire["action"], "run");
        assert!(!events[0].attempted());
        invoke.Invoke().unwrap();
        let click = window.take_click().unwrap();
        assert_eq!(
            window
                .click_events_for_candidate(click, &binding(), 2)
                .unwrap()
                .len(),
            1
        );
        window.set_input_policy(true, false).unwrap();
        assert_eq!(
            invoke.Invoke().unwrap_err().code().0,
            UIA_E_ELEMENTNOTENABLED as i32
        );
        assert!(window.take_click().is_none());
        window.set_input_policy(true, true).unwrap();
        invoke.Invoke().unwrap();
        let click = window.take_click().unwrap();
        window.set_input_policy(false, false).unwrap();
        assert!(window
            .click_events_for_candidate(click, &binding(), 3)
            .is_err());
        assert_eq!(
            invoke.Invoke().unwrap_err().code().0,
            UIA_E_ELEMENTNOTENABLED as i32
        );
        window.set_input_policy(true, true).unwrap();
        let host = window.state.host.borrow_mut().take();
        assert_eq!(
            invoke.Invoke().unwrap_err().code().0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
        *window.state.host.borrow_mut() = host;
        let _ = ShowWindow(window.hwnd, SW_HIDE);
        assert_eq!(
            invoke.Invoke().unwrap_err().code().0,
            UIA_E_ELEMENTNOTENABLED as i32
        );
        let _ = ShowWindow(window.hwnd, SW_SHOWNOACTIVATE);
        window.replace(scene(true)).unwrap();
        assert_eq!(
            invoke.Invoke().unwrap_err().code().0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
        // COM 支持接口集合固定；失效只影响方法，不改变 QI 身份。
        assert!(invoke
            .cast::<IRawElementProviderSimple>()
            .unwrap()
            .cast::<IInvokeProvider>()
            .is_ok());
        let (_, disabled) = providers(&window);
        let disabled: IInvokeProvider = disabled
            .GetPatternProvider(UIA_InvokePatternId)
            .unwrap()
            .cast()
            .unwrap();
        assert_eq!(
            disabled.Invoke().unwrap_err().code().0,
            UIA_E_ELEMENTNOTENABLED as i32
        );
        window.close().unwrap();
        assert_eq!(
            disabled.Invoke().unwrap_err().code().0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
    }
}

#[test]
fn uia_invoke_is_not_exposed_for_switch_or_text() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for kind in ["switch", "text"] {
        let scene = crate::surface_scene::Scene::parse(
            format!(r#"{{"id":"node","type":"{kind}"}}"#).as_bytes(),
        )
        .unwrap();
        let window = super::super::SurfaceWindow::new(scene, 240, 200).unwrap();
        unsafe {
            let (_, node) = providers(&window);
            let mut output = std::ptr::dangling_mut();
            assert_eq!(
                (Interface::vtable(&node).GetPatternProvider)(
                    node.as_raw(),
                    UIA_InvokePatternId,
                    &mut output
                ),
                S_OK
            );
            assert!(output.is_null());
            assert_eq!(
                node.cast::<IInvokeProvider>().unwrap_err().code(),
                E_NOINTERFACE
            );
            assert!(!bool::try_from(
                &node
                    .GetPropertyValue(UIA_IsInvokePatternAvailablePropertyId)
                    .unwrap()
            )
            .unwrap());
        }
    }
}

#[test]
fn uia_invoke_does_not_interleave_with_existing_mouse_press() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = super::super::SurfaceWindow::new(scene(false), 240, 200).unwrap();
    window.set_input_policy(true, true).unwrap();
    unsafe {
        let _ = ShowWindow(window.hwnd, SW_SHOWNOACTIVATE);
        let (_, button) = providers(&window);
        let invoke: IInvokeProvider = button
            .GetPatternProvider(UIA_InvokePatternId)
            .unwrap()
            .cast()
            .unwrap();
        let point = LPARAM((10 << 16) | 10);
        SendMessageW(window.hwnd, WM_LBUTTONDOWN, None, Some(point));
        assert_eq!(
            invoke.Invoke().unwrap_err().code().0,
            UIA_E_INVALIDOPERATION as i32
        );
        assert!(!window.has_pending_click());
        SendMessageW(window.hwnd, WM_LBUTTONUP, None, Some(point));
        let click = window.take_click().unwrap();
        assert!(matches!(click.source, super::super::Activation::Pointer));
        invoke.Invoke().unwrap();
        assert!(matches!(
            window.take_click().unwrap().source,
            super::super::Activation::Automation
        ));
    }
}

#[test]
fn uia_invoke_preserves_native_edit_value_and_dependency_fence() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = crate::surface_scene::Scene::parse(br#"{"id":"form","type":"column","children":[{"id":"button","type":"button","props":{"label":"Run","includeSurfaceValues":true},"events":{"click":"run"}},{"id":"field","type":"input","props":{"value":"source"},"events":{"input":"typing"},"layout":{"width":180,"height":32}}]}"#).unwrap();
    let window = super::super::SurfaceWindow::new(scene, 240, 200).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    unsafe {
        let _ = ShowWindow(window.hwnd, SW_SHOWNOACTIVATE);
        let edit = FindWindowExW(Some(window.hwnd), None, windows::core::w!("EDIT"), None).unwrap();
        let set = |value: &str| {
            use windows::Win32::UI::Controls::{EM_REPLACESEL, EM_SETSEL};
            SendMessageW(edit, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
            let text = windows::core::HSTRING::from(value);
            SendMessageW(
                edit,
                EM_REPLACESEL,
                Some(WPARAM(1)),
                Some(LPARAM(text.as_ptr() as isize)),
            );
        };
        set("中文🙂");
        let (_, form) = providers(&window);
        let button: IRawElementProviderSimple = form
            .cast::<IRawElementProviderFragment>()
            .unwrap()
            .Navigate(NavigateDirection_FirstChild)
            .unwrap()
            .cast()
            .unwrap();
        let invoke: IInvokeProvider = button
            .GetPatternProvider(UIA_InvokePatternId)
            .unwrap()
            .cast()
            .unwrap();
        invoke.Invoke().unwrap();
        let deferred = window
            .defer_click(window.take_click().unwrap(), &binding(), 1)
            .unwrap();
        use crate::desktop::surface_edit_mount::FenceState;
        assert_eq!(window.deferred_state(&deferred), FenceState::Pending);
        assert!(window.prepare_deferred(&deferred, &binding(), 1).is_err());
        let edits = window.prepare_edit(&binding(), 1).unwrap();
        assert_eq!(edits.len(), 1);
        window.settle_edits(&edits).unwrap();
        assert_eq!(window.deferred_state(&deferred), FenceState::Ready);
        set("newer");
        let events = window.prepare_deferred(&deferred, &binding(), 2).unwrap();
        assert_eq!(events.len(), 1);
        let wire: serde_json::Value = serde_json::from_slice(events[0].bytes()).unwrap();
        assert_eq!(wire["payload"]["surfaceValues"]["field"], "中文🙂");
        assert_eq!(wire["action"], "run");
        assert!(!events[0].attempted());
    }
}
