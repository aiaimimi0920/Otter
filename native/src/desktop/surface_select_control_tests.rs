//! 真实 COMBOBOX 选项/选择/销毁；只操作自有隐藏窗口，不注入桌面键盘输入。
use super::*;
#[path = "surface_select_font_tests.rs"]
mod font_tests;
#[path = "surface_select_mount_tests.rs"]
mod mount;
use crate::{desktop::surface_window::SurfaceWindow, surface_scene::Scene};
use windows::Win32::UI::Controls::{GetComboBoxInfo, COMBOBOXINFO};
fn parent() -> SurfaceWindow {
    SurfaceWindow::new(
        Scene::parse(br#"{"id":"root","type":"view"}"#).unwrap(),
        240,
        200,
    )
    .unwrap()
}
fn rect() -> RECT {
    RECT {
        left: 4,
        top: 4,
        right: 204,
        bottom: 164,
    }
}
fn options() -> Options {
    let scene = Scene::parse("{\"id\":\"s\",\"type\":\"select\",\"props\":{\"value\":\"b\",\"options\":[{\"value\":\"a\",\"label\":\"中文🙂\"},{\"value\":\"b\",\"label\":\"Second\"},{\"value\":\"b\",\"label\":\"Duplicate\"}]}}".as_bytes()).unwrap();
    Options::parse(scene.root()).unwrap()
}
#[test]
fn surface_select_control_real_unicode_order_duplicate_and_value_mapping() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let parent = parent();
    let control = Control::new(parent.dialog_owner().unwrap(), rect(), options()).unwrap();
    assert_eq!(control.selected_index().unwrap(), Some(1));
    assert_eq!(control.value().unwrap(), Some("b"));
    let hwnd = control.hwnd().unwrap();
    unsafe {
        assert_eq!(SendMessageW(hwnd, CB_GETCOUNT, None, None).0, 3);
        let mut text = [0_u16; 32];
        let length = SendMessageW(
            hwnd,
            CB_GETLBTEXT,
            Some(WPARAM(0)),
            Some(LPARAM(text.as_mut_ptr() as isize)),
        )
        .0 as usize;
        assert_eq!(String::from_utf16(&text[..length]).unwrap(), "中文🙂");
    }
    control.select(Some(2)).unwrap();
    assert_eq!(control.selected_index().unwrap(), Some(2));
    assert_eq!(control.value().unwrap(), Some("b"));
    assert!(control.select(Some(3)).is_err());
    assert_eq!(control.selected_index().unwrap(), Some(2));
    control.select(None).unwrap();
    assert_eq!(control.value().unwrap(), None);
    control.select(Some(0)).unwrap();
    assert_eq!(control.value().unwrap(), Some("a"));
    unsafe {
        SendMessageW(
            hwnd,
            WM_KEYDOWN,
            Some(WPARAM(
                windows::Win32::UI::Input::KeyboardAndMouse::VK_DOWN.0 as usize,
            )),
            None,
        );
    }
    assert_eq!(control.selected_index().unwrap(), Some(1));
    assert_eq!(control.value().unwrap(), Some("b"));
    drop(control);
    assert!(!unsafe { IsWindow(Some(hwnd)) }.as_bool());
}
#[test]
fn surface_select_control_parent_close_revokes_child_and_list_window() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let parent = parent();
    let control = Control::new(parent.dialog_owner().unwrap(), rect(), options()).unwrap();
    let hwnd = control.hwnd().unwrap();
    let mut info = COMBOBOXINFO {
        cbSize: std::mem::size_of::<COMBOBOXINFO>() as u32,
        ..Default::default()
    };
    unsafe {
        GetComboBoxInfo(hwnd, &mut info).unwrap();
    }
    assert!(!info.hwndList.0.is_null());
    assert!(unsafe { IsWindow(Some(info.hwndList)) }.as_bool());
    parent.show().unwrap();
    control.show().unwrap();
    unsafe {
        SendMessageW(hwnd, CB_SHOWDROPDOWN, Some(WPARAM(1)), None);
        assert_ne!(SendMessageW(hwnd, CB_GETDROPPEDSTATE, None, None).0, 0);
    }
    control.set_enabled(false).unwrap();
    assert_eq!(
        unsafe { SendMessageW(hwnd, CB_GETDROPPEDSTATE, None, None).0 },
        0
    );
    assert!(control.focus().is_err());
    control.set_enabled(true).unwrap();
    control
        .place(RECT {
            left: 8,
            top: 8,
            ..rect()
        })
        .unwrap();
    unsafe {
        SendMessageW(hwnd, CB_SHOWDROPDOWN, Some(WPARAM(1)), None);
        assert_ne!(SendMessageW(hwnd, CB_GETDROPPEDSTATE, None, None).0, 0);
    }
    parent.close().unwrap();
    assert!(!control.is_alive());
    assert!(control.value().is_err());
    assert!(control.select(None).is_err());
    assert!(!unsafe { IsWindow(Some(info.hwndList)) }.as_bool());
}
#[test]
fn surface_select_control_rejects_invalid_parent_bounds_and_handles_empty_list() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let parent = parent();
    assert!(Control::new(HWND::default(), rect(), options()).is_err());
    assert!(Control::new(
        parent.dialog_owner().unwrap(),
        RECT {
            left: i32::MIN,
            right: i32::MAX,
            ..rect()
        },
        options()
    )
    .is_err());
    let scene = Scene::parse(br#"{"id":"s","type":"select"}"#).unwrap();
    let control = Control::new(
        parent.dialog_owner().unwrap(),
        rect(),
        Options::parse(scene.root()).unwrap(),
    )
    .unwrap();
    assert_eq!(control.value().unwrap(), None);
    assert!(control.select(Some(0)).is_err());
}
#[test]
fn surface_select_control_revoked_marker_never_operates_or_destroys_unowned_hwnd() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let parent = parent();
    let control = Control::new(parent.dialog_owner().unwrap(), rect(), options()).unwrap();
    let hwnd = control.hwnd().unwrap();
    unsafe {
        RemovePropW(hwnd, OWNER).unwrap();
    }
    assert!(!control.is_alive());
    assert!(control.value().is_err());
    assert!(control.select(Some(0)).is_err());
    assert!(control.set_enabled(false).is_err());
    drop(control);
    assert!(unsafe { IsWindow(Some(hwnd)) }.as_bool());
    parent.close().unwrap();
    assert!(!unsafe { IsWindow(Some(hwnd)) }.as_bool());
}
