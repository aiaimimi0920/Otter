//! Connect persisted base images and raster annotations to the native editor without flattening or turning restore previews into editable source images.
use super::Images;
use crate::{editor::Editor, erase_layers::Layers};
use windows::{
    core::{Error, Result},
    Win32::Foundation::E_INVALIDARG,
};

impl Images {
    /// The new editor starts from restored content and cannot undo before restoration; subsequent changes use normal history transactions.
    /// Restore only raster layers here; vectors, crop/transforms, and persistent eraser paths must be restored separately before window publication.
    pub fn prepare_raster_editor(&self) -> Result<Editor> {
        self.raster_layers()?.map_or_else(
            || Ok(Editor::default()),
            |layers| {
                Editor::with_layers(layers)
                    .map_err(|_| Error::new(E_INVALIDARG, "Invalid restored raster layers"))
            },
        )
    }

    pub(super) fn raster_layers(&self) -> Result<Option<Layers>> {
        let Some(source) = &self.source else {
            if self.rasterized_annotations.is_some() {
                return Err(Error::new(
                    E_INVALIDARG,
                    "Raster annotations require an explicit source coordinate frame",
                ));
            }
            return Ok(None);
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
        if !layers.valid() {
            return Err(Error::new(E_INVALIDARG, "Invalid restored raster layers"));
        }
        Ok(Some(layers))
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
