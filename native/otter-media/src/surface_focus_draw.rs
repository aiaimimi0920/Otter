//! Keyboard-focus indicators follow node drawing order and inherit group opacity/clipping, rather than floating above later siblings.
use crate::{surface_clip::Rect, surface_layout_tree::Placed};
use windows::Win32::Graphics::Direct2D::{Common::*, *};

pub struct Focus {
    id: String,
    rect: Rect,
    clip: Rect,
}
impl Focus {
    pub fn new(placed: &Placed<'_>, clip: Rect) -> Self {
        Self {
            id: placed.node.id().to_owned(),
            rect: Rect::from_box(placed.origin, placed.size),
            clip,
        }
    }
    pub fn draw(
        &self,
        focus: Option<&str>,
        target: &ID2D1RenderTarget,
        brush: &ID2D1SolidColorBrush,
    ) {
        if focus != Some(self.id.as_str())
            || self.clip.empty()
            || self.rect.right - self.rect.left < 2.
            || self.rect.bottom - self.rect.top < 2.
        {
            return;
        }
        unsafe {
            let previous = brush.GetColor();
            brush.SetColor(&D2D1_COLOR_F {
                r: 217. / 255.,
                g: 1.,
                b: 56. / 255.,
                a: 1.,
            });
            target.PushAxisAlignedClip(
                &D2D_RECT_F {
                    left: self.clip.left,
                    top: self.clip.top,
                    right: self.clip.right,
                    bottom: self.clip.bottom,
                },
                D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
            );
            target.DrawRectangle(
                &D2D_RECT_F {
                    left: self.rect.left + 1.,
                    top: self.rect.top + 1.,
                    right: self.rect.right - 1.,
                    bottom: self.rect.bottom - 1.,
                },
                brush,
                2.,
                None,
            );
            target.PopAxisAlignedClip();
            brush.SetColor(&previous);
        }
    }
}
