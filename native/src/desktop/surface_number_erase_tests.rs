//! 真实 EDIT 删除行为与撤销；不发送 WM_CUT，不接触用户剪贴板。
use super::*;
// 仅改变当前测试线程的键盘快照；不注入桌面按键，退出时恢复原状态。
struct Keys([u8; 256]);
impl Keys {
    fn set(modifier: Option<usize>) -> Self {
        let mut original = [0; 256];
        unsafe { GetKeyboardState(&mut original).unwrap() };
        let mut keys = original;
        for key in [
            VK_CONTROL,
            VK_MENU,
            VK_SHIFT,
            VK_LCONTROL,
            VK_RCONTROL,
            VK_LMENU,
            VK_RMENU,
            VK_LSHIFT,
            VK_RSHIFT,
        ] {
            keys[key.0 as usize] = 0;
        }
        if let Some(key) = modifier {
            keys[key] = 0x80;
        }
        unsafe { SetKeyboardState(&keys).unwrap() };
        Self(original)
    }
}
impl Drop for Keys {
    fn drop(&mut self) {
        unsafe {
            let _ = SetKeyboardState(&self.0);
        }
    }
}
fn select(edit: HWND, start: usize, end: isize) {
    unsafe {
        SendMessageW(edit, EM_SETSEL, Some(WPARAM(start)), Some(LPARAM(end)));
    }
}
#[test]
fn surface_number_erase_native_repeat_and_selection_contract() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let plain = super::super::mount();
    let edit = child(&plain);
    for (message, key, start, end, expected) in [
        (WM_KEYDOWN, VK_DELETE.0 as usize, 1, 1, "1+2"),
        (WM_CHAR, VK_BACK.0 as usize, 3, 3, "12"),
        (WM_KEYDOWN, VK_DELETE.0 as usize, 1, 3, "12"),
        (WM_CHAR, VK_BACK.0 as usize, 1, 3, "2"),
    ] {
        type_text(edit, "1e+2");
        select(edit, start, end);
        let _keys = Keys::set(None);
        unsafe {
            SendMessageW(edit, message, Some(WPARAM(key)), Some(LPARAM(2)));
        }
        assert_eq!(
            text(edit),
            expected,
            "message={message} selection={start}..{end}"
        );
    }
}
#[test]
fn surface_number_erase_rejection_retains_undo_for_delete_backspace_and_clear() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for (message, key, start, end) in [
        (WM_KEYDOWN, VK_DELETE.0 as usize, 1, 1),
        (WM_CHAR, VK_BACK.0 as usize, 2, 2),
        (WM_CLEAR, 0, 1, 2),
    ] {
        let window = mount(serde_json::json!("1"));
        let edit = child(&window);
        type_text(edit, "1e+2");
        select(edit, start, end);
        let _keys = Keys::set(None);
        let undo_before = unsafe { SendMessageW(edit, EM_CANUNDO, None, None) }.0;
        let modifiers = unsafe {
            [
                GetKeyState(VK_CONTROL.0 as i32),
                GetKeyState(VK_MENU.0 as i32),
                GetKeyState(VK_SHIFT.0 as i32),
            ]
        };
        unsafe {
            SendMessageW(edit, message, Some(WPARAM(key)), Some(LPARAM(1)));
        }
        assert_eq!(text(edit), "1e+2");
        assert_ne!(
            unsafe { SendMessageW(edit, EM_CANUNDO, None, None) }.0,
            0,
            "message={message} undo_before={undo_before} modifiers={modifiers:?}"
        );
        unsafe {
            SendMessageW(edit, WM_UNDO, None, None);
        }
        assert_eq!(text(edit), "1");
        assert!(window.take_error().is_none());
    }
}
#[test]
fn surface_number_erase_thread_modifiers_keep_recovery_separate_from_preflight() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for modifier in [None, Some(VK_CONTROL.0 as usize), Some(VK_MENU.0 as usize)] {
        let window = mount(serde_json::json!("1"));
        let edit = child(&window);
        type_text(edit, "1e+2");
        select(edit, 2, 2);
        let _keys = Keys::set(modifier);
        let before = unsafe { SendMessageW(edit, EM_CANUNDO, None, None) }.0;
        unsafe {
            SendMessageW(
                edit,
                WM_CHAR,
                Some(WPARAM(VK_BACK.0 as usize)),
                Some(LPARAM(1)),
            );
        }
        let after = unsafe { SendMessageW(edit, EM_CANUNDO, None, None) }.0;
        eprintln!(
            "erase-modifier={modifier:?} undo={before}->{after} text={}",
            text(edit)
        );
        assert_eq!(text(edit), "1e+2");
        assert_ne!(before, 0);
        if modifier.is_none() {
            assert_ne!(after, 0);
        }
        assert!(window.take_error().is_none());
    }
}
#[test]
fn surface_number_erase_valid_repeat_and_clear_keep_native_edits() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount(serde_json::json!("1e+2"));
    let edit = child(&window);
    select(edit, 3, 3);
    let _keys = Keys::set(None);
    unsafe {
        SendMessageW(
            edit,
            WM_CHAR,
            Some(WPARAM(VK_BACK.0 as usize)),
            Some(LPARAM(2)),
        );
    }
    assert_eq!(text(edit), "12");
    let (_, wire) = event(&window);
    assert_eq!(wire["payload"]["value"], 12.);
    select(edit, 0, -1);
    unsafe {
        SendMessageW(edit, WM_CLEAR, None, None);
    }
    assert_eq!(text(edit), "");
    let (_, wire) = event(&window);
    assert!(wire["payload"]["value"].is_null());
    assert!(window.take_error().is_none());
}
