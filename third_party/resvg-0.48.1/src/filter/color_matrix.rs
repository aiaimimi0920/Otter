// Copyright 2020 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::{f32_bound, ImageRefMut};
use rgb::RGBA8;
use usvg::filter::ColorMatrixKind as ColorMatrix;

/// Exact identity must not introduce a lossy color-space roundtrip. Do not use
/// approximate equality: even small coefficients can affect later primitives.
pub(super) fn is_identity(matrix: &ColorMatrix) -> bool {
    matches!(matrix, ColorMatrix::Matrix(values) if values.iter().enumerate().all(|(i, v)| {
        *v == if matches!(i, 0 | 6 | 12 | 18) { 1.0 } else { 0.0 }
    }))
}

/// Recognize exact RGB channel selection with unchanged alpha and zero offsets.
pub(super) fn channel_mapping(matrix: &ColorMatrix) -> Option<[usize; 3]> {
    let ColorMatrix::Matrix(values) = matrix else {
        return None;
    };
    if values.len() != 20 || values[15..] != [0.0, 0.0, 0.0, 1.0, 0.0] {
        return None;
    }
    let mut mapping = [0; 3];
    for (index, row) in values[..15].chunks_exact(5).enumerate() {
        if row[3] != 0.0 || row[4] != 0.0 {
            return None;
        }
        let selected = row[..3].iter().position(|value| *value == 1.0)?;
        if row[..3]
            .iter()
            .enumerate()
            .any(|(i, value)| i != selected && *value != 0.0)
        {
            return None;
        }
        mapping[index] = selected;
    }
    Some(mapping)
}

/// Applies a color matrix filter.
///
/// Input image pixels should have an **unpremultiplied alpha**.
pub fn apply(matrix: &ColorMatrix, src: ImageRefMut) {
    match matrix {
        ColorMatrix::Matrix(m) => {
            for pixel in src.data {
                let (r, g, b, a) = to_normalized_components(*pixel);

                let new_r = r * m[0] + g * m[1] + b * m[2] + a * m[3] + m[4];
                let new_g = r * m[5] + g * m[6] + b * m[7] + a * m[8] + m[9];
                let new_b = r * m[10] + g * m[11] + b * m[12] + a * m[13] + m[14];
                let new_a = r * m[15] + g * m[16] + b * m[17] + a * m[18] + m[19];

                pixel.r = from_normalized(new_r);
                pixel.g = from_normalized(new_g);
                pixel.b = from_normalized(new_b);
                pixel.a = from_normalized(new_a);
            }
        }
        ColorMatrix::Saturate(v) => {
            let v = v.get().max(0.0);
            let m = [
                0.213 + 0.787 * v,
                0.715 - 0.715 * v,
                0.072 - 0.072 * v,
                0.213 - 0.213 * v,
                0.715 + 0.285 * v,
                0.072 - 0.072 * v,
                0.213 - 0.213 * v,
                0.715 - 0.715 * v,
                0.072 + 0.928 * v,
            ];

            for pixel in src.data {
                let (r, g, b, _) = to_normalized_components(*pixel);

                let new_r = r * m[0] + g * m[1] + b * m[2];
                let new_g = r * m[3] + g * m[4] + b * m[5];
                let new_b = r * m[6] + g * m[7] + b * m[8];

                pixel.r = from_normalized(new_r);
                pixel.g = from_normalized(new_g);
                pixel.b = from_normalized(new_b);
            }
        }
        ColorMatrix::HueRotate(angle) => {
            let angle = angle.to_radians();
            let a1 = angle.cos();
            let a2 = angle.sin();
            let m = [
                0.213 + 0.787 * a1 - 0.213 * a2,
                0.715 - 0.715 * a1 - 0.715 * a2,
                0.072 - 0.072 * a1 + 0.928 * a2,
                0.213 - 0.213 * a1 + 0.143 * a2,
                0.715 + 0.285 * a1 + 0.140 * a2,
                0.072 - 0.072 * a1 - 0.283 * a2,
                0.213 - 0.213 * a1 - 0.787 * a2,
                0.715 - 0.715 * a1 + 0.715 * a2,
                0.072 + 0.928 * a1 + 0.072 * a2,
            ];

            for pixel in src.data {
                let (r, g, b, _) = to_normalized_components(*pixel);

                let new_r = r * m[0] + g * m[1] + b * m[2];
                let new_g = r * m[3] + g * m[4] + b * m[5];
                let new_b = r * m[6] + g * m[7] + b * m[8];

                pixel.r = from_normalized(new_r);
                pixel.g = from_normalized(new_g);
                pixel.b = from_normalized(new_b);
            }
        }
        ColorMatrix::LuminanceToAlpha => {
            for pixel in src.data {
                let (r, g, b, _) = to_normalized_components(*pixel);

                let new_a = r * 0.2125 + g * 0.7154 + b * 0.0721;

                pixel.r = 0;
                pixel.g = 0;
                pixel.b = 0;
                pixel.a = from_normalized(new_a);
            }
        }
    }
}

#[inline]
fn to_normalized_components(pixel: RGBA8) -> (f32, f32, f32, f32) {
    (
        pixel.r as f32 / 255.0,
        pixel.g as f32 / 255.0,
        pixel.b as f32 / 255.0,
        pixel.a as f32 / 255.0,
    )
}

#[inline]
fn from_normalized(c: f32) -> u8 {
    (f32_bound(0.0, c, 1.0) * 255.0) as u8
}

#[cfg(test)]
mod identity_tests {
    use super::*;

    #[test]
    fn mapping_requires_exact_selection_and_unchanged_alpha() {
        for r in 0..3 {
            for g in 0..3 {
                for b in 0..3 {
                    let mut values = vec![0.0; 20];
                    values[r] = 1.0;
                    values[5 + g] = 1.0;
                    values[10 + b] = 1.0;
                    values[18] = 1.0;
                    assert_eq!(
                        channel_mapping(&ColorMatrix::Matrix(values.clone())),
                        Some([r, g, b])
                    );
                    for index in 0..20 {
                        let mut changed = values.clone();
                        changed[index] += f32::EPSILON;
                        assert_eq!(channel_mapping(&ColorMatrix::Matrix(changed)), None);
                    }
                }
            }
        }
    }

    #[test]
    fn only_exact_matrix_is_identity() {
        let mut values = [0.0; 20];
        for index in [0, 6, 12, 18] {
            values[index] = 1.0;
        }
        assert!(is_identity(&ColorMatrix::Matrix(values.to_vec())));
        for index in 0..20 {
            let mut changed = values;
            changed[index] += f32::EPSILON;
            assert!(!is_identity(&ColorMatrix::Matrix(changed.to_vec())));
        }
        assert!(!is_identity(&ColorMatrix::LuminanceToAlpha));
    }
}
