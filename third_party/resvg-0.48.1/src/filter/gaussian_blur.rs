// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Bounded finite Gaussian convolution for small sigmas, without the recursive
//! approximation's edge/tail errors. Larger sigmas use bounded resampling.
use super::{joint_gaussian, ImageRefMut};
use rgb::RGBA8;

pub fn apply(sx: f64, sy: f64, image: ImageRefMut) {
    if sx > 8.0 || sy > 8.0 {
        joint_gaussian::apply(sx, sy, image);
        return;
    }
    if super::small_gaussian_2d::apply(
        sx,
        sy,
        ImageRefMut::new(image.width, image.height, image.data),
    ) {
        return;
    }
    let mut buffer = Vec::new();
    for (sigma, horizontal) in [(sx, true), (sy, false)] {
        if sigma <= 0.0 {
            continue;
        }
        let radius = (3.0 * sigma).ceil() as i32;
        let mut weights: Vec<_> = (-radius..=radius)
            .map(|x| (-f64::from(x * x) / (2.0 * sigma * sigma)).exp())
            .collect();
        let sum: f64 = weights.iter().sum();
        for weight in &mut weights {
            *weight /= sum;
        }
        buffer.resize(image.data.len(), RGBA8::default());
        let width = image.width as i32;
        let height = image.height as i32;
        for y in 0..height {
            for x in 0..width {
                let mut value = [0.0; 4];
                for (offset, weight) in (-radius..=radius).zip(&weights) {
                    let (ix, iy) = if horizontal {
                        (x + offset, y)
                    } else {
                        (x, y + offset)
                    };
                    // The default edge mode is transparent; don't renormalize edges.
                    if ix < 0 || iy < 0 || ix >= width || iy >= height {
                        continue;
                    }
                    let pixel = image.data[(iy * width + ix) as usize];
                    for (sum, channel) in value.iter_mut().zip([pixel.r, pixel.g, pixel.b, pixel.a])
                    {
                        *sum += f64::from(channel) * weight;
                    }
                }
                let [r, g, b, a] = value.map(|v| v.round().clamp(0.0, 255.0) as u8);
                buffer[(y * width + x) as usize] = RGBA8::new(r, g, b, a);
            }
        }
        image.data.copy_from_slice(&buffer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finite_kernel_preserves_zero_axis_and_transparent_edges() {
        for (sigma, expected) in [
            (0.0, vec![0, 0, 0, 255, 0, 0, 0]),
            (0.5, vec![0, 0, 27, 201, 27, 0, 0]),
            (1.0, vec![1, 14, 62, 102, 62, 14, 1]),
        ] {
            let mut image = vec![RGBA8::default(); 7];
            image[3] = RGBA8::new(255, 255, 255, 255);
            apply(sigma, 0.0, ImageRefMut::new(7, 1, &mut image));
            assert_eq!(image.iter().map(|p| p.a).collect::<Vec<_>>(), expected);
            assert!(image.iter().all(|p| p.r == p.a && p.g == p.a && p.b == p.a));
        }
        let mut image = vec![RGBA8::new(255, 255, 255, 255); 1];
        apply(1.0, 0.0, ImageRefMut::new(1, 1, &mut image));
        assert_eq!(image[0].a, 102, "transparent padding is not renormalized");
    }

    #[test]
    fn finite_kernel_resource_probe() {
        for sigma in [0.3, 2.0, 8.0] {
            let mut pixels = vec![RGBA8::new(32, 64, 128, 255); 1024 * 1024];
            let start = std::time::Instant::now();
            apply(sigma, sigma, ImageRefMut::new(1024, 1024, &mut pixels));
            assert_eq!(pixels[512 * 1024 + 512], RGBA8::new(32, 64, 128, 255));
            println!(
                "gaussian 1024x1024 sigma={sigma} elapsed_ms={}",
                start.elapsed().as_millis()
            );
        }
    }
}
