//! 来源历史始终绑定冻结原文；独立像素坐标验证跨Unit、保存后朝向及当前crop/reset。
use super::*;
use crate::crop_geometry::{Rect, Size};
use serde_json::json;
#[path = "export_projection_history_tests.rs"]
mod history;

fn record(
    unit: &str,
    width: f64,
    height: f64,
    edit: serde_json::Value,
    propagation: serde_json::Value,
) -> crate::session_source::Source {
    serde_json::to_vec(
        &json!({"stickers":[{"id":unit,"x":0,"y":0,"w":width,"h":height,
        "imageEditState":edit,"stickerEditPropagation":propagation}]}),
    )
    .unwrap()
    .into()
}
fn baseline(record: crate::session_source::Source) -> Baseline {
    Baseline {
        record,
        unit: crate::workspace_id::UnitId::parse("upstream").unwrap(),
    }
}
fn image(width: u32, height: u32) -> Image {
    Image {
        width,
        height,
        pixels: [20, 40, 60, 255].repeat((width * height) as usize).into(),
    }
}
fn render(
    source: &crate::session_source::Source,
    image: &Image,
    editor: &Editor,
    input: Input,
) -> Image {
    let output = target_frame_with_input(
        source,
        "target",
        image,
        editor,
        Presentation {
            opacity: 1.0,
            kind: crate::projection_delivery::sender::Kind::Sticker,
            geometry: None,
        },
        input,
        &AtomicBool::new(false),
    )
    .unwrap();
    let decoded = crate::desktop::export::decode_png(output.png.to_vec());
    assert_eq!(
        (decoded.width, decoded.height),
        (output.width, output.height)
    );
    decoded
}
fn pixel(image: &Image, x: usize, y: usize) -> &[u8] {
    let start = (y * image.width as usize + x) * 4;
    &image.pixels[start..start + 4]
}

#[test]
fn projection_rasterized_source_preserves_padding_for_crop_propagation_and_reset() {
    let _com = Apartment::new().unwrap();
    let mut source = image(8, 8);
    let mut pixels = source.pixels.to_vec();
    for row in pixels.chunks_exact_mut(32) {
        row[..8].fill(0);
    }
    source.pixels = pixels.into();
    let logical = Size {
        width: 16.0,
        height: 8.0,
    };
    let mut editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (16, 8),
        base: vec![0; 16 * 8 * 4].into(),
        annotations: None,
        annotation_size: None,
    })
    .unwrap()
    .with_restored_crop(crate::crop_geometry::Crop {
        source: logical,
        rect: Rect {
            x: 2.0,
            y: 2.0,
            width: 8.0,
            height: 4.0,
        },
    })
    .unwrap();
    let target = record(
        "target",
        8.0,
        4.0,
        json!(null),
        json!({
            "upstreamSourceFrame":{"w":8,"h":4},"upstreamContentFrame":{"x":4,"y":0,"w":4,"h":4}
        }),
    );
    let mut input = Input {
        source_logical: Some(logical),
        rasterized_source: true,
        ..Input::default()
    };
    let plain = render(&target, &source, &editor, input.clone());
    assert_eq!((plain.width, plain.height), (4, 4));
    assert_eq!(pixel(&plain, 0, 2), &[0; 4]);
    assert_eq!(pixel(&plain, 1, 2), &[20, 40, 60, 255]);
    input.content = true;
    let propagated = render(&target, &source, &editor, input.clone());
    assert_eq!((propagated.width, propagated.height), (4, 4));
    assert_eq!(pixel(&propagated, 1, 2), &[0; 4]);
    assert_eq!(pixel(&propagated, 3, 2), &[20, 40, 60, 255]);
    assert!(editor.reset_crop());
    input.content = false;
    let reset = render(&target, &source, &editor, input);
    assert_eq!(reset.pixels, source.pixels);
}

