//! 自有窗口明确忽略 Z-order，验证成功返回不能被当作层级已改变。
use super::*;
use windows::{core::w, Win32::System::LibraryLoader::GetModuleHandleW};

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
