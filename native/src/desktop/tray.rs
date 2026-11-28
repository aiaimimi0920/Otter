//! Resident/on-demand tray owner; establish recovery before hiding, and restore hidden windows if Explorer restart installation fails.
use super::State;
use std::cell::Cell;
use windows::{
    core::{w, Error, Result},
    Win32::{
        Foundation::*,
        UI::{Shell::*, WindowsAndMessaging::*},
    },
};

pub(super) const COMMAND: usize = 0x1047;
#[path = "tray_actions.rs"]
pub(super) mod actions;
#[path = "tray_capture_delay.rs"]
mod delay;
#[path = "tray_language.rs"]
mod language;
#[path = "tray_startup.rs"]
mod startup;
const CALLBACK: u32 = WM_APP + 91;
const ID: u32 = 1;
// Windows SDK shellapi.h: NIN_KEYSELECT = NIN_SELECT | NINF_KEY。
// windows 0.60 does not export this macro.
const NIN_KEYSELECT: u32 = NIN_SELECT | 1;
#[derive(Default)]
pub(super) struct Tray {
    installed: Cell<bool>,
    pub(super) recovery: std::rc::Rc<Cell<bool>>,
    resident: Cell<bool>,
    hidden: Cell<bool>,
    panel_visible: Cell<bool>,
    restarted: Cell<u32>,
    empty_fallback: Cell<bool>,
}

pub(super) fn is_resident(state: &State) -> bool {
    state.tray.resident.get()
}
pub(super) fn recovery_installed(state: &State) -> bool {
    state.tray.recovery.get()
}
/// Retain an operable entry only if Shell installation fails and the empty host has no registered capture keys.
pub(super) unsafe fn expose_empty_host(hwnd: HWND, state: &State) {
    if state.global_capture.get()
        || state.session_end.paused()
        || state.group_hidden.get()
        || !state.lifetime.is_sole_host()
        || !IsWindow(Some(hwnd)).as_bool()
        || IsWindowVisible(hwnd).as_bool()
    {
        return;
    }
    if empty_host(state) {
        state.tray.empty_fallback.set(true);
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        let _ = ShowWindow(state.tool_panel.get(), SW_SHOWNOACTIVATE);
    }
}
fn empty_host(state: &State) -> bool {
    let canvas = state.canvas.borrow();
    !canvas.pinned && canvas.image.is_none()
}
pub(super) unsafe fn start(hwnd: HWND, state: &State) -> Result<()> {
    start_with(state, || install(hwnd, state))
}
pub(super) fn start_with(state: &State, install: impl FnOnce() -> Result<()>) -> Result<()> {
    // Separate resident ownership from system-installation results; log failure once, never repeating Shell requests per message.
    // TaskbarCreated still reinstalls, so Explorer not yet being ready does not remove later recovery eligibility.
    if state.tray.resident.replace(true) {
        return Ok(());
    }
    install()
}

fn install_with(
    installed: &Cell<bool>,
    mut notify: impl FnMut(NOTIFY_ICON_MESSAGE) -> bool,
) -> Result<()> {
    if installed.get() {
        if !notify(NIM_DELETE) {
            return Err(Error::new(E_FAIL, "Cannot remove previous tray icon"));
        }
        installed.set(false);
    }
    if !notify(NIM_ADD) {
        return Err(Error::new(E_FAIL, "Cannot add tray icon"));
    }
    installed.set(true);
    if !notify(NIM_SETVERSION) {
        if notify(NIM_DELETE) {
            installed.set(false);
        }
        return Err(Error::new(E_FAIL, "Cannot set tray callback version"));
    }
    Ok(())
}

