//! Tray workflows only route existing commands; menu snapshots authorize nothing, and availability is rechecked after host restoration.
use super::*;
use crate::desktop::{
    capture_input, context_menu, open_image, tool_panel_availability::Availability,
};
use windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled;

const ENTRIES: &[(usize, &str)] = &[
    (capture_input::NEW_PIN, "tray-capture"),
    (capture_input::live::COMMAND, "tray-live-capture"),
    (capture_input::LONG_RECORDING, "tray-long-capture"),
    (context_menu::OPEN, "tray-open-image"),
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
pub(in crate::desktop) unsafe fn append(menu: HMENU, state: &State) -> Result<()> {
    for &(command, label) in &ENTRIES[..3] {
        append_action(menu, state, command, label)?;
    }
    super::delay::append(menu, state)?;
    AppendMenuW(menu, MF_SEPARATOR, 0, None)?;
    append_pass(menu, state)?;
    let sub = Menu(CreatePopupMenu()?);
    crate::desktop::device_authorization_ui::policy::append_choices(
        sub.0,
        &state.authorization.borrow(),
    )?;
    AppendMenuW(
        menu,
        MF_POPUP,
        sub.0 .0 as usize,
        &windows::core::HSTRING::from(crate::ui_text::text("tray-receive")),
    )?;
    std::mem::forget(sub);
    append_action(menu, state, context_menu::OPEN, "tray-open-image")?;
    if state.shake.signals.count.get() != 0 {
        AppendMenuW(
            menu,
            MF_STRING,
            RESTORE_SHAKE,
            &windows::core::HSTRING::from(crate::ui_text::text("workspace-shake-restore")),
        )?;
    }
    Ok(())
}
unsafe fn append_action(menu: HMENU, state: &State, command: usize, label: &str) -> Result<()> {
    AppendMenuW(
        menu,
        MF_STRING
            | if available(state, command) {
                MF_ENABLED
            } else {
                MF_GRAYED
            },
        command,
        &windows::core::HSTRING::from(tray_label(state, command, crate::ui_text::text(label))),
    )
}
unsafe fn append_pass(menu: HMENU, state: &State) -> Result<()> {
    let pass_on = state.click.signals.enabled.get();
    AppendMenuW(
        menu,
        MF_STRING | if pass_on { MF_CHECKED } else { MF_UNCHECKED },
        crate::desktop::pin_click_through::RESTORE,
        &windows::core::HSTRING::from(tray_label(
            state,
            crate::desktop::pin_click_through::RESTORE,
            crate::ui_text::text("pin-pass-menu"),
        )),
    )
}
const RESTORE_SHAKE: usize = 0x21c1;
pub(super) fn tray_label(state: &State, command: usize, label: &str) -> String {
    let slots = state
        .shortcut_display
        .get()
        .map(|(_, slots)| slots)
        .unwrap_or_else(|| crate::desktop::shortcut_registry::registered(state));
    let shortcut = match command {
        capture_input::NEW_PIN if slots[0] => Some("capture.region"),
        capture_input::live::COMMAND if slots[1] => Some("capture.live"),
        capture_input::LONG_RECORDING if slots[2] => Some("capture.scroll"),
        crate::desktop::capture_countdown::THREE if slots[3] => Some("capture.delay"),
        crate::desktop::pin_click_through::RESTORE if slots[4] => Some("pin.restore-input"),
        _ => None,
    };
    shortcut
        .and_then(|id| state.shortcut_policy.borrow().chord(id))
        .map_or_else(
            || label.into(),
            |chord| format!("{label}\t{}", chord.label()),
        )
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
    // Window restoration may reenter the message loop; never reuse pre-popup task/modal state.
    if ready() {
        run();
    }
    true
}
pub(super) unsafe fn execute(hwnd: HWND, state: &State, command: usize) -> bool {
    if command == crate::desktop::pin_click_through::RESTORE {
        state.click.signals.toggle_all.set(true);
        return true;
    }
    if command == RESTORE_SHAKE {
        state.shake.signals.restore.set(true);
        return true;
    }
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
            // List mainline product entries separately to prevent adding migration-branch commands or wiring incorrect ones.
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
