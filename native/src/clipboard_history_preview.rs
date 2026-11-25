//! Generate 360-pixel reading previews in the existing history decode worker; budget includes previews, without UI re-decoding.
use crate::{image_decode::Image, pixels::Pixels};
use windows::{
    core::{Error, Result},
    Win32::Foundation::E_OUTOFMEMORY,
};
pub(super) fn image(source: &Image) -> Result<Image> {
    let longest = source.width.max(source.height).max(360);
    let width = (source.width * 360 / longest).max(1);
    let height = (source.height * 360 / longest).max(1);
    let size = (width * height * 4) as usize;
    let lease = crate::pixels::reserve(size)
        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Clipboard preview budget exhausted"))?;
    let mut bytes = vec![0; size];
    for y in 0..height {
        for x in 0..width {
            let from = (((y * source.height / height) * source.width + x * source.width / width)
                * 4) as usize;
            let to = ((y * width + x) * 4) as usize;
            let pixel = &source.pixels[from..from + 4];
            // Same #0e1218 background as menu panels; composite premultiplied pixels only once.
            for (channel, background) in [0x18u16, 0x12, 0x0e].into_iter().enumerate() {
                bytes[to + channel] = (u16::from(pixel[channel])
                    + (background * u16::from(255 - pixel[3]) + 127) / 255)
                    .min(255) as u8;
            }
            bytes[to + 3] = 255;
        }
    }
    Ok(Image {
        width,
        height,
        pixels: Pixels::from_reserved(bytes, lease),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_preserves_aspect_ratio_and_has_more_detail_than_the_card() {
        let source = Image {
            width: 720,
            height: 360,
            pixels: [0, 0, 128, 128].repeat(720 * 360).into(),
        };
        let preview = image(&source).unwrap();
        assert_eq!((preview.width, preview.height), (360, 180));
        assert_eq!(&preview.pixels[..4], &[12, 9, 135, 255]);
    }
}
