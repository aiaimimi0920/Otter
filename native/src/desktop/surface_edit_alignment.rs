//! EDIT alignment changes only presentation style bits, without text writeback, control recreation, or ending composition.
use super::*;
impl Control {
    pub fn set_alignment(&self, value: Option<&str>) -> Result<()> {
        let align = match value {
            Some("center") => ES_CENTER,
            Some("right") => ES_RIGHT,
            _ => ES_LEFT,
        } as isize;
        unsafe {
            let hwnd = self.hwnd()?;
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
            let next = (style & !((ES_CENTER | ES_RIGHT) as isize)) | align;
            if style == next {
                return Ok(());
            }
            SetLastError(WIN32_ERROR(0));
            if SetWindowLongPtrW(hwnd, GWL_STYLE, next) == 0 && GetLastError() != WIN32_ERROR(0) {
                return Err(Error::from_win32());
            }
            // STYLECHANGED can reenter; schedule repaint only while the original owner survives.
            InvalidateRect(Some(self.hwnd()?), None, true).ok()?;
            Ok(())
        }
    }
}
