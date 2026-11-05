//! 场景替换使整棵声明式子树身份失效；在稳定 HWND 根合并报告 ChildrenInvalidated。
use super::*;
use windows::Win32::UI::Accessibility::*;
pub(super) const MESSAGE: u32 = WM_APP + 0x55c;
#[derive(Default)]
pub(super) struct Pending {
    dirty: Cell<bool>,
    posted: Cell<bool>,
    flushing: Cell<bool>,
}
pub(super) fn schedule(state: &State) {
    if state.closed.get() || state.uia.borrow().is_none() {
        return;
    }
    state.structure_notifications.dirty.set(true);
    resume(state);
}
pub(super) fn resume(state: &State) {
    let pending = &state.structure_notifications;
    if state.closed.get() || !pending.dirty.get() || pending.posted.get() || pending.flushing.get()
    {
        return;
    }
    let hwnd = HWND(state.accessibility_key.get() as *mut _);
    if unsafe { PostMessageW(Some(hwnd), MESSAGE, WPARAM(0), LPARAM(0)) }.is_ok() {
        pending.posted.set(true);
    }
}
pub(super) fn flush(state: &State) {
    flush_with(state, |provider| unsafe {
        if UiaClientsAreListening().as_bool() {
            // 非 ChildRemoved 不传 runtime-id；根 provider 的身份不随场景更换。
            let _ = UiaRaiseStructureChangedEvent(
                provider,
                StructureChangeType_ChildrenInvalidated,
                std::ptr::null_mut(),
                0,
            );
        }
    });
}
fn flush_with(state: &State, mut notify: impl FnMut(&IRawElementProviderSimple)) {
    let pending = &state.structure_notifications;
    pending.posted.set(false);
    if state.closed.get() || pending.flushing.get() || !pending.dirty.get() {
        return;
    }
    if state.host.borrow().is_none() {
        return;
    }
    pending.flushing.set(true);
    let _guard = Flushing(state);
    pending.dirty.set(false);
    let provider = state.uia.borrow().as_ref().map(uia::Root::provider);
    // 不跨通知保留 RefCell 借用；回调内的新替换排到下一轮。
    if let Some(provider) = provider {
        notify(&provider);
    }
}
struct Flushing<'a>(&'a State);
impl Drop for Flushing<'_> {
    fn drop(&mut self) {
        self.0.structure_notifications.flushing.set(false);
        resume(self.0);
    }
}
#[cfg(test)]
#[path = "surface_window_edit_structure_tests.rs"]
mod edit_tests;
#[cfg(test)]
#[path = "surface_window_structure_notification_tests.rs"]
mod tests;
