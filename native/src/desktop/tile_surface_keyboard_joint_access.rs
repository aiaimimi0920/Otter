//! 只给隔离联合测试调用自有HWND；环境开关不编译进产品，不向外部应用注入输入。
use super::*;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyboardState, SetKeyboardState, VIRTUAL_KEY, VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT,
    VK_LWIN, VK_MENU, VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT,
};
struct ThreadKeys([u8; 256]);
impl Drop for ThreadKeys {
    fn drop(&mut self) {
        unsafe {
            let _ = SetKeyboardState(&self.0);
        }
    }
}
/// 只隔离当前测试线程的修饰键快照；不注入全局输入，正常/展开退出均恢复全部字节。
pub(super) fn with_modifiers<T>(held: &[VIRTUAL_KEY], action: impl FnOnce() -> T) -> T {
    unsafe {
        let mut keys = [0; 256];
        GetKeyboardState(&mut keys).unwrap();
        let _restore = ThreadKeys(keys);
        for key in [
            VK_CONTROL,
            VK_LCONTROL,
            VK_RCONTROL,
            VK_MENU,
            VK_LMENU,
            VK_RMENU,
            VK_SHIFT,
            VK_LSHIFT,
            VK_RSHIFT,
            VK_LWIN,
            VK_RWIN,
        ] {
            keys[key.0 as usize] = 0;
        }
        for key in held {
            keys[key.0 as usize] |= 0x80;
        }
        SetKeyboardState(&keys).unwrap();
        action()
    }
}
unsafe fn key(hwnd: HWND, key: usize) {
    SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(key)), None);
    SendMessageW(hwnd, WM_KEYUP, Some(WPARAM(key)), None);
}
impl Window {
    pub(crate) fn test_art_keyboard_queue(&self) -> serde_json::Value {
        let ready = actions::active(&self.state).is_some();
        let owns_focus = unsafe { GetFocus() == self.hwnd };
        let modifiers = actions::modifiers();
        let s = self.state.shared.lock();
        serde_json::json!({"notice":s.actions.notice,"dispatch":s.actions.test_dispatch,
            "paintedReady":ready,"actualRootFocus":owns_focus,
            "modifiers":{"control":modifiers.ctrl_key,"alt":modifiers.alt_key,
                "shift":modifiers.shift_key,"meta":modifiers.meta_key},
            "focus":self.state.keyboard.borrow().focus.as_ref().map(|f|&f.node),
            "pending":s.actions.pending.iter().map(|c|serde_json::json!({
                "kind":match &c.kind {crate::tile_surface_actions::Kind::Event(e)=>e.clone(),_=>serde_json::Value::Null},
                "edit":c.edit.as_ref().map(|e|serde_json::json!({"node":e.node,"active":e.active()}))
            })).collect::<Vec<_>>()})
    }
    pub(crate) fn test_art_text_keyboard_submit(&self) -> bool {
        with_modifiers(&[], || {
            let child = text::active_child(&self.state);
            unsafe {
                key(child, 9);
                assert!(
                    self.state
                        .keyboard
                        .borrow()
                        .focus
                        .as_ref()
                        .is_some_and(|f| f.node == "send"),
                    "Tab did not reach send: {}",
                    self.test_art_keyboard_queue()
                );
                // Tab自己可能新增change；只有Enter之后新增的意图才是按钮已受理证明。
                let before = self.state.shared.lock().actions.test_offered;
                let prior = self.test_art_keyboard_queue();
                key(self.hwnd, 13);
                let accepted = self.state.shared.lock().actions.test_offered > before;
                if !accepted {
                    eprintln!(
                        "tile-keyboard-submit-rejected: before={prior}, after={}",
                        self.test_art_keyboard_queue()
                    );
                }
                accepted
            }
        })
    }
    pub(crate) fn test_art_keyboard_switch(&self, pixel: [i32; 2]) -> bool {
        let before = self.state.shared.lock().actions.test_offered;
        pointer(self.hwnd, &self.state, pixel);
        unsafe {
            let _ = SetFocus(Some(self.hwnd));
            key(self.hwnd, 32);
        }
        self.state.shared.lock().actions.test_offered > before
    }
    pub(crate) fn test_art_keyboard_select(&self, selected: Option<usize>) -> bool {
        let before = self.state.shared.lock().actions.test_offered;
        // 从当前逻辑焦点沿真实Tab序到Select，不伪造CBN_SELCHANGE。
        for _ in 0..3 {
            if self
                .state
                .keyboard
                .borrow()
                .focus
                .as_ref()
                .is_some_and(|f| f.kind == Kind::Select)
                && self.state.keyboard_value.borrow().is_some()
            {
                break;
            }
            navigate(self.hwnd, &self.state, false);
        }
        let h = self
            .state
            .keyboard_value
            .borrow()
            .as_ref()
            .unwrap()
            .child()
            .unwrap();
        unsafe {
            key(
                h,
                match selected {
                    None => 27,
                    Some(0) => 36,
                    Some(1) => 35,
                    _ => panic!("unsupported oracle index"),
                },
            );
        }
        self.state.shared.lock().actions.test_offered > before
    }
    pub(crate) fn test_art_keyboard_slider_end(&self) {
        actions::slider::cancel(self.hwnd, &self.state);
        navigate(self.hwnd, &self.state, false);
        let h = self
            .state
            .keyboard_value
            .borrow()
            .as_ref()
            .unwrap()
            .child()
            .unwrap();
        unsafe {
            key(h, 35);
        }
    }
}
