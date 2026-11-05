//! 经真实标准子控件和父窗口过程绘到自有内存 DIB，不采集桌面或注入输入。
use super::*;

struct Image {
    dc: HDC,
    bitmap: HBITMAP,
    old: HGDIOBJ,
    bits: *const u8,
    length: usize,
}
impl Image {
    unsafe fn print(hwnd: HWND) -> Self {
        let mut rect = RECT::default();
        GetClientRect(hwnd, &mut rect).unwrap();
        assert!((1..=1024).contains(&rect.right) && (1..=1024).contains(&rect.bottom));
        let dc = CreateCompatibleDC(None);
        assert!(!dc.is_invalid());
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: rect.right,
                biHeight: -rect.bottom,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let bitmap = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();
        let old = SelectObject(dc, bitmap.into());
        let length = rect.right as usize * rect.bottom as usize * 4;
        std::ptr::write_bytes(bits, 0xa5, length);
        let image = Self {
            dc,
            bitmap,
            old,
            bits: bits.cast(),
            length,
        };
        SendMessageW(
            hwnd,
            WM_PRINTCLIENT,
            Some(WPARAM(dc.0 as usize)),
            Some(LPARAM((PRF_CLIENT | PRF_ERASEBKGND) as isize)),
        );
        assert!(GdiFlush().as_bool());
        image
    }
    unsafe fn readable(&self, background: [u8; 3], name: &str) {
        self.readable_with_ink(background, [0xef, 0xf8, 0xf7], name);
    }
    unsafe fn readable_with_ink(&self, background: [u8; 3], ink: [u8; 3], name: &str) {
        let pixels = std::slice::from_raw_parts(self.bits, self.length);
        let background_count = pixels
            .chunks_exact(4)
            .filter(|p| p[..3] == background)
            .count();
        assert!(
            background_count > self.length / 8,
            "{name}: background {background_count}/{}",
            self.length / 4
        );
        assert!(
            pixels.chunks_exact(4).any(|p| p[..3] == ink),
            "{name}: missing readable text"
        );
    }
    unsafe fn option(parent: HWND, combo: HWND, index: u32, state: ODS_FLAGS) -> Self {
        let image = Self::print(combo);
        std::ptr::write_bytes(image.bits.cast_mut(), 0xa5, image.length);
        let mut rect = RECT::default();
        GetClientRect(combo, &mut rect).unwrap();
        let item = DRAWITEMSTRUCT {
            CtlType: ODT_COMBOBOX,
            itemID: index,
            itemAction: ODA_DRAWENTIRE,
            itemState: state,
            hwndItem: combo,
            hDC: image.dc,
            rcItem: rect,
            ..Default::default()
        };
        assert_eq!(
            SendMessageW(
                parent,
                WM_DRAWITEM,
                None,
                Some(LPARAM((&item as *const DRAWITEMSTRUCT) as isize))
            )
            .0,
            1
        );
        assert!(GdiFlush().as_bool());
        image
    }
}
impl Drop for Image {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.old);
            assert!(DeleteObject(self.bitmap.into()).as_bool());
            assert!(DeleteDC(self.dc).as_bool());
        }
    }
}

#[test]
fn ordinary_label_and_unfocused_buttons_paint_readable_text_through_real_window_procedure() {
    unsafe {
        let f = Fixture::new();
        let title = f.panel.ui.placements.borrow()[0].0;
        Image::print(title).readable([0x18, 0x12, 0x0e], "title");
        for key in ["reload", "setting:0", "run", "stop", "enabled:boolean"] {
            let child = f.binding(key).hwnd;
            assert_ne!(GetFocus(), child);
            Image::print(child).readable([0x20, 0x17, 0x11], key);
        }
    }
}

#[test]
fn ordinary_edit_and_combo_paint_readable_text_without_a_focus_redraw() {
    unsafe {
        let f = Fixture::new();
        for key in ["text:edit", "n:edit", "mode:options"] {
            let child = f.binding(key).hwnd;
            assert_ne!(GetFocus(), child);
            Image::print(child).readable([0x20, 0x17, 0x11], key);
        }
    }
}

