//! 按需绘制有界缩略图，不缓存位图或 GPU 资源。
use crate::desktop::image::Image;
use windows::Win32::{Graphics::Gdi::*, UI::Controls::DRAWITEMSTRUCT};
/// 仅在收到新预览时处理透明区域；不修改采集覆盖数据，opaque 图像直接共享原存储。
pub(super) fn prepare(image: Image) -> Result<Image, crate::long_capture::Error> {
    prepare_with(image, crate::pixels::reserve)
}
fn prepare_with(
    image: Image,
    reserve: impl FnOnce(usize) -> Option<crate::byte_budget::Lease>,
) -> Result<Image, crate::long_capture::Error> {
    use crate::long_capture::Error;
    if image.width == 0 || image.height == 0 || image.width > 384 || image.height > 384 {
        return Err(Error::Dimensions);
    }
    let length = image.width as usize * image.height as usize * 4;
    if image.pixels.len() != length {
        return Err(Error::Pixels);
    }
    if image.pixels.chunks_exact(4).all(|pixel| pixel[3] == 255) {
        return Ok(image);
    }
    let mut lease = reserve(length).ok_or(Error::Budget)?;
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(length)
        .map_err(|_| Error::Budget)?;
    if !lease.grow_to(pixels.capacity()) {
        return Err(Error::Budget);
    }
    pixels.extend_from_slice(&image.pixels);
    for (index, pixel) in pixels.chunks_exact_mut(4).enumerate() {
        let x = index % image.width as usize;
        let y = index / image.width as usize;
        let background = if ((x / 8) + (y / 8)).is_multiple_of(2) {
            208u32
        } else {
            168
        };
        let alpha = u32::from(pixel[3]);
        for channel in &mut pixel[..3] {
            *channel =
                ((u32::from(*channel) * alpha + background * (255 - alpha) + 127) / 255) as u8;
        }
        pixel[3] = 255;
    }
    Ok(Image {
        width: image.width,
        height: image.height,
        pixels: crate::pixels::Pixels::from_reserved(pixels, lease),
    })
}
pub(super) unsafe fn paint(item: &DRAWITEMSTRUCT, image: Option<&Image>) {
    FillRect(item.hDC, &item.rcItem, GetSysColorBrush(COLOR_WINDOW));
    let Some(image) = image else {
        return;
    };
    let width = item.rcItem.right - item.rcItem.left;
    let height = item.rcItem.bottom - item.rcItem.top;
    if width <= 0 || height <= 0 {
        return;
    }
    let ratio = (width as f64 / image.width as f64).min(height as f64 / image.height as f64);
    let w = (image.width as f64 * ratio).round().max(1.0) as i32;
    let h = (image.height as f64 * ratio).round().max(1.0) as i32;
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: image.width as i32,
            biHeight: -(image.height as i32),
            biPlanes: 1,
            biBitCount: 32,
            ..Default::default()
        },
        ..Default::default()
    };
    StretchDIBits(
        item.hDC,
        item.rcItem.left + (width - w) / 2,
        item.rcItem.top + (height - h) / 2,
        w,
        h,
        0,
        0,
        image.width as i32,
        image.height as i32,
        Some(image.pixels.as_ptr().cast()),
        &info,
        DIB_RGB_COLORS,
        SRCCOPY,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transparent_holes_render_as_checkerboard_without_changing_covered_black_or_source() {
        let mut bytes = vec![0; 16 * 4];
        bytes[3] = 255; // 真正的黑色像素不能变成棋盘格。
        bytes[8..12].copy_from_slice(&[100, 100, 100, 128]);
        let source: crate::pixels::Pixels = bytes.into();
        let image = prepare(Image {
            width: 16,
            height: 1,
            pixels: source.clone(),
        })
        .unwrap();
        assert_eq!(&image.pixels[..4], &[0, 0, 0, 255]);
        assert_eq!(&image.pixels[4..8], &[208, 208, 208, 255]);
        assert_eq!(&image.pixels[8..12], &[154, 154, 154, 255]);
        assert_eq!(&image.pixels[32..36], &[168, 168, 168, 255]);
        assert_eq!(source[7], 0);
        let output = crate::desktop::capture::synthetic(16, 1, |dc| {
            let item = DRAWITEMSTRUCT {
                hDC: dc,
                rcItem: windows::Win32::Foundation::RECT {
                    left: 0,
                    top: 0,
                    right: 16,
                    bottom: 1,
                },
                ..Default::default()
            };
            unsafe {
                paint(&item, Some(&image));
            }
            Ok(())
        })
        .unwrap();
        assert_eq!(&*output.pixels, &*image.pixels);
    }
    #[test]
    fn opaque_preview_does_not_allocate_and_transparent_copy_obeys_budget() {
        let source: crate::pixels::Pixels = vec![20, 30, 40, 255].into();
        let result = prepare_with(
            Image {
                width: 1,
                height: 1,
                pixels: source.clone(),
            },
            |_| panic!("opaque preview must not reserve"),
        )
        .unwrap();
        assert!(result.pixels.same_storage(&source));
        let budget = crate::byte_budget::Budget::new(3);
        assert!(matches!(
            prepare_with(
                Image {
                    width: 1,
                    height: 1,
                    pixels: vec![0; 4].into()
                },
                |n| budget.reserve(n)
            ),
            Err(crate::long_capture::Error::Budget)
        ));
        assert_eq!(budget.used(), 0);
        assert!(matches!(
            prepare(Image {
                width: 385,
                height: 1,
                pixels: vec![0; 385 * 4].into()
            }),
            Err(crate::long_capture::Error::Dimensions)
        ));
    }
    #[test]
    fn preview_draws_top_down_bgra_centered_in_memory_dc() {
        let image = Image {
            width: 2,
            height: 2,
            pixels: vec![0, 0, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 0, 0, 255].into(),
        };
        let result = crate::desktop::capture::synthetic(8, 4, |dc| {
            let item = DRAWITEMSTRUCT {
                hDC: dc,
                rcItem: windows::Win32::Foundation::RECT {
                    left: 0,
                    top: 0,
                    right: 8,
                    bottom: 4,
                },
                ..Default::default()
            };
            unsafe {
                paint(&item, Some(&image));
            }
            Ok(())
        })
        .unwrap();
        assert_eq!(&result.pixels[8..12], &[0, 0, 255, 255]);
        let last = (3 * 8 + 5) * 4;
        assert_eq!(&result.pixels[last..last + 4], &[255, 0, 0, 255]);
    }
}
