//! 有界只读 PBGRA 缩略图；后台从已解码图片采样，克隆共享像素预算租约。
use crate::{image_decode::Image, pixels::Pixels};
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_INVALIDARG, E_OUTOFMEMORY},
};
#[path = "native_menu_panel_thumbnail.rs"]
pub(crate) mod draw;
#[derive(Clone)]
pub struct Thumbnail {
    width: u32,
    height: u32,
    pixels: Pixels,
}
impl Thumbnail {
    pub(crate) fn from_image(image: &Image) -> Result<Self> {
        Self::sample(image.width, image.height, &image.pixels, false)
    }
    pub(crate) fn from_rgba(width: u32, height: u32, pixels: &[u8]) -> Result<Self> {
        Self::sample(width, height, pixels, true)
    }
    fn sample(width: u32, height: u32, pixels: &[u8], straight: bool) -> Result<Self> {
        if width == 0
            || height == 0
            || width > 8192
            || height > 8192
            || pixels.len() != width as usize * height as usize * 4
        {
            return Err(Error::new(E_INVALIDARG, "Invalid thumbnail source"));
        }
        let longest = width.max(height).max(64);
        let w = (width * 64 / longest).max(1);
        let h = (height * 64 / longest).max(1);
        let size = (w * h * 4) as usize;
        let lease = crate::pixels::reserve(size)
            .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Thumbnail budget exhausted"))?;
        let mut bytes = vec![0; size];
        for y in 0..h {
            for x in 0..w {
                let source = (((y * height / h) * width + x * width / w) * 4) as usize;
                let target = ((y * w + x) * 4) as usize;
                let pixel = &pixels[source..source + 4];
                if straight {
                    for (dst, src) in [2, 1, 0].into_iter().enumerate() {
                        bytes[target + dst] =
                            ((u32::from(pixel[src]) * u32::from(pixel[3]) + 127) / 255) as u8;
                    }
                    bytes[target + 3] = pixel[3];
                } else {
                    bytes[target..target + 4].copy_from_slice(pixel);
                }
            }
        }
        Ok(Self {
            width: w,
            height: h,
            pixels: Pixels::from_reserved(bytes, lease),
        })
    }
    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_is_bounded_aspect_preserving_and_keeps_premultiplied_channels() {
        let image = Image {
            width: 128,
            height: 64,
            pixels: [10, 20, 30, 40].repeat(128 * 64).into(),
        };
        let thumbnail = Thumbnail::from_image(&image).unwrap();
        assert_eq!((thumbnail.width(), thumbnail.height()), (64, 32));
        assert_eq!(&thumbnail.pixels()[..4], &[10, 20, 30, 40]);
        assert!(thumbnail.pixels.same_storage(&thumbnail.clone().pixels));
        assert!(Thumbnail::from_image(&Image {
            width: 1,
            height: 1,
            pixels: vec![0; 3].into()
        })
        .is_err());
    }
    #[test]
    fn straight_service_preview_is_sampled_and_premultiplied_without_copying_a_full_frame() {
        let preview = Thumbnail::from_rgba(2, 1, &[255, 0, 0, 128, 0, 255, 0, 255]).unwrap();
        assert_eq!([preview.width(), preview.height()], [2, 1]);
        assert_eq!(preview.pixels(), [0, 0, 128, 128, 0, 255, 0, 255]);
        assert!(Thumbnail::from_rgba(1, 1, &[0; 3]).is_err());
    }
}
