//! 实际 Slider HWND 的 COM 身份、实时值、双边沿准入及撤销。
use super::*;
use windows::{
    core::{IUnknown, Interface},
    Win32::UI::WindowsAndMessaging::*,
};
pub(super) fn binding() -> crate::surface_live_binding::Binding {
    crate::surface_live_binding::Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":1}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap()
}
fn scene(disabled: bool) -> crate::surface_scene::Scene {
    crate::surface_scene::Scene::parse(serde_json::json!({"id":"slider","type":"slider","props":{"value":50,"disabled":disabled},"events":{"input":"move","change":"change"}}).to_string().as_bytes()).unwrap()
}
pub(in crate::desktop::surface_window) fn mount() -> super::super::SurfaceWindow {
    let window = super::super::SurfaceWindow::new(scene(false), 240, 200).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    unsafe {
        let _ = ShowWindow(window.hwnd, SW_SHOWNOACTIVATE);
    }
    window
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
unsafe fn range(window: &super::super::SurfaceWindow) -> IRangeValueProvider {
    provider(window)
        .GetPatternProvider(UIA_RangeValuePatternId)
        .unwrap()
        .cast()
        .unwrap()
}
#[test]
fn uia_range_live_value_identity_edges_permissions_and_revocation() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    unsafe {
        let simple = provider(&window);
        let range: IRangeValueProvider = simple
            .GetPatternProvider(UIA_RangeValuePatternId)
            .unwrap()
            .cast()
            .unwrap();
        assert_eq!(
            simple.cast::<IUnknown>().unwrap(),
            range.cast::<IUnknown>().unwrap()
        );
        assert_eq!(range.Value().unwrap(), 50.0);
        assert_eq!(range.Minimum().unwrap(), 0.0);
        assert_eq!(range.Maximum().unwrap(), 100.0);
        assert_eq!(range.SmallChange().unwrap(), 1.0);
        assert_eq!(range.LargeChange().unwrap(), 10.0);
        let child =
            FindWindowExW(Some(window.hwnd), None, windows::core::w!("STATIC"), None).unwrap();
        windows::Win32::UI::Input::KeyboardAndMouse::SetCapture(child);
        assert!(range.SetValue(70.0).is_err());
        assert_eq!(range.Value().unwrap(), 50.0);
        let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
        range.SetValue(60.4).unwrap();
        assert_eq!(range.Value().unwrap(), 60.0);
        range.SetValue(60.0).unwrap();
        for event in ["input", "change"] {
            let events = window.prepare_edit(&binding(), 1).unwrap();
            assert_eq!(events.len(), 1);
            let wire: serde_json::Value = serde_json::from_slice(events[0].bytes()).unwrap();
            assert_eq!(wire["event"], event);
            assert_eq!(wire["payload"]["value"], 60.0);
            window.settle_edits(&events).unwrap();
        }
        assert!(!window.has_pending_edit());
        assert!(window.take_click().is_none());
        for invalid in [-1.0, 101.0, f64::NAN, f64::INFINITY] {
            assert_eq!(range.SetValue(invalid).unwrap_err().code(), E_INVALIDARG);
        }
        assert_eq!(range.Value().unwrap(), 60.0);
        window.set_input_policy(false, false).unwrap();
        assert!(range.IsReadOnly().unwrap().as_bool());
        assert!(range.SetValue(61.0).is_err());
        window.set_input_policy(true, true).unwrap();
        assert!(!range.IsReadOnly().unwrap().as_bool());
        window.replace(scene(true)).unwrap();
        assert_eq!(
            range.Value().unwrap_err().code().0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
        let disabled = provider(&window).cast::<IRangeValueProvider>().unwrap();
        assert!(disabled.IsReadOnly().unwrap().as_bool());
        assert!(disabled.SetValue(61.0).is_err());
        window.close().unwrap();
        assert_eq!(
            disabled.Value().unwrap_err().code().0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
    }
}
#[test]
fn uia_range_full_queue_does_not_mutate() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    unsafe {
        let range = range(&window);
        for value in 51..=66 {
            range.SetValue(value as f64).unwrap();
        }
        assert!(range.SetValue(67.0).is_err());
        assert_eq!(range.Value().unwrap(), 66.0);
        for _ in 0..32 {
            let events = window.prepare_edit(&binding(), 1).unwrap();
            assert_eq!(events.len(), 1);
            window.settle_edits(&events).unwrap();
        }
        assert!(!window.has_pending_edit());
    }
}
#[test]
fn uia_range_metadata_change_rebuilds_contract_and_revokes_old_provider() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    unsafe {
        let old = range(&window);
        old.SetValue(60.0).unwrap();
        let scene=crate::surface_scene::Scene::parse(br#"{"id":"slider","type":"slider","props":{"min":-10,"max":10,"step":0.5,"value":2},"events":{"input":"move","change":"change"}}"#).unwrap();
        window.replace(scene).unwrap();
        assert!(!window.has_pending_edit());
        assert!(old.Minimum().is_err());
        assert!(old.SetValue(3.0).is_err());
        let fresh = range(&window);
        assert_eq!(fresh.Minimum().unwrap(), -10.0);
        assert_eq!(fresh.Maximum().unwrap(), 10.0);
        assert_eq!(fresh.SmallChange().unwrap(), 0.5);
        assert_eq!(fresh.LargeChange().unwrap(), 2.0);
        assert_eq!(fresh.Value().unwrap(), 2.0);
    }
}
