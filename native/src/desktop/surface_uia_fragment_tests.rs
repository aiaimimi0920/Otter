//! 验证跨接口身份、nullable 输出、SAFEARRAY 所有权及场景撤销。
use super::*;
use crate::{surface_accessibility::Owner, surface_scene::Scene};
use windows::{
    core::{IUnknown, Interface},
    Win32::System::{Ole::*, Variant::VT_I4},
};

unsafe fn runtime(fragment: &IRawElementProviderFragment) -> Option<Vec<i32>> {
    let array = fragment.GetRuntimeId().unwrap();
    if array.is_null() {
        return None;
    }
    assert_eq!(SafeArrayGetVartype(array).unwrap(), VT_I4);
    let lower = SafeArrayGetLBound(array, 1).unwrap();
    let upper = SafeArrayGetUBound(array, 1).unwrap();
    let values = (lower..=upper)
        .map(|index| {
            let mut value = 0i32;
            SafeArrayGetElement(array, &index, (&mut value as *mut i32).cast()).unwrap();
            value
        })
        .collect();
    SafeArrayDestroy(array).unwrap();
    Some(values)
}
#[test]
fn uia_fragment_tree_identity_runtime_and_foreign_owner_isolation() {
    let _com = super::super::super::Apartment::new().unwrap();
    let scene = Scene::parse(br#"{"id":"tree","type":"column","children":[{"id":"first","type":"button","props":{"label":"Run"}},{"id":"last","type":"text","props":{"text":"Status"}}]}"#).unwrap();
    let owner = Owner::new(&scene);
    let window = super::super::SurfaceWindow::new(scene, 200, 140).unwrap();
    let root = Root::new(window.hwnd).unwrap();
    root.set_scene(owner.root()).unwrap();
    let simple = root.provider();
    let fragment: IRawElementProviderFragment = simple.cast().unwrap();
    let fragment_root: IRawElementProviderFragmentRoot = simple.cast().unwrap();
    assert_eq!(
        fragment.cast::<IUnknown>().unwrap().as_raw(),
        simple.as_raw()
    );
    assert_eq!(
        fragment_root.cast::<IUnknown>().unwrap().as_raw(),
        simple.as_raw()
    );
    unsafe {
        assert!(runtime(&fragment).is_none());
        assert!(fragment.BoundingRectangle().unwrap().width > 0.);
        assert_eq!(
            fragment_root.GetFocus().unwrap_err().code().0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
        let tree = fragment.Navigate(NavigateDirection_FirstChild).unwrap();
        let first = tree.Navigate(NavigateDirection_FirstChild).unwrap();
        let last = first.Navigate(NavigateDirection_NextSibling).unwrap();
        let properties: IRawElementProviderSimple = first.cast().unwrap();
        assert_eq!(
            properties
                .GetPropertyValue(UIA_NamePropertyId)
                .unwrap()
                .to_string(),
            "Run"
        );
        assert_eq!(
            last.cast::<IRawElementProviderSimple>()
                .unwrap()
                .GetPropertyValue(UIA_AutomationIdPropertyId)
                .unwrap()
                .to_string(),
            "last"
        );
        assert!(first.cast::<IRawElementProviderFragmentRoot>().is_err());
        assert_eq!(
            first
                .FragmentRoot()
                .unwrap()
                .cast::<IUnknown>()
                .unwrap()
                .as_raw(),
            simple.as_raw()
        );
        let first_id = runtime(&first).unwrap();
        assert_eq!(first_id[0], UiaAppendRuntimeId as i32);
        assert_eq!(first_id.len(), 4);
        assert_ne!(Some(first_id.clone()), runtime(&last));
        assert_eq!(
            Some(first_id),
            runtime(&last.Navigate(NavigateDirection_PreviousSibling).unwrap())
        );
        assert_eq!(
            runtime(&tree),
            runtime(&first.Navigate(NavigateDirection_Parent).unwrap())
        );
        let mut output = std::ptr::dangling_mut();
        assert_eq!(
            (Interface::vtable(&properties).HostRawElementProvider)(
                properties.as_raw(),
                &mut output
            ),
            S_OK
        );
        assert!(output.is_null());
        assert_eq!(
            (Interface::vtable(&last).Navigate)(
                last.as_raw(),
                NavigateDirection_NextSibling,
                &mut output
            ),
            S_OK
        );
        assert!(output.is_null());
        assert_eq!(
            (Interface::vtable(&fragment).Navigate)(
                fragment.as_raw(),
                NavigateDirection_Parent,
                &mut output
            ),
            S_OK
        );
        assert!(output.is_null());
        // 此 ABI 夹具的 Owner 与真实窗口不同，不能借同名节点读取另一个快照几何。
        assert_eq!(
            first.BoundingRectangle().unwrap_err().code().0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
        let old_id = runtime(&tree);
        drop(owner);
        assert_eq!(
            first
                .Navigate(NavigateDirection_Parent)
                .unwrap_err()
                .code()
                .0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
        let new_scene =
            Scene::parse(br#"{"id":"tree","type":"text","props":{"text":"New"}}"#).unwrap();
        let new_owner = Owner::new(&new_scene);
        root.set_scene(new_owner.root()).unwrap();
        let new_tree = fragment.Navigate(NavigateDirection_FirstChild).unwrap();
        assert_ne!(old_id, runtime(&new_tree));
        assert_eq!(
            tree.GetRuntimeId().unwrap_err().code().0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
    }
}

#[test]
fn uia_fragment_window_replacement_and_close_revoke_retained_children() {
    use windows::Win32::UI::WindowsAndMessaging::*;
    let _com = super::super::super::Apartment::new().unwrap();
    let make_scene =
        || Scene::parse(br#"{"id":"same","type":"text","props":{"text":"Text"}}"#).unwrap();
    let window = super::super::SurfaceWindow::new(make_scene(), 200, 140).unwrap();
    unsafe {
        SendMessageW(
            window.hwnd,
            WM_GETOBJECT,
            Some(WPARAM(0)),
            Some(LPARAM(UiaRootObjectId as isize)),
        )
    };
    let simple = window.state.uia.borrow().as_ref().unwrap().provider();
    let root: IRawElementProviderFragment = simple.cast().unwrap();
    unsafe {
        let old = root.Navigate(NavigateDirection_FirstChild).unwrap();
        let old_id = runtime(&old);
        window.replace(make_scene()).unwrap();
        assert_eq!(
            old.GetRuntimeId().unwrap_err().code().0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
        let new = root.Navigate(NavigateDirection_FirstChild).unwrap();
        assert_ne!(old_id, runtime(&new));
        window.close().unwrap();
        assert_eq!(
            new.FragmentRoot().unwrap_err().code().0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
        assert_eq!(
            root.Navigate(NavigateDirection_FirstChild)
                .unwrap_err()
                .code()
                .0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
    }
}
