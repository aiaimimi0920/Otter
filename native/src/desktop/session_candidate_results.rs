//! Create idle result owners from decoded session images; keep displayed images separate from official execution ports.
use super::*;
use crate::{
    desktop::loom_result_state::Results, loom_handshake::Handshake, session_art_batch::Entry,
};
#[cfg(test)]
#[path = "session_candidate_output_tests.rs"]
mod output_tests;

pub(in crate::desktop) struct PreparedArts {
    pub arts: Vec<Entry>,
    pub results: Vec<Results>,
}

/// A complete batch of restore resources transferable across threads, holding no UI catalog owner or window.
pub struct PreparedRestore {
    pub vector: VectorCandidate,
    pub(in crate::desktop) arts: PreparedArts,
}
impl PreparedRestore {
    /// Receiver-generated single stickers contain no Art. Consume background pixels and retain collaborative vectors without UI rescanning of inline PNG.
    pub(super) fn projected_sticker<T>(
        bytes: crate::pixels::Pixels,
        unit: UnitId,
        image: crate::image_decode::Image,
        cancel: &AtomicBool,
        retain: impl FnOnce(&session_document::Document<'_>) -> Result<T>,
    ) -> Result<(Self, T)> {
        let source = bytes.into();
        let parsed = parse_source(&source, cancel)?;
        let document = parsed.document();
        let images = vec![Images {
            sticker: Default::default(),
            accept_upstream: true,
            capture_original: None,
            unit,
            source: Some(image),
            preview: None,
            rasterized_annotations: None,
            restored_preview_locked: false,
        }];
        let placements = placements::prepare(document, &images, cancel)?;
        if !matches!(placements.as_slice(), [placement] if !placement.art && placement.unit == images[0].unit)
        {
            return Err(Error::new(E_INVALIDARG, "Projection requires one sticker"));
        }
        let vectors = VectorResources::prepare(&placements, &images, document, cancel)?;
        // Append while borrowing the same document before workspace transfer; send only owned resources to the UI.
        let retained = retain(document)?;
        let candidate = finish_source(parsed, placements, images, cancel)?;
        Ok((
            Self {
                vector: vectors.with_candidate(candidate),
                arts: PreparedArts {
                    arts: Vec::new(),
                    results: Vec::new(),
                },
            },
            retained,
        ))
    }

    pub(in crate::desktop) fn prepare(
        bytes: Vec<u8>,
        root: Option<Root>,
        catalog: Option<&Handshake>,
        cancel: &AtomicBool,
    ) -> Result<Self> {
        let source = bytes.into();
        Self::prepare_parsed(parse_source(&source, cancel)?, root, catalog, cancel)
    }

    pub(in crate::desktop) fn prepare_parsed(
        parsed: crate::session_source::Parsed<'_>,
        root: Option<Root>,
        catalog: Option<&Handshake>,
        cancel: &AtomicBool,
    ) -> Result<Self> {
        // Borrow the document only until all background resources are ready; the final candidate owns the workspace without self-references.
        let document = parsed.document();
        let (placements, images) = prepare_inputs(document, root.as_ref(), cancel)?;
        drop(root); // The external-file root serves only basic image preparation.
        let vectors = VectorResources::prepare(&placements, &images, document, cancel)?;
        let arts = Candidate::prepare_arts_from(&placements, &images, document, catalog, cancel)?;
        let candidate = finish_source(parsed, placements, images, cancel)?;
        Ok(Self {
            vector: vectors.with_candidate(candidate),
            arts,
        })
    }
}

impl Candidate {
    pub(in crate::desktop) fn prepare_arts(
        &self,
        catalog: Option<&Handshake>,
        cancel: &AtomicBool,
    ) -> Result<PreparedArts> {
        Self::prepare_arts_from(
            &self.placements,
            &self.images,
            &self.decode_retained(cancel)?,
            catalog,
            cancel,
        )
    }

