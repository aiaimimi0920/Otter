//! 定位/首次置顶也会同步重入；只使用原协议自有窗口 fixture，不移动系统鼠标。
use super::*;
use crate::native_popup_layer::TEMPORARY;
use std::cell::Cell;

thread_local! {
    static ORIGINAL: Cell<Option<Lease>> = const { Cell::new(None) };
    static IDENTITY_RETAINED: Cell<bool> = const { Cell::new(false) };
    static REOPEN_DENIED: Cell<bool> = const { Cell::new(false) };
    static OTHER: Cell<HWND> = const { Cell::new(HWND(std::ptr::null_mut())) };
    static NESTED: Cell<Option<Lease>> = const { Cell::new(None) };
}
struct Handlers;
impl Handlers {
    fn reset() -> Self {
        BUTTON_HANDLER.with(|slot| slot.set(None));
        POSITION_HANDLER.with(|slot| slot.set(None));
        ORIGINAL.with(|slot| slot.set(None));
        IDENTITY_RETAINED.with(|slot| slot.set(false));
        REOPEN_DENIED.with(|slot| slot.set(false));
        NESTED.with(|slot| slot.set(None));
        Self
    }
}
impl Drop for Handlers {
    fn drop(&mut self) {
        BUTTON_HANDLER.with(|slot| slot.set(None));
        POSITION_HANDLER.with(|slot| slot.set(None));
        POPUP.with(|slot| slot.set(HWND::default()));
        OTHER.with(|slot| slot.set(HWND::default()));
    }
}
fn topmost(hwnd: HWND) -> bool {
    unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0 }
}
fn arm_root(f: &Fixture, popup: &Window) {
    f.root.make_topmost();
    POPUP.with(|slot| slot.set(popup.0));
}
fn during_promotion(hwnd: HWND) -> Option<Lease> {
    let lease = Lease::read(hwnd)?;
    (hwnd == POPUP.with(Cell::get) && value(hwnd, TEMPORARY) == lease.instance).then_some(lease)
}

#[test]
fn promotion_reentrant_hide_show_cannot_publish_new_instance_before_old_write_returns() {
    let f = Fixture::new();
    let popup = Window::new(None);
    let _hooks = Hooks::new();
    let _duplicate_hooks = Hooks::new();
    let _handlers = Handlers::reset();
    arm_root(&f, &popup);
    BUTTON_HANDLER.with(|handler| {
        handler.set(Some(|| unsafe {
            let _ = ShowWindow(POPUP.with(Cell::get), SW_SHOWNOACTIVATE);
        }))
    });
    POSITION_HANDLER.with(|handler| {
        handler.set(Some(|hwnd| unsafe {
            let Some(lease) = during_promotion(hwnd) else {
                return;
            };
            POSITION_HANDLER.with(|handler| handler.set(None));
            ORIGINAL.with(|slot| slot.set(Some(lease)));
            let _ = ShowWindow(hwnd, SW_HIDE);
            IDENTITY_RETAINED.with(|slot| slot.set(lease.same_instance() && !lease.valid()));
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            REOPEN_DENIED
                .with(|slot| slot.set(lease.same_instance() && Lease::read(hwnd).is_none()));
        }))
    });
    assert!(dispatch::dispatch(f.owner.0, 303, f.child.0).unwrap());
    let old = ORIGINAL
        .with(Cell::get)
        .expect("promotion callback entered");
    assert!(IDENTITY_RETAINED.with(Cell::get));
    assert!(REOPEN_DENIED.with(Cell::get));
    assert!(!old.valid());
    assert!(
        Lease::read(popup.0).is_none(),
        "a later hook must not commit the reentrant SHOW after the guard ends"
    );
    assert!(!topmost(popup.0), "revoked promotion must be returned");
    assert_eq!(value(popup.0, TEMPORARY), 0);
    assert_eq!(value(popup.0, MANAGED), 0);

    // 变更已经结算后，独立新输入仍可重开；不能留下永久锁或复用旧显示身份。
    unsafe {
        let _ = ShowWindow(popup.0, SW_HIDE);
    }
    f.publish(304);
    assert!(dispatch::dispatch(f.owner.0, 304, f.child.0).unwrap());
    let new = Lease::read(popup.0).unwrap();
    assert_ne!(old.instance, new.instance);
    assert!(!topmost(popup.0), "new temporary promotion ends with input");
}

#[test]
fn mutation_guard_is_per_window_and_preserves_other_synchronous_popup_leases() {
    let f = Fixture::new();
    let popup = Window::new(None);
    let other = Window::new(None);
    let _hooks = Hooks::new();
    let _handlers = Handlers::reset();
    arm_root(&f, &popup);
    OTHER.with(|slot| slot.set(other.0));
    BUTTON_HANDLER.with(|handler| {
        handler.set(Some(|| unsafe {
            let _ = ShowWindow(POPUP.with(Cell::get), SW_SHOWNOACTIVATE);
        }))
    });
    POSITION_HANDLER.with(|handler| {
        handler.set(Some(|hwnd| unsafe {
            let Some(lease) = during_promotion(hwnd) else {
                return;
            };
            POSITION_HANDLER.with(|handler| handler.set(None));
            ORIGINAL.with(|slot| slot.set(Some(lease)));
            let other = OTHER.with(Cell::get);
            let _ = ShowWindow(other, SW_SHOWNOACTIVATE);
            NESTED.with(|slot| slot.set(Lease::read(other)));
        }))
    });
    assert!(dispatch::dispatch(f.owner.0, 303, f.child.0).unwrap());
    let first = ORIGINAL.with(Cell::get).expect("first promotion entered");
    let second = NESTED
        .with(Cell::get)
        .expect("unrelated popup was admitted");
    assert!(first.valid());
    assert!(second.valid());
    assert_eq!(first.request, second.request);
    assert_ne!(first.popup, second.popup);
    for hwnd in [popup.0, other.0] {
        assert!(!topmost(hwnd), "both temporary layers end with input");
        assert_eq!(value(hwnd, TEMPORARY), 0);
        assert_eq!(value(hwnd, MANAGED), 0);
    }
}
