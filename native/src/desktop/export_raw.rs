//! 会话图片资源编码：仅 WIC 像素转换，不创建 D2D 设备、不合成标注/裁剪/窗口透明度。
use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
#[path = "export_raw_alpha.rs"]
mod alpha;

enum Encoding {
    WicPremultiplied,
    Straight,
    Checkpoint,
}

pub(in crate::desktop) fn png_raw(
    image: &Image,
    cancel: &AtomicBool,
) -> Result<crate::pixels::Pixels> {
    encode_pixels(
        image.width,
        image.height,
        &image.pixels,
        Encoding::WicPremultiplied,
        cancel,
    )
}

/// 持久化底图需可逆恢复；不改变普通导出既有WIC取整或Shader的straight合同。
pub(in crate::desktop) fn png_checkpoint(
    image: &Image,
    cancel: &AtomicBool,
) -> Result<crate::pixels::Pixels> {
    encode_pixels(
        image.width,
        image.height,
        &image.pixels,
        Encoding::Checkpoint,
        cancel,
    )
}

/// Shader 已完成浏览器合同反预乘；保留 straight BGRA，禁止再经 PBGRA 舍入。
pub(in crate::desktop) fn png_straight_bgra(
    width: u32,
    height: u32,
    pixels: &crate::pixels::Pixels,
    cancel: &AtomicBool,
) -> Result<crate::pixels::Pixels> {
    encode_pixels(width, height, pixels, Encoding::Straight, cancel)
}

fn encode_pixels(
    width: u32,
    height: u32,
    pixels: &crate::pixels::Pixels,
    encoding: Encoding,
    cancel: &AtomicBool,
) -> Result<crate::pixels::Pixels> {
    check(cancel)?;
    if checked_image_bytes(width, height) != Some(pixels.len()) {
        return Err(Error::new(
            windows::Win32::Foundation::E_INVALIDARG,
            "Invalid raw image dimensions",
        ));
    }
    let _apartment = Apartment::new()?;
    // WIC 从输入复制像素；单独计入共享预算，不能把这份内存当作零复制。
    let bitmap_lease = crate::pixels::reserve(pixels.len()).ok_or_else(|| {
        Error::new(
            windows::Win32::Foundation::E_OUTOFMEMORY,
            "Raw PNG bitmap budget exceeded",
        )
    })?;
    let wic: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)? };
    let bitmap = match encoding {
        Encoding::Checkpoint => alpha::bitmap(&wic, width, height, pixels, cancel)?,
        Encoding::WicPremultiplied | Encoding::Straight => {
            let format = match encoding {
                Encoding::WicPremultiplied => &GUID_WICPixelFormat32bppPBGRA,
                _ => &GUID_WICPixelFormat32bppBGRA,
            };
            unsafe { wic.CreateBitmapFromMemory(width, height, format, width * 4, pixels)? }
        }
    };
    check(cancel)?;
    let (stream, size) = encode_bitmap(&wic, &bitmap)?;
    drop(bitmap);
    drop(bitmap_lease);
    check(cancel)?;
    if size > 64 * 1024 * 1024 {
        return Err(Error::new(
            E_FAIL,
            "Raw PNG exceeds session resource budget",
        ));
    }
    let pixels = stream_pixels(stream, size)?;
    // WIC 和流复制本身不能强制打断；迟到取消也不得交付结果。
    check(cancel)?;
    Ok(pixels)
}
fn check(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::new(
            windows::Win32::Foundation::E_ABORT,
            "Raw PNG encoding cancelled",
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn raw_png_preserves_dimensions_alpha_and_pixel_order_without_compositing() {
        let _apartment = Apartment::new().unwrap();
        let source = Image {
            width: 2,
            height: 2,
            pixels: vec![0, 0, 255, 255, 0, 255, 0, 255, 64, 0, 0, 128, 0, 0, 0, 0].into(),
        };
        let before = source.pixels.clone();
        let png = png_raw(&source, &AtomicBool::new(false)).unwrap();
        let restored = super::super::tests::decode_png(png.as_ref().to_vec());
        assert_eq!((restored.width, restored.height), (2, 2));
        assert_eq!(restored.pixels.as_ref(), before.as_ref());
        assert!(source.pixels.same_storage(&before));
    }
    #[test]
    fn cancellation_and_invalid_geometry_fail_before_encoding() {
        let source = Image {
            width: 1,
            height: 1,
            pixels: vec![255; 4].into(),
        };
        let bad = Image {
            width: 2,
            ..source.clone()
        };
        for encode in [png_raw, png_checkpoint] {
            assert_eq!(
                encode(&source, &AtomicBool::new(true)).unwrap_err().code(),
                windows::Win32::Foundation::E_ABORT
            );
            assert_eq!(
                encode(&bad, &AtomicBool::new(false)).unwrap_err().code(),
                windows::Win32::Foundation::E_INVALIDARG
            );
        }
    }
}