unsafe fn data(hwnd: HWND) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: ID,
        ..Default::default()
    }
}
unsafe fn install(hwnd: HWND, state: &State) -> Result<()> {
    if state.tray.resident.get() {
        state.tray.recovery.set(false);
    }
    let message = RegisterWindowMessageW(w!("TaskbarCreated"));
    if message == 0 {
        return Err(Error::from_win32());
    }
    state.tray.restarted.set(message);
    let mut icon = data(hwnd);
    icon.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP;
    icon.uCallbackMessage = CALLBACK;
    // Borrow the host branding icon; run_window's icon owner outlives window/tray cleanup.
    icon.hIcon =
        HICON(SendMessageW(hwnd, WM_GETICON, Some(WPARAM(ICON_SMALL as usize)), None).0 as *mut _);
    if icon.hIcon.is_invalid() {
        return Err(Error::new(E_FAIL, "Host icon unavailable"));
    }
    for (slot, character) in icon
        .szTip
        .iter_mut()
        .zip(crate::ui_text::text("tray-tooltip").encode_utf16())
    {
        *slot = character;
    }
    icon.Anonymous.uVersion = NOTIFYICON_VERSION_4;
    install_with(&state.tray.installed, |message| {
        Shell_NotifyIconW(message, &icon).as_bool()
    })?;
    if state.tray.resident.get() {
        state.tray.recovery.set(true);
    }
    Ok(())
}
pub(super) unsafe fn refresh_language(hwnd: HWND, state: &State) {
    if !state.tray.installed.get() {
        return;
    }
    let mut icon = data(hwnd);
    icon.uFlags = NIF_TIP | NIF_SHOWTIP;
    for (slot, character) in icon
        .szTip
        .iter_mut()
        .zip(crate::ui_text::text("tray-tooltip").encode_utf16())
    {
        *slot = character;
    }
    if !Shell_NotifyIconW(NIM_MODIFY, &icon).as_bool() {
        eprintln!("native-tray-language: Cannot update tray tooltip");
    }
}
unsafe fn remove(hwnd: HWND, state: &State) {
    if state.tray.resident.get() {
        state.tray.recovery.set(false);
    }
    if state.tray.installed.get() {
        if Shell_NotifyIconW(NIM_DELETE, &data(hwnd)).as_bool() {
            state.tray.installed.set(false);
        } else {
            eprintln!("native-tray: Cannot remove tray icon");
        }
    }
}
pub(super) unsafe fn hide(hwnd: HWND, state: &State) -> Result<()> {
    hide_with(hwnd, state, || install(hwnd, state))
}
pub(super) fn is_hidden(state: &State) -> bool {
    state.tray.hidden.get()
}
pub(super) unsafe fn hide_with(
    hwnd: HWND,
    state: &State,
    install: impl FnOnce() -> Result<()>,
) -> Result<()> {
    if state.file_dialog.get() || state.tray.hidden.get() {
        return Ok(());
    }
    install()?;
    state
        .tray
        .panel_visible
        .set(state.shake.panel.get() || IsWindowVisible(state.tool_panel.get()).as_bool());
    state.tray.hidden.set(true);
    // Close parameter panels on demand without reopening on tray/group restoration, avoiding floating panels or revived drafts while hidden.
    state.parameters_ui.close.set(true);
    let _ = ShowWindow(state.parameters_ui.window(), SW_HIDE);
    let _ = ShowWindow(state.tool_panel.get(), SW_HIDE);
    let _ = ShowWindow(hwnd, SW_HIDE);
    state.canvas.borrow_mut().discard_device();
    Ok(())
}
pub(super) unsafe fn restore(hwnd: HWND, state: &State) {
    if state.group_hidden.get() {
        let id = state.group_id.borrow().clone();
        if let Some(id) = id {
            // Modal state/existing requests may reject enqueueing; restore at least the current window so not all recovery entries are lost.
            if state.file_dialog.get() || state.group_request.borrow().is_some() {
                restore_window(hwnd, state);
                return;
            }
            if let Err(error) = super::group_menu::queue(
                hwnd,
                state,
                super::group_menu::Request::SetHidden(id, false),
            ) {
                eprintln!("native-group-restore: {error}");
                restore_window(hwnd, state);
            }
            return;
        }
    }
    restore_window(hwnd, state);
}
pub(super) unsafe fn restore_window(hwnd: HWND, state: &State) {
    restore_with(
        hwnd,
        state,
        |window, show| {
            let _ = ShowWindow(window, show);
        },
        |window| {
            let _ = SetForegroundWindow(window);
        },
    );
}
pub(super) unsafe fn restore_with(
    hwnd: HWND,
    state: &State,
    mut show: impl FnMut(HWND, SHOW_WINDOW_CMD),
    foreground: impl FnOnce(HWND),
) {
    if !state.tray.hidden.replace(false) || !IsWindow(Some(hwnd)).as_bool() {
        return;
    }
    let panel = state.tray.panel_visible.replace(false);
    if state.shake.hidden.get() {
        state.shake.panel.set(panel);
    } else {
        show(hwnd, SW_RESTORE);
        if panel {
            show(state.tool_panel.get(), SW_SHOWNOACTIVATE);
        }
        foreground(hwnd);
    }
    if !state.tray.resident.get() {
        remove(hwnd, state);
    }
}

