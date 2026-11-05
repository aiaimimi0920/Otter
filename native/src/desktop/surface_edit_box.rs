//! textarea HWND 留出父 D2D 边框，内部格式区域容纳 padding，沿用外层遮挡裁剪。
use super::*;
use windows::Win32::UI::Controls::{EM_GETRECT, EM_SETRECT};
#[path = "surface_edit_border_clip.rs"]
mod border_clip;
pub(super) fn inset(
    origin: &mut [i32; 2],
    size: &mut [i32; 2],
    style: crate::surface_textarea::BoxStyle,
    scale: f32,
) {
    for axis in 0..2 {
        let start = (style.border.start()[axis] * scale).ceil() as i32;
        let end = ([style.border.right, style.border.bottom][axis] * scale).ceil() as i32;
        let start = start.clamp(0, (size[axis] - 1).max(0));
        let end = end.clamp(0, (size[axis] - start - 1).max(0));
        origin[axis] += start;
        size[axis] -= start + end;
    }
}
pub(super) unsafe fn clip(
    hwnd: HWND,
    region: HRGN,
    size: [i32; 2],
    style: crate::surface_textarea::BoxStyle,
    scale: f32,
) -> Result<()> {
    border_clip::apply(region, size, style, scale)?;
    let mut client = RECT::default();
    GetClientRect(hwnd, &mut client)?; // 客户区已扣掉原生垂直滚动条。
    let padding = style.padding;
    let limit = [client.right.max(0), client.bottom.max(0)];
    let start: [i32; 2] =
        std::array::from_fn(|a| ((padding.start()[a] * scale).round() as i32).clamp(0, limit[a]));
    let end = [padding.right, padding.bottom];
    let end: [i32; 2] =
        std::array::from_fn(|a| ((end[a] * scale).round() as i32).clamp(0, limit[a] - start[a]));
    let rect = RECT {
        left: start[0],
        top: start[1],
        right: client.right - end[0],
        bottom: client.bottom - end[1],
    };
    let mut old = RECT::default();
    SendMessageW(
        hwnd,
        EM_GETRECT,
        None,
        Some(LPARAM((&mut old as *mut RECT) as isize)),
    );
    if old != rect {
        SendMessageW(
            hwnd,
            EM_SETRECT,
            None,
            Some(LPARAM((&rect as *const RECT) as isize)),
        );
    }
    Ok(())
}
