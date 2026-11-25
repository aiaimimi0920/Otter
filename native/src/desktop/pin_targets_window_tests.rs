//! Real LISTBOX messages verify the narrow header, stable HWND, no repaint without changes, and draft events; no user devices are used.
use super::*;
unsafe extern "system" fn count_selection_queries(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    if msg == LB_GETSEL {
        let count = unsafe { &*(data as *const Cell<usize>) };
        count.set(count.get() + 1);
    }
    unsafe { DefSubclassProc(hwnd, msg, wp, lp) }
}
#[test]
fn empty_catalog_click_is_safe_and_loaded_names_are_painted() {
    unsafe {
        let owner = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            windows::core::w!("STATIC"),
            None,
            WS_POPUP,
            300,
            200,
            400,
            300,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let menu = Window::open(
            owner,
            &[],
            RECT {
                left: 320,
                top: 240,
                right: 360,
                bottom: 270,
            },
        )
        .unwrap();
        let list = GetDlgItem(Some(menu.hwnd()), LIST).unwrap();
        assert_eq!(SendMessageW(list, LB_GETCOUNT, None, None).0, 0);
        let point = LPARAM((8 << 16) | 8);
        // An empty LISTBOX requests painting itemID == u32::MAX on focus; this is not a real item.
        let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(list));
        SendMessageW(list, WM_LBUTTONDOWN, Some(WPARAM(1)), Some(point));
        SendMessageW(list, WM_LBUTTONUP, None, Some(point));
        assert!(menu.event().is_none());
        let (width, empty) = painted_control(list);
        let rows = [Entry {
            id: "device".into(),
            name: "CODE 测试设备".into(),
            checked: false,
            enabled: true,
            command: 20,
        }];
        menu.update(&rows, true, true, false);
        let (_, loaded) = painted_control(list);
        let start = 28 * GetDpiForWindow(list).max(96) as usize / 96;
        assert!(
            empty
                .chunks_exact(width * 4)
                .zip(loaded.chunks_exact(width * 4))
                .any(|(a, b)| a[start * 4..] != b[start * 4..]),
            "loaded name must paint actual text"
        );
        let ink = neuro_mini_ui::Palette::default().text;
        assert!(
            loaded.chunks_exact(width * 4).any(|row| row[start * 4..]
                .chunks_exact(4)
                .any(|p| (u32::from(p[2]) << 16 | u32::from(p[1]) << 8 | u32::from(p[0])) == ink)),
            "the text region must contain foreground glyph pixels"
        );
        SendMessageW(list, WM_LBUTTONDOWN, Some(WPARAM(1)), Some(point));
        SendMessageW(list, WM_LBUTTONUP, None, Some(point));
        assert!(matches!(menu.event(), Some(Event::Toggle(id, true)) if id == "device"));
        assert!(menu.event().is_none());
        menu.update(&[], true, true, false);
        SendMessageW(list, WM_LBUTTONDOWN, Some(WPARAM(1)), Some(point));
        SendMessageW(list, WM_LBUTTONUP, None, Some(point));
        assert!(menu.event().is_none());
        drop(menu);
        DestroyWindow(owner).unwrap();
    }
}

