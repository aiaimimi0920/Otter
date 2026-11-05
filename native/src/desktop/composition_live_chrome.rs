//! Live 呈现专用边角；物理客户区像素，同一几何用于绘制和 L 形命中，不修改源帧。
use super::*;
const LINE: f32 = 2.0;
const ARM: f32 = 16.0;
const HIT: f32 = 5.0;
pub(super) struct Chrome {
    green: ID2D1SolidColorBrush,
    yellow: ID2D1SolidColorBrush,
}
fn rect(left: f32, top: f32, right: f32, bottom: f32) -> D2D_RECT_F {
    D2D_RECT_F {
        left,
        top,
        right,
        bottom,
    }
}
fn arms(size: D2D_SIZE_U, thickness: f32) -> [D2D_RECT_F; 8] {
    let (w, h) = (size.width as f32, size.height as f32);
    let (x, y) = (ARM.min(w / 2.0), ARM.min(h / 2.0));
    let t = thickness.min(w / 2.0).min(h / 2.0);
    [
        rect(0.0, 0.0, x, t),
        rect(0.0, 0.0, t, y),
        rect(w - x, 0.0, w, t),
        rect(w - t, 0.0, w, y),
        rect(0.0, h - t, x, h),
        rect(0.0, h - y, t, h),
        rect(w - x, h - t, w, h),
        rect(w - t, h - y, w, h),
    ]
}
pub(super) fn hit(size: D2D_SIZE_U, point: (f32, f32)) -> bool {
    size.width != 0
        && size.height != 0
        && arms(size, HIT).iter().any(|r| {
            point.0 >= r.left && point.0 < r.right && point.1 >= r.top && point.1 < r.bottom
        })
}
impl Chrome {
    pub(super) fn new(target: &ID2D1RenderTarget) -> Result<Self> {
        unsafe {
            Ok(Self {
                green: target.CreateSolidColorBrush(
                    &D2D1_COLOR_F {
                        r: 34.0 / 255.0,
                        g: 197.0 / 255.0,
                        b: 94.0 / 255.0,
                        a: 1.0,
                    },
                    None,
                )?,
                yellow: target.CreateSolidColorBrush(
                    &D2D1_COLOR_F {
                        r: 217.0 / 255.0,
                        g: 1.0,
                        b: 56.0 / 255.0,
                        a: 1.0,
                    },
                    None,
                )?,
            })
        }
    }
    pub(super) unsafe fn draw(&self, target: &ID2D1RenderTarget, size: D2D_SIZE_U) {
        let (w, h) = (size.width as f32, size.height as f32);
        let t = LINE.min(w / 2.0).min(h / 2.0);
        for r in [
            rect(0.0, 0.0, w, t),
            rect(0.0, h - t, w, h),
            rect(0.0, 0.0, t, h),
            rect(w - t, 0.0, w, h),
        ] {
            target.FillRectangle(&r, &self.green);
        }
        for r in arms(size, LINE) {
            target.FillRectangle(&r, &self.yellow);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn live_chrome_hit_is_only_clipped_l_arms_and_all_rectangles_stay_inside() {
        let size = D2D_SIZE_U {
            width: 64,
            height: 48,
        };
        for p in [(0.0, 0.0), (15.0, 4.0), (4.0, 15.0), (63.0, 47.0)] {
            assert!(hit(size, p));
        }
        for p in [
            (8.0, 8.0),
            (16.0, 0.0),
            (32.0, 1.0),
            (64.0, 47.0),
            (-1.0, 0.0),
        ] {
            assert!(!hit(size, p));
        }
        for (width, height) in [(0, 0), (1, 1), (1, 256), (256, 1), (32, 32), (8192, 1)] {
            for r in arms(D2D_SIZE_U { width, height }, HIT) {
                assert!(
                    r.left >= 0.0
                        && r.top >= 0.0
                        && r.right <= width as f32
                        && r.bottom <= height as f32
                );
                assert!(r.left <= r.right && r.top <= r.bottom);
            }
        }
        assert!(!hit(D2D_SIZE_U::default(), (0.0, 0.0)));
    }
}
