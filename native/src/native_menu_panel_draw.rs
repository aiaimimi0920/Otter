//! Neuro semantic colors for panels/controls/text; restore GDI state each time, without cross-frame DC ownership.
use super::*;
#[path = "native_menu_panel_columns.rs"]
mod columns;
#[path = "native_menu_panel_gallery.rs"]
mod gallery;
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
    let target = BeginPaint(hwnd, &mut paint.1);
    let mut bounds = RECT::default();
    let result = (|| -> Result<()> {
        GetClientRect(hwnd, &mut bounds)?;
        let Some(frame) = crate::desktop::buffered_paint::Frame::begin(target, &paint.1.rcPaint)?
        else {
            return Ok(());
        };
        paint_frame(frame.dc, bounds, data);
        if !data.failed.get() {
            frame.commit()?;
        }
        Ok(())
    })();
    if result.is_err() {
        data.failed.set(true);
    }
}

unsafe fn paint_frame(dc: HDC, bounds: RECT, data: &Data) {
    let saved = SaveDC(dc);
    if saved == 0 {
        data.failed.set(true);
        return;
    }
    let _saved = SavedDc(dc, saved);
    {
        fill(dc, &bounds, 0x18120e);
        let bar = data.bar.get();
        if let Some(bar) = bar {
            fill(dc, &bar.track, 0x201711);
            fill(dc, &bar.thumb, 0x9f9a92);
        }
        SetBkMode(dc, TRANSPARENT);
        SelectObject(dc, data.fonts.object(false));
        if let Some(content) = data.preview.borrow().as_ref() {
            super::preview::paint(dc, content, bounds, data);
            return;
        }
        let mut rows = data.rows.borrow_mut();
        if data.columns.get() == 3 {
            gallery::paint(
                dc,
                data,
                &mut rows,
                bar.map_or(bounds.right, |bar| bar.track.left),
            );
            data.paints.set(data.paints.get().saturating_add(1));
            return;
        }
        let Some(shortcut_width) = rows.shortcut_width(dc) else {
            data.failed.set(true);
            return;
        };
        let super::text::PaintRows {
            rows, text: texts, ..
        } = &mut *rows;
        for (index, (row, content)) in rows.iter().zip(texts).enumerate() {
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
            let (label, shortcut) = content.parts();
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
            } else if row.checked {
                let mut glyph = [if row.radio {
                    '●' as u16
                } else {
                    '✓' as u16
                }];
                text(dc, &mut glyph, layout.mark, DT_CENTER | DT_NOPREFIX);
            }
            text(dc, shortcut, layout.shortcut, DT_RIGHT | DT_NOPREFIX);
            if row.submenu {
                text(dc, &mut ['›' as u16], layout.arrow, DT_CENTER | DT_NOPREFIX);
            }
            SelectObject(dc, data.fonts.object(row.default));
            text(dc, label, layout.label, DT_LEFT);
        }
        data.paints.set(data.paints.get().saturating_add(1));
    }
}

unsafe fn text(dc: HDC, value: &mut [u16], mut rect: RECT, flags: DRAW_TEXT_FORMAT) {
    if value.is_empty() || rect.left >= rect.right || rect.top >= rect.bottom {
        return;
    }
    // Do not use DT_MODIFYSTRING; GDI must not mutate strings reused by later redraws.
    DrawTextW(
        dc,
        value,
        &mut rect,
        flags | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
    );
}
