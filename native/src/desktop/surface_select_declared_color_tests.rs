//! 真实 HWND 像素验证声明覆盖、currentcolor、alpha 和恢复默认，不注入全局输入。
use super::*;
fn current(window: &SurfaceWindow) -> HWND {
    unsafe {
        FindWindowExW(
            Some(window.dialog_owner().unwrap()),
            None,
            w!("COMBOBOX"),
            None,
        )
        .unwrap()
    }
}
fn colored(color: serde_json::Value) -> Scene {
    colored_parent(color, "#ff0000")
}
fn colored_parent(color: serde_json::Value, parent: &str) -> Scene {
    Scene::parse(&serde_json::to_vec(&serde_json::json!({
        "id":"root", "type":"column", "style":{"color":parent},
        "children":[{"id":"s", "type":"select", "style":{"color":color},
            "props":{"value":"a","options":[{"value":"a","label":"First"},{"value":"b","label":"第二🙂"}]},
            "events":{"change":"choose"}}]
    })).unwrap()).unwrap()
}
#[test]
fn surface_select_currentcolor_parent_change_updates_existing_hwnd_without_event() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, _, _) = mounted();
    window
        .replace(colored(serde_json::json!("currentcolor")))
        .unwrap();
    let hwnd = current(&window);
    window.show().unwrap();
    window
        .replace(colored_parent(serde_json::json!("currentcolor"), "#00ff00"))
        .unwrap();
    assert_eq!(
        current(&window),
        hwnd,
        "parent color does not change the field contract"
    );
    unsafe {
        assert!(pixels(hwnd, "select-parent-color.png").contains(&[0, 255, 0]));
    }
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
#[test]
fn surface_select_declared_color_override_currentcolor_invalid_and_reset_reach_pixels() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, _, _) = mounted();
    window.show().unwrap();
    for (value, ink) in [
        (serde_json::json!("#00ff00"), [0, 255, 0]),
        (serde_json::json!("currentcolor"), [0, 0, 255]),
        (serde_json::json!("url(secret)"), [0xef, 0xf8, 0xf7]),
        (serde_json::Value::Null, [0xef, 0xf8, 0xf7]),
    ] {
        window.replace(colored(value)).unwrap();
        // 节点样式属于编辑合同，替换会撤销旧 HWND；必须验证当前真实控件。
        let hwnd = current(&window);
        unsafe {
            let image = pixels(hwnd, "select-declared.png");
            assert!(
                image.contains(&ink),
                "resolved color did not reach the real HWND: {ink:?}"
            );
            SendMessageW(hwnd, CB_SHOWDROPDOWN, Some(WPARAM(1)), None);
            let mut info = COMBOBOXINFO {
                cbSize: std::mem::size_of::<COMBOBOXINFO>() as u32,
                ..Default::default()
            };
            GetComboBoxInfo(hwnd, &mut info).unwrap();
            let list = pixels(info.hwndList, "select-declared-list.png");
            assert!(
                list.contains(&ink),
                "unselected list rows must use the same declaration"
            );
            assert!(
                list.contains(&[0x2b, 0x25, 0x20]),
                "selected row must retain contrasting focus ink"
            );
            SendMessageW(hwnd, CB_SHOWDROPDOWN, Some(WPARAM(0)), None);
        }
        assert_eq!(index(hwnd), 0);
        assert!(!window.has_pending_edit());
        assert!(window.take_error().is_none());
    }
}
#[test]
fn surface_select_declared_alpha_composites_against_actual_normal_and_hover_background() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, _, _) = mounted();
    window.show().unwrap();
    window
        .replace(colored(serde_json::json!("rgba(255,0,0,0.5)")))
        .unwrap();
    let hwnd = current(&window);
    unsafe {
        assert!(pixels(hwnd, "select-alpha.png").contains(&[16, 12, 136]));
        SendMessageW(hwnd, WM_MOUSEMOVE, None, Some(LPARAM(5 | (5 << 16))));
        assert!(pixels(hwnd, "select-alpha-hover.png").contains(&[22, 17, 140]));
    }
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
