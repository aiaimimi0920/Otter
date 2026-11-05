//! 冻结帧的有界像素放大镜；固定栈上采样块，不复制整图或产生捕获任务。
use super::image::Image;
use windows::Win32::{Foundation::*, Graphics::Gdi::*};

const SIDE: usize = 21;
const HALF: i32 = (SIDE / 2) as i32;
pub(super) struct Sample {
    pixels: [u8; SIDE * SIDE * 4],
    pub rgb: [u8; 3],
}
pub(super) fn sample(image: &Image, point: POINT) -> Option<Sample> {
    if image.width == 0
        || image.height == 0
        || crate::viewport::checked_image_bytes(image.width, image.height)
            != Some(image.pixels.len())
    {
        return None;
    }
    let x = point.x.clamp(0, image.width as i32 - 1);
    let y = point.y.clamp(0, image.height as i32 - 1);
    let mut pixels = [0u8; SIDE * SIDE * 4];
    for row in 0..SIDE {
        for column in 0..SIDE {
            // 边缘钳位重复最外侧像素，不引入虚假的黑色采样区域。
            let sx = (x + column as i32 - HALF).clamp(0, image.width as i32 - 1) as usize;
            let sy = (y + row as i32 - HALF).clamp(0, image.height as i32 - 1) as usize;
            let src = (sy * image.width as usize + sx) * 4;
            let dst = (row * SIDE + column) * 4;
            pixels[dst..dst + 4].copy_from_slice(&image.pixels[src..src + 4]);
        }
    }
    let center = (SIDE / 2 * SIDE + SIDE / 2) * 4;
    Some(Sample {
        rgb: [pixels[center + 2], pixels[center + 1], pixels[center]],
        pixels,
    })
}
pub(super) fn bounds(image: &Image, point: POINT, dpi: u32) -> RECT {
    let zoom = (6 * dpi.clamp(96, 384) / 96) as i32;
    let width = SIDE as i32 * zoom;
    let height = width + 24;
    let left = if point.x + 24 + width <= image.width as i32 {
        point.x + 24
    } else {
        point.x - 24 - width
    }
    .clamp(0, (image.width as i32 - width).max(0));
    let top = if point.y + 24 + height <= image.height as i32 {
        point.y + 24
    } else {
        point.y - 24 - height
    }
    .clamp(0, (image.height as i32 - height).max(0));
    RECT {
        left,
        top,
        right: left + width,
        bottom: top + height,
    }
}
pub(super) unsafe fn draw(dc: HDC, image: &Image, point: POINT, dpi: u32) -> bool {
    let Some(sample) = sample(image, point) else {
        return false;
    };
    let rect = bounds(image, point, dpi);
    let width = rect.right - rect.left;
    let previous = SetStretchBltMode(dc, COLORONCOLOR);
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: SIDE as i32,
            biHeight: -(SIDE as i32),
            biPlanes: 1,
            biBitCount: 32,
            ..Default::default()
        },
        ..Default::default()
    };
    let result = StretchDIBits(
        dc,
        rect.left,
        rect.top,
        width,
        width,
        0,
        0,
        SIDE as i32,
        SIDE as i32,
        Some(sample.pixels.as_ptr().cast()),
        &info,
        DIB_RGB_COLORS,
        SRCCOPY,
    );
    if previous != 0 {
        SetStretchBltMode(dc, STRETCH_BLT_MODE(previous));
    }
    let zoom = width / SIDE as i32;
    let center = RECT {
        left: rect.left + HALF * zoom,
        top: rect.top + HALF * zoom,
        right: rect.left + (HALF + 1) * zoom,
        bottom: rect.top + (HALF + 1) * zoom,
    };
    let _ = FrameRect(dc, &rect, HBRUSH(GetStockObject(BLACK_BRUSH).0));
    let _ = InvertRect(dc, &center);
    let mut label = RECT {
        left: rect.left,
        top: rect.top + width,
        right: rect.right,
        bottom: rect.bottom,
    };
    let _ = FillRect(dc, &label, HBRUSH(GetStockObject(WHITE_BRUSH).0));
    let mut text: Vec<u16> = format!(
        "#{:02X}{:02X}{:02X}",
        sample.rgb[0], sample.rgb[1], sample.rgb[2]
    )
    .encode_utf16()
    .collect();
    DrawTextW(
        dc,
        &mut text,
        &mut label,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
    // 零扫描线也可能是当前脏区域完全裁掉放大镜，不得误判设备失败。
    result != -1
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_gdi_magnifier_renders_synthetic_frame_without_screen_capture() {
        let image = Image {
            width: 256,
            height: 256,
            pixels: [0u8, 0, 255, 255].repeat(256 * 256).into(),
        };
        let point = POINT { x: 20, y: 20 };
        let rect = bounds(&image, point, 96);
        let rendered = super::super::capture::synthetic(256, 256, |dc| unsafe {
            assert!(draw(dc, &image, point, 96));
            Ok(())
        })
        .unwrap();
        let offset = ((rect.top + 1) as usize * 256 + (rect.left + 1) as usize) * 4;
        assert_eq!(&rendered.pixels[offset..offset + 4], &[0, 0, 255, 255]);
    }
    #[test]
    fn sample_is_top_down_bgra_with_clamped_edges_and_rgb_label() {
        let image = Image {
            width: 2,
            height: 2,
            pixels: vec![1, 2, 3, 255, 4, 5, 6, 255, 7, 8, 9, 255, 10, 11, 12, 255].into(),
        };
        let sampled = sample(&image, POINT { x: 1, y: 0 }).unwrap();
        assert_eq!(sampled.rgb, [6, 5, 4]);
        assert_eq!(&sampled.pixels[..4], &[1, 2, 3, 255]);
        assert_eq!(
            &sampled.pixels[sampled.pixels.len() - 4..],
            &[10, 11, 12, 255]
        );
        assert_eq!(
            sample(&image, POINT { x: -20, y: 50 }).unwrap().rgb,
            [9, 8, 7]
        );
        assert_eq!(sampled.pixels.len(), 1764);
    }
    #[test]
    fn overlay_flips_away_from_right_bottom_and_stays_inside_large_frame() {
        let image = Image {
            width: 1000,
            height: 800,
            pixels: vec![].into(),
        };
        for dpi in [96, 144, 192] {
            let rect = bounds(&image, POINT { x: 990, y: 790 }, dpi);
            assert!(rect.left >= 0 && rect.top >= 0 && rect.right <= 1000 && rect.bottom <= 800);
            assert!(rect.right < 990 && rect.bottom < 790);
        }
        assert!(sample(&image, POINT::default()).is_none());
    }
}
