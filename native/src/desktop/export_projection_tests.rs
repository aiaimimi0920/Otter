//! 发送 PNG 的实际 WIC 像素验证；裁剪/标注与底图透明度保持各自语义。
use super::*;
use crate::{
    editor::Tool,
    stroke_style::{Color, StrokeStyle},
};
#[path = "export_projection_active_blur_tests.rs"]
mod active_blur;
#[path = "export_projection_baseline_tests.rs"]
mod baseline;
#[path = "export_projection_basis_tests.rs"]
mod basis;
#[path = "export_projection_pipeline_tests.rs"]
mod pipeline;
#[test]
fn projection_png_crops_committed_annotations_and_preserves_normal_opacity() {
    let _com = Apartment::new().unwrap();
    let image = Image {
        width: 32,
        height: 32,
        pixels: [255, 0, 0, 255].repeat(32 * 32).into(),
    };
    let mut editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (32, 32),
        base: image.pixels.clone(),
        annotations: None,
        annotation_size: None,
    })
    .unwrap();
    editor.set_style(StrokeStyle::new(6.0, Color(255, 0, 0)).unwrap());
    editor.begin(Tool::Brush, Point { x: 8.0, y: 8.0 }).unwrap();
    assert!(editor.commit());
    editor
        .crop_to(crate::crop_geometry::Rect {
            x: 4.0,
            y: 4.0,
            width: 16.0,
            height: 16.0,
        })
        .unwrap();
    let png = frame(
        &image,
        &editor,
        0.5,
        crate::projection_delivery::sender::Kind::Sticker,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!((png.width, png.height), (16, 16));
    let decoded = super::super::decode_png(png.png.to_vec());
    let marked = &decoded.pixels[(4 * 16 + 4) * 4..(4 * 16 + 4) * 4 + 4];
    assert_eq!(
        marked,
        &[0, 0, 255, 255],
        "annotation is not faded with the base"
    );
    let base = &decoded.pixels[(14 * 16 + 14) * 4..(14 * 16 + 14) * 4 + 4];
    assert!(
        (127..=128).contains(&base[0])
            && base[1] == 0
            && base[2] == 0
            && (127..=128).contains(&base[3])
    );
}
#[test]
fn projection_png_stops_before_encoding_and_rejects_invalid_opacity() {
    let image = Image {
        width: 1,
        height: 1,
        pixels: vec![0; 4].into(),
    };
    let editor = Editor::default();
    assert!(frame(
        &image,
        &editor,
        1.0,
        crate::projection_delivery::sender::Kind::Art,
        &AtomicBool::new(true)
    )
    .is_err());
    assert!(frame(
        &image,
        &editor,
        f32::NAN,
        crate::projection_delivery::sender::Kind::Art,
        &AtomicBool::new(false)
    )
    .is_err());
}
#[test]
fn projection_preserves_explicit_natural_source_instead_of_editor_preview_base() {
    let _com = Apartment::new().unwrap();
    let preview: crate::pixels::Pixels = [255, 0, 0, 255].repeat(80 * 40).into();
    let pixels: crate::pixels::Pixels = [0, 255, 0, 255].repeat(160 * 80).into();
    let natural = Image {
        width: 160,
        height: 80,
        pixels: pixels.clone(),
    };
    let editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (80, 40),
        base: preview,
        annotations: None,
        annotation_size: None,
    })
    .unwrap()
    // 当前恢复层只接受 ceil 与像素缓冲相同的逻辑尺寸；自然源/20×10 逻辑框由出口层另测。
    .with_restored_size(crate::crop_geometry::Size {
        width: 79.5,
        height: 39.75,
    })
    .unwrap();
    let output = frame(
        &natural,
        &editor,
        1.0,
        crate::projection_delivery::sender::Kind::Sticker,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!((output.width, output.height), (160, 80));
    let decoded = super::super::decode_png(output.png.to_vec());
    assert_eq!(decoded.pixels, pixels);
}
#[test]
fn projection_default_editor_logical_frame_uses_raster_coordinates_for_natural_crop() {
    let _com = Apartment::new().unwrap();
    let source = Image {
        width: 24,
        height: 16,
        pixels: [0, 0, 255, 255].repeat(24 * 16).into(),
    };
    let mut editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (12, 8),
        base: [0, 255, 0, 255].repeat(12 * 8).into(),
        annotations: None,
        annotation_size: None,
    })
    .unwrap();
    assert!(editor.full_logical_frame().is_none());
    editor
        .crop_to(crate::crop_geometry::Rect {
            x: 3.0,
            y: 2.0,
            width: 6.0,
            height: 4.0,
        })
        .unwrap();
    let output = frame(
        &source,
        &editor,
        1.0,
        crate::projection_delivery::sender::Kind::Sticker,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!((output.width, output.height), (12, 8));
    let decoded = super::super::decode_png(output.png.to_vec());
    assert_eq!((decoded.width, decoded.height), (12, 8));
    assert!(decoded
        .pixels
        .chunks_exact(4)
        .all(|pixel| pixel == [0, 0, 255, 255]));
}
