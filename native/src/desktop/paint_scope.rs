//! WM_PAINT 的 DC 借用 owner；正常返回与 panic 展开都配对 EndPaint。
use windows::Win32::{Foundation::HWND, Graphics::Gdi::*};

pub(super) struct PaintScope(HWND, PAINTSTRUCT);
impl PaintScope {
    pub(super) unsafe fn begin(hwnd: HWND) -> Self {
        let mut scope = Self(hwnd, PAINTSTRUCT::default());
        BeginPaint(hwnd, &mut scope.1);
        scope
    }
}
impl Drop for PaintScope {
    fn drop(&mut self) {
        unsafe {
            let _ = EndPaint(self.0, &self.1);
        }
    }
}
