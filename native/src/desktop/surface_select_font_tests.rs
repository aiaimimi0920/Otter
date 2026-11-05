//! 真实 GDI 字体句柄：DPI 往返、重复更新与 HWND 生命周期，不读取桌面画面。
use super::*;
unsafe fn descriptor(font: HGDIOBJ) -> LOGFONTW {
    let mut value = LOGFONTW::default();
    assert_ne!(
        GetObjectW(
            font,
            std::mem::size_of::<LOGFONTW>() as i32,
            Some((&mut value as *mut LOGFONTW).cast())
        ),
        0
    );
    value
}
unsafe fn current(hwnd: HWND) -> HGDIOBJ {
    HGDIOBJ(SendMessageW(hwnd, WM_GETFONT, None, None).0 as *mut _)
}
unsafe fn valid(font: HGDIOBJ) -> bool {
    let mut value = LOGFONTW::default();
    GetObjectW(
        font,
        std::mem::size_of::<LOGFONTW>() as i32,
        Some((&mut value as *mut LOGFONTW).cast()),
    ) != 0
}
#[test]
fn surface_select_font_scales_roundtrips_and_reuses_without_changing_selection() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let parent = parent();
    let control = Control::new(parent.dialog_owner().unwrap(), rect(), options()).unwrap();
    let hwnd = control.hwnd().unwrap();
    unsafe {
        let stock = descriptor(GetStockObject(DEFAULT_GUI_FONT));
        for dpi in [96, 144, 192, 288, 96] {
            control.set_dpi(dpi).unwrap();
            let font = current(hwnd);
            let actual = descriptor(font);
            assert_eq!(actual.lfHeight, stock.lfHeight * dpi as i32 / 96);
            assert_eq!(actual.lfFaceName, stock.lfFaceName);
            assert_eq!(control.selected_index().unwrap(), Some(1));
            control.set_dpi(dpi).unwrap();
            assert_eq!(current(hwnd), font, "unchanged DPI must reuse font");
        }
        let before = current(hwnd);
        assert!(control.set_dpi(0).is_err());
        assert!(control.set_dpi(u32::MAX).is_err());
        assert_eq!(current(hwnd), before);
        drop(control);
        assert!(!valid(before));
        assert!(valid(GetStockObject(DEFAULT_GUI_FONT)));
    }
}
#[test]
fn surface_select_font_survives_revoked_rust_owner_until_hwnd_destruction() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let parent = parent();
    let control = Control::new(parent.dialog_owner().unwrap(), rect(), options()).unwrap();
    control.set_dpi(192).unwrap();
    let hwnd = control.hwnd().unwrap();
    unsafe {
        let font = current(hwnd);
        RemovePropW(hwnd, OWNER).unwrap();
        drop(control);
        assert!(IsWindow(Some(hwnd)).as_bool());
        assert!(valid(font), "live HWND still owns font");
        parent.close().unwrap();
        assert!(!valid(font));
    }
}

#[test]
fn surface_select_font_style_replaces_weight_and_rejects_invalid_without_mutation() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let parent = parent();
    let control = Control::new(parent.dialog_owner().unwrap(), rect(), options()).unwrap();
    let hwnd = control.hwnd().unwrap();
    let mut style = crate::surface_presentation::ControlFont {
        family: "Segoe UI".into(),
        size: 20.,
        weight: 400,
    };
    control.set_font(144, &style).unwrap();
    unsafe {
        let old = current(hwnd);
        style.weight = 700;
        control.set_font(144, &style).unwrap();
        assert!(!valid(old), "replaced font must be released");
        let font = current(hwnd);
        let value = descriptor(font);
        assert_eq!(value.lfHeight, -30);
        assert_eq!(value.lfWeight, 700);
        for (family, size, weight) in [
            ("Segoe UI".into(), f32::NAN, 400),
            ("x".repeat(32), 20., 400),
            ("bad\0font".into(), 20., 400),
            ("Segoe UI".into(), 20., 99),
        ] {
            let invalid = crate::surface_presentation::ControlFont {
                family,
                size,
                weight,
            };
            assert!(control.set_font(144, &invalid).is_err());
            assert_eq!(current(hwnd), font);
            assert!(valid(font));
        }
    }
}

#[test]
fn surface_select_owner_draw_rows_track_font_and_unchanged_sync_keeps_popup() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let parent = parent();
    let control = Control::new(parent.dialog_owner().unwrap(), rect(), options()).unwrap();
    let hwnd = control.hwnd().unwrap();
    parent.show().unwrap();
    control.show().unwrap();
    let mut previous = 0;
    for size in [16., 32., 48.] {
        let style = crate::surface_presentation::ControlFont {
            family: "Segoe UI".into(),
            size,
            weight: 400,
        };
        control.set_font(96, &style).unwrap();
        unsafe {
            let row = SendMessageW(hwnd, CB_GETITEMHEIGHT, Some(WPARAM(0)), None).0;
            let field = SendMessageW(hwnd, CB_GETITEMHEIGHT, Some(WPARAM(usize::MAX)), None).0;
            assert!(
                row > previous && row >= size as isize,
                "rows must fit the actual font"
            );
            assert_eq!(row, field);
            previous = row;
            SendMessageW(hwnd, CB_SHOWDROPDOWN, Some(WPARAM(1)), None);
            assert_ne!(SendMessageW(hwnd, CB_GETDROPPEDSTATE, None, None).0, 0);
            control.set_font(96, &style).unwrap();
            assert_ne!(
                SendMessageW(hwnd, CB_GETDROPPEDSTATE, None, None).0,
                0,
                "unchanged font and item height must preserve open popup"
            );
            control.close_popup().unwrap();
            assert_eq!(control.selected_index().unwrap(), Some(1));
        }
    }
}