unsafe fn activate(hwnd: HWND, state: &State) {
    // Empty hosts carry only tray/hotkeys; imageless Art still retains recall eligibility via pinned.
    if empty_host(state) || state.shake.hidden.get() {
        return;
    }
    if state.tray.hidden.get() {
        restore(hwnd, state);
    } else if IsWindow(Some(hwnd)).as_bool() {
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = SetForegroundWindow(hwnd);
    }
}

fn restarted_with(state: &State, install: impl FnOnce() -> Result<()>, recover: impl FnOnce()) {
    state.tray.installed.set(false);
    if state.tray.resident.get() || state.tray.hidden.get() {
        if let Err(error) = install() {
            eprintln!("native-tray-restart: {error}");
            recover();
        }
    }
}

struct Menu(HMENU);
#[path = "tray_history.rs"]
mod history;
impl Drop for Menu {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyMenu(self.0);
        }
    }
}
unsafe fn build_menu(state: &State) -> Result<(Menu, Option<crate::autostart::Status>)> {
    let menu = Menu(CreatePopupMenu()?);
    actions::append(menu.0, state)?;
    let startup = startup::append(menu.0)?;
    language::append(menu.0)?;
    delay::append(menu.0, state)?;
    AppendMenuW(
        menu.0,
        MF_STRING,
        2,
        &windows::core::HSTRING::from(crate::ui_text::text("tray-exit")),
    )?;
    Ok((menu, startup))
}
unsafe fn menu(hwnd: HWND, state: &State, position: WPARAM) -> Result<()> {
    let (menu, startup) = build_menu(state)?;
    let mut point = POINT {
        x: position.0 as i16 as i32,
        y: (position.0 >> 16) as i16 as i32,
    };
    if point.x == -1 && point.y == -1 {
        GetCursorPos(&mut point)?;
    }
    let target = super::popup_menu::target::MenuTarget::capture(&state.canvas.borrow());
    let command = super::popup_menu::track_tray(hwnd, state, menu.0, point, |entry| {
        history::deliver(hwnd, state, entry, &target)
    })?;
    if IsWindow(Some(hwnd)).as_bool() {
        match command {
            1 => activate(hwnd, state),
            2 => {
                super::application::request_exit(hwnd, state)?;
            }
            startup::COMMAND => startup::execute(hwnd, state, startup),
            _ => {
                if !delay::execute(hwnd, state, command)?
                    && !language::execute(hwnd, state, command)?
                    && !actions::execute(hwnd, state, command)
                {
                    let _ = Shell_NotifyIconW(NIM_SETFOCUS, &data(hwnd));
                }
            }
        }
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
    }
    Ok(())
}
fn event(lp: LPARAM) -> Option<u32> {
    ((lp.0 as u32 >> 16) == ID).then_some(lp.0 as u32 & 0xffff)
}
pub(super) unsafe fn dispatch(
    hwnd: HWND,
    state: &State,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Option<LRESULT> {
    // Title-bar closure requests normal exit only for the sole blank fallback host; final commit may still destroy it with WM_CLOSE.
    if msg == WM_SYSCOMMAND
        && wp.0 & 0xfff0 == SC_CLOSE as usize
        && state.tray.empty_fallback.get()
        && !state.session_end.paused()
        && state.lifetime.is_sole_host()
        && empty_host(state)
    {
        if let Err(error) = super::application::request_exit(hwnd, state) {
            eprintln!("native-tray-fallback-exit: {error}");
        }
        return Some(LRESULT(0));
    }
    if msg == WM_DESTROY {
        remove(hwnd, state);
        return None;
    }
    if state.tray.restarted.get() != 0 && msg == state.tray.restarted.get() {
        restarted_with(
            state,
            || install(hwnd, state),
            || {
                super::capture_countdown::cancel();
                if state.tray.hidden.get() {
                    restore(hwnd, state);
                } else {
                    expose_empty_host(hwnd, state);
                }
            },
        );
        return Some(LRESULT(0));
    }
    if msg != CALLBACK {
        return None;
    }
    if state.tray.installed.get() {
        match event(lp) {
            Some(NIN_SELECT | NIN_KEYSELECT) => activate(hwnd, state),
            Some(WM_CONTEXTMENU) => {
                if let Err(error) = menu(hwnd, state, wp) {
                    eprintln!("native-tray-menu: {error}");
                }
            }
            _ => {}
        }
    }
    Some(LRESULT(0))
}

#[cfg(test)]
#[path = "tray_resident_tests.rs"]
mod resident_tests;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_tray_builder_preserves_main_order_without_recovery_or_migration_commands() {
        unsafe {
            let state = State::new(None, false).unwrap();
            let (menu, _) = build_menu(&state).unwrap();
            let tree = crate::native_menu::snapshot(menu.0).unwrap();
            assert_eq!(tree.len(), 10);
            for (index, key) in [
                "tray-capture",
                "tray-live-capture",
                "tray-long-capture",
                "tray-open-image",
                "tray-receive",
            ]
            .into_iter()
            .enumerate()
            {
                assert_eq!(tree[index].label, crate::ui_text::text(key));
            }
            assert!(tree[5]
                .label
                .starts_with(crate::ui_text::text("pin-pass-menu")));
            assert!(!tree[5].checked);
            state.click.signals.enabled.set(true);
            let (checked_menu, _) = build_menu(&state).unwrap();
            let checked_tree = crate::native_menu::snapshot(checked_menu.0).unwrap();
            assert!(checked_tree[5].checked);
            state.click.signals.enabled.set(false);
            assert!(tree[6]
                .label
                .starts_with(crate::ui_text::text("tray-autostart")));
            assert_eq!(tree[7].label, crate::ui_text::text("tray-language"));
            assert_eq!(tree[8].label, crate::ui_text::text("tray-delay"));
            assert_eq!(tree[8].children.as_ref().unwrap().len(), 3);
            assert_eq!(tree[9].label, crate::ui_text::text("tray-exit"));
            assert_eq!(tree[7].children.as_ref().unwrap().len(), 2);
            assert_eq!(tree.len(), 10);
            assert!(tree
                .iter()
                .enumerate()
                .all(
                    |(index, item)| (matches!(index, 7 | 8) || item.children.is_none())
                        && !item.separator
                ));
        }
    }
    #[test]
    fn cleanup_failure_retains_ownership_and_retry_removes_before_readding() {
        let installed = Cell::new(false);
        assert!(install_with(&installed, |message| message == NIM_ADD).is_err());
        assert!(installed.get());
        let mut calls = Vec::new();
        assert!(install_with(&installed, |message| {
            calls.push(message);
            false
        })
        .is_err());
        assert_eq!(calls, [NIM_DELETE]);
        assert!(installed.get());
        calls.clear();
        install_with(&installed, |message| {
            calls.push(message);
            true
        })
        .unwrap();
        assert_eq!(calls, [NIM_DELETE, NIM_ADD, NIM_SETVERSION]);
        assert!(installed.get());
    }
    #[test]
    fn install_version_failure_removes_icon_and_add_failure_does_not_continue() {
        for fail in [Some(NIM_ADD), Some(NIM_SETVERSION), None] {
            let mut calls = Vec::new();
            let installed = Cell::new(false);
            let result = install_with(&installed, |message| {
                calls.push(message);
                Some(message) != fail
            });
            assert_eq!(result.is_ok(), fail.is_none());
            assert_eq!(installed.get(), fail.is_none());
            if fail == Some(NIM_ADD) {
                assert_eq!(calls, [NIM_ADD]);
            } else if fail.is_some() {
                assert_eq!(calls, [NIM_ADD, NIM_SETVERSION, NIM_DELETE]);
            } else {
                assert_eq!(calls, [NIM_ADD, NIM_SETVERSION]);
            }
        }
    }
    #[test]
    fn version_four_event_rejects_other_icon_and_decodes_keyboard_activation() {
        assert_eq!(
            event(LPARAM(((ID << 16) | NIN_KEYSELECT) as isize)),
            Some(NIN_KEYSELECT)
        );
        assert_eq!(event(LPARAM(((2 << 16) | NIN_SELECT) as isize)), None);
        assert_eq!(event(LPARAM(WM_CONTEXTMENU as isize)), None);
    }
}
