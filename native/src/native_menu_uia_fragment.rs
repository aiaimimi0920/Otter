//! UIA Fragment navigation/current-window geometry without retaining external nodes or invoking business actions.
use super::*;
use windows::Win32::{
    System::{Ole::*, Variant::VT_I4},
    UI::WindowsAndMessaging::GetWindowRect,
};
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
unsafe extern "system" fn add(this: *mut c_void) -> u32 {
    add_ref(base(this))
}
unsafe extern "system" fn drop_ref(this: *mut c_void) -> u32 {
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
unsafe extern "system" fn drop_root(this: *mut c_void) -> u32 {
    release(root_base(this))
}

unsafe fn output(provider: IRawElementProviderSimple, out: *mut *mut c_void) -> HRESULT {
    match provider.cast::<IRawElementProviderFragment>() {
        Ok(provider) => {
            out.write(provider.into_raw());
            S_OK
        }
        Err(error) => error.code(),
    }
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
    if !(0..=4).contains(&direction.0) {
        return E_INVALIDARG;
    }
    if object.node.is_some() && direction == NavigateDirection_Parent {
        return output(root(object), out);
    }
    // Popup HWNDs still use their own host providers; only logical parents change.
    if object.node.is_none() && direction == NavigateDirection_Parent {
        if let Some((view, node)) = object.view.parent() {
            return match linked(view) {
                Ok(parent) => output(child(super::object(parent.as_raw()), node), out),
                Err(error) => error.code(),
            };
        }
    }
    if matches!(direction.0, 3 | 4) {
        if let Some(view) = object.node.as_ref().and_then(Node::child) {
            return match linked(view) {
                Ok(child) => output(child, out),
                Err(error) => error.code(),
            };
        }
    }
    let target = match &object.node {
        None if direction == NavigateDirection_FirstChild => object.view.node(0),
        None if direction == NavigateDirection_LastChild => object.view.last_node(),
        Some(node) if direction == NavigateDirection_NextSibling => node.sibling(true),
        Some(node) if direction == NavigateDirection_PreviousSibling => node.sibling(false),
        _ => None,
    };
    match target {
        Some(node) => output(child(object, node), out),
        None => S_OK,
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
    let Some(id) = node.runtime_id() else {
        return HRESULT(UIA_E_ELEMENTNOTAVAILABLE as i32);
    };
    let values = [UiaAppendRuntimeId as i32, id[0], id[1], id[2], id[3], id[4]];
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
    out.write(array);
    S_OK
}
unsafe fn rect(object: &Provider) -> Result<UiaRect> {
    let mut rect = RECT::default();
    GetWindowRect(object.hwnd, &mut rect)?;
    available(object).ok()?;
    if let Some(node) = &object.node {
        let local = node
            .item_rect(rect.right - rect.left, object.row)
            .ok_or_else(|| Error::from_hresult(HRESULT(UIA_E_ELEMENTNOTAVAILABLE as i32)))?;
        rect = RECT {
            left: rect.left + local.left,
            right: rect.left + local.right,
            top: rect.top + local.top,
            bottom: (rect.top + local.bottom).min(rect.bottom),
        };
    }
    Ok(UiaRect {
        left: rect.left as f64,
        top: rect.top as f64,
        width: (rect.right - rect.left).max(0) as f64,
        height: (rect.bottom - rect.top).max(0) as f64,
    })
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
    match rect(object) {
        Ok(rect) => {
            out.write(rect);
            S_OK
        }
        Err(error) => error.code(),
    }
}
unsafe extern "system" fn embedded(this: *mut c_void, out: *mut *mut SAFEARRAY) -> HRESULT {
    if out.is_null() {
        return E_POINTER;
    }
    out.write(ptr::null_mut());
    available(object(base(this)))
}
unsafe extern "system" fn focus(this: *mut c_void) -> HRESULT {
    enqueue(object(base(this)), Action::Focus)
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
    match root(object).cast::<IRawElementProviderFragmentRoot>() {
        Ok(provider) => {
            out.write(provider.into_raw());
            S_OK
        }
        Err(error) => error.code(),
    }
}
unsafe extern "system" fn focused(this: *mut c_void, out: *mut *mut c_void) -> HRESULT {
    if out.is_null() {
        return E_POINTER;
    }
    out.write(ptr::null_mut());
    let object = object(root_base(this));
    let status = available(object);
    if status.is_err() {
        return status;
    }
    match object.view.focused() {
        Some((view, node)) if view.same(&object.view) => output(child(object, node), out),
        Some((view, node)) => match linked(view) {
            Ok(root) => output(child(super::object(root.as_raw()), node), out),
            Err(error) => error.code(),
        },
        None => S_OK,
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
    if !x.is_finite() || !y.is_finite() {
        return E_INVALIDARG;
    }
    let bounds = match rect(object) {
        Ok(rect) => rect,
        Err(error) => return error.code(),
    };
    if x < bounds.left
        || x >= bounds.left + bounds.width
        || y < bounds.top
        || y >= bounds.top + bounds.height
    {
        return S_OK;
    }
    // HWND coordinates are nonnegative; rounding preserves integer-pixel half-open-interval hit results.
    match object.view.node_at_point(
        (x - bounds.left) as i32,
        (y - bounds.top) as i32,
        bounds.width as i32,
        object.row,
    ) {
        Some(node) => output(child(object, node), out),
        None => output(root(object), out),
    }
}
pub(super) static VTABLE: IRawElementProviderFragment_Vtbl = IRawElementProviderFragment_Vtbl {
    base__: IUnknown_Vtbl {
        QueryInterface: query_fragment,
        AddRef: add,
        Release: drop_ref,
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
            Release: drop_root,
        },
        ElementProviderFromPoint: point,
        GetFocus: focused,
    };
