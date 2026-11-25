//! Focus notifications observe existing owners only; defer one message, hold no borrow across UIA, and let system EDIT notify itself.
use super::*;
use accessibility_bridge::focus::{self, Focus};
use windows::Win32::UI::Accessibility::*;
pub(super) const MESSAGE: u32 = WM_APP + 0x55b;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Identity {
    Root,
    Node([i32; 3]),
}
#[derive(Default)]
pub(super) struct Pending {
    dirty: Cell<bool>,
    posted: Cell<bool>,
    flushing: Cell<bool>,
    last: Cell<Option<Identity>>,
}
pub(super) fn lost(state: &State) {
    state.focus_notifications.last.set(None);
}
pub(super) fn schedule(state: &State) {
    if state.closed.get() || state.uia.borrow().is_none() {
        return;
    }
    state.focus_notifications.dirty.set(true);
    resume(state);
}
pub(super) fn resume(state: &State) {
    let pending = &state.focus_notifications;
    if state.closed.get() || !pending.dirty.get() || pending.posted.get() || pending.flushing.get()
    {
        return;
    }
    let hwnd = HWND(state.accessibility_key.get() as *mut _);
    if unsafe { PostMessageW(Some(hwnd), MESSAGE, WPARAM(0), LPARAM(0)) }.is_ok() {
        pending.posted.set(true);
    }
}
pub(super) fn flush(hwnd: HWND, state: &State) {
    flush_with(hwnd, state, |provider| unsafe {
        if UiaClientsAreListening().as_bool() {
            let _ = UiaRaiseAutomationEvent(provider, UIA_AutomationFocusChangedEventId);
        }
    });
}
fn flush_with(hwnd: HWND, state: &State, mut notify: impl FnMut(&IRawElementProviderSimple)) {
    let pending = &state.focus_notifications;
    pending.posted.set(false);
    if state.closed.get() || pending.flushing.get() || !pending.dirty.get() {
        return;
    }
    if state.host.borrow().is_none() {
        return; // The outer State.run reschedules after restoring Host; do not spin private messages.
    }
    pending.flushing.set(true);
    let _guard = Flushing(state);
    pending.dirty.set(false);
    let scene = state
        .accessibility
        .borrow()
        .as_ref()
        .and_then(|owner| owner.root());
    let Ok(focused) = focus::get(hwnd, scene.as_ref()) else {
        pending.last.set(None);
        return;
    };
    let (identity, node) = match focused {
        Focus::None => {
            pending.last.set(None);
            return;
        }
        Focus::Root => (Identity::Root, None),
        Focus::Node(handle) => {
            if handle.read().is_some_and(|entry| {
                matches!(
                    entry.kind,
                    crate::surface_scene::Kind::Input
                        | crate::surface_scene::Kind::Textarea
                        | crate::surface_scene::Kind::Number
                )
            }) {
                // Root queries return hosting nodes; actual EDIT focus events remain with system providers to avoid duplicate sources.
                pending.last.set(None);
                return;
            }
            let Some(id) = handle.runtime_id() else {
                return;
            };
            (Identity::Node(id), Some(handle))
        }
    };
    if pending.last.get() == Some(identity) {
        return;
    }
    let provider = state.uia.borrow().as_ref().and_then(|root| {
        node.map_or_else(|| Some(root.provider()), |handle| root.node(handle).ok())
    });
    if let Some(provider) = provider {
        // Save identity before calling: reentrant callbacks can schedule only the next round, never send stale focus backward.
        pending.last.set(Some(identity));
        notify(&provider);
    }
}
struct Flushing<'a>(&'a State);
impl Drop for Flushing<'_> {
    fn drop(&mut self) {
        self.0.focus_notifications.flushing.set(false);
        resume(self.0);
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-surface")))]
#[path = "surface_window_focus_notification_tests.rs"]
mod tests;
