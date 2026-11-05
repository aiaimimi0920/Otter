//! 缩放预览复用点缓冲；排版、样式和全部坐标先验证，失败保持上一有效候选。
use super::*;
impl Stroke {
    pub(crate) fn update_resize_preview(
        &mut self,
        original: &Self,
        factors: Point,
        measured: Option<Point>,
        pivot: Option<Point>,
    ) -> Result<(), EditError> {
        if self.tool != original.tool
            || self.points.len() != original.points.len()
            || original.points.is_empty()
            || !factors.x.is_finite()
            || !factors.y.is_finite()
            || factors.x <= 0.0
            || factors.y <= 0.0
        {
            return Err(EditError::InvalidPoint);
        }
        if let Some(pivot) = pivot {
            validate(pivot)?;
        }
        if let Some(size) = measured {
            if original.tool != Tool::Text
                || original.points.len() != 2
                || !size.x.is_finite()
                || !size.y.is_finite()
                || size.x <= 0.0
                || size.y <= 0.0
            {
                return Err(EditError::InvalidPoint);
            }
        }
        let factor = (factors.x + factors.y) / 2.0;
        let mut style = original
            .style()
            .with_width_value(original.width * factor)
            .and_then(|s| s.with_corner_radius_value(original.corner_radius * factor))
            .ok_or(EditError::BudgetExceeded)?;
        if original.tool == Tool::Mosaic {
            style = style
                .with_mosaic_cell_value(original.mosaic_cell * factor)
                .ok_or(EditError::BudgetExceeded)?;
        }
        let text = original
            .text
            .as_ref()
            .map(|text| {
                text.with_font_size(text.font_size() * factor)
                    .ok_or(EditError::BudgetExceeded)
            })
            .transpose()?;
        let center = crate::rotation::center(&original.points);
        let path = pivot.is_some()
            && !matches!(
                original.tool,
                Tool::Rectangle
                    | Tool::RoundedRectangle
                    | Tool::Ellipse
                    | Tool::Triangle
                    | Tool::Polygon
                    | Tool::Text
                    | Tool::Serial
            );
        let anchor = pivot.unwrap_or(center);
        let offset = Point {
            x: (center.x - anchor.x) * (factors.x - 1.0),
            y: (center.y - anchor.y) * (factors.y - 1.0),
        };
        let geometry = if original.tool == Tool::Serial {
            Point {
                x: factor,
                y: factor,
            }
        } else {
            factors
        };
        let transform = |index: usize, source: Point| {
            if path {
                let source = crate::rotation::rotate(source, center, original.rotation);
                Point {
                    x: anchor.x + (source.x - anchor.x) * factors.x,
                    y: anchor.y + (source.y - anchor.y) * factors.y,
                }
            } else if let Some(size) = measured {
                let sign = if index == 0 { -0.5 } else { 0.5 };
                Point {
                    x: center.x + size.x * sign + offset.x,
                    y: center.y + size.y * sign + offset.y,
                }
            } else {
                Point {
                    x: center.x + (source.x - center.x) * geometry.x + offset.x,
                    y: center.y + (source.y - center.y) * geometry.y + offset.y,
                }
            }
        };
        for (index, &point) in original.points.iter().enumerate() {
            validate(transform(index, point))?;
        }
        for (index, (point, &source)) in self.points.iter_mut().zip(&original.points).enumerate() {
            *point = transform(index, source);
        }
        self.text = text;
        self.rotation = if path { 0.0 } else { original.rotation };
        self.apply_style(style);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_previews_keep_allocation_and_match_group_commit() {
        for tool in [Tool::Rectangle, Tool::Mosaic, Tool::Line] {
            let mut editor = Editor::default();
            editor.begin(tool, Point { x: 10.0, y: 10.0 }).unwrap();
            editor.extend(Point { x: 20.0, y: 30.0 }).unwrap();
            editor.commit();
            editor.rotate(0, 30.0).unwrap();
            let original = editor.elements[0].clone();
            let mut preview = original.clone();
            let allocation = preview.points.as_ptr();
            for factor in [1.1, 1.2, 0.8, 2.0] {
                preview
                    .update_resize_preview(
                        &original,
                        Point { x: factor, y: 1.0 },
                        None,
                        Some(Point::default()),
                    )
                    .unwrap();
                assert_eq!(preview.points.as_ptr(), allocation);
            }
            editor
                .resize_many_axes_around(
                    &[0],
                    Point::default(),
                    Point { x: 2.0, y: 1.0 },
                    |_| unreachable!(),
                )
                .unwrap();
            assert_eq!(preview.points, editor.elements[0].points);
            assert_eq!(preview.rotation, editor.elements[0].rotation);
            assert_eq!(preview.style(), editor.elements[0].style());
            let previous = preview.points.clone();
            let previous_style = preview.style();
            assert!(preview
                .update_resize_preview(
                    &original,
                    Point {
                        x: f32::NAN,
                        y: 1.0
                    },
                    None,
                    None
                )
                .is_err());
            assert_eq!(preview.points, previous);
            assert!(preview
                .update_resize_preview(
                    &original,
                    Point { x: 2.0, y: 2.0 },
                    None,
                    Some(Point {
                        x: -1_000_000.0,
                        y: -1_000_000.0
                    })
                )
                .is_err());
            assert_eq!(preview.points, previous);
            assert_eq!(preview.style(), previous_style);
        }
    }
}
