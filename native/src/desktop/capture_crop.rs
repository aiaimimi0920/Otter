//! 冻结截图裁剪；仅复制选区，检查范围并为输出单独保留预算。
use super::image::Image;
use windows::{
    core::{Error, Result},
    Win32::Foundation::*,
};

pub(super) fn rectangle(start: POINT, end: POINT, width: u32, height: u32) -> Option<RECT> {
    let x = |v: i32| v.clamp(0, width as i32);
    let y = |v: i32| v.clamp(0, height as i32);
    let rect = RECT {
        left: x(start.x.min(end.x)),
        top: y(start.y.min(end.y)),
        right: x(start.x.max(end.x)),
        bottom: y(start.y.max(end.y)),
    };
    (rect.right > rect.left && rect.bottom > rect.top).then_some(rect)
}
pub(super) fn crop(image: &Image, rect: RECT) -> Result<Image> {
    if rect.left < 0
        || rect.top < 0
        || rect.right > image.width as i32
        || rect.bottom > image.height as i32
        || rect.right <= rect.left
        || rect.bottom <= rect.top
    {
        return Err(Error::new(E_INVALIDARG, "Capture crop is outside image"));
    }
    let width = (rect.right - rect.left) as u32;
    let height = (rect.bottom - rect.top) as u32;
    let count = crate::viewport::checked_image_bytes(width, height)
        .ok_or_else(|| Error::new(E_INVALIDARG, "Capture crop exceeds image budget"))?;
    if crate::viewport::checked_image_bytes(image.width, image.height) != Some(image.pixels.len()) {
        return Err(Error::new(E_INVALIDARG, "Invalid capture pixel storage"));
    }
    let mut lease = crate::pixels::reserve(count)
        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Capture crop budget exhausted"))?;
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(count)
        .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot allocate capture crop"))?;
    if !lease.grow_to(pixels.capacity()) {
        return Err(Error::new(
            E_OUTOFMEMORY,
            "Capture crop capacity exceeds budget",
        ));
    }
    let stride = image.width as usize * 4;
    for y in rect.top..rect.bottom {
        let start = y as usize * stride + rect.left as usize * 4;
        pixels.extend_from_slice(&image.pixels[start..start + width as usize * 4]);
    }
    Ok(Image {
        width,
        height,
        pixels: crate::pixels::Pixels::from_reserved(pixels, lease),
    })
}
pub(super) fn positioned(image: &Image, origin: POINT, rect: RECT) -> Result<(Image, POINT)> {
    let screen = screen_rectangle(origin, rect)?;
    Ok((
        crop(image, rect)?,
        POINT {
            x: screen.left,
            y: screen.top,
        },
    ))
}
pub(super) fn screen_rectangle(origin: POINT, rect: RECT) -> Result<RECT> {
    let x = |value| {
        origin
            .x
            .checked_add(value)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Capture screen X overflow"))
    };
    let y = |value| {
        origin
            .y
            .checked_add(value)
            .ok_or_else(|| Error::new(E_INVALIDARG, "Capture screen Y overflow"))
    };
    Ok(RECT {
        left: x(rect.left)?,
        top: y(rect.top)?,
        right: x(rect.right)?,
        bottom: y(rect.bottom)?,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn positioned_crop_preserves_negative_virtual_desktop_coordinates() {
        let image = Image {
            width: 3,
            height: 3,
            pixels: vec![255; 36].into(),
        };
        let (cropped, origin) = positioned(
            &image,
            POINT { x: -1920, y: -1080 },
            RECT {
                left: 1,
                top: 2,
                right: 3,
                bottom: 3,
            },
        )
        .unwrap();
        assert_eq!((origin.x, origin.y), (-1919, -1078));
        assert_eq!((cropped.width, cropped.height), (2, 1));
        assert_eq!(cropped.pixels.len(), 8);
        assert_eq!(image.pixels.len(), 36);
    }
    #[test]
    fn positioned_crop_rejects_screen_coordinate_overflow() {
        let image = Image {
            width: 2,
            height: 2,
            pixels: vec![255; 16].into(),
        };
        let rect = RECT {
            left: 1,
            top: 1,
            right: 2,
            bottom: 2,
        };
        for origin in [
            POINT { x: i32::MAX, y: 0 },
            POINT { x: 0, y: i32::MAX },
            POINT {
                x: i32::MAX - 1,
                y: 0,
            },
        ] {
            assert!(positioned(&image, origin, rect).is_err());
        }
    }
    #[test]
    fn reverse_and_outside_drag_crop_exact_rows_without_modifying_source() {
        let image = Image {
            width: 3,
            height: 2,
            pixels: (0..24).collect::<Vec<u8>>().into(),
        };
        let rect = rectangle(POINT { x: 9, y: 8 }, POINT { x: 1, y: -2 }, 3, 2).unwrap();
        let cropped = crop(&image, rect).unwrap();
        assert_eq!((cropped.width, cropped.height), (2, 2));
        assert_eq!(
            &*cropped.pixels,
            &[4, 5, 6, 7, 8, 9, 10, 11, 16, 17, 18, 19, 20, 21, 22, 23]
        );
        assert_eq!(image.pixels.len(), 24);
        assert!(rectangle(POINT::default(), POINT::default(), 3, 2).is_none());
        assert!(crop(
            &image,
            RECT {
                left: -1,
                top: 0,
                right: 1,
                bottom: 1
            }
        )
        .is_err());
    }
}
