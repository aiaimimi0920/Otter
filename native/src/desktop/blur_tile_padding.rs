//! 取样邻域需覆盖一个设备像素足迹及线性插值邻点；只留一个源像素会在缩小时露缝。
use windows::{
    core::{Error, Result},
    Win32::{Foundation::E_INVALIDARG, Graphics::Direct2D::ID2D1RenderTarget},
};

pub(super) unsafe fn for_target(
    target: &ID2D1RenderTarget,
    scale: (f32, f32),
    size: (u32, u32),
) -> Result<(u32, u32)> {
    let mut m = Default::default();
    target.GetTransform(&mut m);
    let determinant = (m.M11 as f64 * m.M22 as f64 - m.M12 as f64 * m.M21 as f64).abs();
    if !determinant.is_finite()
        || determinant == 0.0
        || ![scale.0, scale.1].iter().all(|v| v.is_finite() && *v > 0.0)
    {
        return Err(Error::new(E_INVALIDARG, "Invalid blur tile footprint"));
    }
    // 逆矩阵每一行的范数映射单位设备圆；覆盖像素方形的半对角线，同时适用于剪切。
    let x = (m.M22 as f64).hypot(m.M21 as f64) / determinant / scale.0 as f64;
    let y = (m.M12 as f64).hypot(m.M11 as f64) / determinant / scale.1 as f64;
    Ok((bucket(x, size.0), bucket(y, size.1)))
}
fn bucket(footprint: f64, limit: u32) -> u32 {
    // 分桶避免每次轻微缩放都产生新上传键；源尺寸已由 BlurImage 限制在 16384 内。
    ((footprint.ceil() + 1.0).min(limit as f64) as u32)
        .max(1)
        .next_power_of_two()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blur_padding_buckets_cover_minification_and_remain_source_bounded() {
        assert_eq!(bucket(1.0, 2052), 2);
        assert_eq!(bucket(4.01, 2052), 8);
        assert_eq!(bucket(5.4, 2052), 8);
        assert_eq!(bucket(f64::INFINITY, 16384), 16384);
        assert_eq!(bucket(100.0, 1), 1);
    }
}
