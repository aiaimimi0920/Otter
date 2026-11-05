//! 真实 EDIT 的 WM_PRINTCLIENT 像素与值隔离，不读取屏幕或用户剪贴板。
use super::*;
use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
#[path = "surface_edit_alignment_tests.rs"]
mod alignment;
fn pixels(hwnd: HWND) -> Vec<u8> {
    unsafe {
        let dc = CreateCompatibleDC(None);
        assert!(!dc.is_invalid());
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: 200,
                biHeight: -100,
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
        std::ptr::write_bytes(bits, 0, 200 * 100 * 4);
        SendMessageW(
            hwnd,
            WM_PRINTCLIENT,
            Some(WPARAM(dc.0 as usize)),
            Some(LPARAM((PRF_CLIENT | PRF_ERASEBKGND) as isize)),
        );
        assert!(GdiFlush().as_bool());
        let mut result = std::slice::from_raw_parts(bits.cast::<u8>(), 200 * 100 * 4).to_vec();
        // 32-bit BI_RGB 的高字节未定义，并非 alpha；v6 主题绘制会改变它。
        // 归一化保留全部 B/G/R 像素精确比较，不把未定义填充值当成显示差异。
        for pixel in result.chunks_exact_mut(4) {
            pixel[3] = 0;
        }
        SelectObject(dc, old);
        let _ = DeleteObject(bitmap.into());
        assert!(DeleteDC(dc).as_bool());
        result
    }
}
#[test]
fn surface_edit_placeholder_pixels_never_become_text_and_return_after_clear() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let parent = parent();
    let edit = Control::new(parent.dialog_owner().unwrap(), rect(), false, 100).unwrap();
    let hwnd = edit.hwnd().unwrap();
    let empty = pixels(hwnd);
    edit.install_placeholder("输入数字 & name").unwrap();
    let hint = pixels(hwnd);
    assert_ne!(hint, empty);
    assert_eq!(edit.text().unwrap(), "");
    edit.focus().unwrap();
    assert_eq!(pixels(hwnd), hint);
    edit.set_text("12.5").unwrap();
    let filled = pixels(hwnd);
    assert_ne!(filled, hint);
    let plain = Control::new(parent.dialog_owner().unwrap(), rect(), false, 100).unwrap();
    plain.set_text("12.5").unwrap();
    assert_eq!(pixels(plain.hwnd().unwrap()), filled);
    edit.set_text("").unwrap();
    assert_eq!(pixels(hwnd), hint);
    assert_eq!(edit.text().unwrap(), "");
}
#[test]
fn surface_edit_placeholder_multiline_and_ime_visibility() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let parent = parent();
    let edit = Control::new(parent.dialog_owner().unwrap(), rect(), true, 100).unwrap();
    let hwnd = edit.hwnd().unwrap();
    let empty = pixels(hwnd);
    edit.install_placeholder("第一行\r\n第二行").unwrap();
    let hint = pixels(hwnd);
    assert_ne!(hint, empty);
    unsafe {
        SendMessageW(hwnd, WM_IME_STARTCOMPOSITION, None, None);
    }
    assert_eq!(pixels(hwnd), empty);
    unsafe {
        SendMessageW(hwnd, WM_IME_ENDCOMPOSITION, None, None);
    }
    assert_eq!(pixels(hwnd), hint);
    unsafe {
        SendMessageW(hwnd, WM_IME_STARTCOMPOSITION, None, None);
        let _ = EnableWindow(hwnd, false);
        let _ = EnableWindow(hwnd, true);
    }
    assert_eq!(pixels(hwnd), hint);
    assert_eq!(edit.text().unwrap(), "");
}
#[test]
fn surface_edit_placeholder_rejects_oversized_nul_and_duplicate_installation() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let parent = parent();
    let edit = Control::new(parent.dialog_owner().unwrap(), rect(), false, 100).unwrap();
    assert!(edit.install_placeholder("bad\0text").is_err());
    assert!(edit
        .install_placeholder(&"x".repeat(MAX_BYTES + 1))
        .is_err());
    edit.install_placeholder("valid").unwrap();
    let before = pixels(edit.hwnd().unwrap());
    assert!(edit.install_placeholder("replace").is_err());
    assert_eq!(pixels(edit.hwnd().unwrap()), before);
    parent.close().unwrap();
    assert!(edit.install_placeholder("closed").is_err());
}
