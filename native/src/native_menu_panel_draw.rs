//! Neuro 面板/控件/文字语义色；GDI 状态每次恢复，不持有跨帧 DC。
use super::*;
#[path = "native_menu_panel_columns.rs"]
mod columns;
use crate::image_thumbnail::draw as thumbnail;
unsafe fn fill(dc: HDC, rect: &RECT, color: u32) {
    SetDCBrushColor(dc, COLORREF(color));
    FillRect(dc, rect, HBRUSH(GetStockObject(DC_BRUSH).0));
}
struct Paint(HWND, PAINTSTRUCT);
struct SavedDc(HDC, i32);
impl Drop for SavedDc {
    fn drop(&mut self) {
        unsafe {
            let _ = RestoreDC(self.0, self.1);
        }
    }
}
impl Drop for Paint {
    fn drop(&mut self) {
        unsafe {
            let _ = EndPaint(self.0, &self.1);
        }
    }
}
pub(super) unsafe fn paint(hwnd: HWND, data: &Data) {
    let mut paint = Paint(hwnd, PAINTSTRUCT::default());
    let dc = BeginPaint(hwnd, &mut paint.1);
    let saved = SaveDC(dc);
    if saved == 0 {
        data.failed.set(true);
        return;
    }
    let _saved = SavedDc(dc, saved);
    let mut bounds = RECT::default();
    if GetClientRect(hwnd, &mut bounds).is_ok() {
        fill(dc, &bounds, 0x18120e);
        let bar = data.bar.get();
        if let Some(bar) = bar {
            fill(dc, &bar.track, 0x201711);
            fill(dc, &bar.thumb, 0x9f9a92);
        }
        SetBkMode(dc, TRANSPARENT);
        SelectObject(dc, data.fonts.object(false));
        let rows = data.rows.borrow();
        let shortcut_width = rows
            .iter()
            .filter_map(|row| row.label.split_once('\t').map(|(_, shortcut)| shortcut))
            .map(|shortcut| {
                let units: Vec<u16> = shortcut.encode_utf16().collect();
                let mut size = SIZE::default();
                if !GetTextExtentPoint32W(dc, &units, &mut size).as_bool() {
                    data.failed.set(true);
                }
                size.cx
            })
            .max()
            .unwrap_or(0);
        for (index, row) in rows.iter().enumerate() {
            let top = index as i32 * data.row_height;
            if top >= bounds.bottom {
                break;
            }
            let mut rect = RECT {
                left: 0,
                top,
                right: bar.map_or(bounds.right, |bar| bar.track.left),
                bottom: (top + data.row_height).min(bounds.bottom),
            };
            if row.separator {
                rect.top += data.row_height / 2;
                rect.bottom = rect.top + font::scaled(1, data.dpi).max(1);
                fill(dc, &rect, 0x2c2218);
                continue;
            }
            SetTextColor(dc, COLORREF(if row.enabled { 0xeff8f7 } else { 0x9f9a92 }));
            if row.focused && row.enabled {
                fill(dc, &rect, 0x2c2218);
                let marker = RECT {
                    right: rect.left + font::scaled(2, data.dpi).max(1),
                    ..rect
                };
                fill(dc, &marker, 0x38ffd9);
            }
            let layout = columns::columns(rect, data.dpi, shortcut_width);
            let (label, shortcut) = row.label.split_once('\t').unwrap_or((&row.label, ""));
            SelectObject(dc, data.fonts.object(false));
            if let Some(image) = &row.thumbnail {
                if !thumbnail::draw(
                    dc,
                    layout.mark,
                    image,
                    if row.focused && row.enabled {
                        0x2c2218
                    } else {
                        0x18120e
                    },
                ) {
                    data.failed.set(true);
                }
            } else {
                text(
                    dc,
                    if row.checked {
                        if row.radio {
                            "●"
                        } else {
                            "✓"
                        }
                    } else {
                        ""
                    },
                    layout.mark,
                    DT_CENTER | DT_NOPREFIX,
                );
            }
            text(dc, shortcut, layout.shortcut, DT_RIGHT | DT_NOPREFIX);
            text(
                dc,
                if row.submenu { "›" } else { "" },
                layout.arrow,
                DT_CENTER | DT_NOPREFIX,
            );
            SelectObject(dc, data.fonts.object(row.default));
            text(dc, label, layout.label, DT_LEFT);
        }
        data.paints.set(data.paints.get().saturating_add(1));
    } else {
        data.failed.set(true);
    }
}

unsafe fn text(dc: HDC, value: &str, mut rect: RECT, flags: DRAW_TEXT_FORMAT) {
    if value.is_empty() || rect.left >= rect.right || rect.top >= rect.bottom {
        return;
    }
    let mut units: Vec<u16> = value.encode_utf16().collect();
    DrawTextW(
        dc,
        &mut units,
        &mut rect,
        flags | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
    );
}
