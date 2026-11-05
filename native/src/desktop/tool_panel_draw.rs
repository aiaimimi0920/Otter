//! 原生按钮 owner-draw：映射 Neuro 控件/文字/信号黄，不创建持久 GDI 对象。
use super::State;
use windows::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    UI::{Controls::*, WindowsAndMessaging::GetParent},
};
pub(super) unsafe fn fill(dc: HDC, rect: &RECT, color: u32) {
    let previous = SetDCBrushColor(dc, COLORREF(color));
    FillRect(dc, rect, HBRUSH(GetStockObject(DC_BRUSH).0));
    SetDCBrushColor(dc, previous);
}
pub(super) unsafe fn button(item: &DRAWITEMSTRUCT, state: &State) {
    let entry = super::super::tool_commands::TOOLS
        .iter()
        .find(|entry| entry.id == item.CtlID as usize);
    let label = entry
        .map(|entry| {
            entry
                .label
                .split('\t')
                .next()
                .unwrap_or(entry.label)
                .to_owned()
        })
        .or_else(|| super::super::tool_panel_actions::label(item.CtlID as usize, state));
    let Some(label) = label else {
        return;
    };
    let saved = SaveDC(item.hDC);
    if saved == 0 {
        return;
    }
    let selected = entry.is_some_and(|entry| state.canvas.borrow().tool == entry.tool);
    let disabled = item.itemState.0 & ODS_DISABLED.0 != 0;
    let focus = item.itemState.0 & ODS_FOCUS.0 != 0;
    fill(
        item.hDC,
        &item.rcItem,
        if focus { 0x38ffd9 } else { 0x2c2218 },
    );
    let mut inner = item.rcItem;
    inner.left += 2;
    inner.top += 2;
    inner.right -= 2;
    inner.bottom -= 2;
    fill(
        item.hDC,
        &inner,
        if item.itemState.0 & ODS_SELECTED.0 != 0 {
            0x2c2218
        } else {
            0x201711
        },
    );
    SetBkMode(item.hDC, TRANSPARENT);
    SetTextColor(
        item.hDC,
        COLORREF(if disabled {
            0x9f9a92
        } else if selected {
            0x38ffd9
        } else {
            0xeff8f7
        }),
    );
    if let Ok(panel) = GetParent(item.hwndItem) {
        SelectObject(item.hDC, super::font::object(panel));
    }
    let mut text: Vec<u16> = format!("{}{label}", if selected { "✓ " } else { "" })
        .encode_utf16()
        .collect();
    DrawTextW(
        item.hDC,
        &mut text,
        &mut inner,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
    );
    let _ = RestoreDC(item.hDC, saved);
}
