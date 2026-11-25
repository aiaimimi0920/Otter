//! Production borders drawn into owned memory DIBs verify mainline pure white, outer strokes, and DPI pixel-by-pixel, without desktop capture.
use super::*;
const SIZE: usize = 96;
const BACKGROUND: [u8; 3] = [0x35; 3];
struct Image {
    dc: HDC,
    bitmap: HBITMAP,
    old: HGDIOBJ,
    bits: *const u8,
}
impl Image {
    unsafe fn new() -> Self {
        let dc = CreateCompatibleDC(None);
        assert!(!dc.is_invalid());
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: SIZE as i32,
                biHeight: -(SIZE as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let bitmap = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();
        let old = SelectObject(dc, bitmap.into());
        std::ptr::write_bytes(bits, 0x35, SIZE * SIZE * 4);
        Self {
            dc,
            bitmap,
            old,
            bits: bits.cast(),
        }
    }
    unsafe fn verify(&self, rect: Rect, width: i32, clip_right: i32) {
        assert!(GdiFlush().as_bool());
        let pixels = std::slice::from_raw_parts(self.bits, SIZE * SIZE * 4);
        for y in 0..SIZE as i32 {
            for x in 0..SIZE as i32 {
                let point = Point { x, y };
                let outside = Rect {
                    left: rect.left - width,
                    top: rect.top - width,
                    right: rect.right + width,
                    bottom: rect.bottom + width,
                };
                let expected = if x < clip_right && outside.contains(point) && !rect.contains(point)
                {
                    [0xff; 3]
                } else {
                    BACKGROUND
                };
                let offset = (y as usize * SIZE + x as usize) * 4;
                assert_eq!(
                    &pixels[offset..offset + 3],
                    expected,
                    "x={x} y={y} width={width} clip={clip_right}"
                );
            }
        }
    }
}
impl Drop for Image {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.old);
            DeleteObject(self.bitmap.into()).ok().unwrap();
            assert!(DeleteDC(self.dc).as_bool());
        }
    }
}

#[test]
fn pure_white_two_logical_pixels_stay_outside_image_at_every_dpi() {
    unsafe {
        for (dpi, width) in [(96, 2), (120, 3), (144, 3), (192, 4), (768, 16)] {
            let image = Image::new();
            frame(
                image.dc,
                POINT { x: -132, y: -92 },
                Rect {
                    left: -100,
                    top: -60,
                    right: -76,
                    bottom: -40,
                },
                dpi,
            );
            image.verify(
                Rect {
                    left: 32,
                    top: 32,
                    right: 56,
                    bottom: 52,
                },
                width,
                SIZE as i32,
            );
        }
    }
}

#[test]
fn partial_repaint_clip_preserves_image_and_uninvalidated_pixels() {
    unsafe {
        let image = Image::new();
        assert_ne!(
            IntersectClipRect(image.dc, 0, 0, 40, SIZE as i32),
            RGN_ERROR
        );
        let rect = Rect {
            left: 32,
            top: 32,
            right: 56,
            bottom: 52,
        };
        frame(image.dc, POINT::default(), rect, 144);
        image.verify(rect, 3, 40);
    }
}

#[test]
fn tiny_images_and_dpi_bounds_never_turn_the_frame_into_an_inner_border() {
    unsafe {
        for (dpi, width) in [(0, 2), (4096, 16)] {
            let image = Image::new();
            let rect = Rect {
                left: 10,
                top: 10,
                right: 11,
                bottom: 11,
            };
            frame(image.dc, POINT::default(), rect, dpi);
            image.verify(rect, width, SIZE as i32);
        }
    }
}
