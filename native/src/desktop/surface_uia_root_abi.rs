//! Narrow ABI owner for IRawElementProviderSimple. Generated windows 0.60 traits cannot represent
//! S_OK + NULL patterns; implement that contract explicitly rather than creating empty non-null Rust COM interfaces.
use super::*;
use std::{
    ffi::c_void,
    ptr,
    sync::atomic::{fence, AtomicU32},
};
use windows::{
    core::{IUnknown, IUnknown_Vtbl, Interface, GUID, HRESULT},
    Win32::System::{Threading::GetCurrentThreadId, Variant::VARIANT},
};
#[path = "surface_uia_fragment_abi.rs"]
mod fragment;
#[path = "surface_uia_invoke_abi.rs"]
mod invoke;
#[cfg(test)]
static INVOKE_NOTIFICATIONS: AtomicU32 = AtomicU32::new(0);
#[cfg(test)]
pub(super) fn invoke_notifications() -> u32 {
    INVOKE_NOTIFICATIONS.load(Ordering::Acquire)
}
#[cfg(test)]
static TOGGLE_NOTIFICATIONS: AtomicU32 = AtomicU32::new(0);
#[cfg(test)]
pub(super) fn toggle_notifications() -> u32 {
    TOGGLE_NOTIFICATIONS.load(Ordering::Acquire)
}
#[path = "surface_uia_hwnd_override_abi.rs"]
mod hwnd_override;
#[path = "surface_uia_fragment_navigation.rs"]
mod navigation;
#[path = "surface_uia_range_abi.rs"]
mod range;
#[path = "surface_uia_toggle_abi.rs"]
mod toggle;
pub(super) fn node(
    provider: &IRawElementProviderSimple,
    handle: crate::surface_accessibility::Handle,
) -> Result<IRawElementProviderSimple> {
    unsafe {
        let object = object(provider.as_raw());
        available(object).ok()?;
        Ok(navigation::node(object, handle))
    }
}
pub(super) fn notify_toggle(provider: &IRawElementProviderSimple, before: bool, after: bool) {
    unsafe {
        toggle::notify(provider, before, after);
    }
}
pub(super) fn notify_range(provider: &IRawElementProviderSimple, before: f64, after: f64) {
    unsafe { range::notify(provider, before, after) };
}
pub(super) fn notify_range_readonly(
    provider: &IRawElementProviderSimple,
    before: bool,
    after: bool,
) {
    unsafe { range::notify_readonly(provider, before, after) };
}

