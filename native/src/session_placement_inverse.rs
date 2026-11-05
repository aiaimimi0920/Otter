//! 物理窗口框写回逻辑坐标；提示只保留已验证的取整余量，不能绕过恢复预算。
use super::*;

impl Space {
    /// 使用恢复时显式指定的同一 Space；不得从目标显示器 DPI 推断原会话比例。
    /// hint 是原逻辑框：不变的边保留小数，移动/缩放按物理增量换算，再验证物理往返。
    pub fn logical(self, frame: PinRect, hint: Option<Rect>) -> Result<Rect, Error> {
        if !self.scale.is_finite() || self.scale <= 0.0 {
            return Err(Error::Geometry);
        }
        let canonical = Rect {
            x: (i64::from(frame.x) - i64::from(self.origin.0)) as f64 / self.scale,
            y: (i64::from(frame.y) - i64::from(self.origin.1)) as f64 / self.scale,
            w: f64::from(frame.width) / self.scale,
            h: f64::from(frame.height) / self.scale,
        };
        if !canonical.valid() {
            return Err(Error::Geometry);
        }
        if self.physical(canonical)? != frame {
            return Err(Error::Geometry);
        }
        if let Some(hint) = hint.filter(Rect::valid) {
            if let Ok(previous) = self.physical(hint) {
                let preserved = Rect {
                    x: hint.x + (i64::from(frame.x) - i64::from(previous.x)) as f64 / self.scale,
                    y: hint.y + (i64::from(frame.y) - i64::from(previous.y)) as f64 / self.scale,
                    w: hint.w
                        + (i64::from(frame.width) - i64::from(previous.width)) as f64 / self.scale,
                    h: hint.h
                        + (i64::from(frame.height) - i64::from(previous.height)) as f64
                            / self.scale,
                };
                // 浮点边界可能不能保留余量；不能为保留旧小数牺牲真实 HWND 几何。
                if preserved.valid() && self.physical(preserved).is_ok_and(|rect| rect == frame) {
                    return Ok(preserved);
                }
            }
        }
        Ok(canonical)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn space() -> Space {
        Space {
            origin: (-1920, -200),
            scale: 1.5,
        }
    }
    fn original() -> Rect {
        Rect {
            x: -20.25,
            y: 10.125,
            w: 80.25,
            h: 60.125,
        }
    }
    #[test]
    fn unchanged_fractional_geometry_is_preserved_exactly() {
        let hint = original();
        let frame = space().physical(hint).unwrap();
        assert_eq!(space().logical(frame, Some(hint)).unwrap(), hint);
        assert_eq!(
            space()
                .physical(space().logical(frame, None).unwrap())
                .unwrap(),
            frame
        );
    }
    #[test]
    fn movement_and_resize_keep_rounding_residue_and_round_trip() {
        let hint = original();
        let before = space().physical(hint).unwrap();
        let after = PinRect {
            x: before.x + 17,
            y: before.y - 9,
            width: before.width + 3,
            ..before
        };
        let saved = space().logical(after, Some(hint)).unwrap();
        assert_eq!(saved.x, hint.x + 17.0 / 1.5);
        assert_eq!(saved.y, hint.y - 9.0 / 1.5);
        assert_eq!(saved.w, hint.w + 2.0);
        assert_eq!(saved.h, hint.h);
        assert_eq!(space().physical(saved).unwrap(), after);
    }
    #[test]
    fn bad_hint_cannot_replace_real_geometry_or_bypass_capacity() {
        let frame = space().physical(original()).unwrap();
        for hint in [
            Rect {
                x: f64::NAN,
                ..original()
            },
            Rect {
                w: 0.0,
                ..original()
            },
            Rect {
                w: 10000.0,
                ..original()
            },
        ] {
            let saved = space().logical(frame, Some(hint)).unwrap();
            assert_eq!(space().physical(saved).unwrap(), frame);
        }
        assert_eq!(
            space().logical(
                PinRect {
                    width: 8193,
                    ..frame
                },
                None
            ),
            Err(Error::Capacity)
        );
        assert!(space()
            .logical(PinRect { width: 0, ..frame }, None)
            .is_err());
        assert!(space()
            .logical(
                PinRect {
                    x: i32::MAX,
                    ..frame
                },
                None
            )
            .is_err());
        for scale in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(Space { scale, ..space() }.logical(frame, None).is_err());
        }
    }
    #[test]
    fn supported_dpi_scales_negative_coordinates_and_extreme_origins_round_trip() {
        for scale in [0.75, 1.0, 1.25, 1.5, 1.75, 2.0, 2.25, 3.0, 4.0] {
            for origin in [(-1920, -200), (0, 0), (100000, -100000)] {
                let space = Space { origin, scale };
                for step in -20..20 {
                    let rect = Rect {
                        x: step as f64 + 0.125,
                        y: -step as f64 - 0.375,
                        w: 80.25,
                        h: 30.125,
                    };
                    let frame = space.physical(rect).unwrap();
                    for hint in [None, Some(rect)] {
                        assert_eq!(
                            space.physical(space.logical(frame, hint).unwrap()).unwrap(),
                            frame
                        );
                    }
                }
            }
        }
    }
}
