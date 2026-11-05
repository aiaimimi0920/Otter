//! Invoke 接口复用节点 IUnknown/引用计数；业务执行留给既有异步候选消费链。
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
unsafe extern "system" fn add_invoke(this: *mut c_void) -> u32 {
    add_ref(base(this))
}
unsafe extern "system" fn release_invoke(this: *mut c_void) -> u32 {
    release(base(this))
}
unsafe extern "system" fn invoke(this: *mut c_void) -> HRESULT {
    let raw = base(this);
    let object = object(raw);
    let status = available(object);
    if status.is_err() {
        return status;
    }
    let Some(node) = &object.node else {
        return HRESULT(UIA_E_NOTSUPPORTED as i32);
    };
    if let Err(error) =
        super::super::super::accessibility_bridge::invoke::enqueue(object.hwnd, node)
    {
        return error.code();
    }
    // 动作可能需要远端响应或用户确认，事件表示已接受请求，不冒充业务完成。
    // 不持 State/RefCell 借用跨 UIA 回调；通知失败不重放已入槽请求。
    if UiaClientsAreListening().as_bool() {
        #[cfg(test)]
        INVOKE_NOTIFICATIONS.fetch_add(1, Ordering::Release);
        add_ref(raw);
        let simple = IRawElementProviderSimple::from_raw(raw);
        let _ = UiaRaiseAutomationEvent(&simple, UIA_Invoke_InvokedEventId);
    }
    S_OK
}
pub(super) static VTABLE: IInvokeProvider_Vtbl = IInvokeProvider_Vtbl {
    base__: IUnknown_Vtbl {
        QueryInterface: query_invoke,
        AddRef: add_invoke,
        Release: release_invoke,
    },
    Invoke: invoke,
};
