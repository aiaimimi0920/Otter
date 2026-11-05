//! 仅测试自有离屏NOACTIVATE标准控件；无全局hook、焦点请求或外部输入。
use windows::{
    core::w,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
pub(crate) struct Window {
    pub hwnd: HWND,
    edit: HWND,
}
impl Window {
    pub(crate) fn new() -> Self {
        unsafe {
            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                w!("STATIC"),
                w!("Otter UIA source"),
                WS_POPUP,
                -20000,
                -20000,
                320,
                160,
                None,
                None,
                None,
                None,
            )
            .unwrap();
            let edit = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("EDIT"),
                w!("value-seven"),
                WS_CHILD | WS_VISIBLE,
                8,
                8,
                200,
                24,
                Some(hwnd),
                Some(HMENU(101usize as *mut _)),
                None,
                None,
            )
            .unwrap();
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("EDIT"),
                w!("password-secret"),
                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(ES_PASSWORD as u32),
                8,
                40,
                200,
                24,
                Some(hwnd),
                Some(HMENU(102usize as *mut _)),
                None,
                None,
            )
            .unwrap();
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            Self { hwnd, edit }
        }
    }
    pub(crate) fn value(&self, text: &str) {
        let text: Vec<_> = text.encode_utf16().chain(Some(0)).collect();
        unsafe {
            SetWindowTextW(self.edit, windows::core::PCWSTR(text.as_ptr())).unwrap();
        }
    }
    pub(crate) fn pump() {
        unsafe {
            let mut message = MSG::default();
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd).unwrap();
        }
    }
}
