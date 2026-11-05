//! 使用真实进程内 HMENU；不弹窗、不读取系统剪贴板。
use super::*;
use windows::core::{w, HSTRING};

struct Menu(HMENU);
impl Menu {
    fn new() -> Self {
        Self(unsafe { CreatePopupMenu().unwrap() })
    }
}
impl Drop for Menu {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyMenu(self.0);
        }
    }
}

#[test]
fn preserves_nested_commands_unicode_shortcuts_and_states() {
    let root = Menu::new();
    let child = Menu::new();
    unsafe {
        AppendMenuW(
            root.0,
            MF_STRING | MF_CHECKED | MF_GRAYED,
            0x3210,
            w!("选中(&C)\tCtrl+C"),
        )
        .unwrap();
        AppendMenuW(root.0, MF_SEPARATOR, 0, None).unwrap();
        AppendMenuW(child.0, MF_STRING, 0x4321, w!("子命令")).unwrap();
        AppendMenuW(root.0, MF_POPUP, child.0 .0 as usize, w!("子菜单")).unwrap();
        std::mem::forget(child); // 成功追加后由父菜单递归销毁。
    }
    let items = snapshot(root.0).unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].label, "选中(&C)\tCtrl+C");
    assert_eq!(items[0].command, 0x3210);
    assert!(items[0].checked);
    assert!(!items[0].enabled);
    assert!(items[1].separator);
    let child = &items[2].children.as_ref().unwrap()[0];
    assert_eq!(child.command, 0x4321);
    assert_eq!(child.label, "子命令");
    drop(root);
    assert_eq!(child.label, "子命令"); // 快照不依赖句柄生命周期。
}

#[test]
fn preserves_empty_submenu_and_radio_default_flags() {
    let root = Menu::new();
    let child = Menu::new();
    unsafe {
        AppendMenuW(root.0, MF_POPUP, child.0 .0 as usize, w!("空菜单")).unwrap();
        std::mem::forget(child);
        AppendMenuW(root.0, MF_STRING, 42, w!("默认项")).unwrap();
        let info = MENUITEMINFOW {
            cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
            fMask: MIIM_FTYPE | MIIM_STATE,
            fType: MFT_RADIOCHECK,
            fState: MFS_DEFAULT | MFS_CHECKED,
            ..Default::default()
        };
        SetMenuItemInfoW(root.0, 1, true, &info).unwrap();
    }
    let items = snapshot(root.0).unwrap();
    assert!(items[0].children.as_ref().unwrap().is_empty());
    assert!(items[1].radio && items[1].default && items[1].checked && items[1].enabled);
}

#[test]
fn rejects_unsupported_owner_draw_and_excessive_labels_without_mutation() {
    let root = Menu::new();
    unsafe {
        AppendMenuW(root.0, MF_OWNERDRAW, 7, None).unwrap();
    }
    assert!(snapshot(root.0).is_err());
    assert_eq!(unsafe { GetMenuItemCount(Some(root.0)) }, 1);
    let long = Menu::new();
    unsafe {
        AppendMenuW(long.0, MF_STRING, 9, &HSTRING::from("x".repeat(4097))).unwrap();
    }
    assert!(snapshot(long.0).is_err());
}

#[test]
fn rejects_excessive_tree_depth_and_invalid_handles() {
    assert!(snapshot(HMENU::default()).is_err());
    let root = Menu::new();
    let mut current = root.0;
    for _ in 0..16 {
        let child = Menu::new();
        unsafe {
            AppendMenuW(current, MF_POPUP, child.0 .0 as usize, w!("nested")).unwrap();
        }
        current = child.0;
        std::mem::forget(child);
    }
    assert!(snapshot(root.0).is_err());
}

#[test]
fn rejects_total_item_and_text_budgets() {
    let many = Menu::new();
    for id in 0..1025 {
        unsafe {
            AppendMenuW(many.0, MF_STRING, id, w!("item")).unwrap();
        }
    }
    assert!(snapshot(many.0).is_err());
    let text = Menu::new();
    let label = HSTRING::from("x".repeat(4096));
    for id in 0..33 {
        unsafe {
            AppendMenuW(text.0, MF_STRING, id, &label).unwrap();
        }
    }
    assert!(snapshot(text.0).is_err());
}

#[test]
fn rejects_callback_bitmap_sentinel() {
    let root = Menu::new();
    unsafe {
        AppendMenuW(root.0, MF_STRING, 1, w!("callback")).unwrap();
        let info = MENUITEMINFOW {
            cbSize: std::mem::size_of::<MENUITEMINFOW>() as u32,
            fMask: MIIM_BITMAP,
            hbmpItem: windows::Win32::Graphics::Gdi::HBITMAP(-1isize as *mut _),
            ..Default::default()
        };
        SetMenuItemInfoW(root.0, 0, true, &info).unwrap();
    }
    assert!(snapshot(root.0).is_err());
}
