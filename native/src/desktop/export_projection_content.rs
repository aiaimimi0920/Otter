//! Adapt current Editor geometry to existing propagation-layout contracts; canonical crop must not override active crop/reset.
use super::*;
use crate::{
    crop_geometry::{Rect, Size},
    session_window::Window,
};

impl Target<'_> {
    pub(in crate::desktop) fn content(
        &self,
        image: &Image,
        editor: &Editor,
        geometry: source_resolution::Frame,
        logical: Size,
        input: &Input,
        baseline: Option<&Target<'_>>,
    ) -> Result<source_resolution::Frame> {
        let view = editor.crop_view();
        let crop = view
            .filter(|view| view.legacy_pixel_region().is_none())
            .map(|view| crate::session_crop::Restored {
                frame: Rect {
                    x: 0.0,
                    y: 0.0,
                    ..view.source.rect
                },
                crop: view.source,
                flipped_x: false,
                flipped_y: false,
                rasterized_source: false,
            });
        let visible = editor.annotation_frame().unwrap_or(Rect {
            x: 0.0,
            y: 0.0,
            width: logical.width,
            height: logical.height,
        });
        let edit = if input.content {
            self.layout_edit(input.legacy_crop && crop.is_none(), baseline)?
        } else {
            Value::Null
        };
        let mut window = self.window(visible.width, visible.height);
        if !input.content {
            // Working textures materialize historical layout, but current legacy views remain valid; do not replay old propagation/orientation.
            window.sticker_edit_propagation = None;
        }
        if let Some(pixels) = input.legacy_pixels {
            return geometry.with_legacy_pixels(image, &window, view, pixels, &edit);
        }
        geometry.with_content(image, &window, crop, &edit)
    }

    fn layout_edit(&self, legacy_crop: bool, baseline: Option<&Target<'_>>) -> Result<Value> {
        let mut edit = Map::new();
        // Layout orientation binds to the natural-pixel restoration baseline; clearing canonical flip during target save does not change provenance.
        if let Some(raw) = baseline.unwrap_or(self).edit_state {
            let fields = crate::session_json_fields::fields(raw.get()).map_err(|_| invalid())?;
            for key in ["flippedX", "flippedY"] {
                if let Some(raw) = fields.get(key) {
                    if raw.get().len() > 1024 {
                        return Err(invalid());
                    }
                    edit.insert(
                        key.into(),
                        serde_json::from_str(raw.get()).map_err(|_| invalid())?,
                    );
                }
            }
        }
        if legacy_crop {
            if let Some(raw) = baseline.unwrap_or(self).edit_state {
                let fields =
                    crate::session_json_fields::fields(raw.get()).map_err(|_| invalid())?;
                if let Some(raw) = fields.get("cropRect") {
                    if raw.get().len() > 1024 {
                        return Err(invalid());
                    }
                    edit.insert(
                        "cropRect".into(),
                        serde_json::from_str(raw.get()).map_err(|_| invalid())?,
                    );
                }
            }
        }
        Ok(edit.into())
    }

    /// Construct original history space only for natural sources; never install this crop into the current target Editor.
    pub(in crate::desktop) fn history_mapping(
        &self,
        image: &Image,
    ) -> Result<crate::image_projection::Sampling> {
        let logical = self.logical.ok_or_else(invalid)?;
        let window = self.window(logical.width, logical.height);
        let (crop, rasterized_full) =
            crate::session_crop::restore_with_full_source(&window).map_err(|_| invalid())?;
        let visible_crop = crop
            .map(|crop| crop.editor_crop())
            .transpose()
            .map_err(|_| invalid())?;
        let full = visible_crop.map_or(logical, |crop| crop.source);
        let edit = self.layout_edit(crop.is_none(), None)?;
        source_resolution::Frame::restored(image, full, crop)?
            .with_rasterized_source(
                rasterized_full || crop.is_some_and(|crop| crop.rasterized_source),
            )
            .with_content(image, &window, crop, &edit)?
            .history_mapping(image, visible_crop.map(|crop| crop.rect))
    }

    fn window(&self, width: f64, height: f64) -> Window<'_> {
        // with_content consumes only expanded_rect, propagation geometry, and the small supplied edit; it loads neither src nor history.
        Window {
            id: self.id.to_string(),
            x: 0.0,
            y: 0.0,
            w: width,
            h: height,
            group_id: None,
            node_type: None,
            art_id: None,
            otter_native_local_art: false,
            otter_native_local_art_ports: None,
            port_visibility: None,
            hide_collapsed_endpoints: None,
            minified: None,
            saved_rect: None,
            crop_offset: None,
            opacity_normal: None,
            opacity_mini: None,
            src: None,
            preview_src: None,
            rasterized_annotation_layer_src: None,
            file_path: None,
            params: None,
            outputs: None,
            execution_config: None,
            image_edit_state: self.edit_state,
            sticker_edit_propagation: self.propagation,
            extension_state: None,
            ocr_result: None,
            translation_result: None,
            annotation_state: None,
            otter_native_projection_edit: None,
            capture_meta: None,
            otter_native_capture_original_src: None,
        }
    }
}
