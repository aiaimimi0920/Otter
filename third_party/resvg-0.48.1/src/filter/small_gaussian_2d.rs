// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Single quantization for a bounded two-dimensional Gaussian kernel.
use super::ImageRefMut;
use rgb::RGBA8;

pub(super) fn apply(sx: f64, sy: f64, image: ImageRefMut) -> bool {
    if !sx.is_finite() || !sy.is_finite() || sx <= 0.0 || sy <= 0.0 || sx > 4.0 || sy > 4.0 {
        return false;
    }
    let rx = (3.0 * sx).ceil() as i32;
    let ry = (3.0 * sy).ceil() as i32;
    if (2 * rx + 1) * (2 * ry + 1) > 28 {
        return false;
    }
    let mut taps = Vec::new();
    for y in -ry..=ry {
        for x in -rx..=rx {
            let weight =
                (-f64::from(x * x) / (2.0 * sx * sx) - f64::from(y * y) / (2.0 * sy * sy)).exp();
            taps.push((x, y, weight));
        }
    }
    let total: f64 = taps.iter().map(|tap| tap.2).sum();
    for tap in &mut taps {
        tap.2 /= total;
    }
    let mut output = vec![RGBA8::default(); image.data.len()];
    let (width, height) = (image.width as i32, image.height as i32);
    for y in 0..height {
        for x in 0..width {
            let mut sums = [0.0; 4];
            for &(dx, dy, weight) in &taps {
                let (xx, yy) = (x + dx, y + dy);
                if xx < 0 || yy < 0 || xx >= width || yy >= height {
                    continue;
                }
                let p = image.data[(yy * width + xx) as usize];
                for (sum, channel) in sums.iter_mut().zip([p.r, p.g, p.b, p.a]) {
                    *sum += f64::from(channel) * weight;
                }
            }
            let [r, g, b, a] = sums.map(|v| v.round().clamp(0.0, 255.0) as u8);
            output[(y * width + x) as usize] = RGBA8::new(r, g, b, a);
        }
    }
    image.data.copy_from_slice(&output);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_kernel_and_transparent_edge() {
        let mut p = [RGBA8::new(255, 255, 255, 255)];
        assert!(!apply(0.0, 0.5, ImageRefMut::new(1, 1, &mut p)));
        assert!(!apply(1.0, 1.0, ImageRefMut::new(1, 1, &mut p)));
        assert_eq!(p[0].a, 255);
        assert!(apply(0.5, 0.5, ImageRefMut::new(1, 1, &mut p)));
        assert_eq!(p[0], RGBA8::new(158, 158, 158, 158));
    }
}
