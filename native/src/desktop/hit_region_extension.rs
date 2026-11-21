//! selectable root/声明动作补入透明 PNG 的原 HRGN；几何只在场景准备时测量一次。
use super::*;
pub(super) fn identity(state: &State) -> usize {
    super::super::extension_input::views(state).map_or(0, |v| v.identity())
}
pub(super) unsafe fn add(region: &OwnedRegion, state: &State, dpi: u32) -> Result<()> {
    let Some(views) = super::super::extension_input::views(state) else {
        return Ok(());
    };
    let factor = dpi as f32 / 96.;
    for r in views.rects().iter().filter(|r| !r.empty()) {
        let part = OwnedRegion::checked(CreateRectRgn(
            (r.left * factor).floor() as i32,
            (r.top * factor).floor() as i32,
            (r.right * factor).ceil() as i32,
            (r.bottom * factor).ceil() as i32,
        ))?;
        if CombineRgn(Some(region.0), Some(region.0), Some(part.0), RGN_OR) == RGN_ERROR {
            return Err(Error::from_win32());
        }
    }
    Ok(())
}
