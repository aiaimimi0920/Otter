//! Neuro-themed one-stop export dialog layout and custom dark quality slider control.
use crate::{desktop::dialog_controls, ui_text::text as tr};
use std::cell::Cell;
use windows::{
    core::{w, Result},
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        UI::{
            Input::KeyboardAndMouse::*,
            Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
            WindowsAndMessaging::*,
        },
    },
};

pub(super) const QUALITY_ID: i32 = 105;
pub(super) const BROWSE_ID: i32 = 106;
pub(super) const DIR_EDIT_ID: i32 = 107;
pub(super) const FILE_EDIT_ID: i32 = 108;
pub(super) const QUALITY_CHANGED: u16 = 0x5101;
const SLIDER_SUBCLASS: usize = 0x455851;

struct SliderData {
    value: Cell<u8>,
}

pub(super) struct QualitySlider {
    hwnd: HWND,
    data: *const SliderData,
}

impl QualitySlider {
    pub(super) unsafe fn create(parent: HWND, rect: RECT, initial: u8) -> Result<Self> {
        let data = Box::into_raw(Box::new(SliderData {
            value: Cell::new(initial.clamp(1, 100)),
        }));
        let hwnd = match CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!(""),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(0x0100), // SS_NOTIFY
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
            Some(parent),
            Some(HMENU(QUALITY_ID as usize as *mut _)),
            None,
            None,
        ) {
            Ok(h) => h,
            Err(e) => {
                drop(Box::from_raw(data));
                return Err(e);
            }
        };
        if !SetWindowSubclass(hwnd, Some(slider_proc), SLIDER_SUBCLASS, data as usize).as_bool() {
            let _ = DestroyWindow(hwnd);
            drop(Box::from_raw(data));
            return Err(windows::core::Error::from_win32());
        }
        Ok(Self { hwnd, data })
    }

    pub(super) unsafe fn value(&self) -> u8 {
        (*self.data).value.get()
    }

    pub(super) unsafe fn set_enabled(&self, enabled: bool) {
        let _ = EnableWindow(self.hwnd, enabled);
        let _ = InvalidateRect(Some(self.hwnd), None, false);
    }
}

unsafe fn set_from_x(hwnd: HWND, data: &SliderData, x: i32) {
    let mut rc = RECT::default();
    if GetClientRect(hwnd, &mut rc).is_err() {
        return;
    }
    let pad = 6;
    let span = (rc.right - rc.left - pad * 2).max(1);
    let ratio = ((x - pad) as f32 / span as f32).clamp(0.0, 1.0);
    let next = (1.0 + ratio * 99.0).round() as u8;
    if data.value.replace(next) != next {
        let _ = InvalidateRect(Some(hwnd), None, false);
        if let Ok(parent) = GetParent(hwnd) {
            let wp = (usize::from(QUALITY_CHANGED) << 16) | (QUALITY_ID as usize & 0xffff);
            let _ = SendMessageW(
                parent,
                WM_COMMAND,
                Some(WPARAM(wp)),
                Some(LPARAM(hwnd.0 as isize)),
            );
        }
    }
}

unsafe fn paint_slider(hwnd: HWND, dc: HDC, value: u8) {
    let mut rc = RECT::default();
    let _ = GetClientRect(hwnd, &mut rc);
    let p = neuro_mini_ui::Palette::default();
    let enabled = IsWindowEnabled(hwnd).as_bool();
    let focused = GetFocus() == hwnd;
    let brush = HBRUSH(GetStockObject(DC_BRUSH).0);
    let fill = |l: i32, t: i32, r: i32, b: i32, rgb: u32| {
        if r > l && b > t {
            SetDCBrushColor(dc, dialog_controls::color(rgb));
            FillRect(
                dc,
                &RECT {
                    left: l,
                    top: t,
                    right: r,
                    bottom: b,
                },
                brush,
            );
        }
    };
    fill(rc.left, rc.top, rc.right, rc.bottom, p.surface);
    let pad = 6;
    let cy = (rc.top + rc.bottom) / 2;
    let track_h = 4;
    fill(
        rc.left + pad,
        cy - track_h / 2,
        rc.right - pad,
        cy + track_h / 2,
        p.control,
    );
    let span = (rc.right - rc.left - pad * 2).max(1);
    let knob_x = rc.left + pad + ((i32::from(value.saturating_sub(1)) * span) + 49) / 99;
    let accent = if enabled { p.signal } else { p.muted };
    fill(
        rc.left + pad,
        cy - track_h / 2,
        knob_x,
        cy + track_h / 2,
        accent,
    );
    let kw = 4;
    let kh = ((rc.bottom - rc.top) / 2 - 2).clamp(5, 9);
    fill(
        knob_x - kw,
        cy - kh,
        knob_x + kw,
        cy + kh,
        if enabled { p.text } else { p.muted },
    );
    if focused && enabled {
        SetDCBrushColor(dc, dialog_controls::color(p.signal));
        FrameRect(dc, &rc, brush);
    }
}

