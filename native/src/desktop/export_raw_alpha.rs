//! 原始保存的可逆反预乘：直接填充已有预算内的WIC副本，不新增整图中间缓存。
use super::*;

pub(super) fn bitmap(
    wic: &IWICImagingFactory,
    width: u32,
    height: u32,
    pixels: &[u8],
    cancel: &AtomicBool,
) -> Result<IWICBitmap> {
    // bitmap和独占锁不逃逸此线程；锁释放前所有返回路径均保持COM owner和原像素租约。
    let bitmap = unsafe {
        wic.CreateBitmap(
            width,
            height,
            &GUID_WICPixelFormat32bppBGRA,
            WICBitmapCacheOnLoad,
        )?
    };
    {
        let lock = unsafe { bitmap.Lock(std::ptr::null(), WICBitmapLockWrite.0 as u32)? };
        let stride = unsafe { lock.GetStride()? } as usize;
        let (mut capacity, mut pointer) = (0, std::ptr::null_mut());
        unsafe { lock.GetDataPointer(&mut capacity, &mut pointer)? };
        let row = width as usize * 4;
        let required = stride
            .checked_mul(height.saturating_sub(1) as usize)
            .and_then(|value| value.checked_add(row));
        if pointer.is_null()
            || stride < row
            || required.is_none_or(|bytes| bytes > capacity as usize)
        {
            return Err(Error::new(E_FAIL, "Raw PNG bitmap storage changed"));
        }
        // GetDataPointer的范围已逐行核验，独占WICBitmapLock约束此可变slice的生命期。
        let output = unsafe { std::slice::from_raw_parts_mut(pointer, capacity as usize) };
        for (source, target) in pixels.chunks_exact(row).zip(output.chunks_mut(stride)) {
            check(cancel)?;
            for (source, target) in source
                .chunks_exact(4)
                .zip(target[..row].chunks_exact_mut(4))
            {
                let alpha = u32::from(source[3]);
                for index in 0..3 {
                    target[index] = (u32::from(source[index]) * 255 + alpha / 2)
                        .checked_div(alpha)
                        .unwrap_or(0)
                        .min(255) as u8;
                }
                target[3] = source[3];
            }
        }
    }
    check(cancel)?;
    Ok(bitmap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_preserves_ordinary_product_icon_straight_pixels_across_repeated_restores() {
        let original = include_bytes!("../../../src-tauri/icons/32x32.png");
        let decode = |png: &[u8]| {
            let mut reader = ::png::Decoder::new(std::io::Cursor::new(png))
                .read_info()
                .unwrap();
            let mut data = vec![0; reader.output_buffer_size().unwrap()];
            let info = reader.next_frame(&mut data).unwrap();
            assert_eq!(info.color_type, ::png::ColorType::Rgba);
            data.truncate(info.buffer_size());
            (info.width, info.height, data)
        };
        let expected = decode(original);
        let cancel = AtomicBool::new(false);
        let mut url = crate::loom_base64::encode_png_data_url(&original[..], &cancel).unwrap();
        for _ in 0..4 {
            let image = crate::desktop::session_image::inline(url.as_str(), &cancel).unwrap();
            let png = png_checkpoint(&image, &cancel).unwrap();
            assert_eq!(decode(&png), expected);
            url = crate::loom_base64::encode_png_data_url(&png, &cancel).unwrap();
        }
    }

    #[test]
    fn all_valid_premultiplied_channels_round_trip_exactly_without_save_drift() {
        let _apartment = Apartment::new().unwrap();
        let source: Vec<u8> = (0..=255u8)
            .flat_map(|a| {
                (0..=255u8).flat_map(move |c| {
                    let c = c.min(a);
                    [c, a - c, c / 2, a]
                })
            })
            .collect();
        let mut image = Image {
            width: 256,
            height: 256,
            pixels: source.clone().into(),
        };
        let cancel = AtomicBool::new(false);
        for _ in 0..4 {
            let before = image.pixels.clone();
            let png = png_checkpoint(&image, &cancel).unwrap();
            assert!(image.pixels.same_storage(&before));
            assert!(image.pixels.as_ref() == source);
            let mut reader = ::png::Decoder::new(std::io::Cursor::new(png.as_ref()))
                .read_info()
                .unwrap();
            let mut rgba = vec![0; reader.output_buffer_size().unwrap()];
            let info = reader.next_frame(&mut rgba).unwrap();
            assert_eq!(
                (info.width, info.height, info.color_type),
                (256, 256, ::png::ColorType::Rgba)
            );
            // 独立数值oracle检查straight PNG；不是用被测转换函数生成期望。
            for (bgra, rgba) in source
                .chunks_exact(4)
                .zip(rgba[..info.buffer_size()].chunks_exact(4))
            {
                assert_eq!(rgba[3], bgra[3]);
                for (actual, channel) in rgba[..3].iter().zip([bgra[2], bgra[1], bgra[0]]) {
                    let expected = if bgra[3] == 0 {
                        0.0
                    } else {
                        f64::from(channel) * 255.0 / f64::from(bgra[3])
                    };
                    assert_eq!(*actual, expected.round() as u8);
                }
            }
            image = crate::desktop::session_image::inline(
                crate::loom_base64::encode_png_data_url(&png, &cancel)
                    .unwrap()
                    .as_str(),
                &cancel,
            )
            .unwrap();
            assert!(
                image.pixels.as_ref() == source,
                "repeated checkpoint changed premultiplied source pixels"
            );
        }
    }
}
