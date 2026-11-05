//! 钉图窗口以物理像素围绕指针缩放；预算约束在创建交换链缓冲前执行。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PinRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub fn zoom(rect: PinRect, pointer: (i32, i32), steps: f32) -> Option<PinRect> {
    if rect.width == 0 || rect.height == 0 || !steps.is_finite() {
        return None;
    }
    let factor = 1.1_f64.powf(f64::from(steps.clamp(-32.0, 32.0)));
    let (width, height) = bounded_size(rect, factor)?;
    let anchor = |position: i32, mouse: i32, old: u32, new: u32| {
        (f64::from(mouse)
            - (f64::from(mouse) - f64::from(position)) * f64::from(new) / f64::from(old))
        .round() as i32
    };
    Some(PinRect {
        x: anchor(rect.x, pointer.0, rect.width, width),
        y: anchor(rect.y, pointer.1, rect.height, height),
        width,
        height,
    })
}
/// 等比拖动固定初始左上角；沿归一化位移较大的轴调整，始终从初始尺寸计算。
pub fn resize_drag(rect: PinRect, delta: (i64, i64)) -> Option<PinRect> {
    if rect.width == 0 || rect.height == 0 {
        return None;
    }
    let dx = delta.0 as f64 / f64::from(rect.width);
    let dy = delta.1 as f64 / f64::from(rect.height);
    let factor = 1.0 + if dx.abs() >= dy.abs() { dx } else { dy };
    let (width, height) = bounded_size(rect, factor)?;
    rect.x.checked_add(width as i32)?;
    rect.y.checked_add(height as i32)?;
    Some(PinRect {
        width,
        height,
        ..rect
    })
}
fn bounded_size(rect: PinRect, factor: f64) -> Option<(u32, u32)> {
    let max_factor = (8192.0 / f64::from(rect.width.max(rect.height)))
        .min((16_777_216.0 / (f64::from(rect.width) * f64::from(rect.height))).sqrt());
    let min_factor = 1.0 / f64::from(rect.width.min(rect.height));
    if max_factor < min_factor {
        return None;
    }
    let factor = factor.clamp(min_factor, max_factor);
    let width = (f64::from(rect.width) * factor).floor().max(1.0) as u32;
    let height = (f64::from(rect.height) * factor).floor().max(1.0) as u32;
    Some((width, height))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drag_resize_preserves_anchor_ratio_and_budget_from_initial_snapshot() {
        let rect = PinRect {
            x: -500,
            y: -200,
            width: 200,
            height: 100,
        };
        assert_eq!(resize_drag(rect, (0, 0)), Some(rect));
        assert_eq!(
            resize_drag(rect, (100, 0)),
            Some(PinRect {
                width: 300,
                height: 150,
                ..rect
            })
        );
        assert_eq!(resize_drag(rect, (0, 50)), resize_drag(rect, (100, 0)));
        for delta in [(i64::MAX, i64::MAX), (i64::MIN, i64::MIN), (13, 7)] {
            let next = resize_drag(rect, delta).unwrap();
            assert_eq!((next.x, next.y), (rect.x, rect.y));
            assert!(next.width <= 8192 && next.height <= 8192);
            assert!(u64::from(next.width) * u64::from(next.height) <= 16_777_216);
            assert!((i64::from(next.width) - 2 * i64::from(next.height)).abs() <= 1);
        }
        assert!(resize_drag(
            PinRect {
                x: i32::MAX,
                ..rect
            },
            (0, 0)
        )
        .is_none());
    }
    #[test]
    fn negative_monitor_anchor_is_preserved() {
        let before = PinRect {
            x: -1000,
            y: -200,
            width: 800,
            height: 400,
        };
        let after = zoom(before, (-600, 0), 3.0).unwrap();
        assert!(((f64::from(-600 - after.x) / f64::from(after.width)) - 0.5).abs() < 0.002);
        assert!(((f64::from(-after.y) / f64::from(after.height)) - 0.5).abs() < 0.002);
    }
    #[test]
    fn repeated_zoom_cannot_exceed_swapchain_pixel_budget() {
        let mut rect = PinRect {
            x: 0,
            y: 0,
            width: 400,
            height: 200,
        };
        for _ in 0..200 {
            rect = zoom(rect, (0, 0), 32.0).unwrap();
        }
        assert!(u64::from(rect.width) * u64::from(rect.height) <= 16_777_216);
        assert!(rect.width <= 8192 && rect.height <= 8192);
        for _ in 0..200 {
            rect = zoom(rect, (0, 0), -32.0).unwrap();
        }
        assert!(rect.width >= 1 && rect.height >= 1);
        assert!(zoom(rect, (0, 0), f32::NAN).is_none());
    }
}
