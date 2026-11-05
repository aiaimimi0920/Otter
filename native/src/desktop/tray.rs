//! 按需托盘 owner；先建立恢复入口再隐藏，Explorer 重启失败时恢复可见窗口。
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
#[path = "tray_startup.rs"]
mod startup;
const CALLBACK: u32 = WM_APP + 91;
const ID: u32 = 1;
// Windows SDK shellapi.h: NIN_KEYSELECT = NIN_SELECT | NINF_KEY。
// windows 0.60 未导出该宏。
const NIN_KEYSELECT: u32 = NIN_SELECT | 1;
#[derive(Default)]
pub(super) struct Tray {
    installed: Cell<bool>,
    hidden: Cell<bool>,
    panel_visible: Cell<bool>,
    restarted: Cell<u32>,
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
    let message = RegisterWindowMessageW(w!("TaskbarCreated"));
    if message == 0 {
        return Err(Error::from_win32());
    }
    state.tray.restarted.set(message);
    let mut icon = data(hwnd);
    icon.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP;
    icon.uCallbackMessage = CALLBACK;
    // 借用宿主品牌图标；run_window 的图标 owner 晚于窗口及托盘清理析构。
    icon.hIcon =
        HICON(SendMessageW(hwnd, WM_GETICON, Some(WPARAM(ICON_SMALL as usize)), None).0 as *mut _);
    if icon.hIcon.is_invalid() {
        return Err(Error::new(E_FAIL, "Host icon unavailable"));
    }
    for (slot, character) in icon.szTip.iter_mut().zip("Otter · 点击恢复".encode_utf16()) {
        *slot = character;
    }
    icon.Anonymous.uVersion = NOTIFYICON_VERSION_4;
    install_with(&state.tray.installed, |message| {
        Shell_NotifyIconW(message, &icon).as_bool()
    })?;
    Ok(())
}
unsafe fn remove(hwnd: HWND, state: &State) {
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
        .set(IsWindowVisible(state.tool_panel.get()).as_bool());
    state.tray.hidden.set(true);
    // 参数面板按需关闭，不随托盘/组恢复自动重开，避免隐藏期间悬浮或旧草稿复活。
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
            // 模态或已有请求可能拒绝排队；至少恢复当前窗口，不能丢失所有恢复入口。
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
    show(hwnd, SW_RESTORE);
    if state.tray.panel_visible.replace(false) {
        show(state.tool_panel.get(), SW_SHOWNOACTIVATE);
    }
    foreground(hwnd);
    remove(hwnd, state);
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
unsafe fn menu(hwnd: HWND, state: &State, position: WPARAM) -> Result<()> {
    let menu = Menu(CreatePopupMenu()?);
    AppendMenuW(menu.0, MF_STRING, 1, w!("恢复 Otter"))?;
    actions::append(menu.0, state)?;
    let startup = startup::append(menu.0)?;
    AppendMenuW(menu.0, MF_STRING, 2, w!("退出 Otter"))?;
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
            1 => restore(hwnd, state),
            2 => {
                super::application::request_exit(hwnd, state)?;
            }
            startup::COMMAND => startup::execute(hwnd, state, startup),
            _ => {
                if !actions::execute(hwnd, state, command) {
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
    if msg == WM_DESTROY {
        remove(hwnd, state);
        return None;
    }
    if state.tray.restarted.get() != 0 && msg == state.tray.restarted.get() {
        state.tray.installed.set(false);
        if state.tray.hidden.get() {
            if let Err(error) = install(hwnd, state) {
                eprintln!("native-tray-restart: {error}");
                restore(hwnd, state);
            }
        }
        return Some(LRESULT(0));
    }
    if msg != CALLBACK {
        return None;
    }
    if state.tray.installed.get() && state.tray.hidden.get() {
        match event(lp) {
            Some(NIN_SELECT | NIN_KEYSELECT) => restore(hwnd, state),
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
mod tests {
    use super::*;
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
