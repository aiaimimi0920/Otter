//! One private message coalesces Switch/Slider notifications; sending crosses no Host/RefCell borrow.
use super::*;
use crate::surface_accessibility::Handle;
use std::collections::BTreeMap;
use windows::Win32::UI::Accessibility::IRawElementProviderSimple;
#[path = "surface_window_range_readonly.rs"]
pub(super) mod readonly;
pub(super) const MESSAGE: u32 = WM_APP + 0x55a;
#[derive(Default)]
pub(super) struct Pending {
    values: BTreeMap<String, (Handle, bool, bool)>,
    ranges: BTreeMap<String, (Handle, f64, f64)>,
    readonly: BTreeMap<String, (Handle, bool, bool)>,
    posted: bool,
    flushing: bool,
}
impl Pending {
    pub(super) fn clear(&mut self) {
        self.values.clear();
        self.ranges.clear();
        self.readonly.clear();
        // Already queued messages still arrive; retain posted to avoid accumulating empty messages during repeated replacement.
    }
}
pub(super) fn enqueue(state: &State, changes: Vec<(String, bool, bool)>) {
    if state.closed.get() || state.uia.borrow().is_none() {
        return;
    }
    let mut pending = state.notifications.borrow_mut();
    for (id, before, after) in changes {
        let Some(handle) = state
            .accessibility
            .borrow()
            .as_ref()
            .and_then(|owner| owner.node(&id))
        else {
            continue;
        };
        let first = pending
            .values
            .get(&id)
            .filter(|entry| entry.0.runtime_id() == handle.runtime_id())
            .map_or(before, |entry| entry.1);
        if first == after {
            pending.values.remove(&id);
        } else {
            pending.values.insert(id, (handle, first, after));
        }
    }
    schedule(state, &mut pending);
}
pub(super) fn enqueue_ranges(state: &State, changes: Vec<(String, f64, f64)>) {
    if state.closed.get() || state.uia.borrow().is_none() {
        return;
    }
    let mut pending = state.notifications.borrow_mut();
    for (id, before, after) in changes {
        let Some(handle) = state
            .accessibility
            .borrow()
            .as_ref()
            .and_then(|owner| owner.node(&id))
        else {
            continue;
        };
        let first = pending
            .ranges
            .get(&id)
            .filter(|entry| entry.0.runtime_id() == handle.runtime_id())
            .map_or(before, |entry| entry.1);
        if first == after {
            pending.ranges.remove(&id);
        } else {
            pending.ranges.insert(id, (handle, first, after));
        }
    }
    schedule(state, &mut pending);
}
fn schedule(state: &State, pending: &mut Pending) {
    if (!pending.values.is_empty() || !pending.ranges.is_empty() || !pending.readonly.is_empty())
        && !pending.posted
        && !pending.flushing
    {
        let hwnd = HWND(state.accessibility_key.get() as *mut _);
        // PostMessage does not execute receivers synchronously; never substitute SendMessage/immediate Raise.
        if unsafe { PostMessageW(Some(hwnd), MESSAGE, WPARAM(0), LPARAM(0)) }.is_ok() {
            pending.posted = true;
        } else {
            pending.clear();
        }
    }
}
pub(super) fn flush(_hwnd: HWND, state: &State) {
    flush_all(state, uia::notify_toggle, uia::notify_range);
}
#[cfg(test)]
fn flush_with(state: &State, mut notify: impl FnMut(&IRawElementProviderSimple, bool, bool)) {
    flush_all(state, &mut notify, |_, _, _| {});
}
fn flush_all(
    state: &State,
    notify: impl FnMut(&IRawElementProviderSimple, bool, bool),
    range: impl FnMut(&IRawElementProviderSimple, f64, f64),
) {
    flush_properties(state, notify, range, uia::notify_range_readonly);
}
fn flush_properties(
    state: &State,
    mut notify: impl FnMut(&IRawElementProviderSimple, bool, bool),
    mut range: impl FnMut(&IRawElementProviderSimple, f64, f64),
    readonly_notify: impl FnMut(&IRawElementProviderSimple, bool, bool),
) {
    state.notifications.borrow_mut().posted = false;
    if state.notifications.borrow().flushing {
        return;
    }
    if state.closed.get() {
        state.notifications.borrow_mut().clear();
        return;
    }
    // Nested message loops may dispatch private messages during Host operations; let outer run requeue after completion.
    if state.host.borrow().is_none() {
        return;
    }
    state.notifications.borrow_mut().flushing = true;
    let _flushing = Flushing(state);
    let values = std::mem::take(&mut state.notifications.borrow_mut().values);
    let ranges = std::mem::take(&mut state.notifications.borrow_mut().ranges);
    let readonly = std::mem::take(&mut state.notifications.borrow_mut().readonly);
    for (id, (handle, before, after)) in values {
        if !accessibility_bridge::current(state, &handle) {
            continue;
        }
        // Earlier notifications may reentrantly modify later nodes; never report an old batch's values after newer state.
        if state
            .host
            .borrow()
            .as_ref()
            .and_then(|host| host.switch_value(&id))
            != Some(after)
        {
            continue;
        }
        let provider = state
            .uia
            .borrow()
            .as_ref()
            .and_then(|root| root.node(handle).ok());
        if let Some(provider) = provider {
            notify(&provider, before, after);
        }
    }
    for (id, (handle, before, after)) in ranges {
        if !accessibility_bridge::current(state, &handle)
            || state
                .host
                .borrow()
                .as_ref()
                .and_then(|host| host.slider_value(&id))
                != Some(after)
        {
            continue;
        }
        let provider = state
            .uia
            .borrow()
            .as_ref()
            .and_then(|root| root.node(handle).ok());
        if let Some(provider) = provider {
            range(&provider, before, after);
        }
    }
    readonly::flush(state, readonly, readonly_notify);
}
#[cfg(test)]
#[path = "surface_window_range_notification_tests.rs"]
mod range_tests;
#[cfg(test)]
#[path = "surface_window_notification_tests.rs"]
mod tests;
struct Flushing<'a>(&'a State);
impl Drop for Flushing<'_> {
    fn drop(&mut self) {
        self.0.notifications.borrow_mut().flushing = false;
        enqueue(self.0, Vec::new());
    }
}
