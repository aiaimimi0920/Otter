//! 真实 DialogBox 消息循环验证禁用 owner 仍接收结束热键；窗口在屏幕外，不读取桌面或注册热键。
use super::*;
use crate::desktop::{global_capture, State, Window};
use windows::{
    core::w,
    Win32::UI::Input::KeyboardAndMouse::{IsWindowEnabled, MOD_CONTROL},
};
const DEADLINE: usize = 0x4f69;
struct Owner {
    state: State,
    observed_disabled: Cell<bool>,
}
unsafe extern "system" fn owner_procedure(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> LRESULT {
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Owner;
    if let Some(owner) = pointer.as_ref() {
        if message == WM_HOTKEY {
            owner
                .observed_disabled
                .set(!IsWindowEnabled(hwnd).as_bool());
            if let Some(result) = global_capture::dispatch(hwnd, &owner.state, message, wp, lp) {
                return result;
            }
        }
    }
    DefWindowProcW(hwnd, message, wp, lp)
}
unsafe extern "system" fn dialog_procedure(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> isize {
    if message == WM_TIMER && wp.0 == DEADLINE {
        let pointer = GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER)) as *const Input;
        if let Some(input) = pointer.as_ref() {
            *input.error.borrow_mut() = Some("Modal finish deadline exceeded".into());
        }
        return procedure(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
    }
    let result = procedure(hwnd, message, wp, lp);
    if message == WM_INITDIALOG {
        let owner = GetWindow(hwnd, GW_OWNER).unwrap();
        // 只有该通知是合成的；owner 禁用、分派、后台拼接、timer 和 EndDialog 均走真实 Win32。
        for _ in 0..2 {
            PostMessageW(
                Some(owner),
                WM_HOTKEY,
                WPARAM(0x4f55),
                LPARAM(((u32::from(b'3') << 16) | MOD_CONTROL.0) as isize),
            )
            .unwrap();
        }
        SetTimer(Some(hwnd), DEADLINE, 10_000, None);
    }
    result
}
#[repr(C, align(4))]
struct Template {
    style: u32,
    extended_style: u32,
    items: u16,
    x: i16,
    y: i16,
    width: i16,
    height: i16,
    menu: u16,
    class: u16,
    title: u16,
}
#[test]
fn real_modal_loop_dispatches_disabled_owners_hotkey_and_returns_composed_image() {
    unsafe {
        let owner = Box::new(Owner {
            state: State::new(None, false).unwrap(),
            observed_disabled: Cell::new(false),
        });
        owner.state.capture_active.set(true);
        owner.state.file_dialog.set(true);
        owner.state.global_capture.set(true);
        owner.state.global_long_capture.set(true);
        owner.state.modals.register(&owner.state.file_dialog);
        let window = Window(
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("modal capture test"),
                WS_OVERLAPPED,
                0,
                0,
                1,
                1,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        SetWindowLongPtrW(window.0, GWLP_USERDATA, (&*owner as *const Owner) as isize);
        let original = SetWindowLongPtrW(
            window.0,
            GWLP_WNDPROC,
            owner_procedure as *const () as isize,
        );
        let mut input = tests::input();
        input.finish = finish::State::new(owner.state.capture_active.clone());
        // 无控件模板刻意只验证消息/生命周期；完整控件合同由隐藏窗口组合用例承担。
        let template = Template {
            style: WS_POPUP.0,
            extended_style: WS_EX_TOOLWINDOW.0,
            items: 0,
            x: -32000,
            y: -32000,
            width: 1,
            height: 1,
            menu: 0,
            class: 0,
            title: 0,
        };
        let result = DialogBoxIndirectParamW(
            None,
            (&template as *const Template).cast(),
            Some(window.0),
            Some(dialog_procedure),
            LPARAM(&input as *const Input as isize),
        );
        owner.state.global_capture.set(false);
        owner.state.global_long_capture.set(false);
        SetWindowLongPtrW(window.0, GWLP_WNDPROC, original);
        SetWindowLongPtrW(window.0, GWLP_USERDATA, 0);
        assert_eq!(result, 1, "{:?}", input.error.borrow());
        assert!(owner.observed_disabled.get());
        assert!(IsWindowEnabled(window.0).as_bool());
        assert!(owner.state.capture_active.get() && owner.state.file_dialog.get());
        assert_eq!(
            input.output.borrow().as_ref().map(|p| (p.width, p.height)),
            Some((4, 16))
        );
        assert!(!super::super::finish_route::request(
            &owner.state.capture_active
        ));
    }
}
