//! Temporary visibility is owned by the workspace instance, never by a group
//! or tray flag. Validate document identity before every show, without focus.
use super::*;
impl Application {
    pub(in crate::desktop::application) fn reset_pin_layout(&mut self) {
        // The old windows are about to be destroyed; do not briefly show them.
        self.pin_layout.undo.clear();
        self.pin_layout.shake_origin = None;
        self.pin_layout.signals.count.set(0);
        self.pin_layout.signals.restore.set(false);
        self.pin_layout.click.pointer.take();
        self.pin_layout.click.all.set(false);
    }
    unsafe fn restore_shake(&mut self) {
        self.pin_layout.shake_origin = None;
        self.pin_layout.signals.count.set(0);
        for d in &self.documents {
            if !d.state.shake.hidden.replace(false) {
                continue;
            }
            let panel = d.state.shake.panel.replace(false);
            if !d.is_alive()
                || !self.workspace.contains(&d.state.unit_id)
                || d.state.group_hidden.get()
                || self
                    .workspace
                    .group_of(&d.state.unit_id)
                    .is_some_and(|g| g.hidden)
                || crate::desktop::tray::is_hidden(&d.state)
            {
                continue;
            }
            let _ = ShowWindow(d.window.0, SW_SHOWNOACTIVATE);
            if panel && d.is_alive() {
                let _ = ShowWindow(d.panel.0, SW_SHOWNOACTIVATE);
            }
        }
    }
    pub(in crate::desktop::application) unsafe fn settle_pin_shake(&mut self) {
        let restore = self.pin_layout.signals.restore.replace(false);
        let active = self.pin_layout.shake_origin.is_some();
        if active
            && (restore
                || !self.tray_recovery.get()
                || !self.pin_layout.shake_origin.as_ref().is_some_and(|t| {
                    self.source_window(&t.id) == Some(t.hwnd)
                        && GetWindowLongPtrW(t.hwnd, GWLP_USERDATA) == t.marker
                }))
        {
            self.restore_shake();
            return;
        }
        let Some(origin) = self
            .documents
            .iter()
            .find(|d| d.state.shake.pending.replace(false))
            .map(|d| d.state.unit_id.clone())
        else {
            if active {
                let count = self
                    .documents
                    .iter()
                    .filter(|d| d.state.shake.hidden.get() && d.is_alive())
                    .count();
                self.pin_layout.signals.count.set(count);
                if count == 0 {
                    self.pin_layout.shake_origin = None
                }
            }
            return;
        };
        if !self.layout_idle() || !self.tray_recovery.get() {
            return;
        }
        let Some(t) = self.pin_target(&origin, true) else {
            return;
        };
        if active {
            self.restore_shake();
            return;
        }
        let mut count = 0;
        for d in &self.documents {
            if d.state.unit_id == origin
                || self.layout_target(&d.state.unit_id).is_none()
                || d.parameters.is_some()
                || crate::desktop::pin_settings::window(&d.state).is_some()
            {
                continue;
            }
            d.state
                .shake
                .panel
                .set(IsWindowVisible(d.panel.0).as_bool());
            d.state.shake.hidden.set(true);
            let _ = ShowWindow(d.panel.0, SW_HIDE);
            let _ = ShowWindow(d.window.0, SW_HIDE);
            count += 1;
        }
        if count > 0 {
            self.pin_layout.shake_origin = Some(t);
            self.pin_layout.signals.count.set(count);
        }
    }
}
