//! Neuro-themed geometric icons and button surfaces for the desktop pin arrangement palette.
use crate::{desktop::dialog_controls, pin_arrange_math::Layout};
use windows::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    UI::{Controls::*, WindowsAndMessaging::*},
};

pub(super) unsafe fn draw_icon_button(item: &DRAWITEMSTRUCT, layout: Layout, active: bool) {
    let p = neuro_mini_ui::Palette::default();
    let mut cursor = POINT::default();
    let mut screen = RECT::default();
    let hot = GetCursorPos(&mut cursor).is_ok()
        && GetWindowRect(item.hwndItem, &mut screen).is_ok()
        && cursor.x >= screen.left
        && cursor.x < screen.right
        && cursor.y >= screen.top
        && cursor.y < screen.bottom;
    let disabled = item.itemState.0 & ODS_DISABLED.0 != 0;
    let focus = item.itemState.0 & ODS_FOCUS.0 != 0;
    let selected = item.itemState.0 & ODS_SELECTED.0 != 0;
    let dc = item.hDC;
    let rc = item.rcItem;
    let brush = HBRUSH(GetStockObject(DC_BRUSH).0);
    let fill_rect = |l: i32, t: i32, r: i32, b: i32, rgb: u32| {
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
    let bg = if !disabled && (hot || active || selected) {
        p.control
    } else {
        p.surface
    };
    fill_rect(rc.left, rc.top, rc.right, rc.bottom, bg);
    let border_rgb = if !disabled && (active || focus) {
        p.signal
    } else {
        p.muted
    };
    SetDCBrushColor(dc, dialog_controls::color(border_rgb));
    FrameRect(dc, &rc, brush);

    let ink = if disabled {
        p.muted
    } else if active || hot {
        p.signal
    } else {
        p.text
    };
    let accent = if disabled { p.muted } else { p.signal };
    let cx = (rc.left + rc.right) / 2;
    let cy = (rc.top + rc.bottom) / 2;
    let s = ((rc.bottom - rc.top).min(rc.right - rc.left) - 10).max(12);
    let h = s / 2;
    let bar = (s / 10).max(2);

    match layout {
        Layout::Left => {
            fill_rect(cx - h, cy - h, cx - h + bar, cy + h, accent);
            fill_rect(cx - h + bar + 2, cy - h + 2, cx + h - 1, cy - 1, ink);
            fill_rect(cx - h + bar + 2, cy + 2, cx + 2, cy + h - 2, ink);
        }
        Layout::CenterX => {
            fill_rect(cx - bar / 2, cy - h, cx - bar / 2 + bar, cy + h, accent);
            fill_rect(cx - h + 1, cy - h + 3, cx + h - 1, cy - 1, ink);
            fill_rect(cx - h / 2 - 1, cy + 2, cx + h / 2 + 1, cy + h - 3, ink);
        }
        Layout::Right => {
            fill_rect(cx + h - bar, cy - h, cx + h, cy + h, accent);
            fill_rect(cx - h + 1, cy - h + 2, cx + h - bar - 2, cy - 1, ink);
            fill_rect(cx - 2, cy + 2, cx + h - bar - 2, cy + h - 2, ink);
        }
        Layout::Top => {
            fill_rect(cx - h, cy - h, cx + h, cy - h + bar, accent);
            fill_rect(cx - h + 2, cy - h + bar + 2, cx - 1, cy + h - 1, ink);
            fill_rect(cx + 2, cy - h + bar + 2, cx + h - 2, cy + 2, ink);
        }
        Layout::CenterY => {
            fill_rect(cx - h, cy - bar / 2, cx + h, cy - bar / 2 + bar, accent);
            fill_rect(cx - h + 3, cy - h + 1, cx - 1, cy + h - 1, ink);
            fill_rect(cx + 2, cy - h / 2 - 1, cx + h - 3, cy + h / 2 + 1, ink);
        }
        Layout::Bottom => {
            fill_rect(cx - h, cy + h - bar, cx + h, cy + h, accent);
            fill_rect(cx - h + 2, cy - h + 1, cx - 1, cy + h - bar - 2, ink);
            fill_rect(cx + 2, cy - 2, cx + h - 2, cy + h - bar - 2, ink);
        }
        Layout::DistributeX => {
            fill_rect(cx - h, cy - h, cx - h + bar, cy + h, accent);
            fill_rect(cx + h - bar, cy - h, cx + h, cy + h, accent);
            fill_rect(cx - 3, cy - h + 3, cx + 3, cy + h - 3, ink);
        }
        Layout::DistributeY => {
            fill_rect(cx - h, cy - h, cx + h, cy - h + bar, accent);
            fill_rect(cx - h, cy + h - bar, cx + h, cy + h, accent);
            fill_rect(cx - h + 3, cy - 3, cx + h - 3, cy + 3, ink);
        }
        Layout::CompactX => {
            let w = (s - 6) / 3;
            let x0 = cx - h + 1;
            fill_rect(x0, cy - h + 3, x0 + w, cy + h - 3, ink);
            fill_rect(x0 + w + 2, cy - h + 1, x0 + w + 3, cy + h - 1, accent);
            fill_rect(x0 + w + 5, cy - h + 3, x0 + 2 * w + 5, cy + h - 3, ink);
        }
        Layout::CompactY => {
            let bh = (s - 6) / 3;
            let y0 = cy - h + 1;
            fill_rect(cx - h + 3, y0, cx + h - 3, y0 + bh, ink);
            fill_rect(cx - h + 1, y0 + bh + 2, cx + h - 1, y0 + bh + 3, accent);
            fill_rect(cx - h + 3, y0 + bh + 5, cx + h - 3, y0 + 2 * bh + 5, ink);
        }
        Layout::Grid => {
            let cell = (h - 2).max(4);
            fill_rect(cx - cell - 1, cy - cell - 1, cx - 1, cy - 1, ink);
            fill_rect(cx + 1, cy - cell - 1, cx + cell + 1, cy - 1, accent);
            fill_rect(cx - cell - 1, cy + 1, cx - 1, cy + cell + 1, ink);
            fill_rect(cx + 1, cy + 1, cx + cell + 1, cy + cell + 1, ink);
        }
    }
}
