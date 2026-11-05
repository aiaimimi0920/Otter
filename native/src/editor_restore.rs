//! 只供会话转换器构造新文档的基础矢量入口；不经过交互采样，避免丢弃紧邻的持久点。
use super::*;
impl Editor {
    /// 恢复的逻辑画框独立于向上取整的像素缓冲；不创建一次假的裁剪或历史。
    pub fn with_restored_size(
        mut self,
        size: crate::crop_geometry::Size,
    ) -> Result<Self, EditError> {
        let layers = self.raster.as_ref().ok_or(EditError::InvalidPoint)?;
        if self.logical_size.is_some()
            || self.crop.is_some()
            || self.revision != 0
            || !self.history.is_empty()
            || self.draft.is_some()
            || !size.width.is_finite()
            || !size.height.is_finite()
            || size.width <= 0.0
            || size.height <= 0.0
            || size.width.ceil() != layers.size.0 as f64
            || size.height.ceil() != layers.size.1 as f64
        {
            return Err(EditError::InvalidPoint);
        }
        self.logical_size = Some(size);
        Ok(self)
    }
    /// 安装已归一到完整源坐标的新文档裁剪视图，不裁掉像素、不创建导入历史。
    pub fn with_restored_crop(
        mut self,
        crop: crate::crop_geometry::Crop,
    ) -> Result<Self, EditError> {
        if self.crop.is_some()
            || self.revision != 0
            || !self.history.is_empty()
            || self.draft.is_some()
        {
            return Err(EditError::InvalidPoint);
        }
        crate::crop_geometry::reset(crop.rect, Some(crop)).map_err(|_| EditError::InvalidPoint)?;
        let layers = self.raster.as_ref().ok_or(EditError::InvalidPoint)?;
        if crop.source.width.ceil() != layers.size.0 as f64
            || crop.source.height.ceil() != layers.size.1 as f64
        {
            return Err(EditError::InvalidPoint);
        }
        self.logical_size = Some(crop.source);
        self.crop = Some(CropView {
            frame: crop.rect,
            source: crop,
            legacy_pixels: None,
            legacy_natural_size: None,
        });
        Ok(self)
    }
    /// 新建恢复文档从可见框平移回完整源框；效果纹理必须由调用方先按完整源框准备。
    pub fn with_restored_offset(mut self, offset: Point) -> Result<Self, EditError> {
        if self.raster.is_some()
            || self.crop.is_some()
            || self.revision != 0
            || !self.history.is_empty()
            || self.draft.is_some()
        {
            return Err(EditError::InvalidPoint);
        }
        validate(offset)?;
        for stroke in &mut self.elements {
            for point in &mut stroke.points {
                let translated = Point {
                    x: point.x + offset.x,
                    y: point.y + offset.y,
                };
                validate(translated)?;
                *point = translated;
            }
        }
        Ok(self)
    }
    /// 只合并尚未发布的新文档，避免绕过正常历史替换活动栅格；坐标对齐由恢复层负责。
    pub fn with_restored_layers(
        mut self,
        layers: crate::erase_layers::Layers,
    ) -> Result<Self, EditError> {
        if !layers.valid()
            || self.raster.is_some()
            || self.crop.is_some()
            || self.revision != 0
            || !self.history.is_empty()
            || self.draft.is_some()
        {
            return Err(EditError::InvalidPoint);
        }
        self.raster = Some(layers);
        Ok(self)
    }
    pub(crate) fn append_restored_blur(
        &mut self,
        points: Vec<Point>,
        rotation: f32,
        image: crate::blur_image::BlurImage,
    ) -> Result<(), EditError> {
        self.append_restored(Tool::Brush, points, rotation, Vec::new())?;
        let stroke = self.elements.last_mut().expect("appended blur path");
        stroke.tool = Tool::Blur;
        stroke.blur = Some(image);
        Ok(())
    }
    pub(crate) fn append_restored_text(
        &mut self,
        points: Vec<Point>,
        rotation: f32,
        text: crate::text_annotation::TextAnnotation,
        serial: bool,
    ) -> Result<(), EditError> {
        self.append_restored(Tool::Rectangle, points, rotation, Vec::new())?;
        let stroke = self.elements.last_mut().expect("appended text bounds");
        stroke.tool = if serial { Tool::Serial } else { Tool::Text };
        stroke.text = Some(text);
        Ok(())
    }
    pub(crate) fn append_restored(
        &mut self,
        tool: Tool,
        points: Vec<Point>,
        rotation: f32,
        brush_weights: Vec<f32>,
    ) -> Result<(), EditError> {
        if !self.history.is_empty() || self.draft.is_some() || self.revision != 0 {
            return Err(EditError::InvalidPoint);
        }
        let count = points.len();
        let valid_count = match tool {
            Tool::Rectangle
            | Tool::RoundedRectangle
            | Tool::Ellipse
            | Tool::Triangle
            | Tool::Polygon => count == 2,
            Tool::Line | Tool::Arrow | Tool::Polyline => count >= 1,
            Tool::Bezier => (4..=193).contains(&count) && (count - 1).is_multiple_of(3),
            Tool::Brush | Tool::Highlighter | Tool::Paintbrush | Tool::Mosaic => count >= 1,
            _ => false,
        };
        if !valid_count || !rotation.is_finite() || rotation.abs() > 36000.0 {
            return Err(EditError::InvalidPoint);
        }
        if brush_weights.len() > MAX_POINTS_PER_STROKE
            || brush_weights.iter().any(|weight| !weight.is_finite())
            || (tool != Tool::Paintbrush && !brush_weights.is_empty())
        {
            return Err(EditError::InvalidPoint);
        }
        if count > MAX_POINTS_PER_STROKE
            || self.elements.len() >= MAX_ELEMENTS
            || self.retained_points + count > MAX_TOTAL_POINTS
        {
            return Err(EditError::BudgetExceeded);
        }
        for point in &points {
            validate(*point)?;
        }
        self.elements.push(Stroke {
            annotation_id: annotation_id::AnnotationId::new()?,
            tool,
            points,
            width: self.style.width(),
            color: self.style.color(),
            opacity: self.style.opacity(),
            fill: self.style.fill(),
            paint_alpha: self.style.paint_alpha(),
            corner_radius: self.style.corner_radius(),
            dash: self.style.dash(),
            sides: self.style.sides(),
            rotation,
            arrow: self.style.arrow(),
            brush_weights,
            text: None,
            mosaic_cell: self.style.mosaic_cell(),
            blur: None,
        });
        self.retained_points += count;
        Ok(())
    }
    pub(crate) fn finish_restore(mut self, serial_counter: u32) -> Self {
        self.serial_count = serial_counter;
        self.style = StrokeStyle::default();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crop_geometry::{Crop, Rect, Size};
    fn crop() -> Crop {
        Crop {
            rect: Rect {
                x: 1.25,
                y: 0.5,
                width: 2.5,
                height: 1.5,
            },
            source: Size {
                width: 7.5,
                height: 3.75,
            },
        }
    }
    fn document() -> Editor {
        Editor::with_layers(crate::erase_layers::Layers {
            size: (8, 4),
            base: vec![255; 128].into(),
            annotation_size: None,
            annotations: None,
        })
        .unwrap()
    }
    #[test]
    fn restored_size_rejects_invalid_buffers_and_active_documents() {
        let size = Size {
            width: 7.25,
            height: 3.25,
        };
        assert!(Editor::default().with_restored_size(size).is_err());
        for invalid in [
            Size {
                width: f64::NAN,
                ..size
            },
            Size {
                height: 0.0,
                ..size
            },
            Size {
                width: 8.25,
                ..size
            },
            Size {
                height: f64::INFINITY,
                ..size
            },
        ] {
            assert!(document().with_restored_size(invalid).is_err());
        }
        let restored = document().with_restored_size(size).unwrap();
        assert!(restored.with_restored_size(size).is_err());
        let mut active = document();
        active.begin(Tool::Line, Point { x: 1.0, y: 1.0 }).unwrap();
        assert!(active.with_restored_size(size).is_err());
        let mut active = document();
        active.begin(Tool::Brush, Point { x: 1.0, y: 1.0 }).unwrap();
        active.commit();
        assert!(active.with_restored_size(size).is_err());
        assert!(document()
            .with_restored_crop(crop())
            .unwrap()
            .with_restored_size(size)
            .is_err());
    }
    #[test]
    fn fractional_full_frame_survives_reset_snapshot_adoption_and_history() {
        let mut source = document().with_restored_crop(crop()).unwrap();
        let pixels = source.layers().unwrap().clone();
        let view = source.crop_view();
        assert!(source.reset_crop());
        let full = Rect {
            x: 0.0,
            y: 0.0,
            width: 7.5,
            height: 3.75,
        };
        assert_eq!(source.visible_frame(), Some(full));
        assert_eq!(source.export_snapshot().visible_frame(), Some(full));
        assert!(source.undo());
        assert_eq!(source.crop_view(), view);
        assert!(source.redo());
        assert_eq!(source.visible_frame(), Some(full));
        assert!(source.layers().unwrap().same_storage(&pixels));
        let mut target = document();
        assert_eq!(target.visible_frame().unwrap().width, 8.0);
        assert!(target.adopt_snapshot(target.revision(), &source).unwrap());
        assert_eq!(target.visible_frame(), Some(full));
        assert!(target.undo());
        assert_eq!(target.visible_frame().unwrap().width, 8.0);
        assert!(target.redo());
        assert_eq!(target.visible_frame(), Some(full));
        assert!(target
            .crop_to(Rect {
                x: 0.25,
                y: 0.25,
                width: 2.5,
                height: 1.5
            })
            .unwrap());
        assert_eq!(target.crop_view().unwrap().source.source, crop().source);
        assert!(target.reset_crop());
        assert_eq!(target.visible_frame(), Some(full));
    }
    #[test]
    fn restored_crop_preserves_fractional_view_and_rejects_wrong_source_or_active_document() {
        let mut editor = document().with_restored_crop(crop()).unwrap();
        assert_eq!(editor.visible_frame(), Some(crop().rect));
        assert_eq!(editor.crop_view().unwrap().source.source, crop().source);
        assert!(!editor.undo());
        assert!(Editor::default().with_restored_crop(crop()).is_err());
        let mut wrong = crop();
        wrong.source.width = 9.0;
        assert!(document().with_restored_crop(wrong).is_err());
        assert!(editor.with_restored_crop(crop()).is_err());
        let mut active = document();
        active.begin(Tool::Line, Point { x: 1.0, y: 1.0 }).unwrap();
        assert!(active.with_restored_crop(crop()).is_err());
        let mut active = document();
        active
            .crop_to(Rect {
                x: 1.0,
                y: 1.0,
                width: 2.0,
                height: 2.0,
            })
            .unwrap();
        active.reset_crop();
        assert!(active.with_restored_crop(crop()).is_err());
    }
}
