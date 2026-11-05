//! 将持久化底图和栅格标注接入原生编辑器，不展平、不把恢复预览变成可编辑源图。
use super::Images;
use crate::{editor::Editor, erase_layers::Layers};
use windows::{
    core::{Error, Result},
    Win32::Foundation::E_INVALIDARG,
};

impl Images {
    /// 新编辑器以恢复内容为初始基线，不能撤销到恢复之前；后续修改走正常历史事务。
    /// 这里只恢复栅格层，矢量标注、裁剪/变换与持久橡皮擦路径需另行恢复后才能发布窗口。
    pub fn prepare_raster_editor(&self) -> Result<Editor> {
        let Some(source) = &self.source else {
            if self.rasterized_annotations.is_some() {
                return Err(Error::new(
                    E_INVALIDARG,
                    "Raster annotations require an explicit source coordinate frame",
                ));
            }
            return Ok(Editor::default());
        };
        let layers = Layers {
            size: (source.width, source.height),
            base: source.pixels.clone(),
            annotation_size: self
                .rasterized_annotations
                .as_ref()
                .map(|image| (image.width, image.height)),
            annotations: self
                .rasterized_annotations
                .as_ref()
                .map(|image| image.pixels.clone()),
        };
        Editor::with_layers(layers)
            .map_err(|_| Error::new(E_INVALIDARG, "Invalid restored raster layers"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{desktop::image::Image, erase_layers::Mode, viewport::Point, workspace_id::UnitId};
    fn image(width: u32, height: u32, value: u8) -> Image {
        Image {
            width,
            height,
            pixels: vec![value; (width * height * 4) as usize].into(),
        }
    }
    fn images() -> Images {
        Images {
            sticker: Default::default(),
            accept_upstream: true,
            capture_original: None,
            unit: UnitId::parse("u").unwrap(),
            source: Some(image(8, 8, 255)),
            preview: Some(image(4, 4, 99)),
            rasterized_annotations: Some(image(16, 16, 255)),
            restored_preview_locked: true,
        }
    }
    #[test]
    fn restored_independent_layer_can_be_erased_and_undone_without_touching_source() {
        let images = images();
        let mut editor = images.prepare_raster_editor().unwrap();
        assert!(!editor.undo());
        let original = editor.layers().unwrap().clone();
        assert_eq!(original.size, (8, 8));
        assert_eq!(original.annotation_dimensions(), (16, 16));
        assert!(original
            .base
            .same_storage(&images.source.as_ref().unwrap().pixels));
        assert!(original
            .annotations
            .as_ref()
            .unwrap()
            .same_storage(&images.rasterized_annotations.as_ref().unwrap().pixels));
        assert!(!original
            .base
            .same_storage(&images.preview.as_ref().unwrap().pixels));
        let candidate = original
            .erase(
                Mode::RasterAnnotations,
                &[Point { x: 4.0, y: 4.0 }],
                4.0,
                || false,
            )
            .unwrap();
        assert!(candidate.base.same_storage(&original.base));
        assert!(!candidate
            .annotations
            .as_ref()
            .unwrap()
            .same_storage(original.annotations.as_ref().unwrap()));
        assert!(editor
            .replace_layers(editor.revision(), &original, candidate)
            .unwrap());
        assert!(editor.undo());
        assert!(editor.layers().unwrap().same_storage(&original));
        assert!(editor.redo());
        drop(images);
        assert_eq!(editor.layers().unwrap().base.len(), 8 * 8 * 4);
    }
    #[test]
    fn missing_or_invalid_source_never_uses_preview_as_silent_fallback() {
        let mut images = images();
        images.source = None;
        assert!(images.prepare_raster_editor().is_err());
        images.rasterized_annotations = None;
        assert!(images.prepare_raster_editor().unwrap().layers().is_none());
        images.source = Some(image(8, 8, 255));
        images.rasterized_annotations = Some(Image {
            width: 2,
            height: 2,
            pixels: vec![255; 4].into(),
        });
        assert!(images.prepare_raster_editor().is_err());
    }
}
