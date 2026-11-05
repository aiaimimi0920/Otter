// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Resample both axes before convolution, then reconstruct once to avoid an
//! intermediate full-resolution quantization between horizontal/vertical passes.
use super::{scaled_frame::Frame, ImageRefMut};
use rgb::RGBA8;

pub fn apply(sx: f64, sy: f64, image: ImageRefMut) {
    let sigmas = [sx, sy];
    if sigmas
        .iter()
        .any(|s| !s.is_finite() || *s > f64::from(f32::MAX))
    {
        image.data.fill(RGBA8::default());
        return;
    }
    let net = sigmas.map(|s| if s > 4.0 { 4.0 / s } else { 1.0 });
    let mut steps = net.map(|n| {
        let mut count = (1.0 / n).log2().ceil() as i32;
        if count > 0 && n * 2.0f64.powi(count - 1) >= if count == 1 { 0.999 } else { 0.9 } {
            count -= 1;
        }
        count
    });
    let mut frame = Frame {
        width: image.width as usize,
        height: image.height as usize,
        origin: [0, 0],
        // The first reduction only reads input; do not allocate a full-size copy.
        pixels: std::borrow::Cow::Borrowed(image.data),
    };
    let (mut left, mut right) = ([0.0; 2], [f64::from(image.width), f64::from(image.height)]);
    let (mut totals, mut translations) = ([1.0; 2], [0.0; 2]);
    while steps.iter().any(|n| *n > 0) {
        let mut scales = [1.0; 2];
        let mut centers = [0.0; 2];
        for i in 0..2 {
            if steps[i] > 0 {
                scales[i] = if steps[i] > 1 {
                    0.5
                } else {
                    net[i] / totals[i]
                };
                centers[i] = (left[i] + right[i]) * 0.5;
            }
            left[i] = (left[i] - centers[i]) * scales[i];
            right[i] = (right[i] - centers[i]) * scales[i];
            totals[i] *= scales[i];
            translations[i] = (translations[i] - centers[i]) * scales[i];
            steps[i] = (steps[i] - 1).max(0);
        }
        frame = frame.resample(left, right, scales, centers);
    }
    frame = frame.blur((sx * totals[0]).min(4.0), true);
    frame = frame.blur((sy * totals[1]).min(4.0), false);
    let frame = frame.into_owned();
    for y in 0..image.height as usize {
        for x in 0..image.width as usize {
            image.data[y * image.width as usize + x] = frame.sample([
                (x as f64 + 0.5) * totals[0] + translations[0],
                (y as f64 + 0.5) * totals[1] + translations[1],
            ]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_sigma_clears_before_allocating_frames() {
        for sigma in [f64::INFINITY, f64::NAN, f64::MAX] {
            let mut pixels = vec![RGBA8::new(1, 2, 3, 255); 4];
            apply(sigma, 0.0, ImageRefMut::new(2, 2, &mut pixels));
            assert!(pixels.iter().all(|p| *p == RGBA8::default()));
        }
    }
    #[test]
    fn single_axis_transposition_preserves_premultiplication() {
        for sigma in [8.01, 9.0, 16.0, 100.0, 1e38] {
            let input: Vec<_> = (0..64)
                .map(|i| {
                    let a = ((i * 71) % 256) as u8;
                    RGBA8::new(a / 2, a / 3, a, a)
                })
                .collect();
            let mut x = input.clone();
            let mut y = input;
            apply(sigma, 0.0, ImageRefMut::new(64, 1, &mut x));
            apply(0.0, sigma, ImageRefMut::new(1, 64, &mut y));
            assert_eq!(x, y, "sigma={sigma}");
            assert!(x.iter().all(|p| p.r <= p.a && p.g <= p.a && p.b <= p.a));
        }
    }
    #[test]
    fn large_sigma_joint_resource_probe() {
        for sigma in [9.0, 32.0, 1e9, 1e38] {
            let mut pixels = vec![RGBA8::new(32, 64, 128, 255); 1024 * 1024];
            let start = std::time::Instant::now();
            apply(sigma, sigma, ImageRefMut::new(1024, 1024, &mut pixels));
            let expected = if sigma < 100.0 {
                RGBA8::new(32, 64, 128, 255)
            } else {
                RGBA8::default()
            };
            assert_eq!(pixels[512 * 1024 + 512], expected);
            println!(
                "joint-gaussian 1024x1024 sigma={sigma} elapsed_ms={}",
                start.elapsed().as_millis()
            );
        }
    }
}
