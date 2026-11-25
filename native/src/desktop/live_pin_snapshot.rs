//! 在原读回/保存 worker 内刷新 Live 底图；保留编辑坐标、栅格标注、裁剪及镜像。
use super::image::Image;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-live")))]
#[path = "live_pin_snapshot_tests.rs"]
mod tests;
use crate::{editor::Editor, erase_layers::Layers};

pub(super) fn refresh(
    mut editor: Editor,
    image: Image,
    cancelled: impl Fn() -> bool,
) -> Result<(Editor, Image), String> {
    let layers = editor.layers().ok_or("Live editor has no raster")?;
    let size = layers.size;
    let mirror = editor.mirror_state();
    let base = crate::raster_flip_resize::resized_axes(
        &image.pixels,
        (image.width, image.height),
        size,
        mirror.x,
        mirror.y,
        &cancelled,
    )
    .map_err(|error| format!("Live frame transform failed: {error:?}"))?;
    let candidate = Layers {
        base: base.clone(),
        ..layers.clone()
    };
    if !editor.replace_source(base.clone()) {
        let plan = editor
            .prepare_raster_refresh(candidate)
            .map_err(|error| format!("Live refresh preparation failed: {error:?}"))?
            .ok_or("Live refresh is busy")?
            .compute(&cancelled)
            .map_err(|error| format!("Live refresh effects failed: {error:?}"))?;
        if !editor
            .commit_raster_refresh(plan)
            .map_err(|error| format!("Live refresh commit failed: {error:?}"))?
        {
            return Err("Live refresh became stale".into());
        }
    }
    Ok((
        editor,
        Image {
            width: size.0,
            height: size.1,
            pixels: base,
        },
    ))
}
