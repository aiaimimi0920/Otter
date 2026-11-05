// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Keep straight intermediate matrix values in floating point until final output.
use super::ImageRefMut;
use rgb::RGBA8;

pub(super) fn apply(matrix: &[f32], image: ImageRefMut, transfer: Option<bool>) {
    for pixel in image.data {
        let a = f64::from(pixel.a) / 255.0;
        let mut input = if pixel.a == 0 {
            [0.0; 4]
        } else {
            [
                f64::from(pixel.r) / f64::from(pixel.a),
                f64::from(pixel.g) / f64::from(pixel.a),
                f64::from(pixel.b) / f64::from(pixel.a),
                a,
            ]
        };
        // Keep both color transforms in the same pixel operation. The resulting
        // bytes retain the source color space, not the matrix's working space.
        if let Some(linear) = transfer {
            for value in &mut input[..3] {
                *value = super::color_transfer::channel(*value, linear);
            }
        }
        let mut output = [0.0; 4];
        for (channel, row) in matrix.chunks_exact(5).enumerate() {
            output[channel] = (input
                .iter()
                .zip(row)
                .map(|(v, m)| v * f64::from(*m))
                .sum::<f64>()
                + f64::from(row[4]))
            .clamp(0.0, 1.0);
        }
        if let Some(linear) = transfer {
            for value in &mut output[..3] {
                *value = super::color_transfer::channel(*value, !linear);
            }
        }
        let alpha = output[3] * 255.0;
        *pixel = RGBA8::new(
            (output[0] * alpha) as u8,
            (output[1] * alpha) as u8,
            (output[2] * alpha) as u8,
            alpha as u8,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gain_quantizes_once_and_transparent_offsets_are_clamped() {
        let mut matrix = [0.0; 20];
        for index in [0, 6, 12] {
            matrix[index] = 0.5;
        }
        matrix[18] = 1.0;
        let mut pixels = [RGBA8::new(1, 3, 8, 8)];
        apply(&matrix, ImageRefMut::new(1, 1, &mut pixels), None);
        assert_eq!(pixels[0], RGBA8::new(0, 1, 4, 8));
        matrix[4] = 2.0;
        matrix[9] = -1.0;
        matrix[19] = 0.5;
        pixels[0] = RGBA8::default();
        apply(&matrix, ImageRefMut::new(1, 1, &mut pixels), None);
        assert_eq!(pixels[0], RGBA8::new(127, 0, 0, 127));
    }
}
