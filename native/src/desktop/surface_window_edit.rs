//! HWND 输入接线；默认过程产生的同步通知在 Host 被取出时忽略，避免程序回写递归派发。
use super::*;
use crate::desktop::surface_edit_messages as messages;
use crate::{surface_click_event::ClickEvent, surface_live_binding::Binding};
impl SurfaceWindow {
    pub(crate) fn bind_edits(&self, binding: &Binding) -> Result<()> {
        if !self.is_alive() || !self.state.run(|host| host.bind_edits(binding))? {
            return Err(Error::new(E_FAIL, "Surface edit mount unavailable"));
        }
        Ok(())
    }
    pub(crate) fn has_pending_edit(&self) -> bool {
        self.is_alive()
            && self
                .state
                .host
                .borrow()
                .as_ref()
                .is_some_and(Host::edit_pending)
    }
    #[cfg(test)]
    pub(crate) fn prepare_edit(&self, binding: &Binding, revision: u64) -> Result<Vec<ClickEvent>> {
        self.prepare_edit_before(binding, revision, None)
    }
    pub(crate) fn prepare_edit_before(
        &self,
        binding: &Binding,
        revision: u64,
        pending: Option<&DeferredClick>,
    ) -> Result<Vec<ClickEvent>> {
        if !self.pointer_enabled() {
            return Err(Error::new(E_FAIL, "Surface edit submission disabled"));
        }
        let mut events = Vec::new();
        if !self.state.run(|host| {
            events = if let Some(pending) = pending {
                host.prepare_edit_before(binding, revision, pending.fence())?
            } else {
                host.prepare_edit(binding, revision)?
            };
            Ok(())
        })? || !self.is_alive()
        {
            return Err(Error::new(E_FAIL, "Surface edit preparation unavailable"));
        }
        Ok(events)
    }
    pub(crate) fn cancel_edits(&self, events: &[ClickEvent]) {
        let _ = self.state.run(|host| {
            host.cancel_edits(events);
            Ok(())
        });
    }
    pub(crate) fn prepare_javascript_edit(
        &self,
        binding: &Binding,
        pending: Option<&DeferredClick>,
    ) -> Result<Option<crate::desktop::surface_edit_mount::JavaScriptInput>> {
        if !self.pointer_enabled() {
            return Err(Error::new(E_FAIL, "JavaScript edit submission disabled"));
        }
        let mut input = None;
        if self.state.javascript_keyboard.get()
            && !unsafe { javascript_pointer::hover_eligible(self.hwnd, &self.state) }
        {
            pointer::cancel(self.hwnd, &self.state);
        }
        if !self.state.run(|host| {
            input = host.prepare_javascript_edit(
                binding,
                pending.map(DeferredClick::fence),
                self.viewport_epoch(),
                unsafe { javascript_pointer::eligible(self.hwnd, &self.state) },
                unsafe { javascript_pointer::hover_eligible(self.hwnd, &self.state) },
            )?;
            Ok(())
        })? || !self.is_alive()
        {
            return Err(Error::new(
                E_FAIL,
                "JavaScript edit preparation unavailable",
            ));
        }
        Ok(input)
    }
    #[cfg(test)]
    pub(crate) fn settle_javascript_edit(&self, id: &str) -> Result<()> {
        self.settle_javascript_ticket(id)?;
        self.complete_javascript_edit(id, None)
    }
    pub(crate) fn settle_javascript_ticket(&self, id: &str) -> Result<()> {
        if !self.is_alive() || !self.state.run(|host| host.settle_javascript_edit(id))? {
            return Err(Error::new(E_FAIL, "JavaScript edit settlement unavailable"));
        }
        Ok(())
    }
    pub(crate) fn fail_edits(&self, events: &[ClickEvent]) {
        let _ = self.state.run(|host| {
            host.fail_edits(events);
            Ok(())
        });
    }
    pub(crate) fn settle_edits(&self, events: &[ClickEvent]) -> Result<()> {
        if !self.is_alive() {
            return Ok(());
        }
        if !self.state.run(|host| host.settle_edits(events))? {
            return Err(Error::new(E_FAIL, "Surface edit settlement unavailable"));
        }
        // 无子 HWND 的字段也可能在结算时采用权威值，需要请求一次父窗口重绘。
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
        Ok(())
    }
}
pub(super) unsafe fn message(
    hwnd: HWND,
    state: &State,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Option<LRESULT> {
    if pointer::slider_click::message(hwnd, state, message, wp, lp) {
        return Some(LRESULT(0));
    }
    if let Some(result) = javascript_keyboard::message(hwnd, state, message, wp, lp) {
        return Some(result);
    }
    if message == messages::CAN_PASTE {
        return Some(LRESULT(isize::from(
            !state.closed.get()
                && state.local_edit.get()
                && state
                    .host
                    .borrow()
                    .as_ref()
                    .is_some_and(|host| host.number_editable(HWND(lp.0 as *mut _))),
        )));
    }
    if matches!(message, WM_CANCELMODE | WM_SIZE | WM_DPICHANGED)
        || (message == WM_ACTIVATE && wp.0 & 0xffff == WA_INACTIVE as usize)
    {
        let _ = state.run(|host| {
            host.cancel_spinners();
            Ok(())
        });
    }
    if message == WM_NOTIFY && lp.0 != 0 {
        use windows::Win32::UI::{
            Controls::{NMHDR, NMUPDOWN, UDN_DELTAPOS},
            Input::KeyboardAndMouse::{GetFocus, SetFocus},
        };
        let header = &*(lp.0 as *const NMHDR);
        if header.code == UDN_DELTAPOS {
            let notification = &*(lp.0 as *const NMUPDOWN);
            // WM_NOTIFY 指针仅在同步消息期有效，不进入草稿队列。
            let spinner = header.hwndFrom;
            let delta = notification.iDelta;
            let edit = state
                .host
                .borrow()
                .as_ref()
                .and_then(|host| host.spinner_edit(spinner));
            if !state.closed.get() && state.local_edit.get() && matches!(delta, -1 | 1) {
                if let Some(edit) = edit {
                    // 切焦可能提交旧字段并重入窗口；不得跨 SetFocus 持有 Host 借用。
                    let _ = SetFocus(Some(edit));
                    if !state.closed.get() && state.local_edit.get() && GetFocus() == edit {
                        if let Err(error) = state.run(|host| {
                            if host.spinner_edit(spinner) == Some(edit) {
                                host.edit_message(
                                    edit,
                                    if delta > 0 {
                                        messages::STEP_UP
                                    } else {
                                        messages::STEP_DOWN
                                    },
                                    0,
                                )?;
                            }
                            Ok(())
                        }) {
                            *state.error.borrow_mut() = Some(error);
                        }
                    }
                }
            }
            // 永不让系统 integer position 成为第二份值状态，包括撤权和重入期间。
            return Some(LRESULT(1));
        }
    }
    if matches!(message, WM_CTLCOLOREDIT | WM_CTLCOLORSTATIC) {
        if let Some(result) = crate::desktop::surface_edit_control::paint(
            hwnd,
            HWND(lp.0 as *mut _),
            HDC(wp.0 as *mut _),
        ) {
            return Some(result);
        }
        let owned = state
            .host
            .borrow()
            .as_ref()
            .is_some_and(|host| host.edit_owns(HWND(lp.0 as *mut _)));
        if owned {
            let dc = HDC(wp.0 as *mut _);
            return crate::desktop::surface_control_colors::paint(dc);
        }
    }
    if message == messages::NAVIGATE {
        if state.closed.get()
            || !state.local_edit.get()
            || windows::Win32::UI::Input::KeyboardAndMouse::GetFocus() != HWND(lp.0 as *mut _)
        {
            return Some(LRESULT(0));
        }
        let child = HWND(lp.0 as *mut _);
        let captured_slider = windows::Win32::UI::Input::KeyboardAndMouse::GetCapture() == child
            && state.host.borrow().as_ref().is_some_and(|host| {
                host.accessibility_edit_id(child)
                    .is_some_and(|id| host.slider_value(&id).is_some())
            });
        if captured_slider {
            // Tab 只切换键盘焦点，不能撤销仍在捕获的 Slider 原始鼠标候选。
            state.keyboard.borrow_mut().cancel_press();
        } else {
            pointer::invalidate(hwnd, state);
        }
        let mut next = None;
        let mut child = false;
        let result = state.run(|host| {
            (next, child) = host.navigate_edit(HWND(lp.0 as *mut _), wp.0 != 0)?;
            Ok(())
        });
        if let Err(error) = result {
            *state.error.borrow_mut() = Some(error);
        }
        if !state.closed.get() {
            if !child {
                let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(hwnd));
            }
            keyboard::set_focus(state, next);
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
        return Some(LRESULT(0));
    }
    if message == crate::desktop::surface_slider_control::NOTIFY {
        if !state.closed.get() && state.host.borrow().is_some() && wp.0 <= 1 {
            if let Err(error) =
                state.run(|host| host.edit_message(HWND(lp.0 as *mut _), message, wp.0 as u32))
            {
                *state.error.borrow_mut() = Some(error);
            }
        }
        return Some(LRESULT(0));
    }
    if message == WM_COMMAND
        || matches!(
            message,
            messages::COMPOSITION_START
                | messages::COMPOSITION_END
                | messages::COMMIT
                | messages::EDIT_BEGIN
                | messages::EDIT_END
                | messages::STEP_UP
                | messages::STEP_DOWN
        )
    {
        if matches!(message, messages::STEP_UP | messages::STEP_DOWN)
            && (!state.local_edit.get()
                || windows::Win32::UI::Input::KeyboardAndMouse::GetFocus() != HWND(lp.0 as *mut _))
        {
            return Some(LRESULT(0));
        }
        if !state.closed.get() && state.host.borrow().is_some() {
            if let Err(error) = state
                .run(|host| host.edit_message(HWND(lp.0 as *mut _), message, (wp.0 >> 16) as u32))
            {
                *state.error.borrow_mut() = Some(error);
            }
        }
        return Some(LRESULT(0));
    }
    None
}
