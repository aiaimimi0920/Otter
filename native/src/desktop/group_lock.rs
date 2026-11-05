//! 分组锁定阻止用户修改输入，不拦截生命周期/绘制/任务回收；应用只在编辑任务空闲时锁定。
use super::{context_menu as menu, State};
use windows::Win32::{
    Foundation::*,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};
pub(super) fn editing_busy(state: &State) -> bool {
    state.open_job.borrow().is_some()
        || state.blur_job.borrow().is_busy()
        || state.erase_job.borrow().is_busy()
        || state.resize_job.borrow().is_busy()
        || state.flip_job.borrow().is_busy()
        || state.crop_job.borrow().is_some()
        || state.rasterize_job.borrow().is_some()
}
pub(super) fn allows(command: usize) -> bool {
    super::group_menu::handles(command)
        || super::render_commands::handles(command)
        || super::device_authorization_ui::handles(command)
        || matches!(
            command,
            super::application::NEW_WINDOW
                | super::application::EXIT_APPLICATION
                | super::application::SAVE_SESSION
                | super::application::SAVE_SESSION_AS
                | super::application::OPEN_SESSION
                | super::application::OPEN_SESSION_ASSETS
                | super::application::IMPORT_NAMING
                | super::application::SAVE_NAMING
                | super::application::UPDATE_NAMING
                | super::application::AUTO_NAMING
                | super::application::SAVE_PNG_DEFAULT
                | super::naming_input::COMMAND
                | super::cache_input::COMMAND
                | super::execution_cancel::COMMAND
                | super::capability_catalog::COMMAND
                | menu::OPEN
                | menu::EXPORT
                | super::copy_job::COMMAND
                | super::paste_input::COMMAND
                | super::capture_input::NEW_PIN
                | super::capture_input::NEW_FULL_PIN
                | super::capture_input::LONG_RECORDING
                | super::capture_input::LONG_VERTICAL
                | super::capture_input::LONG_HORIZONTAL
                | super::tray::COMMAND
                | super::global_capture::COMMAND
                | super::capture_protection::COMMAND
        )
}
pub(super) unsafe fn dispatch(state: &State, message: u32, wp: WPARAM) -> Option<LRESULT> {
    if !state.group_locked.get() {
        return None;
    }
    match message {
        WM_NCHITTEST => Some(LRESULT(HTCLIENT as isize)),
        WM_LBUTTONDOWN | WM_LBUTTONUP | WM_LBUTTONDBLCLK | WM_MOUSEMOVE | WM_MOUSEWHEEL
        | WM_MOUSEHWHEEL => Some(LRESULT(0)),
        WM_NCLBUTTONDOWN
            if matches!(
                wp.0 as u32,
                HTCAPTION
                    | HTLEFT
                    | HTRIGHT
                    | HTTOP
                    | HTBOTTOM
                    | HTTOPLEFT
                    | HTTOPRIGHT
                    | HTBOTTOMLEFT
                    | HTBOTTOMRIGHT
            ) =>
        {
            Some(LRESULT(0))
        }
        WM_SYSCOMMAND if matches!(wp.0 as u32 & 0xfff0, SC_MOVE | SC_SIZE) => Some(LRESULT(0)),
        WM_KEYDOWN => {
            if wp.0 == VK_ESCAPE.0 as usize
                || (GetKeyState(VK_CONTROL.0 as i32) < 0
                    && matches!(wp.0, 0x43 | 0x4f | 0x56 | 0x53))
            {
                None
            } else {
                Some(LRESULT(0))
            }
        }
        _ => None,
    }
}
pub(super) unsafe fn restrict_menu(menu: HMENU, state: &State) {
    if !state.group_locked.get() {
        return;
    }
    for index in 0..GetMenuItemCount(Some(menu)) {
        let child = GetSubMenu(menu, index);
        if !child.is_invalid() {
            restrict_menu(child, state);
            continue;
        }
        let command = GetMenuItemID(menu, index);
        if command != u32::MAX && !allows(command as usize) {
            let _ = EnableMenuItem(menu, command, MF_BYCOMMAND | MF_GRAYED);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saving_workspace_is_allowed_without_unlocking_group_content() {
        assert!(allows(super::super::application::SAVE_SESSION));
        assert!(allows(super::super::application::SAVE_SESSION_AS));
    }
    #[test]
    fn lock_blocks_drag_and_replacement_without_blocking_lifetime_messages() {
        let state = State::new(None, false).unwrap();
        state.group_locked.set(true);
        unsafe {
            assert_eq!(
                dispatch(&state, WM_NCHITTEST, WPARAM(0)),
                Some(LRESULT(HTCLIENT as isize))
            );
            assert_eq!(
                dispatch(&state, WM_SYSCOMMAND, WPARAM(SC_MOVE as usize)),
                Some(LRESULT(0))
            );
            for message in [
                WM_PAINT,
                WM_TIMER,
                WM_CLOSE,
                WM_DESTROY,
                WM_NCDESTROY,
                WM_CONTEXTMENU,
            ] {
                assert!(dispatch(&state, message, WPARAM(0)).is_none());
            }
            let image = super::super::image::Image {
                width: 1,
                height: 1,
                pixels: vec![255; 4].into(),
            };
            assert!(super::super::open_job::start_destination(
                HWND::default(),
                &state,
                super::super::open_job::Input::Image(image),
                false
            )
            .is_err());
            assert!(state.open_job.borrow().is_none());
        }
    }
    #[test]
    fn locked_commands_keep_recovery_and_outputs_but_reject_mutations() {
        for command in [
            menu::OPEN,
            menu::EXPORT,
            super::super::group_menu::LOCK,
            super::super::copy_job::COMMAND,
        ] {
            assert!(allows(command));
        }
        for command in [
            super::super::open_image::REPLACE,
            super::super::paste_input::REPLACE,
            super::super::capture_input::COMMAND,
            menu::APPLY_STYLE,
            0x42,
        ] {
            assert!(!allows(command));
        }
    }
}
