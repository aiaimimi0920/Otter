//! 缩略展开事务的几何与来源；运行时缺省回退不等于可持久化的 savedRect。
use crate::pin_geometry::PinRect;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transition {
    pub initial: PinRect,
    pub expanded: PinRect,
    pub opacity: f32,
    pub persist_expanded: bool,
}
impl Transition {
    pub fn target(self, current: (i32, i32)) -> Option<PinRect> {
        let x = i32::try_from(self.expanded.x as i64 + current.0 as i64 - self.initial.x as i64)
            .ok()?;
        let y = i32::try_from(self.expanded.y as i64 + current.1 as i64 - self.initial.y as i64)
            .ok()?;
        let frame = self.expanded;
        if frame.width == 0
            || frame.height == 0
            || frame.width > 8192
            || frame.height > 8192
            || frame.width as u64 * frame.height as u64 > 16_000_000
            || x as i64 + frame.width as i64 > i32::MAX as i64
            || y as i64 + frame.height as i64 > i32::MAX as i64
        {
            return None;
        }
        Some(PinRect { x, y, ..frame })
    }
    pub fn saved_target(self, current: (i32, i32)) -> Option<Option<PinRect>> {
        let target = self.target(current)?;
        Some(self.persist_expanded.then_some(target))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_window_round_trip_preserves_missing_frame_and_dragged_expansion() {
        let space = crate::session_placement::Space {
            origin: (-1920, -200),
            scale: 1.5,
        };
        for saved in [false, true] {
            let mut original = serde_json::json!({"id":"one","x":10,"y":20,"w":100,"h":100,
                "minified":true,"cropOffset":{"x":4,"y":6},"extension":123});
            if saved {
                original["savedRect"] = serde_json::json!({"x":6,"y":14,"w":400,"h":200});
            }
            let source = original.to_string();
            let window: crate::session_window::Window = serde_json::from_str(&source).unwrap();
            let initial = space.physical(window.rect()).unwrap();
            let transition = Transition {
                initial,
                expanded: space.physical(window.expanded_rect().unwrap()).unwrap(),
                opacity: 1.0,
                persist_expanded: saved,
            };
            let current = PinRect {
                x: initial.x + 30,
                y: initial.y - 15,
                ..initial
            };
            let merged = crate::session_window_write::merge(
                original,
                crate::session_window_write::Geometry {
                    frame: current,
                    minified: true,
                    expanded: transition.saved_target((current.x, current.y)).unwrap(),
                    normal_opacity: 1.0,
                    mini_opacity: 0.9,
                },
                space,
            )
            .unwrap();
            assert_eq!(merged["extension"], 123);
            let json = merged.to_string();
            let read: crate::session_window::Window = serde_json::from_str(&json).unwrap();
            assert_eq!(read.saved_rect.is_some(), saved);
            assert_eq!(space.physical(read.rect()).unwrap(), current);
            assert_eq!(
                space.physical(read.expanded_rect().unwrap()).unwrap(),
                transition.target((current.x, current.y)).unwrap()
            );
            assert_eq!(
                read.crop_offset.unwrap(),
                crate::session_window::Offset { x: 4.0, y: 6.0 }
            );
        }
    }
    #[test]
    fn dragged_target_and_unsaved_fallback_keep_distinct_persistence() {
        let initial = PinRect {
            x: -1800,
            y: -200,
            width: 150,
            height: 150,
        };
        let transition = Transition {
            initial,
            expanded: PinRect {
                x: -1830,
                y: -260,
                width: 600,
                height: 300,
            },
            opacity: 0.8,
            persist_expanded: true,
        };
        let expected = PinRect {
            x: -1730,
            y: -210,
            ..transition.expanded
        };
        assert_eq!(transition.target((-1700, -150)), Some(expected));
        assert_eq!(transition.saved_target((-1700, -150)), Some(Some(expected)));
        let fallback = Transition {
            expanded: initial,
            persist_expanded: false,
            ..transition
        };
        assert_eq!(
            fallback.target((-1700, -150)),
            Some(PinRect {
                x: -1700,
                y: -150,
                ..initial
            })
        );
        assert_eq!(fallback.saved_target((-1700, -150)), Some(None));
        assert!(transition.target((i32::MAX, 0)).is_none());
        // 即使展开框与缩略框碰巧相同，主动缩略也必须保留持久化语义。
        assert!(Transition {
            persist_expanded: true,
            ..fallback
        }
        .saved_target((-1700, -150))
        .unwrap()
        .is_some());
    }
}