#[test]
fn projection_high_zoom_crop_keeps_natural_png_density() {
    let _com = Apartment::new().unwrap();
    let mut natural = image(4096, 8);
    let mut pixels = natural.pixels.to_vec();
    for row in pixels.chunks_exact_mut(4096 * 4) {
        row[64 * 4..128 * 4].copy_from_slice(&[90, 100, 110, 255].repeat(64));
    }
    natural.pixels = pixels.into();
    let logical = Size {
        width: 32_768.0,
        height: 128.0,
    };
    let size = logical.crop_raster_size().unwrap();
    let editor = Editor::with_layers(crate::erase_layers::Layers {
        size,
        base: vec![0; crate::viewport::checked_image_bytes(size.0, size.1).unwrap()].into(),
        annotations: None,
        annotation_size: None,
    })
    .unwrap()
    .with_restored_crop(crate::crop_geometry::Crop {
        source: logical,
        rect: Rect {
            x: 512.0,
            // The natural image occupies y=32..96 after contain.
            y: 32.0,
            width: 512.0,
            height: 48.0,
        },
    })
    .unwrap();
    let target = record("target", 512.0, 48.0, json!(null), json!(null));
    let decoded = render(
        &target,
        &natural,
        &editor,
        Input {
            source_logical: Some(Size {
                width: 4096.0,
                height: 8.0,
            }),
            content: true,
            ..Input::default()
        },
    );
    assert_eq!((decoded.width, decoded.height), (64, 3));
    for (index, pixel) in decoded.pixels.chunks_exact(4).enumerate() {
        assert_eq!(pixel, [90, 100, 110, 255], "pixel {index}");
    }
}

#[test]
fn projection_source_logical_preserves_nonuniform_crop_density_and_reset_undo() {
    let _com = Apartment::new().unwrap();
    let natural = image(16, 16);
    let mut editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (16, 8),
        base: vec![0; 16 * 8 * 4].into(),
        annotations: None,
        annotation_size: None,
    })
    .unwrap()
    .with_restored_size(Size {
        width: 16.0,
        height: 8.0,
    })
    .unwrap();
    editor
        .crop_to(Rect {
            x: 4.0,
            y: 2.0,
            width: 8.0,
            height: 4.0,
        })
        .unwrap();
    let target = record(
        "target",
        8.0,
        4.0,
        json!({"cropRect":{"x":0,"y":0,"w":1,"h":1},
        "sourceSize":{"w":8,"h":8}}),
        json!(null),
    );
    let input = Input {
        source_logical: Some(Size {
            width: 8.0,
            height: 8.0,
        }),
        content: true,
        ..Input::default()
    };
    let cropped = render(&target, &natural, &editor, input.clone());
    assert_eq!((cropped.width, cropped.height), (8, 8));
    assert!(cropped
        .pixels
        .chunks_exact(4)
        .all(|p| p == [20, 40, 60, 255]));
    assert!(editor.reset_crop());
    let reset = render(&target, &natural, &editor, input.clone());
    assert_eq!((reset.width, reset.height), (16, 16));
    assert_eq!(pixel(&reset, 0, 8), &[0; 4]);
    assert_eq!(pixel(&reset, 8, 8), &[20, 40, 60, 255]);
    assert!(editor.undo());
    assert_eq!(
        render(&target, &natural, &editor, input).pixels,
        cropped.pixels
    );
}

#[test]
fn projection_cross_unit_history_uses_source_visible_space_not_target_canonical() {
    let _com = Apartment::new().unwrap();
    let natural = image(16, 8);
    let old = record(
        "upstream",
        8.0,
        4.0,
        json!({"contentEraseStrokes":[
        {"width":2,"points":[{"x":2.25,"y":2.25}]}]}),
        json!(null),
    );
    let target = record(
        "target",
        16.0,
        4.0,
        json!({"flippedX":true,"contentEraseStrokes":false}),
        json!(null),
    );
    let result = render(
        &target,
        &natural,
        &Editor::default(),
        Input {
            logical: Some(Size {
                width: 16.0,
                height: 4.0,
            }),
            baseline: Some(baseline(old)),
            historical_erase: true,
            content: true,
            ..Input::default()
        },
    );
    assert_eq!((result.width, result.height), (16, 8));
    assert_eq!(pixel(&result, 2, 4), &[0; 4]);
    assert_eq!(
        pixel(&result, 6, 4),
        &[0; 4],
        "source x=2.25 maps inside target x=6, not x=2.25"
    );
    assert_eq!(pixel(&result, 10, 4), &[20, 40, 60, 255]);
    assert!(natural
        .pixels
        .chunks_exact(4)
        .all(|p| p == [20, 40, 60, 255]));
}

