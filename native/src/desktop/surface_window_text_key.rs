//! Existing text-selection keyboard operations precede page scrolling without taking EDIT or other-node focus.
use super::*;
pub(in crate::desktop::surface_window) unsafe fn message(
    hwnd: HWND,
    state: &State,
    message: u32,
    key: u16,
) -> bool {
    let mods = modifiers();
    route(
        hwnd,
        state,
        message,
        key,
        mods.shift_key,
        mods.ctrl_key || mods.alt_key || mods.meta_key,
    )
}
unsafe fn route(
    hwnd: HWND,
    state: &State,
    message: u32,
    key: u16,
    extend: bool,
    blocked: bool,
) -> bool {
    if blocked
        || state.closed.get()
        || !state.local_edit.get()
        || GetFocus() != hwnd
        || !matches!(message, WM_KEYDOWN | WM_KEYUP)
    {
        return false;
    }
    let forward = match key {
        k if k == VK_LEFT.0 => false,
        k if k == VK_RIGHT.0 => true,
        k if k == VK_UP.0 && extend => false,
        k if k == VK_DOWN.0 && extend => true,
        k if k == VK_HOME.0 && extend => false,
        k if k == VK_END.0 && extend => true,
        _ => return false,
    };
    let Some(node) = state.keyboard.borrow().anchor().map(str::to_owned) else {
        return false;
    };
    let mut handled = false;
    let result = state.run(|host| {
        handled = if message == WM_KEYDOWN {
            if key == VK_HOME.0 || key == VK_END.0 {
                host.line_edge_text_selection(&node, forward)?
            } else if key == VK_UP.0 || key == VK_DOWN.0 {
                host.vertical_text_selection(&node, forward)?
            } else {
                host.step_text_selection(&node, forward, extend)?
            }
        } else if key == VK_UP.0 || key == VK_DOWN.0 {
            host.presentation().text_selection_vertical_ready(&node)
        } else {
            host.presentation()
                .text_selection_keyboard_ready(&node, extend)
        };
        Ok(())
    });
    if let Err(error) = result {
        *state.error.borrow_mut() = Some(error);
        cancel(state, true);
        handled = true;
    }
    if handled && message == WM_KEYDOWN {
        // Retain completed selection and discard old clicks/gestures; keyboard operations never generate remote click/edit.
        super::super::invalidate_candidates(hwnd, state);
    }
    handled
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-surface")))]
#[path = "surface_window_text_key_tests.rs"]
mod tests;
