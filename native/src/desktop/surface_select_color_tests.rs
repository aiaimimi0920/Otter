//! 只绘制自有 COMBOBOX 列表到内存 DIB，不读取桌面或其他应用像素。
use super::*;
#[path = "surface_select_declared_color_tests.rs"]
mod declared;
#[path = "surface_select_hover_tests.rs"]
mod hover;
use windows::Win32::UI::Controls::{GetComboBoxInfo, COMBOBOXINFO};
unsafe fn pixels(hwnd: HWND, name: &str) -> Vec<[u8; 3]> {
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
    std::ptr::write_bytes(bits, 0, length);
    SendMessageW(
        hwnd,
        WM_PRINTCLIENT,
        Some(WPARAM(dc.0 as usize)),
        Some(LPARAM((PRF_CLIENT | PRF_ERASEBKGND) as isize)),
    );
    assert!(GdiFlush().as_bool());
    let result: Vec<[u8; 3]> = std::slice::from_raw_parts(bits.cast::<u8>(), length)
        .chunks_exact(4)
        .map(|p| [p[0], p[1], p[2]])
        .collect();
    SelectObject(dc, old);
    let _ = DeleteObject(bitmap.into());
    assert!(DeleteDC(dc).as_bool());
    // 显式启用时只导出本测试的内存绘制结果，供视觉核验；默认不写文件。
    if let Some(directory) = std::env::var_os("OTTER_NATIVE_TEST_ARTIFACTS") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        let file = std::fs::File::create(directory.join(name)).unwrap();
        let mut encoder = png::Encoder::new(file, rect.right as u32, rect.bottom as u32);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let rgb: Vec<_> = result.iter().flat_map(|p| [p[2], p[1], p[0]]).collect();
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&rgb)
            .unwrap();
    }
    result
}
#[test]
fn surface_select_popup_unselected_rows_use_neuro_control_background() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, _, hwnd) = mounted();
    window.show().unwrap();
    unsafe {
        SendMessageW(hwnd, CB_SHOWDROPDOWN, Some(WPARAM(1)), None);
        let mut info = COMBOBOXINFO {
            cbSize: std::mem::size_of::<COMBOBOXINFO>() as u32,
            ..Default::default()
        };
        GetComboBoxInfo(hwnd, &mut info).unwrap();
        let pixels = pixels(info.hwndList, "select-popup.png");
        let dark = pixels.iter().filter(|p| **p == [0x20, 0x17, 0x11]).count();
        assert!(
            dark > pixels.len() / 4,
            "unselected list rows not themed: {dark}/{}",
            pixels.len()
        );
        assert!(
            pixels.contains(&[0xef, 0xf8, 0xf7]),
            "unselected text must use Neuro text color"
        );
        let selected = pixels.iter().filter(|p| **p == [0xf1, 0xf5, 0xf3]).count();
        assert!(
            selected > pixels.len() / 4,
            "selected row must use focus surface: {selected}/{}",
            pixels.len()
        );
        assert!(
            pixels.contains(&[0x2b, 0x25, 0x20]),
            "selected text must use focus ink"
        );
        SendMessageW(hwnd, CB_SHOWDROPDOWN, Some(WPARAM(0)), None);
    }
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}

#[test]
fn surface_select_closed_control_uses_readable_neuro_background() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, _, hwnd) = mounted();
    window.show().unwrap();
    unsafe {
        let pixels = pixels(hwnd, "select-closed.png");
        let dark = pixels.iter().filter(|p| **p == [0x20, 0x17, 0x11]).count();
        assert!(
            dark > pixels.len() / 2,
            "closed field not themed: {dark}/{}",
            pixels.len()
        );
        assert!(
            pixels.contains(&[0xef, 0xf8, 0xf7]),
            "closed field text not readable"
        );
    }
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}

#[test]
fn surface_select_closed_focus_and_disabled_states_keep_native_policy() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, _, hwnd) = mounted();
    window.show().unwrap();
    unsafe {
        let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(hwnd));
        assert_eq!(
            windows::Win32::UI::Input::KeyboardAndMouse::GetFocus(),
            hwnd
        );
        let focused = pixels(hwnd, "select-focused.png");
        assert!(
            focused.contains(&[0x38, 0xff, 0xd9]),
            "focus border must use signal yellow"
        );
        window.set_input_policy(false, false).unwrap();
        assert!(!windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(hwnd).as_bool());
        let disabled = pixels(hwnd, "select-disabled.png");
        assert!(!disabled.contains(&[0x38, 0xff, 0xd9]));
        assert!(
            disabled.contains(&[0x9f, 0x9a, 0x92]),
            "disabled text/arrow must remain readable"
        );
        assert_eq!(index(hwnd), 0);
    }
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}

#[test]
fn surface_select_closed_print_restores_callers_dc_state() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, _, hwnd) = mounted();
    unsafe {
        let dc = CreateCompatibleDC(None);
        assert!(!dc.is_invalid());
        SetTextColor(dc, COLORREF(0x010203));
        SetDCBrushColor(dc, COLORREF(0x040506));
        let before = GetCurrentObject(dc, OBJ_FONT);
        SendMessageW(
            hwnd,
            WM_PRINTCLIENT,
            Some(WPARAM(dc.0 as usize)),
            Some(LPARAM(PRF_CLIENT as isize)),
        );
        let after = (
            GetTextColor(dc),
            GetDCBrushColor(dc),
            GetCurrentObject(dc, OBJ_FONT),
        );
        assert!(DeleteDC(dc).as_bool());
        assert_eq!(after, (COLORREF(0x010203), COLORREF(0x040506), before));
    }
    assert!(window.take_error().is_none());
}
