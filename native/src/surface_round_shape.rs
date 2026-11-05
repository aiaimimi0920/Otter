//! 非对称内容边圆角；只生成帧内几何，工厂缓存和 COM 生命周期仍归 Clips。
use crate::surface_box::Insets;
use windows::{
    core::{Interface, Result},
    Win32::Graphics::Direct2D::{Common::*, *},
};

#[derive(Clone, Copy)]
pub(crate) struct Shape {
    pub(super) rect: D2D_RECT_F,
    // 顺时针 TL/TR/BR/BL，各自的水平和垂直半径。
    pub(super) radii: [[f32; 2]; 4],
}
impl From<D2D1_ROUNDED_RECT> for Shape {
    fn from(value: D2D1_ROUNDED_RECT) -> Self {
        Self {
            rect: value.rect,
            radii: [[value.radiusX, value.radiusY]; 4],
        }
    }
}
impl Shape {
    pub(crate) fn inset(mut self, inset: Insets) -> Self {
        let rect = &mut self.rect;
        rect.left += inset.left;
        rect.top += inset.top;
        rect.right = (rect.right - inset.right).max(rect.left);
        rect.bottom = (rect.bottom - inset.bottom).max(rect.top);
        for (radius, side) in self.radii.iter_mut().zip([
            [inset.left, inset.top],
            [inset.right, inset.top],
            [inset.right, inset.bottom],
            [inset.left, inset.bottom],
        ]) {
            *radius = std::array::from_fn(|a| (radius[a] - side[a]).max(0.));
            if radius.contains(&0.) {
                *radius = [0.; 2];
            }
        }
        // 相邻弧不能重叠；和外边圆角相同，使用共同缩放保持椭圆比例。
        let mut scale = 1_f32;
        for (axis, first, last, extent) in [
            (0, 0, 1, rect.right - rect.left),
            (0, 3, 2, rect.right - rect.left),
            (1, 0, 3, rect.bottom - rect.top),
            (1, 1, 2, rect.bottom - rect.top),
        ] {
            let sum = self.radii[first][axis] + self.radii[last][axis];
            if sum > 0. {
                scale = scale.min(extent / sum);
            }
        }
        for radius in &mut self.radii {
            *radius = radius.map(|v| v * scale);
        }
        self
    }
    pub(crate) fn contains(&self, [x, y]: [f32; 2]) -> bool {
        let r = self.rect;
        if x < r.left || x >= r.right || y < r.top || y >= r.bottom {
            return false;
        }
        for ([cx, cy], radius, corner) in [
            (
                [r.left, r.top],
                self.radii[0],
                [x < r.left + self.radii[0][0], y < r.top + self.radii[0][1]],
            ),
            (
                [r.right, r.top],
                self.radii[1],
                [x > r.right - self.radii[1][0], y < r.top + self.radii[1][1]],
            ),
            (
                [r.right, r.bottom],
                self.radii[2],
                [
                    x > r.right - self.radii[2][0],
                    y > r.bottom - self.radii[2][1],
                ],
            ),
            (
                [r.left, r.bottom],
                self.radii[3],
                [
                    x < r.left + self.radii[3][0],
                    y > r.bottom - self.radii[3][1],
                ],
            ),
        ] {
            if corner == [true, true] && radius[0] > 0. && radius[1] > 0. {
                let center = [
                    if cx == r.left {
                        cx + radius[0]
                    } else {
                        cx - radius[0]
                    },
                    if cy == r.top {
                        cy + radius[1]
                    } else {
                        cy - radius[1]
                    },
                ];
                return ((x - center[0]) / radius[0]).powi(2)
                    + ((y - center[1]) / radius[1]).powi(2)
                    <= 1.;
            }
        }
        true
    }
    pub(crate) fn geometry(&self, factory: &ID2D1Factory) -> Result<ID2D1Geometry> {
        if self.radii.iter().all(|r| *r == self.radii[0]) {
            return unsafe {
                factory
                    .CreateRoundedRectangleGeometry(&D2D1_ROUNDED_RECT {
                        rect: self.rect,
                        radiusX: self.radii[0][0],
                        radiusY: self.radii[0][1],
                    })?
                    .cast()
            };
        }
        let [tl, tr, br, bl] = self.radii;
        let r = self.rect;
        let path = unsafe { factory.CreatePathGeometry()? };
        let sink = unsafe { path.Open()? };
        let point = |[x, y]: [f32; 2]| D2D_POINT_2F { x, y };
        unsafe {
            sink.BeginFigure(point([r.left + tl[0], r.top]), D2D1_FIGURE_BEGIN_FILLED);
            for (line, end, radius) in [
                ([r.right - tr[0], r.top], [r.right, r.top + tr[1]], tr),
                ([r.right, r.bottom - br[1]], [r.right - br[0], r.bottom], br),
                ([r.left + bl[0], r.bottom], [r.left, r.bottom - bl[1]], bl),
                ([r.left, r.top + tl[1]], [r.left + tl[0], r.top], tl),
            ] {
                sink.AddLine(point(line));
                if radius.contains(&0.) {
                    sink.AddLine(point(end));
                } else {
                    sink.AddArc(&D2D1_ARC_SEGMENT {
                        point: point(end),
                        size: D2D_SIZE_F {
                            width: radius[0],
                            height: radius[1],
                        },
                        rotationAngle: 0.,
                        sweepDirection: D2D1_SWEEP_DIRECTION_CLOCKWISE,
                        arcSize: D2D1_ARC_SIZE_SMALL,
                    });
                }
            }
            sink.EndFigure(D2D1_FIGURE_END_CLOSED);
            sink.Close()?;
        }
        path.cast()
    }
}
