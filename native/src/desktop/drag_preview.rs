//! 拖出缩略图：后台从已合成的 WIC 位图缩放，输出非预乘 BGRA；最长边 256 像素。
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, Graphics::Imaging::*},
};
pub const MAX_EDGE: u32 = 256;
pub struct Preview {
    pub width: u32,
    pub height: u32,
    pub pixels: crate::pixels::Pixels,
}
pub(super) fn dimensions(width: u32, height: u32) -> Result<(u32, u32)> {
    if width == 0 || height == 0 {
        return Err(Error::new(E_INVALIDARG, "Empty drag preview"));
    }
    let edge = width.max(height);
    if edge <= MAX_EDGE {
        return Ok((width, height));
    }
    Ok((
        ((width as u64 * MAX_EDGE as u64) / edge as u64).max(1) as u32,
        ((height as u64 * MAX_EDGE as u64) / edge as u64).max(1) as u32,
    ))
}
pub(super) fn prepare(factory: &IWICImagingFactory, source: &IWICBitmap) -> Result<Preview> {
    unsafe {
        let (mut width, mut height) = (0, 0);
        source.GetSize(&mut width, &mut height)?;
        let (width, height) = dimensions(width, height)?;
        let scaler = factory.CreateBitmapScaler()?;
        scaler.Initialize(source, width, height, WICBitmapInterpolationModeFant)?;
        // Shell helper 自行预乘，必须交付直通 alpha，避免半透明图再次变暗。
        let converted = WICConvertBitmapSource(&GUID_WICPixelFormat32bppBGRA, &scaler)?;
        let size = (width * height * 4) as usize;
        let mut lease = crate::pixels::reserve(size)
            .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Drag preview budget exhausted"))?;
        let mut bytes = vec![0; size];
        if !lease.grow_to(bytes.capacity()) {
            return Err(Error::new(
                E_OUTOFMEMORY,
                "Drag preview capacity exceeds budget",
            ));
        }
        converted.CopyPixels(std::ptr::null(), width * 4, &mut bytes)?;
        Ok(Preview {
            width,
            height,
            pixels: crate::pixels::Pixels::from_reserved(bytes, lease),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounds_preserve_aspect_without_upscaling_or_zero_thin_edges() {
        assert_eq!(dimensions(800, 400).unwrap(), (256, 128));
        assert_eq!(dimensions(400, 800).unwrap(), (128, 256));
        assert_eq!(dimensions(12, 4).unwrap(), (12, 4));
        assert_eq!(dimensions(u32::MAX, 1).unwrap(), (256, 1));
        assert!(dimensions(0, 1).is_err());
    }
    #[test]
    fn wic_preview_is_bounded_straight_alpha_and_matches_final_composition() {
        let image = crate::desktop::image::Image {
            width: 800,
            height: 400,
            pixels: vec![255; 800 * 400 * 4].into(),
        };
        let (png, preview) =
            crate::desktop::export::png_with_opacity(&image, &Default::default(), 0.5).unwrap();
        assert_eq!((preview.width, preview.height), (256, 128));
        assert_eq!(preview.pixels.len(), 256 * 128 * 4);
        assert!(preview.pixels.chunks_exact(4).all(|pixel| pixel[0] >= 250
            && pixel[1] >= 250
            && pixel[2] >= 250
            && (127..=128).contains(&pixel[3])));
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }
}
