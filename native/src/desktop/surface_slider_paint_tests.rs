//! 自有 HWND 的内存绘制，验证轨道/拇指/禁用色与借用 DC 恢复，不读取桌面。
use super::*;
unsafe fn render(hwnd: HWND, name: &str) -> Vec<[u8; 3]> {
    render_size(hwnd, name, None)
}
unsafe fn render_size(hwnd: HWND, name: &str, size: Option<[i32; 2]>) -> Vec<[u8; 3]> {
    let mut rect = RECT::default();
    GetClientRect(hwnd, &mut rect).unwrap();
    if let Some([width, height]) = size {
        rect.right = width;
        rect.bottom = height;
    }
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
    SetTextColor(dc, COLORREF(0x010203));
    SetDCBrushColor(dc, COLORREF(0x040506));
    let pen = GetCurrentObject(dc, OBJ_PEN);
    SendMessageW(
        hwnd,
        WM_PRINTCLIENT,
        Some(WPARAM(dc.0 as usize)),
        Some(LPARAM(PRF_CLIENT as isize)),
    );
    assert!(GdiFlush().as_bool());
    let restored = (
        GetTextColor(dc),
        GetDCBrushColor(dc),
        GetCurrentObject(dc, OBJ_PEN),
    );
    let image: Vec<[u8; 3]> = std::slice::from_raw_parts(bits.cast::<u8>(), length)
        .chunks_exact(4)
        .map(|p| [p[0], p[1], p[2]])
        .collect();
    SelectObject(dc, old);
    let _ = DeleteObject(bitmap.into());
    assert!(DeleteDC(dc).as_bool());
    assert_eq!(restored, (COLORREF(0x010203), COLORREF(0x040506), pen));
    if let Some(path) = std::env::var_os("OTTER_NATIVE_TEST_ARTIFACTS") {
        std::fs::create_dir_all(&path).unwrap();
        let file = std::fs::File::create(std::path::Path::new(&path).join(name)).unwrap();
        let mut encoder = png::Encoder::new(file, rect.right as u32, rect.bottom as u32);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let rgb: Vec<_> = image.iter().flat_map(|p| [p[2], p[1], p[0]]).collect();
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&rgb)
            .unwrap();
    }
    image
}

#[test]
fn surface_slider_control_tiny_client_never_paints_outside_borrowed_dc_region() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let parent = Parent::new();
    let control = control(&parent);
    control
        .place(RECT {
            left: 0,
            top: 0,
            right: 1,
            bottom: 1,
        })
        .unwrap();
    let image = unsafe {
        render_size(
            control.hwnd().unwrap(),
            "slider-tiny-clip.png",
            Some([200, 30]),
        )
    };
    assert!(
        image[1..].iter().all(|pixel| *pixel == [0, 0, 0]),
        "tiny thumb must stay within client clip"
    );
}
#[test]
fn surface_slider_control_native_pixels_track_value_disabled_and_restore_dc() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let parent = Parent::new();
    let control = control(&parent);
    let hwnd = control.hwnd().unwrap();
    unsafe {
        let first = render(hwnd, "slider-middle.png");
        assert!(first.contains(&[0x20, 0x17, 0x11]));
        assert!(first.contains(&[0x38, 0xff, 0xd9]));
        control.set_value(100.).unwrap();
        let last = render(hwnd, "slider-end.png");
        assert_ne!(first, last);
        control.set_enabled(false).unwrap();
        let disabled = render(hwnd, "slider-disabled.png");
        assert!(disabled.contains(&[0x9f, 0x9a, 0x92]));
        assert!(!disabled.contains(&[0x38, 0xff, 0xd9]));
    }
    assert!(parent.log.events.is_empty());
}
