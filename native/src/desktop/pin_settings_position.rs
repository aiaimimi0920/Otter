//! Position left child strips and the image/border together; pure dragging reads existing layout without fields, control relayout, or waiting for host notifications.
use super::*;
fn target(anchor: RECT, work: RECT, size: (i32, i32)) -> Result<RECT> {
    if size.0 > work.right - work.left || size.1 > work.bottom - work.top {
        return Err(Error::new(
            E_BOUNDS,
            "Work area cannot contain pin settings",
        ));
    }
    Ok(super::super::pin_settings_geometry::place(
        anchor, work, size,
    ))
}
pub(super) unsafe fn place(owner: HWND, panel: &PropertyPanel) -> Result<()> {
    let mut anchor = RECT::default();
    GetWindowRect(owner, &mut anchor)?;
    let work = super::super::tool_panel::position::work_area(&anchor)?;
    panel.set_max_height(work.bottom - work.top)?;
    panel.place(target(anchor, work, panel.size())?)
}
fn panel(state: &State) -> Option<Rc<PropertyPanel>> {
    let session = state.pin_settings.try_borrow().ok()?;
    session.open.then(|| session.panel.clone()).flatten()
}
pub(in crate::desktop) unsafe fn moving_position(
    state: &State,
    proposed: RECT,
    work: RECT,
) -> Result<Option<(HWND, RECT)>> {
    let Some(panel) = panel(state) else {
        return Ok(None);
    };
    let Some(hwnd) = panel.window().filter(|h| IsWindowVisible(*h).as_bool()) else {
        return Ok(None);
    };
    Ok(Some((hwnd, target(proposed, work, panel.size())?)))
}
pub(in crate::desktop) unsafe fn follow(owner: HWND, state: &State) {
    if let Some(panel) = panel(state) {
        if let Err(error) = place(owner, &panel) {
            eprintln!("native-pin-settings-follow: {error}");
        }
    }
}
