//! 根的 HWND relocation ABI；未知 HWND 为 S_OK + NULL，接口身份共享根 IUnknown。
use super::super::super::accessibility_bridge::native_edit;
use super::*;
pub(super) fn is_edit(object: &Provider) -> bool {
    object.node.as_ref().is_some_and(navigation::native_edit)
}
pub(super) fn hwnd(object: &Provider) -> Result<Option<HWND>> {
    match object.node.as_ref().filter(|_| is_edit(object)) {
        Some(node) => native_edit::hwnd(object.hwnd, node),
        None => Ok(None),
    }
}
unsafe fn base(this: *mut c_void) -> *mut c_void {
    this.cast::<u8>()
        .sub(std::mem::offset_of!(Provider, hwnd_override))
        .cast()
}
unsafe extern "system" fn query_override(
    this: *mut c_void,
    iid: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    query(base(this), iid, out)
}
unsafe extern "system" fn add_override(this: *mut c_void) -> u32 {
    add_ref(base(this))
}
unsafe extern "system" fn release_override(this: *mut c_void) -> u32 {
    release(base(this))
}
unsafe extern "system" fn get(this: *mut c_void, hwnd: HWND, out: *mut *mut c_void) -> HRESULT {
    if out.is_null() {
        return E_POINTER;
    }
    out.write(ptr::null_mut());
    let object = object(base(this));
    let status = available(object);
    if status.is_err() {
        return status;
    }
    match native_edit::node(object.hwnd, hwnd) {
        Ok(Some(handle)) => {
            out.write(navigation::node(object, handle).into_raw());
            S_OK
        }
        Ok(None) => S_OK,
        Err(error) => error.code(),
    }
}
pub(super) static VTABLE: IRawElementProviderHwndOverride_Vtbl =
    IRawElementProviderHwndOverride_Vtbl {
        base__: IUnknown_Vtbl {
            QueryInterface: query_override,
            AddRef: add_override,
            Release: release_override,
        },
        GetOverrideProviderForHwnd: get,
    };
