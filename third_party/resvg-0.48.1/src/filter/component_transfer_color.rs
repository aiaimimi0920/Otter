//! Fuse component transfer while retaining sRGB premultiplied output.
use usvg::filter::ComponentTransfer;

pub(super) fn apply(fe: &ComponentTransfer, linear: bool, pixels: &mut [u8]) {
    let tables = [fe.func_r(), fe.func_g(), fe.func_b()].map(|func| {
        super::component_transfer::lookup(func).unwrap_or_else(|| std::array::from_fn(|i| i as u8))
    });
    let alpha_table = super::component_transfer::lookup(fe.func_a())
        .unwrap_or_else(|| std::array::from_fn(|i| i as u8));
    apply_tables(tables, alpha_table, linear, pixels);
}

fn apply_tables(tables: [[u8; 256]; 3], alpha_table: [u8; 256], linear: bool, pixels: &mut [u8]) {
    let encoded = tables.map(|table| {
        table.map(|value| {
            let value = f64::from(value) / 255.0;
            if linear {
                super::color_transfer::channel(value, false)
            } else {
                value
            }
        })
    });
    // At most 64 KiB of stack indices, scoped to this call. Each observed alpha
    // builds one row, avoiding repeated powers even for alternating alpha values.
    let mut indices = [[0u8; 256]; 256];
    let mut ready = [false; 256];
    for pixel in pixels.chunks_exact_mut(4) {
        let alpha = usize::from(pixel[3]);
        let output_alpha = alpha_table[alpha];
        if output_alpha == 0 {
            pixel.fill(0);
            continue;
        }
        // Transparent input has straight RGB zero even when the alpha function creates coverage.
        if alpha != 0 && !ready[alpha] {
            for (channel, index) in indices[alpha].iter_mut().enumerate().take(alpha + 1) {
                let value = channel as f64 / alpha as f64;
                let value = if linear {
                    super::color_transfer::channel(value, true)
                } else {
                    value
                };
                *index = (value * 255.0).round().clamp(0.0, 255.0) as u8;
            }
            ready[alpha] = true;
        }
        for channel in 0..3 {
            let index = usize::from(indices[alpha][usize::from(pixel[channel]).min(alpha)]);
            pixel[channel] = (encoded[channel][index] * f64::from(output_alpha))
                .round()
                .clamp(0.0, f64::from(output_alpha)) as u8;
        }
        pixel[3] = output_alpha;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_alpha_rows_match_direct_evaluation() {
        let tables = [
            std::array::from_fn(|i| 255 - i as u8),
            std::array::from_fn(|i| i as u8),
            std::array::from_fn(|i| (i / 2) as u8),
        ];
        let mut original = Vec::new();
        for alpha in (0..=255u8).rev() {
            for value in 0..=alpha {
                original.extend_from_slice(&[value, alpha - value, value / 2, alpha]);
            }
        }
        for (linear, change_alpha) in [(false, false), (true, false), (false, true), (true, true)] {
            let alpha_table = std::array::from_fn(|i| {
                if change_alpha {
                    (i / 2 + 32) as u8
                } else {
                    i as u8
                }
            });
            let mut actual = original.clone();
            apply_tables(tables, alpha_table, linear, &mut actual);
            for (before, after) in original.chunks_exact(4).zip(actual.chunks_exact(4)) {
                let alpha = f64::from(before[3]);
                let output_alpha = alpha_table[usize::from(before[3])];
                assert_eq!(output_alpha, after[3]);
                for c in 0..3 {
                    let straight = if alpha == 0.0 {
                        0.0
                    } else {
                        f64::from(before[c]) / alpha
                    };
                    let straight = if linear {
                        super::super::color_transfer::channel(straight, true)
                    } else {
                        straight
                    };
                    let index = (straight * 255.0).round() as usize;
                    let value = f64::from(tables[c][index]) / 255.0;
                    let value = if linear {
                        super::super::color_transfer::channel(value, false)
                    } else {
                        value
                    };
                    assert_eq!(after[c], (value * f64::from(output_alpha)).round() as u8);
                    assert!(after[c] <= after[3]);
                }
            }
        }
    }
}
