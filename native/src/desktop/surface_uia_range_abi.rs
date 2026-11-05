//! Slider RangeValue ABI，共享 canonical IUnknown 和 UI owner 的实时准入。
use super::*;
use windows::core::BOOL;
pub(super) unsafe fn notify_readonly(
    provider: &IRawElementProviderSimple,
    before: bool,
    after: bool,
) {
    if UiaClientsAreListening().as_bool() {
        let _ = UiaRaiseAutomationPropertyChangedEvent(
            provider,
            UIA_RangeValueIsReadOnlyPropertyId,
            &VARIANT::from(before),
            &VARIANT::from(after),
        );
    }
}
pub(super) unsafe fn notify(provider: &IRawElementProviderSimple, before: f64, after: f64) {
    if UiaClientsAreListening().as_bool() {
        let _ = UiaRaiseAutomationPropertyChangedEvent(
            provider,
            UIA_RangeValueValuePropertyId,
            &VARIANT::from(before),
            &VARIANT::from(after),
        );
    }
}
fn value(object: &Provider) -> Result<(crate::surface_slider::Range, f64, bool)> {
    let node = object
        .node
        .as_ref()
        .ok_or_else(|| Error::from_hresult(HRESULT(UIA_E_NOTSUPPORTED as i32)))?;
    super::super::super::accessibility_bridge::range::value(object.hwnd, node)
}
pub(super) fn property(object: &Provider, id: UIA_PROPERTY_ID) -> Result<Option<VARIANT>> {
    if ![
        UIA_RangeValueValuePropertyId,
        UIA_RangeValueIsReadOnlyPropertyId,
        UIA_RangeValueMinimumPropertyId,
        UIA_RangeValueMaximumPropertyId,
        UIA_RangeValueSmallChangePropertyId,
        UIA_RangeValueLargeChangePropertyId,
    ]
    .contains(&id)
    {
        return Ok(None);
    }
    let (range, current, readonly) = value(object)?;
    Ok(Some(if id == UIA_RangeValueIsReadOnlyPropertyId {
        VARIANT::from(readonly)
    } else {
        VARIANT::from(if id == UIA_RangeValueValuePropertyId {
            current
        } else if id == UIA_RangeValueMinimumPropertyId {
            range.min()
        } else if id == UIA_RangeValueMaximumPropertyId {
            range.max()
        } else if id == UIA_RangeValueSmallChangePropertyId {
            range.step()
        } else {
            ((range.max() - range.min()) / 10.0).max(range.step())
        })
    }))
}
unsafe fn base(this: *mut c_void) -> *mut c_void {
    this.cast::<u8>()
        .sub(std::mem::offset_of!(Provider, range))
        .cast()
}
unsafe extern "system" fn query_range(
    this: *mut c_void,
    iid: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    query(base(this), iid, out)
}
unsafe extern "system" fn add_range(this: *mut c_void) -> u32 {
    add_ref(base(this))
}
unsafe extern "system" fn release_range(this: *mut c_void) -> u32 {
    release(base(this))
}
unsafe extern "system" fn set(this: *mut c_void, new: f64) -> HRESULT {
    let object = object(base(this));
    let status = available(object);
    if status.is_err() {
        return status;
    }
    let Some(node) = &object.node else {
        return HRESULT(UIA_E_NOTSUPPORTED as i32);
    };
    match super::super::super::accessibility_bridge::range::set(object.hwnd, node, new) {
        Ok(()) => S_OK,
        Err(error) => error.code(),
    }
}
unsafe fn number(this: *mut c_void, out: *mut f64, id: UIA_PROPERTY_ID) -> HRESULT {
    if out.is_null() {
        return E_POINTER;
    }
    out.write(0.0);
    let object = object(base(this));
    let status = available(object);
    if status.is_err() {
        return status;
    }
    match property(object, id).and_then(|value| f64::try_from(&value.unwrap())) {
        Ok(value) => {
            out.write(value);
            S_OK
        }
        Err(error) => error.code(),
    }
}
unsafe extern "system" fn current(this: *mut c_void, out: *mut f64) -> HRESULT {
    number(this, out, UIA_RangeValueValuePropertyId)
}
unsafe extern "system" fn minimum(this: *mut c_void, out: *mut f64) -> HRESULT {
    number(this, out, UIA_RangeValueMinimumPropertyId)
}
unsafe extern "system" fn maximum(this: *mut c_void, out: *mut f64) -> HRESULT {
    number(this, out, UIA_RangeValueMaximumPropertyId)
}
unsafe extern "system" fn small(this: *mut c_void, out: *mut f64) -> HRESULT {
    number(this, out, UIA_RangeValueSmallChangePropertyId)
}
unsafe extern "system" fn large(this: *mut c_void, out: *mut f64) -> HRESULT {
    number(this, out, UIA_RangeValueLargeChangePropertyId)
}
unsafe extern "system" fn readonly(this: *mut c_void, out: *mut BOOL) -> HRESULT {
    if out.is_null() {
        return E_POINTER;
    }
    out.write(BOOL(0));
    let object = object(base(this));
    let status = available(object);
    if status.is_err() {
        return status;
    }
    match value(object) {
        Ok((_, _, readonly)) => {
            out.write(BOOL::from(readonly));
            S_OK
        }
        Err(error) => error.code(),
    }
}
pub(super) static VTABLE: IRangeValueProvider_Vtbl = IRangeValueProvider_Vtbl {
    base__: IUnknown_Vtbl {
        QueryInterface: query_range,
        AddRef: add_range,
        Release: release_range,
    },
    SetValue: set,
    Value: current,
    IsReadOnly: readonly,
    Maximum: maximum,
    Minimum: minimum,
    LargeChange: large,
    SmallChange: small,
};
