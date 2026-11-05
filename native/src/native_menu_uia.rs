//! 菜单专用 COM owner；STA 回调只读可撤销快照或排队，不借用菜单/业务模型。
use super::accessibility::{Action, Node, Rejected, View};
use std::{
    ffi::c_void,
    ptr,
    sync::atomic::{fence, AtomicU32, Ordering},
};
use windows::{
    core::{Error, IUnknown, IUnknown_Vtbl, Interface, Result, GUID, HRESULT},
    Win32::{
        Foundation::*,
        System::{Com::*, Threading::GetCurrentThreadId, Variant::VARIANT},
        UI::Accessibility::*,
    },
};
#[path = "native_menu_uia_events.rs"]
mod events;
#[path = "native_menu_uia_expand.rs"]
mod expand;
#[path = "native_menu_uia_fragment.rs"]
mod fragment;
#[path = "native_menu_uia_invoke.rs"]
mod invoke;
#[path = "native_menu_uia_scroll.rs"]
mod scroll;
pub(super) use events::{changes, notify, Snapshot};
pub(super) fn notify_invoked(provider: &IRawElementProviderSimple, node: Node) {
    unsafe {
        let object = object(provider.as_raw());
        if available(object).is_ok() && node.read().is_some() {
            // 表示菜单选择已被接受，外层文档目标准入和业务结果仍独立核验。
            let _ = UiaRaiseAutomationEvent(&child(object, node), UIA_Invoke_InvokedEventId);
        }
    }
}

