//! 真实 WIC/D2D 路径，保护自然分辨率、小数逻辑坐标和底图透明度。
use super::*;
#[test]
fn raster_annotation_keeps_its_opacity_when_source_is_transparent() {
    let _com = Apartment::new().unwrap();
    let image = Image {
        width: 4,
        height: 2,
        pixels: vec![255; 32].into(),
    };
    let logical = Size {
        width: 2.0,
        height: 1.0,
    };
    let editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (2, 1),
        base: vec![0; 8].into(),
        annotations: Some(vec![0, 0, 255, 255].into()),
        annotation_size: Some((1, 1)),
    })
    .unwrap()
    .with_restored_size(logical)
    .unwrap();
    let border = frame_border::Border::read(&serde_json::json!({
        "borderWidth": 0.25, "borderColor": "blue"
    }))
    .unwrap();
    for border in [None, border] {
        let png = png_source_resolution(
            &image,
            &editor,
            logical.into(),
            0.0,
            border,
            0.0,
            &AtomicBool::new(false),
        )
        .unwrap();
        let decoded = super::super::decode_png(png.to_vec());
        assert_eq!((decoded.width, decoded.height), (4, 2));
        // 独立红色标注覆盖蓝色边框，且不继承底图透明度。
        assert!(decoded
            .pixels
            .chunks_exact(4)
            .all(|pixel| pixel == [0, 0, 255, 255]));
    }
}
fn source() -> Image {
    let mut bytes = Vec::new();
    for y in 0..17 {
        for x in 0..31 {
            bytes.extend_from_slice(&[x * 7, y * 13, (x + y) * 5, 255]);
        }
    }
    Image {
        width: 31,
        height: 17,
        pixels: bytes.into(),
    }
}

#[test]
fn rounded_crop_clips_independent_raster_annotation_in_visible_coordinates() {
    let _com = Apartment::new().unwrap();
    let image = Image {
        width: 20,
        height: 20,
        pixels: vec![255; 1600].into(),
    };
    let logical = Size {
        width: 20.0,
        height: 20.0,
    };
    let mut editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (20, 20),
        base: image.pixels.clone(),
        annotations: Some([0, 0, 255, 255].repeat(400).into()),
        annotation_size: Some((20, 20)),
    })
    .unwrap();
    editor
        .crop_to(Rect {
            x: 5.0,
            y: 5.0,
            width: 10.0,
            height: 10.0,
        })
        .unwrap();
    let png = png_source_resolution(
        &image,
        &editor,
        logical.into(),
        0.0,
        None,
        5.0,
        &AtomicBool::new(false),
    )
    .unwrap();
    let decoded = super::super::decode_png(png.to_vec());
    assert_eq!((decoded.width, decoded.height), (10, 10));
    assert_eq!(decoded.pixels[3], 0);
    assert_eq!(
        &decoded.pixels[(5 * 10 + 5) * 4..(5 * 10 + 5) * 4 + 4],
        &[0, 0, 255, 255]
    );
}
#[test]
fn fractional_logical_frames_keep_natural_dimensions_and_exact_pixels() {
    let _com = Apartment::new().unwrap();
    let image = source();
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let logical = Size {
            width: 31.0 / scale,
            height: 17.0 / scale,
        };
        let size = (logical.width.ceil() as u32, logical.height.ceil() as u32);
        let editor = Editor::with_layers(crate::erase_layers::Layers {
            size,
            base: vec![0; size.0 as usize * size.1 as usize * 4].into(),
            annotations: None,
            annotation_size: None,
        })
        .unwrap()
        .with_restored_size(logical)
        .unwrap();
        let png = png_source_resolution(
            &image,
            &editor,
            logical.into(),
            1.0,
            None,
            0.0,
            &AtomicBool::new(false),
        )
        .unwrap();
        let decoded = super::super::decode_png(png.to_vec());
        assert_eq!((decoded.width, decoded.height), (31, 17));
        assert_eq!(decoded.pixels.as_ref(), image.pixels.as_ref());
    }
}
#[test]
fn crop_density_and_contain_placement_use_unrounded_logical_frame() {
    let image = source();
    let logical = Size {
        width: 15.5,
        height: 8.5,
    };
    let layout = Layout::new(
        &image,
        logical,
        Some(Rect {
            x: 1.25,
            y: 0.5,
            width: 5.5,
            height: 3.5,
        }),
    )
    .unwrap();
    assert_eq!(layout.output, (11, 7));
    let padded = Layout::new(
        &image,
        Size {
            width: 31.0,
            height: 34.0,
        },
        None,
    )
    .unwrap();
    assert_eq!(padded.output, (31, 17));
    assert_eq!(padded.base.top, 8.5);
    assert_eq!(padded.base.bottom, 25.5);
    assert!(Layout::new(
        &image,
        logical,
        Some(Rect {
            x: -1.0,
            y: 0.0,
            width: 1.0,
            height: 1.0
        })
    )
    .is_err());
}
#[test]
fn explicit_source_is_not_replaced_by_projected_layers_and_cancel_is_respected() {
    let _com = Apartment::new().unwrap();
    let image = source();
    let logical = Size {
        width: 31.0,
        height: 17.0,
    };
    let editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (31, 17),
        base: vec![0; 31 * 17 * 4].into(),
        annotations: None,
        annotation_size: None,
    })
    .unwrap();
    let png = png_source_resolution(
        &image,
        &editor,
        logical.into(),
        0.5,
        None,
        0.0,
        &AtomicBool::new(false),
    )
    .unwrap();
    let decoded = super::super::decode_png(png.to_vec());
    assert!((127..=128).contains(&decoded.pixels[3]));
    assert!(decoded.pixels[4] > 0);
    assert!(png_source_resolution(
        &image,
        &editor,
        logical.into(),
        1.0,
        None,
        0.0,
        &AtomicBool::new(true)
    )
    .is_err());
}
