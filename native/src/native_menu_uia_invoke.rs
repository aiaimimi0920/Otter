//! Invoke 只提交请求；通知或业务结果不用于重放操作。
use super::*;
unsafe fn base(this: *mut c_void) -> *mut c_void {
    this.cast::<u8>()
        .sub(std::mem::offset_of!(Provider, invoke))
        .cast()
}
unsafe extern "system" fn query_invoke(
    this: *mut c_void,
    iid: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    query(base(this), iid, out)
}
unsafe extern "system" fn add(this: *mut c_void) -> u32 {
    add_ref(base(this))
}
unsafe extern "system" fn drop_ref(this: *mut c_void) -> u32 {
    release(base(this))
}
unsafe extern "system" fn invoke(this: *mut c_void) -> HRESULT {
    enqueue(object(base(this)), Action::Invoke)
}
pub(super) static VTABLE: IInvokeProvider_Vtbl = IInvokeProvider_Vtbl {
    base__: IUnknown_Vtbl {
        QueryInterface: query_invoke,
        AddRef: add,
        Release: drop_ref,
    },
    Invoke: invoke,
};
