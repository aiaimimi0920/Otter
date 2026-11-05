//! 用可辨认像素和独立坐标oracle验证待应用delta，禁止复活canonical旧裁剪。
use super::*;
use crate::crop_geometry::{Rect, Size};
use serde_json::json;
#[path = "export_projection_legacy_tests.rs"]
mod legacy;

fn patterned() -> Image {
    Image {
        width: 8,
        height: 8,
        pixels: (0..64)
            .flat_map(|i| [((i % 8) * 24) as u8, ((i / 8) * 24) as u8, 32, 255])
            .collect::<Vec<_>>()
            .into(),
    }
}
fn editor(image: &Image) -> Editor {
    Editor::with_layers(crate::erase_layers::Layers {
        size: (image.width, image.height),
        base: image.pixels.clone(),
        annotations: None,
        annotation_size: None,
    })
    .unwrap()
}
fn source(
    edit: serde_json::Value,
    propagation: serde_json::Value,
) -> crate::session_source::Source {
    serde_json::to_vec(&json!({"stickers":[{"id":"target","w":8,"h":8,
        "imageEditState":edit,"stickerEditPropagation":propagation}]}))
    .unwrap()
    .into()
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

#[test]
fn projection_current_crop_flip_and_history_erase_are_composed_once_without_source_mutation() {
    let _com = Apartment::new().unwrap();
    let image = patterned();
    let original = image.pixels.clone();
    let mut editor = editor(&image);
    editor
        .crop_to(Rect {
            x: 2.0,
            y: 2.0,
            width: 4.0,
            height: 4.0,
        })
        .unwrap();
    let source = source(
        json!({"flippedX":true,"cropRect":{"x":0,"y":0,"w":1,"h":1},
        "contentEraseStrokes":[{"width":2,"points":[{"x":1.5,"y":1.5}]}]}),
        json!(null),
    );
    let before = editor.document_stamp();
    let output = render(
        &source,
        &image,
        &editor,
        Input {
            flip: (true, false),
            content: true,
            historical_erase: true,
            ..Input::default()
        },
    );
    assert_eq!((output.width, output.height), (4, 4));
    assert_eq!(&output.pixels[60..64], &[48, 120, 32, 255]);
    assert_eq!(output.pixels[(4 + 1) * 4 + 3], 0);
    assert_eq!(image.pixels, original);
    assert!(before.matches(&editor));
}

#[test]
fn projection_reset_and_materialized_pixels_ignore_stale_crop_flip_and_erase() {
    let _com = Apartment::new().unwrap();
    let image = patterned();
    let mut editor = editor(&image);
    editor
        .crop_to(Rect {
            x: 2.0,
            y: 2.0,
            width: 4.0,
            height: 4.0,
        })
        .unwrap();
    assert!(editor.reset_crop());
    let source = source(
        json!({"flippedX":true,"cropRect":{"x":2,"y":2,"w":4,"h":4},
        "sourceSize":{"w":8,"h":8},"contentEraseStrokes":[{"invalid":true}]}),
        json!(null),
    );
    let output = render(&source, &image, &editor, Input::default());
    assert_eq!(output.pixels, image.pixels);
    assert_eq!((output.width, output.height), (8, 8));
    assert!(editor.undo());
    let output = render(&source, &image, &editor, Input::default());
    assert_eq!((output.width, output.height), (4, 4));
    assert_eq!(&output.pixels[..4], &[48, 48, 32, 255]);
}

#[test]
fn projection_nonuniform_propagation_keeps_natural_output_and_mirrors_only_pending_placement() {
    let _com = Apartment::new().unwrap();
    let image = Image {
        width: 8,
        height: 8,
        pixels: [10, 20, 30, 255].repeat(64).into(),
    };
    for flipped in [false, true] {
        let source = source(
            json!({"flippedX":flipped}),
            json!({
            "upstreamSourceFrame":{"w":8,"h":4},
            "upstreamContentFrame":{"x":0,"y":0,"w":4,"h":4}}),
        );
        let output = render(
            &source,
            &image,
            &Editor::default(),
            Input {
                logical: Some(Size {
                    width: 16.0,
                    height: 4.0,
                }),
                content: true,
                ..Input::default()
            },
        );
        assert_eq!((output.width, output.height), (8, 8));
        for y in 0..8 {
            for x in 0..8 {
                let inside = if flipped {
                    (4..6).contains(&x)
                } else {
                    (2..4).contains(&x)
                };
                let expected = if inside { [10, 20, 30, 255] } else { [0; 4] };
                let at = (y * 8 + x) * 4;
                assert_eq!(&output.pixels[at..at + 4], &expected, "{flipped}: {x},{y}");
            }
        }
    }
}

#[test]
fn projection_legacy_crop_is_explicit_and_cannot_override_a_current_modern_crop() {
    let _com = Apartment::new().unwrap();
    let image = patterned();
    let source = source(json!({"cropRect":{"x":2,"y":0,"w":2,"h":8}}), json!(null));
    let output = render(
        &source,
        &image,
        &Editor::default(),
        Input {
            content: true,
            legacy_crop: true,
            ..Input::default()
        },
    );
    for y in 0..8 {
        for x in 0..8 {
            let at = (y * 8 + x) * 4;
            assert_eq!(
                &output.pixels[at..at + 4],
                &[((2 + x / 4) * 24) as u8, (y * 24) as u8, 32, 255]
            );
        }
    }
    let mut current = editor(&image);
    current
        .crop_to(Rect {
            x: 0.0,
            y: 0.0,
            width: 4.0,
            height: 4.0,
        })
        .unwrap();
    let output = render(
        &source,
        &image,
        &current,
        Input {
            content: true,
            legacy_crop: true,
            ..Input::default()
        },
    );
    assert_eq!(&output.pixels[..4], &[0, 0, 32, 255]);
}

#[test]
fn projection_invalid_history_and_cancel_leave_source_and_editor_unchanged() {
    let image = patterned();
    let editor = editor(&image);
    let stamp = editor.document_stamp();
    for edit in [
        json!({"contentEraseStrokes":false}),
        json!({"contentEraseStrokes":[{"width":2,"points":[{"x":"bad","y":1}]}]}),
    ] {
        assert!(target_frame_with_input(
            &source(edit, json!(null)),
            "target",
            &image,
            &editor,
            Presentation {
                opacity: 1.0,
                kind: crate::projection_delivery::sender::Kind::Sticker,
                geometry: None
            },
            Input {
                historical_erase: true,
                ..Input::default()
            },
            &AtomicBool::new(false)
        )
        .is_err());
    }
    assert!(target_frame_with_input(
        &source(json!({}), json!(null)),
        "target",
        &image,
        &editor,
        Presentation {
            opacity: 1.0,
            kind: crate::projection_delivery::sender::Kind::Sticker,
            geometry: None
        },
        Input::default(),
        &AtomicBool::new(true)
    )
    .is_err());
    assert!(stamp.matches(&editor));
    assert_eq!(image.pixels, patterned().pixels);
}
