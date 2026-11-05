//! Ctrl+C 只发布当前本地选区；不调用远端 click，不经过表单队列。
use super::*;
pub(in crate::desktop::surface_window) unsafe fn message(
    hwnd: HWND,
    state: &State,
    message: u32,
    key: u16,
    lp: LPARAM,
) -> bool {
    let mods = modifiers();
    if !mods.ctrl_key
        || mods.alt_key
        || mods.meta_key
        || !matches!((message, key), (WM_KEYDOWN | WM_KEYUP, 0x43) | (WM_CHAR, 3))
    {
        return false;
    }
    if message != WM_KEYDOWN || (lp.0 as usize & (1 << 30)) != 0 {
        return state.host.borrow().as_ref().is_some_and(|host| {
            host.presentation()
                .selected_text()
                .is_ok_and(|text| text.is_some())
        });
    }
    copy_with(hwnd, state, |text, current| unsafe {
        crate::desktop::clipboard::copy_text_checked(hwnd, text, current)
    })
}
unsafe fn copy_with(
    hwnd: HWND,
    state: &State,
    publish: impl FnOnce(&str, &dyn Fn() -> bool) -> Result<()>,
) -> bool {
    if state.closed.get() || !state.local_edit.get() || GetFocus() != hwnd {
        return false;
    }
    let scene = state.pointer.borrow().identity();
    let mut text = None;
    match state.run(|host| {
        text = host.selected_text()?;
        Ok(())
    }) {
        Err(error) => {
            *state.error.borrow_mut() = Some(error);
            return true;
        }
        Ok(false) => return true,
        _ => {}
    }
    let Some(text) = text else {
        return false;
    };
    let current = || {
        !state.closed.get()
            && state.local_edit.get()
            && GetFocus() == hwnd
            && state.pointer.borrow().same_identity(&scene)
            && state.host.try_borrow().ok().is_some_and(|host| {
                host.as_ref().is_some_and(|host| {
                    host.presentation()
                        .selected_text()
                        .is_ok_and(|current| current.as_deref() == Some(text.as_str()))
                })
            })
    };
    if !current() {
        return true;
    }
    if let Err(error) = publish(&text, &current) {
        *state.error.borrow_mut() = Some(error);
    }
    true
}
#[cfg(test)]
#[path = "surface_window_text_copy_tests.rs"]
mod tests;
