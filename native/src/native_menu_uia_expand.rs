//! 子菜单展开/收起只排队；状态来自控制器已经建立的子页面，不预测操作成功。
use super::*;
unsafe fn base(this: *mut c_void) -> *mut c_void {
    this.cast::<u8>()
        .sub(std::mem::offset_of!(Provider, expand))
        .cast()
}
unsafe extern "system" fn query_expand(
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
unsafe extern "system" fn expand(this: *mut c_void) -> HRESULT {
    enqueue(object(base(this)), Action::Expand)
}
unsafe extern "system" fn collapse(this: *mut c_void) -> HRESULT {
    enqueue(object(base(this)), Action::Collapse)
}
unsafe extern "system" fn state(this: *mut c_void, out: *mut ExpandCollapseState) -> HRESULT {
    if out.is_null() {
        return E_POINTER;
    }
    out.write(ExpandCollapseState_Collapsed);
    let object = object(base(this));
    let status = available(object);
    if status.is_err() {
        return status;
    }
    let Some(entry) = object.node.as_ref().and_then(Node::read) else {
        return HRESULT(UIA_E_ELEMENTNOTAVAILABLE as i32);
    };
    out.write(if entry.expanded {
        ExpandCollapseState_Expanded
    } else {
        ExpandCollapseState_Collapsed
    });
    S_OK
}
pub(super) static VTABLE: IExpandCollapseProvider_Vtbl = IExpandCollapseProvider_Vtbl {
    base__: IUnknown_Vtbl {
        QueryInterface: query_expand,
        AddRef: add,
        Release: drop_ref,
    },
    Expand: expand,
    Collapse: collapse,
    ExpandCollapseState: state,
};
