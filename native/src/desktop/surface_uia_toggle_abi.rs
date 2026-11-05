//! Switch Toggle ABI；状态来自实时会话，通知在提交后且无 Host/RefCell 借用时发送。
use super::*;
fn state(value: bool) -> ToggleState {
    if value {
        ToggleState_On
    } else {
        ToggleState_Off
    }
}
pub(super) fn value(object: &Provider) -> Result<ToggleState> {
    let node = object
        .node
        .as_ref()
        .ok_or_else(|| Error::from_hresult(HRESULT(UIA_E_NOTSUPPORTED as i32)))?;
    super::super::super::accessibility_bridge::toggle::value(object.hwnd, node).map(state)
}
unsafe fn base(this: *mut c_void) -> *mut c_void {
    this.cast::<u8>()
        .sub(std::mem::offset_of!(Provider, toggle))
        .cast()
}
unsafe extern "system" fn query_toggle(
    this: *mut c_void,
    iid: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    query(base(this), iid, out)
}
unsafe extern "system" fn add_toggle(this: *mut c_void) -> u32 {
    add_ref(base(this))
}
unsafe extern "system" fn release_toggle(this: *mut c_void) -> u32 {
    release(base(this))
}
unsafe extern "system" fn current(this: *mut c_void, out: *mut ToggleState) -> HRESULT {
    if out.is_null() {
        return E_POINTER;
    }
    out.write(ToggleState_Off);
    let object = object(base(this));
    let status = available(object);
    if status.is_err() {
        return status;
    }
    match value(object) {
        Ok(value) => {
            out.write(value);
            S_OK
        }
        Err(error) => error.code(),
    }
}
unsafe extern "system" fn toggle(this: *mut c_void) -> HRESULT {
    let raw = base(this);
    let object = object(raw);
    let status = available(object);
    if status.is_err() {
        return status;
    }
    let Some(node) = &object.node else {
        return HRESULT(UIA_E_NOTSUPPORTED as i32);
    };
    match super::super::super::accessibility_bridge::toggle::change(object.hwnd, node) {
        Ok(_) => S_OK,
        Err(error) => error.code(),
    }
}
pub(super) unsafe fn notify(provider: &IRawElementProviderSimple, before: bool, after: bool) {
    if UiaClientsAreListening().as_bool() {
        #[cfg(test)]
        TOGGLE_NOTIFICATIONS.fetch_add(1, Ordering::Release);
        let _ = UiaRaiseAutomationPropertyChangedEvent(
            provider,
            UIA_ToggleToggleStatePropertyId,
            &VARIANT::from(state(before).0),
            &VARIANT::from(state(after).0),
        );
    }
}
pub(super) static VTABLE: IToggleProvider_Vtbl = IToggleProvider_Vtbl {
    base__: IUnknown_Vtbl {
        QueryInterface: query_toggle,
        AddRef: add_toggle,
        Release: release_toggle,
    },
    Toggle: toggle,
    ToggleState: current,
};
