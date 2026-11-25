//! Menus and close keys for frameless Live; reuse the original system menu and revalidate HWND ownership after modal exit.
use super::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
pub(super) unsafe fn install(hwnd: HWND) -> Result<()> {
    let menu = GetSystemMenu(hwnd, false);
    for (command, label) in [
        (
            0x1f90,
            &windows::core::HSTRING::from(crate::ui_text::text("integration-disconnect-live")),
        ),
        (
            0x1fa0,
            &windows::core::HSTRING::from(crate::ui_text::text("integration-create-live-link")),
        ),
        (
            0x1fb0,
            &windows::core::HSTRING::from(crate::ui_text::text("integration-refresh-live-links")),
        ),
        (
            0x1fe0,
            &windows::core::HSTRING::from(crate::ui_text::text("integration-stop-live-keep-frame")),
        ),
        (
            0x1fd0,
            &windows::core::HSTRING::from(crate::ui_text::text("integration-close-live-recycle")),
        ),
        (
            0x1fc0,
            &windows::core::HSTRING::from(crate::ui_text::text("integration-discard-live")),
        ),
    ] {
        AppendMenuW(menu, MF_STRING, command, label)?;
    }
    Ok(())
}
pub(super) unsafe fn corner(hwnd: HWND, state: &State, lp: LPARAM) -> bool {
    if !crate::desktop::application::chrome::live_frame(&state.chrome) {
        return false;
    }
    let Ok(rect) = local::client_rect(hwnd) else {
        return false;
    };
    let size = D2D_SIZE_U {
        width: rect.width,
        height: rect.height,
    };
    if state
        .host
        .borrow()
        .as_ref()
        .and_then(|host| host.presented_viewport([size.width, size.height]))
        .is_none()
    {
        return false;
    }
    super::super::chrome::hit(
        size,
        (lp.0 as u16 as i16 as f32, (lp.0 >> 16) as u16 as i16 as f32),
    )
}
pub(super) unsafe fn handle(
    hwnd: HWND,
    state: &State,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> bool {
    if message == WM_SYSCOMMAND && wp.0 == crate::desktop::application::chrome::ports::LIVE_COMMAND
    {
        input::cancel(hwnd, state);
        crate::desktop::application::chrome::ports::request(&state.chrome, hwnd);
        return true;
    }
    let alt = message == WM_SYSKEYDOWN
        && local::pressed(VK_MENU)
        && !local::pressed(VK_CONTROL)
        && !local::pressed(VK_SHIFT)
        && !local::pressed(VK_LWIN)
        && !local::pressed(VK_RWIN);
    if alt && wp.0 == VK_DELETE.0 as usize {
        local::drag::cancel(hwnd, state);
        input::cancel(hwnd, state);
        if !state.closed.get() {
            SendMessageW(hwnd, WM_CLOSE, None, None);
        }
        return true;
    }
    if (alt && wp.0 == VK_SPACE.0 as usize)
        || (state.drag.get().is_none() && message == WM_RBUTTONUP && corner(hwnd, state, lp))
    {
        show_with(hwnd, state, |menu| {
            if !IsWindowVisible(hwnd).as_bool() {
                return 0;
            }
            let mut point = POINT::default();
            if GetCursorPos(&mut point).is_err() {
                return 0;
            }
            let _ = SetForegroundWindow(hwnd);
            let command = TrackPopupMenu(
                menu,
                TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
                point.x,
                point.y,
                None,
                hwnd,
                None,
            )
            .0 as usize;
            let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
            command
        });
        return true;
    }
    // Right-clicking yellow corners belongs to the menu; never send an isolated down to the source window.
    if state.drag.get().is_none() && message == WM_RBUTTONDOWN && corner(hwnd, state, lp) {
        input::suspend(hwnd, state);
        return true;
    }
    false
}
unsafe fn show_with(hwnd: HWND, state: &State, select: impl FnOnce(HMENU) -> usize) {
    if state.menu_open.replace(true) {
        return;
    }
    local::drag::cancel(hwnd, state);
    input::suspend(hwnd, state);
    let command = if state.closed.get() {
        0
    } else {
        select(GetSystemMenu(hwnd, false))
    };
    state.menu_open.set(false);
    if command != 0
        && !state.closed.get()
        && GetWindowLongPtrW(hwnd, GWLP_USERDATA) == state as *const State as isize
    {
        SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(command)), None);
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-live")))]
#[path = "composition_live_menu_tests.rs"]
mod tests;
