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
        self.pin_layout.signals.active_origin.set(None);
        self.pin_layout.signals.sync_chrome.set(false);
        self.pin_layout.signals.peers.borrow_mut().clear();
        self.pin_layout.click.pointer.take();
        self.pin_layout.click.all.set(false);
        self.pin_layout.click.enabled.set(false);
    }
    unsafe fn sync_shake_peers(&self) {
        let peers = self
            .documents
            .iter()
            .filter(|d| {
                d.is_alive()
                    && self.workspace.contains(&d.state.unit_id)
                    && !self
                        .workspace
                        .group_of(&d.state.unit_id)
                        .is_some_and(|g| g.hidden)
            })
            .map(|d| {
                (
                    d.window.0,
                    d.panel.0,
                    GetWindowLongPtrW(d.window.0, GWLP_USERDATA),
                )
            })
            .collect();
        *self.pin_layout.signals.peers.borrow_mut() = peers;
    }
    pub(in crate::desktop::application) unsafe fn restore_shake(&mut self) {
        self.pin_layout.shake_origin = None;
        self.pin_layout.signals.count.set(0);
        self.pin_layout.signals.active_origin.set(None);
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
                if neuro_mini_ui::is_toolbar(d.panel.0) {
                    if let Some(props) = neuro_mini_ui::property_window(d.panel.0) {
                        let _ = ShowWindow(props, SW_SHOWNOACTIVATE);
                    }
                }
            }
        }
        self.sync_workspace_chrome(false);
    }
    pub(in crate::desktop::application) unsafe fn toggle_shake_from(&mut self, origin: &UnitId) {
        if self.pin_layout.signals.count.get() > 0 || self.pin_layout.shake_origin.is_some() {
            self.restore_shake();
            return;
        }
        let Some(t) = self.pin_target(origin, true) else {
            return;
        };
        self.hide_others_except(origin, t);
    }
    unsafe fn hide_others_except(&mut self, origin: &UnitId, t: Target) {
        let mut count = 0;
        for d in &self.documents {
            if &d.state.unit_id == origin
                || !d.is_alive()
                || !self.workspace.contains(&d.state.unit_id)
                || !d.state.canvas.try_borrow().is_ok_and(|c| c.pinned)
                || d.state.group_hidden.get()
                || self
                    .workspace
                    .group_of(&d.state.unit_id)
                    .is_some_and(|g| g.hidden)
                || crate::desktop::tray::is_hidden(&d.state)
                || !IsWindowVisible(d.window.0).as_bool()
            {
                continue;
            }
            if let Some(settings) = crate::desktop::pin_settings::window(&d.state) {
                let _ = ShowWindow(settings, SW_HIDE);
            }
            d.state
                .shake
                .panel
                .set(IsWindowVisible(d.panel.0).as_bool());
            if neuro_mini_ui::is_toolbar(d.panel.0) {
                if let Some(props) = neuro_mini_ui::property_window(d.panel.0) {
                    let _ = ShowWindow(props, SW_HIDE);
                }
            }
            d.state.shake.hidden.set(true);
            let _ = ShowWindow(d.panel.0, SW_HIDE);
            let _ = crate::desktop::application::chrome::hide_pin(d.window.0, &d.state);
            let _ = ShowWindow(d.window.0, SW_HIDE);
            count += 1;
        }
        if count > 0 {
            self.pin_layout
                .signals
                .active_origin
                .set(Some((t.hwnd, t.marker)));
            self.pin_layout.shake_origin = Some(t);
            self.pin_layout.signals.count.set(count);
            self.sync_workspace_chrome(false);
        }
    }
    pub(in crate::desktop::application) unsafe fn settle_pin_shake(&mut self) {
        self.sync_shake_peers();
        if let Some((hwnd, marker)) = self.pin_layout.signals.active_origin.get() {
            if self.pin_layout.shake_origin.is_none() {
                if let Some(d) = self.documents.iter().find(|d| {
                    d.is_alive()
                        && d.window.0 == hwnd
                        && GetWindowLongPtrW(hwnd, GWLP_USERDATA) == marker
                }) {
                    self.pin_layout.shake_origin = Some(Target {
                        id: d.state.unit_id.clone(),
                        hwnd,
                        marker,
                    });
                }
            }
        } else if self.pin_layout.signals.sync_chrome.get() {
            self.pin_layout.shake_origin = None;
        }
        if self.pin_layout.signals.sync_chrome.replace(false) {
            self.sync_workspace_chrome(false);
        }
        let restore = self.pin_layout.signals.restore.replace(false);
        let active = self.pin_layout.shake_origin.is_some();
        if active
            && (restore
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
                    self.pin_layout.shake_origin = None;
                    self.pin_layout.signals.active_origin.set(None);
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
        self.hide_others_except(&origin, t);
    }
}