#[test]
fn action_buttons_have_no_selected_outline_even_when_focused_or_pressed() {
    unsafe {
        let owner = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            windows::core::w!("STATIC"),
            None,
            WS_POPUP,
            300,
            200,
            400,
            300,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let menu = Window::open(
            owner,
            &[Entry {
                id: "device".into(),
                name: "CODE".into(),
                checked: false,
                enabled: true,
                command: 20,
            }],
            RECT {
                left: 320,
                top: 240,
                right: 360,
                bottom: 270,
            },
        )
        .unwrap();
        let p = neuro_mini_ui::Palette::default();
        for id in [4, 3, 1] {
            let button = GetDlgItem(Some(menu.hwnd()), id).unwrap();
            let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(button));
            for pressed in [false, true] {
                SendMessageW(
                    button,
                    BM_SETSTATE,
                    Some(WPARAM(usize::from(pressed))),
                    None,
                );
                let (width, pixels) = painted_control(button);
                let background = |pixel: &[u8]| {
                    let rgb =
                        u32::from(pixel[2]) << 16 | u32::from(pixel[1]) << 8 | u32::from(pixel[0]);
                    rgb == p.surface || rgb == p.control
                };
                assert!(
                    pixels[..width * 4].chunks_exact(4).all(background),
                    "button {id} top outline"
                );
                assert!(
                    pixels
                        .chunks_exact(width * 4)
                        .all(|row| background(&row[..4])),
                    "button {id} left outline"
                );
            }
            SendMessageW(button, BM_SETSTATE, Some(WPARAM(0)), None);
        }
        drop(menu);
        DestroyWindow(owner).unwrap();
    }
}

