//! Separate COM interface headers for Fragment/FragmentRoot, sharing canonical IUnknown and reference counts.
use super::*;
use windows::Win32::System::{Ole::*, Variant::VT_I4};

unsafe fn base(this: *mut c_void) -> *mut c_void {
    this.cast::<u8>()
        .sub(std::mem::offset_of!(Provider, fragment))
        .cast()
}
unsafe fn root_base(this: *mut c_void) -> *mut c_void {
    this.cast::<u8>()
        .sub(std::mem::offset_of!(Provider, fragment_root))
        .cast()
}
unsafe extern "system" fn query_fragment(
    this: *mut c_void,
    iid: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    query(base(this), iid, out)
}
unsafe extern "system" fn add_fragment(this: *mut c_void) -> u32 {
    add_ref(base(this))
}
unsafe extern "system" fn release_fragment(this: *mut c_void) -> u32 {
    release(base(this))
}
unsafe extern "system" fn query_root(
    this: *mut c_void,
    iid: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    query(root_base(this), iid, out)
}
unsafe extern "system" fn add_root(this: *mut c_void) -> u32 {
    add_ref(root_base(this))
}
unsafe extern "system" fn release_root(this: *mut c_void) -> u32 {
    release(root_base(this))
}

unsafe extern "system" fn navigate(
    this: *mut c_void,
    direction: NavigateDirection,
    out: *mut *mut c_void,
) -> HRESULT {
    if out.is_null() {
        return E_POINTER;
    }
    out.write(ptr::null_mut());
    let object = object(base(this));
    let status = available(object);
    if status.is_err() {
        return status;
    }
    match navigation::navigate(object, direction) {
        Ok(Some(provider)) => {
            out.write(provider.into_raw());
            S_OK
        }
        Ok(None) => S_OK,
        Err(error) => error.code(),
    }
}
unsafe extern "system" fn runtime(this: *mut c_void, out: *mut *mut SAFEARRAY) -> HRESULT {
    if out.is_null() {
        return E_POINTER;
    }
    out.write(ptr::null_mut());
    let object = object(base(this));
    let status = available(object);
    if status.is_err() {
        return status;
    }
    let Some(node) = &object.node else {
        return S_OK;
    };
    if navigation::native_edit(node) {
        // HWND-hosted elements use system runtime IDs; appending owner-drawn node IDs must not split them into a second element.
        return S_OK;
    }
    let Some(id) = node.runtime_id() else {
        return HRESULT(UIA_E_ELEMENTNOTAVAILABLE as i32);
    };
    let values = [UiaAppendRuntimeId as i32, id[0], id[1], id[2]];
    let array = SafeArrayCreateVector(VT_I4, 0, values.len() as u32);
    if array.is_null() {
        return E_OUTOFMEMORY;
    }
    for (index, value) in values.iter().enumerate() {
        if let Err(error) =
            SafeArrayPutElement(array, &(index as i32), (value as *const i32).cast())
        {
            let _ = SafeArrayDestroy(array);
            return error.code();
        }
    }
    // On success, array ownership transfers to the UIA caller; allocate an independent array per call.
    out.write(array);
    S_OK
}
unsafe extern "system" fn bounds(this: *mut c_void, out: *mut UiaRect) -> HRESULT {
    if out.is_null() {
        return E_POINTER;
    }
    out.write(UiaRect::default());
    let object = object(base(this));
    let status = available(object);
    if status.is_err() {
        return status;
    }
    if let Some(node) = &object.node {
        let result = super::super::super::accessibility_bridge::bounds(object.hwnd, node);
        let status = available(object);
        if status.is_err() {
            return status;
        }
        return match result {
            Ok((rect, _)) => {
                out.write(rect);
                S_OK
            }
            Err(error) => error.code(),
        };
    }
    let mut rect = RECT::default();
    if let Err(error) =
        windows::Win32::UI::WindowsAndMessaging::GetWindowRect(object.hwnd, &mut rect)
    {
        return error.code();
    }
    out.write(UiaRect {
        left: rect.left as f64,
        top: rect.top as f64,
        width: rect.right as f64 - rect.left as f64,
        height: rect.bottom as f64 - rect.top as f64,
    });
    S_OK
}
unsafe extern "system" fn embedded(this: *mut c_void, out: *mut *mut SAFEARRAY) -> HRESULT {
    if out.is_null() {
        return E_POINTER;
    }
    out.write(ptr::null_mut());
    available(object(base(this)))
}
unsafe extern "system" fn focus(this: *mut c_void) -> HRESULT {
    let object = object(base(this));
    let status = available(object);
    if status.is_err() {
        return status;
    }
    match navigation::set_focus(object) {
        Ok(()) => available(object),
        Err(error) => error.code(),
    }
}
unsafe extern "system" fn fragment_root(this: *mut c_void, out: *mut *mut c_void) -> HRESULT {
    if out.is_null() {
        return E_POINTER;
    }
    out.write(ptr::null_mut());
    let object = object(base(this));
    let status = available(object);
    if status.is_err() {
        return status;
    }
    match navigation::root(object).cast::<IRawElementProviderFragmentRoot>() {
        Ok(root) => {
            out.write(root.into_raw());
            S_OK
        }
        Err(error) => error.code(),
    }
}
unsafe extern "system" fn get_focus(this: *mut c_void, out: *mut *mut c_void) -> HRESULT {
    if out.is_null() {
        return E_POINTER;
    }
    out.write(ptr::null_mut());
    let object = object(root_base(this));
    let status = available(object);
    if status.is_err() {
        return status;
    }
    let result = navigation::focused(object);
    let status = available(object);
    if status.is_err() {
        return status;
    }
    match result {
        Ok(Some(provider)) => {
            out.write(provider.into_raw());
            S_OK
        }
        Ok(None) => S_OK,
        Err(error) => error.code(),
    }
}
unsafe extern "system" fn point(
    this: *mut c_void,
    x: f64,
    y: f64,
    out: *mut *mut c_void,
) -> HRESULT {
    if out.is_null() {
        return E_POINTER;
    }
    out.write(ptr::null_mut());
    let object = object(root_base(this));
    let status = available(object);
    if status.is_err() {
        return status;
    }
    let result = navigation::point(object, [x, y]);
    let status = available(object);
    if status.is_err() {
        return status;
    }
    match result {
        Ok(Some(provider)) => {
            out.write(provider.into_raw());
            S_OK
        }
        Ok(None) => S_OK,
        Err(error) => error.code(),
    }
}
pub(super) static VTABLE: IRawElementProviderFragment_Vtbl = IRawElementProviderFragment_Vtbl {
    base__: IUnknown_Vtbl {
        QueryInterface: query_fragment,
        AddRef: add_fragment,
        Release: release_fragment,
    },
    Navigate: navigate,
    GetRuntimeId: runtime,
    BoundingRectangle: bounds,
    GetEmbeddedFragmentRoots: embedded,
    SetFocus: focus,
    FragmentRoot: fragment_root,
};
pub(super) static ROOT_VTABLE: IRawElementProviderFragmentRoot_Vtbl =
    IRawElementProviderFragmentRoot_Vtbl {
        base__: IUnknown_Vtbl {
            QueryInterface: query_root,
            AddRef: add_root,
            Release: release_root,
        },
        ElementProviderFromPoint: point,
        GetFocus: get_focus,
    };
