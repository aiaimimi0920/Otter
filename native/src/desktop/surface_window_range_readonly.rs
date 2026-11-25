//! Slider read-only permission deltas; reuse existing single-message scheduling/refresh guards without independent message or state ownership.
use super::*;
pub(in crate::desktop::surface_window) fn enqueue(
    state: &State,
    changes: Vec<(String, bool, bool)>,
) {
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
            .readonly
            .get(&id)
            .filter(|entry| entry.0.runtime_id() == handle.runtime_id())
            .map_or(before, |entry| entry.1);
        if first == after {
            pending.readonly.remove(&id);
        } else {
            pending.readonly.insert(id, (handle, first, after));
        }
    }
    schedule(state, &mut pending);
}
pub(super) fn flush(
    state: &State,
    values: BTreeMap<String, (Handle, bool, bool)>,
    mut notify: impl FnMut(&IRawElementProviderSimple, bool, bool),
) {
    for (id, (handle, before, after)) in values {
        if !accessibility_bridge::current(state, &handle)
            || state
                .host
                .borrow()
                .as_ref()
                .and_then(|host| host.slider_readonly(&id))
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
}
