//! Switch Toggle 的真实 COM 状态/权限/FIFO；不发送远端动作。
use super::*;
use windows::{
    core::{IUnknown, Interface},
    Win32::UI::WindowsAndMessaging::*,
};
fn binding() -> crate::surface_live_binding::Binding {
    crate::surface_live_binding::Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":1}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap()
}
fn scene(disabled: bool) -> crate::surface_scene::Scene {
    crate::surface_scene::Scene::parse(serde_json::json!({"id":"switch","type":"switch","props":{"value":false,"disabled":disabled},"events":{"change":"change"}}).to_string().as_bytes()).unwrap()
}
unsafe fn provider(window: &super::super::SurfaceWindow) -> IRawElementProviderSimple {
    SendMessageW(
        window.hwnd,
        WM_GETOBJECT,
        Some(WPARAM(0)),
        Some(LPARAM(UiaRootObjectId as isize)),
    );
    window
        .state
        .uia
        .borrow()
        .as_ref()
        .unwrap()
        .provider()
        .cast::<IRawElementProviderFragment>()
        .unwrap()
        .Navigate(NavigateDirection_FirstChild)
        .unwrap()
        .cast()
        .unwrap()
}
fn mount() -> super::super::SurfaceWindow {
    let window = super::super::SurfaceWindow::new(scene(false), 240, 200).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    unsafe {
        let _ = ShowWindow(window.hwnd, SW_SHOWNOACTIVATE);
    }
    window
}
#[test]
fn uia_toggle_live_state_soft_stop_and_revoked_provider() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    unsafe {
        let simple = provider(&window);
        let toggle: IToggleProvider = simple
            .GetPatternProvider(UIA_TogglePatternId)
            .unwrap()
            .cast()
            .unwrap();
        assert_eq!(
            toggle.cast::<IUnknown>().unwrap(),
            simple.cast::<IUnknown>().unwrap()
        );
        assert!(simple.cast::<IInvokeProvider>().is_err());
        assert_eq!(toggle.ToggleState().unwrap(), ToggleState_Off);
        window.set_input_policy(true, false).unwrap();
        toggle.Toggle().unwrap();
        assert_eq!(toggle.ToggleState().unwrap(), ToggleState_On);
        assert_eq!(
            i32::try_from(
                &simple
                    .GetPropertyValue(UIA_ToggleToggleStatePropertyId)
                    .unwrap()
            )
            .unwrap(),
            ToggleState_On.0
        );
        assert!(window.take_click().is_none());
        assert!(window.prepare_edit(&binding(), 1).is_err());
        window.set_input_policy(true, true).unwrap();
        let events = window.prepare_edit(&binding(), 1).unwrap();
        assert_eq!(events.len(), 1);
        let wire: serde_json::Value = serde_json::from_slice(events[0].bytes()).unwrap();
        assert_eq!(wire["payload"]["value"], true);
        assert_eq!(wire["event"], "change");
        assert!(!events[0].attempted());
        toggle.Toggle().unwrap();
        assert_eq!(toggle.ToggleState().unwrap(), ToggleState_Off);
        window.set_input_policy(false, false).unwrap();
        assert_eq!(
            toggle.Toggle().unwrap_err().code().0,
            UIA_E_ELEMENTNOTENABLED as i32
        );
        assert_eq!(toggle.ToggleState().unwrap(), ToggleState_Off);
        window.set_input_policy(true, true).unwrap();
        let host = window.state.host.borrow_mut().take();
        assert_eq!(
            toggle.Toggle().unwrap_err().code().0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
        *window.state.host.borrow_mut() = host;
        window.replace(scene(true)).unwrap();
        assert_eq!(
            toggle.ToggleState().unwrap_err().code().0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
        assert!(toggle
            .cast::<IRawElementProviderSimple>()
            .unwrap()
            .cast::<IToggleProvider>()
            .is_ok());
        let disabled: IToggleProvider = provider(&window)
            .GetPatternProvider(UIA_TogglePatternId)
            .unwrap()
            .cast()
            .unwrap();
        assert_eq!(
            disabled.Toggle().unwrap_err().code().0,
            UIA_E_ELEMENTNOTENABLED as i32
        );
        window.close().unwrap();
        assert_eq!(
            disabled.ToggleState().unwrap_err().code().0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
    }
}
#[test]
fn uia_toggle_capacity_failure_does_not_mutate_state() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    unsafe {
        let toggle: IToggleProvider = provider(&window)
            .GetPatternProvider(UIA_TogglePatternId)
            .unwrap()
            .cast()
            .unwrap();
        for _ in 0..32 {
            toggle.Toggle().unwrap();
        }
        assert_eq!(toggle.ToggleState().unwrap(), ToggleState_Off);
        assert!(toggle.Toggle().is_err());
        assert_eq!(toggle.ToggleState().unwrap(), ToggleState_Off);
        for index in 0..32 {
            let events = window.prepare_edit(&binding(), 1).unwrap();
            assert_eq!(events.len(), 1);
            let wire: serde_json::Value = serde_json::from_slice(events[0].bytes()).unwrap();
            assert_eq!(wire["payload"]["value"], index % 2 == 0);
            window.settle_edits(&events).unwrap();
        }
    }
}
#[test]
fn uia_toggle_ancestor_click_precedes_own_change_without_self_fence_deadlock() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = crate::surface_scene::Scene::parse(br#"{"id":"outer","type":"view","events":{"click":"outer-click"},"children":[{"id":"switch","type":"switch","props":{"value":false},"events":{"change":"change"}}]}"#).unwrap();
    let window = super::super::SurfaceWindow::new(scene, 240, 200).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    unsafe {
        let _ = ShowWindow(window.hwnd, SW_SHOWNOACTIVATE);
        let switch: IRawElementProviderSimple = provider(&window)
            .cast::<IRawElementProviderFragment>()
            .unwrap()
            .Navigate(NavigateDirection_FirstChild)
            .unwrap()
            .cast()
            .unwrap();
        let toggle: IToggleProvider = switch
            .GetPatternProvider(UIA_TogglePatternId)
            .unwrap()
            .cast()
            .unwrap();
        toggle.Toggle().unwrap();
        assert_eq!(
            toggle.Toggle().unwrap_err().code().0,
            UIA_E_INVALIDOPERATION as i32
        );
        let click = window.take_click().unwrap();
        let deferred = window.defer_click(click, &binding(), 1).unwrap();
        assert_eq!(
            window.deferred_state(&deferred),
            crate::desktop::surface_edit_mount::FenceState::Ready
        );
        let events = window.prepare_deferred(&deferred, &binding(), 1).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(events[0].bytes()).unwrap()["action"],
            "outer-click"
        );
        assert_eq!(window.prepare_edit(&binding(), 1).unwrap().len(), 1);
    }
}
