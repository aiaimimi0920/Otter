//! 黄色角选中 Live 钉图；本地按键不会进入源输入队列，编辑快照由应用单槽准备。
use super::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;

#[derive(Default)]
pub(super) struct Pin {
    pub selected: Cell<bool>,
    pub request: Cell<Option<(usize, isize, [u8; 256])>>,
}
pub(super) unsafe fn select(hwnd: HWND, state: &State) {
    if !state.pin.selected.replace(true) {
        input::suspend(hwnd, state);
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
    let _ = SetForegroundWindow(hwnd);
    let _ = SetFocus(Some(hwnd));
}
pub(super) unsafe fn handle(hwnd: HWND, state: &State, msg: u32, wp: WPARAM, lp: LPARAM) -> bool {
    if !state.pin.selected.get() || state.closed.get() || state.group_locked.get() {
        return false;
    }
    if msg == WM_KEYDOWN {
        if lp.0 & (1 << 30) == 0 {
            if wp.0 == VK_ESCAPE.0 as usize {
                if ![VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN]
                    .iter()
                    .any(|key| GetKeyState(key.0 as i32) < 0)
                {
                    SendMessageW(hwnd, WM_CLOSE, None, None);
                }
            } else if !matches!(wp.0, 0x10..=0x12 | 0x5b | 0x5c) {
                let mut keys = [0; 256];
                if GetKeyboardState(&mut keys).is_ok() {
                    state.pin.request.set(Some((wp.0, lp.0, keys)));
                }
            }
        }
        return true;
    }
    matches!(msg, WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP)
}
impl LiveWindow {
    pub(in crate::desktop) fn apply_pin(
        &self,
        rect: &RECT,
        opacity: f32,
        editor: crate::editor::Editor,
    ) -> Result<()> {
        self.state.opacity.set(opacity);
        self.state.with_host(|host| {
            host.set_pin_editor(editor);
            Ok(())
        })?;
        unsafe {
            SetWindowPos(
                self.hwnd,
                None,
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SWP_NOACTIVATE | SWP_NOZORDER,
            )
        }
    }
    pub(in crate::desktop) fn take_pin_request(&self) -> Option<(usize, isize, [u8; 256])> {
        self.state.pin.request.take()
    }
    pub(in crate::desktop) fn pin_editing(&self, editing: bool) {
        unsafe {
            input::suspend(self.hwnd, &self.state);
            let _ = ShowWindow(self.hwnd, if editing { SW_HIDE } else { SW_SHOWNOACTIVATE });
        }
    }
    pub(in crate::desktop) fn resume_source(&self) {
        self.state.pin.selected.set(false);
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
            let _ = SetForegroundWindow(self.hwnd);
            let _ = SetFocus(Some(self.hwnd));
        }
    }
}
