//! Rectangle redactions retain their four perimeter vertices through arbitrary
//! document transforms. They never fall back to a polyline mask.
use super::*;
impl Editor {
    pub fn mark_effect_rect(&mut self) -> Result<(), EditError> {
        if self.retained_points + 4 > MAX_TOTAL_POINTS {
            self.cancel();
            return Err(EditError::BudgetExceeded);
        }
        let stroke = self.draft.as_mut().ok_or(EditError::InvalidPoint)?;
        if !matches!(stroke.tool, Tool::Mosaic | Tool::Blur) || stroke.points.len() != 1 {
            return Err(EditError::InvalidPoint);
        }
        stroke.effect_rect = true;
        stroke.points.resize(4, stroke.points[0]);
        Ok(())
    }
    pub(crate) fn restored_effect_rect(&mut self) -> Result<(), EditError> {
        let stroke = self.elements.last_mut().ok_or(EditError::InvalidPoint)?;
        if !matches!(stroke.tool, Tool::Mosaic | Tool::Blur) || !valid(&stroke.points) {
            return Err(EditError::InvalidPoint);
        }
        stroke.effect_rect = true;
        Ok(())
    }
}
pub(super) fn extend(stroke: &mut Stroke, p: Point) -> bool {
    let a = stroke.points[0];
    if stroke.points[2] == p {
        return false;
    }
    stroke
        .points
        .copy_from_slice(&[a, Point { x: p.x, y: a.y }, p, Point { x: a.x, y: p.y }]);
    true
}
/// A rotated, mirrored or nonuniformly scaled rectangle is a convex quadrilateral.
pub(crate) fn valid(p: &[Point]) -> bool {
    if p.len() != 4 {
        return false;
    }
    let cross = |i: usize| {
        let (a, b, c) = (p[i], p[(i + 1) % 4], p[(i + 2) % 4]);
        (f64::from(b.x) - f64::from(a.x)) * (f64::from(c.y) - f64::from(b.y))
            - (f64::from(b.y) - f64::from(a.y)) * (f64::from(c.x) - f64::from(b.x))
    };
    (0..4).all(|i| cross(i) > 0.0) || (0..4).all(|i| cross(i) < 0.0)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_drag_directions_and_flat_cancel_cost_exactly_one_or_zero_undo() {
        for (x, y) in [
            (1., 1.),
            (-1., 1.),
            (1., -1.),
            (-1., -1.),
            (0., 3.),
            (3., 0.),
        ] {
            let mut e = Editor::default();
            e.begin(Tool::Mosaic, Point::default()).unwrap();
            e.mark_effect_rect().unwrap();
            e.extend(Point { x, y }).unwrap();
            assert_eq!(e.commit(), x * y != 0.);
            if x * y != 0. {
                assert!(valid(&e.elements()[0].points));
                assert!(e.undo());
                assert!(e.redo());
                assert!(e.elements()[0].effect_rect);
            } else {
                assert!(!e.undo());
            }
        }
    }
}
