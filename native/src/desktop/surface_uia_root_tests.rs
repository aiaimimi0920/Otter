//! 真实 COM ABI 合同与 HWND 根 provider 测试，不替代跨进程 UIA 客户端验收。
use super::*;
use windows::{
    core::{IUnknown, Interface, GUID},
    Win32::System::Variant::*,
};

fn window() -> super::super::SurfaceWindow {
    let scene = crate::surface_scene::Scene::parse(
        br#"{"id":"root","type":"text","props":{"text":"Text"}}"#,
    )
    .unwrap();
    super::super::SurfaceWindow::new(scene, 200, 140).unwrap()
}
#[test]
fn uia_root_properties_null_patterns_and_host_follow_com_contract() {
    let _com = super::super::super::Apartment::new().unwrap();
    let window = window();
    let root = Root::new(window.hwnd).unwrap();
    let provider = root.provider();
    unsafe {
        assert_eq!(
            provider.ProviderOptions().unwrap(),
            ProviderOptions_ServerSideProvider
                | ProviderOptions_UseComThreading
                | ProviderOptions_ProviderOwnsSetFocus
        );
        let name = provider.GetPropertyValue(UIA_NamePropertyId).unwrap();
        assert_eq!(name.to_string(), "Otter Surface");
        let framework = provider
            .GetPropertyValue(UIA_FrameworkIdPropertyId)
            .unwrap();
        assert_eq!(framework.to_string(), "Otter.Native");
        let unknown = provider.GetPropertyValue(UIA_PROPERTY_ID(-123)).unwrap();
        assert_eq!(unknown.Anonymous.Anonymous.vt, VT_EMPTY);
        let mut pattern = std::ptr::dangling_mut::<std::ffi::c_void>();
        let vtable = Interface::vtable(&provider);
        assert_eq!(
            (vtable.GetPatternProvider)(provider.as_raw(), UIA_InvokePatternId, &mut pattern),
            S_OK
        );
        assert!(pattern.is_null());
        assert_eq!(
            (vtable.GetPatternProvider)(
                provider.as_raw(),
                UIA_InvokePatternId,
                std::ptr::null_mut()
            ),
            E_POINTER
        );
        assert!(provider.HostRawElementProvider().is_ok());
        let unknown: IUnknown = provider.cast().unwrap();
        let again: IRawElementProviderSimple = unknown.cast().unwrap();
        assert_eq!(again.as_raw(), provider.as_raw());
        let mut output = std::ptr::dangling_mut::<std::ffi::c_void>();
        assert_eq!(
            (vtable.base__.QueryInterface)(provider.as_raw(), &GUID::zeroed(), &mut output),
            E_NOINTERFACE
        );
        assert!(output.is_null());
    }
    drop(root);
    unsafe {
        assert_eq!(
            provider
                .GetPropertyValue(UIA_NamePropertyId)
                .unwrap_err()
                .code()
                .0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
        assert_eq!(
            provider.HostRawElementProvider().unwrap_err().code().0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
    }
}

#[test]
fn uia_root_references_release_object_and_wrong_thread_is_rejected() {
    let _com = super::super::super::Apartment::new().unwrap();
    let window = window();
    let root = Root::new(window.hwnd).unwrap();
    let alive = Arc::downgrade(&root.alive);
    let provider = root.provider();
    // 故意绕过 COM marshal 验证防御检查；仅传地址数值，不跨线程调用 HWND。
    let raw = provider.as_raw() as usize;
    let result = std::thread::spawn(move || unsafe {
        let borrowed =
            IRawElementProviderSimple::from_raw_borrowed(&(raw as *mut std::ffi::c_void))
                .unwrap()
                .clone();
        borrowed.ProviderOptions().unwrap_err().code()
    })
    .join()
    .unwrap();
    assert_eq!(result, RPC_E_WRONG_THREAD);
    drop(root);
    assert!(alive.upgrade().is_some());
    drop(provider);
    assert!(alive.upgrade().is_none());
}

#[test]
fn uia_root_getobject_reuses_window_identity_and_revokes_on_destroy() {
    use windows::Win32::UI::WindowsAndMessaging::*;
    let _com = super::super::super::Apartment::new().unwrap();
    let window = window();
    assert!(window.state.uia.borrow().is_none());
    unsafe {
        SendMessageW(
            window.hwnd,
            WM_GETOBJECT,
            Some(WPARAM(0)),
            Some(LPARAM(UiaRootObjectId as isize)),
        )
    };
    let first = window.state.uia.borrow().as_ref().unwrap().provider();
    let scene =
        crate::surface_scene::Scene::parse(br#"{"id":"new","type":"text","props":{"text":"New"}}"#)
            .unwrap();
    window.replace(scene).unwrap();
    unsafe {
        SendMessageW(
            window.hwnd,
            WM_GETOBJECT,
            Some(WPARAM(0)),
            Some(LPARAM(UiaRootObjectId as isize)),
        )
    };
    let second = window.state.uia.borrow().as_ref().unwrap().provider();
    assert_eq!(first.as_raw(), second.as_raw());
    window.close().unwrap();
    assert!(window.state.uia.borrow().is_none());
    assert_eq!(
        unsafe { first.ProviderOptions() }.unwrap_err().code().0,
        UIA_E_ELEMENTNOTAVAILABLE as i32
    );
}

#[test]
fn uia_root_drop_and_external_destroy_revoke_retained_provider() {
    use windows::Win32::UI::WindowsAndMessaging::*;
    let _com = super::super::super::Apartment::new().unwrap();
    for external in [false, true] {
        let window = window();
        unsafe {
            SendMessageW(
                window.hwnd,
                WM_GETOBJECT,
                Some(WPARAM(0)),
                Some(LPARAM(UiaRootObjectId as isize)),
            )
        };
        let provider = window.state.uia.borrow().as_ref().unwrap().provider();
        if external {
            unsafe { DestroyWindow(window.hwnd).unwrap() };
        }
        drop(window);
        assert_eq!(
            unsafe { provider.HostRawElementProvider() }
                .unwrap_err()
                .code()
                .0,
            UIA_E_ELEMENTNOTAVAILABLE as i32
        );
    }
}
