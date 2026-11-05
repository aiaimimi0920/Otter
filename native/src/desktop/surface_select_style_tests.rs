//! 测量层继承样式到真实 COMBOBOX；不在控件重复计算相对单位。
use super::*;
fn styled(size: &str, weight: u16) -> Scene {
    Scene::parse(&serde_json::to_vec(&serde_json::json!({
        "id":"root", "type":"column", "style":{"fontSize":20,"fontWeight":weight},
        "children":[{"id":"s","type":"select","style":{"fontSize":size},
            "props":{"value":"a","options":[{"value":"a","label":"中文"},{"value":"b","label":"Second"}]},
            "events":{"change":"choose"}}]
    })).unwrap()).unwrap()
}
fn font(window: &SurfaceWindow) -> (HWND, HGDIOBJ, LOGFONTW) {
    unsafe {
        let hwnd = FindWindowExW(
            Some(window.dialog_owner().unwrap()),
            None,
            w!("COMBOBOX"),
            None,
        )
        .unwrap();
        let handle = HGDIOBJ(SendMessageW(hwnd, WM_GETFONT, None, None).0 as *mut _);
        let mut value = LOGFONTW::default();
        assert_ne!(
            GetObjectW(
                handle,
                std::mem::size_of::<LOGFONTW>() as i32,
                Some((&mut value as *mut LOGFONTW).cast())
            ),
            0
        );
        (hwnd, handle, value)
    }
}
#[test]
fn surface_select_style_inherits_relative_size_weight_and_replaces_without_event() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = SurfaceWindow::new(styled("150%", 700), 300, 200).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    window.show().unwrap();
    let dpi = unsafe { windows::Win32::UI::HiDpi::GetDpiForWindow(window.dialog_owner().unwrap()) };
    for (size, weight, expected) in [("150%", 700, 30.), ("150%", 400, 30.), ("2em", 500, 40.)] {
        window.replace(styled(size, weight)).unwrap();
        let (hwnd, handle, value) = font(&window);
        assert_eq!(
            value.lfHeight,
            -((expected * dpi as f32 / 96.).round() as i32)
        );
        assert_eq!(value.lfWeight, weight as i32);
        let length = value.lfFaceName.iter().position(|c| *c == 0).unwrap();
        assert_eq!(
            String::from_utf16(&value.lfFaceName[..length]).unwrap(),
            "Segoe UI"
        );
        assert_eq!(index(hwnd), 0);
        assert!(!window.has_pending_edit());
        window.replace(styled(size, weight)).unwrap();
        assert_eq!(
            font(&window).1,
            handle,
            "identical resolved font must reuse GDI resource"
        );
        assert!(window.take_error().is_none());
    }
}
#[test]
fn font_family_select_native_descriptor_inherits_order_and_reuses_selected_font() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = |family| {
        Scene::parse(&serde_json::to_vec(&serde_json::json!({
        "id":"root","type":"column","style":{"fontFamily":family},
        "children":[{"id":"s","type":"select","props":{"value":"a","options":[{"value":"a","label":"First"}]},"events":{"change":"choose"}}]
    })).unwrap()).unwrap()
    };
    let window = SurfaceWindow::new(scene("MissingOtterFamily,monospace"), 300, 200).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    for (source, expected) in [
        ("MissingOtterFamily,monospace", "Consolas"),
        ("Segoe UI,Consolas", "Segoe UI"),
    ] {
        window.replace(scene(source)).unwrap();
        let (hwnd, handle, value) = font(&window);
        let end = value.lfFaceName.iter().position(|u| *u == 0).unwrap();
        assert_eq!(
            String::from_utf16(&value.lfFaceName[..end]).unwrap(),
            expected
        );
        assert_eq!(index(hwnd), 0);
        assert!(!window.has_pending_edit());
        window.replace(scene(source)).unwrap();
        assert_eq!(font(&window).1, handle);
    }
    window.close().unwrap();
}
