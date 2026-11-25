//! Otter owns pin attachment policy; themes project attached rows only, and pure movement neither relayouts tools nor accesses application models.
use super::super::position::work_area;
use super::*;
pub(in crate::desktop::tool_panel) unsafe fn place(
    owner: HWND,
    state: &State,
    proposed: Option<RECT>,
) -> Result<()> {
    let panel = state.tool_panel.get();
    let mut anchor = proposed.unwrap_or_default();
    if proposed.is_none() {
        GetWindowRect(owner, &mut anchor)?;
    }
    let work = work_area(&anchor)?;
    let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(owner).max(96);
    neuro_mini_ui::resize(panel, work.right - work.left, dpi)?;
    let mut old = RECT::default();
    GetWindowRect(panel, &mut old)?;
    let target = anchored(
        anchor,
        work,
        old.right - old.left,
        old.bottom - old.top,
        (12 * dpi / 96) as i32,
    );
    if old != target {
        SetWindowPos(
            panel,
            None,
            target.left,
            target.top,
            target.right - target.left,
            target.bottom - target.top,
            SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
        )?;
    }
    neuro_mini_ui::sync_attached(panel)
}
pub(in crate::desktop) unsafe fn moving_positions(
    owner: HWND,
    state: &State,
    proposed: RECT,
) -> Result<Vec<(HWND, RECT)>> {
    let panel = state.tool_panel.get();
    if !neuro_mini_ui::is_toolbar(panel) || !IsWindowVisible(panel).as_bool() {
        return Ok(Vec::new());
    }
    let mut rect = RECT::default();
    GetWindowRect(panel, &mut rect)?;
    let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(owner).max(96);
    let work = work_area(&proposed)?;
    let next = anchored(
        proposed,
        work,
        rect.right - rect.left,
        rect.bottom - rect.top,
        (12 * dpi / 96) as i32,
    );
    let mut result = vec![(panel, next)];
    result.extend(neuro_mini_ui::attached_position(panel, next)?);
    result.extend(crate::desktop::pin_settings::moving_position(
        state, proposed, work,
    )?);
    Ok(result)
}
pub(super) fn anchored(anchor: RECT, work: RECT, width: i32, height: i32, overhang: i32) -> RECT {
    let left = (anchor.left - overhang).clamp(work.left, (work.right - width).max(work.left));
    let above = anchor.top - height;
    let top = if above >= work.top {
        above
    } else if anchor.bottom + height <= work.bottom {
        anchor.bottom
    } else {
        work.top
    };
    RECT {
        left,
        top,
        right: left + width,
        bottom: top + height,
    }
}
