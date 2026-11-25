//! Source-resolution baking for restored candidates; do not reacquire from active windows or guess source-image identity.
use super::*;
#[path = "session_workflow_prepare.rs"]
pub mod workflow;

impl VectorCandidate {
    /// Accept only restored candidates without further edits, preventing retained originals from overwriting new erasures; explicitly reject unimplemented effects.
    pub fn bake_source_png(
        &self,
        unit: &UnitId,
        cancel: &AtomicBool,
    ) -> Result<crate::pixels::Pixels> {
        check(cancel)?;
        let document = self.candidate.decode_retained(cancel)?;
        let (index, window) = document
            .windows
            .iter()
            .enumerate()
            .find(|(_, window)| window.id == unit.as_str())
            .ok_or_else(|| Error::new(E_INVALIDARG, "Missing bake session node"))?;
        self.bake_window_image_png(index, window, None, cancel)
    }

    /// Candidates follow source-session order; indexed access still checks identity to avoid fetching another node's resources if order differs.
    fn bake_window_image_png(
        &self,
        index: usize,
        window: &crate::session_window::Window<'_>,
        selected: Option<&crate::desktop::image::Image>,
        cancel: &AtomicBool,
    ) -> Result<crate::pixels::Pixels> {
        check(cancel)?;
        let mut border = None;
        let mut radius = 0.0;
        let mut edit_state = serde_json::Value::Null;
        if let Some(raw) = window.image_edit_state {
            let edit: serde_json::Value = serde_json::from_str(raw.get())
                .map_err(|_| Error::new(E_INVALIDARG, "Invalid bake edit state"))?;
            border = super::super::export::source_resolution::frame_border::Border::read(&edit)?;
            radius = super::super::export::source_resolution::frame_clip::radius(&edit)?;
            // The mainline compositor does not read imageEditState.rotation; this is legacy metadata and must not cause
            // session rejection or invented whole-image rotation here. The annotation drawing path handles each annotation's own rotation.
            edit_state = edit;
        }
        let placement = self
            .candidate
            .placements
            .get(index)
            .filter(|value| value.unit.as_str() == window.id)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Missing bake placement"))?;
        if placement.art {
            return Err(Error::new(E_INVALIDARG, "Art result is not a sticker bake"));
        }
        let editor = &self
            .vectors
            .get(index)
            .filter(|(id, _)| *id == placement.unit)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Missing bake editor"))?
            .1
            .editor;
        if editor.revision() != 0 || editor.draft().is_some() {
            return Err(Error::new(
                E_ABORT,
                "Bake candidate changed after restoration",
            ));
        }
        let images = self
            .candidate
            .images
            .get(index)
            .filter(|images| images.unit == placement.unit)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Missing bake images"))?;
        let source = selected
            .or(images.source.as_ref())
            .or(images.preview.as_ref())
            .ok_or_else(|| Error::new(E_INVALIDARG, "Missing bake source"))?;
        // The marker belongs to this window's stored src, never a selected preview,
        // manual image or upstream replacement.
        let stored = images
            .source
            .as_ref()
            .is_some_and(|stored| std::ptr::eq(stored, source));
        let crop = placement.crop.map(|mut crop| {
            crop.rasterized_source &= stored;
            crop
        });
        let logical = if let Some(crop) = placement.crop {
            crop.editor_crop()
                .map_err(|_| Error::new(E_INVALIDARG, "Invalid bake crop"))?
                .source
        } else {
            let frame = placement
                .expanded_rect()
                .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid bake frame"))?;
            crate::crop_geometry::Size {
                width: frame.w,
                height: frame.h,
            }
        };
        let geometry =
            super::super::export::source_resolution::Frame::restored(source, logical, crop)?
                .with_rasterized_source(
                    (stored && placement.rasterized_full)
                        || crop.is_some_and(|crop| crop.rasterized_source),
                )
                .with_beautify(&edit_state)?;
        // Mainline blur sampling uses the whole unit box, not propagated content bounds; do not translate it with the base image.
        let prepared = super::super::export::source_resolution::source_blur::prepare_restored(
            source, editor, &geometry, window, crop, cancel,
        )?;
        let mut geometry = geometry.with_content(source, window, crop, &edit_state)?;
        let erased = super::super::export::source_resolution::source_erase::prepare(
            source,
            &mut geometry,
            editor.crop_view().map(|crop| crop.source.rect),
            &edit_state,
            cancel,
        )?;
        super::super::export::source_resolution::png_source_resolution(
            &erased,
            &prepared,
            geometry,
            placement.opacity_normal as f32,
            border,
            radius,
            cancel,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn workflow_rasterized_crop_stretches_only_the_stored_source() {
        let _com = crate::desktop::Apartment::new().unwrap();
        let cancel = AtomicBool::new(false);
        let stored = crate::desktop::image::Image {
            width: 8,
            height: 8,
            pixels: [
                [0; 4],
                [0; 4],
                [20, 40, 60, 255],
                [70, 80, 90, 255],
                [0; 4],
                [0; 4],
                [0; 4],
                [0; 4],
            ]
            .concat()
            .repeat(8)
            .into(),
        };
        let png = crate::desktop::export::png_raw(&stored, &cancel).unwrap();
        let url = crate::loom_base64::encode_png_data_url(&png, &cancel).unwrap();
        let raw = serde_json::to_vec(&json!({"documentRevision":1,"stickers":[{
            "id":"s","type":"sticker","x":0,"y":0,"w":8,"h":4,"src":url.as_str(),
            "imageEditState":{"cropRect":{"x":2,"y":2,"w":8,"h":4},
                "sourceSize":{"w":16,"h":8},"otterNativeRasterizedCropSource":true}
        }],"links":[]}))
        .unwrap();
        let candidate =
            VectorCandidate::prepare(prepare(raw, None, &cancel).unwrap(), &cancel).unwrap();
        let snapshot = candidate.prepare_workflow_snapshot(&[], &cancel).unwrap();
        let image = crate::desktop::session_image::inline(
            snapshot.snapshot().value()["nodes"][0]["data"]["src"]
                .as_str()
                .unwrap(),
            &cancel,
        )
        .unwrap();
        assert_eq!((image.width, image.height), (4, 4));
        for row in image.pixels.chunks_exact(16) {
            assert_eq!(row, &stored.pixels[4..20]);
        }

        let natural = crate::desktop::image::Image {
            width: 8,
            height: 8,
            pixels: [1, 2, 3, 255].repeat(64).into(),
        };
        let doc = candidate.candidate.decode_retained(&cancel).unwrap();
        let baked = candidate
            .bake_window_image_png(0, &doc.windows[0], Some(&natural), &cancel)
            .unwrap();
        let image = crate::desktop::export::decode_png(baked.to_vec());
        assert_eq!((image.width, image.height), (8, 4));
        for row in image.pixels.chunks_exact(32) {
            assert_eq!(&row[..8], &[0; 8]);
            assert_eq!(&row[8..], &[1, 2, 3, 255].repeat(6));
        }
    }
    #[test]
    fn restored_workflow_bake_high_zoom_uses_natural_crop_pixels() {
        let _com = crate::desktop::Apartment::new().unwrap();
        let cancel = AtomicBool::new(false);
        let mut candidate = prepare(
            br#"{"stickers":[{"id":"s","x":0,"y":0,"w":512,"h":48,
            "imageEditState":{"cropRect":{"x":64,"y":2,"w":64,"h":3},
            "sourceSize":{"w":4096,"h":8}}}],"links":[]}"#
                .to_vec(),
            None,
            &cancel,
        )
        .unwrap();
        let mut pixels = [10, 20, 30, 255].repeat(4096 * 8);
        for row in pixels.chunks_exact_mut(4096 * 4) {
            row[64 * 4..128 * 4].copy_from_slice(&[90, 100, 110, 255].repeat(64));
        }
        candidate.images[0].source = Some(crate::desktop::image::Image {
            width: 4096,
            height: 8,
            pixels: pixels.into(),
        });
        let candidate = VectorCandidate::prepare(candidate, &cancel).unwrap();
        let unit = UnitId::parse("s").unwrap();
        let png = candidate.bake_source_png(&unit, &cancel).unwrap();
        let decoded = crate::desktop::export::decode_png(png.to_vec());
        assert_eq!((decoded.width, decoded.height), (64, 3));
        for (index, pixel) in decoded.pixels.chunks_exact(4).enumerate() {
            assert_eq!(pixel, [90, 100, 110, 255], "pixel {index}");
        }
        cancel.store(true, Ordering::Release);
        assert_eq!(
            candidate
                .bake_source_png(&unit, &cancel)
                .unwrap_err()
                .code(),
            E_ABORT
        );
    }

    #[test]
    fn restored_candidate_bakes_natural_size_and_rejects_later_edits() {
        let _com = crate::desktop::Apartment::new().unwrap();
        let cancel = AtomicBool::new(false);
        let mut candidate = prepare(
            br#"{"stickers":[{"id":"s","x":0,"y":0,"w":20,"h":10}],"links":[]}"#.to_vec(),
            None,
            &cancel,
        )
        .unwrap();
        let pixels = [10, 20, 30, 255].repeat(40 * 20);
        candidate.images[0].source = Some(crate::desktop::image::Image {
            width: 40,
            height: 20,
            pixels: pixels.clone().into(),
        });
        let mut candidate = VectorCandidate::prepare(candidate, &cancel).unwrap();
        let unit = UnitId::parse("s").unwrap();
        let png = candidate.bake_source_png(&unit, &cancel).unwrap();
        let decoded = crate::desktop::export::decode_png(png.to_vec());
        assert_eq!((decoded.width, decoded.height), (40, 20));
        assert_eq!(decoded.pixels.as_ref(), pixels);
        candidate.vectors[0]
            .1
            .editor
            .crop_to(crate::crop_geometry::Rect {
                x: 1.0,
                y: 1.0,
                width: 5.0,
                height: 5.0,
            })
            .unwrap();
        assert!(candidate.bake_source_png(&unit, &cancel).is_err());
    }
}
