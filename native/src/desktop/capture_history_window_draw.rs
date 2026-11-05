//! Neuro深色历史面板绘制；缩略图复用原64显示primitive，240复用图不退化。
use super::*;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
unsafe fn fill(dc: HDC, rect: RECT, color: u32) {
    SetDCBrushColor(dc, COLORREF(color));
    FillRect(dc, &rect, HBRUSH(GetStockObject(DC_BRUSH).0));
}
unsafe fn text(dc: HDC, mut rect: RECT, text: &str, color: u32) {
    SetBkMode(dc, TRANSPARENT);
    SetTextColor(dc, COLORREF(color));
    SelectObject(dc, GetStockObject(DEFAULT_GUI_FONT));
    let mut units: Vec<u16> = text.encode_utf16().collect();
    DrawTextW(
        dc,
        &mut units,
        &mut rect,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
    );
}
pub(super) unsafe fn item(state: &State, item: &DRAWITEMSTRUCT) {
    let dc = item.hDC;
    let saved = SaveDC(dc);
    if saved == 0 {
        return;
    }
    let rect = item.rcItem;
    let id = item.CtlID as usize;
    let selected = item.itemState.0 & ODS_SELECTED.0 != 0;
    fill(dc, rect, if selected { 0x2c2218 } else { 0x201711 });
    let (label, thumbnail, color) = {
        let data = state.data.borrow();
        match id {
            100..=123 => data.colors.get(id - 100).map(|entry| {
                (
                    entry.hex.clone(),
                    None,
                    Some(
                        u32::from(entry.rgb.r)
                            | u32::from(entry.rgb.g) << 8
                            | u32::from(entry.rgb.b) << 16,
                    ),
                )
            }),
            200..=223 => data.screenshots.get(id - 200).map(|entry| {
                (
                    format!("{}×{}", entry.width, entry.height),
                    entry.preview.as_ref().map(|p| p.small.clone()),
                    None,
                )
            }),
            300..=323 => Some(("删除".into(), None, None)),
            _ => None,
        }
        .unwrap_or_default()
    };
    if let Some(color) = color {
        let swatch = RECT {
            left: rect.left + 4,
            top: rect.top + 4,
            right: rect.left + 14,
            bottom: rect.bottom - 4,
        };
        fill(dc, swatch, color);
    }
    if let Some(thumbnail) = thumbnail {
        let image_rect = RECT {
            bottom: rect.bottom - 20,
            ..rect
        };
        let _ = crate::image_thumbnail::draw::draw(dc, image_rect, &thumbnail, 0x201711);
        text(
            dc,
            RECT {
                top: image_rect.bottom,
                ..rect
            },
            &label,
            0xeff8f7,
        );
    } else {
        text(
            dc,
            RECT {
                left: rect.left + if color.is_some() { 14 } else { 0 },
                ..rect
            },
            &label,
            if id >= 300 { 0x5e3ff4 } else { 0xeff8f7 },
        );
    }
    if item.itemState.0 & ODS_FOCUS.0 != 0 {
        let brush = CreateSolidBrush(COLORREF(0x38ffd9));
        FrameRect(dc, &rect, brush);
        let _ = DeleteObject(brush.into());
    }
    let _ = RestoreDC(dc, saved);
}
pub(super) unsafe fn paint(hwnd: HWND, state: &State) {
    let mut ps = PAINTSTRUCT::default();
    let dc = BeginPaint(hwnd, &mut ps);
    let saved = SaveDC(dc);
    if saved != 0 {
        let mut rect = RECT::default();
        let _ = GetClientRect(hwnd, &mut rect);
        fill(dc, rect, 0x18120e);
        let scale = |v: i32| (i64::from(v) * GetDpiForWindow(hwnd).max(96) as i64 / 96) as i32;
        let scroll = state.scroll.get();
        let (empty_colors, empty_shots) = {
            let data = state.data.borrow();
            (data.colors.is_empty(), data.screenshots.is_empty())
        };
        text(
            dc,
            RECT {
                top: scale(8) - scroll,
                bottom: scale(36) - scroll,
                ..rect
            },
            if empty_colors {
                "取色历史 · 暂无记录"
            } else {
                "取色历史"
            },
            0xeff8f7,
        );
        text(
            dc,
            RECT {
                top: scale(184) - scroll,
                bottom: scale(216) - scroll,
                ..rect
            },
            if empty_shots {
                "截图历史 · 暂无记录"
            } else {
                "截图历史"
            },
            0xeff8f7,
        );
        let status = state.status.borrow().clone();
        if !status.is_empty() {
            text(
                dc,
                RECT {
                    top: rect.bottom - scale(28),
                    ..rect
                },
                &status,
                0x5e3ff4,
            );
        }
        let _ = RestoreDC(dc, saved);
    }
    let _ = EndPaint(hwnd, &ps);
}
