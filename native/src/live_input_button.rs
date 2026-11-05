//! WinForms 遮挡 push button 补偿；只在成功的普通 mouse-up 后发送一次父通知。
use super::*;
use crate::{live_button_gesture::Gesture, live_input::Button};
use windows::Win32::{
    Foundation::*,
    Graphics::Gdi::ClientToScreen,
    System::Threading::GetCurrentThreadId,
    UI::{HiDpi::*, Input::KeyboardAndMouse::IsWindowEnabled, WindowsAndMessaging::*},
};

#[derive(Default)]
pub(super) struct Tracker {
    gesture: Option<(Destination, Gesture<usize>)>,
    buttons: u8,
}
impl Tracker {
    pub(super) fn clear(&mut self) {
        self.gesture = None;
        self.buttons = 0;
    }
    pub(super) fn track(
        &mut self,
        target: &Destination,
        event: Event,
        point: [i32; 2],
        buttons: u8,
    ) -> Option<Destination> {
        let previous = self.buttons;
        self.buttons = buttons;
        match event {
            Event::Button {
                button, down: true, ..
            } => {
                self.gesture = None;
                if button == Button::Left && previous == 0 && eligible(target, point) {
                    let dpi = unsafe { GetDpiForWindow(target.hwnd()) }.max(96);
                    let threshold = [SM_CXDRAG, SM_CYDRAG].map(|metric| {
                        (unsafe { GetSystemMetricsForDpi(metric, dpi) }.max(2) / 2) as u32
                    });
                    self.gesture = Gesture::begin(
                        target.hwnd().0 as usize,
                        point,
                        threshold,
                        button,
                        previous,
                        true,
                    )
                    .map(|gesture| (target.clone(), gesture));
                }
                None
            }
            Event::Move { .. } => {
                if let Some((original, gesture)) = &mut self.gesture {
                    if !original.same_target(target) {
                        gesture.cancel();
                    }
                    gesture.observe(&(target.hwnd().0 as usize), point);
                }
                None
            }
            Event::Button {
                button,
                down: false,
                ..
            } => {
                let (original, gesture) = self.gesture.take()?;
                let valid = original.same_target(target) && eligible(target, point);
                gesture.finish(&(target.hwnd().0 as usize), point, button, valid)?;
                Some(original)
            }
            Event::Wheel { .. } => {
                self.gesture = None;
                None
            }
            Event::Key { .. } => None,
        }
    }
}

fn push_button_class(name: &[u16], style: isize, enabled: bool) -> bool {
    enabled
        && matches!(style & 0x0f, 0 | 1 | 11)
        && String::from_utf16_lossy(name)
            .to_ascii_uppercase()
            .starts_with("WINDOWSFORMS10.BUTTON.")
}
fn eligible(target: &Destination, point: [i32; 2]) -> bool {
    if target.preflight().is_err() {
        return false;
    }
    let Ok(_dpi) = coordinates::Dpi::enter(target.hwnd()) else {
        return false;
    };
    unsafe {
        let window = target.hwnd();
        let mut name = [0u16; 256];
        let length = GetClassNameW(window, &mut name);
        if length <= 0
            || !push_button_class(
                &name[..length as usize],
                GetWindowLongPtrW(window, GWL_STYLE),
                IsWindowEnabled(window).as_bool(),
            )
        {
            return false;
        }
        let mut rect = RECT::default();
        let mut point = POINT {
            x: point[0],
            y: point[1],
        };
        GetClientRect(window, &mut rect).is_ok()
            && point.x >= 0
            && point.y >= 0
            && point.x < rect.right
            && point.y < rect.bottom
            && ClientToScreen(window, &mut point).as_bool()
            && WindowFromPoint(point) != window
    }
}

/// 不用 BM_CLICK（会重复产生 down/up）；只使用受限的 BN_CLICKED/WM_COMMAND。
pub(super) fn complete(target: Destination, point: [i32; 2]) -> Result<(), Error> {
    if !eligible(&target, point) {
        return Ok(());
    }
    let parent = unsafe { GetParent(target.hwnd()) }.map_err(|_| Error::InvalidDelivery)?;
    let parent = Destination::bind(target.root(), parent).map_err(Error::Target)?;
    if parent.thread() == unsafe { GetCurrentThreadId() } {
        return Err(Error::Message(message::Error::SameThread));
    }
    // 绑定父窗口之后再复核原始子目标及父子关系；句柄本身不是权限凭证。
    target.preflight().map_err(Error::Target)?;
    parent.preflight().map_err(Error::Target)?;
    if unsafe { GetParent(target.hwnd()) }.ok() != Some(parent.hwnd())
        || unsafe { IsIconic(target.root().hwnd()) }.as_bool()
    {
        return Err(Error::InvalidDelivery);
    }
    let id = unsafe { GetDlgCtrlID(target.hwnd()) } as u16;
    let completed = unsafe {
        SendMessageTimeoutW(
            parent.hwnd(),
            WM_COMMAND,
            WPARAM(usize::from(id)),
            LPARAM(target.hwnd().0 as isize),
            SMTO_ABORTIFHUNG | SMTO_BLOCK | SMTO_ERRORONEXIT,
            250,
            None,
        )
    };
    if completed.0 == 0 {
        Err(Error::Message(message::Error::DeliveryFailed))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_enabled_winforms_push_styles_are_eligible() {
        let name: Vec<u16> = "WindowsForms10.BUTTON.app.0.fixture"
            .encode_utf16()
            .collect();
        for style in 0..16 {
            assert_eq!(
                push_button_class(&name, style, true),
                matches!(style, 0 | 1 | 11)
            );
            assert!(!push_button_class(&name, style, false));
        }
        for name in [
            "BUTTON",
            "WindowsForms10.EDIT.app.0",
            "WindowsForms10.BUTTON",
        ] {
            assert!(!push_button_class(
                &name.encode_utf16().collect::<Vec<_>>(),
                0,
                true
            ));
        }
    }
}
