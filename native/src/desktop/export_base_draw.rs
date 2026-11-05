//! 底图专属裁剪与源子矩形绘制；clip 不得泄漏到边框或标注。
use super::*;

pub(in crate::desktop::export) fn draw(
    target: &ID2D1RenderTarget,
    bitmap: &ID2D1Bitmap,
    layout: Option<&Layout>,
    opacity: f32,
) {
    let mut destination = layout.map(|layout| layout.base);
    let mut source = None;
    match layout.map(|layout| layout.sampling).unwrap_or_default() {
        Sampling::Empty => return,
        Sampling::Region(projection) => {
            destination = Some(rect(projection.target));
            source = Some(rect(projection.source));
        }
        Sampling::Full => {}
    }
    let clip = layout.and_then(|layout| layout.base_clip);
    unsafe {
        if let Some(clip) = clip {
            target.PushAxisAlignedClip(&clip, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
        }
        target.DrawBitmap(
            bitmap,
            destination.as_ref().map(|r| r as *const _),
            opacity,
            layout.map_or(D2D1_BITMAP_INTERPOLATION_MODE_NEAREST_NEIGHBOR, |layout| {
                layout.interpolation
            }),
            source.as_ref().map(|r| r as *const _),
        );
        if clip.is_some() {
            target.PopAxisAlignedClip();
        }
    }
}
fn rect(r: Rect) -> D2D_RECT_F {
    D2D_RECT_F {
        left: r.x as f32,
        top: r.y as f32,
        right: (r.x + r.width) as f32,
        bottom: (r.y + r.height) as f32,
    }
}
