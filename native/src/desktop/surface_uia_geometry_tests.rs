//! 根点命中与子节点几何经真实 COM vtable 接入实时窗口桥接。
use super::*;
use windows::{core::Interface, Win32::UI::WindowsAndMessaging::*};

#[test]
fn uia_geometry_empty_tree_returns_null_and_rejects_nonfinite_point() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = crate::surface_scene::Scene::parse(br#"{"id":"root","type":"text"}"#).unwrap();
    let window = super::super::SurfaceWindow::new(scene, 240, 200).unwrap();
    let owner = Root::new(window.hwnd).unwrap();
    let root: IRawElementProviderFragmentRoot = owner.provider().cast().unwrap();
    unsafe {
        let mut output = std::ptr::dangling_mut();
        let point = Interface::vtable(&root).ElementProviderFromPoint;
        assert_eq!(point(root.as_raw(), 1., 1., &mut output), S_OK);
        assert!(output.is_null());
        assert_eq!(
            point(root.as_raw(), f64::INFINITY, 1., &mut output),
            E_INVALIDARG
        );
        assert!(output.is_null());
    }
}

#[test]
fn uia_geometry_provider_bounds_offscreen_and_point_use_current_window() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = crate::surface_scene::Scene::parse(br#"{"id":"button","type":"button","props":{"label":"Run"},"layout":{"width":100,"height":32}}"#).unwrap();
    let window = super::super::SurfaceWindow::new(scene, 240, 200).unwrap();
    unsafe {
        SendMessageW(
            window.hwnd,
            WM_GETOBJECT,
            Some(WPARAM(0)),
            Some(LPARAM(UiaRootObjectId as isize)),
        );
        let simple = window.state.uia.borrow().as_ref().unwrap().provider();
        let root: IRawElementProviderFragmentRoot = simple.cast().unwrap();
        let fragment: IRawElementProviderFragment = simple.cast().unwrap();
        let button = fragment.Navigate(NavigateDirection_FirstChild).unwrap();
        let bounds = button.BoundingRectangle().unwrap();
        assert!(bounds.width > 0. && bounds.height > 0.);
        let properties: IRawElementProviderSimple = button.cast().unwrap();
        assert!(bool::try_from(
            &properties
                .GetPropertyValue(UIA_IsOffscreenPropertyId)
                .unwrap()
        )
        .unwrap());
        let _ = ShowWindow(window.hwnd, SW_SHOWNOACTIVATE);
        assert!(!bool::try_from(
            &properties
                .GetPropertyValue(UIA_IsOffscreenPropertyId)
                .unwrap()
        )
        .unwrap());
        let hit = root
            .ElementProviderFromPoint(bounds.left + 5., bounds.top + 5.)
            .unwrap();
        assert_eq!(
            hit.cast::<IRawElementProviderSimple>()
                .unwrap()
                .GetPropertyValue(UIA_AutomationIdPropertyId)
                .unwrap()
                .to_string(),
            "button"
        );
        let mut output = std::ptr::dangling_mut();
        let table = Interface::vtable(&root);
        assert_eq!(
            (table.ElementProviderFromPoint)(root.as_raw(), f64::NAN, 0., &mut output),
            E_INVALIDARG
        );
        assert!(output.is_null());
        assert_eq!(
            (table.ElementProviderFromPoint)(root.as_raw(), -1e30, 0., &mut output),
            S_OK
        );
        assert!(output.is_null());
        let _ = ShowWindow(window.hwnd, SW_HIDE);
        assert_eq!(
            (table.ElementProviderFromPoint)(
                root.as_raw(),
                bounds.left + 5.,
                bounds.top + 5.,
                &mut output
            ),
            S_OK
        );
        assert!(output.is_null());
        window.close().unwrap();
        assert_eq!(
            button.BoundingRectangle().unwrap_err().code().0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
        assert_eq!(
            properties
                .GetPropertyValue(UIA_IsOffscreenPropertyId)
                .unwrap_err()
                .code()
                .0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
    }
}
