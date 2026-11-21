//! 自有窗口验证模态菜单内的按钮，不访问用户应用或系统输入。
use super::*;
use crate::native_dispatch as dispatch;
use std::cell::Cell;

thread_local! {
    static RESULT: Cell<(bool, bool, bool)> = const { Cell::new((false, false, false)) };
    static ENTER_PUMP: Cell<bool> = const { Cell::new(false) };
}

fn nested_button() {
    let parent = dispatch::collecting().unwrap();
    let child = Fixture::new();
    unsafe {
        SetWindowLongPtrW(child.root.0, GWLP_HWNDPARENT, parent.root.0 as isize);
        SetWindowLongPtrW(child.owner.0, GWLP_HWNDPARENT, parent.owner.0 as isize);
    }
    BUTTON_HANDLER.with(|handler| {
        handler.set(if ENTER_PUMP.with(Cell::get) {
            Some(dispatch::message_pump_entered)
        } else {
            None
        });
    });
    let accepted = dispatch::dispatch(child.owner.0, 303, child.child.0).unwrap();
    RESULT.with(|result| {
        result.set((
            accepted,
            value(child.owner.0, FINISHED) == 303,
            dispatch::collecting() == Some(parent),
        ))
    });
}

#[test]
fn owned_menu_button_completes_without_overwriting_parent_transaction() {
    ENTER_PUMP.with(|slot| slot.set(false));
    let f = Fixture::new();
    BUTTON_HANDLER.with(|handler| handler.set(Some(nested_button)));
    assert!(dispatch::dispatch(f.owner.0, 303, f.child.0).unwrap());
    BUTTON_HANDLER.with(|handler| handler.set(None));
    assert_eq!(RESULT.with(Cell::get), (true, true, true));
    assert_eq!(value(f.owner.0, FINISHED), 303);
    assert!(dispatch::collecting().is_none());
}

#[test]
fn nested_message_pump_does_not_reopen_parent_popup_collection() {
    ENTER_PUMP.with(|slot| slot.set(true));
    let f = Fixture::new();
    BUTTON_HANDLER.with(|handler| handler.set(Some(nested_button)));
    assert!(dispatch::dispatch(f.owner.0, 303, f.child.0).unwrap());
    BUTTON_HANDLER.with(|handler| handler.set(None));
    ENTER_PUMP.with(|slot| slot.set(false));
    assert_eq!(RESULT.with(Cell::get), (true, true, false));
    assert_eq!(value(f.owner.0, FINISHED), 303);
}

#[test]
fn nesting_requires_both_source_and_mirror_lineage_and_live_parent_epoch() {
    let parent = Fixture::new();
    let child = Fixture::new();
    let p = parent.read().unwrap();
    let c = child.read().unwrap();
    assert!(!crate::native_nested::allowed(p, c));
    unsafe {
        SetWindowLongPtrW(child.root.0, GWLP_HWNDPARENT, parent.root.0 .0 as isize);
    }
    assert!(!crate::native_nested::allowed(p, c));
    unsafe {
        SetWindowLongPtrW(child.owner.0, GWLP_HWNDPARENT, parent.owner.0 .0 as isize);
    }
    assert!(crate::native_nested::allowed(p, c));
    assert!(!crate::native_nested::allowed(p, p));
    unsafe {
        SetWindowLongPtrW(child.root.0, GWLP_HWNDPARENT, 0);
    }
    assert!(!crate::native_nested::allowed(p, c));
    unsafe {
        SetWindowLongPtrW(child.root.0, GWLP_HWNDPARENT, parent.root.0 .0 as isize);
    }
    put(parent.owner.0, EPOCH, 0);
    assert!(!crate::native_nested::allowed(p, c));
}

#[test]
fn nested_completion_keeps_parent_menu_topmost_until_parent_returns() {
    let f = Fixture::new();
    let popup = Window::new(None);
    let _hooks = Hooks::new();
    unsafe {
        SetWindowPos(
            f.root.0,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
        .unwrap();
    }
    ENTER_PUMP.with(|slot| slot.set(false));
    POPUP.with(|slot| slot.set(popup.0));
    BUTTON_HANDLER.with(|handler| {
        handler.set(Some(|| unsafe {
            let popup = POPUP.with(Cell::get);
            let _ = ShowWindow(popup, SW_SHOWNOACTIVATE);
            nested_button();
            BUTTONS.with(|count| {
                count.set(usize::from(
                    GetWindowLongPtrW(popup, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0,
                ))
            });
        }))
    });
    assert!(dispatch::dispatch(f.owner.0, 303, f.child.0).unwrap());
    BUTTON_HANDLER.with(|handler| handler.set(None));
    assert_eq!(RESULT.with(Cell::get), (true, true, true));
    assert_eq!(BUTTONS.with(Cell::get), 1);
    assert_eq!(
        unsafe { GetWindowLongPtrW(popup.0, GWL_EXSTYLE) } as u32 & WS_EX_TOPMOST.0,
        0
    );
}
