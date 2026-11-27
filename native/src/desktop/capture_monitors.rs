//! Physical rectangles only, including anonymous monitors; never enumerate window titles or devices.
use crate::capture_loupe_layout::Bounds;
use windows::{
    core::{Error, Result, BOOL},
    Win32::{Foundation::*, Graphics::Gdi::*},
};
pub(super) unsafe fn remember(
    origin: POINT,
    width: u32,
    height: u32,
) -> Option<crate::capture_last_region::Last> {
    let rect = [
        origin.x,
        origin.y,
        origin.x.checked_add(width.try_into().ok()?)?,
        origin.y.checked_add(height.try_into().ok()?)?,
    ];
    match snapshot() {
        Ok(displays) => crate::capture_last_region::Last::new(rect, displays),
        Err(e) => {
            eprintln!("native-capture-region-history: {e}");
            None
        }
    }
}
pub(super) unsafe fn snapshot() -> Result<Vec<Bounds>> {
    unsafe extern "system" fn visit(_: HMONITOR, _: HDC, r: *mut RECT, p: LPARAM) -> BOOL {
        let out = &mut *(p.0 as *mut Vec<Bounds>);
        if r.is_null() || out.len() >= 64 {
            return BOOL(0);
        }
        let r = *r;
        out.push([r.left, r.top, r.right, r.bottom]);
        BOOL(1)
    }
    let mut displays: Vec<Bounds> = Vec::new();
    if !EnumDisplayMonitors(
        None,
        None,
        Some(visit),
        LPARAM(&mut displays as *mut _ as isize),
    )
    .as_bool()
        || displays.is_empty()
    {
        return Err(Error::new(E_FAIL, "Cannot determine capture monitors"));
    }
    displays.sort_unstable();
    Ok(displays)
}
