//! 图片卡片沿主干三列平铺并居中裁切；删除按钮单独响应悬停且避开选中边框。
use super::*;

pub(super) unsafe fn paint(dc: HDC, data: &Data, rows: &[Row], width: i32) {
    let layout = crate::native_menu::layout::Layout {
        columns: 3,
        row: data.row_height,
    };
    for (slot, row) in rows.iter().enumerate() {
        if row.label.is_empty() && row.thumbnail.is_none() {
            continue;
        }
        let rect = layout.rect(slot, width);
        let background = if row.focused && row.enabled {
            0x2c2218
        } else {
            0x201711
        };
        fill(dc, &rect, background);
        if let Some(image) = &row.thumbnail {
            if !thumbnail::draw_cover(dc, rect, image, background) {
                data.failed.set(true);
            }
        } else {
            let padding = font::scaled(5, data.dpi);
            let mut text_rect = RECT {
                left: rect.left + padding,
                right: rect.right - padding,
                top: rect.top + padding,
                bottom: rect.bottom - padding,
            };
            SetTextColor(dc, COLORREF(if row.enabled { 0xeff8f7 } else { 0x9f9a92 }));
            let mut units: Vec<u16> = row.label.encode_utf16().collect();
            DrawTextW(
                dc,
                &mut units,
                &mut text_rect,
                DT_CENTER | DT_WORDBREAK | DT_END_ELLIPSIS,
            );
        }
        if row.focused && row.enabled {
            let thickness = font::scaled(2, data.dpi).max(1);
            for edge in [
                RECT {
                    bottom: rect.top + thickness,
                    ..rect
                },
                RECT {
                    top: rect.bottom - thickness,
                    ..rect
                },
                RECT {
                    right: rect.left + thickness,
                    ..rect
                },
                RECT {
                    left: rect.right - thickness,
                    ..rect
                },
            ] {
                fill(dc, &edge, 0x38ffd9);
            }
        }
        if row.deletable && row.enabled {
            let mut button = layout.delete_rect(slot, width);
            let hovered = data.delete_hover.get() == Some(row.key);
            fill(dc, &button, if hovered { 0x5e3ff4 } else { 0x18120e });
            SetTextColor(dc, COLORREF(if hovered { 0xffffff } else { 0x5e3ff4 }));
            let mut glyph: Vec<u16> = "×".encode_utf16().collect();
            DrawTextW(
                dc,
                &mut glyph,
                &mut button,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Window(HWND);
    impl Drop for Window {
        fn drop(&mut self) {
            unsafe {
                let _ = DestroyWindow(self.0);
            }
        }
    }
    struct Buffer(HDC, HBITMAP, HGDIOBJ);
    impl Buffer {
        unsafe fn new(width: i32, height: i32) -> Self {
            let dc = CreateCompatibleDC(None);
            assert!(!dc.is_invalid());
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
            let bitmap =
                CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();
            let old = SelectObject(dc, HGDIOBJ(bitmap.0));
            Self(dc, bitmap, old)
        }
    }
    impl Drop for Buffer {
        fn drop(&mut self) {
            unsafe {
                SelectObject(self.0, self.2);
                let _ = DeleteObject(HGDIOBJ(self.1 .0));
                let _ = DeleteDC(self.0);
            }
        }
    }
    #[test]
    fn focused_deletable_card_keeps_all_four_border_edges_in_rendered_pixels() {
        unsafe {
            let owner = Window(
                CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("STATIC"),
                    w!("gallery paint test"),
                    WS_OVERLAPPED,
                    0,
                    0,
                    100,
                    100,
                    None,
                    None,
                    None,
                    None,
                )
                .unwrap(),
            );
            for dpi in [96, 120, 144, 192, 288] {
                let width = font::scaled(248, dpi);
                let height = font::scaled(80, dpi);
                let row = Row {
                    deletable: true,
                    key: 0,
                    thumbnail: Some(
                        crate::image_thumbnail::Thumbnail::from_rgba(1, 1, &[40, 50, 60, 255])
                            .unwrap(),
                    ),
                    label: String::new(),
                    enabled: true,
                    checked: false,
                    submenu: false,
                    separator: false,
                    focused: true,
                    radio: false,
                    default: false,
                };
                let bounds = RECT {
                    right: width,
                    bottom: height,
                    ..Default::default()
                };
                let mut panel = Panel::create(owner.0, bounds, height, vec![row]).unwrap();
                panel.data.dpi = dpi;
                panel.data.fonts = font::Fonts::new(dpi).unwrap();
                let buffer = Buffer::new(width, height);
                SetBkMode(buffer.0, TRANSPARENT);
                SelectObject(buffer.0, panel.data.fonts.object(false));
                paint(buffer.0, &panel.data, &panel.data.rows.borrow(), width);
                GdiFlush().ok().unwrap();
                assert!(!panel.data.failed.get());
                let layout = crate::native_menu::layout::Layout {
                    columns: 3,
                    row: height,
                };
                let card = layout.rect(0, width);
                let thickness = font::scaled(2, dpi);
                for y in card.top..card.bottom {
                    for x in card.left..card.right {
                        if x < card.left + thickness
                            || x >= card.right - thickness
                            || y < card.top + thickness
                            || y >= card.bottom - thickness
                        {
                            assert_eq!(
                                GetPixel(buffer.0, x, y),
                                COLORREF(0x38ffd9),
                                "dpi={dpi}, x={x}, y={y}"
                            );
                        }
                    }
                }
                let button = layout.delete_rect(0, width);
                assert_eq!(
                    GetPixel(buffer.0, button.left, button.top),
                    COLORREF(0x18120e)
                );
                // ClearType 在 125% 下可能没有纯前景色像素；仍须有可见红色字形。
                let red_pixels = (button.top..button.bottom)
                    .flat_map(|y| (button.left..button.right).map(move |x| (x, y)))
                    .filter(|&(x, y)| {
                        let color = GetPixel(buffer.0, x, y).0;
                        let red = color & 0xff;
                        red > ((color >> 8) & 0xff) + 32 && red > ((color >> 16) & 0xff) + 32
                    })
                    .count();
                assert!(red_pixels >= 4, "dpi={dpi}, red_pixels={red_pixels}");
                panel.set_delete_hover(Some(0));
                paint(buffer.0, &panel.data, &panel.data.rows.borrow(), width);
                GdiFlush().ok().unwrap();
                assert_eq!(
                    GetPixel(buffer.0, button.left, button.top),
                    COLORREF(0x5e3ff4)
                );
                assert_eq!(
                    GetPixel(buffer.0, card.right - 1, card.top),
                    COLORREF(0x38ffd9)
                );
                panel.set_delete_hover(None);
                paint(buffer.0, &panel.data, &panel.data.rows.borrow(), width);
                GdiFlush().ok().unwrap();
                assert_eq!(
                    GetPixel(buffer.0, button.left, button.top),
                    COLORREF(0x18120e)
                );
            }
        }
    }
}