#[test]
fn option_items_keep_readable_selected_disabled_and_distinct_system_strings() {
    unsafe {
        let f = Fixture::new();
        let combo = f.binding("mode:options").hwnd;
        let normal = Image::option(f.panel.handle(), combo, 0, ODS_FLAGS(0));
        normal.readable([0x20, 0x17, 0x11], "unselected option");
        let other = Image::option(f.panel.handle(), combo, 1, ODS_FLAGS(0));
        other.readable([0x20, 0x17, 0x11], "second option");
        assert_ne!(
            std::slice::from_raw_parts(normal.bits, normal.length),
            std::slice::from_raw_parts(other.bits, other.length),
            "option labels were collapsed"
        );
        Image::option(
            f.panel.handle(),
            combo,
            1,
            ODS_FLAGS(ODS_SELECTED.0 | ODS_FOCUS.0),
        )
        .readable_with_ink([0xf1, 0xf5, 0xf3], [0x2b, 0x25, 0x20], "selected option");
        Image::option(f.panel.handle(), combo, 1, ODS_DISABLED).readable_with_ink(
            [0x20, 0x17, 0x11],
            [0x9f, 0x9a, 0x92],
            "disabled option",
        );
        assert_eq!(SendMessageW(combo, CB_GETCURSEL, None, None).0, 0);
        assert_eq!(f.value("mode"), "fast");
    }
}

#[test]
fn option_labels_are_bounded_without_changing_indices_values_or_dpi_owner() {
    unsafe {
        let label = format!("长🙂\n{}", "🙂".repeat(200));
        let f = Fixture::with_parameters(serde_json::json!([
            {"id":"mode","widget":"select","options":[
                {"label":label,"value":"fast"},{"label":"数","value":2}
            ],"default":"fast"}
        ]));
        let combo = f.binding("mode:options").hwnd;
        let count = SendMessageW(combo, CB_GETLBTEXTLEN, Some(WPARAM(0)), None).0;
        assert!((1..=256).contains(&count));
        let mut text = [0u16; 257];
        assert_eq!(
            SendMessageW(
                combo,
                CB_GETLBTEXT,
                Some(WPARAM(0)),
                Some(LPARAM(text.as_mut_ptr() as isize))
            )
            .0,
            count
        );
        assert!(!String::from_utf16(&text[..count as usize])
            .unwrap()
            .contains('\n'));
        let expected = (24 * GetDpiForWindow(combo).max(96)).div_ceil(96) as isize;
        assert_eq!(
            SendMessageW(combo, CB_GETITEMHEIGHT, None, None).0,
            expected
        );
        SendMessageW(combo, CB_SETCURSEL, Some(WPARAM(1)), None);
        SendMessageW(
            f.panel.handle(),
            WM_COMMAND,
            Some(WPARAM((CBN_SELCHANGE as usize) << 16)),
            Some(LPARAM(combo.0 as isize)),
        );
        f.tick();
        assert_eq!(f.value("mode"), 2);
    }
}

#[test]
fn shown_panel_dispatches_initial_paint_for_every_child_without_focusing_each_one() {
    unsafe {
        let f = Fixture::new();
        let _ = ShowWindow(f.host.0, SW_SHOWNOACTIVATE);
        f.panel.show();
        let mut client = RECT::default();
        GetClientRect(f.panel.handle(), &mut client).unwrap();
        let dpi = GetDpiForWindow(f.panel.handle());
        // 完全位于滚动客户区外的字段尚无可见失效区，不要求其提前paint。
        let children: Vec<_> = f
            .panel
            .ui
            .placements
            .borrow()
            .iter()
            .filter(|(_, r)| {
                let r = layout::scaled(*r, dpi);
                r.left < client.right && r.right > 0 && r.top < client.bottom && r.bottom > 0
            })
            .map(|p| p.0)
            .collect();
        let mut painted = std::collections::HashSet::new();
        let mut message = MSG::default();
        for _ in 0..512 {
            if !PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                break;
            }
            if message.message == WM_PAINT {
                painted.insert(message.hwnd.0 as usize);
            }
            if !f.panel.translate(&message) {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            f.tick();
        }
        for child in children {
            assert!(
                painted.contains(&(child.0 as usize)),
                "child {} was never painted",
                GetDlgCtrlID(child)
            );
            assert!(
                !GetUpdateRect(child, None, false).as_bool(),
                "child {} still invalid",
                GetDlgCtrlID(child)
            );
        }
    }
}