unsafe fn painted_control(hwnd: HWND) -> (usize, Vec<u8>) {
    let mut rect = RECT::default();
    GetClientRect(hwnd, &mut rect).unwrap();
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
    let length = (rect.right * rect.bottom * 4) as usize;
    std::ptr::write_bytes(bits.cast::<u8>(), 0, length);
    SendMessageW(
        hwnd,
        WM_PRINTCLIENT,
        Some(WPARAM(dc.0 as usize)),
        Some(LPARAM((PRF_CLIENT | PRF_ERASEBKGND) as isize)),
    );
    assert!(GdiFlush().as_bool());
    let pixels = std::slice::from_raw_parts(bits.cast::<u8>(), length).to_vec();
    SelectObject(dc, old);
    assert!(DeleteObject(bitmap.into()).as_bool());
    assert!(DeleteDC(dc).as_bool());
    (rect.right as usize, pixels)
}
#[test]
fn picker_preserves_native_selection_during_idle_refresh_and_emits_only_draft_events() {
    let queries = Cell::new(0usize);
    unsafe {
        let owner = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            windows::core::w!("STATIC"),
            None,
            WS_POPUP,
            300,
            200,
            400,
            300,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let rows = vec![
            Entry {
                id: "a".into(),
                name: "同名设备".into(),
                checked: false,
                enabled: true,
                command: 20,
            },
            Entry {
                id: "b".into(),
                name: "同名设备".into(),
                checked: true,
                enabled: true,
                command: 21,
            },
        ];
        let menu = Window::open(
            owner,
            &rows,
            RECT {
                left: 320,
                top: 240,
                right: 360,
                bottom: 270,
            },
        )
        .unwrap();
        let hwnd = menu.0.hwnd.get().unwrap();
        let list = GetDlgItem(Some(hwnd), LIST).unwrap();
        let rect = |window| {
            let mut r = RECT::default();
            GetWindowRect(window, &mut r).unwrap();
            r
        };
        let bounds = rect(hwnd);
        assert_eq!((bounds.left, bounds.top), (320, 270));
        assert_eq!(
            bounds.right - bounds.left,
            176 * GetDpiForWindow(owner).max(96) as i32 / 96
        );
        let header = [4, 3, 1].map(|id| rect(GetDlgItem(Some(hwnd), id).unwrap()));
        assert!(header.iter().all(|r| r.top == header[0].top));
        assert!(header[0].right <= header[1].left && header[1].right <= header[2].left);
        menu.update(&rows, false, true, false);
        assert!(
            windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(
                GetDlgItem(Some(hwnd), 3).unwrap()
            )
            .as_bool()
        );
        assert!(
            !windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(
                GetDlgItem(Some(hwnd), 1).unwrap()
            )
            .as_bool()
        );
        ValidateRect(Some(list), None).ok().unwrap();
        ValidateRect(Some(hwnd), None).ok().unwrap();
        assert!(SetWindowSubclass(
            list,
            Some(count_selection_queries),
            2,
            &queries as *const _ as usize
        )
        .as_bool());
        for _ in 0..16 {
            menu.update_draft(&rows, Some(&BTreeMap::new()), false, true, false);
        }
        let removed = RemoveWindowSubclass(list, Some(count_selection_queries), 2);
        assert!(removed.as_bool());
        assert_eq!(queries.get(), 0);
        assert_eq!(menu.0.hwnd.get(), Some(hwnd));
        assert!(!GetUpdateRect(list, None, false).as_bool());
        assert!(!GetUpdateRect(hwnd, None, false).as_bool());
        let before = painted_row(&menu, list);
        let mut checked_rows = rows.clone();
        checked_rows[0].checked = true;
        menu.update_draft(
            &rows,
            Some(&[("a".into(), true)].into()),
            false,
            true,
            false,
        );
        let mut reordered_commands = checked_rows.clone();
        reordered_commands[0].command += 100;
        menu.update(&reordered_commands, false, true, false);
        assert_eq!(
            menu.0.rows.borrow()[0].command,
            reordered_commands[0].command
        );
        let after = painted_row(&menu, list);
        assert_eq!(rect(hwnd), bounds);
        let width = (bounds.right - bounds.left) as usize;
        let text_start = 26 * GetDpiForWindow(list).max(96) as usize / 96;
        for (a, b) in before
            .chunks_exact(width * 4)
            .zip(after.chunks_exact(width * 4))
        {
            assert_eq!(
                &a[text_start * 4..],
                &b[text_start * 4..],
                "checking moved target name"
            );
        }
        assert_ne!(before, after, "check mark must be visible");
        menu.update(&rows, false, true, false);
        // The native list notifies only on release; background refresh must not cancel selection already changed on press.
        let point = LPARAM((8 << 16) | 8);
        SendMessageW(list, WM_LBUTTONDOWN, Some(WPARAM(1)), Some(point));
        assert!(SendMessageW(list, LB_GETSEL, Some(WPARAM(0)), None).0 > 0);
        for _ in 0..8 {
            menu.update(&rows, false, true, false);
        }
        SendMessageW(list, WM_LBUTTONUP, None, Some(point));
        assert!(matches!(menu.event(),Some(Event::Toggle(id,true)) if id=="a"));
        assert!(menu.event().is_none());
        assert!(GetUpdateRect(list, None, false).as_bool());
        menu.update(&checked_rows, false, true, false);
        SendMessageW(list, WM_LBUTTONDOWN, Some(WPARAM(1)), Some(point));
        menu.update(&checked_rows, false, true, false);
        SendMessageW(list, WM_LBUTTONUP, None, Some(point));
        assert!(matches!(menu.event(),Some(Event::Toggle(id,false)) if id=="a"));
        assert!(menu.event().is_none());
        menu.update(&rows, false, true, false);
        SendMessageW(list, LB_SETSEL, Some(WPARAM(1)), Some(LPARAM(0)));
        SendMessageW(
            hwnd,
            WM_COMMAND,
            Some(WPARAM((LBN_SELCHANGE as usize) << 16 | LIST as usize)),
            Some(LPARAM(list.0 as isize)),
        );
        assert!(matches!(menu.event(),Some(Event::Toggle(id,true)) if id=="a"));
        assert!(menu.event().is_none());
        SendMessageW(GetDlgItem(Some(hwnd), 4).unwrap(), BM_CLICK, None, None);
        assert!(matches!(menu.event(), Some(Event::All)));
        menu.update(&rows, true, true, false);
        SendMessageW(GetDlgItem(Some(hwnd), 1).unwrap(), BM_CLICK, None, None);
        assert!(matches!(menu.event(), Some(Event::Confirm)));
        let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(list));
        assert!(menu.translate(&MSG {
            hwnd: list,
            message: WM_KEYDOWN,
            wParam: WPARAM(13),
            ..Default::default()
        }));
        assert!(matches!(menu.event(), Some(Event::Confirm)));
        assert!(menu.event().is_none());
        drop(menu);
        assert!(!IsWindow(Some(hwnd)).as_bool());
        DestroyWindow(owner).unwrap();
    }
}

