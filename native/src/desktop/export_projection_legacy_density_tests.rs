//! 自然8×4、逻辑8×8的旧式伸展；完整和二次crop都保留逐轴自然密度。
use super::*;

#[test]
fn projection_legacy_nonuniform_logical_frame_keeps_natural_output_density_per_axis() {
    let _com = Apartment::new().unwrap();
    let image = Image {
        width: 8,
        height: 4,
        pixels: (0..32)
            .flat_map(|i| [((i % 8) * 24) as u8, ((i / 8) * 24) as u8, 32, 255])
            .collect::<Vec<_>>()
            .into(),
    };
    let mut editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (8, 8),
        base: vec![0; 256].into(),
        annotations: None,
        annotation_size: None,
    })
    .unwrap()
    .with_restored_size(Size {
        width: 8.0,
        height: 8.0,
    })
    .unwrap()
    .with_restored_legacy_crop(
        region(2.0, 0.0, 2.0, 4.0),
        Size {
            width: 8.0,
            height: 4.0,
        },
    )
    .unwrap();
    let target = source(json!(null), json!(null));
    let full = render(
        &target,
        &image,
        &editor,
        Input {
            content: true,
            legacy_pixels: editor.crop_view().unwrap().legacy_pixel_region(),
            ..Input::default()
        },
    );
    assert_eq!((full.width, full.height), (8, 4));
    assert_pixels(&full, |x, y| {
        [((2 + x / 4) * 24) as u8, (y * 24) as u8, 32, 255]
    });
    editor.crop_to(region(2.0, 2.0, 4.0, 4.0)).unwrap();
    let cropped = render(
        &target,
        &image,
        &editor,
        Input {
            content: true,
            legacy_pixels: editor.crop_view().unwrap().legacy_pixel_region(),
            ..Input::default()
        },
    );
    assert_eq!((cropped.width, cropped.height), (4, 2));
    assert_pixels(&cropped, |x, y| {
        [((2 + x / 2) * 24) as u8, ((1 + y) * 24) as u8, 32, 255]
    });
}
