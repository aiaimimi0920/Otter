//! 真实 HWND 的双向 Tab；只改测试线程键盘状态并恢复，不注入全局输入。
use super::*;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetFocus, GetKeyboardState, SetFocus, SetKeyboardState, VK_SHIFT, VK_TAB,
};
struct Keyboard([u8; 256]);
impl Drop for Keyboard {
    fn drop(&mut self) {
        unsafe {
            let _ = SetKeyboardState(&self.0);
        }
    }
}
fn tab(hwnd: HWND, reverse: bool) {
    unsafe {
        let mut state = [0; 256];
        GetKeyboardState(&mut state).unwrap();
        let _restore = Keyboard(state);
        state = [0; 256];
        state[VK_SHIFT.0 as usize] = if reverse { 0x80 } else { 0 };
        SetKeyboardState(&state).unwrap();
        SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(VK_TAB.0 as usize)), None);
        SendMessageW(hwnd, WM_KEYUP, Some(WPARAM(VK_TAB.0 as usize)), None);
    }
}
#[test]
fn surface_select_navigation_tabs_across_edit_select_and_button_and_closes_popup() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"input","type":"input"},{"id":"select","type":"select","props":{"value":"a","options":[{"value":"a","label":"A"},{"value":"b","label":"B"}]}},{"id":"button","type":"button","props":{"label":"Run"}}]}"#).unwrap();
    let window = SurfaceWindow::new(scene, 300, 240).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    window.show().unwrap();
    let parent = window.dialog_owner().unwrap();
    unsafe {
        let edit = FindWindowExW(Some(parent), None, w!("EDIT"), None).unwrap();
        let select = FindWindowExW(Some(parent), None, w!("COMBOBOX"), None).unwrap();
        let _ = SetFocus(Some(parent));
        tab(parent, false);
        assert_eq!(GetFocus(), edit);
        tab(edit, false);
        assert_eq!(GetFocus(), select);
        SendMessageW(
            select,
            WM_KEYDOWN,
            Some(WPARAM(VK_TAB.0 as usize)),
            Some(LPARAM(1 << 30)),
        );
        SendMessageW(select, WM_CHAR, Some(WPARAM(VK_TAB.0 as usize)), None);
        assert_eq!(GetFocus(), select);
        SendMessageW(select, CB_SHOWDROPDOWN, Some(WPARAM(1)), None);
        assert_ne!(SendMessageW(select, CB_GETDROPPEDSTATE, None, None).0, 0);
        tab(select, false);
        assert_eq!(GetFocus(), parent); // 自绘 Button 由父 HWND 承载。
        assert_eq!(SendMessageW(select, CB_GETDROPPEDSTATE, None, None).0, 0);
        tab(parent, true);
        assert_eq!(GetFocus(), select);
        tab(select, true);
        assert_eq!(GetFocus(), edit);
    }
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
#[test]
fn surface_select_navigation_skips_disabled_and_hidden_selects() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"input","type":"input"},{"id":"disabled","type":"select","props":{"disabled":true}},{"id":"hidden","type":"select","props":{"visible":false}},{"id":"button","type":"button","props":{"label":"Run"}}]}"#).unwrap();
    let window = SurfaceWindow::new(scene, 300, 240).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    window.show().unwrap();
    let parent = window.dialog_owner().unwrap();
    unsafe {
        let edit = FindWindowExW(Some(parent), None, w!("EDIT"), None).unwrap();
        let _ = SetFocus(Some(parent));
        tab(parent, false);
        assert_eq!(GetFocus(), edit);
        tab(edit, false);
        assert_eq!(GetFocus(), parent);
        tab(parent, true);
        assert_eq!(GetFocus(), edit);
    }
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
