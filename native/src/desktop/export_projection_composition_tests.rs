//! 独立构造的像素锚点验证组合顺序，不以待测渲染器计算预期结果。
use super::*;
use crate::{
    editor::Tool,
    erase_layers::Layers,
    stroke_style::{Color, StrokeStyle},
};

fn fixture() -> (Image, Editor) {
    let mut pixels = [80, 40, 20, 128].repeat(32 * 16);
    pixels[(8 * 32 + 16) * 4..(8 * 32 + 16) * 4 + 4].fill(0);
    let image = Image {
        width: 32,
        height: 16,
        pixels: pixels.into(),
    };
    let mut overlay = vec![0; 32 * 16 * 4];
    for y in 5..11 {
        for x in 7..13 {
            overlay[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4].copy_from_slice(&[0, 255, 0, 255]);
        }
    }
    let mut editor = Editor::with_layers(Layers {
        size: (32, 16),
        base: image.pixels.clone(),
        annotations: Some(overlay.into()),
        annotation_size: None,
    })
    .unwrap();
    editor.set_style(StrokeStyle::new(4.0, Color(0, 0, 255)).unwrap());
    editor
        .begin(Tool::Brush, Point { x: 24.0, y: 8.0 })
        .unwrap();
    assert!(editor.commit());
    (image, editor)
}

fn render(beautify: bool) -> Image {
    let _com = Apartment::new().unwrap();
    let (image, editor) = fixture();
    let source = source(serde_json::json!({
        "borderWidth": 2, "borderColor": "#ff0000", "cornerRadius": 4,
        "beautify": {"enabled": beautify, "padding": 3, "backgroundId": "white",
            "cornerRadius": 0, "shadow": false}
    }));
    let output = target_frame(
        &source,
        "target",
        &image,
        &editor,
        Presentation {
            opacity: 0.5,
            kind: crate::projection_delivery::sender::Kind::Sticker,
            geometry: None,
        },
        &AtomicBool::new(false),
    )
    .unwrap();
    let decoded = crate::desktop::export::decode_png(output.png.to_vec());
    assert_eq!(
        (decoded.width, decoded.height),
        (output.width, output.height)
    );
    assert_eq!(
        (output.width, output.height),
        if beautify { (38, 22) } else { (32, 16) }
    );
    decoded
}

fn pixel(image: &Image, x: usize, y: usize) -> &[u8] {
    let at = (y * image.width as usize + x) * 4;
    &image.pixels[at..at + 4]
}

#[test]
fn projection_combines_transparency_raster_vector_border_and_whole_stack_rounding() {
    let image = render(false);
    assert_eq!(pixel(&image, 0, 0), &[0, 0, 0, 0]);
    assert_eq!(pixel(&image, 0, 8), &[0, 0, 255, 255]);
    assert_eq!(pixel(&image, 10, 8), &[0, 255, 0, 255]);
    assert_eq!(pixel(&image, 24, 8), &[255, 0, 0, 255]);
    assert_eq!(pixel(&image, 16, 8), &[0, 0, 0, 0]);
    let base = pixel(&image, 19, 12);
    for (actual, expected) in base.iter().zip([40u8, 20, 10, 64]) {
        assert!(actual.abs_diff(expected) <= 1, "base opacity: {base:?}");
    }
}

#[test]
fn projection_beautify_wraps_final_content_without_fading_annotations_or_border() {
    let image = render(true);
    assert_eq!(pixel(&image, 0, 0), &[255; 4]);
    assert_eq!(pixel(&image, 3, 3), &[255; 4]);
    assert_eq!(pixel(&image, 3, 11), &[0, 0, 255, 255]);
    assert_eq!(pixel(&image, 13, 11), &[0, 255, 0, 255]);
    assert_eq!(pixel(&image, 27, 11), &[255, 0, 0, 255]);
    assert_eq!(pixel(&image, 19, 11), &[255; 4]);
    let base = pixel(&image, 22, 15);
    for (actual, expected) in base.iter().zip([231u8, 211, 201, 255]) {
        assert!(actual.abs_diff(expected) <= 1, "wrapped opacity: {base:?}");
    }
}
