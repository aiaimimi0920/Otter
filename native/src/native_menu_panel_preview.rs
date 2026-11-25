//! Hover reading surfaces consume only prepared bounded pixels/text; no decoding, clipboard access, or business permissions.
use super::*;

#[derive(Clone)]
pub enum Preview {
    Image(crate::image_decode::Image),
    Text(std::sync::Arc<str>),
}
impl Panel {
    #[cfg(test)]
    pub(crate) fn preview_content(&self) -> Option<Preview> {
        self.data.preview.borrow().clone()
    }
    pub(crate) fn set_preview(&self, content: Preview) {
        self.data.preview.replace(Some(content));
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }
}
pub(super) unsafe fn paint(dc: HDC, content: &Preview, mut bounds: RECT, data: &Data) {
    let padding = font::scaled(10, data.dpi);
    bounds.left += padding;
    bounds.top += padding;
    bounds.right -= padding;
    bounds.bottom -= padding;
    match content {
        Preview::Text(text) => {
            SetTextColor(dc, COLORREF(0xeff8f7));
            let mut units: Vec<u16> = text.encode_utf16().collect();
            DrawTextW(
                dc,
                &mut units,
                &mut bounds,
                DT_LEFT | DT_WORDBREAK | DT_END_ELLIPSIS | DT_NOPREFIX,
            );
        }
        Preview::Image(image) => {
            let scale = ((bounds.right - bounds.left) as f64 / image.width as f64)
                .min((bounds.bottom - bounds.top) as f64 / image.height as f64);
            let width = (image.width as f64 * scale).round().max(1.0) as i32;
            let height = (image.height as f64 * scale).round().max(1.0) as i32;
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: image.width as i32,
                    biHeight: -(image.height as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let result = StretchDIBits(
                dc,
                bounds.left + (bounds.right - bounds.left - width) / 2,
                bounds.top + (bounds.bottom - bounds.top - height) / 2,
                width,
                height,
                0,
                0,
                image.width as i32,
                image.height as i32,
                Some(image.pixels.as_ptr().cast()),
                &info,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
            if result == 0 || result == -1 {
                data.failed.set(true);
            }
        }
    }
}
