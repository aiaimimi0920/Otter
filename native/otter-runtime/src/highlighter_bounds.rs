//! 共享荧光层的可见范围；无临时点数组，包含旋转、笔宽和抗锯齿安全边。
use crate::{
    editor::{Stroke, Tool},
    rotation,
    viewport::{Point, Viewport},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

pub fn visible<'a>(
    strokes: impl IntoIterator<Item = (usize, &'a Stroke)>,
    viewport: Viewport,
    preview: Option<(usize, Point)>,
    clip: Bounds,
) -> Option<Bounds> {
    let mut union: Option<Bounds> = None;
    for (index, stroke) in strokes {
        if stroke.tool != Tool::Highlighter || stroke.opacity <= 0.0 {
            continue;
        }
        let Some(&first) = stroke.points.first() else {
            continue;
        };
        let (mut low, mut high) = (first, first);
        for point in &stroke.points {
            low.x = low.x.min(point.x);
            low.y = low.y.min(point.y);
            high.x = high.x.max(point.x);
            high.y = high.y.max(point.y);
        }
        let center = Point {
            x: (low.x + high.x) / 2.0,
            y: (low.y + high.y) / 2.0,
        };
        let pad = stroke.width / 2.0;
        let rotated = rotation::bounds(
            (low.x - pad, low.y - pad, high.x + pad, high.y + pad),
            center,
            stroke.rotation,
        );
        let offset = preview
            .filter(|(selected, _)| *selected == index)
            .map_or(Point::default(), |(_, offset)| offset);
        let x = viewport.origin.x + offset.x * viewport.scale;
        let y = viewport.origin.y + offset.y * viewport.scale;
        let bounds = Bounds {
            left: (x + rotated.0 * viewport.scale - 2.0).max(clip.left),
            top: (y + rotated.1 * viewport.scale - 2.0).max(clip.top),
            right: (x + rotated.2 * viewport.scale + 2.0).min(clip.right),
            bottom: (y + rotated.3 * viewport.scale + 2.0).min(clip.bottom),
        };
        if bounds.left >= bounds.right || bounds.top >= bounds.bottom {
            continue;
        }
        union = Some(match union {
            None => bounds,
            Some(previous) => Bounds {
                left: previous.left.min(bounds.left),
                top: previous.top.min(bounds.top),
                right: previous.right.max(bounds.right),
                bottom: previous.bottom.max(bounds.bottom),
            },
        });
    }
    union
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::Editor;
    fn document() -> Editor {
        let mut editor = Editor::default();
        editor.set_style(editor.style().with_width(4.0).unwrap());
        editor
            .begin(Tool::Highlighter, Point { x: 20.0, y: 20.0 })
            .unwrap();
        editor.extend(Point { x: 40.0, y: 20.0 }).unwrap();
        editor.commit();
        editor
    }
    fn viewport() -> Viewport {
        Viewport {
            origin: Point::default(),
            scale: 1.0,
        }
    }
    #[test]
    fn small_stroke_does_not_request_full_target_layer() {
        let editor = document();
        assert_eq!(
            visible(
                editor.elements().iter().enumerate(),
                viewport(),
                None,
                Bounds {
                    left: 0.0,
                    top: 0.0,
                    right: 4000.0,
                    bottom: 3000.0,
                }
            ),
            Some(Bounds {
                left: 16.0,
                top: 16.0,
                right: 44.0,
                bottom: 24.0
            })
        );
    }
    #[test]
    fn preview_rotation_and_transparency_affect_visible_admission() {
        let mut editor = document();
        editor.rotate(0, 90.0).unwrap();
        let size = Bounds {
            left: 0.0,
            top: 0.0,
            right: 100.0,
            bottom: 100.0,
        };
        let bounds = visible(editor.elements().iter().enumerate(), viewport(), None, size).unwrap();
        assert!((bounds.left - 26.0).abs() < 0.001);
        assert!((bounds.top - 6.0).abs() < 0.001);
        assert!(visible(
            editor.elements().iter().enumerate(),
            viewport(),
            Some((0, Point { x: 200.0, y: 0.0 })),
            size
        )
        .is_none());
        editor.restyle(0, editor.style().with_opacity(0.0).unwrap());
        assert!(visible(editor.elements().iter().enumerate(), viewport(), None, size).is_none());
    }
}
