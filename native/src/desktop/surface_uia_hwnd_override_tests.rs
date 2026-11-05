//! HWND override ABI 身份、空结果、当前挂载、替换撤销；不接受外部或未挂载 HWND。
use super::*;
use windows::{
    core::{IUnknown, Interface},
    Win32::UI::WindowsAndMessaging::*,
};
fn scene() -> crate::surface_scene::Scene {
    crate::surface_scene::Scene::parse(br#"{"id":"group","type":"column","children":[{"id":"input","type":"input","props":{"value":"hello"}}]}"#).unwrap()
}
fn provider(window: &super::super::SurfaceWindow) -> IRawElementProviderSimple {
    unsafe {
        SendMessageW(
            window.hwnd,
            WM_GETOBJECT,
            None,
            Some(LPARAM(UiaRootObjectId as isize)),
        );
    }
    window.state.uia.borrow().as_ref().unwrap().provider()
}
#[test]
fn uia_hwnd_override_identity_nullable_results_and_scene_revocation() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = super::super::SurfaceWindow::new(scene(), 240, 200).unwrap();
    let binding = crate::surface_live_binding::Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":1}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap();
    window.bind_edits(&binding).unwrap();
    window.set_input_policy(true, false).unwrap();
    window.show().unwrap();
    let simple = provider(&window);
    let mapping: IRawElementProviderHwndOverride = simple.cast().unwrap();
    assert_eq!(
        mapping.cast::<IUnknown>().unwrap().as_raw(),
        simple.as_raw()
    );
    let edit = window
        .state
        .host
        .borrow()
        .as_ref()
        .unwrap()
        .accessibility_edit_hwnd("input")
        .unwrap();
    unsafe {
        let mut output = std::ptr::dangling_mut();
        assert_eq!(
            (mapping.vtable().GetOverrideProviderForHwnd)(
                mapping.as_raw(),
                window.hwnd,
                &mut output
            ),
            S_OK
        );
        assert!(output.is_null());
        assert_eq!(
            (mapping.vtable().GetOverrideProviderForHwnd)(
                mapping.as_raw(),
                edit,
                std::ptr::null_mut()
            ),
            E_POINTER
        );
        let native = mapping.GetOverrideProviderForHwnd(edit).unwrap();
        assert!(native.cast::<IRawElementProviderHwndOverride>().is_err());
        assert!(native.HostRawElementProvider().is_ok());
        assert_eq!(
            native
                .GetPropertyValue(UIA_AutomationIdPropertyId)
                .unwrap()
                .to_string(),
            "input"
        );
        let fragment: IRawElementProviderFragment = native.cast().unwrap();
        assert!(fragment.GetRuntimeId().unwrap().is_null());
        let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(None);
        fragment.SetFocus().unwrap();
        let focus = simple
            .cast::<IRawElementProviderFragmentRoot>()
            .unwrap()
            .GetFocus()
            .unwrap();
        assert_eq!(
            focus
                .cast::<IRawElementProviderSimple>()
                .unwrap()
                .GetPropertyValue(UIA_AutomationIdPropertyId)
                .unwrap()
                .to_string(),
            "input"
        );
        assert_eq!(
            fragment
                .Navigate(NavigateDirection_Parent)
                .unwrap()
                .cast::<IRawElementProviderSimple>()
                .unwrap()
                .GetPropertyValue(UIA_AutomationIdPropertyId)
                .unwrap()
                .to_string(),
            "group"
        );
        window.replace(scene()).unwrap();
        assert_eq!(
            native.HostRawElementProvider().unwrap_err().code().0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
        window.close().unwrap();
        assert_eq!(
            mapping
                .GetOverrideProviderForHwnd(edit)
                .unwrap_err()
                .code()
                .0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
    }
}
#[test]
fn uia_hwnd_override_does_not_advertise_unmounted_edit_nodes() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = super::super::SurfaceWindow::new(scene(), 240, 200).unwrap();
    let root: IRawElementProviderFragment = provider(&window).cast().unwrap();
    unsafe {
        let group = root.Navigate(NavigateDirection_FirstChild).unwrap();
        let mut output = std::ptr::dangling_mut();
        assert_eq!(
            (group.vtable().Navigate)(group.as_raw(), NavigateDirection_FirstChild, &mut output),
            S_OK
        );
        assert!(output.is_null());
    }
}
