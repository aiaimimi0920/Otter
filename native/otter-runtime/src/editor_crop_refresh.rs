//! 裁剪候选与依赖可见框的模糊作为单一事务；准备/计算不改变现有文档。
use super::*;
use crate::crop_geometry::{self, Rect};

impl Editor {
    /// Some 是当前可见框中的局部选区，None 表示重置；返回计划必须后台 compute 后提交。
    pub fn prepare_crop_refresh(
        &self,
        selection: Option<Rect>,
    ) -> Result<Option<RasterRefreshPlan>, EditError> {
        let frame = self.visible_frame().ok_or(EditError::InvalidPoint)?;
        if self.draft.is_some() {
            return Ok(None);
        }
        let next = if let Some(selection) = selection {
            if selection.x < 0.0
                || selection.y < 0.0
                || selection.x + selection.width > frame.width
                || selection.y + selection.height > frame.height
            {
                return Err(EditError::InvalidPoint);
            }
            let change = crop_geometry::next(frame, self.crop.map(|view| view.source), selection)
                .map_err(|_| EditError::InvalidPoint)?;
            if selection.x == 0.0
                && selection.y == 0.0
                && selection.width == frame.width
                && selection.height == frame.height
            {
                return Ok(None);
            }
            Some(CropView {
                frame: change.frame,
                source: change.crop.ok_or(EditError::InvalidPoint)?,
                legacy_pixels: self.crop.and_then(|view| view.legacy_pixels),
                legacy_natural_size: self.crop.and_then(|view| view.legacy_natural_size),
            })
        } else {
            if self.crop.is_none() {
                return Ok(None);
            }
            None
        };
        Ok(self
            .prepare_raster_edit()?
            .map(|plan| plan.with_crop_target(next)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn document() -> Editor {
        let mut editor = Editor::with_layers(crate::erase_layers::Layers {
            annotation_size: None,
            size: (16, 16),
            base: vec![0; 1024].into(),
            annotations: Some(
                (0..256)
                    .flat_map(|i| {
                        if i % 16 < 8 {
                            [0, 0, 255, 255]
                        } else {
                            [0, 255, 0, 255]
                        }
                    })
                    .collect::<Vec<_>>()
                    .into(),
            ),
        })
        .unwrap();
        editor
            .begin_blur(
                Point { x: 10.0, y: 6.0 },
                crate::blur_image::BlurImage::with_strength(16, 16, vec![128; 1024].into(), 2.4)
                    .unwrap(),
            )
            .unwrap();
        assert!(editor.commit());
        editor
    }
    fn rect() -> Rect {
        Rect {
            x: 8.0,
            y: 4.0,
            width: 8.0,
            height: 4.0,
        }
    }
    #[test]
    fn reset_refresh_uses_fractional_full_frame_for_overlay_and_blur() {
        let layers = document().layers().unwrap().clone();
        let full = Rect {
            x: 0.0,
            y: 0.0,
            width: 15.25,
            height: 15.25,
        };
        let mut editor = Editor::with_layers(layers.clone())
            .unwrap()
            .with_restored_crop(crate::crop_geometry::Crop {
                rect: rect(),
                source: crate::crop_geometry::Size {
                    width: full.width,
                    height: full.height,
                },
            })
            .unwrap();
        editor
            .begin_blur(
                Point { x: 10.0, y: 6.0 },
                crate::blur_image::BlurImage::new(16, 16, vec![128; 1024].into(), 2).unwrap(),
            )
            .unwrap();
        assert!(editor.commit());
        let plan = editor
            .prepare_crop_refresh(None)
            .unwrap()
            .unwrap()
            .compute(|| false)
            .unwrap();
        assert!(editor.commit_raster_refresh(plan).unwrap());
        assert_eq!(editor.annotation_frame(), Some(full));
        let composed =
            crate::raster_source::compose_in_frame(&layers, Some(full), || false).unwrap();
        let expected = crate::blur::region_precise(
            &composed,
            (16, 16),
            crate::blur::Region {
                x: 0,
                y: 0,
                width: 16,
                height: 16,
            },
            2.0,
            || false,
        )
        .unwrap();
        assert_eq!(
            &editor.elements()[0].blur.as_ref().unwrap().pixels()[..],
            &expected[..]
        );
        assert!(editor.undo());
        assert_eq!(editor.annotation_frame(), Some(rect()));
        assert!(editor.redo());
        assert_eq!(editor.annotation_frame(), Some(full));
    }
    #[test]
    fn crop_and_reset_swap_effects_with_view_in_one_history_entry() {
        let mut editor = document();
        let source = editor.layers().unwrap().clone();
        let original = editor.elements()[0].blur.as_ref().unwrap().clone();
        let revision = editor.revision();
        let plan = editor
            .prepare_crop_refresh(Some(rect()))
            .unwrap()
            .unwrap()
            .compute(|| false)
            .unwrap();
        assert!(editor.crop_view().is_none());
        assert_eq!(editor.revision(), revision);
        assert!(editor.commit_raster_refresh(plan).unwrap());
        assert_eq!(editor.revision(), revision + 1);
        let crop = editor.crop_view();
        let cropped = editor.elements()[0].blur.as_ref().unwrap().clone();
        let composed =
            crate::raster_source::compose_in_frame(&source, Some(rect()), || false).unwrap();
        let expected = crate::blur::region_precise(
            &composed,
            (16, 16),
            crate::blur::Region {
                x: 0,
                y: 0,
                width: 16,
                height: 16,
            },
            2.4,
            || false,
        )
        .unwrap();
        assert_eq!(&cropped.pixels()[..], &expected[..]);
        assert!(editor.layers().unwrap().same_storage(&source));
        for _ in 0..3 {
            assert!(editor.undo());
            assert!(editor.crop_view().is_none());
            assert!(editor.elements()[0]
                .blur
                .as_ref()
                .unwrap()
                .pixels()
                .same_storage(original.pixels()));
            assert!(editor.redo());
            assert_eq!(editor.crop_view(), crop);
            assert!(editor.elements()[0]
                .blur
                .as_ref()
                .unwrap()
                .pixels()
                .same_storage(cropped.pixels()));
        }
        let reset = editor
            .prepare_crop_refresh(None)
            .unwrap()
            .unwrap()
            .compute(|| false)
            .unwrap();
        assert!(editor.commit_raster_refresh(reset).unwrap());
        assert!(editor.crop_view().is_none());
        assert!(editor.undo());
        assert_eq!(editor.crop_view(), crop);
        assert!(editor.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .pixels()
            .same_storage(cropped.pixels()));
    }
    #[test]
    fn plain_image_crop_transaction_preserves_pixels_and_restores_view() {
        let mut editor = Editor::with_layers(crate::erase_layers::Layers {
            annotation_size: None,
            size: (16, 16),
            base: vec![255; 1024].into(),
            annotations: None,
        })
        .unwrap();
        let source = editor.layers().unwrap().clone();
        let plan = editor
            .prepare_crop_refresh(Some(rect()))
            .unwrap()
            .unwrap()
            .compute(|| false)
            .unwrap();
        assert!(editor.commit_raster_refresh(plan).unwrap());
        assert!(editor.layers().unwrap().same_storage(&source));
        assert!(editor.undo());
        assert!(editor.crop_view().is_none());
        assert!(editor.redo());
        assert_eq!(editor.crop_view().unwrap().source.rect, rect());
        assert!(editor.elements().is_empty());
    }

    #[test]
    fn cancellation_stale_result_noop_and_bad_geometry_do_not_change_view() {
        let mut editor = document();
        assert!(editor.prepare_crop_refresh(None).unwrap().is_none());
        assert!(editor
            .prepare_crop_refresh(Some(Rect {
                width: 0.0,
                ..rect()
            }))
            .is_err());
        let plan = editor.prepare_crop_refresh(Some(rect())).unwrap().unwrap();
        assert!(matches!(
            plan.compute(|| true),
            Err(crate::blur::BlurError::Cancelled)
        ));
        let plan = editor
            .prepare_crop_refresh(Some(rect()))
            .unwrap()
            .unwrap()
            .compute(|| false)
            .unwrap();
        assert!(editor.undo());
        assert!(!editor.commit_raster_refresh(plan).unwrap());
        assert!(editor.crop_view().is_none());
        assert!(editor.redo());
    }
}
