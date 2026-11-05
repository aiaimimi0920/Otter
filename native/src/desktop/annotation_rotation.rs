//! 临时 D2D 变换在所有返回路径恢复，避免一条标注的角度污染后续绘制。
use crate::{editor::Stroke, rotation, viewport::Viewport};
use windows::Win32::Graphics::Direct2D::ID2D1RenderTarget;

/// 当前变换已经包含对象旋转及宿主裁剪/缩放。将目标四角完整逆变换，
/// 而非只逆对象角度，否则裁剪导出会错误剔除仍可见的效果区域。
pub(super) unsafe fn visible_bounds(
    target: &ID2D1RenderTarget,
    margin: f32,
) -> windows::core::Result<(f32, f32, f32, f32)> {
    let mut m = Default::default();
    target.GetTransform(&mut m);
    let determinant = m.M11 as f64 * m.M22 as f64 - m.M12 as f64 * m.M21 as f64;
    if !determinant.is_finite() || determinant == 0.0 {
        return Err(windows::core::Error::new(
            windows::Win32::Foundation::E_INVALIDARG,
            "Invalid render transform",
        ));
    }
    let size = target.GetSize();
    let mut bounds = (
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    );
    for (x, y) in [
        (-margin, -margin),
        (size.width + margin, -margin),
        (-margin, size.height + margin),
        (size.width + margin, size.height + margin),
    ] {
        let x = x as f64 - m.M31 as f64;
        let y = y as f64 - m.M32 as f64;
        let px = ((x * m.M22 as f64 - y * m.M21 as f64) / determinant) as f32;
        let py = ((y * m.M11 as f64 - x * m.M12 as f64) / determinant) as f32;
        if !px.is_finite() || !py.is_finite() {
            return Err(windows::core::Error::new(
                windows::Win32::Foundation::E_INVALIDARG,
                "Invalid visible bounds",
            ));
        }
        bounds.0 = bounds.0.min(px);
        bounds.1 = bounds.1.min(py);
        bounds.2 = bounds.2.max(px);
        bounds.3 = bounds.3.max(py);
    }
    Ok(bounds)
}

struct Restore<F: FnOnce()>(Option<F>);
impl<F: FnOnce()> Drop for Restore<F> {
    fn drop(&mut self) {
        if let Some(restore) = self.0.take() {
            restore();
        }
    }
}

pub(super) unsafe fn apply<'a>(
    target: &'a ID2D1RenderTarget,
    stroke: &Stroke,
    viewport: Viewport,
) -> impl Drop + 'a {
    let mut original = Default::default();
    target.GetTransform(&mut original);
    if stroke.rotation != 0.0 {
        // 已恢复局部效果按持久框中心旋转；普通工具仍使用原来的笔迹中心。
        let center = stroke
            .blur
            .as_ref()
            .filter(|image| image.effect_clip().is_some())
            .map(|image| {
                let frame = image.frame();
                crate::viewport::Point {
                    x: (frame.x + frame.width / 2.0) as f32,
                    y: (frame.y + frame.height / 2.0) as f32,
                }
            })
            .unwrap_or_else(|| rotation::center(&stroke.points));
        let x = viewport.origin.x + center.x * viewport.scale;
        let y = viewport.origin.y + center.y * viewport.scale;
        let (sin, cos) = (stroke.rotation % 360.0).to_radians().sin_cos();
        let (tx, ty) = (x - x * cos + y * sin, y - x * sin - y * cos);
        let mut matrix = original;
        matrix.M11 = cos * original.M11 + sin * original.M21;
        matrix.M12 = cos * original.M12 + sin * original.M22;
        matrix.M21 = -sin * original.M11 + cos * original.M21;
        matrix.M22 = -sin * original.M12 + cos * original.M22;
        matrix.M31 = tx * original.M11 + ty * original.M21 + original.M31;
        matrix.M32 = tx * original.M12 + ty * original.M22 + original.M32;
        target.SetTransform(&matrix);
    }
    Restore(Some(move || {
        target.SetTransform(&original);
    }))
}
