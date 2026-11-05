// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Convert premultiplied colors without quantizing intermediate straight channels.
use rgb::RGBA8;

pub(super) fn channel(straight: f64, linear: bool) -> f64 {
    if linear {
        if straight <= 0.04045 {
            straight / 12.92
        } else {
            ((straight + 0.055) / 1.055).powf(2.4)
        }
    } else if straight <= 0.0031308 {
        straight * 12.92
    } else {
        1.055 * straight.powf(1.0 / 2.4) - 0.055
    }
}

pub(super) fn convert(pixels: &mut [RGBA8], linear: bool) {
    for pixel in pixels {
        let alpha = f64::from(pixel.a);
        if alpha == 0.0 {
            *pixel = RGBA8::default();
            continue;
        }
        for channel in [&mut pixel.r, &mut pixel.g, &mut pixel.b] {
            let straight = (f64::from(*channel) / alpha).clamp(0.0, 1.0);
            let value = self::channel(straight, linear);
            *channel = (value * alpha).round().clamp(0.0, alpha) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_reference_values_and_transparent_sanitization() {
        let mut pixels = [RGBA8::new(64, 0, 128, 128), RGBA8::new(12, 34, 56, 0)];
        convert(&mut pixels, true);
        assert_eq!(pixels, [RGBA8::new(27, 0, 128, 128), RGBA8::default()]);
        convert(&mut pixels, false);
        assert_eq!(pixels, [RGBA8::new(64, 0, 128, 128), RGBA8::default()]);
    }

    #[test]
    fn transfer_preserves_alpha_and_premultiplication() {
        for linear in [false, true] {
            for alpha in 0..=255 {
                let mut pixels: Vec<_> = (0..=alpha)
                    .map(|v| RGBA8::new(v, alpha - v, v / 2, alpha))
                    .collect();
                convert(&mut pixels, linear);
                assert!(pixels
                    .iter()
                    .all(|p| p.a == alpha && p.r <= alpha && p.g <= alpha && p.b <= alpha));
                assert!(pixels.windows(2).all(|pair| pair[0].r <= pair[1].r));
            }
        }
    }
}
