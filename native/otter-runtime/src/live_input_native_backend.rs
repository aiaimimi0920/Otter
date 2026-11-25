//! Capture coordinates and synchronous motion entry point for ordinary applications; browsers, remote control, and existing modals retain their original paths.
use super::*;
impl Backend {
    pub(super) fn ignore_inactive_motion(&mut self, event: Event) -> bool {
        // Ordinary applications disable the original window when opening a modal dialog. Consume only new pointer input without edges,
        // without synchronous motion to a root changing windows or discarding the original held up/cleanup.
        let ignore = self.local
            && matches!(
                event,
                Event::Move { .. } | Event::Hover { .. } | Event::Wheel { .. }
            )
            && !unsafe {
                windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(self.root.hwnd())
            }
            .as_bool()
            && !crate::live_browser_scroll::eligible(&self.root);
        if ignore {
            self.last_delivery = live_input::Delivery::IgnoredWhileModal;
        }
        ignore
    }
    pub fn with_hover(mut self, owner: Option<crate::live_hover_owner::Control>) -> Self {
        self.hover_owner = owner;
        self
    }
    pub(super) fn check_control(&mut self) -> Result<(), Error> {
        if let Some(control) = self.hover_owner {
            if !self.local {
                return Err(Error::InvalidDelivery);
            }
            if !control.identity().valid() {
                return Err(Error::NativePopup(
                    "Live controller identity changed".into(),
                ));
            }
            if !control.valid() {
                // Revocation may silently end only new operations known not to have been sent; it must not hide unknown outcomes of an original in-flight modal.
                self.modal.check_revoked_control().map_err(Error::Message)?;
                return Err(Error::ControlRevoked);
            }
        }
        Ok(())
    }
    pub(super) fn map_point(&self, target: &Destination, point: [f64; 2]) -> Result<Mapped, Error> {
        let map = if self.local && !crate::live_browser_scroll::eligible(&self.root) {
            coordinates::map_capture
        } else {
            coordinates::map
        };
        map(&self.root, target, point, self.region).map_err(Error::Coordinates)
    }
    pub(super) fn prepare_native(&mut self) -> Result<(), Error> {
        self.check_control()?;
        if let Some(control) = self.hover_owner {
            let owner = control.identity();
            if self.hover_binding.is_none() {
                self.hover_binding = Some(
                    crate::live_hover_binding::Binding::new_native(&self.root, owner)
                        .map_err(|e| Error::NativePopup(e.to_string()))?,
                );
            }
            // Native hooks also include system Leave filtering; installing without publishing a binding cannot preserve hover.
            // activate rechecks the B containing the real mouse; late samples must not take another mirror's slot.
            self.hover_binding
                .as_ref()
                .unwrap()
                .activate()
                .map_err(|e| Error::NativePopup(e.to_string()))?;
            self.modal.set_owner(control);
        }
        Ok(())
    }
    pub(super) fn local_button_modal(&self, event: Event) -> bool {
        use live_input::Button;
        // Only left/right button releases and left-button compensation need this decision; retain live classification for each relevant message.
        self.local
            && matches!(event, Event::Button { button, down, .. }
                if button == Button::Left || (button == Button::Right && !down))
            && !crate::live_browser_scroll::eligible(&self.root)
    }
    pub(super) fn send_native_motion(
        &mut self,
        target: &Destination,
        packet: Message,
        event: Event,
        timeout: u32,
    ) -> Result<(), Error> {
        let native = self.local
            && matches!(event, Event::Move { .. } | Event::Hover { .. })
            && self.hover_owner.is_some()
            && self.modal.escape_target().is_none()
            && !crate::live_browser_scroll::eligible(&self.root)
            && target.thread()
                == unsafe {
                    windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(
                        self.root.hwnd(),
                        None,
                    )
                };
        if native {
            self.prepare_native()?;
            message::motion_native(target, packet, timeout, &mut self.modal)
        } else {
            message::send_to_with_timeout(target, packet, timeout)
        }
        .map_err(Error::Message)
    }
}
#[cfg(test)]
#[path = "live_input_control_epoch_tests.rs"]
mod epoch_tests;
#[cfg(test)]
#[path = "live_input_native_hover_tests.rs"]
mod hover_tests;
