//! 自建离屏窗口独立记录实际Win32消息；不移动鼠标、不切前台、不触用户应用。
use std::sync::Mutex;
use windows::{
    core::w,
    Win32::{Foundation::*, System::LibraryLoader::GetModuleHandleW, UI::WindowsAndMessaging::*},
};
pub(in crate::desktop) struct Window {
    pub hwnd: HWND,
    messages: Box<Mutex<Vec<(u32, usize, isize)>>>,
}
unsafe extern "system" fn proc(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if message == WM_NCCREATE {
        let create = &*(lp.0 as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
    }
    if matches!(
        message,
        WM_KEYDOWN
            | WM_KEYUP
            | WM_MOUSEMOVE
            | WM_LBUTTONDOWN
            | WM_LBUTTONUP
            | WM_LBUTTONDBLCLK
            | WM_MOUSEWHEEL
            | WM_MOUSEHWHEEL
    ) {
        let data = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Mutex<Vec<(u32, usize, isize)>>;
        if !data.is_null() {
            let mut messages = (*data).lock().unwrap();
            assert!(messages.len() < 128);
            messages.push((message, wp.0, lp.0));
        }
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, message, wp, lp)
}
impl Window {
    pub fn new() -> Self {
        unsafe {
            let instance = GetModuleHandleW(None).unwrap();
            let class = WNDCLASSW {
                hInstance: instance.into(),
                lpfnWndProc: Some(proc),
                lpszClassName: w!("OtterRelaySourceInputOracle"),
                ..Default::default()
            };
            assert_ne!(RegisterClassW(&class), 0);
            let mut messages = Box::new(Mutex::new(Vec::new()));
            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                w!("OtterRelaySourceInputOracle"),
                w!("Otter owned input oracle"),
                WS_POPUP,
                -30000,
                -30000,
                200,
                120,
                None,
                None,
                Some(instance.into()),
                Some((&mut *messages as *mut Mutex<Vec<(u32, usize, isize)>>).cast()),
            )
            .unwrap();
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            Self { hwnd, messages }
        }
    }
    pub fn messages(&self) -> Vec<(u32, usize, isize)> {
        self.messages.lock().unwrap().clone()
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
            let _ = UnregisterClassW(
                w!("OtterRelaySourceInputOracle"),
                Some(GetModuleHandleW(None).unwrap().into()),
            );
        }
    }
}
