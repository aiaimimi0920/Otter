//! 本地滚动先准备、再提交/隐藏活动EDIT，最后实际paint；不联网、不改变canonical或服务授权。
use super::*;
use crate::surface_scroll::Scroll;

pub(super) fn install(
    hwnd: HWND,
    state: &State,
    id: &str,
    scroll: &Scroll,
    snapshot: &Arc<crate::tile_surface_wire::Snapshot>,
) -> Result<bool> {
    let frame =
        actions::active(state).ok_or_else(|| Error::new(E_ABORT, "Tile scroll is not active"))?;
    if frame
        .arts
        .get(id)
        .is_none_or(|a| !Arc::ptr_eq(&a.snapshot, snapshot))
    {
        return Err(Error::new(E_ABORT, "Tile scroll snapshot changed"));
    }
    if state
        .renderer
        .borrow()
        .as_ref()
        .is_some_and(|r| r.matches_scroll(&frame, id, scroll))
    {
        return Ok(false);
    }
    state.painted.set(false);
    {
        let mut s = state.shared.lock();
        s.presented = None;
        s.presented_frame = None;
    }
    let result = state
        .renderer
        .borrow_mut()
        .as_mut()
        .ok_or_else(|| Error::new(E_ABORT, "Tile renderer retired"))?
        .install_scroll(&frame, id, scroll);
    if let Err(error) = result {
        state.shared.stop();
        return Err(error);
    }
    // UpdateWindow可同步重入停止/换代；不持renderer/Shared借用，调用方只接纳重新paint后的身份。
    unsafe {
        let _ = InvalidateRect(Some(hwnd), None, false);
        let _ = UpdateWindow(hwnd);
    }
    if actions::active(state).is_none() {
        return Err(Error::new(E_ABORT, "Tile scroll paint not current"));
    }
    Ok(true)
}
pub(super) fn wheel(
    hwnd: HWND,
    state: &State,
    pixel: [i32; 2],
    axis: usize,
    delta: f32,
    page: bool,
) -> Result<bool> {
    let Some(frame) = actions::active(state) else {
        return Ok(false);
    };
    let plan = state
        .renderer
        .borrow()
        .as_ref()
        .and_then(|r| r.planned_wheel(&frame, pixel, axis, delta, page));
    let Some((id, scroll)) = plan else {
        return Ok(false);
    };
    if !text::commit_for_click(state) {
        return Ok(false);
    }
    super::scrollbar::cancel(hwnd, state);
    keyboard::close_for_text(state);
    actions::slider::cancel(hwnd, state);
    keyboard::scrolled(state);
    install(hwnd, state, &id, &scroll, &frame.arts[&id].snapshot)
}
pub(super) unsafe fn message(hwnd: HWND, state: &State, msg: u32, wp: WPARAM, lp: LPARAM) -> bool {
    if !IsWindowVisible(hwnd).as_bool() {
        return false;
    }
    let Some(input) = crate::desktop::surface_scroll_input::wheel(hwnd, msg, wp, lp) else {
        return false;
    };
    match wheel(
        hwnd,
        state,
        input.pixel,
        input.axis,
        input.delta,
        input.page,
    ) {
        Ok(changed) => changed,
        Err(error) => {
            state.shared.lock().actions.notice = "Art滚动未完成";
            if error.code() != E_ABORT {
                *state.error.borrow_mut() = Some(error);
                state.shared.stop();
            }
            true
        }
    }
}
#[cfg(test)]
impl Window {
    pub(crate) fn test_art_scroll(&self, pixel: [i32; 2], axis: usize, delta: f32) -> bool {
        wheel(self.hwnd, &self.state, pixel, axis, delta, false).unwrap()
    }
}
