//! Native swatch and HEX rendering for the color preview; subclasses belong only to their windows, fonts are reused per window, and drawing borrows system brushes.
use super::*;
use std::cell::Cell;
use windows::core::PCWSTR;
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
const ID: usize = 0x4f5357;
fn scaled(value: i32, dpi: u32) -> i32 {
    ((value as u64 * dpi.clamp(48, 768) as u64 + 48) / 96) as i32
}
pub(super) fn size(dpi: u32) -> (i32, i32) {
    (scaled(160, dpi), scaled(32, dpi))
}
pub(super) unsafe fn attach(hwnd: HWND) -> Result<()> {
    let cache = Box::new(FontCache::default());
    if !SetWindowSubclass(
        hwnd,
        Some(procedure),
        ID,
        &*cache as *const FontCache as usize,
    )
    .as_bool()
    {
        return Err(Error::from_win32());
    }
    let _ = Box::into_raw(cache); // WM_NCDESTROY owns this allocation after registration.
    Ok(())
}
pub(super) unsafe fn update(hwnd: HWND, Color(r, g, b): Color) -> bool {
    let value = (r as u32 | (g as u32) << 8 | (b as u32) << 16) as isize + 1;
    if GetWindowLongPtrW(hwnd, GWLP_USERDATA) == value {
        return false;
    }
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, value);
    // Owner drawing must preserve native STATIC accessible text.
    let text = hex(Color(r, g, b));
    let _ = SetWindowTextW(hwnd, PCWSTR(text.as_ptr()));
    true
}
// The same fixed UTF-16 representation serves both native text and owner-draw.
fn hex(Color(r, g, b): Color) -> [u16; 8] {
    let mut text = [0; 8];
    text[0] = u16::from(b'#');
    let digits = b"0123456789abcdef";
    for (pair, byte) in text[1..7].chunks_exact_mut(2).zip([r, g, b]) {
        pair[0] = u16::from(digits[(byte >> 4) as usize]);
        pair[1] = u16::from(digits[(byte & 15) as usize]);
    }
    text
}
#[derive(Default)]
struct FontCache(Cell<Option<(i32, HGDIOBJ)>>);
impl FontCache {
    unsafe fn get(&self, dpi: u32) -> HGDIOBJ {
        let height = -scaled(14, dpi);
        if let Some((cached, font)) = self.0.get() {
            if cached == height {
                return font;
            }
        }
        let mut description = LOGFONTW {
            lfHeight: height,
            ..Default::default()
        };
        for (slot, value) in description
            .lfFaceName
            .iter_mut()
            .zip("Segoe UI".encode_utf16())
        {
            *slot = value;
        }
        let font: HGDIOBJ = CreateFontIndirectW(&description).into();
        if !font.is_invalid() {
            if let Some((_, old)) = self.0.replace(Some((height, font))) {
                let _ = DeleteObject(old);
            }
        }
        font
    }
}
impl Drop for FontCache {
    fn drop(&mut self) {
        if let Some((_, font)) = self.0.take() {
            unsafe {
                let _ = DeleteObject(font);
            }
        }
    }
}
#[cfg(test)]
struct Object(HGDIOBJ);
#[cfg(test)]
impl Drop for Object {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(self.0);
        }
    }
}
struct Selected(HDC, HGDIOBJ);
impl Drop for Selected {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.0, self.1);
        }
    }
}
struct Paint(HWND, PAINTSTRUCT);
impl Drop for Paint {
    fn drop(&mut self) {
        unsafe {
            let _ = EndPaint(self.0, &self.1);
        }
    }
}
unsafe extern "system" fn procedure(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    std::panic::catch_unwind(|| match msg {
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let mut paint = Paint(hwnd, PAINTSTRUCT::default());
            let dc = BeginPaint(hwnd, &mut paint.1);
            let mut rect = RECT::default();
            if GetClientRect(hwnd, &mut rect).is_ok() {
                let value = GetWindowLongPtrW(hwnd, GWLP_USERDATA).saturating_sub(1) as u32;
                // Only GDI calls while borrowing the window-owned font; selection is
                // restored before leaving render, so a later paint can replace it.
                let cache = &*(data as *const FontCache);
                let dpi = GetDpiForWindow(hwnd);
                render(dc, rect, dpi, COLORREF(value), cache);
            }
            LRESULT(0)
        }
        WM_DPICHANGED => {
            if lp.0 != 0 {
                let rect = &*(lp.0 as *const RECT);
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    rect.left,
                    rect.top,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                    SWP_NOACTIVATE | SWP_NOZORDER,
                );
            }
            let _ = InvalidateRect(Some(hwnd), None, false);
            LRESULT(0)
        }
        WM_NCDESTROY => {
            let _ = RemoveWindowSubclass(hwnd, Some(procedure), ID);
            drop(Box::from_raw(data as *mut FontCache));
            DefSubclassProc(hwnd, msg, wp, lp)
        }
        _ => DefSubclassProc(hwnd, msg, wp, lp),
    })
    .unwrap_or_else(|_| DefSubclassProc(hwnd, msg, wp, lp))
}
unsafe fn render(dc: HDC, rect: RECT, dpi: u32, color: COLORREF, cache: &FontCache) {
    FillRect(dc, &rect, GetSysColorBrush(COLOR_WINDOW));
    let swatch = RECT {
        left: scaled(6, dpi),
        top: scaled(6, dpi),
        right: scaled(26, dpi),
        bottom: scaled(26, dpi),
    };
    let previous_brush = SetDCBrushColor(dc, color);
    if previous_brush.0 != u32::MAX {
        FillRect(dc, &swatch, HBRUSH(GetStockObject(DC_BRUSH).0));
        SetDCBrushColor(dc, previous_brush);
    }
    FrameRect(dc, &swatch, GetSysColorBrush(COLOR_WINDOWTEXT));
    let font = cache.get(dpi);
    let _selected = (!font.is_invalid()).then(|| Selected(dc, SelectObject(dc, font)));
    SetBkMode(dc, TRANSPARENT);
    SetTextColor(dc, COLORREF(GetSysColor(COLOR_WINDOWTEXT)));
    let text = hex(Color(
        color.0 as u8,
        (color.0 >> 8) as u8,
        (color.0 >> 16) as u8,
    ));
    let _ = TextOutW(dc, scaled(36, dpi), scaled(8, dpi), &text[..7]);
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Dc(HDC);
    impl Drop for Dc {
        fn drop(&mut self) {
            unsafe {
                let _ = DeleteDC(self.0);
            }
        }
    }
    unsafe fn font_height(font: HGDIOBJ) -> Option<i32> {
        let mut result = LOGFONTW::default();
        let read = GetObjectW(
            font,
            std::mem::size_of::<LOGFONTW>() as i32,
            Some((&mut result as *mut LOGFONTW).cast()),
        );
        (read > 0).then_some(result.lfHeight)
    }
    #[test]
    fn native_swatch_draws_correct_rgb_at_multiple_dpis_without_gdi_growth() {
        use windows::Win32::System::Threading::{
            GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS,
        };
        unsafe {
            let cache = FontCache::default();
            for dpi in [96, 144, 192] {
                let (width, height) = size(dpi);
                let dc = Dc(CreateCompatibleDC(None));
                assert!(!dc.0.is_invalid());
                let info = BITMAPINFO {
                    bmiHeader: BITMAPINFOHEADER {
                        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                        biWidth: width,
                        biHeight: -height,
                        biPlanes: 1,
                        biBitCount: 32,
                        biCompression: BI_RGB.0,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                let mut bits = std::ptr::null_mut();
                let bitmap = Object(
                    CreateDIBSection(Some(dc.0), &info, DIB_RGB_COLORS, &mut bits, None, 0)
                        .unwrap()
                        .into(),
                );
                let _selected = Selected(dc.0, SelectObject(dc.0, bitmap.0));
                let rect = RECT {
                    left: 0,
                    top: 0,
                    right: width,
                    bottom: height,
                };
                let color = COLORREF(0x00563412);
                let previous_brush = COLORREF(0x00774422);
                SetDCBrushColor(dc.0, previous_brush);
                render(dc.0, rect, dpi, color, &cache);
                assert_eq!(GetPixel(dc.0, scaled(12, dpi), scaled(12, dpi)), color);
                assert_eq!(GetDCBrushColor(dc.0), previous_brush);
                let font = cache.get(dpi);
                assert!(!font.is_invalid());
                let before = GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS);
                for _ in 0..16 {
                    render(dc.0, rect, dpi, color, &cache);
                }
                assert_eq!(cache.get(dpi), font);
                let after = GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS);
                assert!(after <= before + 1, "GDI count grew: {before} -> {after}");
            }
        }
    }
    #[test]
    fn layout_scales_and_repeated_equal_color_does_not_request_repaint() {
        assert_eq!(size(96), (160, 32));
        assert_eq!(size(144), (240, 48));
        assert_eq!(size(192), (320, 64));
        unsafe {
            let owner = crate::desktop::Window(
                CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("STATIC"),
                    w!("preview test"),
                    WS_OVERLAPPED,
                    0,
                    0,
                    20,
                    20,
                    None,
                    None,
                    None,
                    None,
                )
                .unwrap(),
            );
            let popup = super::super::create(owner.0).unwrap();
            assert!(update(popup.0, Color(1, 2, 3)));
            let mut text = [0u16; 16];
            let count = GetWindowTextW(popup.0, &mut text);
            assert_eq!(
                String::from_utf16(&text[..count as usize]).unwrap(),
                "#010203"
            );
            assert!(!update(popup.0, Color(1, 2, 3)));
            assert!(update(popup.0, Color(0, 0, 0)));
            assert!(!IsWindowVisible(popup.0).as_bool());
            use windows::Win32::UI::Shell::GetWindowSubclass;
            let mut data = 0;
            assert!(GetWindowSubclass(popup.0, Some(procedure), ID, Some(&mut data)).as_bool());
            let cache = &*(data as *const FontCache);
            let old = cache.get(96);
            assert_eq!(cache.get(96), old);
            let next = cache.get(192);
            assert_ne!(old, next);
            // Flush the thread's GDI batch before checking retired objects. GetObjectType
            // can still report the former handle type; read the full descriptor instead.
            GdiFlush().unwrap();
            assert_eq!(font_height(old), None);
            assert_eq!(font_height(next), Some(-28));
            drop(popup);
            GdiFlush().unwrap();
            assert_eq!(font_height(next), None);
        }
    }
}
