//! 原生 GDI Slider：按 DPI 绘制轨道/拇指/焦点，高对比度使用系统色；借用 stock 对象。
use super::*;
use crate::desktop::surface_control_colors as colors;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
struct Saved(HDC, i32);
impl Drop for Saved {
    fn drop(&mut self) {
        unsafe {
            let _ = RestoreDC(self.0, self.1);
        }
    }
}
pub(super) unsafe fn track(hwnd: HWND) -> Option<(i32, i32, i32, i32)> {
    let mut rect = RECT::default();
    GetClientRect(hwnd, &mut rect).ok()?;
    let radius = ((7 * GetDpiForWindow(hwnd).clamp(48, 768)).div_ceil(96) as i32)
        .min((rect.right / 2).max(1))
        .min((rect.bottom / 2).max(1));
    Some((
        radius,
        (rect.right - radius).max(radius),
        rect.bottom / 2,
        radius,
    ))
}
unsafe fn draw(hwnd: HWND, dc: HDC) -> bool {
    let saved = SaveDC(dc);
    if saved == 0 {
        return false;
    }
    let _saved = Saved(dc, saved);
    let mut rect = RECT::default();
    if GetClientRect(hwnd, &mut rect).is_err() {
        return false;
    }
    IntersectClipRect(dc, rect.left, rect.top, rect.right, rect.bottom);
    let Some((left, right, y, radius)) = track(hwnd) else {
        return false;
    };
    let enabled = IsWindowEnabled(hwnd).as_bool();
    let themed = colors::enabled();
    let background = if themed {
        colors::BACKGROUND
    } else {
        COLORREF(GetSysColor(COLOR_WINDOW))
    };
    let rail = if themed {
        colors::LINE
    } else {
        COLORREF(GetSysColor(COLOR_GRAYTEXT))
    };
    let ink = if !enabled {
        if themed {
            colors::MUTED
        } else {
            COLORREF(GetSysColor(COLOR_GRAYTEXT))
        }
    } else if themed {
        colors::FOCUS
    } else {
        COLORREF(GetSysColor(COLOR_HIGHLIGHT))
    };
    let brush = HBRUSH(GetStockObject(DC_BRUSH).0);
    SetDCBrushColor(dc, background);
    FillRect(dc, &rect, brush);
    let fraction = (*state(hwnd))
        .range
        .fraction((*state(hwnd)).value)
        .unwrap_or(0.);
    let x = left + ((right - left) as f64 * fraction).round() as i32;
    let half = (radius / 3).max(1);
    SetDCBrushColor(dc, rail);
    FillRect(
        dc,
        &RECT {
            left,
            top: y - half,
            right,
            bottom: y + half,
        },
        brush,
    );
    SetDCBrushColor(dc, ink);
    FillRect(
        dc,
        &RECT {
            left,
            top: y - half,
            right: x,
            bottom: y + half,
        },
        brush,
    );
    SelectObject(dc, GetStockObject(NULL_PEN));
    SelectObject(dc, GetStockObject(DC_BRUSH));
    let _ = Ellipse(dc, x - radius, y - radius, x + radius, y + radius);
    if enabled && GetFocus() == hwnd {
        let thickness = (2 * GetDpiForWindow(hwnd).clamp(48, 768)).div_ceil(96);
        for _ in 0..thickness {
            if rect.right <= rect.left || rect.bottom <= rect.top {
                break;
            }
            FrameRect(dc, &rect, brush);
            rect.left += 1;
            rect.top += 1;
            rect.right -= 1;
            rect.bottom -= 1;
        }
    }
    true
}
pub(super) unsafe fn message(hwnd: HWND, message: u32, wp: WPARAM) -> Option<LRESULT> {
    match message {
        WM_ERASEBKGND => Some(LRESULT(1)),
        WM_PRINTCLIENT if wp.0 != 0 => draw(hwnd, HDC(wp.0 as *mut _)).then_some(LRESULT(0)),
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let dc = BeginPaint(hwnd, &mut ps);
            if dc.is_invalid() {
                return None;
            }
            draw(hwnd, dc);
            let _ = EndPaint(hwnd, &ps);
            Some(LRESULT(0))
        }
        _ => None,
    }
}
