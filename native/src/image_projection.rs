//! 自然图片子矩形到目标框的有界映射；不构造放大后的完整图片坐标。
use crate::crop_geometry::Rect;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Projection {
    pub source: Rect,
    pub target: Rect,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rect(x: f64, width: f64) -> Rect {
        Rect {
            x,
            y: 0.0,
            width,
            height: 1.0,
        }
    }
    #[test]
    fn tiny_source_and_partial_intersection_keep_destination_bounded() {
        let Sampling::Region(p) =
            Projection::clipped(rect(1234.0, 1.0), rect(0.0, 8192.0), (8192, 1)).unwrap()
        else {
            panic!("region expected")
        };
        assert_eq!(p.source, rect(1234.0, 1.0));
        assert_eq!(p.target, rect(0.0, 8192.0));
        let Sampling::Region(p) =
            Projection::clipped(rect(-1.0, 2.0), rect(10.0, 100.0), (8192, 1)).unwrap()
        else {
            panic!("region expected")
        };
        assert_eq!(p.source, rect(0.0, 1.0));
        assert_eq!(p.target, rect(60.0, 50.0));
    }
    #[test]
    fn empty_intersection_is_not_full_and_invalid_geometry_is_rejected() {
        assert!(matches!(
            Projection::clipped(rect(8192.0, 1.0), rect(0.0, 8192.0), (8192, 1)),
            Some(Sampling::Empty)
        ));
        for bad in [
            rect(0.0, 0.0),
            rect(0.0, -1.0),
            rect(f64::NAN, 1.0),
            rect(0.0, f64::INFINITY),
        ] {
            assert!(Projection::clipped(bad, rect(0.0, 10.0), (8192, 1)).is_none());
            assert!(Projection::clipped(rect(0.0, 1.0), bad, (8192, 1)).is_none());
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub enum Sampling {
    #[default]
    Full,
    Region(Projection),
    Empty,
}

impl Projection {
    /// 输入必须是已翻转像素中的源框及镜像后的目标框；源越界部分保留透明。
    pub fn clipped(source: Rect, target: Rect, size: (u32, u32)) -> Option<Sampling> {
        let valid = |r: Rect| {
            [r.x, r.y, r.width, r.height, r.x + r.width, r.y + r.height]
                .iter()
                .all(|v| v.is_finite() && v.abs() <= i32::MAX as f64)
                && r.width > 0.0
                && r.height > 0.0
        };
        if !valid(source) || !valid(target) || size.0 == 0 || size.1 == 0 {
            return None;
        }
        let x = source.x.max(0.0);
        let y = source.y.max(0.0);
        let right = (source.x + source.width).min(size.0 as f64);
        let bottom = (source.y + source.height).min(size.1 as f64);
        if right <= x || bottom <= y {
            return Some(Sampling::Empty);
        }
        // 先取 [0,1] 比例再乘目标尺寸；极窄 source 不产生巨大中间 placement。
        let target = Rect {
            x: target.x + (x - source.x) / source.width * target.width,
            y: target.y + (y - source.y) / source.height * target.height,
            width: (right - x) / source.width * target.width,
            height: (bottom - y) / source.height * target.height,
        };
        if !valid(target) {
            return None;
        }
        Some(Sampling::Region(Self {
            source: Rect {
                x,
                y,
                width: right - x,
                height: bottom - y,
            },
            target,
        }))
    }
}
