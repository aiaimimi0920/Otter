//! 旧自然像素采样是裁剪视图的一部分；历史交换完整视图，底图与标注不被烘焙。
use super::*;
use crate::crop_geometry::{Crop, Rect, Size};
use crate::image_projection::Projection;

impl CropView {
    /// 完整工作纹理已 contain 到逻辑框；仅映射其中的有效源区，不放大整图placement。
    /// nested crop只改变clip，完整底图owner不变，仍可reset/undo。
    pub(crate) fn base_projection(&self, _raster: (u32, u32)) -> Option<Projection> {
        let pixels = self.legacy_pixels?;
        let natural = self.legacy_natural_size?;
        let full = self.source.source;
        let scale = (full.width / natural.width).min(full.height / natural.height);
        let x = (full.width - natural.width * scale) / 2.0 + pixels.x * scale;
        let y = (full.height - natural.height * scale) / 2.0 + pixels.y * scale;
        Some(Projection {
            source: Rect {
                x,
                y,
                width: pixels.width * scale,
                height: pixels.height * scale,
            },
            target: Rect {
                x: 0.0,
                y: 0.0,
                width: full.width,
                height: full.height,
            },
        })
    }
    /// 返回当前可见部分的自然像素区域；再次裁剪只组合坐标，不复制图片。
    pub fn legacy_pixel_region(&self) -> Option<Rect> {
        let original = self.legacy_pixels?;
        Some(Rect {
            x: original.x + self.source.rect.x / self.source.source.width * original.width,
            y: original.y + self.source.rect.y / self.source.source.height * original.height,
            width: self.source.rect.width / self.source.source.width * original.width,
            height: self.source.rect.height / self.source.source.height * original.height,
        })
    }
}

impl Editor {
    pub fn raster_base_projection(&self) -> Option<Projection> {
        self.crop?.base_projection(self.raster.as_ref()?.size)
    }
    /// retained自然图失效后保存的是完整工作纹理，采样元数据必须一同换基准。
    pub(crate) fn raster_crop_view(&self) -> Option<CropView> {
        let mut view = self.crop?;
        if let Some(mapping) = self.raster_base_projection() {
            let size = self.raster.as_ref()?.size;
            view.legacy_pixels = Some(mapping.source);
            view.legacy_natural_size = Some(Size {
                width: size.0 as f64,
                height: size.1 as f64,
            });
        }
        Some(view)
    }
    /// 只供新文档恢复；允许源区域部分或全部越界，交由有界采样保留透明。
    pub fn with_restored_legacy_crop(
        mut self,
        pixels: Rect,
        natural: Size,
    ) -> Result<Self, EditError> {
        let frame = self.full_logical_frame().ok_or(EditError::InvalidPoint)?;
        if self.crop.is_some()
            || self.raster.is_none()
            || self.revision != 0
            || !self.history.is_empty()
            || self.draft.is_some()
            || !valid_region(pixels)
            || !valid_region(Rect {
                x: 0.0,
                y: 0.0,
                width: natural.width,
                height: natural.height,
            })
        {
            return Err(EditError::InvalidPoint);
        }
        let view = CropView {
            frame,
            source: Crop {
                source: Size {
                    width: frame.width,
                    height: frame.height,
                },
                rect: frame,
            },
            legacy_pixels: Some(pixels),
            legacy_natural_size: Some(natural),
        };
        if !view
            .base_projection(self.raster.as_ref().unwrap().size)
            .is_some_and(|mapping| {
                valid_region(mapping.source)
                    && valid_region(mapping.target)
                    && Projection::clipped(
                        mapping.source,
                        mapping.target,
                        self.raster.as_ref().unwrap().size,
                    )
                    .is_some()
            })
        {
            return Err(EditError::InvalidPoint);
        }
        self.crop = Some(view);
        Ok(self)
    }
}

fn valid_region(rect: Rect) -> bool {
    rect.width > 0.0
        && rect.height > 0.0
        && [
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            rect.x + rect.width,
            rect.y + rect.height,
        ]
        .iter()
        .all(|value| value.is_finite() && value.abs() <= 1_000_000.0)
}

#[cfg(test)]
#[path = "editor_crop_bounded_tests.rs"]
mod bounded_tests;
#[cfg(test)]
#[path = "editor_crop_mapping_tests.rs"]
mod mapping_tests;

#[cfg(test)]
mod tests {
    use super::*;
    fn editor() -> Editor {
        Editor::with_layers(crate::erase_layers::Layers {
            size: (8, 8),
            base: vec![255; 256].into(),
            annotations: None,
            annotation_size: None,
        })
        .unwrap()
        .with_restored_size(Size {
            width: 8.0,
            height: 8.0,
        })
        .unwrap()
    }
    fn region() -> Rect {
        Rect {
            x: 2.0,
            y: 0.0,
            width: 2.0,
            height: 8.0,
        }
    }
    #[test]
    fn legacy_view_reset_and_history_preserve_complete_base_owner() {
        let mut editor = editor()
            .with_restored_legacy_crop(
                region(),
                Size {
                    width: 8.0,
                    height: 8.0,
                },
            )
            .unwrap();
        let original = editor.layers().unwrap().base.clone();
        assert_eq!(editor.revision(), 0);
        assert_eq!(
            editor.crop_view().unwrap().legacy_pixel_region(),
            Some(region())
        );
        assert!(editor.reset_crop());
        assert!(editor.crop_view().is_none());
        assert!(editor.layers().unwrap().base.same_storage(&original));
        assert!(editor.undo());
        assert_eq!(
            editor.crop_view().unwrap().legacy_pixel_region(),
            Some(region())
        );
        assert!(editor.redo());
        assert!(editor.crop_view().is_none());
        assert!(!editor.reset_crop());
    }
    #[test]
    fn nested_crop_composes_pixels_and_undo_restores_exact_sampling() {
        let mut editor = editor()
            .with_restored_legacy_crop(
                region(),
                Size {
                    width: 8.0,
                    height: 8.0,
                },
            )
            .unwrap();
        editor
            .crop_to(Rect {
                x: 2.0,
                y: 2.0,
                width: 4.0,
                height: 4.0,
            })
            .unwrap();
        assert_eq!(
            editor.crop_view().unwrap().legacy_pixel_region(),
            Some(Rect {
                x: 2.5,
                y: 2.0,
                width: 1.0,
                height: 4.0,
            })
        );
        assert!(editor.undo());
        assert_eq!(
            editor.crop_view().unwrap().legacy_pixel_region(),
            Some(region())
        );
        assert!(editor.redo());
        assert!(editor
            .export_snapshot()
            .crop_view()
            .unwrap()
            .legacy_pixel_region()
            .is_some());
    }
    #[test]
    fn invalid_or_late_install_never_creates_an_active_view() {
        for pixels in [
            Rect {
                width: 0.0,
                ..region()
            },
            Rect {
                x: f64::NAN,
                ..region()
            },
        ] {
            assert!(editor()
                .with_restored_legacy_crop(
                    pixels,
                    Size {
                        width: 8.0,
                        height: 8.0
                    }
                )
                .is_err());
        }
        let mut edited = editor();
        edited.begin(Tool::Brush, Point { x: 1.0, y: 1.0 }).unwrap();
        edited.commit();
        assert!(edited
            .with_restored_legacy_crop(
                region(),
                Size {
                    width: 8.0,
                    height: 8.0
                }
            )
            .is_err());
        assert!(editor()
            .with_restored_legacy_crop(
                Rect {
                    x: -100.0,
                    ..region()
                },
                Size {
                    width: 8.0,
                    height: 8.0
                }
            )
            .is_ok());
    }
}
