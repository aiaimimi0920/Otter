// Copyright 2020 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::ImageRefMut;
use usvg::filter::{ComponentTransfer, TransferFunction};

/// Whole-image identity can skip component processing, but not the working color space.
pub(super) fn is_identity(fe: &ComponentTransfer) -> bool {
    [fe.func_r(), fe.func_g(), fe.func_b(), fe.func_a()]
        .into_iter()
        .all(identity_function)
}

fn identity_function(func: &TransferFunction) -> bool {
    match func {
        TransferFunction::Identity => true,
        TransferFunction::Table(values) => values.is_empty() || values.as_slice() == [0.0, 1.0],
        TransferFunction::Discrete(values) => values.is_empty(),
        TransferFunction::Linear { slope, intercept } => *slope == 1.0 && *intercept == 0.0,
        TransferFunction::Gamma {
            amplitude,
            exponent,
            offset,
        } => *amplitude == 1.0 && *exponent == 1.0 && *offset == 0.0,
    }
}

/// Applies component transfer functions for each `src` image channel.
///
/// Input image pixels should have an **unpremultiplied alpha**.
pub fn apply(fe: &ComponentTransfer, src: ImageRefMut) {
    // Fixed stack tables cap expensive evaluations at 256 per channel.
    // Small images avoid setup that would exceed their pixel count.
    if src.data.len() < 256 {
        apply_direct(fe, src);
        return;
    }
    let tables = [fe.func_r(), fe.func_g(), fe.func_b(), fe.func_a()].map(lookup);
    for pixel in src.data {
        for (channel, table) in [&mut pixel.r, &mut pixel.g, &mut pixel.b, &mut pixel.a]
            .into_iter()
            .zip(&tables)
        {
            if let Some(table) = table {
                *channel = table[usize::from(*channel)];
            }
        }
    }
}

pub(super) fn lookup(func: &TransferFunction) -> Option<[u8; 256]> {
    if is_dummy(func) {
        None
    } else {
        Some(std::array::from_fn(|i| transfer(func, i as u8)))
    }
}

fn apply_direct(fe: &ComponentTransfer, src: ImageRefMut) {
    for pixel in src.data {
        if !is_dummy(fe.func_r()) {
            pixel.r = transfer(fe.func_r(), pixel.r);
        }

        if !is_dummy(fe.func_b()) {
            pixel.b = transfer(fe.func_b(), pixel.b);
        }

        if !is_dummy(fe.func_g()) {
            pixel.g = transfer(fe.func_g(), pixel.g);
        }

        if !is_dummy(fe.func_a()) {
            pixel.a = transfer(fe.func_a(), pixel.a);
        }
    }
}

fn is_dummy(func: &TransferFunction) -> bool {
    match func {
        TransferFunction::Identity => true,
        TransferFunction::Table(values) => values.is_empty(),
        TransferFunction::Discrete(values) => values.is_empty(),
        TransferFunction::Linear { .. } => false,
        TransferFunction::Gamma { .. } => false,
    }
}

fn transfer(func: &TransferFunction, c: u8) -> u8 {
    // Build byte-domain tables before color conversion or premultiplication.
    // Preserve float coefficient arithmetic for linear/discrete functions;
    // interpolation and powers need double precision at quantization boundaries.
    let byte = c;
    let c = f64::from(c) / 255.0;
    let c = match func {
        TransferFunction::Identity => return byte,
        TransferFunction::Table(values) => {
            let Some(n) = values.len().checked_sub(1) else {
                return byte;
            };
            let position = c * n as f64;
            let k = (position as usize).min(n);
            let start = f64::from(values[k]);
            let end = f64::from(values[(k + 1).min(n)]);
            255.0 * (start + (position - k as f64) * (end - start))
        }
        TransferFunction::Discrete(values) => {
            let Some(last) = values.len().checked_sub(1) else {
                return byte;
            };
            let k = (c * values.len() as f64) as usize;
            f64::from(255.0 * values[k.min(last)])
        }
        TransferFunction::Linear { slope, intercept } => {
            f64::from(slope * f32::from(byte) + 255.0 * intercept)
        }
        TransferFunction::Gamma {
            amplitude,
            exponent,
            offset,
        } => 255.0 * (f64::from(*amplitude) * c.powf(f64::from(*exponent)) + f64::from(*offset)),
    };

    c.clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod identity_tests {
    use super::{identity_function, TransferFunction as F};

    #[test]
    fn exact_identity_and_empty_tables_only() {
        for f in [
            F::Identity,
            F::Table(vec![]),
            F::Discrete(vec![]),
            F::Table(vec![0.0, 1.0]),
            F::Linear {
                slope: 1.0,
                intercept: 0.0,
            },
            F::Gamma {
                amplitude: 1.0,
                exponent: 1.0,
                offset: 0.0,
            },
        ] {
            assert!(identity_function(&f));
        }
        for f in [
            F::Discrete(vec![0.0, 1.0]),
            F::Table(vec![1.0]),
            F::Table(vec![0.0, 1.0 - f32::EPSILON]),
            F::Linear {
                slope: 1.0 + f32::EPSILON,
                intercept: 0.0,
            },
            F::Linear {
                slope: 1.0,
                intercept: f32::from_bits(1),
            },
            F::Gamma {
                amplitude: 1.0 + f32::EPSILON,
                exponent: 1.0,
                offset: 0.0,
            },
            F::Gamma {
                amplitude: 1.0,
                exponent: 1.0 + f32::EPSILON,
                offset: 0.0,
            },
            F::Gamma {
                amplitude: 1.0,
                exponent: 1.0,
                offset: f32::from_bits(1),
            },
            F::Gamma {
                amplitude: f32::NAN,
                exponent: 1.0,
                offset: 0.0,
            },
        ] {
            assert!(!identity_function(&f));
        }
    }
}

#[cfg(test)]
mod lookup_tests {
    use super::{lookup, transfer, TransferFunction as F};

    #[test]
    fn lookup_matches_scalar_for_every_byte() {
        for func in [
            F::Table(vec![0.0, 0.8, 0.2, 1.0]),
            F::Table(vec![0.3]),
            F::Discrete(vec![0.0, 0.8, 0.2, 1.0]),
            F::Discrete(vec![0.3]),
            F::Linear {
                slope: 1.3,
                intercept: -0.1,
            },
            F::Linear {
                slope: -2.0,
                intercept: 1.2,
            },
            F::Gamma {
                amplitude: 0.8,
                exponent: 1.7,
                offset: 0.05,
            },
            F::Gamma {
                amplitude: 1.0,
                exponent: -1.0,
                offset: -0.2,
            },
            F::Gamma {
                amplitude: 1.0,
                exponent: 0.0,
                offset: 0.0,
            },
        ] {
            let table = lookup(&func).unwrap();
            for byte in 0..=255u8 {
                assert_eq!(table[usize::from(byte)], transfer(&func, byte));
            }
        }
        for func in [F::Identity, F::Table(vec![]), F::Discrete(vec![])] {
            assert!(lookup(&func).is_none());
        }
    }
}
