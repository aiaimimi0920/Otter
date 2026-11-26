//! Synthetic images and memory DIBs only; verify solid strokes, label contracts, and whole-frame publication boundaries without desktop reads.
use super::*;
use windows::Win32::Graphics::Direct2D::Common::D2D_SIZE_U;

#[test]
fn clipped_nonzero_scanlines_preserve_colors_and_pixels_outside_dirty_rect() {
    let _session = buffered_paint::Session::new().unwrap();
    unsafe {
        let image = Image {
            width: 8,
            height: 8,
            pixels: (0..8u8)
                .flat_map(|y| [y * 20, y * 10, 100, 255].repeat(8))
                .collect::<Vec<_>>()
                .into(),
        };
        let target = crate::software_dib::Bitmap::new(D2D_SIZE_U {
            width: 8,
            height: 8,
        })
        .unwrap();
        let sentinel = COLORREF(0x112233);
        fill(
            target.dc(),
            &RECT {
                left: 0,
                top: 0,
                right: 8,
                bottom: 8,
            },
            sentinel,
        )
        .unwrap();
        let regions = [
            RECT {
                left: 2,
                top: 2,
                right: 5,
                bottom: 4,
            },
            RECT {
                left: 1,
                top: 5,
                right: 7,
                bottom: 7,
            },
        ];
        for (index, dirty) in regions.iter().enumerate() {
            let frame = buffered_paint::Frame::begin(target.dc(), dirty)
                .unwrap()
                .unwrap();
            render_clipped(frame.dc, &image, Some(*dirty), None, 96, false).unwrap();
            frame.commit().unwrap();
            GdiFlush().unwrap();
            for y in 0..8 {
                for x in 0..8 {
                    let painted = regions[..=index]
                        .iter()
                        .any(|r| x >= r.left && x < r.right && y >= r.top && y < r.bottom);
                    let expected = if painted {
                        COLORREF(100 | ((y as u32 * 10) << 8) | ((y as u32 * 20) << 16))
                    } else {
                        sentinel
                    };
                    assert_eq!(GetPixel(target.dc(), x, y), expected, "({x}, {y})");
                }
            }
        }
    }
}

#[test]
fn labels_clipped_at_top_right_and_outside_dirty_region_do_not_cancel_capture() {
    let _session = buffered_paint::Session::new().unwrap();
    unsafe {
        let image = Image {
            width: 256,
            height: 220,
            pixels: [20, 40, 60, 255].repeat(256 * 220).into(),
        };
        let target = crate::software_dib::Bitmap::new(D2D_SIZE_U {
            width: 256,
            height: 220,
        })
        .unwrap();
        for rect in [
            RECT {
                left: 0,
                top: 0,
                right: 60,
                bottom: 60,
            },
            RECT {
                left: 250,
                top: 10,
                right: 256,
                bottom: 100,
            },
        ] {
            for dirty in [
                RECT {
                    left: 0,
                    top: 0,
                    right: 256,
                    bottom: 220,
                },
                RECT {
                    left: 100,
                    top: 200,
                    right: 110,
                    bottom: 210,
                },
            ] {
                let frame = buffered_paint::Frame::begin(target.dc(), &dirty)
                    .unwrap()
                    .unwrap();
                render(frame.dc, &image, Some(rect), 96, false).unwrap();
                frame.commit().unwrap();
            }
        }
    }
}

#[test]
fn signal_outline_stays_outside_measured_pixels_at_each_dpi() {
    unsafe {
        let image = Image {
            width: 256,
            height: 220,
            pixels: [20, 40, 60, 255].repeat(256 * 220).into(),
        };
        let target = crate::software_dib::Bitmap::new(D2D_SIZE_U {
            width: 256,
            height: 220,
        })
        .unwrap();
        let rect = RECT {
            left: 50,
            top: 100,
            right: 180,
            bottom: 190,
        };
        assert_eq!(label_text(rect), "130 x 90");
        for dpi in [96, 144, 192] {
            render(target.dc(), &image, Some(rect), dpi, false).unwrap();
            GdiFlush().ok().unwrap();
            let width = (2 * dpi / 96) as i32;
            for x in rect.left - width..rect.right + width {
                assert_eq!(GetPixel(target.dc(), x, rect.bottom), SIGNAL);
            }
            for y in rect.top..rect.bottom {
                assert_eq!(GetPixel(target.dc(), rect.left - 1, y), SIGNAL);
                assert_eq!(GetPixel(target.dc(), rect.right, y), SIGNAL);
            }
            assert_eq!(
                GetPixel(target.dc(), rect.left, rect.top),
                COLORREF(0x14283c)
            );
            assert_eq!(
                GetPixel(target.dc(), rect.right - 1, rect.bottom - 1),
                COLORREF(0x14283c)
            );
        }
        assert_eq!(&image.pixels[..4], &[20, 40, 60, 255]);
    }
}

#[test]
fn moving_selection_never_publishes_undecorated_or_half_drawn_frame() {
    let _session = buffered_paint::Session::new().unwrap();
    unsafe {
        let image = Image {
            width: 256,
            height: 220,
            pixels: [20, 40, 60, 255].repeat(256 * 220).into(),
        };
        let target = crate::software_dib::Bitmap::new(D2D_SIZE_U {
            width: 256,
            height: 220,
        })
        .unwrap();
        let area = RECT {
            left: 0,
            top: 0,
            right: 256,
            bottom: 220,
        };
        fill(target.dc(), &area, COLORREF(0x112233)).unwrap();
        for x in [50, 70, 90] {
            let before = GetPixel(target.dc(), x - 1, 120);
            let frame = buffered_paint::Frame::begin(target.dc(), &area)
                .unwrap()
                .unwrap();
            render(
                frame.dc,
                &image,
                Some(RECT {
                    left: x,
                    top: 100,
                    right: x + 100,
                    bottom: 190,
                }),
                96,
                false,
            )
            .unwrap();
            assert_eq!(GetPixel(target.dc(), x - 1, 120), before);
            frame.commit().unwrap();
            assert_eq!(GetPixel(target.dc(), x - 1, 120), SIGNAL);
            assert_eq!(GetPixel(target.dc(), x + 1, 120), COLORREF(0x14283c));
            assert_eq!(GetPixel(target.dc(), 5, 5), COLORREF(0x14283c));
        }
        let dirty = RECT {
            left: 0,
            top: 0,
            right: 20,
            bottom: 20,
        };
        let frame = buffered_paint::Frame::begin(target.dc(), &dirty)
            .unwrap()
            .unwrap();
        render(frame.dc, &image, None, 96, false).unwrap();
        frame.commit().unwrap();
        assert_eq!(GetPixel(target.dc(), 89, 120), SIGNAL);
    }
}
