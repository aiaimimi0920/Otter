//! 导出局部模糊资源：持久框与绘制半径共同决定缓存，不修改活动编辑历史。
use super::*;
use crate::blur_image::BlurImage;
use std::collections::HashMap;
#[path = "export_projection_active_blur.rs"]
mod active;
#[path = "export_blur_boxes.rs"]
mod boxes;
#[path = "export_blur_local.rs"]
mod local;
#[path = "export_blur_sampling.rs"]
mod sampling;
pub(in crate::desktop) use active::prepare_active;
#[cfg(test)]
#[path = "export_blur_local_resource_tests.rs"]
mod tests;

/// 已恢复会话导出必须使用持久效果框；无 blur 时不解析标注或分配取样纹理。
pub(in crate::desktop) fn prepare_restored(
    image: &Image,
    editor: &Editor,
    geometry: &Frame,
    window: &crate::session_window::Window<'_>,
    restored_crop: Option<crate::session_crop::Restored>,
    cancel: &AtomicBool,
) -> Result<Editor> {
    check(cancel)?;
    if !editor.elements().iter().any(|stroke| stroke.blur.is_some()) {
        return Ok(editor.export_snapshot());
    }
    let boxes = boxes::read(window.annotation_state, cancel)?;
    let edit: serde_json::Value = window
        .image_edit_state
        .map(|raw| serde_json::from_str(raw.get()))
        .transpose()
        .map_err(|_| Error::new(E_INVALIDARG, "Invalid blur edit state"))?
        .unwrap_or(serde_json::Value::Null);
    let original = sampling::original(
        image,
        (
            edit["flippedX"].as_bool() == Some(true),
            edit["flippedY"].as_bool() == Some(true),
        ),
        cancel,
    )?;
    let layout = Layout::for_frame(
        image,
        geometry,
        editor.crop_view().map(|crop| crop.source.rect),
    )?;
    let mut cache = HashMap::<[u64; 5], BlurImage>::new();
    editor.export_snapshot_with_blur_strokes(|stroke| {
        check(cancel)?;
        let rect = boxes
            .get(stroke.annotation_id())
            .ok_or_else(|| Error::new(E_INVALIDARG, "Missing blur annotation identity"))?;
        let strength = stroke.blur.as_ref().unwrap().strength().round().max(1.0);
        let key = [
            rect.x.to_bits(),
            rect.y.to_bits(),
            rect.width.to_bits(),
            rect.height.to_bits(),
            strength.to_bits() as u64,
        ];
        if let Some(image) = cache.get(&key) {
            return Ok(image.clone());
        }
        let image = local::prepare(&original, &layout, *rect, restored_crop, strength, cancel)?;
        cache.try_reserve(1).map_err(|_| {
            Error::new(
                windows::Win32::Foundation::E_OUTOFMEMORY,
                "Cannot cache local blur texture",
            )
        })?;
        cache.insert(key, image.clone());
        Ok(image)
    })
}

fn check(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::new(
            windows::Win32::Foundation::E_ABORT,
            "Source blur cancelled",
        ))
    } else {
        Ok(())
    }
}

fn failure(error: crate::blur::BlurError) -> Error {
    Error::new(
        match error {
            crate::blur::BlurError::Cancelled => windows::Win32::Foundation::E_ABORT,
            crate::blur::BlurError::BudgetExceeded => windows::Win32::Foundation::E_OUTOFMEMORY,
            _ => E_INVALIDARG,
        },
        "Cannot prepare source-density blur",
    )
}
