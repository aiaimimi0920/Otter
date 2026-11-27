//! Bind pass-through to the exact menu owner. Recovery never activates a pin.
use super::*;
use crate::desktop::pin_click_through as pass;

impl Application {
    pub(super) unsafe fn click_target(&self, id: &UnitId) -> Option<Target> {
        let target = self.layout_target(id)?;
        let d = self.documents.iter().find(|d| &d.state.unit_id == id)?;
        (self.layout_idle()
            && self.tray_recovery.get()
            && crate::desktop::pin_click_through::panels::idle(&d.state)
            && d.state.resolved_display.borrow().is_none()
            && !crate::desktop::pin_window::preview::active(&d.state))
        .then_some(target)
    }
    pub(super) unsafe fn enter_click(&mut self, target: &Target) -> Result<()> {
        if self.click_target(&target.id).as_ref() != Some(target) {
            return Err(failure("pin-pass-unavailable"));
        }
        let d = self
            .documents
            .iter_mut()
            .find(|d| d.state.unit_id == target.id)
            .ok_or_else(stale)?;
        if d.parameters.as_ref().is_some_and(|p| !p.exit_idle()) {
            return Err(failure("pin-pass-unavailable"));
        }
        let probes = pass::enter(target.hwnd, &d.state)?;
        // Ordinary pins use an inline parameter rail. Retire only its idle
        // child/auxiliary grid; the node's expanded preference survives.
        if let Some(panel) = d.parameters.as_ref() {
            if let Some(inline) = panel.inline_binding() {
                inline.hide_auxiliary();
            }
        }
        d.parameters.take();
        self.sync_workspace_chrome(true);
        let doc = self
            .documents
            .iter()
            .find(|d| {
                d.state.unit_id == target.id
                    && d.is_alive()
                    && d.window.0 == target.hwnd
                    && (&*d.state as *const State as isize) == target.marker
            })
            .ok_or_else(stale)?;
        if let Err(err) = pass::confirm(target.hwnd, &doc.state, &probes) {
            pass::leave(target.hwnd, &doc.state)?;
            self.sync_workspace_chrome(true);
            return Err(err);
        }
        Ok(())
    }

    pub(in crate::desktop::application) unsafe fn settle_pin_click(&mut self, visible: bool) {
        let signals = &self.pin_layout.click;
        let pointer = signals.pointer.take();
        let all = signals.all.replace(false);
        let lost_tray = signals.try_tray_recovery(self.tray_recovery.get());
        if signals.count.get() == 0 || (!all && pointer.is_none() && !lost_tray) {
            return;
        }
        let top = if all || lost_tray {
            None
        } else {
            pointer.and_then(|p| {
                let hits: Vec<_> = self
                    .documents
                    .iter()
                    .filter(|d| {
                        let mut r = RECT::default();
                        d.is_alive()
                            && d.state.click.active()
                            && self.workspace.contains(&d.state.unit_id)
                            && IsWindowVisible(d.window.0).as_bool()
                            && !IsIconic(d.window.0).as_bool()
                            && GetWindowRect(d.window.0, &mut r).is_ok()
                            && r.left <= p.x
                            && p.x < r.right
                            && r.top <= p.y
                            && p.y < r.bottom
                    })
                    .map(|d| d.window.0)
                    .collect();
                topmost(&hits)
            })
        };
        let mut first = None;
        for d in &self.documents {
            if d.is_alive()
                && self.workspace.contains(&d.state.unit_id)
                && d.state.click.active()
                && top.is_none_or(|h| h == d.window.0)
            {
                if let Err(e) = pass::leave(d.window.0, &d.state) {
                    first.get_or_insert((d.state.unit_id.clone(), e));
                }
            }
        }
        let retry = lost_tray && signals.retry_pending();
        self.sync_workspace_chrome(visible);
        if retry {
            self.execution_wake.callback()();
        }
        if let Some((id, e)) = first.filter(|_| !retry || all || pointer.is_some()) {
            self.save_message(&id, &e.to_string(), visible);
        }
    }
}

// EnumWindows enumerates top-level windows in desktop Z order. Do not walk
// GW_HWNDNEXT across destruction/reuse, and never query unrelated window text.
unsafe fn topmost(hits: &[HWND]) -> Option<HWND> {
    if hits.len() < 2 {
        return hits.first().copied();
    }
    struct Search<'a> {
        hits: &'a [HWND],
        found: Option<HWND>,
    }
    unsafe extern "system" fn visit(hwnd: HWND, arg: LPARAM) -> windows::core::BOOL {
        let s = &mut *(arg.0 as *mut Search<'_>);
        if s.hits.contains(&hwnd) {
            s.found = Some(hwnd);
            return false.into();
        }
        true.into()
    }
    let mut search = Search { hits, found: None };
    let _ = EnumWindows(Some(visit), LPARAM(&mut search as *mut _ as isize));
    search.found
}
