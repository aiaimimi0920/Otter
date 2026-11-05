//! 小图使用固定栈缓冲按当前背景合成；无解码、HBITMAP 或跨帧 DC 所有权。
use crate::image_thumbnail::Thumbnail;
use windows::Win32::{Foundation::RECT, Graphics::Gdi::*};

fn composite(source: &[u8], background: u32, output: &mut [u8]) {
    let bg = [
        (background >> 16) as u8,
        (background >> 8) as u8,
        background as u8,
    ];
    for (src, dst) in source.chunks_exact(4).zip(output.chunks_exact_mut(4)) {
        for channel in 0..3 {
            dst[channel] = (u16::from(src[channel])
                + (u16::from(bg[channel]) * u16::from(255 - src[3]) + 127) / 255)
                .min(255) as u8;
        }
        dst[3] = 255;
    }
}

pub(crate) unsafe fn draw(dc: HDC, rect: RECT, thumbnail: &Thumbnail, background: u32) -> bool {
    let (width, height) = (rect.right - rect.left - 2, rect.bottom - rect.top - 4);
    if width <= 0 || height <= 0 {
        return true;
    }
    let scale =
        (width as f64 / thumbnail.width() as f64).min(height as f64 / thumbnail.height() as f64);
    let w = (thumbnail.width() as f64 * scale).round().max(1.0) as i32;
    let h = (thumbnail.height() as f64 * scale).round().max(1.0) as i32;
    let mut buffer = [0u8; 64 * 64 * 4];
    composite(
        thumbnail.pixels(),
        background,
        &mut buffer[..thumbnail.pixels().len()],
    );
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: thumbnail.width() as i32,
            biHeight: -(thumbnail.height() as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let result = StretchDIBits(
        dc,
        rect.left + (rect.right - rect.left - w) / 2,
        rect.top + (rect.bottom - rect.top - h) / 2,
        w,
        h,
        0,
        0,
        thumbnail.width() as i32,
        thumbnail.height() as i32,
        Some(buffer.as_ptr().cast()),
        &info,
        DIB_RGB_COLORS,
        SRCCOPY,
    );
    result != 0 && result != -1
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn premultiplied_alpha_composites_over_bgr_colorref_without_dark_fringe() {
        let mut out = [0; 12];
        composite(
            &[0, 0, 0, 0, 10, 20, 30, 255, 10, 20, 30, 128],
            0x302010,
            &mut out,
        );
        assert_eq!(&out[..8], &[48, 32, 16, 255, 10, 20, 30, 255]);
        assert_eq!(&out[8..], &[34, 36, 38, 255]);
    }
}
