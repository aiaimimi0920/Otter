//! Chromium mft2 RGB→XYZ 兼容求值；不把曲线/目标 gamma 预融合进近似网格。
use moxcms::{ColorProfile, DataColorSpace, LutStore, LutType, LutWarehouse};
use std::io;

pub(super) fn apply(source: &ColorProfile, pixels: &mut [u8], channels: usize) -> io::Result<bool> {
    // 与调用方显式 Perceptual intent 一致，只处理实际选中的 A2B0 / mft2。
    let Some(LutWarehouse::Lut(lut)) = &source.lut_a_to_b_perceptual else {
        return Ok(false);
    };
    if source.color_space != DataColorSpace::Rgb
        || source.pcs != DataColorSpace::Xyz
        || lut.lut_type != LutType::Lut16
    {
        return Ok(false);
    }
    let invalid = || io::Error::new(io::ErrorKind::InvalidData, "Invalid RGB mft2 tables");
    let (LutStore::Store16(input), LutStore::Store16(grid), LutStore::Store16(output)) =
        (&lut.input_table, &lut.clut_table, &lut.output_table)
    else {
        return Err(invalid());
    };
    let size = usize::from(lut.num_clut_grid_points);
    let inputs = usize::from(lut.num_input_table_entries);
    let outputs = usize::from(lut.num_output_table_entries);
    // u8 grid size 的三次方在 Windows usize 内安全；不信任 parser 对表长的保证。
    if lut.num_input_channels != 3
        || lut.num_output_channels != 3
        || size < 2
        || !(2..=4096).contains(&inputs)
        || !(2..=4096).contains(&outputs)
        || input.len() != inputs * 3
        || output.len() != outputs * 3
        || grid.len() != size * size * size * 3
        || !matches!(channels, 3 | 4)
        || !pixels.len().is_multiple_of(channels)
    {
        return Err(invalid());
    }
    let inverse = ColorProfile::new_srgb().rgb_to_xyz_matrix().inverse();
    for pixel in pixels.chunks_exact_mut(channels) {
        let rgb = std::array::from_fn(|i| {
            curve(
                &input[i * inputs..(i + 1) * inputs],
                f64::from(pixel[i]) / 255.0,
            )
        });
        let xyz = interpolate(grid, size, rgb);
        let xyz: [f64; 3] =
            std::array::from_fn(|i| curve(&output[i * outputs..(i + 1) * outputs], xyz[i]));
        // 当前 Chromium mft2 以归一化表值进入 XYZ 矩阵；不额外乘 65535/32768。
        // RGB 输入的 mft2 matrix 按该基线忽略；这不是通用 ICC/PCSXYZ 实现。
        for (channel, row) in inverse.v.iter().enumerate() {
            let linear = row.iter().zip(xyz).map(|(a, b)| a * b).sum::<f64>();
            let value = if linear <= 0.0031308 {
                linear * 12.92
            } else {
                1.055 * linear.powf(1.0 / 2.4) - 0.055
            };
            pixel[channel] = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
        // 仅写 RGB，alpha 保持原字节；无像素级分配/额外全图，仍在 Job/deadline 内。
    }
    Ok(true)
}

fn curve(table: &[u16], value: f64) -> f64 {
    let position = value.clamp(0.0, 1.0) * (table.len() - 1) as f64;
    let low = position as usize;
    let high = (low + 1).min(table.len() - 1);
    let fraction = position - low as f64;
    (f64::from(table[low]) * (1.0 - fraction) + f64::from(table[high]) * fraction) / 65535.0
}

fn interpolate(table: &[u16], size: usize, rgb: [f64; 3]) -> [f64; 3] {
    let positions = rgb.map(|v| v.clamp(0.0, 1.0) * (size - 1) as f64);
    let low = positions.map(|v| v as usize);
    let high = low.map(|v| (v + 1).min(size - 1));
    let fraction: [f64; 3] = std::array::from_fn(|i| positions[i] - low[i] as f64);
    let mut result = [0.0; 3];
    // 三线性插值的八顶点；r 最慢、b 最快。所有索引受上方完整表长校验保护。
    for r in 0..2 {
        for g in 0..2 {
            for b in 0..2 {
                let choices = [r, g, b];
                let mut index = 0;
                let mut weight = 1.0;
                for axis in 0..3 {
                    let upper = choices[axis] != 0;
                    index = index * size + if upper { high[axis] } else { low[axis] };
                    weight *= if upper {
                        fraction[axis]
                    } else {
                        1.0 - fraction[axis]
                    };
                }
                for channel in 0..3 {
                    result[channel] += weight * f64::from(table[index * 3 + channel]) / 65535.0;
                }
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> ColorProfile {
        let mut source = ColorProfile::new_srgb();
        source.lut_a_to_b_perceptual = Some(LutWarehouse::Lut(moxcms::LutDataType {
            num_input_channels: 3,
            num_output_channels: 3,
            num_clut_grid_points: 2,
            matrix: source.rgb_to_xyz_matrix(),
            num_input_table_entries: 2,
            num_output_table_entries: 2,
            input_table: LutStore::Store16(vec![0, 65535, 0, 65535, 0, 65535]),
            output_table: LutStore::Store16(vec![0, 65535, 0, 65535, 0, 65535]),
            clut_table: LutStore::Store16(vec![0; 24]),
            lut_type: LutType::Lut16,
        }));
        source
    }

    #[test]
    fn surface_webp_icc_lut_validates_tables_and_preserves_alpha() {
        let mut profile = source();
        let mut pixels: Vec<u8> = (0..=255).flat_map(|a| [100, 150, 200, a]).collect();
        assert!(apply(&profile, &mut pixels, 4).unwrap());
        for (alpha, pixel) in pixels.chunks_exact(4).enumerate() {
            assert_eq!(pixel, [0, 0, 0, alpha as u8]);
        }
        assert!(apply(&profile, &mut [1, 2], 3).is_err());
        assert!(apply(&profile, &mut [], 0).is_err());
        let Some(LutWarehouse::Lut(lut)) = &mut profile.lut_a_to_b_perceptual else {
            unreachable!()
        };
        lut.clut_table = LutStore::Store16(vec![0; 23]);
        assert!(apply(&profile, &mut [1, 2, 3], 3).is_err());
        assert!(!apply(&ColorProfile::new_srgb(), &mut [1, 2, 3], 3).unwrap());
        profile.pcs = DataColorSpace::Lab;
        assert!(!apply(&profile, &mut [1, 2, 3], 3).unwrap());
    }

    #[test]
    fn surface_webp_icc_lut_interpolates_before_nonlinear_curve() {
        let mut grid = vec![0; 24];
        grid[21..].fill(65535);
        assert_eq!(interpolate(&grid, 2, [0.5; 3]), [0.125; 3]);
        assert_eq!(interpolate(&grid, 2, [1.0; 3]), [1.0; 3]);
        assert_eq!(interpolate(&grid, 2, [-1.0, 1.0, 1.0]), [0.0; 3]);
        assert!((curve(&[0, 16384, 65535], 0.75) - 0.625).abs() < 0.00001);
    }
}
