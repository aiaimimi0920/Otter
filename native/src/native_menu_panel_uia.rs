//! Panel UIA provider/notification bridge; after notification reentry, recheck selected-window/capture/foreground prerequisites.
use super::*;
use windows::Win32::UI::{
    Accessibility::{IRawElementProviderSimple, UiaClientsAreListening},
    Input::KeyboardAndMouse::GetCapture,
};
pub(super) fn provider(data: &Data, hwnd: HWND) -> Option<IRawElementProviderSimple> {
    if data.destroyed.get() {
        return None;
    }
    if data.provider.borrow().is_none() {
        if let Ok(provider) =
            super::super::uia::create(hwnd, data.accessibility.view(), data.row_height)
        {
            *data.provider.borrow_mut() = Some(provider);
        }
    }
    data.provider.borrow().clone()
}
impl Panel {
    pub(crate) fn notify_accessibility(&self) {
        if !self.alive() {
            self.data.notifications.borrow_mut().take();
            return;
        }
        let listening = unsafe { UiaClientsAreListening() }.as_bool();
        let changes = super::super::uia::changes(
            &self.data.accessibility.view(),
            &mut self.data.notifications.borrow_mut(),
            listening,
        );
        if !changes.is_empty() {
            if let Some(provider) = provider(&self.data, self.hwnd) {
                super::super::uia::notify(&provider, changes);
            }
        }
    }
    pub(crate) fn notify_invoked(&self, slot: usize) -> bool {
        let Some(node) = self.data.accessibility.view().node(slot) else {
            return false;
        };
        if !node
            .with_entry(|entry, _| entry.enabled && !entry.separator && !entry.submenu)
            .unwrap_or(false)
        {
            return false;
        }
        self.notification_guard(|| {
            if unsafe { UiaClientsAreListening() }.as_bool() {
                if let Some(provider) = provider(&self.data, self.hwnd) {
                    super::super::uia::notify_invoked(&provider, node);
                }
            }
        })
    }
    pub(super) fn notification_guard(&self, notify: impl FnOnce()) -> bool {
        let ready = || {
            self.alive()
                && !self.data.failed.get()
                && !self
                    .data
                    .events
                    .borrow()
                    .iter()
                    .any(|event| matches!(event, Event::Cancel | Event::Dpi))
        };
        if !ready() {
            return false;
        }
        let foreground = unsafe { GetForegroundWindow() };
        let capture = unsafe { GetCapture() };
        notify();
        ready() && unsafe { GetForegroundWindow() == foreground && GetCapture() == capture }
    }
}
