//! 在 B 跟踪真实悬停/离开，再由原输入 worker 转发到 A；不操作全局指针。
use super::*;
use windows::Win32::UI::Controls::HOVER_DEFAULT;
use windows::Win32::UI::Input::KeyboardAndMouse::*;

#[derive(Default)]
pub(super) struct Tracking {
    armed: bool,
}
impl Tracking {
    pub(super) unsafe fn observe(&mut self, hwnd: HWND, message: u32) -> Result<()> {
        if matches!(message, WM_MOUSEHOVER | WM_MOUSELEAVE) {
            self.armed = false;
        }
        if message == WM_MOUSEMOVE && !self.armed {
            TrackMouseEvent(&mut TRACKMOUSEEVENT {
                cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                dwFlags: TME_HOVER | TME_LEAVE,
                hwndTrack: hwnd,
                dwHoverTime: HOVER_DEFAULT,
            })?;
            self.armed = true;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabling_control_resets_tracking_even_if_leave_is_not_forwarded() {
        let mut input = Control::default();
        input.tracking.armed = true;
        input.cancel();
        assert!(
            !input.tracking.armed,
            "next move must register tracking again"
        );
    }
    #[test]
    fn repeated_letterbox_leave_does_not_exhaust_input_queue() {
        let mut input = Control::default();
        for _ in 0..32 {
            assert!(input.push(Event::Leave));
        }
        assert_eq!(input.events.len(), 1);
    }
}
