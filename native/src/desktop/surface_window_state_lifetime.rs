//! 原窗口 Host 临时取出/归还与关闭的唯一生命周期接缝；不跨系统回调保留 RefCell 借用。
use super::*;
impl State {
    pub(super) fn run(&self, operation: impl FnOnce(&mut Host) -> Result<()>) -> Result<bool> {
        let Some(mut host) = self.host.borrow_mut().take() else {
            self.deferred.set(true);
            return Ok(false);
        };
        let result = operation(&mut host);
        if self.pointer_cancel_pending.replace(false) && !self.closed.get() {
            if let Err(error) = host.cancel_javascript_pointers(self.viewport_epoch.get()) {
                *self.error.borrow_mut() = Some(error);
            }
        }
        pointer::reconcile_selection(self, &mut host);
        let changes = host.take_switch_changes();
        let ranges = host.take_slider_changes();
        let readonly = host.take_slider_readonly_changes();
        let structure_changed = host.take_edit_structure_change();
        // D2D 可能触发同步窗口消息；关闭后不得复活已取出的 Host。
        if !self.closed.get() {
            *self.host.borrow_mut() = Some(host);
            notifications::enqueue(self, changes);
            notifications::enqueue_ranges(self, ranges);
            notifications::readonly::enqueue(self, readonly);
            if structure_changed {
                structure_notifications::schedule(self);
            }
            focus_notifications::resume(self);
            structure_notifications::resume(self);
        }
        result.map(|_| true)
    }
    pub(super) fn stop(&self) {
        self.closed.set(true);
        self.notifications.borrow_mut().clear();
        accessibility_bridge::unregister(self);
        self.uia.borrow_mut().take();
        self.accessibility.borrow_mut().take();
        self.local_edit.set(false);
        javascript_pointer::clear(self);
        self.pointer.borrow_mut().clear();
        self.keyboard.borrow_mut().clear();
        scrollbar::clear(self);
        gesture::clear(self);
        let host = self.host.borrow_mut().take();
        drop(host);
    }
}
