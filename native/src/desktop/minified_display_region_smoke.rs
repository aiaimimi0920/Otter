//! 同像素同 revision 的不同外部标注必须更新实际 HWND 区域；仅使用合成透明图。
use super::*;
use crate::{
    editor::{Editor, Tool},
    erase_layers::Layers,
    viewport::Point,
};
use windows::Win32::Graphics::Gdi::{CreateRectRgn, DeleteObject, GetWindowRgn, PtInRegion, HRGN};

struct Region(HRGN);
impl Drop for Region {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(self.0.into());
        }
    }
}

pub(super) unsafe fn verify(hwnd: HWND, state: &State) -> Result<()> {
    let stamp = state.canvas.borrow().editor.document_stamp();
    let (views, mini) = state.canvas.borrow().minified.as_ref().unwrap().geometry();
    let mut client = RECT::default();
    GetClientRect(hwnd, &mut client)?;
    let x = (10.0 / 64.0 * views.annotations.size.width / mini.width * client.right as f64).round()
        as i32;
    let y = (10.0 / 32.0 * views.annotations.size.height / mini.height * client.bottom as f64)
        .round() as i32;
    let lease = crate::pixels::reserve(64 * 32 * 4)
        .ok_or_else(|| Error::new(E_FAIL, "Synthetic region budget"))?;
    let image = crate::desktop::image::Image {
        width: 64,
        height: 32,
        pixels: crate::pixels::Pixels::from_reserved(vec![0; 64 * 32 * 4], lease),
    };
    let frames = [
        frame(&image, Point { x: 10.0, y: 10.0 })?,
        frame(&image, Point { x: 60.0, y: 25.0 })?,
    ];
    if frames[0].editor.as_ref().unwrap().revision()
        != frames[1].editor.as_ref().unwrap().revision()
        || !frames[0].image.pixels.same_storage(&frames[1].image.pixels)
    {
        return Err(Error::new(
            E_FAIL,
            "Synthetic display frames do not share cache-confusable inputs",
        ));
    }
    for (frame, expected) in frames.into_iter().zip([true, false]) {
        *state.resolved_display.borrow_mut() = Some(frame);
        crate::desktop::hit_region::update(hwnd, state)?;
        if contains(hwnd, x, y)? != expected {
            return Err(Error::new(
                E_FAIL,
                "Minified hit region reused stale external annotation frame",
            ));
        }
    }
    state.resolved_display.borrow_mut().take();
    crate::desktop::hit_region::update(hwnd, state)?;
    if !contains(hwnd, x, y)? || !stamp.matches(&state.canvas.borrow().editor) {
        return Err(Error::new(
            E_FAIL,
            "Minified display disconnect failed to restore local region",
        ));
    }
    Ok(())
}

fn frame(
    image: &crate::desktop::image::Image,
    point: Point,
) -> Result<crate::desktop::display_frame::Frame> {
    let mut editor = Editor::with_layers(Layers {
        size: (image.width, image.height),
        base: image.pixels.clone(),
        annotations: None,
        annotation_size: None,
    })
    .map_err(|_| Error::new(E_FAIL, "Synthetic editor"))?;
    editor
        .begin(Tool::Brush, point)
        .map_err(|_| Error::new(E_FAIL, "Synthetic annotation"))?;
    editor.commit();
    Ok(crate::desktop::display_frame::Frame::capture(
        crate::workspace_id::UnitId::parse("synthetic-region-source").unwrap(),
        image.clone(),
        Some(&editor),
    ))
}
unsafe fn contains(hwnd: HWND, x: i32, y: i32) -> Result<bool> {
    let region = Region(CreateRectRgn(0, 0, 0, 0));
    if region.0.is_invalid() || GetWindowRgn(hwnd, region.0).0 == 0 {
        return Err(Error::from_win32());
    }
    Ok(PtInRegion(region.0, x, y).as_bool())
}
