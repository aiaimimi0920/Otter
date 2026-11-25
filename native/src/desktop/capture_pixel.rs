//! Single-pixel color sampling from frozen frames; no local magnifier drawing, sample-block copies, or recapture.
use super::image::Image;
use windows::Win32::Foundation::POINT;

pub(super) fn rgb(image: &Image, point: POINT) -> Option<[u8; 3]> {
    if image.width == 0
        || image.height == 0
        || crate::viewport::checked_image_bytes(image.width, image.height)
            != Some(image.pixels.len())
    {
        return None;
    }
    let x = point.x.clamp(0, image.width as i32 - 1) as usize;
    let y = point.y.clamp(0, image.height as i32 - 1) as usize;
    let offset = (y * image.width as usize + x) * 4;
    Some([
        image.pixels[offset + 2],
        image.pixels[offset + 1],
        image.pixels[offset],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn single_pixel_sample_preserves_top_down_bgra_and_clamped_edges() {
        let image = Image {
            width: 2,
            height: 2,
            pixels: vec![1, 2, 3, 255, 4, 5, 6, 255, 7, 8, 9, 255, 10, 11, 12, 255].into(),
        };
        assert_eq!(rgb(&image, POINT { x: 1, y: 0 }), Some([6, 5, 4]));
        assert_eq!(rgb(&image, POINT { x: -20, y: 50 }), Some([9, 8, 7]));
        assert_eq!(rgb(&image, POINT { x: 50, y: -20 }), Some([6, 5, 4]));
        let invalid = Image {
            pixels: vec![].into(),
            ..image
        };
        assert!(rgb(&invalid, POINT::default()).is_none());
    }
}
