//! 左侧子栏与图框同批定位；纯拖动只读既有布局，不访问字段、重排控件或等待宿主通知。
use super::*;
use windows::Win32::Graphics::Gdi::*;

unsafe fn work(anchor: &RECT) -> Result<RECT> {
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !GetMonitorInfoW(MonitorFromRect(anchor, MONITOR_DEFAULTTONEAREST), &mut info).as_bool() {
        return Err(Error::from_win32());
    }
    Ok(info.rcWork)
}
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
    let work = work(&anchor)?;
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
) -> Result<Option<(HWND, RECT)>> {
    let Some(panel) = panel(state) else {
        return Ok(None);
    };
    let Some(hwnd) = panel.window().filter(|h| IsWindowVisible(*h).as_bool()) else {
        return Ok(None);
    };
    Ok(Some((
        hwnd,
        target(proposed, work(&proposed)?, panel.size())?,
    )))
}
pub(in crate::desktop) unsafe fn follow(owner: HWND, state: &State) {
    if let Some(panel) = panel(state) {
        if let Err(error) = place(owner, &panel) {
            eprintln!("native-pin-settings-follow: {error}");
        }
    }
}
