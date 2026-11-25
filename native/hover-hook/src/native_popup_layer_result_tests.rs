//! 结算只移动同一事务的租约；自有窗口忽略 Z-order 时不能假报已改变。
use super::*;
use windows::{core::w, Win32::System::LibraryLoader::GetModuleHandleW};

#[test]
fn finished_promotions_preserve_pending_slots_and_reuse_allocation() {
    let request = Request {
        owner: HWND(1 as *mut _),
        cookie: 2,
        epoch: 3,
        root: HWND::default(),
        destination: HWND::default(),
        nonce: 4,
        message: WM_RBUTTONUP,
        wp: 0,
        lp: 0,
        popup_scope: Default::default(),
    };
    let lease = |instance| Lease {
        popup: HWND::default(),
        owner: request.owner,
        cookie: request.cookie,
        epoch: request.epoch,
        root: request.root,
        request: request.nonce,
        instance,
    };
    let mut pending = vec![lease(1), lease(2), lease(3), lease(4)];
    let ptr = pending.as_ptr();
    for stale in [
        Request {
            owner: HWND::default(),
            ..request
        },
        Request {
            cookie: 5,
            ..request
        },
        Request {
            epoch: 5,
            ..request
        },
        Request {
            nonce: 5,
            ..request
        },
    ] {
        assert_eq!(take_finished(&mut pending, stale), [None; MAX_PROMOTED]);
    }
    pending[0].request += 1;
    pending[2].epoch += 1;
    let retained = [pending[0], pending[2]];
    assert_eq!(
        take_finished(&mut pending, request),
        [Some(lease(2)), Some(lease(4)), None, None]
    );
    assert_eq!(pending, retained);
    assert_eq!(pending.as_ptr(), ptr);
    assert_eq!(
        take_finished(&mut vec![lease(1); MAX_PROMOTED], request),
        [Some(lease(1)); MAX_PROMOTED]
    );
}

unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if message == WM_WINDOWPOSCHANGING && lp.0 != 0 {
        (*(lp.0 as *mut WINDOWPOS)).flags |= SWP_NOZORDER;
    }
    DefWindowProcW(hwnd, message, wp, lp)
}
struct Window(HWND);
impl Drop for Window {
    fn drop(&mut self) {
        unsafe { DestroyWindow(self.0).unwrap() };
    }
}
#[test]
fn ignored_topmost_request_is_failure_not_a_successful_borrow() {
    unsafe {
        let instance = HINSTANCE(GetModuleHandleW(None).unwrap().0);
        let class = w!("Otter.Test.PopupLayerIgnoredZOrder");
        assert_ne!(
            RegisterClassW(&WNDCLASSW {
                lpfnWndProc: Some(procedure),
                hInstance: instance,
                lpszClassName: class,
                ..Default::default()
            }),
            0
        );
        let window = Window(
            CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                class,
                w!(""),
                WS_POPUP,
                -30000,
                -30000,
                20,
                20,
                None,
                None,
                Some(instance),
                None,
            )
            .unwrap(),
        );
        assert_eq!(
            GetWindowLongPtrW(window.0, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0,
            0
        );
        assert_eq!(set_topmost(window.0, true).unwrap_err().code(), E_FAIL);
        assert_eq!(
            GetWindowLongPtrW(window.0, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0,
            0
        );
        set_topmost(window.0, false).unwrap();
    }
}
