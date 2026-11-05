//! 不读取源图的遮挡网格；绝对网格哈希与主干 stickerEffects.ts 保持一致。
//! 只分配目标区域，整数像素边界向外覆盖，避免小数坐标造成透明接缝。
use crate::{
    pixels::{self, Pixels},
    viewport::{checked_image_bytes, Point},
};

const PALETTE: [[u8; 3]; 8] = [
    [108, 122, 142],
    [116, 130, 149],
    [124, 138, 156],
    [132, 145, 163],
    [140, 153, 170],
    [148, 160, 176],
    [120, 134, 152],
    [112, 126, 145],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MosaicError {
    InvalidInput,
    BudgetExceeded,
}

fn color(column: i32, row: i32) -> [u8; 4] {
    // wrapping_mul 和无符号右移对应 JavaScript Math.imul / >>>。
    let mut h = (column as u32 ^ 0x9e3779b9).wrapping_mul(0x85ebca6b);
    h = (h ^ (h >> 13) ^ row as u32).wrapping_mul(0xc2b2ae35);
    h ^= h >> 16;
    h = (h
        ^ (row as u32)
            .wrapping_add(0x165667b1)
            .wrapping_mul(0x27d4eb2f))
    .wrapping_mul(0x9e3779b1);
    h ^= h >> 15;
    let rgb = PALETTE[h as usize % PALETTE.len()];
    [rgb[2], rgb[1], rgb[0], 255]
}

/// 返回不透明 PBGRA；接口故意不接收原图，防止未来误引入内容采样。
pub fn generate(width: u32, height: u32, cell: f32, origin: Point) -> Result<Pixels, MosaicError> {
    let count = checked_image_bytes(width, height).ok_or(MosaicError::InvalidInput)?;
    if !cell.is_finite()
        || cell.abs() > 1_000_000.0
        || !origin.x.is_finite()
        || !origin.y.is_finite()
        || origin.x.abs() > 1_000_000.0
        || origin.y.abs() > 1_000_000.0
    {
        return Err(MosaicError::InvalidInput);
    }
    // Math.round 的负半数规则不影响结果，因为最小格宽为 2。
    let size = f64::from(cell.round().max(2.0));
    let x = f64::from(origin.x);
    let y = f64::from(origin.y);
    let mut lease = pixels::reserve(count).ok_or(MosaicError::BudgetExceeded)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(count)
        .map_err(|_| MosaicError::BudgetExceeded)?;
    if !lease.grow_to(bytes.capacity()) {
        return Err(MosaicError::BudgetExceeded);
    }
    bytes.resize(count, 0);
    let stride = width as usize * 4;
    // 按主干的行优先顺序覆盖重叠边界，网格身份不随裁剪区域原点改变。
    for row in (y / size).floor() as i32..((y + f64::from(height)) / size).ceil() as i32 {
        let top = (f64::from(row) * size - y).floor().max(0.0) as usize;
        let bottom = ((f64::from(row) + 1.0) * size - y)
            .ceil()
            .min(f64::from(height)) as usize;
        for column in (x / size).floor() as i32..((x + f64::from(width)) / size).ceil() as i32 {
            let left = (f64::from(column) * size - x).floor().max(0.0) as usize;
            let right = ((f64::from(column) + 1.0) * size - x)
                .ceil()
                .min(f64::from(width)) as usize;
            let pixel = color(column, row);
            for line in top..bottom {
                for target in
                    bytes[line * stride + left * 4..line * stride + right * 4].chunks_exact_mut(4)
                {
                    target.copy_from_slice(&pixel);
                }
            }
        }
    }
    Ok(Pixels::from_reserved(bytes, lease))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hash_matches_javascript_imul_fixtures_including_negative_cells() {
        for (column, row, index) in [
            (0, 0, 4),
            (1, 0, 4),
            (0, 1, 6),
            (-1, -1, 0),
            (-5, 7, 2),
            (12345, -9876, 2),
            (500000, 500000, 0),
        ] {
            let rgb = PALETTE[index];
            assert_eq!(color(column, row), [rgb[2], rgb[1], rgb[0], 255]);
        }
    }
    #[test]
    fn cropped_grid_keeps_absolute_identity_and_has_no_transparent_seams() {
        let whole = generate(24, 20, 3.0, Point::default()).unwrap();
        let crop = generate(11, 9, 3.0, Point { x: 5.0, y: 7.0 }).unwrap();
        for row in 0..9 {
            assert_eq!(
                &crop[row * 44..(row + 1) * 44],
                &whole[(row + 7) * 96 + 20..(row + 7) * 96 + 64]
            );
        }
        for origin in [Point { x: -5.25, y: -3.5 }, Point { x: 1.25, y: 2.75 }] {
            let pixels = generate(17, 15, 2.0, origin).unwrap();
            assert!(pixels.chunks_exact(4).all(|p| p[3] == 255));
        }
    }
    #[test]
    fn fractional_overlap_uses_later_cell_like_canvas_fill_rect() {
        let pixels = generate(4, 4, 2.0, Point { x: 0.5, y: 0.5 }).unwrap();
        for y in 0..4 {
            for x in 0..4 {
                assert_eq!(
                    &pixels[(y * 4 + x) * 4..(y * 4 + x + 1) * 4],
                    &color(x.div_ceil(2) as i32, y.div_ceil(2) as i32)
                );
            }
        }
    }
    #[test]
    fn invalid_inputs_are_rejected_before_allocation() {
        for (w, h, cell, origin) in [
            (0, 2, 12.0, Point::default()),
            (u32::MAX, 2, 12.0, Point::default()),
            (2, 2, f32::NAN, Point::default()),
            (
                2,
                2,
                12.0,
                Point {
                    x: f32::INFINITY,
                    y: 0.0,
                },
            ),
        ] {
            assert_eq!(generate(w, h, cell, origin), Err(MosaicError::InvalidInput));
        }
        assert_eq!(
            generate(4, 4, 1.0, Point::default()).unwrap(),
            generate(4, 4, 2.0, Point::default()).unwrap()
        );
    }
}
