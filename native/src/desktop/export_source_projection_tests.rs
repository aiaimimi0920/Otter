//! 有界源分辨率出口的真实 PNG 回归；不是 retained-source/目标 metadata 的生产捕获证明。
use super::*;
use serde_json::json;
fn image(width: u32, height: u32) -> Image {
    Image {
        width,
        height,
        pixels: [255, 0, 0, 255].repeat((width * height) as usize).into(),
    }
}
fn encode(image: &Image, editor: &Editor, frame: Frame, opacity: f32) -> Result<Image> {
    let _com = Apartment::new()?;
    let (frame, expected) = frame.for_projection(image, editor)?;
    let bytes = png_source_resolution(
        image,
        editor,
        frame,
        opacity,
        None,
        0.0,
        &AtomicBool::new(false),
    )?;
    let decoded = super::super::decode_png(bytes.to_vec());
    assert_eq!((decoded.width, decoded.height), expected);
    Ok(decoded)
}
#[test]
fn projection_source_natural_pixels_and_logical_annotations_are_independent() {
    let source = image(80, 40);
    let logical = Size {
        width: 20.0,
        height: 10.0,
    };
    let mut editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (20, 10),
        base: vec![0; 800].into(),
        annotations: None,
        annotation_size: None,
    })
    .unwrap()
    .with_restored_size(logical)
    .unwrap();
    editor.set_style(
        crate::stroke_style::StrokeStyle::new(2.0, crate::stroke_style::Color(255, 0, 0)).unwrap(),
    );
    editor
        .begin(crate::editor::Tool::Brush, Point { x: 5.0, y: 5.0 })
        .unwrap();
    assert!(editor.commit());
    let decoded = encode(&source, &editor, logical.into(), 0.5).unwrap();
    assert_eq!((decoded.width, decoded.height), (80, 40));
    let marked = (20 * 80 + 20) * 4;
    assert_eq!(&decoded.pixels[marked..marked + 4], &[0, 0, 255, 255]);
    assert!((127..=128).contains(&decoded.pixels[3]));
    assert_eq!(editor.layers().unwrap().base.as_ref(), &[0; 800]);
    editor
        .crop_to(Rect {
            x: 5.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        })
        .unwrap();
    let cropped = encode(&source, &editor, logical.into(), 1.0).unwrap();
    assert_eq!((cropped.width, cropped.height), (40, 40));
}
#[test]
fn projection_beautify_wraps_source_pixels_without_scaling_padding_or_fading_background() {
    let source = image(80, 40);
    let logical = Size {
        width: 20.0,
        height: 10.0,
    };
    let editor = Editor::default();
    let frame = Frame::from(logical)
        .with_beautify(&json!({"beautify":{
            "enabled":true,"padding":4,"cornerRadius":3,"backgroundId":"graphite"
        }}))
        .unwrap();
    let decoded = encode(&source, &editor, frame, 0.0).unwrap();
    assert_eq!((decoded.width, decoded.height), (88, 48));
    assert_eq!(&decoded.pixels[..4], &[51, 41, 31, 255]);
    let center = (24 * 88 + 44) * 4;
    assert_eq!(&decoded.pixels[center..center + 4], &[255; 4]);
}
#[test]
fn projection_checks_source_inner_and_wrapped_dimensions_before_composition() {
    let editor = Editor::default();
    // 合法源图即使裁剪成 1×1，仍须拒绝超长自然源，不借裁剪规避准入。
    let source = image(8193, 1);
    let logical = Size {
        width: 1.0,
        height: 1.0,
    };
    let mut frame = Frame::from(logical);
    frame.output = Some((1, 1));
    assert!(frame.for_projection(&source, &editor).is_err());
    let source = image(2, 2);
    for output in [(8193, 1), (4097, 4096)] {
        let mut frame = Frame::from(logical);
        frame.output = Some(output);
        assert!(frame.for_projection(&source, &editor).is_err());
    }
    for padding in [4096, 2048] {
        let frame = Frame::from(logical)
            .with_beautify(&json!({"beautify":{
                "enabled":true,"padding":padding
            }}))
            .unwrap();
        assert!(frame.for_projection(&source, &editor).is_err());
    }
}
#[test]
fn projection_rejects_actual_png_over_four_mib_without_resizing() {
    let _com = Apartment::new().unwrap();
    let mut source = image(2048, 1024);
    let mut random = 0xabcdef01u32;
    let mut pixels = Vec::with_capacity(source.pixels.len());
    for _ in 0..source.width * source.height {
        random ^= random << 13;
        random ^= random >> 17;
        random ^= random << 5;
        pixels.extend_from_slice(&[random as u8, (random >> 8) as u8, (random >> 16) as u8, 255]);
    }
    source.pixels = pixels.into();
    let frame = Frame::from(Size {
        width: 2048.0,
        height: 1024.0,
    });
    let (frame, _) = frame.for_projection(&source, &Editor::default()).unwrap();
    let layout = frame.layout(&source, &Editor::default()).unwrap();
    let output = png::Output::new(&source, &frame, &layout).unwrap();
    let wic: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap() };
    let error = output
        .raw(&wic, &source, &AtomicBool::new(false))
        .unwrap_err();
    assert_eq!(error.code(), E_INVALIDARG);
    assert!(error.message().contains("Projection PNG exceeds limit"));
}
