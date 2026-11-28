//! Desktop pin positioning transaction. Only move host HWNDs; their ordinary
//! geometry notifications move the toolbar, border and port windows.
use super::*;
use crate::{
    pin_arrange_math::{plan_with_gap, Layout},
    pin_geometry::PinRect,
    workspace_id::UnitId,
};
use windows::Win32::{Graphics::Gdi::*, UI::HiDpi::GetDpiForWindow};
#[path = "application_pin_click.rs"]
mod click;
#[path = "application_pin_arrange_menu.rs"]
pub(super) mod menu;
#[path = "pin_arrange_palette.rs"]
pub(super) mod palette;
#[path = "application_pin_shake.rs"]
mod shake;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "application_pin_arrange_tests.rs"]
mod tests;

pub(in crate::desktop::application) struct PaletteTransaction(std::rc::Rc<std::cell::Cell<bool>>);
impl Drop for PaletteTransaction {
    fn drop(&mut self) {
        self.0.set(true);
    }
}

pub(super) struct Owner {
    pub(super) click: std::rc::Rc<crate::desktop::pin_click_through::Signals>,
    undo: Vec<Move>,
    shake_origin: Option<Target>,
    pub(super) signals: std::rc::Rc<crate::desktop::pin_shake_input::Signals>,
    pub(super) gap_dip: std::cell::Cell<u16>,
    pub(super) palette_host: std::cell::Cell<Option<HWND>>,
    pub(super) palette_gate: std::cell::RefCell<Option<std::rc::Rc<std::cell::Cell<bool>>>>,
}
impl Default for Owner {
    fn default() -> Self {
        Self {
            click: Default::default(),
            undo: Vec::new(),
            shake_origin: None,
            signals: Default::default(),
            gap_dip: std::cell::Cell::new(12),
            palette_host: std::cell::Cell::new(None),
            palette_gate: std::cell::RefCell::new(None),
        }
    }
}
impl Owner {
    /// Suspend only this palette's modal gate during synchronous non-pumping layout transactions.
    pub(in crate::desktop::application) fn suspend_palette_gate(
        &self,
    ) -> Option<PaletteTransaction> {
        let gate = self.palette_gate.borrow().clone()?;
        gate.replace(false).then_some(PaletteTransaction(gate))
    }
}
#[derive(Clone, PartialEq, Eq)]
pub(in crate::desktop::application) struct Target {
    id: UnitId,
    hwnd: HWND,
    marker: isize,
}
struct Move {
    target: Target,
    before: PinRect,
    after: PinRect,
}
fn failure(key: &str) -> Error {
    Error::new(E_FAIL, crate::ui_text::text(key))
}
fn rect(r: RECT) -> Result<PinRect> {
    let size = |a: i32, b: i32| {
        u32::try_from(i64::from(b) - i64::from(a))
            .ok()
            .filter(|v| *v != 0)
    };
    Ok(PinRect {
        x: r.left,
        y: r.top,
        width: size(r.left, r.right).ok_or_else(stale)?,
        height: size(r.top, r.bottom).ok_or_else(stale)?,
    })
}
fn stale() -> Error {
    failure("workspace-layout-unavailable")
}
unsafe fn frame(hwnd: HWND) -> Result<PinRect> {
    let mut r = RECT::default();
    GetWindowRect(hwnd, &mut r)?;
    rect(r)
}
unsafe fn position(hwnd: HWND, to: PinRect) -> Result<()> {
    SetWindowPos(
        hwnd,
        None,
        to.x,
        to.y,
        0,
        0,
        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
    )
}
impl Application {
    unsafe fn layout_target(&self, id: &UnitId) -> Option<Target> {
        self.pin_target(id, false)
    }
    unsafe fn pin_target(&self, id: &UnitId, shake_origin: bool) -> Option<Target> {
        if !self.workspace.contains(id)
            || self.link_owner_blocked(id)
            || self.workspace.group_of(id).is_some_and(|g| g.hidden)
        {
            return None;
        }
        let d = self.documents.iter().find(|d| &d.state.unit_id == id)?;
        let s = &d.state;
        let hwnd = d.window.0;
        let c = s.canvas.try_borrow().ok()?;
        (c.pinned
            && !s.click.active()
            && !s.live_pin.get()
            && !s.is_art()
            && !s.file_dialog.get()
            && !s.group_locked.get()
            && !s.group_hidden.get()
            && !crate::desktop::tray::is_hidden(s)
            && c.minified.is_none()
            && !c.crop_mode
            && c.drag.is_none()
            && !c.blocks_wheel()
            && !crate::desktop::group_lock::editing_busy(s)
            && !crate::desktop::pin_window::animation::zoom_busy(s)
            && (shake_origin || !crate::desktop::pin_window::animation::moving(s))
            && d.parameters.as_ref().is_none_or(|p| p.exit_idle())
            && IsWindowVisible(hwnd).as_bool()
            && !IsIconic(hwnd).as_bool()
            && (windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(hwnd).as_bool()
                || self.pin_layout.palette_host.get() == Some(hwnd))
            && d.is_alive())
        .then(|| Target {
            id: id.clone(),
            hwnd,
            marker: GetWindowLongPtrW(hwnd, GWLP_USERDATA),
        })
    }
    fn layout_idle(&self) -> bool {
        !self.modals.blocked()
            && !self.capture_active.get()
            && !self.exit_requested.get()
            && !self.session_end.paused()
            && self.restore_job.is_none()
            && self.chrome.idle()
    }
    unsafe fn arrange_pins(
        &mut self,
        targets: &[Target],
        anchor: &UnitId,
        layout: Layout,
    ) -> Result<()> {
        if !self.layout_idle() || targets.len() < layout.min_count() {
            return Err(stale());
        }
        let mut monitors =
            std::collections::BTreeMap::<isize, (PinRect, u32, Vec<(Target, PinRect)>)>::new();
        for t in targets {
            if self.layout_target(&t.id).as_ref() != Some(t) {
                return Err(stale());
            }
            let h = MonitorFromWindow(t.hwnd, MONITOR_DEFAULTTONULL);
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if h.is_invalid() || !GetMonitorInfoW(h, &mut info).as_bool() {
                return Err(stale());
            }
            let work = rect(info.rcWork)?;
            let dpi = GetDpiForWindow(t.hwnd);
            let entry = monitors
                .entry(h.0 as isize)
                .or_insert_with(|| (work, dpi, Vec::new()));
            if entry.0 != work || dpi == 0 || entry.1 != dpi {
                return Err(stale());
            }
            entry.2.push((t.clone(), frame(t.hwnd)?));
        }
        let mut moves = Vec::new();
        for (_, (work, dpi, group)) in monitors {
            if group.len() < layout.min_count() {
                return Err(failure("workspace-layout-monitor-count"));
            }
            let frames = group.iter().map(|(_, r)| *r).collect::<Vec<_>>();
            let primary = group.iter().position(|(t, _)| &t.id == anchor).unwrap_or(0);
            let gap_dip = self.pin_layout.gap_dip.get();
            let positions = plan_with_gap(layout, &frames, primary, work, dpi, gap_dip)
                .ok_or_else(|| failure("workspace-layout-space"))?;
            for ((target, before), after) in group.into_iter().zip(positions) {
                if before != after {
                    moves.push(Move {
                        target,
                        before,
                        after,
                    })
                }
            }
        }
        let mut done = Vec::new();
        for mut op in moves {
            let result = (|| {
                let hwnd = op.target.hwnd;
                if self.layout_target(&op.target.id).as_ref() != Some(&op.target)
                    || frame(hwnd)? != op.before
                {
                    return Err(stale());
                }
                let requested = op.after;
                let moved = position(hwnd, requested);
                if self.layout_target(&op.target.id).as_ref() != Some(&op.target) {
                    return Err(stale());
                }
                op.after = frame(hwnd)?;
                let exact = op.after == requested;
                if op.after != op.before {
                    done.push(op)
                }
                moved?;
                exact.then_some(()).ok_or_else(stale)
            })();
            if let Err(error) = result {
                let (remaining, _) = self.restore_layout(done);
                if !remaining.is_empty() {
                    self.pin_layout.undo = remaining;
                    return Err(failure("workspace-layout-rollback"));
                }
                return Err(error);
            }
        }
        if !done.is_empty() {
            self.pin_layout.undo = done;
            self.sync_workspace_chrome(true);
        }
        Ok(())
    }
    unsafe fn restore_layout(&self, ops: Vec<Move>) -> (Vec<Move>, bool) {
        let mut remaining = Vec::new();
        let mut skipped = false;
        for op in ops.into_iter().rev() {
            let t = &op.target;
            if self.layout_target(&t.id).as_ref() != Some(t) || frame(t.hwnd).ok() != Some(op.after)
            {
                skipped = true;
                continue;
            }
            if position(t.hwnd, op.before).is_err() || frame(t.hwnd).ok() != Some(op.before) {
                remaining.push(op)
            }
        }
        remaining.reverse();
        (remaining, skipped)
    }
    unsafe fn undo_layout(&mut self) -> Result<()> {
        if !self.layout_idle() {
            return Err(stale());
        }
        let undo = std::mem::take(&mut self.pin_layout.undo);
        let (remaining, skipped) = self.restore_layout(undo);
        self.pin_layout.undo = remaining;
        if !self.pin_layout.undo.is_empty() || skipped {
            return Err(failure("workspace-layout-rollback"));
        }
        self.sync_workspace_chrome(true);
        Ok(())
    }
}
