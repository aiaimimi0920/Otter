//! 有界源投影的真实 WIC 读回：模糊可平滑取样，底图/擦除默认仍使用最近邻。
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
