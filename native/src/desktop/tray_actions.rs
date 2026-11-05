//! 托盘工作流只路由已有命令；菜单快照不授权执行，恢复宿主后再次检查可用性。
use super::*;
use crate::desktop::{
    capture_input, context_menu, open_image, tool_panel_availability::Availability,
};
use windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled;

const ENTRIES: &[(usize, &str)] = &[
    (capture_input::NEW_PIN, "区域截图"),
    (capture_input::live::COMMAND, "区域 Live"),
    (capture_input::live::WINDOW, "窗口 Live…"),
    (capture_input::LONG_RECORDING, "长截图录制（自行滚动）"),
    (capture_input::LONG_VERTICAL, "垂直长截图"),
    (capture_input::LONG_HORIZONTAL, "水平长截图"),
    (context_menu::OPEN, "打开图片为新钉图…"),
];
fn known(command: usize) -> bool {
    ENTRIES.iter().any(|(id, _)| *id == command)
}
fn available(state: &State, command: usize) -> bool {
    known(command)
        && capture_input::exclusive_available(state)
        && state
            .new_pin
            .try_borrow()
            .is_ok_and(|value| value.is_none())
        && if matches!(
            command,
            capture_input::live::COMMAND | capture_input::live::WINDOW
        ) {
            capture_input::live::available(state)
        } else {
            Availability::capture(state).is_some_and(|value| value.allows(command))
        }
}
pub(super) unsafe fn append(menu: HMENU, state: &State) -> Result<()> {
    AppendMenuW(menu, MF_SEPARATOR, 0, None)?;
    for &(command, label) in ENTRIES {
        AppendMenuW(
            menu,
            MF_STRING
                | if available(state, command) {
                    MF_ENABLED
                } else {
                    MF_GRAYED
                },
            command,
            &windows::core::HSTRING::from(label),
        )?;
    }
    AppendMenuW(menu, MF_SEPARATOR, 0, None)
}
pub(in crate::desktop) fn execute_with(
    command: usize,
    ready: impl Fn() -> bool,
    restore: impl FnOnce(),
    run: impl FnOnce(),
) -> bool {
    if !known(command) || !ready() {
        return false;
    }
    restore();
    // 恢复窗口可能重入消息循环；不沿用弹出菜单前的任务/模态状态。
    if ready() {
        run();
    }
    true
}
pub(super) unsafe fn execute(hwnd: HWND, state: &State, command: usize) -> bool {
    execute_with(
        command,
        || {
            IsWindow(Some(hwnd)).as_bool()
                && IsWindowEnabled(hwnd).as_bool()
                && available(state, command)
        },
        || super::restore_window(hwnd, state),
        || {
            if command == capture_input::live::WINDOW {
                capture_input::live::start_window(hwnd, state);
            } else if command == capture_input::live::COMMAND {
                capture_input::live::start(hwnd, state);
            } else if command == context_menu::OPEN {
                open_image::start(hwnd, state);
            } else {
                capture_input::start(hwnd, state, command);
            }
        },
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn commands_restore_before_running_and_recheck_after_reentry() {
        for &(command, _) in ENTRIES {
            let calls = std::cell::RefCell::new(Vec::new());
            assert!(execute_with(
                command,
                || true,
                || calls.borrow_mut().push("restore"),
                || calls.borrow_mut().push("run")
            ));
            assert_eq!(*calls.borrow(), ["restore", "run"]);
            let ready = Cell::new(true);
            assert!(execute_with(
                command,
                || ready.get(),
                || ready.set(false),
                || panic!("stale action")
            ));
            assert!(!execute_with(
                command,
                || false,
                || panic!("busy restore"),
                || panic!("busy run")
            ));
        }
        assert!(!execute_with(
            0,
            || true,
            || panic!("unknown restore"),
            || panic!("unknown run")
        ));
    }
    #[test]
    fn menu_uses_existing_commands_and_rejects_modal_or_pending_delivery() {
        unsafe {
            let _apartment = crate::desktop::Apartment::new().unwrap();
            let state = State::new(None, true).unwrap();
            let menu = Menu(CreatePopupMenu().unwrap());
            append(menu.0, &state).unwrap();
            // 独立列出产品入口，防止新增录制时丢掉原手动双轴入口或误接命令。
            let expected = [
                capture_input::NEW_PIN,
                capture_input::live::COMMAND,
                capture_input::live::WINDOW,
                capture_input::LONG_RECORDING,
                capture_input::LONG_VERTICAL,
                capture_input::LONG_HORIZONTAL,
                context_menu::OPEN,
            ];
            assert_eq!(GetMenuItemCount(Some(menu.0)), (expected.len() + 2) as i32);
            for position in [0, (expected.len() + 1) as u32] {
                assert_ne!(
                    GetMenuState(menu.0, position, MF_BYPOSITION) & MF_SEPARATOR.0,
                    0
                );
            }
            for (index, command) in expected.into_iter().enumerate() {
                assert_eq!(GetMenuItemID(menu.0, index as i32 + 1), command as u32);
            }
            for &(command, _) in ENTRIES {
                assert!(available(&state, command));
                assert_eq!(
                    GetMenuState(menu.0, command as u32, MF_BYCOMMAND) & MF_GRAYED.0,
                    0
                );
            }
            state.file_dialog.set(true);
            for &(command, _) in ENTRIES {
                assert!(!available(&state, command));
            }
            state.file_dialog.set(false);
            *state.new_pin.borrow_mut() = Some(Vec::new());
            for &(command, _) in ENTRIES {
                assert!(!available(&state, command));
            }
        }
    }
}
