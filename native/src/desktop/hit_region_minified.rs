//! 缩略绘制的双层坐标直接映射到物理命中区域；不使用普通 fit viewport。
use super::*;
use crate::{
    crop_geometry::Size,
    session_minified::{View, Views},
};

pub(super) unsafe fn update(hwnd: HWND, state: &State) -> Result<()> {
    let prepared = {
        let canvas = state.canvas.borrow();
        let display = state.resolved_display.borrow();
        let empty = crate::editor::Editor::default();
        let editor = display.as_ref().map_or(&canvas.editor, |frame| {
            frame.editor.as_deref().unwrap_or(&empty)
        });
        let original = canvas.minified.as_ref().expect("checked by update");
        let external = display
            .as_ref()
            .map(|frame| original.for_display(&frame.image, editor))
            .transpose()?;
        let presentation = external.as_ref().unwrap_or(original);
        if !presentation.matches(editor) {
            return Err(Error::new(E_INVALIDARG, "Stale minified hit region"));
        }
        let mut client = RECT::default();
        GetClientRect(hwnd, &mut client)?;
        let geometry = presentation.geometry();
        let key = Key {
            source: presentation
                .image()
                .map_or(0, |image| image.pixels.as_ptr() as usize),
            revision: editor.revision(),
            pan: canvas.tool == Tool::Pan,
            client: (client.right as u32, client.bottom as u32),
            dpi: GetDpiForWindow(hwnd),
            viewport: Default::default(),
            minified: Some(geometry),
        };
        let mut cache = state.hit_region.borrow_mut();
        let identity = display.as_ref().map(|frame| &frame.identity);
        if match (&cache.display_identity, identity) {
            (None, None) => false,
            (Some(previous), Some(current)) => !std::sync::Arc::ptr_eq(previous, current),
            _ => true,
        } {
            cache.key = None;
        }
        if cache.key == Some(key) {
            return Ok(());
        }
        let region = if let Some(image) = presentation.image() {
            let layers = editor.layers();
            let overlay = layers.and_then(|layers| layers.annotations.as_ref());
            let dimensions = layers.map_or((image.width, image.height), |layers| {
                layers.annotation_dimensions()
            });
            if cache
                .source
                .as_ref()
                .is_none_or(|(pixels, _)| !pixels.same_storage(&image.pixels))
                || cache.annotation_size != dimensions
                || match (&cache.annotations, overlay) {
                    (None, None) => false,
                    (Some((a, _)), Some(b)) => !a.same_storage(b),
                    _ => true,
                }
            {
                *cache = HitRegion::prepare_sized(image, overlay, dimensions)?;
            }
            build(&cache, image, editor, geometry, key.client)?
        } else {
            // 无图 Art 仍保留可拖动宿主；不伪造图片 alpha。
            OwnedRegion::checked(CreateRectRgn(0, 0, client.right, client.bottom))?
        };
        cache.key = Some(key);
        cache.display_identity = identity.cloned();
        region
    };
    // SetWindowRgn 可重入，所有 RefCell 借用必须已释放。
    if SetWindowRgn(hwnd, Some(prepared.0), false) == 0 {
        state.hit_region.borrow_mut().key = None;
        return Err(Error::from_win32());
    }
    std::mem::forget(prepared);
    Ok(())
}

unsafe fn build(
    cache: &HitRegion,
    image: &super::super::image::Image,
    editor: &crate::editor::Editor,
    geometry: (Views, Size),
    client: (u32, u32),
) -> Result<OwnedRegion> {
    let (views, size) = geometry;
    let scale = (client.0 as f64 / size.width, client.1 as f64 / size.height);
    let region = transformed_region(
        &cache.source.as_ref().unwrap().1,
        image.width,
        image.height,
        transform(views.image, (image.width, image.height), scale),
    )?;
    if let Some((_, mask)) = &cache.annotations {
        let overlay = transformed_region(
            mask,
            cache.annotation_size.0,
            cache.annotation_size.1,
            transform(views.annotations, cache.annotation_size, scale),
        )?;
        combine(&region, &overlay, RGN_OR)?;
    }
    let mapping = views
        .annotations
        .document_mapping(editor.annotation_frame().or_else(|| editor.visible_frame()))
        .map_err(|_| Error::new(E_INVALIDARG, "Invalid minified hit mapping"))?;
    annotations::add_transformed(
        &region,
        editor,
        XFORM {
            eM11: (scale.0 * mapping.scale.0) as f32,
            eM22: (scale.1 * mapping.scale.1) as f32,
            eDx: (mapping.offset.0 * scale.0) as f32,
            eDy: (mapping.offset.1 * scale.1) as f32,
            ..Default::default()
        },
    )?;
    let clip = OwnedRegion::checked(CreateRectRgn(0, 0, client.0 as i32, client.1 as i32))?;
    combine(&region, &clip, RGN_AND)?;
    Ok(region)
}
fn transform(view: View, pixels: (u32, u32), scale: (f64, f64)) -> XFORM {
    XFORM {
        eM11: (view.size.width / pixels.0 as f64 * scale.0) as f32,
        eM22: (view.size.height / pixels.1 as f64 * scale.1) as f32,
        eDx: (-view.offset.0 * scale.0) as f32,
        eDy: (-view.offset.1 * scale.1) as f32,
        ..Default::default()
    }
}
unsafe fn combine(a: &OwnedRegion, b: &OwnedRegion, mode: RGN_COMBINE_MODE) -> Result<()> {
    if CombineRgn(Some(a.0), Some(a.0), Some(b.0), mode) == RGN_ERROR {
        return Err(Error::from_win32());
    }
    Ok(())
}

#[cfg(test)]
#[path = "hit_region_minified_tests.rs"]
mod tests;
