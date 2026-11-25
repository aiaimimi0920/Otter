//! WinForms obscured push-button compensation; send one parent notification only after a successful ordinary mouse-up.
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
            Event::Wheel { .. } | Event::Leave => {
                self.gesture = None;
                None
            }
            Event::Key { .. } | Event::Character { .. } | Event::Hover { .. } => None,
        }
    }
}

fn push_button_class(name: &[u16], style: isize, enabled: bool) -> bool {
    const PREFIX: &[u8] = b"WINDOWSFORMS10.BUTTON.";
    enabled
        && matches!(style & 0x0f, 0 | 1 | 11)
        && name.get(..PREFIX.len()).is_some_and(|head| {
            head.iter().zip(PREFIX).all(|(&unit, &ascii)| {
                u8::try_from(unit).is_ok_and(|byte| byte.eq_ignore_ascii_case(&ascii))
            })
        })
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

/// Do not use BM_CLICK (which duplicates down/up); use only constrained BN_CLICKED/WM_COMMAND.
pub(super) fn complete(
    target: Destination,
    point: [i32; 2],
    modal: Option<&mut message::Modal>,
) -> Result<(), Error> {
    if !eligible(&target, point) {
        return Ok(());
    }
    let parent = unsafe { GetParent(target.hwnd()) }.map_err(|_| Error::InvalidDelivery)?;
    let parent = Destination::bind(target.root(), parent).map_err(Error::Target)?;
    if parent.thread() == unsafe { GetCurrentThreadId() } {
        return Err(Error::Message(message::Error::SameThread));
    }
    // After binding the parent window, recheck the original child target and parent-child relationship; a handle alone is not an authorization credential.
    target.preflight().map_err(Error::Target)?;
    parent.preflight().map_err(Error::Target)?;
    if unsafe { GetParent(target.hwnd()) }.ok() != Some(parent.hwnd()) {
        return Err(Error::InvalidDelivery);
    }
    if unsafe { IsIconic(target.root().hwnd()) }.as_bool() {
        return Err(Error::Message(message::Error::SourceTransition));
    }
    let id = unsafe { GetDlgCtrlID(target.hwnd()) } as u16;
    if let Some(modal) = modal {
        // Notifications may also open modal windows. Retain the original parent target for Escape/cancellation without replaying the notification.
        return message::button_command(&parent, &target, id, modal).map_err(Error::Message);
    }
    let completed = unsafe {
        SetLastError(ERROR_SUCCESS);
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
        let error = unsafe { GetLastError() }.0;
        eprintln!(
            "native-live-message-failed: message=0x{WM_COMMAND:04X} win32={error} timeout_ms=250"
        );
        Err(Error::Message(message::Error::DeliveryFailed))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn class_prefix_matches_legacy_ascii_case_rules_without_decoding_suffix() {
        let mut name: Vec<u16> = "wInDoWsFoRmS10.bUtToN.".encode_utf16().collect();
        name.extend([0xd800, 0x4e2d]);
        assert!(push_button_class(&name, 0, true));
        for unit in [0, 0x017f, 0xff37, 0xd800] {
            name[0] = unit;
            assert!(!push_button_class(&name, 0, true));
        }
    }
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
