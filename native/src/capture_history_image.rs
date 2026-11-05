//! 历史图像只保存最长边240的PNG；后台准备240复用图及64显示图，共用Pixels预算。
use crate::{image_decode::Image, image_thumbnail::Thumbnail};
use base64::{engine::general_purpose::STANDARD, Engine};

pub(crate) const EDGE: u32 = 240;
#[derive(Clone)]
pub(crate) struct Preview {
    pub image: Image,
    pub small: Thumbnail,
}
pub(crate) fn size(width: u32, height: u32) -> Option<(u32, u32)> {
    if width == 0 || height == 0 {
        return None;
    }
    let longest = width.max(height).max(EDGE) as u64;
    let scale =
        |value: u32| ((u64::from(value) * u64::from(EDGE) + longest / 2) / longest).max(1) as u32;
    Some((scale(width), scale(height)))
}
pub(crate) fn prepare(image: &Image) -> Result<Preview, String> {
    if crate::viewport::checked_image_bytes(image.width, image.height) != Some(image.pixels.len()) {
        return Err("Invalid capture history source".into());
    }
    let (width, height) = size(image.width, image.height).ok_or("Invalid history dimensions")?;
    let count = (width * height * 4) as usize;
    let lease = crate::pixels::reserve(count).ok_or("History pixel budget exhausted")?;
    let mut bytes = vec![0; count];
    for y in 0..height {
        for x in 0..width {
            let src = (((y * image.height / height) * image.width + x * image.width / width) * 4)
                as usize;
            let dst = ((y * width + x) * 4) as usize;
            bytes[dst..dst + 4].copy_from_slice(&image.pixels[src..src + 4]);
        }
    }
    preview(Image {
        width,
        height,
        pixels: crate::pixels::Pixels::from_reserved(bytes, lease),
    })
}
fn preview(image: Image) -> Result<Preview, String> {
    let small = Thumbnail::from_image(&image).map_err(|error| error.to_string())?;
    Ok(Preview { image, small })
}
pub(crate) fn encode(image: &Image) -> Result<String, String> {
    if image.width > EDGE
        || image.height > EDGE
        || crate::viewport::checked_image_bytes(image.width, image.height)
            != Some(image.pixels.len())
    {
        return Err("Invalid history preview".into());
    }
    let _lease =
        crate::pixels::reserve(image.pixels.len()).ok_or("History encoding budget exhausted")?;
    let mut rgba = vec![0u8; image.pixels.len()];
    for (src, dst) in image.pixels.chunks_exact(4).zip(rgba.chunks_exact_mut(4)) {
        for (target, source) in [2, 1, 0].into_iter().enumerate() {
            dst[target] = if src[3] == 0 {
                0
            } else {
                ((u32::from(src[source]) * 255 + u32::from(src[3]) / 2) / u32::from(src[3]))
                    .min(255) as u8
            };
        }
        dst[3] = src[3];
    }
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, image.width, image.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .map_err(|e| e.to_string())?
            .write_image_data(&rgba)
            .map_err(|e| e.to_string())?;
    }
    Ok(format!("data:image/png;base64,{}", STANDARD.encode(bytes)))
}
pub(crate) fn decode(url: &str) -> Result<Preview, String> {
    if url.len() > super::MAX_BYTES {
        return Err("History image exceeds input budget".into());
    }
    let (kind, payload) = url.split_once(',').ok_or("Invalid history data URL")?;
    if !matches!(
        kind,
        "data:image/png;base64"
            | "data:image/jpeg;base64"
            | "data:image/bmp;base64"
            | "data:image/gif;base64"
            | "data:image/webp;base64"
    ) {
        return Err("History requires an embedded image".into());
    }
    let bytes = STANDARD
        .decode(payload)
        .map_err(|_| "Invalid history base64")?;
    preview(Image::from_capture_history(&bytes).map_err(|e| e.to_string())?)
}
pub(crate) fn capture(id: String, image: &Image, at: u64) -> Result<super::Screenshot, String> {
    let preview = prepare(image)?;
    Ok(super::Screenshot {
        id,
        thumbnail: encode(&preview.image)?,
        width: image.width,
        height: image.height,
        at,
        preview: Some(preview),
    })
}
