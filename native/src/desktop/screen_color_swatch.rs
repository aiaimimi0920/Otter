//! 取色预览的原生色块与 HEX 绘制；子类仅属于自身窗口，GDI 临时资源在每次绘制后回收。
use super::*;
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
const ID: usize = 0x4f5357;
fn scaled(value: i32, dpi: u32) -> i32 {
    ((value as u64 * dpi.clamp(48, 768) as u64 + 48) / 96) as i32
}
pub(super) fn size(dpi: u32) -> (i32, i32) {
    (scaled(160, dpi), scaled(32, dpi))
}
pub(super) unsafe fn attach(hwnd: HWND) -> Result<()> {
    if !SetWindowSubclass(hwnd, Some(procedure), ID, 0).as_bool() {
        return Err(Error::from_win32());
    }
    Ok(())
}
pub(super) unsafe fn update(hwnd: HWND, Color(r, g, b): Color) -> bool {
    let value = (r as u32 | (g as u32) << 8 | (b as u32) << 16) as isize + 1;
    if GetWindowLongPtrW(hwnd, GWLP_USERDATA) == value {
        return false;
    }
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, value);
    // 自绘不应丢失原生 STATIC 的可访问文本。
    let text = windows::core::HSTRING::from(crate::desktop::color_dialog::hex(Color(r, g, b)));
    let _ = SetWindowTextW(hwnd, &text);
    true
}
struct Object(HGDIOBJ);
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
    _: usize,
) -> LRESULT {
    std::panic::catch_unwind(|| match msg {
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let mut paint = Paint(hwnd, PAINTSTRUCT::default());
            let dc = BeginPaint(hwnd, &mut paint.1);
            let mut rect = RECT::default();
            if GetClientRect(hwnd, &mut rect).is_ok() {
                let value = GetWindowLongPtrW(hwnd, GWLP_USERDATA).saturating_sub(1) as u32;
                render(dc, rect, GetDpiForWindow(hwnd), COLORREF(value));
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
            DefSubclassProc(hwnd, msg, wp, lp)
        }
        _ => DefSubclassProc(hwnd, msg, wp, lp),
    })
    .unwrap_or_else(|_| DefSubclassProc(hwnd, msg, wp, lp))
}
unsafe fn render(dc: HDC, rect: RECT, dpi: u32, color: COLORREF) {
    FillRect(dc, &rect, GetSysColorBrush(COLOR_WINDOW));
    let swatch = RECT {
        left: scaled(6, dpi),
        top: scaled(6, dpi),
        right: scaled(26, dpi),
        bottom: scaled(26, dpi),
    };
    let brush = Object(CreateSolidBrush(color).into());
    if !brush.0.is_invalid() {
        FillRect(dc, &swatch, HBRUSH(brush.0 .0));
    }
    FrameRect(dc, &swatch, GetSysColorBrush(COLOR_WINDOWTEXT));
    let mut description = LOGFONTW {
        lfHeight: -scaled(14, dpi),
        ..Default::default()
    };
    for (slot, value) in description
        .lfFaceName
        .iter_mut()
        .zip("Segoe UI".encode_utf16())
    {
        *slot = value;
    }
    let font = Object(CreateFontIndirectW(&description).into());
    let _selected = if font.0.is_invalid() {
        None
    } else {
        Some(Selected(dc, SelectObject(dc, font.0)))
    };
    SetBkMode(dc, TRANSPARENT);
    SetTextColor(dc, COLORREF(GetSysColor(COLOR_WINDOWTEXT)));
    let text = crate::desktop::color_dialog::hex(Color(
        color.0 as u8,
        (color.0 >> 8) as u8,
        (color.0 >> 16) as u8,
    ));
    let text: Vec<u16> = text.encode_utf16().collect();
    let _ = TextOutW(dc, scaled(36, dpi), scaled(8, dpi), &text);
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
    #[test]
    fn native_swatch_draws_correct_rgb_at_multiple_dpis_without_gdi_growth() {
        use windows::Win32::System::Threading::{
            GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS,
        };
        unsafe {
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
                render(dc.0, rect, dpi, color);
                assert_eq!(GetPixel(dc.0, scaled(12, dpi), scaled(12, dpi)), color);
                let before = GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS);
                for _ in 0..16 {
                    render(dc.0, rect, dpi, color);
                }
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
        }
    }
}
