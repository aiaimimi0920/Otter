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
fn mul(a: [f32; 6], b: [f32; 6]) -> [f32; 6] {
    [
        a[0] * b[0] + a[1] * b[2],
        a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2],
        a[2] * b[1] + a[3] * b[3],
        a[4] * b[0] + a[5] * b[2] + b[4],
        a[4] * b[1] + a[5] * b[3] + b[5],
    ]
}
fn apply_affine(m: [f32; 6], p: D2D_POINT_2F) -> D2D_POINT_2F {
    D2D_POINT_2F {
        x: m[0] * p.x + m[2] * p.y + m[4],
        y: m[1] * p.x + m[3] * p.y + m[5],
    }
}
fn rotation_matrix(degrees: f32, center: D2D_POINT_2F) -> [f32; 6] {
    if degrees == 0.0 {
        return [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    }
    let rad = f64::from(degrees).to_radians();
    let (s, c) = rad.sin_cos();
    let (s, c) = (s as f32, c as f32);
    [
        c,
        s,
        -s,
        c,
        center.x - c * center.x + s * center.y,
        center.y - s * center.x - c * center.y,
    ]
}
fn ray_exit_t(
    vx: f32,
    vy: f32,
    hw: f32,
    hh: f32,
    radius: f32,
    shape: crate::editor::LensShape,
) -> f32 {
    if hw <= 0.0 || hh <= 0.0 || (vx == 0.0 && vy == 0.0) {
        return 1.0;
    }
    match shape {
        crate::editor::LensShape::Ellipse => {
            let d = (vx / hw).hypot(vy / hh);
            if d > 1e-5 {
                1.0 / d
            } else {
                1.0
            }
        }
        crate::editor::LensShape::Rect => {
            let d = (vx.abs() / hw).max(vy.abs() / hh);
            if d > 1e-5 {
                1.0 / d
            } else {
                1.0
            }
        }
        crate::editor::LensShape::Rounded => {
            let ax = vx.abs();
            let ay = vy.abs();
            let d = (ax / hw).max(ay / hh);
            if d <= 1e-5 {
                return 1.0;
            }
            let t_rect = 1.0 / d;
            let cr = radius.clamp(0.0, hw.min(hh));
            if cr <= 0.5 || t_rect * ax <= hw - cr || t_rect * ay <= hh - cr {
                return t_rect;
            }
            let (cx, cy) = (hw - cr, hh - cr);
            let a = ax * ax + ay * ay;
            let b = ax * cx + ay * cy;
            let c = cx * cx + cy * cy - cr * cr;
            ((b + (b * b - a * c).max(0.0).sqrt()) / a).min(t_rect)
        }
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
    let limit = stage.image.GetSize();
    if !limit.width.is_finite()
        || !limit.height.is_finite()
        || limit.width <= 0.
        || limit.height <= 0.
    {
        return Ok(());
    }
    let mut m = Default::default();
    target.GetTransform(&mut m);
    let m_arr = [m.M11, m.M12, m.M21, m.M22, m.M31, m.M32];
    let center_view = transform(center, view);
    let p_view = transform(
        crate::rotation::rotate(source, center, stroke.rotation),
        view,
    );
    let c_stage = apply_affine(m_arr, center_view);
    let p_stage = apply_affine(m_arr, p_view);
    let z = f32::from(lens.percent) / 100.;
    let factory = target.GetFactory()?;
    let hole_tool = match lens.shape {
        crate::editor::LensShape::Rect => crate::editor::Tool::SpotRect,
        crate::editor::LensShape::Rounded => crate::editor::Tool::SpotRounded,
        crate::editor::LensShape::Ellipse => crate::editor::Tool::SpotEllipse,
    };
    let local_geom = super::spotlight::hole(&factory, hole_tool, dest, stroke.corner_radius)?;
    let g_arr = mul(rotation_matrix(stroke.rotation, center_view), m_arr);
    let mut geom_to_stage = m;
    geom_to_stage.M11 = g_arr[0];
    geom_to_stage.M12 = g_arr[1];
    geom_to_stage.M21 = g_arr[2];
    geom_to_stage.M22 = g_arr[3];
    geom_to_stage.M31 = g_arr[4];
    geom_to_stage.M32 = g_arr[5];
    let lens_stage = factory.CreateTransformedGeometry(&local_geom, &geom_to_stage)?;
    let clip_rect = D2D_RECT_F {
        left: c_stage.x + z * (0.0 - p_stage.x),
        top: c_stage.y + z * (0.0 - p_stage.y),
        right: c_stage.x + z * (limit.width - p_stage.x),
        bottom: c_stage.y + z * (limit.height - p_stage.y),
    };
    let clipped = if clip_rect.right <= clip_rect.left || clip_rect.bottom <= clip_rect.top {
        None
    } else {
        let bounds_geom = factory.CreateRectangleGeometry(&clip_rect)?;
        let path = factory.CreatePathGeometry()?;
        let sink = path.Open()?;
        lens_stage.CombineWithGeometry(
            &bounds_geom,
            D2D1_COMBINE_MODE_INTERSECT,
            None,
            0.25,
            &sink,
        )?;
        sink.Close()?;
        Some(path)
    };
    let _restore = super::super::render_frame::save_transform(target);
    let mut identity = m;
    identity.M11 = 1.0;
    identity.M12 = 0.0;
    identity.M21 = 0.0;
    identity.M22 = 1.0;
    identity.M31 = 0.0;
    identity.M32 = 0.0;
    target.SetTransform(&identity);
    let alpha = (stroke.opacity * stroke.paint_alpha.stroke()).clamp(0.0, 1.0);
    let crate::stroke_style::Color(cr, cg, cb) = stroke.color;
    let rim = target.CreateSolidColorBrush(
        &D2D1_COLOR_F {
            r: f32::from(cr) / 255.0,
            g: f32::from(cg) / 255.0,
            b: f32::from(cb) / 255.0,
            a: alpha,
        },
        None,
    )?;
    let mut detached_src = None;
    if lens.offset != crate::viewport::Point::default() && alpha > 0.0 {
        let src_view = transform(source, view);
        let dst_hw = (dest.right - dest.left) * 0.5;
        let dst_hh = (dest.bottom - dest.top) * 0.5;
        let hw = dst_hw / z;
        let hh = dst_hh / z;
        let src_rect = D2D_RECT_F {
            left: src_view.x - hw,
            top: src_view.y - hh,
            right: src_view.x + hw,
            bottom: src_view.y + hh,
        };
        if src_rect.right > src_rect.left && src_rect.bottom > src_rect.top {
            let src_r = (stroke.corner_radius / z).max(1.0);
            let src_local = super::spotlight::hole(&factory, hole_tool, src_rect, src_r)?;
            let src_stage = factory.CreateTransformedGeometry(&src_local, &geom_to_stage)?;
            let vx = center_view.x - src_view.x;
            let vy = center_view.y - src_view.y;
            let t0 = ray_exit_t(vx, vy, hw, hh, src_r, lens.shape);
            let t1 = ray_exit_t(-vx, -vy, dst_hw, dst_hh, stroke.corner_radius, lens.shape);
            // Draw pure stroke.color dashed leader line strictly underneath magnified fill (no out-of-phase halo).
            if t0 + t1 < 0.98 {
                let l0 = apply_affine(
                    g_arr,
                    D2D_POINT_2F {
                        x: src_view.x + vx * t0,
                        y: src_view.y + vy * t0,
                    },
                );
                let l1 = apply_affine(
                    g_arr,
                    D2D_POINT_2F {
                        x: center_view.x - vx * t1,
                        y: center_view.y - vy * t1,
                    },
                );
                let dashed = factory.CreateStrokeStyle(
                    &D2D1_STROKE_STYLE_PROPERTIES {
                        startCap: D2D1_CAP_STYLE_ROUND,
                        endCap: D2D1_CAP_STYLE_ROUND,
                        dashCap: D2D1_CAP_STYLE_ROUND,
                        dashStyle: D2D1_DASH_STYLE_CUSTOM,
                        ..Default::default()
                    },
                    Some(&[4.0, 3.0]),
                )?;
                target.DrawLine(l0, l1, &rim, 1.6, &dashed);
            }
            detached_src = Some(src_stage);
        }
    }
    if let Some(path) = clipped {
        let mut brush_matrix = identity;
        brush_matrix.M11 = z;
        brush_matrix.M22 = z;
        brush_matrix.M31 = c_stage.x - z * p_stage.x;
        brush_matrix.M32 = c_stage.y - z * p_stage.y;
        let brush = target.CreateBitmapBrush(
            &stage.image,
            Some(&D2D1_BITMAP_BRUSH_PROPERTIES {
                extendModeX: D2D1_EXTEND_MODE_CLAMP,
                extendModeY: D2D1_EXTEND_MODE_CLAMP,
                interpolationMode: D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
            }),
            Some(&D2D1_BRUSH_PROPERTIES {
                opacity: 1.0,
                transform: brush_matrix,
            }),
        )?;
        target.FillGeometry(&path, &brush, None);
    }
    if let Some(src_stage) = detached_src {
        target.DrawGeometry(&src_stage, &rim, 1.6, None);
    }
    if alpha > 0.0 {
        target.DrawGeometry(&lens_stage, &rim, 1.8, None);
    }
    Ok(())
}