unsafe fn painted_row(menu: &Window, list: HWND) -> Vec<u8> {
    let width = 176 * GetDpiForWindow(list).max(96) as i32 / 96;
    let height = 24 * GetDpiForWindow(list).max(96) as i32 / 96;
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
    let bitmap = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();
    let old = SelectObject(dc, bitmap.into());
    let item = DRAWITEMSTRUCT {
        CtlID: LIST as u32,
        CtlType: ODT_LISTBOX,
        itemID: 0,
        itemAction: ODA_DRAWENTIRE,
        hwndItem: list,
        hDC: dc,
        rcItem: RECT {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        },
        ..Default::default()
    };
    let length = (width * height * 4) as usize;
    std::ptr::write_bytes(bits.cast::<u8>(), 0, length);
    draw(&menu.0, &item);
    assert!(GdiFlush().as_bool());
    let pixels = std::slice::from_raw_parts(bits.cast::<u8>(), length).to_vec();
    SelectObject(dc, old);
    assert!(DeleteObject(bitmap.into()).as_bool());
    assert!(DeleteDC(dc).as_bool());
    pixels
}

#[test]
fn custom_scroll_reaches_last_target_and_keeps_native_keyboard_selection() {
    unsafe {
        let owner = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            windows::core::w!("STATIC"),
            None,
            WS_POPUP,
            300,
            200,
            400,
            300,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let rows: Vec<_> = (0..30)
            .map(|i| Entry {
                id: i.to_string(),
                name: format!("设备 {i}"),
                checked: false,
                enabled: i != 0,
                command: i,
            })
            .collect();
        let menu = Window::open(
            owner,
            &rows,
            RECT {
                left: 300,
                top: 200,
                right: 330,
                bottom: 230,
            },
        )
        .unwrap();
        let hwnd = menu.hwnd();
        let list = GetDlgItem(Some(hwnd), LIST).unwrap();
        let bar = GetDlgItem(Some(hwnd), scroll::ID).unwrap();
        assert_eq!(GetWindowLongW(list, GWL_STYLE) as u32 & WS_VSCROLL.0, 0);
        let mut bounds = RECT::default();
        GetClientRect(bar, &mut bounds).unwrap();
        SendMessageW(
            bar,
            WM_LBUTTONDOWN,
            Some(WPARAM(1)),
            Some(LPARAM((2 << 16) | 2)),
        );
        SendMessageW(
            bar,
            WM_MOUSEMOVE,
            Some(WPARAM(1)),
            Some(LPARAM(((bounds.bottom + 20) << 16) as isize | 2)),
        );
        SendMessageW(bar, WM_LBUTTONUP, None, None);
        let top = SendMessageW(list, LB_GETTOPINDEX, None, None).0 as usize;
        let row = SendMessageW(list, LB_GETITEMHEIGHT, None, None).0 as i32;
        assert!(top > 0 && (top as i32 + bounds.bottom / row) >= 30);
        ValidateRect(Some(bar), None).ok().unwrap();
        for _ in 0..8 {
            menu.update(&rows, true, true, false);
        }
        assert!(!GetUpdateRect(bar, None, false).as_bool());
        SendMessageW(list, WM_KEYDOWN, Some(WPARAM(0x24)), None); // Home
        SendMessageW(list, WM_KEYDOWN, Some(WPARAM(0x20)), None); // Space, disabled first row
        assert!(menu.event().is_none());
        assert_eq!(SendMessageW(list, LB_GETSEL, Some(WPARAM(0)), None).0, 0);
        SendMessageW(list, WM_KEYDOWN, Some(WPARAM(0x23)), None); // End
        SendMessageW(list, WM_KEYDOWN, Some(WPARAM(0x20)), None);
        assert!(matches!(menu.event(), Some(Event::Toggle(id, true)) if id == "29"));
        drop(menu);
        DestroyWindow(owner).unwrap();
    }
}
