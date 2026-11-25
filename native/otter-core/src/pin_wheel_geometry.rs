//! Preserve continuous-wheel subpixel geometry and round only at HWND commit; rebuild references after external movement/resizing.
use super::PinRect;
#[derive(Clone, Copy, Default)]
pub struct WheelZoom {
    last: Option<PinRect>,
    precise: [f64; 4],
}
impl WheelZoom {
    pub fn next(&mut self, rect: PinRect, pointer: (i32, i32), steps: f32) -> Option<PinRect> {
        if rect.width == 0 || rect.height == 0 || !steps.is_finite() {
            return None;
        }
        if self.last != Some(rect) {
            self.precise = [
                f64::from(rect.x),
                f64::from(rect.y),
                f64::from(rect.width),
                f64::from(rect.height),
            ];
        }
        let [x, y, width, height] = self.precise;
        let minimum = 1.0 / width.min(height);
        let maximum = (8192.0 / width.max(height)).min((16_777_216.0 / (width * height)).sqrt());
        if maximum < minimum {
            return None;
        }
        let factor = 1.1_f64
            .powf(f64::from(steps.clamp(-32.0, 32.0)))
            .clamp(minimum, maximum);
        let next_width = width * factor;
        let next_height = height * factor;
        let next_x = f64::from(pointer.0) - (f64::from(pointer.0) - x) * factor;
        let next_y = f64::from(pointer.1) - (f64::from(pointer.1) - y) * factor;
        let next = PinRect {
            x: next_x.round() as i32,
            y: next_y.round() as i32,
            width: next_width.round().max(1.0) as u32,
            height: next_height.round().max(1.0) as u32,
        };
        // Rounding may push extreme dimensions over pixel budgets; round inward only here, preserving exact accumulated values.
        let next = if u64::from(next.width) * u64::from(next.height) > 16_777_216 {
            PinRect {
                width: next_width.floor().max(1.0) as u32,
                height: next_height.floor().max(1.0) as u32,
                ..next
            }
        } else {
            next
        };
        self.precise = [next_x, next_y, next_width, next_height];
        self.last = Some(next);
        Some(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inverse_steps_preserve_odd_rectangles_and_fractional_wheel_steps_accumulate() {
        let original = PinRect {
            x: -501,
            y: 203,
            width: 161,
            height: 121,
        };
        let mut zoom = WheelZoom::default();
        let mut rect = original;
        for _ in 0..100 {
            rect = zoom.next(rect, (-400, 240), 1.0).unwrap();
            rect = zoom.next(rect, (-400, 240), -1.0).unwrap();
            assert_eq!(rect, original);
        }
        for _ in 0..100 {
            rect = zoom.next(rect, (-400, 240), 0.01).unwrap();
        }
        assert_eq!((rect.width, rect.height), (177, 133));
    }
    #[test]
    fn pointer_changes_external_moves_and_extreme_zoom_keep_the_anchor_and_budget() {
        let mut zoom = WheelZoom::default();
        let rect = PinRect {
            x: 100,
            y: 100,
            width: 161,
            height: 121,
        };
        let next = zoom.next(rect, (100, 100), 1.0).unwrap();
        let moved = PinRect {
            x: -900,
            y: -300,
            ..next
        };
        let next = zoom.next(moved, (-900, -300), -1.0).unwrap();
        assert_eq!((next.x, next.y), (moved.x, moved.y));
        let mut rect = next;
        for delta in [32.0; 100].into_iter().chain([-32.0; 100]) {
            rect = zoom.next(rect, (-800, -250), delta).unwrap();
            assert!(rect.width <= 8192 && rect.height <= 8192);
            assert!(u64::from(rect.width) * u64::from(rect.height) <= 16_777_216);
        }
        assert!(zoom.next(rect, (0, 0), f32::NAN).is_none());
    }
}
