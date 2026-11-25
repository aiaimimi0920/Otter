//! 平移预览复用已克隆的点缓冲，每次从原始几何计算；组拖动只在开始时扫一遍边界。
use super::*;

/// Raw point extrema (not visual/rotated bounds), computed once from the original selection.
pub struct TranslationBounds {
    low: Point,
    high: Point,
}

impl TranslationBounds {
    pub fn new(mut points: impl Iterator<Item = Point>) -> Option<Self> {
        let first = points.next()?;
        let mut low = first;
        let mut high = first;
        for point in points {
            low.x = low.x.min(point.x);
            low.y = low.y.min(point.y);
            high.x = high.x.max(point.x);
            high.y = high.y.max(point.y);
        }
        Some(Self { low, high })
    }

    pub fn permits(&self, offset: Point) -> bool {
        // Use the same f32 addition and limits as commit, including rounding at the boundary.
        [
            offset,
            translated(self.low, offset),
            translated(self.high, offset),
        ]
        .into_iter()
        .all(|point| validate(point).is_ok())
    }
}

fn translated(point: Point, offset: Point) -> Point {
    Point {
        x: point.x + offset.x,
        y: point.y + offset.y,
    }
}
impl Stroke {
    pub fn update_translation_preview(
        &mut self,
        original: &Self,
        offset: Point,
    ) -> Result<(), EditError> {
        validate(offset)?;
        if self.points.len() != original.points.len() || self.tool != original.tool {
            return Err(EditError::InvalidPoint);
        }
        for &point in &original.points {
            validate(translated(point, offset))?;
        }
        self.apply_valid_translation_preview(original, offset);
        Ok(())
    }

    /// The group owner validates its extrema before changing any preview and keeps the source stamp.
    pub fn apply_valid_translation_preview(&mut self, original: &Self, offset: Point) {
        debug_assert_eq!(self.points.len(), original.points.len());
        debug_assert_eq!(self.tool, original.tool);
        for (point, &source) in self.points.iter_mut().zip(&original.points) {
            *point = translated(source, offset);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_extrema_match_point_by_point_admission_at_f32_limits() {
        for x in [
            -1_000_000.0,
            -999_999.94,
            -320.75,
            0.0,
            671.0,
            999_999.94,
            1_000_000.0,
        ] {
            let points = [
                Point { x, y: -5.5 },
                Point { x: x * 0.5, y: x },
                Point { x: -671.0, y: -x },
            ];
            let bounds = TranslationBounds::new(points.into_iter()).unwrap();
            for dx in [
                -1_000_000.0,
                -0.125,
                -0.0625,
                0.0,
                0.03125,
                0.0625,
                0.125,
                1_000_000.0,
                f32::NAN,
                f32::INFINITY,
            ] {
                for dy in [-0.125, 0.0, 0.125, 1_000_000.0, f32::NEG_INFINITY] {
                    let offset = Point { x: dx, y: dy };
                    let accepted = validate(offset).is_ok()
                        && points.iter().all(|p| {
                            validate(Point {
                                x: p.x + offset.x,
                                y: p.y + offset.y,
                            })
                            .is_ok()
                        });
                    assert_eq!(bounds.permits(offset), accepted, "{points:?} {offset:?}");
                }
            }
        }
        assert!(TranslationBounds::new(std::iter::empty()).is_none());
    }
}
