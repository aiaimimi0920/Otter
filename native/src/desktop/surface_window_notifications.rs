//! 单个私有消息合并 Switch/Slider 通知；发送不跨 Host/RefCell 借用。
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
        // 已排队消息仍会抵达；保留 posted 以免连续替换累积空消息。
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
        // PostMessage 不同步执行接收过程；不能替换为 SendMessage/立即 Raise。
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
    // 嵌套消息循环可能在 Host 操作中分派私有消息；交给外层 run 结束重新排队。
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
        // 前一通知可重入修改后续节点；不在较新状态之后再报告旧批次的值。
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
