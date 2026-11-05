//! 翻转标注保持文字可读及盒形对象朝向；路径/效果镜像采样点，不翻转共享效果像素。
use super::*;
use crate::{crop_geometry::Rect, raster_flip::Axis};

/// 由布局层提供的文字锚点，均为未缩放逻辑单位，不含对象旋转。
#[derive(Debug, Clone, Copy)]
pub struct TextFlipMetrics {
    pub advance_width: f32,
    pub left_inset: f32,
    pub baseline_from_top: f32,
}

impl Stroke {
    /// frame 使用保留源坐标，允许框外标注；不截断负坐标或改变绘制顺序。
    /// 文字/序号采用原生布局已测量的包围框，不在后台调用字体 API。
    pub fn flipped_in_frame(&self, frame: Rect, axis: Axis) -> Result<Self, EditError> {
        self.flipped_with_text_metrics(frame, axis, None)
    }

    pub fn flipped_with_text_metrics(
        &self,
        frame: Rect,
        axis: Axis,
        metrics: Option<TextFlipMetrics>,
    ) -> Result<Self, EditError> {
        if !frame.x.is_finite()
            || !frame.y.is_finite()
            || !frame.width.is_finite()
            || !frame.height.is_finite()
            || frame.width <= 0.0
            || frame.height <= 0.0
        {
            return Err(EditError::InvalidPoint);
        }
        let mirror = |point: Point| -> Point {
            Point {
                x: if axis == Axis::Horizontal {
                    (2.0 * frame.x + frame.width - point.x as f64) as f32
                } else {
                    point.x
                },
                y: if axis == Axis::Vertical {
                    (2.0 * frame.y + frame.height - point.y as f64) as f32
                } else {
                    point.y
                },
            }
        };
        let boxed = matches!(
            self.tool,
            Tool::Rectangle
                | Tool::RoundedRectangle
                | Tool::Ellipse
                | Tool::Triangle
                | Tool::Polygon
                | Tool::Text
                | Tool::Serial
        );
        let offset = if self.tool == Tool::Text {
            let metrics = metrics.ok_or(EditError::InvalidPoint)?;
            let text = self.text.as_ref().ok_or(EditError::InvalidPoint)?;
            if self.points.len() != 2
                || !metrics.advance_width.is_finite()
                || metrics.advance_width < 0.0
                || !metrics.left_inset.is_finite()
                || !metrics.baseline_from_top.is_finite()
                || metrics.baseline_from_top < 0.0
            {
                return Err(EditError::InvalidPoint);
            }
            let start = self.points[0];
            // 主干 x'=W-x-textWidth，y'=H-y+fontSize；先转成文字锚点再转回布局框。
            match axis {
                Axis::Horizontal => Point {
                    x: (2.0 * frame.x + frame.width
                        - 2.0 * (start.x as f64 + metrics.left_inset as f64)
                        - metrics.advance_width as f64) as f32,
                    y: 0.0,
                },
                Axis::Vertical => Point {
                    x: 0.0,
                    y: (2.0 * frame.y + frame.height
                        - 2.0 * (start.y as f64 + metrics.baseline_from_top as f64)
                        + text.font_size() as f64) as f32,
                },
            }
        } else if boxed {
            if self.points.len() != 2 {
                return Err(EditError::InvalidPoint);
            }
            let center = Point {
                x: ((self.points[0].x as f64 + self.points[1].x as f64) / 2.0) as f32,
                y: ((self.points[0].y as f64 + self.points[1].y as f64) / 2.0) as f32,
            };
            let reflected = mirror(center);
            Point {
                x: reflected.x - center.x,
                y: reflected.y - center.y,
            }
        } else {
            Point::default()
        };
        let transformed = |point: Point| {
            if boxed {
                Point {
                    x: point.x + offset.x,
                    y: point.y + offset.y,
                }
            } else {
                mirror(point)
            }
        };
        for &point in &self.points {
            validate(transformed(point))?;
        }
        let mut result = self.clone();
        for point in &mut result.points {
            *point = transformed(*point);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame() -> Rect {
        Rect {
            x: 10.0,
            y: 5.0,
            width: 100.0,
            height: 60.0,
        }
    }
    #[test]
    fn shapes_move_without_mirroring_orientation_style_or_rotated_geometry() {
        for tool in [
            Tool::Rectangle,
            Tool::RoundedRectangle,
            Tool::Ellipse,
            Tool::Triangle,
            Tool::Polygon,
        ] {
            let mut editor = Editor::default();
            editor.begin(tool, Point { x: 20.0, y: 15.0 }).unwrap();
            editor.extend(Point { x: 40.0, y: 35.0 }).unwrap();
            editor.commit();
            editor.rotate(0, 25.0).unwrap();
            let source = &editor.elements()[0];
            let flipped = source.flipped_in_frame(frame(), Axis::Horizontal).unwrap();
            assert_eq!(
                flipped.points,
                vec![Point { x: 80.0, y: 15.0 }, Point { x: 100.0, y: 35.0 }]
            );
            assert_eq!(flipped.rotation, 25.0);
            assert_eq!(flipped.style(), source.style());
            assert_eq!(
                flipped
                    .flipped_in_frame(frame(), Axis::Horizontal)
                    .unwrap()
                    .points,
                source.points
            );
            assert_eq!(source.points[0], Point { x: 20.0, y: 15.0 });
        }
    }
    #[test]
    fn paths_and_effects_mirror_points_and_keep_shared_blur_source() {
        let mut editor = Editor::default();
        let image =
            crate::blur_image::BlurImage::with_strength(4, 4, vec![128; 64].into(), 2.4).unwrap();
        editor
            .begin_blur(Point { x: -10.0, y: 15.0 }, image.clone())
            .unwrap();
        editor.extend(Point { x: 30.0, y: 25.0 }).unwrap();
        editor.commit();
        let source = &editor.elements()[0];
        let flipped = source.flipped_in_frame(frame(), Axis::Horizontal).unwrap();
        assert_eq!(flipped.points[0], Point { x: 130.0, y: 15.0 });
        assert_eq!(flipped.points[1], Point { x: 90.0, y: 25.0 });
        assert!(flipped
            .blur
            .as_ref()
            .unwrap()
            .pixels()
            .same_storage(image.pixels()));
        assert_eq!(
            flipped
                .flipped_in_frame(frame(), Axis::Horizontal)
                .unwrap()
                .points,
            source.points
        );
        for tool in [
            Tool::Line,
            Tool::Arrow,
            Tool::Bezier,
            Tool::Brush,
            Tool::Highlighter,
            Tool::Mosaic,
        ] {
            let mut editor = Editor::default();
            editor.begin(tool, Point { x: 20.0, y: 15.0 }).unwrap();
            editor.extend(Point { x: 30.0, y: 25.0 }).unwrap();
            editor.commit();
            let source = &editor.elements()[0];
            let flipped = source.flipped_in_frame(frame(), Axis::Vertical).unwrap();
            for (before, after) in source.points.iter().zip(&flipped.points) {
                assert_eq!(after.x, before.x);
                assert_eq!(after.y, 70.0 - before.y);
            }
            assert_eq!(flipped.brush_weights, source.brush_weights);
        }
    }
    #[test]
    fn measured_text_and_serial_remain_readable_and_invalid_input_is_atomic() {
        let mut editor = Editor::default();
        let text =
            crate::text_annotation::TextAnnotation::new("Text", "Segoe UI", 16.0, false).unwrap();
        editor
            .insert_text(Point { x: 20.0, y: 15.0 }, Point { x: 30.0, y: 20.0 }, text)
            .unwrap();
        editor
            .insert_serial(Point { x: 30.0, y: 25.0 }, 10.0, "Segoe UI")
            .unwrap();
        for source in editor.elements() {
            let metrics = (source.tool == Tool::Text).then_some(TextFlipMetrics {
                advance_width: 22.0,
                left_inset: 4.0,
                baseline_from_top: 17.0,
            });
            let flipped = source
                .flipped_with_text_metrics(frame(), Axis::Vertical, metrics)
                .unwrap();
            if source.tool == Tool::Text {
                assert!(source.flipped_in_frame(frame(), Axis::Vertical).is_err());
                // 原基线 15+17=32；主干镜像基线为 70-32+16=54。
                assert_eq!(flipped.points[0].y + 17.0, 54.0);
                let horizontal = source
                    .flipped_with_text_metrics(frame(), Axis::Horizontal, metrics)
                    .unwrap();
                // 原文本 x=20+4=24；新文本 x=120-24-22=74。
                assert_eq!(horizontal.points[0].x + 4.0, 74.0);
            }
            assert_eq!(flipped.text, source.text);
            assert_eq!(
                flipped.points[1].y - flipped.points[0].y,
                source.points[1].y - source.points[0].y
            );
            assert_eq!(
                flipped
                    .flipped_with_text_metrics(frame(), Axis::Vertical, metrics)
                    .unwrap()
                    .points,
                source.points
            );
            let bad = Rect {
                x: f64::INFINITY,
                ..frame()
            };
            assert!(source.flipped_in_frame(bad, Axis::Horizontal).is_err());
            let too_far = Rect {
                x: 1_000_000.0,
                ..frame()
            };
            assert!(source.flipped_in_frame(too_far, Axis::Horizontal).is_err());
        }
    }
}
