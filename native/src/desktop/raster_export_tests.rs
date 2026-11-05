//! PNG 使用已提交栅格快照，擦除/撤销不重建矢量效果，也不误用旧宿主底图。
use super::*;
use crate::{
    editor::Tool,
    erase_layers::{Layers, Mode},
    stroke_style::Color,
};

#[test]
fn export_tracks_raster_history_and_keeps_vector_annotations_above_raster() {
    let _apartment = Apartment::new().unwrap();
    let source = Layers {
        annotation_size: None,
        size: (32, 32),
        base: [0, 0, 255, 255].repeat(32 * 32).into(),
        annotations: Some([0, 255, 0, 255].repeat(32 * 32).into()),
    };
    let stale = Image {
        width: 32,
        height: 32,
        pixels: source.base.clone(),
    };
    let mut editor = Editor::with_layers(source.clone()).unwrap();
    let original = tests::decode_png(encode(&stale, &editor).unwrap());
    assert_eq!(&original.pixels[..4], &[0, 255, 0, 255]);
    let point = Point { x: 16.0, y: 16.0 };
    let annotations_only = source
        .erase(Mode::RasterAnnotations, &[point], 12.0, || false)
        .unwrap();
    assert!(editor
        .replace_layers(editor.revision(), &source, annotations_only.clone())
        .unwrap());
    let middle = (16 * 32 + 16) * 4;
    let output = tests::decode_png(encode(&stale, &editor).unwrap());
    assert_eq!(&output.pixels[middle..middle + 4], &[0, 0, 255, 255]);
    assert_eq!(&output.pixels[..4], &[0, 255, 0, 255]);
    let content = annotations_only
        .erase(Mode::Content, &[point], 12.0, || false)
        .unwrap();
    assert!(editor
        .replace_layers(editor.revision(), &annotations_only, content.clone())
        .unwrap());
    let erased = tests::decode_png(encode(&stale, &editor).unwrap());
    assert_eq!(&erased.pixels[middle..middle + 4], &[0; 4]);
    editor.set_style(
        editor
            .style()
            .with_width(4.0)
            .unwrap()
            .with_color(Color(0, 0, 255)),
    );
    editor.begin(Tool::Brush, point).unwrap();
    assert!(editor.commit());
    let vector = tests::decode_png(encode(&stale, &editor).unwrap());
    assert_eq!(&vector.pixels[middle..middle + 4], &[255, 0, 0, 255]);
    let snapshot = editor.export_snapshot();
    assert!(editor.undo());
    assert_eq!(
        tests::decode_png(encode(&stale, &editor).unwrap()).pixels,
        erased.pixels
    );
    assert!(editor.undo());
    assert_eq!(
        tests::decode_png(encode(&stale, &editor).unwrap()).pixels,
        output.pixels
    );
    assert!(editor.undo());
    assert_eq!(
        tests::decode_png(encode(&stale, &editor).unwrap()).pixels,
        original.pixels
    );
    assert_eq!(
        tests::decode_png(encode(&stale, &snapshot).unwrap()).pixels,
        vector.pixels
    );
}
