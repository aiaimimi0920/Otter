//! chrome 与截图共用提交层：自有 DIB 验证局部坐标、取消帧和提交前目标不变。
use super::*;
use windows::Win32::{Foundation::*, Graphics::Gdi::*};

struct Image {
    dc: HDC,
    bitmap: HBITMAP,
    old: HGDIOBJ,
}
impl Image {
    unsafe fn new() -> Self {
        let dc = CreateCompatibleDC(None);
        assert!(!dc.is_invalid());
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: 16,
                biHeight: -16,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let bitmap = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();
        let old = SelectObject(dc, bitmap.into());
        std::ptr::write_bytes(bits, 0x44, 16 * 16 * 4);
        Self { dc, bitmap, old }
    }
}
impl Drop for Image {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.old);
            DeleteObject(self.bitmap.into()).ok().unwrap();
            DeleteDC(self.dc).ok().unwrap();
        }
    }
}
unsafe fn fill(dc: HDC, rect: &RECT, color: u32) {
    SetDCBrushColor(dc, COLORREF(color));
    assert_ne!(FillRect(dc, rect, HBRUSH(GetStockObject(DC_BRUSH).0)), 0);
    GdiFlush().ok().unwrap();
}
#[test]
fn clearing_and_drawing_stay_offscreen_until_one_complete_commit() {
    let _session = Session::new().unwrap();
    unsafe {
        let target = Image::new();
        let rect = RECT {
            left: 3,
            top: 4,
            right: 12,
            bottom: 13,
        };
        let frame = Frame::begin(target.dc, &rect).unwrap().unwrap();
        fill(frame.dc, &rect, 0);
        assert_eq!(GetPixel(target.dc, 4, 5), COLORREF(0x444444));
        fill(frame.dc, &RECT { bottom: 6, ..rect }, 0xffffff);
        assert_eq!(GetPixel(target.dc, 4, 5), COLORREF(0x444444));
        frame.commit().unwrap();
        assert_eq!(GetPixel(target.dc, 4, 5), COLORREF(0xffffff));
        assert_eq!(GetPixel(target.dc, 4, 10), COLORREF(0));
        assert_eq!(GetPixel(target.dc, 2, 5), COLORREF(0x444444));
        assert_eq!(GetPixel(target.dc, 12, 5), COLORREF(0x444444));
    }
}
#[test]
fn dropping_incomplete_frame_preserves_old_pixels_and_empty_paint_allocates_nothing() {
    let _session = Session::new().unwrap();
    unsafe {
        let target = Image::new();
        let rect = RECT {
            left: 0,
            top: 0,
            right: 16,
            bottom: 16,
        };
        let frame = Frame::begin(target.dc, &rect).unwrap().unwrap();
        fill(frame.dc, &rect, 0);
        drop(frame);
        assert_eq!(GetPixel(target.dc, 4, 5), COLORREF(0x444444));
        assert!(Frame::begin(target.dc, &RECT::default()).unwrap().is_none());
    }
}
