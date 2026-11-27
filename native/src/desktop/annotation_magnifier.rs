//! A lens can only consume a successfully ended, same-transaction redacted
//! render target. Neither HWND pixels nor the input image are sampling APIs.
use super::*;
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_FAIL, E_OUTOFMEMORY},
};

#[derive(Clone, Copy)]
pub(in crate::desktop) enum Pass<'a> {
    Plain,
    Lower,
    Paused,
    Ready(&'a Sealed),
}
pub(in crate::desktop) struct Sealed {
    image: ID2D1Bitmap,
    _lease: crate::byte_budget::Lease,
}
impl Pass<'_> {
    pub(in crate::desktop) fn lower(self) -> bool {
        matches!(self, Self::Lower)
    }
}
pub(in crate::desktop) unsafe fn scene(
    frame: &mut Option<Sealed>,
    target: &ID2D1RenderTarget,
    needed: bool,
    preview: bool,
    reserve: impl Fn(usize) -> Option<crate::byte_budget::Lease>,
    mut paint: impl FnMut(&ID2D1RenderTarget, Pass<'_>) -> Result<()>,
) -> Result<()> {
    if !needed {
        return paint(target, Pass::Plain);
    }
    let stage = (|| -> Result<Sealed> {
        let pixels = target.GetPixelSize();
        let size = target.GetSize();
        let lease = crate::viewport::checked_image_bytes(pixels.width, pixels.height)
            .and_then(reserve)
            .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Redacted lens surface budget exhausted"))?;
        let intermediate = target.CreateCompatibleRenderTarget(
            Some(&size),
            Some(&pixels),
            None,
            D2D1_COMPATIBLE_RENDER_TARGET_OPTIONS_NONE,
        )?;
        super::super::render_frame::draw(&intermediate, || paint(&intermediate, Pass::Lower))?;
        Ok(Sealed {
            image: intermediate.GetBitmap()?,
            _lease: lease,
        })
    })();
    match stage {
        // The caller owns this slot outside BeginDraw/EndDraw. Direct2D may
        // defer DrawBitmap until EndDraw, including when later painting fails.
        Ok(sealed) => paint(target, Pass::Ready(frame.insert(sealed))),
        Err(_) if preview => paint(target, Pass::Paused),
        Err(e) => Err(e),
    }
}
pub(super) unsafe fn draw(
    target: &ID2D1RenderTarget,
    pass: Pass<'_>,
    stroke: &Stroke,
    view: Viewport,
) -> Result<()> {
    let stage = match pass {
        Pass::Ready(stage) => stage,
        Pass::Paused => return Ok(()),
        _ => {
            return Err(Error::new(
                E_FAIL,
                "Lens requires a redacted document surface",
            ))
        }
    };
    let Some([source, center]) = stroke.lens_handles() else {
        return Ok(());
    };
    let [a, b] = stroke.points.as_slice() else {
        return Ok(());
    };
    let a = transform(*a, view);
    let b = transform(*b, view);
    let dest = D2D_RECT_F {
        left: a.x.min(b.x),
        top: a.y.min(b.y),
        right: a.x.max(b.x),
        bottom: a.y.max(b.y),
    };
    if dest.right <= dest.left || dest.bottom <= dest.top {
        return Ok(());
    }
    let lens = stroke
        .lens
        .ok_or_else(|| Error::new(E_FAIL, "Missing lens geometry"))?;
    let p = transform(
        crate::rotation::rotate(source, center, stroke.rotation),
        view,
    );
    let mut m = Default::default();
    target.GetTransform(&mut m);
    if m.M12 != 0. || m.M21 != 0. || m.M11 <= 0. || m.M22 <= 0. {
        return Err(Error::new(E_FAIL, "Unsupported lens document transform"));
    }
    let x = m.M11 * p.x + m.M31;
    let y = m.M22 * p.y + m.M32;
    let z = f32::from(lens.percent) / 100.;
    let w = (dest.right - dest.left) * m.M11 / (2. * z);
    let h = (dest.bottom - dest.top) * m.M22 / (2. * z);
    let source = D2D_RECT_F {
        left: x - w,
        right: x + w,
        top: y - h,
        bottom: y + h,
    };
    let limit = stage.image.GetSize();
    let cl = source.left.max(0.);
    let ct = source.top.max(0.);
    let cr = source.right.min(limit.width);
    let cb = source.bottom.min(limit.height);
    if !limit.width.is_finite()
        || !limit.height.is_finite()
        || source.right <= source.left
        || source.bottom <= source.top
        || cr <= cl
        || cb <= ct
    {
        return Ok(());
    }
    let (sw, sh) = (source.right - source.left, source.bottom - source.top);
    let (dw, dh) = (dest.right - dest.left, dest.bottom - dest.top);
    let dest = D2D_RECT_F {
        left: dest.left + (cl - source.left) / sw * dw,
        top: dest.top + (ct - source.top) / sh * dh,
        right: dest.left + (cr - source.left) / sw * dw,
        bottom: dest.top + (cb - source.top) / sh * dh,
    };
    let source = D2D_RECT_F {
        left: cl,
        top: ct,
        right: cr,
        bottom: cb,
    };
    let _rotation = super::super::annotation_rotation::apply(target, stroke, view);
    target.DrawBitmap(
        &stage.image,
        Some(&dest),
        1.,
        D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
        Some(&source),
    );
    Ok(())
}
