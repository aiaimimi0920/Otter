//! 独立矩形高斯oracle：传播/legacy取样与底图clip必须先于活动blur重建。
use super::*;
use serde_json::{json, Value};
#[path = "export_projection_legacy_blur_tests.rs"]
mod explicit_legacy;

fn logical() -> Size {
    Size {
        width: 8.0,
        height: 8.0,
    }
}
fn record(edit: Value, propagation: Value) -> crate::session_source::Source {
    serde_json::to_vec(&json!({"stickers":[{"id":"target","w":8,"h":8,
        "imageEditState":edit,"stickerEditPropagation":propagation}]}))
    .unwrap()
    .into()
}
fn editor() -> Editor {
    let mut editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (8, 8),
        base: vec![0; 8 * 8 * 4].into(),
        annotations: None,
        annotation_size: None,
    })
    .unwrap();
    editor.set_style(
        crate::stroke_style::StrokeStyle::new(64.0, crate::stroke_style::Color(0, 0, 0)).unwrap(),
    );
    editor
        .begin_blur(
            Point { x: 4.0, y: 4.0 },
            BlurImage::new(8, 8, vec![0; 8 * 8 * 4].into(), 1).unwrap(),
        )
        .unwrap();
    assert!(editor.commit());
    editor
}
fn prepare(
    image: &Image,
    editor: &Editor,
    source: &crate::session_source::Source,
    legacy: bool,
) -> BlurImage {
    let target = target::read(source, "target").unwrap();
    let geometry = target
        .content(
            image,
            editor,
            logical().into(),
            logical(),
            &Input {
                content: true,
                legacy_crop: legacy,
                ..Input::default()
            },
            None,
        )
        .unwrap();
    source_resolution::source_blur::prepare_active(
        image,
        editor,
        &geometry,
        &AtomicBool::new(false),
    )
    .unwrap()
    .elements()[0]
        .blur
        .clone()
        .unwrap()
}
fn axis(p: usize, range: std::ops::Range<i32>) -> f64 {
    range.map(|q| weight(p as i32 - q, 1.0)).sum()
}
fn assert_rectangle(blur: &BlurImage, x: std::ops::Range<i32>, y: std::ops::Range<i32>) {
    assert_eq!((blur.width(), blur.height()), (8, 8));
    for row in 0..8 {
        for column in 0..8 {
            let expected = (255.0 * axis(column, x.clone()) * axis(row, y.clone())).round() as u8;
            for channel in 0..4 {
                let actual = blur.pixels()[(row * 8 + column) * 4 + channel];
                assert!(
                    actual.abs_diff(expected) <= 1,
                    "{column},{row},{channel}: {actual} vs {expected}"
                );
            }
        }
    }
}

#[test]
fn projection_active_blur_samples_propagated_content_not_a_fresh_full_image_contain() {
    let image = Image {
        width: 8,
        height: 8,
        pixels: vec![255; 8 * 8 * 4].into(),
    };
    let source = record(
        Value::Null,
        json!({"upstreamSourceFrame":{"w":8,"h":8},
        "upstreamContentFrame":{"x":4,"y":0,"w":4,"h":8}}),
    );
    assert_rectangle(&prepare(&image, &editor(), &source, false), 4..8, 2..6);
}

#[test]
fn projection_active_blur_current_crop_never_truncates_the_full_sampling_neighborhood() {
    let image = Image {
        width: 8,
        height: 8,
        pixels: vec![255; 8 * 8 * 4].into(),
    };
    let mut editor = editor();
    editor
        .crop_to(Rect {
            x: 2.0,
            y: 2.0,
            width: 4.0,
            height: 4.0,
        })
        .unwrap();
    let source = record(Value::Null, Value::Null);
    assert_rectangle(&prepare(&image, &editor, &source, false), 0..8, 0..8);
}

#[test]
fn projection_active_blur_respects_legacy_region_and_empty_sampling() {
    let image = Image {
        width: 8,
        height: 8,
        pixels: vec![255; 8 * 8 * 4].into(),
    };
    for (left, expected) in [(-4, 4..8), (100, 0..0)] {
        let source = record(
            json!({"cropRect":{"x":left,"y":0,"w":8,"h":8}}),
            Value::Null,
        );
        assert_rectangle(&prepare(&image, &editor(), &source, true), expected, 0..8);
    }
}

#[test]
fn projection_active_blur_keeps_base_content_clip_separate_from_raster_and_effect_clip() {
    let image = Image {
        width: 8,
        height: 8,
        pixels: [80, 0, 0, 255].repeat(64).into(),
    };
    let mut editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (8, 8),
        base: vec![0; 8 * 8 * 4].into(),
        annotations: Some([0, 120, 0, 128].repeat(64).into()),
        annotation_size: None,
    })
    .unwrap();
    editor
        .crop_to(Rect {
            x: 2.0,
            y: 2.0,
            width: 4.0,
            height: 4.0,
        })
        .unwrap();
    let blur = BlurImage::new(8, 8, vec![0; 8 * 8 * 4].into(), 1)
        .unwrap()
        .with_effect_clip(Rect {
            x: 3.0,
            y: 1.0,
            width: 2.0,
            height: 6.0,
        })
        .unwrap();
    editor.begin_blur(Point { x: 4.0, y: 4.0 }, blur).unwrap();
    assert!(editor.commit());
    let source = record(
        Value::Null,
        json!({"upstreamSourceFrame":{"w":4,"h":4},
        "upstreamContentFrame":{"x":0,"y":0,"w":2,"h":4}}),
    );
    let blur = prepare(&image, &editor, &source, false);
    for y in 0..8 {
        for x in 0..8 {
            let vertical = axis(y, 2..6);
            let left = weight(x as i32 - 3, 1.0);
            let right = weight(x as i32 - 4, 1.0);
            let expected = [
                40.0 * left,
                120.0 * (left + right),
                0.0,
                255.0 * left + 128.0 * right,
            ];
            for (channel, value) in expected.iter().enumerate() {
                let actual = blur.pixels()[(y * 8 + x) * 4 + channel];
                assert!(
                    actual.abs_diff((value * vertical).round() as u8) <= 1,
                    "{x},{y},{channel}"
                );
            }
        }
    }
}

#[test]
fn projection_pipeline_resolves_content_before_blur_and_applies_final_wash_only_once() {
    let _com = Apartment::new().unwrap();
    let image = Image {
        width: 8,
        height: 8,
        pixels: [255, 0, 0, 255].repeat(64).into(),
    };
    let source = record(
        Value::Null,
        json!({"upstreamSourceFrame":{"w":8,"h":8},
        "upstreamContentFrame":{"x":4,"y":0,"w":4,"h":8}}),
    );
    let output = target_frame_with_input(
        &source,
        "target",
        &image,
        &editor(),
        Presentation {
            opacity: 1.0,
            kind: crate::projection_delivery::sender::Kind::Sticker,
            geometry: None,
        },
        Input {
            content: true,
            ..Input::default()
        },
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!((output.width, output.height), (8, 8));
    let decoded = crate::desktop::export::decode_png(output.png.to_vec());
    // x=0距离内容至少4px，sigma=1的有限3px核无贡献；这里仅有原绘制器8%白wash。
    let pixel = &decoded.pixels[(4 * 8) * 4..(4 * 8) * 4 + 4];
    assert!(
        pixel.iter().all(|value| (19..=21).contains(value)),
        "{pixel:?}"
    );
}
