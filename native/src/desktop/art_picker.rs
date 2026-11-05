//! Shift+1 的源绑定目录选择；只排一个创建意图，不在对话框中建窗、联网或改变源。
use crate::desktop::{self, capability_job::Loaded, State};
use std::rc::Rc;
use windows::{
    core::Result,
    Win32::{
        Foundation::*,
        UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};
#[path = "art_picker_dialog.rs"]
pub(in crate::desktop) mod dialog;
#[path = "art_picker_model.rs"]
pub(in crate::desktop) mod model;
pub(in crate::desktop) use model::{Request, Route};
#[cfg(test)]
#[path = "art_picker_tests.rs"]
pub(in crate::desktop) mod tests;

fn chord(message: u32, key: usize, control: bool, shift: bool, alt: bool, win: bool) -> bool {
    message == WM_KEYDOWN && key == 0x31 && shift && !control && !alt && !win
}
fn key_state(
    message: u32,
    key: usize,
    digit_one: bool,
    control: bool,
    shift: bool,
    alt: bool,
    win: bool,
) -> bool {
    // 某些 IME 在宿主把 Shift+1 交为全角标点；只消费真实同按的该 chord。
    // 不处理 WM_CHAR/VK_PROCESSKEY，也不改变标准 EDIT 的输入法消息管线。
    let ime_digit = message == WM_IME_CHAR && matches!(key, 0x21 | 0xff01) && digit_one;
    chord(
        if ime_digit { WM_KEYDOWN } else { message },
        if ime_digit { 0x31 } else { key },
        control,
        shift,
        alt,
        win,
    )
}
pub(in crate::desktop) unsafe fn key(message: u32, wp: WPARAM) -> bool {
    key_state(
        message,
        wp.0,
        GetKeyState(0x31) < 0,
        GetKeyState(VK_CONTROL.0 as i32) < 0,
        GetKeyState(VK_SHIFT.0 as i32) < 0,
        GetKeyState(VK_MENU.0 as i32) < 0,
        GetKeyState(VK_LWIN.0 as i32) < 0 || GetKeyState(VK_RWIN.0 as i32) < 0,
    )
}
pub(in crate::desktop) fn available(state: &State) -> bool {
    let canvas = state.canvas.borrow();
    !state.session_end.paused()
        && !state.exit_requested.get()
        && !state.group_locked.get()
        && !state.group_hidden.get()
        && desktop::capture_input::exclusive_available(state)
        && canvas.minified.is_none()
        && (canvas.image.is_some() || state.is_art())
}
pub(in crate::desktop) unsafe fn owns(hwnd: HWND, state: &State) -> bool {
    state.lifetime.owns(hwnd)
        && IsWindow(Some(hwnd)).as_bool()
        && GetWindowLongPtrW(hwnd, GWLP_USERDATA) == state as *const State as isize
}
pub(in crate::desktop) unsafe fn dispatch(
    hwnd: HWND,
    state: &State,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Option<LRESULT> {
    if !key(msg, wp) {
        return None;
    }
    if lp.0 & (1 << 30) == 0
        && owns(hwnd, state)
        && available(state)
        && state.art_request.borrow().is_none()
    {
        if let Err(error) = choose(hwnd, state, |snapshot| dialog::show(hwnd, state, snapshot)) {
            if owns(hwnd, state) {
                MessageBoxW(
                    Some(hwnd),
                    &windows::core::HSTRING::from(format!("Art 未添加。\n{error}")),
                    windows::core::w!("Otter"),
                    MB_OK | MB_ICONWARNING,
                );
            }
        }
    }
    Some(LRESULT(0))
}
unsafe fn choose(
    hwnd: HWND,
    state: &State,
    show: impl FnOnce(&model::Snapshot) -> Result<dialog::Outcome>,
) -> Result<()> {
    desktop::menu_transition::prepare(hwnd, state)?;
    if !owns(hwnd, state) || !available(state) {
        return Ok(());
    }
    let snapshot = model::Snapshot::capture(hwnd, state);
    let flag = state.file_dialog.clone();
    let Some(guard) = desktop::file_dialog::Guard::begin(&flag) else {
        return Ok(());
    };
    let outcome = show(&snapshot)?;
    drop(guard);
    if !snapshot.current(hwnd, state) || !available(state) {
        return Ok(());
    }
    match outcome {
        dialog::Outcome::Cancel => (),
        dialog::Outcome::Refresh => desktop::capability_catalog::request(hwnd, state)?,
        dialog::Outcome::Select(index) => {
            if let Some(request) = snapshot.request(index, state) {
                request.queue(hwnd, state)?;
            }
        }
    }
    Ok(())
}
