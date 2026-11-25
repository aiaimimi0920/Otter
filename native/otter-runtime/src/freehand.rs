//! 普通画笔的基线采样与提交平滑；三轮邻域平滑原地执行，不克隆点数组。
use crate::viewport::Point;
pub const MAX_POINTS: usize = 2048;

pub fn smooth(points: &mut [Point], strength: f32) {
    for p in points.iter_mut() {
        p.x = p.x.clamp(-100_000.0, 100_000.0);
        p.y = p.y.clamp(-100_000.0, 100_000.0);
    }
    let strength = if strength.is_finite() {
        strength.clamp(0.0, 100.0) / 100.0
    } else {
        0.0
    };
    if strength == 0.0 || points.len() < 3 {
        return;
    }
    for _ in 0..3 {
        let mut previous = points[0];
        for i in 1..points.len() - 1 {
            let current = points[i];
            let next = points[i + 1];
            points[i] = Point {
                x: current.x * (1.0 - strength / 2.0) + (previous.x + next.x) * strength / 4.0,
                y: current.y * (1.0 - strength / 2.0) + (previous.y + next.y) * strength / 4.0,
            };
            previous = current;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::{Editor, Tool};
    #[test]
    fn paintbrush_retains_long_path_instead_of_pencil_halving() {
        let mut editor = Editor::default();
        editor.begin(Tool::Paintbrush, Point::default()).unwrap();
        for x in 1..3000 {
            editor
                .extend(Point {
                    x: x as f32,
                    y: 0.0,
                })
                .unwrap();
        }
        assert_eq!(editor.draft().unwrap().points.len(), 3000);
        editor.commit_smoothed(50.0);
        assert_eq!(editor.elements()[0].points.len(), 3000);
        editor.undo();
        editor.redo();
        assert_eq!(editor.elements()[0].points.len(), 3000);
    }
    #[test]
    fn brush_sampling_skips_subpixel_motion_and_halves_at_baseline_limit() {
        let mut editor = Editor::default();
        editor.begin(Tool::Brush, Point::default()).unwrap();
        assert!(!editor.extend(Point { x: 0.5, y: 0.0 }).unwrap());
        for x in 1..2049 {
            editor
                .extend(Point {
                    x: x as f32,
                    y: 0.0,
                })
                .unwrap();
        }
        let points = &editor.draft().unwrap().points;
        assert_eq!(points.len(), 1025);
        assert_eq!(points[1], Point { x: 2.0, y: 0.0 });
        assert_eq!(points.last(), Some(&Point { x: 2048.0, y: 0.0 }));
    }
    #[test]
    fn smoothing_is_baked_once_and_undo_redo_keep_same_points() {
        let mut editor = Editor::default();
        editor.begin(Tool::Brush, Point::default()).unwrap();
        editor.extend(Point { x: 10.0, y: 10.0 }).unwrap();
        editor.extend(Point { x: 20.0, y: 0.0 }).unwrap();
        assert_eq!(editor.draft().unwrap().points[1].y, 10.0);
        editor.commit_smoothed(100.0);
        assert_eq!(editor.elements()[0].points[1].y, 1.25);
        editor.undo();
        assert!(editor.elements().is_empty());
        editor.redo();
        assert_eq!(editor.elements()[0].points[1].y, 1.25);
        editor.begin(Tool::Highlighter, Point::default()).unwrap();
        editor.extend(Point { x: 10.0, y: 10.0 }).unwrap();
        editor.extend(Point { x: 20.0, y: 0.0 }).unwrap();
        editor.commit_smoothed(100.0);
        assert_eq!(editor.elements()[1].points[1].y, 10.0);
    }
    #[test]
    fn three_passes_preserve_endpoints_and_match_baseline_weighting() {
        let mut points = [
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 10.0 },
            Point { x: 20.0, y: 0.0 },
        ];
        smooth(&mut points, 100.0);
        assert_eq!(
            points,
            [
                Point { x: 0.0, y: 0.0 },
                Point { x: 10.0, y: 1.25 },
                Point { x: 20.0, y: 0.0 }
            ]
        );
    }
}
