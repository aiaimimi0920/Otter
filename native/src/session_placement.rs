//! 主干 CSS 会话框到屏幕物理框：backendRects.ts 的 Math.round，再加覆盖窗口客户区原点。
use crate::{pin_geometry::PinRect, session_window::Rect};
#[path = "session_placement_inverse.rs"]
mod inverse;

#[derive(Debug, Clone, Copy)]
pub struct Space {
    pub origin: (i32, i32),
    /// 兼容主干覆盖窗口的 devicePixelRatio；不是待创建钉图所在显示器的 DPI。
    pub scale: f64,
}
#[derive(Debug, PartialEq)]
pub enum Error {
    Geometry,
    Capacity,
}
impl Space {
    pub fn physical(self, rect: Rect) -> Result<PinRect, Error> {
        if !self.scale.is_finite()
            || self.scale <= 0.0
            || !rect.x.is_finite()
            || !rect.y.is_finite()
            || !rect.w.is_finite()
            || !rect.h.is_finite()
            || rect.w <= 0.0
            || rect.h <= 0.0
        {
            return Err(Error::Geometry);
        }
        // JS 半整数向正无穷取整；Rust f64::round 在负半整数上不同，不能替换。
        let round = |value: f64| (value * self.scale + 0.5).floor();
        let x = round(rect.x) + f64::from(self.origin.0);
        let y = round(rect.y) + f64::from(self.origin.1);
        let width = round(rect.w);
        let height = round(rect.h);
        if !x.is_finite()
            || !y.is_finite()
            || !width.is_finite()
            || !height.is_finite()
            || x < i32::MIN as f64
            || y < i32::MIN as f64
            || x + width > i32::MAX as f64
            || y + height > i32::MAX as f64
            || width < 1.0
            || height < 1.0
        {
            return Err(Error::Geometry);
        }
        if width > 8192.0 || height > 8192.0 || width * height > 16_777_216.0 {
            return Err(Error::Capacity);
        }
        Ok(PinRect {
            x: x as i32,
            y: y as i32,
            width: width as u32,
            height: height as u32,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn css_rounding_precedes_negative_overlay_origin_translation() {
        let space = Space {
            origin: (-1920, -200),
            scale: 1.5,
        };
        let physical = space
            .physical(Rect {
                x: -1.0,
                y: 1.0,
                w: 100.25,
                h: 50.5,
            })
            .unwrap();
        assert_eq!(
            (physical.x, physical.y, physical.width, physical.height),
            (-1921, -198, 150, 76)
        );
        let zero = space
            .physical(Rect {
                x: -1.0 / 3.0,
                y: 0.0,
                w: 10.0,
                h: 10.0,
            })
            .unwrap();
        assert_eq!(zero.x, -1920);
    }
    #[test]
    fn bad_scale_overflow_tiny_frames_and_surface_budget_are_rejected() {
        let rect = Rect {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
        };
        for scale in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(
                Space {
                    origin: (0, 0),
                    scale
                }
                .physical(rect),
                Err(Error::Geometry)
            );
        }
        let space = Space {
            origin: (0, 0),
            scale: 1.0,
        };
        assert_eq!(
            space.physical(Rect { w: 0.1, ..rect }),
            Err(Error::Geometry)
        );
        assert_eq!(
            space.physical(Rect {
                x: i32::MAX as f64,
                ..rect
            }),
            Err(Error::Geometry)
        );
        assert_eq!(
            space.physical(Rect { w: 8193.0, ..rect }),
            Err(Error::Capacity)
        );
        assert_eq!(
            space.physical(Rect {
                w: 8192.0,
                h: 8192.0,
                ..rect
            }),
            Err(Error::Capacity)
        );
    }
}