#[test]
fn projection_natural_layout_orientation_survives_new_canonical_save() {
    let _com = Apartment::new().unwrap();
    let natural = image(8, 8);
    let old = record("upstream", 8.0, 8.0, json!({"flippedX":true}), json!(null));
    let target = record(
        "target",
        16.0,
        4.0,
        json!({"flippedX":false}),
        json!({
        "upstreamSourceFrame":{"w":8,"h":4},"upstreamContentFrame":{"x":0,"y":0,"w":4,"h":4}}),
    );
    let output = render(
        &target,
        &natural,
        &Editor::default(),
        Input {
            logical: Some(Size {
                width: 16.0,
                height: 4.0,
            }),
            baseline: Some(baseline(old)),
            content: true,
            ..Input::default()
        },
    );
    for y in 0..8 {
        for x in 0..8 {
            assert_eq!(
                pixel(&output, x, y),
                if (4..6).contains(&x) {
                    &[20, 40, 60, 255]
                } else {
                    &[0; 4]
                }
            );
        }
    }
}

#[test]
fn projection_source_history_crop_is_not_reinstalled_after_current_reset() {
    let _com = Apartment::new().unwrap();
    let natural = image(16, 8);
    let old = record(
        "upstream",
        4.0,
        4.0,
        json!({"cropRect":{"x":2,"y":0,"w":4,"h":4},
        "sourceSize":{"w":8,"h":4},"contentEraseStrokes":[
            {"width":2,"points":[{"x":1.25,"y":2.25}]}]}),
        json!(null),
    );
    let target = record("target", 8.0, 4.0, json!({}), json!(null));
    let output = render(
        &target,
        &natural,
        &Editor::default(),
        Input {
            logical: Some(Size {
                width: 8.0,
                height: 4.0,
            }),
            baseline: Some(baseline(old)),
            historical_erase: true,
            content: true,
            ..Input::default()
        },
    );
    assert_eq!((output.width, output.height), (16, 8));
    assert_eq!(pixel(&output, 6, 4), &[0; 4]);
    assert_eq!(pixel(&output, 0, 0), &[20, 40, 60, 255]);
    assert_eq!(pixel(&output, 14, 4), &[20, 40, 60, 255]);
}

#[test]
fn projection_missing_natural_baseline_unit_cannot_fall_back_to_target_history() {
    let natural = image(4, 4);
    let target = record("target", 4.0, 4.0, json!({}), json!(null));
    let error = target_frame_with_input(
        &target,
        "target",
        &natural,
        &Editor::default(),
        Presentation {
            opacity: 1.0,
            kind: crate::projection_delivery::sender::Kind::Sticker,
            geometry: None,
        },
        Input {
            baseline: Some(baseline(target.clone())),
            historical_erase: true,
            ..Input::default()
        },
        &AtomicBool::new(false),
    )
    .err()
    .unwrap();
    assert_eq!(error.code(), E_INVALIDARG);
    assert!(natural
        .pixels
        .chunks_exact(4)
        .all(|p| p == [20, 40, 60, 255]));
}

#[test]
fn projection_large_original_logical_source_does_not_expand_or_reject_small_natural_crop() {
    let _com = Apartment::new().unwrap();
    let natural = image(8, 8);
    let mut editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (8, 8),
        base: natural.pixels.clone(),
        annotations: None,
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
    let target = record("target", 4.0, 4.0, json!({}), json!(null));
    for original in [16_384.0, 1_000_000.0] {
        let output = render(
            &target,
            &natural,
            &editor,
            Input {
                source_logical: Some(Size {
                    width: original,
                    height: original,
                }),
                content: true,
                ..Input::default()
            },
        );
        // 原逻辑crop/source都同比增长，自然密度反比缩小；最终仍是原8×8图的一半。
        assert_eq!((output.width, output.height), (4, 4));
        assert!(output
            .pixels
            .chunks_exact(4)
            .all(|p| p == [20, 40, 60, 255]));
    }
    assert!(target_frame_with_input(
        &target,
        "target",
        &natural,
        &editor,
        Presentation {
            opacity: 1.0,
            kind: crate::projection_delivery::sender::Kind::Sticker,
            geometry: None,
        },
        Input {
            source_logical: Some(Size {
                width: 1_000_001.0,
                height: 8.0
            }),
            ..Input::default()
        },
        &AtomicBool::new(false)
    )
    .is_err());
}
