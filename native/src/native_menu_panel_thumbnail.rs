//! Thumbnails composite against current background using fixed stack buffers; no decoding, HBITMAP, or cross-frame DC ownership.
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

pub unsafe fn draw(dc: HDC, rect: RECT, thumbnail: &Thumbnail, background: u32) -> bool {
    let (width, height) = (rect.right - rect.left - 2, rect.bottom - rect.top - 4);
    draw_fitted(dc, rect, thumbnail, background, width, height, false)
}
/// Candidate cards use mainline centered object-fit: cover cropping; menu thumbnails preserve full images/padding.
pub unsafe fn draw_cover(dc: HDC, rect: RECT, thumbnail: &Thumbnail, background: u32) -> bool {
    draw_fitted(
        dc,
        rect,
        thumbnail,
        background,
        rect.right - rect.left,
        rect.bottom - rect.top,
        true,
    )
}
unsafe fn draw_fitted(
    dc: HDC,
    rect: RECT,
    thumbnail: &Thumbnail,
    background: u32,
    width: i32,
    height: i32,
    cover: bool,
) -> bool {
    if width <= 0 || height <= 0 {
        return true;
    }
    let x = width as f64 / thumbnail.width() as f64;
    let y = height as f64 / thumbnail.height() as f64;
    let scale = if cover { x.max(y) } else { x.min(y) };
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
    let saved = SaveDC(dc);
    if cover {
        IntersectClipRect(dc, rect.left, rect.top, rect.right, rect.bottom);
    }
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
    let _ = RestoreDC(dc, saved);
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
    struct Buffer(HDC, HBITMAP, HGDIOBJ);
    impl Buffer {
        unsafe fn new() -> Self {
            let dc = CreateCompatibleDC(None);
            assert!(!dc.is_invalid());
            let mut bits = std::ptr::null_mut();
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: 24,
                    biHeight: -24,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let bitmap =
                CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();
            let old = SelectObject(dc, HGDIOBJ(bitmap.0));
            Self(dc, bitmap, old)
        }
    }
    impl Drop for Buffer {
        fn drop(&mut self) {
            unsafe {
                SelectObject(self.0, self.2);
                let _ = DeleteObject(HGDIOBJ(self.1 .0));
                let _ = DeleteDC(self.0);
            }
        }
    }
    #[test]
    fn candidate_cover_fills_the_preview_and_clips_overflow_without_changing_menu_contain() {
        unsafe {
            let buffer = Buffer::new();
            let thumbnail = Thumbnail::from_rgba(
                4,
                1,
                &[
                    255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
                ],
            )
            .unwrap();
            let all = RECT {
                right: 24,
                bottom: 24,
                ..Default::default()
            };
            let rect = RECT {
                left: 6,
                top: 6,
                right: 14,
                bottom: 14,
            };
            let sentinel = windows::Win32::Foundation::COLORREF(0x050403);
            SetDCBrushColor(buffer.0, sentinel);
            FillRect(buffer.0, &all, HBRUSH(GetStockObject(DC_BRUSH).0));
            let mut before = RECT::default();
            GetClipBox(buffer.0, &mut before);
            assert!(draw_cover(buffer.0, rect, &thumbnail, 0x110c09));
            GdiFlush().ok().unwrap();
            assert_ne!(GetPixel(buffer.0, 6, 6), sentinel);
            assert_ne!(GetPixel(buffer.0, 13, 13), sentinel);
            assert_eq!(GetPixel(buffer.0, 5, 10), sentinel);
            assert_eq!(GetPixel(buffer.0, 14, 10), sentinel);
            let mut after = RECT::default();
            GetClipBox(buffer.0, &mut after);
            assert_eq!(after, before);
            FillRect(buffer.0, &all, HBRUSH(GetStockObject(DC_BRUSH).0));
            assert!(draw(buffer.0, rect, &thumbnail, 0x110c09));
            GdiFlush().ok().unwrap();
            assert_eq!(GetPixel(buffer.0, 6, 6), sentinel);
        }
    }
}