#[repr(C)]
struct Provider {
    vtable: *const IRawElementProviderSimple_Vtbl,
    fragment: *const IRawElementProviderFragment_Vtbl,
    fragment_root: *const IRawElementProviderFragmentRoot_Vtbl,
    hwnd_override: *const IRawElementProviderHwndOverride_Vtbl,
    invoke: *const IInvokeProvider_Vtbl,
    invokable: bool,
    toggle: *const IToggleProvider_Vtbl,
    toggleable: bool,
    range: *const IRangeValueProvider_Vtbl,
    rangeable: bool,
    references: AtomicU32,
    alive: Arc<AtomicBool>,
    hwnd: HWND,
    thread: u32,
    scene: Arc<Mutex<Option<crate::surface_accessibility::Handle>>>,
    node: Option<crate::surface_accessibility::Handle>,
    root: Option<IRawElementProviderSimple>,
}
pub(super) fn create(
    hwnd: HWND,
    alive: Arc<AtomicBool>,
    scene: Arc<Mutex<Option<crate::surface_accessibility::Handle>>>,
) -> IRawElementProviderSimple {
    let object = Box::new(Provider {
        vtable: &VTABLE,
        fragment: &fragment::VTABLE,
        fragment_root: &fragment::ROOT_VTABLE,
        hwnd_override: &hwnd_override::VTABLE,
        invoke: &invoke::VTABLE,
        invokable: false,
        toggle: &toggle::VTABLE,
        toggleable: false,
        range: &range::VTABLE,
        rangeable: false,
        references: AtomicU32::new(1),
        alive,
        hwnd,
        thread: unsafe { GetCurrentThreadId() },
        scene,
        node: None,
        root: None,
    });
    // The first repr(C) field is a static vtable; transfer the initial reference to COM, and final Release reconstructs Box.
    unsafe { IRawElementProviderSimple::from_raw(Box::into_raw(object).cast()) }
}
unsafe fn object<'a>(this: *mut c_void) -> &'a Provider {
    // All entries are invoked only through valid COM interfaces with at least one reference held during calls.
    &*this.cast::<Provider>()
}
fn available(object: &Provider) -> HRESULT {
    if !object.alive.load(Ordering::Acquire) {
        HRESULT(UIA_E_ELEMENTNOTAVAILABLE as i32)
    } else if unsafe { GetCurrentThreadId() } != object.thread {
        RPC_E_WRONG_THREAD
    } else if object
        .node
        .as_ref()
        .is_some_and(|node| node.runtime_id().is_none())
    {
        HRESULT(UIA_E_ELEMENTNOTAVAILABLE as i32)
    } else {
        S_OK
    }
}
unsafe extern "system" fn query(
    this: *mut c_void,
    iid: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    if out.is_null() {
        return E_POINTER;
    }
    out.write(ptr::null_mut());
    if iid.is_null() {
        return E_POINTER;
    }
    let result = if *iid == IUnknown::IID || *iid == IRawElementProviderSimple::IID {
        this
    } else if *iid == IRawElementProviderFragment::IID {
        ptr::addr_of!((*this.cast::<Provider>()).fragment)
            .cast_mut()
            .cast()
    } else if *iid == IRawElementProviderFragmentRoot::IID && object(this).node.is_none() {
        ptr::addr_of!((*this.cast::<Provider>()).fragment_root)
            .cast_mut()
            .cast()
    } else if *iid == IRawElementProviderHwndOverride::IID && object(this).node.is_none() {
        ptr::addr_of!((*this.cast::<Provider>()).hwnd_override)
            .cast_mut()
            .cast()
    } else if *iid == IInvokeProvider::IID && object(this).invokable {
        ptr::addr_of!((*this.cast::<Provider>()).invoke)
            .cast_mut()
            .cast()
    } else if *iid == IToggleProvider::IID && object(this).toggleable {
        ptr::addr_of!((*this.cast::<Provider>()).toggle)
            .cast_mut()
            .cast()
    } else if *iid == IRangeValueProvider::IID && object(this).rangeable {
        ptr::addr_of!((*this.cast::<Provider>()).range)
            .cast_mut()
            .cast()
    } else {
        return E_NOINTERFACE;
    };
    add_ref(this);
    out.write(result);
    S_OK
}
unsafe extern "system" fn add_ref(this: *mut c_void) -> u32 {
    let previous = object(this).references.fetch_add(1, Ordering::Relaxed);
    // Never let counts wrap or panics cross the COM ABI.
    if previous >= i32::MAX as u32 {
        std::process::abort();
    }
    previous + 1
}
unsafe extern "system" fn release(this: *mut c_void) -> u32 {
    let remaining = object(this).references.fetch_sub(1, Ordering::Release) - 1;
    if remaining == 0 {
        fence(Ordering::Acquire);
        drop(Box::from_raw(this.cast::<Provider>()));
    }
    remaining
}
unsafe extern "system" fn options(this: *mut c_void, out: *mut ProviderOptions) -> HRESULT {
    if out.is_null() {
        return E_POINTER;
    }
    out.write(ProviderOptions::default());
    let status = available(object(this));
    if status.is_err() {
        return status;
    }
    let mut options = ProviderOptions_ServerSideProvider | ProviderOptions_UseComThreading;
    // Owner-drawn Sliders must use their own child-HWND focus bridge, not let UIA pick the nearest window.
    if object(this).node.is_none() || object(this).rangeable {
        options |= ProviderOptions_ProviderOwnsSetFocus;
    }
    if hwnd_override::is_edit(object(this)) {
        options |= ProviderOptions_OverrideProvider;
    }
    out.write(options);
    S_OK
}
unsafe extern "system" fn pattern(
    this: *mut c_void,
    id: UIA_PATTERN_ID,
    out: *mut *mut c_void,
) -> HRESULT {
    if out.is_null() {
        return E_POINTER;
    }
    out.write(ptr::null_mut());
    let status = available(object(this));
    if status.is_err() {
        return status;
    }
    if id == UIA_InvokePatternId && object(this).invokable {
        return query(this, &IInvokeProvider::IID, out);
    }
    if id == UIA_TogglePatternId && object(this).toggleable {
        return query(this, &IToggleProvider::IID, out);
    }
    if id == UIA_RangeValuePatternId && object(this).rangeable {
        return query(this, &IRangeValueProvider::IID, out);
    }
    S_OK
}
unsafe extern "system" fn property(
    this: *mut c_void,
    id: UIA_PROPERTY_ID,
    out: *mut VARIANT,
) -> HRESULT {
    if out.is_null() {
        return E_POINTER;
    }
    out.write(VARIANT::default());
    let status = available(object(this));
    if status.is_err() {
        return status;
    }
    if hwnd_override::is_edit(object(this)) && !object(this).rangeable {
        // System providers supply EDIT patterns/focus/values/geometry; supplement only declarative identity and labels here.
        if let Some(entry) = object(this).node.as_ref().and_then(|node| node.read()) {
            if id == UIA_AutomationIdPropertyId
                || id == UIA_FrameworkIdPropertyId
                || (id == UIA_NamePropertyId && entry.label.is_some())
            {
                out.write(navigation::property(&entry, id));
            }
        }
        return S_OK;
    }
    if id == UIA_IsRangeValuePatternAvailablePropertyId {
        out.write(VARIANT::from(object(this).rangeable));
        return S_OK;
    }
    if object(this).rangeable {
        match range::property(object(this), id) {
            Ok(Some(value)) => {
                out.write(value);
                return S_OK;
            }
            Ok(None) => {}
            Err(error) => return error.code(),
        }
    }
    if id == UIA_IsTogglePatternAvailablePropertyId {
        out.write(VARIANT::from(object(this).toggleable));
        return S_OK;
    }
    if id == UIA_ToggleToggleStatePropertyId && object(this).toggleable {
        return match toggle::value(object(this)) {
            Ok(value) => {
                out.write(VARIANT::from(value.0));
                S_OK
            }
            Err(error) => error.code(),
        };
    }
    if id == UIA_IsInvokePatternAvailablePropertyId {
        out.write(VARIANT::from(object(this).invokable));
        return S_OK;
    }
    match navigation::focus_property(object(this), id) {
        Ok(Some(value)) => {
            out.write(value);
            return available(object(this));
        }
        Ok(None) => {}
        Err(error) => return error.code(),
    }
    let value = if let Some(node) = object(this).node.as_ref() {
        if id == UIA_IsOffscreenPropertyId {
            let result = super::super::accessibility_bridge::offscreen(object(this).hwnd, node);
            let status = available(object(this));
            if status.is_err() {
                return status;
            }
            match result {
                Ok(value) => {
                    out.write(VARIANT::from(value));
                    return S_OK;
                }
                Err(error) => return error.code(),
            }
        }
        let Some(entry) = node.read() else {
            return HRESULT(UIA_E_ELEMENTNOTAVAILABLE as i32);
        };
        navigation::property(&entry, id)
    } else if id == UIA_NamePropertyId {
        VARIANT::from("Otter Surface")
    } else if id == UIA_FrameworkIdPropertyId {
        VARIANT::from("Otter.Native")
    } else if id == UIA_ControlTypePropertyId {
        VARIANT::from(UIA_PaneControlTypeId.0)
    } else {
        VARIANT::default()
    };
    // Return VT_EMPTY for unsupported properties, delegating to the HWND host provider; fabricate no focus/enabled/geometry.
    out.write(value);
    S_OK
}
unsafe extern "system" fn host(this: *mut c_void, out: *mut *mut c_void) -> HRESULT {
    if out.is_null() {
        return E_POINTER;
    }
    out.write(ptr::null_mut());
    let object = object(this);
    let status = available(object);
    if status.is_err() {
        return status;
    }
    let hwnd = if object.node.is_none() {
        object.hwnd
    } else {
        match hwnd_override::hwnd(object) {
            Ok(Some(hwnd)) => hwnd,
            Ok(None) => return S_OK,
            Err(error) => return error.code(),
        }
    };
    match UiaHostProviderFromHwnd(hwnd) {
        Ok(provider) => {
            // System calls may reenter closure; recheck before returning HWND providers after revocation.
            let status = available(object);
            if status.is_err() {
                return status;
            }
            if hwnd_override::is_edit(object)
                && hwnd_override::hwnd(object).ok().flatten() != Some(hwnd)
            {
                return HRESULT(UIA_E_ELEMENTNOTAVAILABLE as i32);
            }
            out.write(provider.into_raw());
            S_OK
        }
        Err(error) => error.code(),
    }
}
static VTABLE: IRawElementProviderSimple_Vtbl = IRawElementProviderSimple_Vtbl {
    base__: IUnknown_Vtbl {
        QueryInterface: query,
        AddRef: add_ref,
        Release: release,
    },
    ProviderOptions: options,
    GetPatternProvider: pattern,
    GetPropertyValue: property,
    HostRawElementProvider: host,
};
