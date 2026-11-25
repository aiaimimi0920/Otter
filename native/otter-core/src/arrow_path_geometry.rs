//! Multipoint arrows follow the last valid anchor segment; borrow prefixes/store one head without sample copies or length-dependent allocation.
use super::{straight, Arrow};
use crate::viewport::Point;

pub struct PathArrow<'a> {
    prefix: &'a [Point],
    pub arrow: Arrow,
}
impl PathArrow<'_> {
    pub fn shaft(&self) -> impl Iterator<Item = Point> + '_ {
        self.prefix
            .iter()
            .copied()
            .chain((self.prefix.last() != Some(&self.arrow.base)).then_some(self.arrow.base))
    }
}

/// Match live Web annotations: first find an anchor at least 2 units from the endpoint, otherwise use the last distinct point.
/// Repeated/tiny trailing samples stay inside the head without folded-back shafts; one point/all coincident points produce no head.
pub fn build_path(points: &[Point], width: f32) -> Option<PathArrow<'_>> {
    let &end = points.last()?;
    if points.len() < 2 || !width.is_finite() || width <= 0.0 {
        return None;
    }
    if points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
        return None;
    }
    let candidates = &points[..points.len() - 1];
    let anchor = candidates
        .iter()
        .rposition(|p| (end.x - p.x).hypot(end.y - p.y) >= 2.0)
        .or_else(|| candidates.iter().rposition(|p| *p != end))?;
    Some(PathArrow {
        prefix: &points[..=anchor],
        arrow: straight(points[anchor], end, width)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bent_arrow_preserves_prefix_and_trims_only_final_effective_segment() {
        let points = [
            Point { x: 8.0, y: 8.0 },
            Point { x: 8.0, y: 48.0 },
            Point { x: 56.0, y: 48.0 },
            Point { x: 56.0, y: 48.0 },
        ];
        let path = build_path(&points, 4.0).unwrap();
        assert_eq!(
            path.shaft().collect::<Vec<_>>(),
            [points[0], points[1], Point { x: 32.0, y: 48.0 }]
        );
        assert_eq!(
            path.arrow.head,
            [
                points[2],
                Point { x: 32.0, y: 58.0 },
                Point { x: 32.0, y: 38.0 }
            ]
        );
        assert_eq!(points.len(), 4);
    }
    #[test]
    fn tiny_terminal_samples_use_web_two_unit_anchor_without_empty_or_duplicate_shaft() {
        let points = [
            Point::default(),
            Point { x: 10.0, y: 0.0 },
            Point { x: 10.5, y: 0.0 },
        ];
        let path = build_path(&points, 4.0).unwrap();
        assert_eq!(path.shaft().collect::<Vec<_>>(), [Point::default()]);
        assert_eq!(path.arrow.base, Point::default());
        assert!(build_path(&points[..1], 4.0).is_none());
        assert!(build_path(&[Point::default(); 3], 4.0).is_none());
        assert!(build_path(&points, 0.0).is_none());
        assert!(build_path(
            &[
                Point {
                    x: f32::NAN,
                    y: 0.0
                },
                points[1]
            ],
            4.0
        )
        .is_none());
        let tiny = [Point::default(), Point { x: 0.5, y: 0.0 }];
        assert_eq!(build_path(&tiny, 4.0).unwrap().arrow.head[0], tiny[1]);
    }
}
