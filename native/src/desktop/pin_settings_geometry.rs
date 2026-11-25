//! Logical-pixel geometry for mainline pin settings; convert to physical pixels only at the end and honor native rendering budgets.
use crate::pin_geometry::PinRect;
use windows::Win32::Foundation::RECT;
fn round(value: f64) -> f64 {
    (value + 0.5).floor()
}
fn output(x: f64, y: f64, w: f64, h: f64, dpi: u32) -> Option<PinRect> {
    let d = f64::from(dpi) / 96.0;
    let (x, y, w, h) = (
        (x * d).round(),
        (y * d).round(),
        (w * d).round(),
        (h * d).round(),
    );
    if ![x, y, w, h].iter().all(|v| v.is_finite())
        || !(1.0..=8192.0).contains(&w)
        || !(1.0..=8192.0).contains(&h)
        || w * h > 16_777_216.0
        || x < f64::from(i32::MIN)
        || y < f64::from(i32::MIN)
        || x + w > f64::from(i32::MAX)
        || y + h > f64::from(i32::MAX)
    {
        return None;
    }
    Some(PinRect {
        x: x as i32,
        y: y as i32,
        width: w as u32,
        height: h as u32,
    })
}
pub(super) fn width(frame: PinRect, width: i32, dpi: u32) -> Option<PinRect> {
    if dpi == 0 || frame.width == 0 || frame.height == 0 || !(32..=8192).contains(&width) {
        return None;
    }
    let d = f64::from(dpi) / 96.0;
    let (x, y, w, h) = (
        f64::from(frame.x) / d,
        f64::from(frame.y) / d,
        f64::from(frame.width) / d,
        f64::from(frame.height) / d,
    );
    let factor = f64::from(width) / w;
    let (nw, nh) = (round(w * factor).max(16.0), round(h * factor).max(16.0));
    output(
        round(x + w / 2.0 - nw / 2.0),
        round(y + h / 2.0 - nh / 2.0),
        nw,
        nh,
        dpi,
    )
}
pub(super) fn fit(frame: PinRect, image: (u32, u32), dpi: u32) -> Option<PinRect> {
    if dpi == 0 || image.0 == 0 || image.1 == 0 {
        return None;
    }
    let d = f64::from(dpi) / 96.0;
    let (w, h) = (f64::from(frame.width) / d, f64::from(frame.height) / d);
    let ratio = f64::from(image.0) / f64::from(image.1);
    let (w, h) = if ratio > w / h {
        (w, w / ratio)
    } else {
        (h * ratio, h)
    };
    output(f64::from(frame.x) / d, f64::from(frame.y) / d, w, h, dpi)
}
pub(super) fn place(anchor: RECT, work: RECT, size: (i32, i32)) -> RECT {
    let (w, h) = size;
    let left_fits = anchor.left - w >= work.left;
    let right_fits = anchor.right + w <= work.right;
    let x = if left_fits {
        anchor.left - w
    } else if right_fits {
        anchor.right
    } else {
        anchor.left
    };
    let y = if !left_fits && !right_fits {
        if anchor.bottom + h <= work.bottom {
            anchor.bottom
        } else {
            anchor.top - h
        }
    } else {
        anchor.top
    };
    let left = x.clamp(work.left, (work.right - w).max(work.left));
    let top = y.clamp(work.top, (work.bottom - h).max(work.top));
    RECT {
        left,
        top,
        right: left + w,
        bottom: top + h,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn main_width_contract_uses_logical_rounding_and_fixed_center_at_each_dpi() {
        for dpi in [96, 120, 144, 192] {
            let d = f64::from(dpi) / 96.0;
            let old = PinRect {
                x: (-200.0 * d) as i32,
                y: (300.0 * d) as i32,
                width: (320.0 * d) as u32,
                height: (240.0 * d) as u32,
            };
            let new = width(old, 640, dpi).unwrap();
            assert_eq!(
                new,
                PinRect {
                    x: (-360.0 * d) as i32,
                    y: (180.0 * d) as i32,
                    width: (640.0 * d) as u32,
                    height: (480.0 * d) as u32
                }
            );
        }
        let old = PinRect {
            x: 0,
            y: 0,
            width: 320,
            height: 10,
        };
        assert_eq!(width(old, 32, 96).unwrap().height, 16);
        assert!(width(old, 8193, 96).is_none());
        assert!(width(PinRect { height: 320, ..old }, 8192, 192).is_none());
    }
    #[test]
    fn fitting_never_enlarges_and_keeps_top_left() {
        let r = PinRect {
            x: -900,
            y: 40,
            width: 320,
            height: 240,
        };
        assert_eq!(fit(r, (1600, 900), 96), Some(PinRect { height: 180, ..r }));
        assert_eq!(fit(r, (800, 1200), 96), Some(PinRect { width: 160, ..r }));
        assert_eq!(fit(r, (4, 3), 96), Some(r));
    }
    #[test]
    fn settings_touch_left_then_right_then_avoid_above_or_below() {
        let work = RECT {
            left: -1000,
            top: 0,
            right: 0,
            bottom: 800,
        };
        let anchor = RECT {
            left: -500,
            top: 100,
            right: -200,
            bottom: 400,
        };
        assert_eq!(
            place(anchor, work, (125, 150)),
            RECT {
                left: -625,
                top: 100,
                right: -500,
                bottom: 250
            }
        );
        let anchor = RECT {
            left: -990,
            top: 100,
            right: -200,
            bottom: 400,
        };
        assert_eq!(place(anchor, work, (125, 150)).left, -200);
        let anchor = RECT {
            right: -20,
            ..anchor
        };
        assert_eq!(place(anchor, work, (125, 150)).top, 400);
    }
}
