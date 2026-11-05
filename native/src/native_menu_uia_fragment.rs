//! UIA Fragment 导航和当前窗口几何；不保存外部节点或调用业务。
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
    // 弹出 HWND 仍使用自己的 host provider；只重设其逻辑父节点。
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
    let nodes = object.view.nodes();
    let index = match (&object.node, direction.0) {
        (None, 3) => Some(0),
        (None, 4) => nodes.len().checked_sub(1),
        (Some(node), 1) => node.slot().checked_add(1),
        (Some(node), 2) => node.slot().checked_sub(1),
        _ => None,
    };
    match index.and_then(|index| nodes.get(index)) {
        Some(node) => output(child(object, node.clone()), out),
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
    let (top, bottom) = object
        .node
        .as_ref()
        .map_or((rect.top, rect.bottom), |node| {
            let top = rect.top + node.slot() as i32 * object.row;
            (top, (top + object.row).min(rect.bottom))
        });
    Ok(UiaRect {
        left: rect.left as f64,
        top: top as f64,
        width: (rect.right - rect.left).max(0) as f64,
        height: (bottom - top).max(0) as f64,
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
    let index = ((y - bounds.top) / object.row as f64) as usize;
    match object.view.nodes().get(index) {
        Some(node) => output(child(object, node.clone()), out),
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
