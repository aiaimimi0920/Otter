//! 本地无像素标记；复用前景色/裁剪/透明层，不创建控件、纹理或事件。
use super::*;
pub(crate) struct Paint {
    origin: [f32; 2],
    clip: D2D_RECT_F,
}
impl Paint {
    pub(crate) fn new(placed: &Placed<'_>, clip: Rect, context: Context) -> Option<Self> {
        let inset = crate::surface_box::inset(placed.node, placed.containing_width, context);
        let origin = std::array::from_fn(|a| placed.origin[a] + inset.start()[a]);
        let size = crate::surface_box::inner(placed.size, inset);
        let clip = clip.intersect(Rect::from_box(origin, size));
        (!clip.empty()).then_some(Self {
            origin,
            clip: native(clip),
        })
    }
    pub(crate) fn draw(&self, target: &ID2D1RenderTarget, brush: &ID2D1SolidColorBrush) {
        let [x, y] = self.origin;
        let side = crate::surface_native_measure::placeholder::ICON;
        unsafe {
            target.PushAxisAlignedClip(&self.clip, D2D1_ANTIALIAS_MODE_ALIASED);
            target.DrawRectangle(
                &D2D_RECT_F {
                    left: x + 0.5,
                    top: y + 0.5,
                    right: x + side - 0.5,
                    bottom: y + side - 0.5,
                },
                brush,
                1.,
                None,
            );
            target.DrawLine(
                D2D_POINT_2F {
                    x: x + 3.,
                    y: y + side - 4.,
                },
                D2D_POINT_2F {
                    x: x + 7.,
                    y: y + 7.,
                },
                brush,
                1.,
                None,
            );
            target.DrawLine(
                D2D_POINT_2F {
                    x: x + 7.,
                    y: y + 7.,
                },
                D2D_POINT_2F {
                    x: x + side - 3.,
                    y: y + side - 4.,
                },
                brush,
                1.,
                None,
            );
            target.DrawLine(
                D2D_POINT_2F {
                    x: x + 2.,
                    y: y + side - 2.,
                },
                D2D_POINT_2F {
                    x: x + side - 2.,
                    y: y + 2.,
                },
                brush,
                1.,
                None,
            );
            target.PopAxisAlignedClip();
        }
    }
}
