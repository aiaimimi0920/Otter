//! 真实 textarea 的格式矩形、裁剪、CTLCOLOR 与像素；不读取用户屏幕。
use super::*;
fn scene(padding: Option<u32>) -> Scene {
    let mut value = serde_json::json!({"id":"field","type":"textarea","props":{"value":""},"layout":{"width":180,"height":80},
        "style":{"color":"#123456","background":"#abcdef","borderWidth":3,"borderColor":"red","borderRadius":12}});
    if let Some(padding) = padding {
        value["layout"]["padding"] = padding.into();
    }
    Scene::parse(value.to_string().as_bytes()).unwrap()
}
unsafe fn color_at(hwnd: HWND) -> COLORREF {
    let dc = CreateCompatibleDC(None);
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: 256,
            biHeight: -128,
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
    SendMessageW(
        hwnd,
        WM_PRINTCLIENT,
        Some(WPARAM(dc.0 as usize)),
        Some(LPARAM((PRF_CLIENT | PRF_ERASEBKGND) as isize)),
    );
    let value = GetPixel(dc, 30, 30);
    SelectObject(dc, old);
    let _ = DeleteObject(bitmap.into());
    assert!(DeleteDC(dc).as_bool());
    value
}
#[test]
fn surface_textarea_box_preserves_border_padding_round_clip_and_declared_colors() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for padding in [None, Some(0), Some(12)] {
        let window = SurfaceWindow::new(scene(padding), 240, 160).unwrap();
        window.bind_edits(&binding()).unwrap();
        window.set_input_policy(true, true).unwrap();
        let edit = child(&window);
        unsafe {
            let scale = windows::Win32::UI::HiDpi::GetDpiForWindow(window.hwnd) as f32 / 96.;
            let mut bounds = RECT::default();
            GetWindowRect(edit, &mut bounds).unwrap();
            let mut origin = POINT {
                x: bounds.left,
                y: bounds.top,
            };
            ScreenToClient(window.hwnd, &mut origin).ok().unwrap();
            assert_eq!(origin.x, (3. * scale).ceil() as i32);
            assert_eq!(origin.y, (3. * scale).ceil() as i32);
            let mut format = RECT::default();
            SendMessageW(
                edit,
                EM_GETRECT,
                None,
                Some(LPARAM((&mut format as *mut RECT) as isize)),
            );
            assert_eq!(
                format.left,
                (padding.unwrap_or(9) as f32 * scale).round() as i32
            );
            assert_eq!(
                format.top,
                (padding.unwrap_or(6) as f32 * scale).round() as i32
            );
            let region = CreateRectRgn(0, 0, 0, 0);
            assert_ne!(GetWindowRgn(edit, region), RGN_ERROR);
            assert!(!PtInRegion(region, 0, 0).as_bool());
            assert!(PtInRegion(region, 20, 20).as_bool());
            let _ = DeleteObject(region.into());
            if crate::desktop::surface_control_colors::enabled() {
                let dc = CreateCompatibleDC(None);
                for message in [WM_CTLCOLOREDIT, WM_CTLCOLORSTATIC] {
                    assert_ne!(
                        SendMessageW(
                            window.hwnd,
                            message,
                            Some(WPARAM(dc.0 as usize)),
                            Some(LPARAM(edit.0 as isize))
                        )
                        .0,
                        0
                    );
                    assert_eq!(GetTextColor(dc), COLORREF(0x563412));
                    assert_eq!(GetBkColor(dc), COLORREF(0xefcdab));
                }
                assert!(DeleteDC(dc).as_bool());
                assert_eq!(color_at(edit), COLORREF(0xefcdab));
            }
        }
        assert!(window.take_error().is_none());
        window.close().unwrap();
    }
}
#[test]
fn surface_textarea_box_percentage_padding_uses_parent_width_not_field_width() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene=Scene::parse(br#"{"id":"root","type":"column","layout":{"width":300,"height":140},"children":[{"id":"field","type":"textarea","layout":{"width":180,"height":120,"padding":"10%"},"style":{"borderWidth":0,"borderRadius":"20%"}}]}"#).unwrap();
    let window = SurfaceWindow::new(scene, 400, 200).unwrap();
    window.bind_edits(&binding()).unwrap();
    let edit = child(&window);
    unsafe {
        let scale = windows::Win32::UI::HiDpi::GetDpiForWindow(window.hwnd) as f32 / 96.;
        let mut rect = RECT::default();
        SendMessageW(
            edit,
            EM_GETRECT,
            None,
            Some(LPARAM((&mut rect as *mut RECT) as isize)),
        );
        assert_eq!(rect.left, (30. * scale).round() as i32);
        assert_eq!(rect.top, (30. * scale).round() as i32);
    }
    assert!(window.take_error().is_none());
}
#[test]
fn surface_textarea_padding_four_edges_update_format_without_recreating_hwnd_or_draft() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = |updated| {
        Scene::parse(
            serde_json::json!({"id":"root","type":"column","layout":{"width":if updated {300} else {200},"height":120},
                "children":[{"id":"field","type":"textarea","props":{"value":"source"},
                "events":{"input":"typing"},"style":{"borderWidth":3},"layout":{"width":180,"height":100,
                "paddingLeft":"10%","paddingTop":"2%","paddingRight":"5%","paddingBottom":"3%"}}]})
            .to_string()
            .as_bytes(),
        )
        .unwrap()
    };
    let window = SurfaceWindow::new(scene(false), 400, 160).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    let edit = child(&window);
    let check = |updated| unsafe {
        let scale = windows::Win32::UI::HiDpi::GetDpiForWindow(window.hwnd) as f32 / 96.;
        let mut client = RECT::default();
        GetClientRect(edit, &mut client).unwrap();
        let mut format = RECT::default();
        SendMessageW(
            edit,
            EM_GETRECT,
            None,
            Some(LPARAM((&mut format as *mut RECT) as isize)),
        );
        let [left, top, right, bottom] = if updated {
            [30., 6., 15., 9.]
        } else {
            [20., 4., 10., 6.]
        };
        assert_eq!(
            format,
            RECT {
                left: (left * scale).round() as i32,
                top: (top * scale).round() as i32,
                right: client.right - (right * scale).round() as i32,
                bottom: client.bottom - (bottom * scale).round() as i32
            }
        );
    };
    check(false);
    type_text(edit, "保留 draft");
    window.replace(scene(true)).unwrap();
    assert_eq!(child(&window), edit);
    assert_eq!(text(edit), "保留 draft");
    check(true);
    assert_eq!(window.prepare_edit(&binding(), 1).unwrap().len(), 1);
    assert!(window.take_error().is_none());
    window.close().unwrap();
    assert!(!window.is_alive());
}