unsafe extern "system" fn slider_proc(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    id: usize,
    raw: usize,
) -> LRESULT {
    let data = &*(raw as *const SliderData);
    match msg {
        WM_GETDLGCODE => LRESULT(DLGC_WANTARROWS as isize),
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let dc = BeginPaint(hwnd, &mut ps);
            if !dc.is_invalid() {
                paint_slider(hwnd, dc, data.value.get());
            }
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_LBUTTONDOWN if IsWindowEnabled(hwnd).as_bool() => {
            let _ = SetFocus(Some(hwnd));
            SetCapture(hwnd);
            set_from_x(hwnd, data, lp.0 as i16 as i32);
            LRESULT(0)
        }
        WM_MOUSEMOVE if GetCapture() == hwnd => {
            set_from_x(hwnd, data, lp.0 as i16 as i32);
            LRESULT(0)
        }
        WM_LBUTTONUP if GetCapture() == hwnd => {
            set_from_x(hwnd, data, lp.0 as i16 as i32);
            let _ = ReleaseCapture();
            LRESULT(0)
        }
        WM_KEYDOWN if IsWindowEnabled(hwnd).as_bool() => {
            let cur = data.value.get();
            let next = match wp.0 as u16 {
                0x25 => cur.saturating_sub(1).max(1),
                0x27 => (cur + 1).min(100),
                0x24 => 1,
                0x23 => 100,
                _ => cur,
            };
            if data.value.replace(next) != next {
                let _ = InvalidateRect(Some(hwnd), None, false);
                if let Ok(parent) = GetParent(hwnd) {
                    let cmd = (usize::from(QUALITY_CHANGED) << 16) | (QUALITY_ID as usize & 0xffff);
                    let _ = SendMessageW(
                        parent,
                        WM_COMMAND,
                        Some(WPARAM(cmd)),
                        Some(LPARAM(hwnd.0 as isize)),
                    );
                }
            }
            LRESULT(0)
        }
        WM_SETFOCUS | WM_KILLFOCUS => {
            let _ = InvalidateRect(Some(hwnd), None, false);
            DefSubclassProc(hwnd, msg, wp, lp)
        }
        WM_NCDESTROY => {
            let _ = RemoveWindowSubclass(hwnd, Some(slider_proc), id);
            drop(Box::from_raw(raw as *mut SliderData));
            DefSubclassProc(hwnd, msg, wp, lp)
        }
        _ => DefSubclassProc(hwnd, msg, wp, lp),
    }
}

pub(super) fn template() -> Vec<u32> {
    fn text(v: &mut Vec<u16>, s: &str) {
        v.extend(s.encode_utf16());
        v.push(0);
    }
    fn dword(v: &mut Vec<u16>, n: u32) {
        v.extend([n as u16, (n >> 16) as u16]);
    }
    let mut v = Vec::new();
    dword(
        &mut v,
        WS_POPUP.0
            | WS_CLIPCHILDREN.0
            | WS_CAPTION.0
            | WS_SYSMENU.0
            | DS_MODALFRAME as u32
            | DS_SETFONT as u32,
    );
    dword(&mut v, 0);
    v.extend([14, 0, 0, 324, 198, 0, 0]);
    text(&mut v, tr("export-title"));
    v.push(9);
    text(&mut v, "Microsoft YaHei UI");

    let owner_btn = WS_TABSTOP.0 | 0x000B; // BS_OWNERDRAW
    let edit_style = WS_BORDER.0 | WS_TABSTOP.0 | ES_AUTOHSCROLL as u32;
    for (id, class, style, x, y, w, h, s) in [
        (202, 0x82, 0, 12, 9, 300, 13, tr("export-format-label")),
        (101, 0x80, owner_btn, 12, 24, 56, 18, "PNG"),
        (102, 0x80, owner_btn, 72, 24, 56, 18, "JPEG"),
        (
            103,
            0x80,
            owner_btn,
            132,
            24,
            88,
            18,
            tr("export-webp-lossless"),
        ),
        (
            104,
            0x80,
            owner_btn,
            224,
            24,
            88,
            18,
            tr("export-webp-lossy"),
        ),
        (200, 0x82, 0, 12, 47, 300, 13, ""),
        (201, 0x82, 0, 12, 81, 300, 13, ""),
        (203, 0x82, 0, 12, 99, 300, 13, tr("export-folder-label")),
        (DIR_EDIT_ID as u16, 0x81, edit_style, 12, 114, 242, 18, ""),
        (
            BROWSE_ID as u16,
            0x80,
            owner_btn,
            258,
            114,
            54,
            18,
            tr("export-browse-folder"),
        ),
        (204, 0x82, 0, 12, 137, 300, 13, tr("export-filename-label")),
        (FILE_EDIT_ID as u16, 0x81, edit_style, 12, 152, 300, 18, ""),
        (
            IDOK.0 as u16,
            0x80,
            owner_btn,
            176,
            174,
            66,
            18,
            tr("export-confirm-button"),
        ),
        (
            IDCANCEL.0 as u16,
            0x80,
            owner_btn,
            246,
            174,
            66,
            18,
            tr("common-cancel"),
        ),
    ] {
        if !v.len().is_multiple_of(2) {
            v.push(0);
        }
        dword(&mut v, WS_CHILD.0 | WS_VISIBLE.0 | style);
        dword(&mut v, 0);
        v.extend([x, y, w, h, id, 0xffff, class]);
        text(&mut v, s);
        v.push(0);
    }
    if !v.len().is_multiple_of(2) {
        v.push(0);
    }
    v.chunks_exact(2)
        .map(|p| u32::from(p[0]) | (u32::from(p[1]) << 16))
        .collect()
}
