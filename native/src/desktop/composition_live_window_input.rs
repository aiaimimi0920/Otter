//! Live HWND 的输入适配；只排队，不运行权限查询/跨窗口投递。撤销信号可越过嵌套消息泵。
use super::*;
use crate::{live_input::Event, live_input_worker::StopSignal};
use std::collections::VecDeque;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetCapture, ReleaseCapture, SetCapture};
pub(super) const TOGGLE: usize = 0x1ff0;
#[derive(Default)]
pub(super) struct Control {
    allowed: bool,
    remote: bool,
    enabled: bool,
    pending: bool,
    buttons: u8,
    request: Option<bool>,
    events: VecDeque<Event>,
    stop: Option<StopSignal>,
    shown: Option<(bool, bool)>,
    failed: bool,
    notice: bool,
}
impl Control {
    pub(super) fn source_input_allowed(&self) -> bool {
        self.allowed
    }
    pub(super) fn cancel(&mut self) {
        self.request = Some(false);
        self.clear();
    }
    fn clear(&mut self) {
        self.enabled = false;
        self.pending = false;
        self.buttons = 0;
        self.events.clear();
        if let Some(stop) = self.stop.take() {
            stop.request();
        }
    }
    fn push(&mut self, event: Event) -> bool {
        if matches!(event, Event::Move { .. })
            && matches!(self.events.back(), Some(Event::Move { .. }))
        {
            *self.events.back_mut().unwrap() = event;
            return true;
        }
        if self.events.len() >= 16 {
            self.cancel();
            return false;
        }
        self.events.push_back(event);
        true
    }
}
impl LiveWindow {
    pub(crate) fn block_local_input(&self, remote: bool) {
        self.state.input.borrow_mut().remote = remote;
        if self.is_alive() {
            unsafe {
                menu_status(self.hwnd, &self.state, false, false);
            }
        }
    }
    pub(in crate::desktop) fn take_input_notice(&self) -> bool {
        std::mem::take(&mut self.state.input.borrow_mut().notice)
    }
    pub(crate) fn allow_input(&self) -> Result<()> {
        self.state.input.borrow_mut().allowed = true;
        unsafe {
            let _ = EnableMenuItem(
                GetSystemMenu(self.hwnd, false),
                TOGGLE as u32,
                MF_BYCOMMAND | MF_ENABLED,
            );
            let _ = EnableMenuItem(
                GetSystemMenu(self.hwnd, false),
                super::publication::PUBLISH_CONTROL as u32,
                MF_BYCOMMAND | MF_ENABLED,
            );
            let _ = EnableMenuItem(
                GetSystemMenu(self.hwnd, false),
                super::publication::PUBLISH_OBSERVE as u32,
                MF_BYCOMMAND | MF_ENABLED,
            );
        }
        Ok(())
    }
    pub(crate) fn take_input_request(&self) -> Option<bool> {
        self.state.input.borrow_mut().request.take()
    }
    pub(crate) fn pop_input(&self) -> Option<Event> {
        self.state.input.borrow_mut().events.pop_front()
    }
    pub(crate) fn input_status(&self, enabled: bool, pending: bool, stop: Option<StopSignal>) {
        {
            let mut input = self.state.input.borrow_mut();
            if (enabled || pending) && input.failed {
                input.failed = false;
                input.notice = false;
                input.shown = None;
            }
            if !enabled && !pending {
                input.clear();
            }
            input.enabled = enabled;
            input.pending = pending;
            input.stop = stop;
        }
        if self.is_alive() {
            unsafe {
                display_status(self.hwnd, &self.state, enabled, pending);
            }
        }
    }
    pub(crate) fn input_failure(&self) {
        let changed = {
            let mut input = self.state.input.borrow_mut();
            let changed = !input.failed;
            input.failed = true;
            input.notice |= changed;
            changed
        };
        if changed && self.is_alive() {
            unsafe {
                menu_status(self.hwnd, &self.state, false, false);
                let _ = SetWindowTextW(self.hwnd, w!("Otter Live · 控制不可用"));
            }
        }
    }
}
unsafe fn display_status(hwnd: HWND, state: &State, enabled: bool, pending: bool) {
    let changed =
        state.input.borrow_mut().shown.replace((enabled, pending)) != Some((enabled, pending));
    if changed {
        menu_status(hwnd, state, enabled, pending);
        let _ = CheckMenuItem(
            GetSystemMenu(hwnd, false),
            TOGGLE as u32,
            (MF_BYCOMMAND | if enabled { MF_CHECKED } else { MF_UNCHECKED }).0,
        );
        let _ = SetWindowTextW(
            hwnd,
            if enabled {
                w!("Otter Live · 控制中")
            } else if pending {
                w!("Otter Live · 启用中")
            } else {
                w!("Otter Live")
            },
        );
    }
    if !enabled && state.drag.get().is_none() && GetCapture() == hwnd {
        let _ = ReleaseCapture();
    }
}
unsafe fn menu_status(hwnd: HWND, state: &State, enabled: bool, pending: bool) {
    let (allowed, failed) = {
        let input = state.input.borrow();
        (input.allowed && !input.remote, input.failed)
    };
    let label = if enabled {
        w!("停止控制源窗口")
    } else if pending {
        w!("控制源窗口（启用中）")
    } else if failed {
        w!("控制源窗口（上次失败，可重试）")
    } else {
        w!("控制源窗口")
    };
    let _ = ModifyMenuW(
        GetSystemMenu(hwnd, false),
        TOGGLE as u32,
        MF_BYCOMMAND | MF_STRING | if allowed { MF_ENABLED } else { MF_GRAYED },
        TOGGLE,
        label,
    );
}
pub(super) unsafe fn install(hwnd: HWND) -> Result<()> {
    let menu = GetSystemMenu(hwnd, false);
    AppendMenuW(menu, MF_SEPARATOR, 0, None)?;
    AppendMenuW(menu, MF_STRING | MF_GRAYED, TOGGLE, w!("控制源窗口"))
}
fn viewport(hwnd: HWND, state: &State) -> Option<crate::live_viewport::Viewport> {
    let mut rect = RECT::default();
    unsafe { GetClientRect(hwnd, &mut rect) }.ok()?;
    state
        .host
        .borrow()
        .as_ref()?
        .presented_viewport([
            (rect.right - rect.left).max(0) as u32,
            (rect.bottom - rect.top).max(0) as u32,
        ])
        .map(|(_, view)| view)
}
pub(super) unsafe fn cancel(hwnd: HWND, state: &State) {
    state.input.borrow_mut().cancel();
    display_status(hwnd, state, false, false);
    if GetCapture() == hwnd {
        let _ = ReleaseCapture();
    }
}
/// Some 表示已消费；无输入控制时保留系统窗口行为。
pub(super) unsafe fn handle(
    hwnd: HWND,
    state: &State,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Option<Result<()>> {
    if super::menu::handle(hwnd, state, message, wp, lp) {
        return Some(Ok(()));
    }
    // Popup 的嵌套消息泵不能创建新的源控制或本地几何事务；关闭/生命周期仍可达。
    if state.menu_open.get()
        && (matches!(message, WM_KEYDOWN | WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP)
            || (WM_MOUSEFIRST..=WM_MOUSELAST).contains(&message)
            || (message == WM_SYSCOMMAND && wp.0 & 0xfff0 == TOGGLE))
    {
        return Some(Ok(()));
    }
    if super::art::handle(hwnd, state, message, wp, lp) {
        return Some(Ok(()));
    }
    if super::local::drag::handle(hwnd, state, message, wp, lp) {
        return Some(Ok(()));
    }
    if message == WM_SYSCOMMAND && wp.0 & 0xfff0 == TOGGLE {
        let toggle = {
            let input = state.input.borrow();
            (input.allowed && !input.remote).then_some(!(input.enabled || input.pending))
        };
        if let Some(enable) = toggle {
            if !enable {
                cancel(hwnd, state);
            } else {
                {
                    let mut input = state.input.borrow_mut();
                    input.pending = true;
                    input.request = Some(true);
                }
                display_status(hwnd, state, false, true);
            }
        }
        return Some(Ok(()));
    }
    if matches!(message, WM_KILLFOCUS | WM_CANCELMODE | WM_ENTERMENULOOP)
        || (message == WM_ACTIVATE && wp.0 & 0xffff == WA_INACTIVE as usize)
        || (message == WM_SIZE && wp.0 == SIZE_MINIMIZED as usize)
    {
        cancel(hwnd, state);
        return None;
    }
    if message == WM_CAPTURECHANGED {
        let held = state.input.borrow().buttons != 0;
        if held {
            cancel(hwnd, state);
        }
        return None;
    }
    if let Some(result) = super::local::handle(hwnd, state, message, wp, lp) {
        return Some(result);
    }
    if !state.input.borrow().enabled {
        return None;
    }
    // 保留本地系统菜单和关闭快捷键；进入菜单/关闭时立即撤销源输入。
    if message == WM_SYSKEYDOWN && (wp.0 == 0x20 || wp.0 == 0x73) {
        return None;
    }
    use crate::desktop::window_input_event as event;
    if !event::recognized(message) {
        return None;
    }
    let Some(view) = viewport(hwnd, state) else {
        cancel(hwnd, state);
        return Some(Ok(()));
    };
    let held = state.input.borrow().buttons != 0;
    let event = match event::decode(hwnd, message, wp, lp, view, held) {
        Ok(Some(event)) => event,
        Ok(None) => return Some(Ok(())),
        Err(()) => {
            cancel(hwnd, state);
            return Some(Ok(()));
        }
    };
    let edge = event::button(event);
    if let Some((mask, down)) = edge {
        let mut input = state.input.borrow_mut();
        if down {
            input.buttons |= mask;
        } else {
            input.buttons &= !mask;
        }
    }
    let accepted = state.input.borrow_mut().push(event);
    if !accepted {
        cancel(hwnd, state);
    } else if edge.is_some_and(|(_, down)| down) {
        let _ = SetCapture(hwnd);
        if GetCapture() != hwnd {
            cancel(hwnd, state);
        }
    } else if edge.is_some_and(|(_, down)| !down) {
        let held = state.input.borrow().buttons != 0;
        if !held && GetCapture() == hwnd {
            let _ = ReleaseCapture();
        }
    }
    Some(Ok(()))
}
