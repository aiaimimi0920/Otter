//! 从已解码会话图像建立空闲结果 owner；显示图与正式执行端口保持分离。
use super::*;
use crate::desktop::loom_result_state::Results;
#[cfg(test)]
#[path = "session_candidate_output_tests.rs"]
mod output_tests;

impl Candidate {
    /// 仅生成 Art 状态。钉图底图属于 Canvas，不应伪装为 Art 结果。
    /// 像素通过共享所有权传递；窗口安装仍需处理编辑层与 restored_preview_locked。
    pub fn prepare_results(&self, cancel: &AtomicBool) -> Result<Vec<Results>> {
        check(cancel)?;
        if self.images.len() != self.placements.len() {
            return Err(Error::new(E_INVALIDARG, "Session image count mismatch"));
        }
        let mut results = Vec::new();
        let document = session_document::decode(&self.original)
            .map_err(|_| Error::new(E_INVALIDARG, "Invalid retained session results"))?;
        for placement in &self.placements {
            check(cancel)?;
            let mut matches = self
                .images
                .iter()
                .filter(|images| images.unit == placement.unit);
            let images = matches
                .next()
                .ok_or_else(|| Error::new(E_INVALIDARG, "Missing session image record"))?;
            if matches.next().is_some() || !self.workspace.contains(&placement.unit) {
                return Err(Error::new(E_INVALIDARG, "Session result identity mismatch"));
            }
            if placement.art {
                if results
                    .iter()
                    .any(|result: &Results| result.unit() == &placement.unit)
                {
                    return Err(Error::new(
                        E_INVALIDARG,
                        "Duplicate session result identity",
                    ));
                }
                let mut result = Results::from_restored_display(
                    placement.unit.clone(),
                    images.display().cloned(),
                    images.restored_preview_locked,
                );
                let window = document
                    .windows
                    .iter()
                    .find(|window| window.id == placement.unit.as_str())
                    .ok_or_else(|| Error::new(E_INVALIDARG, "Missing retained result identity"))?;
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
        // 未提供底图时失败；之后只填入合成测试图，不读取屏幕或真实会话。
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
