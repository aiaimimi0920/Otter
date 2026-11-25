//! Real WIC readback for bounded source projection: blur may smooth sampling, while base/erasure paths still default to nearest-neighbor.
use super::*;

fn source() -> Image {
    Image {
        width: 2,
        height: 1,
        pixels: vec![0, 0, 0, 255, 255, 255, 255, 255].into(),
    }
}
fn layout(image: &Image) -> Layout {
    let mut layout = Layout::for_frame(
        image,
        &Size {
            width: 2.0,
            height: 1.0,
        }
        .into(),
        None,
    )
    .unwrap();
    layout.output = (8, 1);
    layout
}

#[test]
fn long_source_readback_preserves_every_row_across_strips_and_the_tail() {
    let (width, height) = (12, 16384);
    let pixels = (0..height)
        .flat_map(|row| [row as u8, (row >> 8) as u8, 35, 255].repeat(width as usize))
        .collect::<Vec<_>>();
    let image = Image {
        width,
        height,
        pixels: pixels.into(),
    };
    let layout = Layout::for_frame(
        &image,
        &Size {
            width: 6.0,
            height: 8192.0,
        }
        .into(),
        None,
    )
    .unwrap();
    let result = readback(&image, &layout, &AtomicBool::new(false)).unwrap();
    assert_eq!((result.width, result.height), (width, height));
    assert_eq!(&*result.pixels, &*image.pixels);
    assert_eq!(
        readback(&image, &layout, &AtomicBool::new(true))
            .err()
            .unwrap()
            .code(),
        windows::Win32::Foundation::E_ABORT
    );
}

#[test]
fn source_readback_keeps_nearest_default_and_explicit_linear_is_smooth() {
    let image = source();
    let mut layout = layout(&image);
    let cancel = AtomicBool::new(false);
    let nearest = readback(&image, &layout, &cancel).unwrap();
    assert_eq!(
        nearest
            .pixels
            .chunks_exact(4)
            .map(|p| p[0])
            .collect::<Vec<_>>(),
        vec![0, 0, 0, 0, 255, 255, 255, 255]
    );
    layout.interpolation = D2D1_BITMAP_INTERPOLATION_MODE_LINEAR;
    let linear = readback(&image, &layout, &cancel).unwrap();
    for (pixel, expected) in linear
        .pixels
        .chunks_exact(4)
        .zip([0u8, 0, 32, 96, 159, 223, 255, 255])
    {
        assert!(
            pixel[0].abs_diff(expected) <= 1,
            "unexpected sample {pixel:?}, expected {expected}"
        );
        assert_eq!(pixel[0], pixel[1]);
        assert_eq!(pixel[1], pixel[2]);
        assert_eq!(pixel[3], 255);
    }
}
