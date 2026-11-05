//! RLE4/8 直接写入已预算的 PBGRA 输出，不分配中间索引图；每条指令可取消。
use super::{invalid, Layout};
use windows::core::Result;

pub(super) fn decode(
    bytes: &[u8],
    layout: &Layout,
    output: &mut [u8],
    check: &impl Fn() -> Result<()>,
) -> Result<()> {
    let palette = &bytes[layout.palette.clone()];
    let background = palette.get(..3).ok_or_else(invalid)?;
    // Delta/提前 EOB 未覆盖区域确定为调色板零色，不暴露未初始化数据。
    for row in output.chunks_exact_mut(layout.width as usize * 4) {
        check()?;
        for pixel in row.chunks_exact_mut(4) {
            pixel[..3].copy_from_slice(background);
            pixel[3] = 255;
        }
    }
    let input = &bytes[layout.offset..];
    let (mut cursor, mut x, mut y) = (0usize, 0usize, 0usize);
    let width = layout.width as usize;
    let height = layout.height as usize;
    loop {
        check()?;
        let pair = input.get(cursor..cursor + 2).ok_or_else(invalid)?;
        cursor += 2;
        let (count, value) = (pair[0] as usize, pair[1] as usize);
        if count == 0 {
            match value {
                0 => {
                    if y >= height {
                        return Err(invalid());
                    }
                    x = 0;
                    y += 1;
                    continue;
                }
                1 => return Ok(()),
                2 => {
                    let delta = input.get(cursor..cursor + 2).ok_or_else(invalid)?;
                    cursor += 2;
                    x += delta[0] as usize;
                    y += delta[1] as usize;
                    if x > width || y >= height {
                        return Err(invalid());
                    }
                    continue;
                }
                _ => {}
            }
        }
        let length = if count == 0 { value } else { count };
        if y >= height || x + length > width {
            return Err(invalid());
        }
        let literal_bytes = if layout.bits == 8 {
            length
        } else {
            length.div_ceil(2)
        };
        let literal = if count == 0 {
            let padded = literal_bytes.div_ceil(2) * 2;
            let literal = input.get(cursor..cursor + padded).ok_or_else(invalid)?;
            cursor += padded;
            literal
        } else {
            &[]
        };
        for index in 0..length {
            let packed = if count != 0 {
                value as u8
            } else if layout.bits == 8 {
                literal[index]
            } else {
                literal[index / 2]
            };
            let color = if layout.bits == 8 {
                packed as usize
            } else if index % 2 == 0 {
                (packed >> 4) as usize
            } else {
                (packed & 15) as usize
            };
            let color = palette.get(color * 4..color * 4 + 3).ok_or_else(invalid)?;
            let offset = ((height - 1 - y) * width + x + index) * 4;
            output[offset..offset + 3].copy_from_slice(color);
        }
        x += length;
    }
}
