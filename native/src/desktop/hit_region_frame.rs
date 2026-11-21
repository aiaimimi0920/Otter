//! 贴图区域缓存和内容样式命中：边框补入透明底图区域，圆角只裁 Pan 内容。
use super::*;
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Key {
    pub source: usize,
    pub revision: u64,
    pub pan: bool,
    pub client: (u32, u32),
    pub dpi: u32,
    pub viewport: crate::viewport::Viewport,
    pub minified: Option<(crate::session_minified::Views, crate::crop_geometry::Size)>,
    pub style: crate::pin_frame_style::Style,
    pub overlay: usize,
}
pub(super) unsafe fn apply(region: &OwnedRegion, key: Key) -> Result<()> {
    if !key.pan {
        return Ok(());
    }
    let (w, h) = (key.client.0 as i32, key.client.1 as i32);
    let factor = key.dpi as f32 / 96.0;
    if let Some((width, crate::surface_color::Color([_, _, _, alpha]))) =
        key.style.border().filter(|(_, c)| c.0[3] > 0.0)
    {
        debug_assert!(alpha > 0.0);
        let inset = (width * factor).ceil() as i32;
        let outer = OwnedRegion::checked(CreateRectRgn(0, 0, w, h))?;
        let inner = OwnedRegion::checked(CreateRectRgn(
            inset,
            inset,
            (w - inset).max(inset),
            (h - inset).max(inset),
        ))?;
        combine(&outer, &inner, RGN_DIFF)?;
        combine(region, &outer, RGN_OR)?;
    }
    let radius = (key.style.radius() * factor).min(w.min(h) as f32 / 2.0);
    if radius > 0.0 {
        let diameter = (radius * 2.0).round() as i32;
        let rounded =
            OwnedRegion::checked(CreateRoundRectRgn(0, 0, w + 1, h + 1, diameter, diameter))?;
        combine(region, &rounded, RGN_AND)?;
    }
    Ok(())
}
unsafe fn combine(a: &OwnedRegion, b: &OwnedRegion, mode: RGN_COMBINE_MODE) -> Result<()> {
    if CombineRgn(Some(a.0), Some(a.0), Some(b.0), mode) == RGN_ERROR {
        return Err(Error::from_win32());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn styled_transparent_source_keeps_border_hits_and_round_corners_at_two_dpi() {
        unsafe {
            for dpi in [96, 192] {
                let factor = (dpi / 96) as i32;
                let region = OwnedRegion::checked(CreateRectRgn(0, 0, 0, 0)).unwrap();
                let key = Key {
                    source: 0,
                    revision: 0,
                    pan: true,
                    client: (40 * factor as u32, 20 * factor as u32),
                    dpi,
                    viewport: Default::default(),
                    minified: None,
                    style: crate::pin_frame_style::Style::read(
                        &serde_json::json!({"borderWidth":2,"borderColor":"red","cornerRadius":4}),
                    )
                    .unwrap(),
                    overlay: 0,
                };
                apply(&region, key).unwrap();
                assert!(PtInRegion(region.0, 20 * factor, factor).as_bool());
                assert!(!PtInRegion(region.0, 20 * factor, 6 * factor).as_bool());
                assert!(!PtInRegion(region.0, 0, 0).as_bool());
            }
        }
    }
}
