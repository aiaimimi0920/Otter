//! 按钮键盘状态与 HWND 焦点绑定；候选仍走共享单槽及场景身份验证，不直接派发动作。
use super::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
#[path = "surface_window_scroll_key.rs"]
mod scroll_key;

#[derive(Default)]
pub(super) struct Keyboard {
    focused: Option<String>,
    visible: bool,
    space: Option<String>,
    anchor: Option<String>,
}
impl Keyboard {
    pub(super) fn clear(&mut self) {
        self.focused = None;
        self.visible = false;
        self.space = None;
        self.anchor = None;
    }
    pub(super) fn focused(&self) -> Option<&str> {
        self.focused.as_deref()
    }
    pub(super) fn anchor(&self) -> Option<&str> {
        self.anchor.as_deref()
    }
    pub(super) fn indicator(&self) -> Option<&str> {
        self.focused.as_deref().filter(|_| self.visible)
    }
    pub(super) fn pressed(&self) -> Option<&str> {
        self.space
            .as_deref()
            .filter(|id| Some(*id) == self.focused())
    }
    pub(super) fn cancel_press(&mut self) {
        self.space = None;
    }
}
pub(super) fn set_focus(state: &State, focused: Option<String>) {
    let mut keyboard = state.keyboard.borrow_mut();
    keyboard.anchor = focused.clone();
    keyboard.focused = focused;
    keyboard.visible = true;
    keyboard.space = None;
    drop(keyboard);
    focus_notifications::schedule(state);
}
pub(super) unsafe fn focus_node(hwnd: HWND, state: &State, node: &str) {
    focus_node_inner(hwnd, state, node, false);
}
pub(super) unsafe fn focus_canvas_node(hwnd: HWND, state: &State, node: &str) {
    // 同 HWND 的下一 Canvas 手势不能撤销原 FIFO 中上一手势的 down/up。
    focus_node_inner(hwnd, state, node, GetFocus() == hwnd);
}
unsafe fn focus_node_inner(hwnd: HWND, state: &State, node: &str, canvas: bool) {
    let mut eligible = false;
    let result = state.run(|host| {
        eligible = host.focusable(node)?;
        Ok(())
    });
    if !matches!(result, Ok(true)) || state.closed.get() {
        return;
    }
    // SetFocus 可重入；只在其返回且窗口仍存活时设置逻辑焦点。
    // 点击非文本背景也建立键盘滚动入口；系统 EDIT 自己处理其内部鼠标/编辑焦点。
    let _ = SetFocus(Some(hwnd));
    if state.closed.get() || !state.local_edit.get() || GetFocus() != hwnd {
        return;
    }
    if canvas {
        pointer::invalidate_soft(hwnd, state);
    } else {
        pointer::invalidate(hwnd, state);
    }
    if state.closed.get() || !state.local_edit.get() || GetFocus() != hwnd {
        return;
    }
    let mut keyboard = state.keyboard.borrow_mut();
    keyboard.space = None;
    keyboard.focused = eligible.then(|| node.to_owned());
    keyboard.anchor = Some(node.to_owned());
    drop(keyboard);
    focus_notifications::schedule(state);
}
/// 已验证的新场景可保留同一可聚焦节点，但按压/候选必须使用新场景身份。
pub(super) unsafe fn reconcile_scene(hwnd: HWND, state: &State, javascript: bool) {
    let (focused, visible) = {
        let keyboard = state.keyboard.borrow();
        (keyboard.focused.clone(), keyboard.visible)
    };
    let mut eligible = false;
    if let Some(id) = focused
        .as_deref()
        .filter(|_| state.local_edit.get() && GetFocus() == hwnd)
    {
        match state.run(|host| {
            eligible = host.focusable(id)?;
            Ok(())
        }) {
            Ok(true) => {}
            Err(error) => {
                *state.error.borrow_mut() = Some(error);
                eligible = false;
            }
            _ => eligible = false,
        }
    }
    eligible &= state.keyboard.borrow().focused == focused;
    if javascript {
        pointer::invalidate_soft(hwnd, state);
        state.keyboard.borrow_mut().clear();
    } else {
        pointer::invalidate(hwnd, state);
    }
    if eligible && !state.closed.get() && state.local_edit.get() && GetFocus() == hwnd {
        let mut keyboard = state.keyboard.borrow_mut();
        keyboard.focused = focused;
        keyboard.anchor = keyboard.focused.clone();
        keyboard.visible = visible;
    }
    focus_notifications::schedule(state);
}
pub(super) unsafe fn message(
    hwnd: HWND,
    state: &State,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> bool {
    if state.closed.get() || !state.local_edit.get() || GetFocus() != hwnd {
        return false;
    }
    let key = wp.0 as u16;
    if pointer::copy_message(hwnd, state, message, key, lp) {
        return true;
    }
    if pointer::text_key_message(hwnd, state, message, key) {
        return true;
    }
    if scroll_key::message(hwnd, state, message, key) {
        return true;
    }
    if ![VK_TAB.0, VK_RETURN.0, VK_SPACE.0].contains(&key) {
        return false;
    }
    let modifiers = pointer::modifiers();
    if modifiers.alt_key || modifiers.ctrl_key || modifiers.meta_key {
        state.keyboard.borrow_mut().space = None;
        let _ = InvalidateRect(Some(hwnd), None, false);
        return false;
    }
    if message == WM_CHAR {
        return true;
    }
    if message == WM_KEYDOWN && (lp.0 as usize & (1 << 30)) != 0 {
        return true;
    }
    if key == VK_TAB.0 {
        if message == WM_KEYDOWN {
            let current = state.keyboard.borrow().focused.clone();
            // 导航可能滚动几何或在 EDIT 重定位时失败；先撤销旧按压/候选。
            pointer::invalidate(hwnd, state);
            let mut next = None;
            match state.run(|host| {
                next = host.next_focus(current.as_deref(), modifiers.shift_key)?;
                Ok(())
            }) {
                Ok(true) if !state.closed.get() => {
                    if let Some(id) = &next {
                        if let Err(error) = state.run(|host| host.focus_edit(id).map(|_| ())) {
                            *state.error.borrow_mut() = Some(error);
                        }
                    }
                    set_focus(state, next);
                }
                Err(error) => *state.error.borrow_mut() = Some(error),
                _ => {}
            }
        }
        return true;
    }
    let focused = state.keyboard.borrow().focused.clone();
    if key == VK_SPACE.0 && message == WM_KEYDOWN {
        state.keyboard.borrow_mut().space = focused;
        let _ = InvalidateRect(Some(hwnd), None, false);
        return true;
    }
    let activate = if key == VK_RETURN.0 {
        message == WM_KEYDOWN
    } else {
        let pressed = state.keyboard.borrow_mut().space.take();
        pressed.is_some() && pressed == focused
    };
    if activate {
        if let Some(node) = focused {
            if switch::is_switch(state, &node) {
                // checkbox 使用 Space 释放激活；Enter 不伪装成 Button click。
                if key == VK_SPACE.0 && !state.pointer.borrow().has_pending() {
                    if let Some((values, fence)) = pointer::form_snapshot(state) {
                        if switch::activate(hwnd, state, &node) == Some(true)
                            && switch::has_click(state, &node)
                        {
                            state
                                .pointer
                                .borrow_mut()
                                .queue_keyboard(node, modifiers, values, fence);
                        }
                    }
                }
                return true;
            }
            if let Some((values, fence)) = pointer::form_snapshot(state) {
                state
                    .pointer
                    .borrow_mut()
                    .queue_keyboard(node, modifiers, values, fence);
            }
        }
    }
    let _ = InvalidateRect(Some(hwnd), None, false);
    true
}
