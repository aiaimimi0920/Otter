//! 父窗口实际持焦时才接收滚动键；不截获 EDIT 光标键或带修饰键的快捷操作。
use super::*;
pub(super) unsafe fn message(hwnd: HWND, state: &State, message: u32, key: u16) -> bool {
    use crate::surface_scroll::Command;
    let command = match key {
        k if k == VK_LEFT.0 => Command::Left,
        k if k == VK_RIGHT.0 => Command::Right,
        k if k == VK_UP.0 => Command::Up,
        k if k == VK_DOWN.0 => Command::Down,
        k if k == VK_PRIOR.0 => Command::PageUp,
        k if k == VK_NEXT.0 => Command::PageDown,
        k if k == VK_HOME.0 => Command::Home,
        k if k == VK_END.0 => Command::End,
        _ => return false,
    };
    let modifiers = pointer::modifiers();
    if modifiers.alt_key || modifiers.ctrl_key || modifiers.meta_key || modifiers.shift_key {
        return false;
    }
    // VK 与字符码有数值重叠，WM_CHAR 不能据此当作滚动命令。
    if message != WM_KEYDOWN {
        return message == WM_KEYUP;
    }
    let anchor = state.keyboard.borrow().anchor.clone();
    // 保留逻辑焦点和导航锚点，只废弃旧按压/待发动作和鼠标滚动手势。
    pointer::invalidate_candidates(hwnd, state);
    if state.closed.get() || !state.local_edit.get() || GetFocus() != hwnd {
        return true;
    }
    let mut changed = false;
    let result = state.run(|host| {
        changed = host.scroll_key(anchor.as_deref(), command)?;
        Ok(())
    });
    if let Err(error) = result {
        *state.error.borrow_mut() = Some(error);
        let _ = InvalidateRect(Some(hwnd), None, false);
    } else if changed {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
    true
}
