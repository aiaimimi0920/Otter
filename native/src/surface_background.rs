//! 节点自身背景形状；圆角不隐式裁剪子节点，祖先裁剪不能改变原圆角几何。
use crate::{surface_clip::Rect, surface_layout_tree::Placed, surface_length::Context};
use windows::Win32::Graphics::Direct2D::{
    Common::D2D_RECT_F, ID2D1RenderTarget, ID2D1SolidColorBrush, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
    D2D1_ROUNDED_RECT,
};
pub(crate) struct Background {
    bounds: Rect,
    clip: Rect,
    radius: [f32; 2],
}
impl Background {
    pub(crate) fn content_round(
        &self,
        inset: crate::surface_box::Insets,
    ) -> Option<crate::surface_round_clip::Shape> {
        self.rounded()
            .map(|shape| crate::surface_round_clip::Shape::from(shape).inset(inset))
    }
    pub(crate) fn padding_round(
        &self,
        border: crate::surface_box::Insets,
    ) -> Option<crate::surface_round_clip::Shape> {
        self.content_round(border)
    }
    pub(crate) fn outline(&self) -> (D2D1_ROUNDED_RECT, Rect) {
        (
            D2D1_ROUNDED_RECT {
                rect: native(self.bounds),
                radiusX: self.radius[0],
                radiusY: self.radius[1],
            },
            self.clip,
        )
    }
    pub(crate) fn rounded(&self) -> Option<D2D1_ROUNDED_RECT> {
        (!self.radius.contains(&0.)).then(|| D2D1_ROUNDED_RECT {
            rect: native(self.bounds),
            radiusX: self.radius[0],
            radiusY: self.radius[1],
        })
    }
    pub(crate) fn new(placed: &Placed<'_>, clip: Rect, context: Context) -> Option<Self> {
        let bounds = Rect::from_box(placed.origin, placed.size);
        if bounds.intersect(clip).empty() {
            return None;
        }
        let mut radius = [0.; 2];
        if let Some(length) = placed.node.style_length("borderRadius") {
            for (axis, radius) in radius.iter_mut().enumerate() {
                *radius = length
                    .resolve(Context {
                        axis: placed.size[axis],
                        fraction: None,
                        ..context
                    })
                    .unwrap_or(0.);
            }
        }
        // CSS 单值百分比分别按宽高计算；超大半径用共同系数缩小，保持椭圆比例。
        let scale = (0..2)
            .filter(|axis| radius[*axis] > 0.)
            .map(|axis| placed.size[axis] / (2. * radius[axis]))
            .fold(1., f32::min);
        radius = radius.map(|value| value * scale);
        Some(Self {
            bounds,
            clip,
            radius,
        })
    }
    pub(crate) fn draw(&self, target: &ID2D1RenderTarget, brush: &ID2D1SolidColorBrush) {
        unsafe {
            if self.radius.contains(&0.) {
                target.FillRectangle(&native(self.bounds.intersect(self.clip)), brush);
            } else {
                target.PushAxisAlignedClip(&native(self.clip), D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
                target.FillRoundedRectangle(
                    &D2D1_ROUNDED_RECT {
                        rect: native(self.bounds),
                        radiusX: self.radius[0],
                        radiusY: self.radius[1],
                    },
                    brush,
                );
                target.PopAxisAlignedClip();
            }
        }
    }
}
fn native(rect: Rect) -> D2D_RECT_F {
    D2D_RECT_F {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn radius(value: &str) -> [f32; 2] {
        let scene = crate::surface_scene::Scene::parse(
            format!(r#"{{"id":"root","type":"view","style":{{"borderRadius":{value}}}}}"#)
                .as_bytes(),
        )
        .unwrap();
        let placed = Placed {
            containing_width: 100.,
            grid_cell: None,
            node: scene.root(),
            origin: [0.; 2],
            size: [100., 50.],
            parent: None,
        };
        Background::new(
            &placed,
            Rect::from_box([0.; 2], [100., 50.]),
            Context {
                axis: 100.,
                viewport: [100., 50.],
                container: [200., 100.],
                root_font: 16.,
                font: 10.,
                fraction: None,
            },
        )
        .unwrap()
        .radius
    }
    #[test]
    fn percent_uses_box_axes_and_large_radii_shrink_together() {
        assert_eq!(radius(r#""50%""#), [50., 25.]);
        assert_eq!(radius("100"), [25., 25.]);
        assert_eq!(radius(r#""200%""#), [50., 25.]);
    }
    #[test]
    fn relative_lengths_use_computed_font_and_invalid_values_remain_square() {
        assert_eq!(radius(r#""1em""#), [10., 10.]);
        assert_eq!(radius(r#""1rem""#), [16., 16.]);
        assert_eq!(radius(r#""10cqw""#), [20., 20.]);
        for value in ["-1", r#""auto""#, r#""1fr""#, r#""url(x)""#] {
            assert_eq!(radius(value), [0.; 2]);
        }
    }
}
