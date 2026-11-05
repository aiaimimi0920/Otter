//! 将已提交 DIP 布局转为 EDIT 客户区像素；原生区域归 Windows 持有。
use crate::surface_geometry::NodeGeometry;
#[path = "surface_edit_box.rs"]
mod textarea;
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*},
};
#[derive(Clone, Copy)]
pub enum Part {
    Whole,
    Edit,
    Spinner,
    Select,
    Textarea(crate::surface_textarea::BoxStyle),
}
pub fn place(
    hwnd: HWND,
    geometry: &NodeGeometry,
    later: &[NodeGeometry],
    overlays: &[crate::surface_scroll::Bar],
    dpi: u32,
    part: Part,
) -> Result<bool> {
    place_inner(hwnd, geometry, later, overlays, dpi, part, true)
}
/// 相同 Select 锚点只刷新遮挡区域；v6 COMBOBOX 的冗余 SetWindowPos 会关闭下拉。
pub fn refresh_select(
    hwnd: HWND,
    geometry: &NodeGeometry,
    later: &[NodeGeometry],
    overlays: &[crate::surface_scroll::Bar],
    dpi: u32,
) -> Result<bool> {
    place_inner(hwnd, geometry, later, overlays, dpi, Part::Select, false)
}
/// Slider 几何未变时仅刷新遮挡裁剪，不中断进行中的捕获手势。
pub fn refresh_slider(
    hwnd: HWND,
    geometry: &NodeGeometry,
    later: &[NodeGeometry],
    overlays: &[crate::surface_scroll::Bar],
    dpi: u32,
) -> Result<bool> {
    place_inner(hwnd, geometry, later, overlays, dpi, Part::Whole, false)
}
fn place_inner(
    hwnd: HWND,
    geometry: &NodeGeometry,
    later: &[NodeGeometry],
    overlays: &[crate::surface_scroll::Bar],
    dpi: u32,
    part: Part,
    reposition: bool,
) -> Result<bool> {
    let scale = dpi as f32 / 96.;
    let mut origin = geometry.origin().map(|v| (v * scale).round() as i32);
    let mut size = geometry.size().map(|v| (v * scale).round() as i32);
    if size.iter().any(|v| !(1..=32768).contains(v)) {
        return Err(Error::new(E_INVALIDARG, "Invalid edit geometry"));
    }
    let strip = ((20. * scale).round() as i32).min(size[0] / 2);
    match part {
        Part::Spinner if strip == 0 || size[1] < 2 => return Ok(false),
        Part::Spinner => {
            origin[0] += size[0] - strip;
            size[0] = strip;
        }
        Part::Edit if size[1] >= 2 => size[0] -= strip,
        Part::Textarea(style) => textarea::inset(&mut origin, &mut size, style, scale),
        _ => {}
    }
    let clip = geometry.clip();
    unsafe {
        if reposition {
            SetWindowPos(
                hwnd,
                None,
                origin[0],
                origin[1],
                size[0],
                if matches!(part, Part::Select) {
                    size[1]
                        .saturating_add((160. * scale).round() as i32)
                        .min(32768)
                } else {
                    size[1]
                },
                SWP_NOACTIVATE | SWP_NOZORDER,
            )?;
        }
        let region = CreateRectRgn(
            ((clip.left * scale).ceil() as i32 - origin[0]).clamp(0, size[0]),
            ((clip.top * scale).ceil() as i32 - origin[1]).clamp(0, size[1]),
            ((clip.right * scale).floor() as i32 - origin[0]).clamp(0, size[0]),
            ((clip.bottom * scale).floor() as i32 - origin[1]).clamp(0, size[1]),
        );
        if region.is_invalid() {
            return Err(Error::from_win32());
        }
        if let Part::Textarea(style) = part {
            if let Err(error) = textarea::clip(hwnd, region, size, style, scale) {
                let _ = DeleteObject(region.into());
                return Err(error);
            }
        }
        // HWND 子控件天然置于父 D2D 内容上方；减去后绘制节点的命中盒，避免被遮挡的
        // 字段仍截获输入。区域至多与场景节点数同阶，不建立逐像素掩码。
        for cover in later
            .iter()
            .map(NodeGeometry::clip)
            .chain(overlays.iter().map(|bar| bar.track.intersect(bar.clip)))
        {
            let covered = cover.intersect(clip);
            if covered.empty() {
                continue;
            }
            let cover = CreateRectRgn(
                (covered.left * scale).floor() as i32 - origin[0],
                (covered.top * scale).floor() as i32 - origin[1],
                (covered.right * scale).ceil() as i32 - origin[0],
                (covered.bottom * scale).ceil() as i32 - origin[1],
            );
            if cover.is_invalid() {
                let _ = DeleteObject(region.into());
                return Err(Error::from_win32());
            }
            let result = CombineRgn(Some(region), Some(region), Some(cover), RGN_DIFF);
            let _ = DeleteObject(cover.into());
            if result == RGN_ERROR {
                let _ = DeleteObject(region.into());
                return Err(Error::from_win32());
            }
        }
        let mut bounds = RECT::default();
        let visible = GetRgnBox(region, &mut bounds) != NULLREGION;
        if SetWindowRgn(hwnd, Some(region), true) == 0 {
            let _ = DeleteObject(region.into());
            return Err(Error::from_win32());
        }
        Ok(visible)
    }
}