#[repr(C)]
struct Provider {
    simple: *const IRawElementProviderSimple_Vtbl,
    fragment: *const IRawElementProviderFragment_Vtbl,
    fragment_root: *const IRawElementProviderFragmentRoot_Vtbl,
    invoke: *const IInvokeProvider_Vtbl,
    expand: *const IExpandCollapseProvider_Vtbl,
    scroll: *const IScrollProvider_Vtbl,
    references: AtomicU32,
    hwnd: HWND,
    thread: u32,
    row: i32,
    view: View,
    node: Option<Node>,
    root: Option<IRawElementProviderSimple>,
    invokable: bool,
    expandable: bool,
}
pub(super) fn create(hwnd: HWND, view: View, row: i32) -> Result<IRawElementProviderSimple> {
    let mut apartment = APTTYPE::default();
    let mut qualifier = APTTYPEQUALIFIER::default();
    unsafe {
        CoGetApartmentType(&mut apartment, &mut qualifier)?;
    }
    if apartment != APTTYPE_STA && apartment != APTTYPE_MAINSTA {
        return Err(Error::from_hresult(RPC_E_WRONG_THREAD));
    }
    Ok(allocate(hwnd, row, view, None, None))
}
fn allocate(
    hwnd: HWND,
    row: i32,
    view: View,
    node: Option<Node>,
    root: Option<IRawElementProviderSimple>,
) -> IRawElementProviderSimple {
    let invokable = node
        .as_ref()
        .and_then(Node::read)
        .is_some_and(|entry| !entry.separator && !entry.submenu);
    let expandable = node
        .as_ref()
        .and_then(Node::read)
        .is_some_and(|entry| !entry.separator && entry.submenu);
    let object = Box::new(Provider {
        simple: &VTABLE,
        fragment: &fragment::VTABLE,
        fragment_root: &fragment::ROOT_VTABLE,
        invoke: &invoke::VTABLE,
        expand: &expand::VTABLE,
        scroll: &scroll::VTABLE,
        references: AtomicU32::new(1),
        hwnd,
        thread: unsafe { GetCurrentThreadId() },
        row,
        view,
        node,
        root,
        invokable,
        expandable,
    });
    unsafe { IRawElementProviderSimple::from_raw(Box::into_raw(object).cast()) }
}
unsafe fn object<'a>(this: *mut c_void) -> &'a Provider {
    &*this.cast::<Provider>()
}
fn available(object: &Provider) -> HRESULT {
    if !object.view.alive()
        || object
            .node
            .as_ref()
            .is_some_and(|node| node.read().is_none())
    {
        HRESULT(UIA_E_ELEMENTNOTAVAILABLE as i32)
    } else if unsafe { GetCurrentThreadId() } != object.thread {
        RPC_E_WRONG_THREAD
    } else {
        S_OK
    }
}
unsafe fn root(object: &Provider) -> IRawElementProviderSimple {
    if let Some(root) = &object.root {
        return root.clone();
    }
    let raw = (object as *const Provider).cast_mut().cast();
    add_ref(raw);
    IRawElementProviderSimple::from_raw(raw)
}
unsafe fn child(object: &Provider, node: Node) -> IRawElementProviderSimple {
    allocate(
        object.hwnd,
        object.row,
        object.view.clone(),
        Some(node),
        Some(root(object)),
    )
}
fn linked(view: View) -> Result<IRawElementProviderSimple> {
    let (hwnd, row) = view
        .location()
        .ok_or_else(|| Error::from_hresult(HRESULT(UIA_E_ELEMENTNOTAVAILABLE as i32)))?;
    create(HWND(hwnd as *mut _), view, row)
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
    let provider = this.cast::<Provider>();
    let result = if *iid == IUnknown::IID || *iid == IRawElementProviderSimple::IID {
        this
    } else if *iid == IRawElementProviderFragment::IID {
        ptr::addr_of_mut!((*provider).fragment).cast()
    } else if *iid == IRawElementProviderFragmentRoot::IID && object(this).node.is_none() {
        ptr::addr_of_mut!((*provider).fragment_root).cast()
    } else if *iid == IInvokeProvider::IID && object(this).invokable {
        ptr::addr_of_mut!((*provider).invoke).cast()
    } else if *iid == IExpandCollapseProvider::IID && object(this).expandable {
        ptr::addr_of_mut!((*provider).expand).cast()
    } else if *iid == IScrollProvider::IID && object(this).node.is_none() {
        ptr::addr_of_mut!((*provider).scroll).cast()
    } else {
        return E_NOINTERFACE;
    };
    add_ref(this);
    out.write(result);
    S_OK
}
unsafe extern "system" fn add_ref(this: *mut c_void) -> u32 {
    let previous = object(this).references.fetch_add(1, Ordering::Relaxed);
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
    out.write(
        ProviderOptions_ServerSideProvider
            | ProviderOptions_UseComThreading
            | ProviderOptions_ProviderOwnsSetFocus,
    );
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
    if id == UIA_ExpandCollapsePatternId && object(this).expandable {
        return query(this, &IExpandCollapseProvider::IID, out);
    }
    if id == UIA_ScrollPatternId && object(this).node.is_none() {
        return query(this, &IScrollProvider::IID, out);
    }
    // 未支持模式必须是 S_OK + NULL，不能伪造非空 Rust COM 接口。
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
    let object = object(this);
    let status = available(object);
    if status.is_err() {
        return status;
    }
    let entry = object.node.as_ref().and_then(Node::read);
    let value = if id == UIA_NamePropertyId {
        VARIANT::from(
            entry
                .as_ref()
                .map_or("Otter 菜单", |entry| entry.name.as_str()),
        )
    } else if id == UIA_FrameworkIdPropertyId {
        VARIANT::from("Otter.Native")
    } else if id == UIA_ControlTypePropertyId {
        VARIANT::from(match entry.as_ref() {
            None => UIA_MenuControlTypeId.0,
            Some(entry) if entry.separator => UIA_SeparatorControlTypeId.0,
            Some(_) => UIA_MenuItemControlTypeId.0,
        })
    } else if id == UIA_IsEnabledPropertyId {
        VARIANT::from(entry.as_ref().is_none_or(|entry| entry.enabled))
    } else if id == UIA_IsKeyboardFocusablePropertyId {
        VARIANT::from(
            entry
                .as_ref()
                .is_some_and(|entry| entry.enabled && !entry.separator),
        )
    } else if id == UIA_HasKeyboardFocusPropertyId {
        VARIANT::from(entry.as_ref().is_some_and(|entry| entry.focused))
    } else if id == UIA_IsControlElementPropertyId || id == UIA_IsContentElementPropertyId {
        VARIANT::from(entry.as_ref().is_none_or(|entry| !entry.separator))
    } else if id == UIA_IsInvokePatternAvailablePropertyId {
        VARIANT::from(object.invokable)
    } else if id == UIA_IsExpandCollapsePatternAvailablePropertyId {
        VARIANT::from(object.expandable)
    } else if id == UIA_IsScrollPatternAvailablePropertyId {
        VARIANT::from(object.node.is_none())
    } else if id == UIA_AcceleratorKeyPropertyId {
        VARIANT::from(entry.as_ref().map_or("", |entry| entry.shortcut.as_str()))
    } else if id == UIA_ItemStatusPropertyId {
        // HMENU 未勾选项不携带 checkbox 角色；不能把普通命令猜成未选中开关。
        // 仅呈现已知标记，不承诺 Toggle/SelectionItem 的操作或分组语义。
        VARIANT::from(match entry.as_ref() {
            Some(entry) if !entry.separator && entry.radio && entry.checked => "已选中",
            Some(entry) if !entry.separator && entry.radio => "未选中",
            Some(entry) if !entry.separator && entry.checked => "已勾选",
            _ => "",
        })
    } else {
        VARIANT::default()
    };
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
    if object.node.is_some() {
        return S_OK;
    }
    match UiaHostProviderFromHwnd(object.hwnd) {
        Ok(provider) => {
            let status = available(object);
            if status.is_err() {
                return status;
            }
            out.write(provider.into_raw());
            S_OK
        }
        Err(error) => error.code(),
    }
}
fn enqueue(object: &Provider, action: Action) -> HRESULT {
    let status = available(object);
    if status.is_err() {
        return status;
    }
    let Some(node) = &object.node else {
        return HRESULT(UIA_E_NOTSUPPORTED as i32);
    };
    queued(node.enqueue(action))
}
fn queued(result: std::result::Result<(), Rejected>) -> HRESULT {
    match result {
        Ok(()) => S_OK,
        Err(Rejected::Unavailable) => HRESULT(UIA_E_ELEMENTNOTAVAILABLE as i32),
        Err(Rejected::Disabled) => HRESULT(UIA_E_ELEMENTNOTENABLED as i32),
        Err(Rejected::Unsupported) => HRESULT(UIA_E_NOTSUPPORTED as i32),
        Err(Rejected::Busy) => HRESULT(UIA_E_INVALIDOPERATION as i32),
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

#[cfg(test)]
#[path = "native_menu_uia_tests.rs"]
mod tests;
