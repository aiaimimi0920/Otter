//! 验证源层级的接管标志语义；登记 ACK 顺序由宿主 journal 测试单独保护。
use super::*;
use crate::{native_dispatch as dispatch, native_popup::Lease, native_popup_layer::MANAGED};

#[path = "native_popup_mutation_tests.rs"]
mod mutation;
#[path = "native_layer_protocol_tests.rs"]
mod requests;

thread_local! {
    static RETAINED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[test]
fn synchronous_restore_reentry_cannot_discard_cleanup_identity_or_revive_input() {
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
    POPUP.with(|slot| slot.set(popup.0));
    RETAINED.with(|slot| slot.set(false));
    BUTTON_HANDLER.with(|handler| {
        handler.set(Some(|| unsafe {
            let popup = POPUP.with(std::cell::Cell::get);
            let _ = ShowWindow(popup, SW_SHOWNOACTIVATE);
            let lease = Lease::read(popup).unwrap();
            SetPropW(popup, MANAGED, Some(HANDLE(lease.instance as *mut _))).unwrap();
        }))
    });
    assert!(dispatch::dispatch(f.owner.0, 303, f.child.0).unwrap());
    BUTTON_HANDLER.with(|handler| handler.set(None));
    let lease = Lease::read(popup.0).unwrap();
    POSITION_HANDLER.with(|handler| {
        handler.set(Some(|hwnd| unsafe {
            POSITION_HANDLER.with(|handler| handler.set(None));
            let lease = Lease::read(hwnd).unwrap();
            // 内层隐藏真实进入 before/return hooks；外层恢复尚未返回，身份必须保留。
            let _ = ShowWindow(hwnd, SW_HIDE);
            RETAINED.with(|slot| slot.set(lease.same_instance() && !lease.valid()));
        }))
    });
    assert!(crate::native_popup_layer::release(popup.0, false));
    POSITION_HANDLER.with(|handler| handler.set(None));
    assert!(RETAINED.with(std::cell::Cell::get));
    assert!(
        lease.same_instance(),
        "reentry cannot erase in-flight recovery identity"
    );
    assert!(!lease.valid(), "hide revoked the old input lease");
    assert_eq!(value(popup.0, MANAGED), 0);
    assert_eq!(
        unsafe { GetWindowLongPtrW(popup.0, GWL_EXSTYLE) } as u32 & WS_EX_TOPMOST.0,
        0
    );
    // 随后的普通隐藏结算才能清除已归还的旧身份。
    let mut position = WINDOWPOS {
        flags: SWP_HIDEWINDOW,
        ..Default::default()
    };
    unsafe {
        crate::native_popup::before(&CWPSTRUCT {
            hwnd: popup.0,
            message: WM_WINDOWPOSCHANGING,
            lParam: LPARAM((&mut position as *mut WINDOWPOS) as isize),
            ..Default::default()
        });
    }
    assert!(!lease.same_instance());
}

#[test]
fn acknowledged_host_marker_survives_button_return_but_hide_restores_synchronously() {
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
    POPUP.with(|slot| slot.set(popup.0));
    BUTTON_HANDLER.with(|handler| {
        handler.set(Some(|| unsafe {
            let popup = POPUP.with(std::cell::Cell::get);
            let _ = ShowWindow(popup, SW_SHOWNOACTIVATE);
            let lease = Lease::read(popup).unwrap();
            // 模拟已取得 journal ACK 的宿主接管；并非以标志本身验证 journal。
            SetPropW(popup, MANAGED, Some(HANDLE(lease.instance as *mut _))).unwrap();
        }))
    });
    assert!(dispatch::dispatch(f.owner.0, 303, f.child.0).unwrap());
    BUTTON_HANDLER.with(|handler| handler.set(None));
    let lease = Lease::read(popup.0).unwrap();
    assert_ne!(
        unsafe { GetWindowLongPtrW(popup.0, GWL_EXSTYLE) } as u32 & WS_EX_TOPMOST.0,
        0
    );
    unsafe {
        let _ = ShowWindow(popup.0, SW_HIDE);
    }
    assert!(!lease.valid());
    assert_eq!(
        unsafe { GetWindowLongPtrW(popup.0, GWL_EXSTYLE) } as u32 & WS_EX_TOPMOST.0,
        0
    );
    assert_eq!(value(popup.0, MANAGED), 0);
}

#[test]
fn revocation_can_deny_input_without_losing_identity_needed_by_layer_recovery() {
    let f = Fixture::new();
    let popup = Window::new(None);
    let _hooks = Hooks::new();
    POPUP.with(|slot| slot.set(popup.0));
    BUTTON_HANDLER.with(|handler| {
        handler.set(Some(|| unsafe {
            let _ = ShowWindow(POPUP.with(std::cell::Cell::get), SW_SHOWNOACTIVATE);
        }))
    });
    assert!(dispatch::dispatch(f.owner.0, 303, f.child.0).unwrap());
    BUTTON_HANDLER.with(|handler| handler.set(None));
    let lease = Lease::read(popup.0).unwrap();
    put(popup.0, w!("Otter.Live.Popup.Revoked.v1"), lease.instance);
    assert!(!lease.valid());
    assert!(
        lease.same_instance(),
        "cleanup identity must survive input revocation"
    );
}
