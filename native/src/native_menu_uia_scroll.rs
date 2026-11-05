//! 根菜单的垂直整行滚动；百分比请求绑定当前快照代次，不直接移动窗口。
use super::*;
use crate::native_menu::accessibility::Scroll;
unsafe fn base(this: *mut c_void) -> *mut c_void {
    this.cast::<u8>()
        .sub(std::mem::offset_of!(Provider, scroll))
        .cast()
}
unsafe extern "system" fn query_scroll(
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
fn snapshot(object: &Provider) -> Result<Scroll> {
    available(object).ok()?;
    object
        .view
        .scroll()
        .ok_or_else(|| Error::from_hresult(HRESULT(UIA_E_ELEMENTNOTAVAILABLE as i32)))
}
unsafe extern "system" fn scroll(
    this: *mut c_void,
    horizontal: ScrollAmount,
    vertical: ScrollAmount,
) -> HRESULT {
    let object = object(base(this));
    let state = match snapshot(object) {
        Ok(state) => state,
        Err(error) => return error.code(),
    };
    if !(0..=4).contains(&horizontal.0) || !(0..=4).contains(&vertical.0) {
        return E_INVALIDARG;
    }
    if horizontal != ScrollAmount_NoAmount {
        return HRESULT(UIA_E_INVALIDOPERATION as i32);
    }
    if vertical == ScrollAmount_NoAmount {
        return S_OK;
    }
    if state.total <= state.visible {
        return HRESULT(UIA_E_INVALIDOPERATION as i32);
    }
    let step = match vertical.0 {
        0 => -(state.visible as i64),
        1 => -1,
        3 => state.visible as i64,
        _ => 1,
    };
    let first = (state.first as i64 + step).clamp(0, (state.total - state.visible) as i64) as usize;
    queued(object.view.scroll_to(first))
}
unsafe extern "system" fn percent(this: *mut c_void, horizontal: f64, vertical: f64) -> HRESULT {
    let object = object(base(this));
    let state = match snapshot(object) {
        Ok(state) => state,
        Err(error) => return error.code(),
    };
    let valid = |value: f64| {
        value.is_finite() && (value == UIA_ScrollPatternNoScroll || (0.0..=100.0).contains(&value))
    };
    if !valid(horizontal) || !valid(vertical) {
        return E_INVALIDARG;
    }
    if horizontal != UIA_ScrollPatternNoScroll {
        return HRESULT(UIA_E_INVALIDOPERATION as i32);
    }
    if vertical == UIA_ScrollPatternNoScroll {
        return S_OK;
    }
    if state.total <= state.visible {
        return HRESULT(UIA_E_INVALIDOPERATION as i32);
    }
    queued(
        object
            .view
            .scroll_to(((state.total - state.visible) as f64 * vertical / 100.0).round() as usize),
    )
}
unsafe fn number(this: *mut c_void, out: *mut f64, read: impl FnOnce(Scroll) -> f64) -> HRESULT {
    if out.is_null() {
        return E_POINTER;
    }
    out.write(0.0);
    match snapshot(object(base(this))) {
        Ok(state) => {
            out.write(read(state));
            S_OK
        }
        Err(error) => error.code(),
    }
}
unsafe extern "system" fn horizontal_percent(this: *mut c_void, out: *mut f64) -> HRESULT {
    number(this, out, |_| UIA_ScrollPatternNoScroll)
}
unsafe extern "system" fn vertical_percent(this: *mut c_void, out: *mut f64) -> HRESULT {
    number(this, out, |state| {
        if state.total <= state.visible {
            UIA_ScrollPatternNoScroll
        } else {
            100.0 * state.first as f64 / (state.total - state.visible) as f64
        }
    })
}
unsafe extern "system" fn horizontal_size(this: *mut c_void, out: *mut f64) -> HRESULT {
    number(this, out, |_| 100.0)
}
unsafe extern "system" fn vertical_size(this: *mut c_void, out: *mut f64) -> HRESULT {
    number(this, out, |state| {
        if state.total == 0 {
            100.0
        } else {
            (100.0 * state.visible as f64 / state.total as f64).min(100.0)
        }
    })
}
unsafe fn boolean(this: *mut c_void, out: *mut windows::core::BOOL, vertical: bool) -> HRESULT {
    if out.is_null() {
        return E_POINTER;
    }
    out.write(false.into());
    match snapshot(object(base(this))) {
        Ok(state) => {
            out.write((vertical && state.total > state.visible).into());
            S_OK
        }
        Err(error) => error.code(),
    }
}
unsafe extern "system" fn horizontal_scrollable(
    this: *mut c_void,
    out: *mut windows::core::BOOL,
) -> HRESULT {
    boolean(this, out, false)
}
unsafe extern "system" fn vertical_scrollable(
    this: *mut c_void,
    out: *mut windows::core::BOOL,
) -> HRESULT {
    boolean(this, out, true)
}
pub(super) static VTABLE: IScrollProvider_Vtbl = IScrollProvider_Vtbl {
    base__: IUnknown_Vtbl {
        QueryInterface: query_scroll,
        AddRef: add,
        Release: drop_ref,
    },
    Scroll: scroll,
    SetScrollPercent: percent,
    HorizontalScrollPercent: horizontal_percent,
    VerticalScrollPercent: vertical_percent,
    HorizontalViewSize: horizontal_size,
    VerticalViewSize: vertical_size,
    HorizontallyScrollable: horizontal_scrollable,
    VerticallyScrollable: vertical_scrollable,
};
