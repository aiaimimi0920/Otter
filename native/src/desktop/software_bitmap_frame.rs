//! 临时交互帧：内容与独立透明度外框一次写入预乘 DIB，不覆盖源图。
use super::*;

impl Bitmap {
    /// 与完整交互帧使用相同的预乘白框；透明中心不覆盖宿主内容。
    pub fn border(size: D2D_SIZE_U, padding: u32, alpha: u8) -> Result<Self> {
        if padding > 16 || size.width <= 2 * padding || size.height <= 2 * padding {
            return Err(Error::new(E_INVALIDARG, "Invalid border geometry"));
        }
        byte_count(D2D_SIZE_U {
            width: size.width - 2 * padding,
            height: size.height - 2 * padding,
        })?;
        let result = Self::allocate(size, size.width as usize * size.height as usize * 4)?;
        unsafe {
            let output = std::slice::from_raw_parts_mut(
                result._bits.as_ptr().cast::<[u8; 4]>(),
                (size.width * size.height) as usize,
            );
            for (y, row) in output.chunks_exact_mut(size.width as usize).enumerate() {
                if y < padding as usize || y >= (size.height - padding) as usize {
                    row.fill([alpha; 4]);
                } else {
                    row[..padding as usize].fill([alpha; 4]);
                    row[(size.width - padding) as usize..].fill([alpha; 4]);
                }
            }
        }
        Ok(result)
    }
    pub fn framed(&self, padding: u32, opacity: f32, border: bool) -> Result<Self> {
        if padding > 16 || !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
            return Err(Error::new(E_INVALIDARG, "Invalid zoom frame decoration"));
        }
        // 内容仍遵守原 8192/16M 预算；只为至多 16px 的外圈增加有界空间。
        byte_count(self.size)?;
        let size = D2D_SIZE_U {
            width: self.size.width + 2 * padding,
            height: self.size.height + 2 * padding,
        };
        let bytes = size.width as usize * size.height as usize * 4;
        let result = Self::allocate(size, bytes)?;
        unsafe {
            GdiFlush().ok()?;
            let source = std::slice::from_raw_parts(
                self._bits.as_ptr().cast::<[u8; 4]>(),
                (self.size.width * self.size.height) as usize,
            );
            let output = std::slice::from_raw_parts_mut(
                result._bits.as_ptr().cast::<[u8; 4]>(),
                (size.width * size.height) as usize,
            );
            let alpha = (opacity * 255.0).round() as u32;
            let edge = if border {
                [crate::desktop::pin_border_style::alpha(opacity); 4]
            } else {
                [0; 4]
            };
            for (y, row) in output.chunks_exact_mut(size.width as usize).enumerate() {
                if y < padding as usize || y >= (size.height - padding) as usize {
                    row.fill(edge);
                    continue;
                }
                let start = (y - padding as usize) * self.size.width as usize;
                let source = &source[start..start + self.size.width as usize];
                row[..padding as usize].fill(edge);
                row[(size.width - padding) as usize..].fill(edge);
                let content = &mut row[padding as usize..(size.width - padding) as usize];
                if alpha == 255 {
                    content.copy_from_slice(source);
                } else {
                    for (pixel, src) in content.iter_mut().zip(source) {
                        *pixel = src.map(|c| ((u32::from(c) * alpha + 127) / 255) as u8);
                    }
                }
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stationary_border_matches_preview_edges_byte_for_byte() {
        for padding in [1, 2, 3, 8, 16] {
            for opacity in [0.0, 0.3, 0.5, 1.0] {
                let source = Bitmap::new(D2D_SIZE_U {
                    width: 7,
                    height: 5,
                })
                .unwrap();
                let preview = source.framed(padding, opacity, true).unwrap();
                let border = Bitmap::border(
                    preview.size,
                    padding,
                    crate::desktop::pin_border_style::alpha(opacity),
                )
                .unwrap();
                assert_eq!(preview.pixels(), border.pixels());
            }
        }
    }
    #[test]
    fn border_tracks_half_opacity_floor_and_content_is_premultiplied_once() {
        let source = Bitmap::new(D2D_SIZE_U {
            width: 2,
            height: 1,
        })
        .unwrap();
        unsafe {
            std::ptr::copy_nonoverlapping(
                [20u8, 40, 60, 128, 0, 0, 0, 0].as_ptr(),
                source._bits.as_ptr(),
                8,
            );
        }
        let framed = source.framed(2, 0.5, true).unwrap();
        let pixels = framed.pixels();
        assert_eq!(pixels[2 * 6 + 2], [10, 20, 30, 64]);
        assert_eq!(pixels[2 * 6 + 3], [0; 4]);
        for (i, pixel) in pixels.iter().enumerate() {
            if i != 14 && i != 15 {
                assert_eq!(*pixel, [191; 4]);
            }
        }
        assert_eq!(source.pixels()[0], [20, 40, 60, 128]);
        assert_eq!(source.framed(2, 1.0, false).unwrap().pixels()[0], [0; 4]);
        assert!(source.framed(17, 1.0, true).is_err());
    }
}
