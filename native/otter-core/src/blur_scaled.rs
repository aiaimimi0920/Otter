//! 源像素密度的双轴高斯核；逻辑强度不变，避免先低分辨率模糊再放大。
use super::*;

pub fn region_scaled(
    source: &Pixels,
    size: (u32, u32),
    area: Region,
    strength: f32,
    scale: (f32, f32),
    cancelled: impl Fn() -> bool,
) -> Result<Pixels, BlurError> {
    if !strength.is_finite()
        || !(1.0..=64.0).contains(&strength)
        || ![scale.0, scale.1].iter().all(|n| n.is_finite() && *n > 0.0)
    {
        return Err(BlurError::InvalidInput);
    }
    region_axes(
        source,
        size,
        area,
        (strength * scale.0, strength * scale.1),
        cancelled,
    )
}

pub(super) struct Kernel {
    pub weights: Vec<f32>,
    pub radius: i32,
    _lease: crate::byte_budget::Lease,
}
impl Kernel {
    pub fn new(sigma: f32) -> Result<Self, BlurError> {
        // 极端密度不能造成无界核和 CPU 工作；拒绝而不是静默钳制模糊强度。
        if !sigma.is_finite() || sigma <= 0.0 || sigma > 1024.0 {
            return Err(BlurError::InvalidInput);
        }
        let radius = (sigma * 3.0).ceil() as i32;
        let (mut weights, lease) = allocate::<f32>((radius * 2 + 1) as usize)?;
        let mut total = 0.0;
        for (i, weight) in weights.iter_mut().enumerate() {
            let distance = i as f32 - radius as f32;
            // 常规强度保留旧内核浮点运算顺序；极小标准差平方下溢时再避免 0/0。
            let denominator = 2.0 * sigma.powi(2);
            *weight = if denominator > 0.0 {
                (-distance * distance / denominator).exp()
            } else {
                let normalized = distance / sigma;
                (-normalized * normalized / 2.0).exp()
            };
            total += *weight;
        }
        for weight in &mut weights {
            *weight /= total;
        }
        Ok(Self {
            weights,
            radius,
            _lease: lease,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn anisotropic_impulse_matches_independent_two_axis_reference() {
        let mut bytes = vec![0; 31 * 31 * 4];
        bytes[(15 * 31 + 15) * 4..(15 * 31 + 15) * 4 + 4].fill(255);
        let source: Pixels = bytes.into();
        let area = Region {
            x: 0,
            y: 0,
            width: 31,
            height: 31,
        };
        let output = region_scaled(&source, (31, 31), area, 1.5, (2.0, 0.5), || false).unwrap();
        let kernel = |offset: i32, sigma: f64| {
            let radius = (3.0 * sigma).ceil() as i32;
            if offset.abs() > radius {
                return 0.0;
            }
            let weight = |x: i32| (-(x as f64).powi(2) / (2.0 * sigma * sigma)).exp();
            weight(offset) / (-radius..=radius).map(weight).sum::<f64>()
        };
        for y in 0..31 {
            for x in 0..31 {
                let expected = (255.0 * kernel(x as i32 - 15, 3.0) * kernel(y as i32 - 15, 0.75))
                    .round() as u8;
                assert_eq!(output[(y * 31 + x) * 4 + 3], expected);
            }
        }
    }
    #[test]
    fn unit_density_and_subregion_keep_existing_result() {
        let source: Pixels = (0..16 * 24)
            .flat_map(|n| [n as u8; 4])
            .collect::<Vec<_>>()
            .into();
        let area = Region {
            x: 3,
            y: 4,
            width: 7,
            height: 16,
        };
        for strength in [1.0, 4.4, 64.0] {
            assert_eq!(
                region_precise(&source, (16, 24), area, strength, || false).unwrap(),
                region_scaled(&source, (16, 24), area, strength, (1.0, 1.0), || false).unwrap()
            );
        }
    }
    #[test]
    fn invalid_scale_and_cancel_do_not_return_partial_pixels() {
        let source: Pixels = vec![255; 64].into();
        let area = Region {
            x: 0,
            y: 0,
            width: 4,
            height: 4,
        };
        for scale in [
            (0.0, 1.0),
            (-1.0, 1.0),
            (f32::NAN, 1.0),
            (1.0, f32::INFINITY),
            (2048.0, 1.0),
        ] {
            assert_eq!(
                region_scaled(&source, (4, 4), area, 1.0, scale, || false),
                Err(BlurError::InvalidInput)
            );
        }
        assert_eq!(
            region_scaled(&source, (4, 4), area, 1.0, (2.0, 0.5), || true),
            Err(BlurError::Cancelled)
        );
    }
}