    fn prepare_arts_from(
        placements: &[Placement],
        images: &[Images],
        document: &session_document::Document<'_>,
        catalog: Option<&Handshake>,
        cancel: &AtomicBool,
    ) -> Result<PreparedArts> {
        let results = Self::prepare_results_from(placements, images, document, cancel)?;
        let arts = Self::resolve_arts_from(document, catalog, cancel)?;
        Ok(PreparedArts { arts, results })
    }

    #[cfg(test)]
    pub fn prepare_results(&self, cancel: &AtomicBool) -> Result<Vec<Results>> {
        Self::prepare_results_from(
            &self.placements,
            &self.images,
            &self.decode_retained(cancel)?,
            cancel,
        )
    }

    /// Generate only Art state. A pin's base image belongs to Canvas and must not masquerade as an Art result.
    /// Transfer pixels through shared ownership; window installation must still handle edit layers and restored_preview_locked.
    fn prepare_results_from(
        placements: &[Placement],
        images: &[Images],
        document: &session_document::Document<'_>,
        cancel: &AtomicBool,
    ) -> Result<Vec<Results>> {
        check(cancel)?;
        if document.windows.len() != placements.len() || placements.len() != images.len() {
            return Err(Error::new(E_INVALIDARG, "Session result count mismatch"));
        }
        let mut results = Vec::new();
        for ((window, placement), images) in document.windows.iter().zip(placements).zip(images) {
            check(cancel)?;
            if window.id != placement.unit.as_str()
                || images.unit != placement.unit
                || window.is_art() != placement.art
            {
                return Err(Error::new(E_INVALIDARG, "Session result identity mismatch"));
            }
            if placement.art {
                let mut result = Results::from_restored_display(
                    placement.unit.clone(),
                    images.display().cloned(),
                    images.restored_preview_locked,
                );
                if let Some(outputs) = window.outputs {
                    result.restore_outputs(outputs, cancel).map_err(|error| {
                        Error::new(E_INVALIDARG, format!("Invalid saved outputs: {error:?}"))
                    })?;
                }
                results.push(result);
            }
        }
        check(cancel)?;
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::{image::Image, loom_output_batch, loom_result_state::Status};
    #[test]
    fn uncropped_fractional_session_keeps_logical_frame_through_real_composition() {
        let cancel = AtomicBool::new(false);
        let mut candidate = prepare(
            br#"{"stickers":[{"id":"u","x":-30.5,"y":-5.25,"w":7.25,"h":3.25}],"links":[]}"#
                .to_vec(),
            None,
            &cancel,
        )
        .unwrap();
        let image = Image {
            width: 2,
            height: 2,
            pixels: vec![255; 16].into(),
        };
        candidate.images[0].source = Some(image.clone());
        let mut prepared = VectorCandidate::prepare(candidate, &cancel).unwrap();
        let editor = &mut prepared.vectors[0].1.editor;
        let frame = crate::crop_geometry::Rect {
            x: 0.0,
            y: 0.0,
            width: 7.25,
            height: 3.25,
        };
        assert_eq!(editor.annotation_frame(), Some(frame));
        assert_eq!(editor.visible_frame(), Some(frame));
        assert!(editor.crop_view().is_none());
        assert!(!editor.undo());
        assert_eq!(editor.layers().unwrap().size, (8, 4));
        let output = crate::desktop::display_composition::compose(&image, editor, &cancel).unwrap();
        assert_eq!((output.width, output.height), (7, 3));
        assert_eq!(editor.export_snapshot().visible_frame(), Some(frame));
        editor
            .crop_to(crate::crop_geometry::Rect {
                x: 1.0,
                y: 0.5,
                width: 4.0,
                height: 2.0,
            })
            .unwrap();
        assert!(editor.undo());
        assert_eq!(editor.visible_frame(), Some(frame));
        assert!(editor.crop_view().is_none());
    }
    #[test]
    fn assembled_editor_retains_vectors_and_independent_raster_undo() {
        let bytes = serde_json::to_vec(&serde_json::json!({"stickers":[{"id":"u","x":0,"y":0,"w":8,"h":4,
            "annotationState":{"serialCounter":0,"elements":[{"id":"line","type":"line","zIndex":0,
                "points":[{"x":1,"y":1},{"x":3,"y":2}],"style":{"width":1,"color":"#fff"}}]}}],"links":[]})).unwrap();
        let cancel = AtomicBool::new(false);
        let mut candidate = prepare(bytes, None, &cancel).unwrap();
        candidate.images[0].source = Some(Image {
            width: 2,
            height: 2,
            pixels: vec![255; 16].into(),
        });
        candidate.images[0].rasterized_annotations = Some(Image {
            width: 4,
            height: 2,
            pixels: vec![255; 32].into(),
        });
        let mut prepared = VectorCandidate::prepare(candidate, &cancel).unwrap();
        let editor = &mut prepared.vectors[0].1.editor;
        assert_eq!(
            editor.elements()[0].points[0],
            crate::viewport::Point { x: 1.0, y: 1.0 }
        );
        assert!(!editor.undo());
        let before = editor.layers().unwrap().clone();
        assert_eq!(before.size, (8, 4));
        assert_eq!(before.annotation_dimensions(), (4, 2));
        assert_eq!(&before.base[..4], &[0; 4]);
        let next = before
            .erase(
                crate::erase_layers::Mode::RasterAnnotations,
                &[crate::viewport::Point { x: 4.0, y: 2.0 }],
                4.0,
                || false,
            )
            .unwrap();
        assert!(editor
            .replace_layers(editor.revision(), &before, next)
            .unwrap());
        assert!(editor.undo());
        assert!(editor.layers().unwrap().same_storage(&before));
        assert_eq!(editor.elements().len(), 1);
    }
    #[test]
    fn candidate_restores_blur_paths_with_projected_shared_texture() {
        let effect = |id: &str| {
            serde_json::json!({"id":id,"type":"blur","zIndex":0,
            "points":[{"x":1,"y":1},{"x":3,"y":2}],"brushWidth":8,"strength":4.4,
            "style":{"width":1,"color":"#fff"}})
        };
        let bytes = serde_json::to_vec(&serde_json::json!({"stickers":[{"id":"u","x":-50,"y":20,"w":8,"h":4,
            "annotationState":{"serialCounter":0,"elements":[effect("a"),effect("b")]}}],"links":[]})).unwrap();
        let cancel = AtomicBool::new(false);
        let mut candidate = prepare(bytes, None, &cancel).unwrap();
        // Fail without a base image; afterward supply only synthetic test images, never screen or real-session data.
        assert!(candidate.prepare_vectors(&cancel).is_err());
        candidate.images[0].source = Some(Image {
            width: 2,
            height: 2,
            pixels: vec![255; 16].into(),
        });
        let vectors = candidate.prepare_vectors(&cancel).unwrap();
        let strokes = vectors[0].1.editor.elements();
        let first = strokes[0].blur.as_ref().unwrap();
        assert_eq!((first.width(), first.height()), (8, 4));
        assert_eq!(first.strength(), 4.4);
        assert!(first
            .pixels()
            .same_storage(strokes[1].blur.as_ref().unwrap().pixels()));
        assert_eq!(
            strokes[0].points[0],
            crate::viewport::Point { x: 1.0, y: 1.0 }
        );
        assert!(candidate.prepare_vectors(&AtomicBool::new(true)).is_err());
    }
    #[test]
    #[ignore = "must run alone because it reserves the process-wide pixel budget"]
    fn cropped_blur_pressure_reports_resource_failure_and_can_retry() {
        use crate::pixels::{reserve, DECODED_PIXEL_LIMIT};
        let bytes = br##"{"stickers":[{
            "id":"u","x":1,"y":2,"w":4,"h":2,
            "imageEditState":{"cropRect":{"x":2,"y":1,"w":4,"h":2},"sourceSize":{"w":8,"h":4}},
            "annotationState":{"serialCounter":0,"elements":[{
                "id":"b","type":"blur","zIndex":0,"points":[{"x":1,"y":1},{"x":2,"y":1}],
                "brushWidth":2,"strength":2.5,"style":{"width":1,"color":"#fff"}
            }]}}],"links":[]}"##;
        let cancel = AtomicBool::new(false);
        for overlay in [false, true] {
            let mut candidate = prepare(bytes.to_vec(), None, &cancel).unwrap();
            candidate.images[0].source = Some(Image {
                width: 8,
                height: 4,
                pixels: vec![255; 128].into(),
            });
            candidate.images[0].rasterized_annotations = overlay.then(|| Image {
                width: 1,
                height: 1,
                pixels: vec![128; 4].into(),
            });
            let held = reserve(DECODED_PIXEL_LIMIT - 128 - usize::from(overlay) * 4).unwrap();
            let failure = candidate.prepare_vectors(&cancel).err().unwrap();
            assert_eq!(failure.code(), E_OUTOFMEMORY);
            assert_eq!(candidate.original_json(), bytes);
            drop(held);
            let restored = VectorCandidate::prepare(candidate, &cancel).unwrap();
            let editor = &restored.vectors[0].1.editor;
            assert!(editor.crop_view().is_some());
            assert!(editor.elements()[0].blur.is_some());
            assert_eq!(editor.layers().unwrap().annotations.is_some(), overlay);
            drop(restored);
            assert!(reserve(DECODED_PIXEL_LIMIT).is_some());
        }
    }

    #[test]
    fn cropped_vectors_and_blur_texture_share_full_source_coordinates() {
        let bytes = serde_json::to_vec(&serde_json::json!({"stickers":[{
            "id":"u","x":-50,"y":20,"w":4,"h":2,
            "imageEditState":{"cropRect":{"x":2,"y":1,"w":2,"h":1},"sourceSize":{"w":8,"h":4}},
            "annotationState":{"serialCounter":0,"elements":[{
                "id":"b","type":"blur","zIndex":0,"points":[{"x":1,"y":1},{"x":2,"y":1}],
                "brushWidth":2,"strength":2.5,"style":{"width":1,"color":"#fff"}
            }]}}],"links":[]}))
        .unwrap();
        let cancel = AtomicBool::new(false);
        let mut candidate = prepare(bytes.clone(), None, &cancel).unwrap();
        candidate.images[0].source = Some(Image {
            width: 8,
            height: 4,
            pixels: vec![255; 128].into(),
        });
        let overlay = Image {
            width: 2,
            height: 1,
            pixels: vec![128; 8].into(),
        };
        candidate.images[0].rasterized_annotations = Some(overlay.clone());
        let mut vectors = candidate.prepare_vectors(&cancel).unwrap();
        let editor = &mut vectors[0].1.editor;
        let stroke = &editor.elements()[0];
        assert_eq!(stroke.points[0], crate::viewport::Point { x: 5.0, y: 3.0 });
        assert_eq!(stroke.points[1], crate::viewport::Point { x: 6.0, y: 3.0 });
        let blur = stroke.blur.as_ref().unwrap();
        assert_eq!((blur.width(), blur.height()), (16, 8));
        assert!(!editor.undo());
        assert_eq!(candidate.original_json(), bytes);
        let mut assembled = VectorCandidate::prepare(candidate, &cancel).unwrap();
        let editor = &mut assembled.vectors[0].1.editor;
        let view = editor.crop_view().unwrap();
        assert_eq!(view.source.rect.x, 4.0);
        assert_eq!(view.source.rect.y, 2.0);
        assert_eq!(view.source.rect.width, 4.0);
        assert_eq!(view.source.rect.height, 2.0);
        let pixels = editor.layers().unwrap().clone();
        assert_eq!(pixels.size, (16, 8));
        assert_eq!(pixels.annotation_dimensions(), (2, 1));
        assert!(pixels
            .annotations
            .as_ref()
            .unwrap()
            .same_storage(&overlay.pixels));
        assert!(!editor.undo());
        assert!(editor.reset_crop());
        assert_eq!(editor.visible_frame().unwrap().width, 16.0);
        assert!(editor.layers().unwrap().same_storage(&pixels));
        assert_eq!(
            editor.elements()[0].points[0],
            crate::viewport::Point { x: 5.0, y: 3.0 }
        );
        assert!(editor.undo());
        assert_eq!(editor.crop_view(), Some(view));
        assert!(editor.redo());
        assert!(editor.crop_view().is_none());
    }
    #[test]
    fn session_text_uses_real_directwrite_baseline_and_retains_unicode() {
        let bytes = serde_json::to_vec(
            &serde_json::json!({"stickers":[{"id":"u","x":0,"y":0,"w":100,"h":100,
            "annotationState":{"serialCounter":0,"elements":[{"id":"text","type":"text","zIndex":0,
                "x":20,"y":40,"text":"你好 Otter 🦦","fontFamily":"Segoe UI","fontSize":18,
                "style":{"color":"#123456","width":1}}]}}],"links":[]}),
        )
        .unwrap();
        let cancel = AtomicBool::new(false);
        let candidate = prepare(bytes, None, &cancel).unwrap();
        let vectors = candidate.prepare_vectors(&cancel).unwrap();
        let stroke = &vectors[0].1.editor.elements()[0];
        let text = stroke.text.as_ref().unwrap();
        let measured = crate::desktop::annotation_text::measure_flip_metrics(text).unwrap();
        assert_eq!(text.content(), "你好 Otter 🦦");
        assert!((stroke.points[0].y + measured.baseline_from_top - 40.0).abs() < 0.001);
        assert_eq!(stroke.points[0].x + 4.0, 20.0);
        assert!(stroke.points[1].x > stroke.points[0].x);
    }
    fn candidate() -> Candidate {
        prepare(
            br#"{"stickers":[{"id":"art","type":"art","x":0,"y":0,"w":1,"h":1},
            {"id":"sticker","x":0,"y":0,"w":1,"h":1}],"links":[]}"#
                .to_vec(),
            None,
            &AtomicBool::new(false),
        )
        .unwrap()
    }
    #[test]
    fn restored_preview_is_shared_but_cannot_be_used_as_formal_output() {
        let cancel = AtomicBool::new(false);
        let mut candidate = candidate();
        let preview = Image {
            width: 1,
            height: 1,
            pixels: vec![1, 2, 3, 255].into(),
        };
        candidate.images[0].source = Some(Image {
            width: 1,
            height: 1,
            pixels: vec![4, 5, 6, 255].into(),
        });
        candidate.images[0].preview = Some(preview.clone());
        candidate.images[0].restored_preview_locked = true;
        let mut results = candidate.prepare_results(&cancel).unwrap();
        assert_eq!(results.len(), 1);
        let result = &mut results[0];
        assert_eq!(result.status(), Status::Idle);
        assert!(result.holds_restored_preview());
        assert!(result.active().is_none() && result.ports().is_empty());
        assert!(result
            .display()
            .unwrap()
            .pixels
            .same_storage(&preview.pixels));
        assert!(result.image_source("output_image").is_none());
        let request = crate::graph_requests::Request {
            id: UnitId::parse("request").unwrap(),
            generation: 1,
        };
        result.begin(request.clone());
        assert!(!result.holds_restored_preview());
        assert!(result.fail(&request, "test failure"));
        assert!(result
            .display()
            .unwrap()
            .pixels
            .same_storage(&preview.pixels));
        result.begin(request.clone());
        let batch = loom_output_batch::decode(
            &serde_json::json!({"text":{"kind":"value","value":"done"}}),
            None,
            &cancel,
        )
        .unwrap();
        result.finish(&request, batch).unwrap();
        assert_eq!(result.status(), Status::Completed);
        assert_eq!(result.ports().len(), 1);
        assert!(result.image_source("output_image").is_none());
        assert!(result
            .display()
            .unwrap()
            .pixels
            .same_storage(&preview.pixels));
        drop(candidate);
        assert_eq!(result.display().unwrap().pixels.as_ref(), &[1, 2, 3, 255]);
    }
    #[test]
    fn invalid_image_identity_and_cancel_return_no_partial_results() {
        let mut candidate = candidate();
        let cancel = AtomicBool::new(true);
        assert!(candidate.prepare_results(&cancel).is_err());
        cancel.store(false, Ordering::Release);
        candidate.images[1].unit = candidate.images[0].unit.clone();
        assert!(candidate.prepare_results(&cancel).is_err());
        assert_eq!(candidate.placements.len(), 2);
    }
}
