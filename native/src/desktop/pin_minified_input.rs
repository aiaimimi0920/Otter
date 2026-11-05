//! 与主干 interactionEnabled 一致：缩略不启用标注交互，生命周期和双击展开仍可路由。
use super::State;
use windows::Win32::{
    Foundation::*,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

pub(in crate::desktop) unsafe fn dispatch(
    state: &State,
    message: u32,
    wp: WPARAM,
) -> Option<LRESULT> {
    state.canvas.borrow().minified.as_ref()?;
    match message {
        WM_NCHITTEST => Some(LRESULT(HTCAPTION as isize)),
        WM_LBUTTONDOWN | WM_LBUTTONUP | WM_MOUSEMOVE => Some(LRESULT(0)),
        WM_KEYDOWN if blocks_key(wp.0, GetKeyState(VK_CONTROL.0 as i32) < 0) => Some(LRESULT(0)),
        _ => None,
    }
}
fn blocks_key(key: usize, control: bool) -> bool {
    key != VK_ESCAPE.0 as usize && !(control && matches!(key, 0x43 | 0x53 | 0x4F | 0x56))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keyboard_edit_commands_are_blocked_but_copy_export_and_escape_remain() {
        for key in [0x5A, 0x59, 0x44, 0x0D] {
            assert!(blocks_key(key, true));
        }
        for key in [0x2E, 0x41, 0x54, 0xBB, 0xBD, VK_F2.0 as usize] {
            assert!(blocks_key(key, false));
        }
        assert!(!blocks_key(0x43, true));
        assert!(!blocks_key(0x53, true));
        assert!(!blocks_key(0x4F, true));
        assert!(!blocks_key(0x56, true));
        assert!(!blocks_key(VK_ESCAPE.0 as usize, false));
    }
}
