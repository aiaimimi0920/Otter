//! 联合夹具的按钮回执必须区别字段commit，并隔离当前线程的环境修饰键。
use super::*;
use windows::Win32::UI::{
    Input::KeyboardAndMouse::{GetKeyboardState, VK_CONTROL, VK_LCONTROL, VK_MENU},
    Shell::*,
};
fn form() -> Value {
    json!({"id":"root","type":"column","layout":{"width":64,"height":64},"children":[
        {"id":"field","type":"number","layout":{"width":64,"height":32,"minHeight":32},
            "props":{"value":"0"},"events":{"input":"preview","change":"save"}},
        {"id":"send","type":"button","layout":{"width":64,"height":32,"minHeight":32},
            "props":{"includeSurfaceValues":true},"events":{"click":"go"}}]})
}
fn edit(w: &Window) {
    w.test_art_text([8, 8], "4", false, false);
}
unsafe extern "system" fn reject_enter(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    _: usize,
) -> LRESULT {
    if matches!(msg, WM_KEYDOWN | WM_KEYUP) && wp.0 == 13 {
        return LRESULT(0);
    }
    DefSubclassProc(hwnd, msg, wp, lp)
}
#[test]
fn tab_commit_is_not_misreported_as_an_accepted_enter_button() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (f, w) = setup(form());
    edit(&w);
    let before = f.shared.lock().actions.test_offered;
    unsafe {
        assert!(SetWindowSubclass(w.hwnd, Some(reject_enter), 0x524a454e, 0).as_bool());
    }
    assert!(!w.test_art_text_keyboard_submit());
    assert!(f.shared.lock().actions.test_offered > before);
    let events = events(&f);
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["event"], "input");
    assert_eq!(events[1]["event"], "change");
    assert_eq!(focus(&w), "send");
}
#[test]
fn joint_unmodified_keys_restore_ambient_state_and_accept_exactly_one_button() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let foreground = unsafe { GetForegroundWindow() };
    let (f, w) = setup(form());
    edit(&w);
    joint_access::with_modifiers(&[VK_CONTROL, VK_LCONTROL, VK_MENU], || {
        let mut before = [0; 256];
        unsafe { GetKeyboardState(&mut before).unwrap() };
        assert!(w.test_art_text_keyboard_submit());
        let mut after = [0; 256];
        unsafe { GetKeyboardState(&mut after).unwrap() };
        assert_eq!(before, after);
    });
    let events = events(&f);
    assert_eq!(events.len(), 3);
    assert_eq!(events[2]["event"], "click");
    assert_eq!(events[2]["payload"]["surfaceValues"]["field"], "4");
    assert_eq!(unsafe { GetForegroundWindow() }, foreground);
}
#[test]
fn thread_key_isolation_restores_on_unwind_and_product_still_rejects_control_enter() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (f, w) = setup(form());
    navigate(w.hwnd, &w.state, false);
    navigate(w.hwnd, &w.state, false);
    joint_access::with_modifiers(&[VK_CONTROL, VK_LCONTROL], || {
        let mut before = [0; 256];
        unsafe {
            GetKeyboardState(&mut before).unwrap();
            key(w.hwnd, WM_KEYDOWN, 13, false);
            key(w.hwnd, WM_KEYUP, 13, false);
        }
        assert!(events(&f).is_empty());
        let result = std::panic::catch_unwind(|| {
            joint_access::with_modifiers(&[], || {
                assert!(!actions::modifiers().ctrl_key);
                panic!("isolated keyboard unwind probe");
            });
        });
        assert!(result.is_err());
        let mut after = [0; 256];
        unsafe { GetKeyboardState(&mut after).unwrap() };
        assert_eq!(before, after);
    });
}
