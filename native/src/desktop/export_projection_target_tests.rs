//! 目标纯外观的借用读取、失败关闭及真实 WIC 合成回归。
use super::*;
use serde_json::json;
#[path = "export_projection_composition_tests.rs"]
mod composition;
fn source(edit: Value) -> crate::session_source::Source {
    serde_json::to_vec(&json!({"stickers":[{"id":"target","w":32,"h":16,"imageEditState":edit}]}))
        .unwrap()
        .into()
}
#[test]
fn projection_target_reads_only_selected_appearance_and_skips_large_unknown_state() {
    let huge = "x".repeat(128 * 1024);
    let bytes = serde_json::to_vec(&json!({"stickers":[
        {"id":"upstream","imageEditState":{"borderWidth":9000}},
        {"id":"target","src":huge,"imageEditState":{"cornerRadius":4,"unknown":huge,
            "cropRect":{"x":99},"contentEraseStrokes":[{"invalid":"not an appearance operation"}]}}
    ]}))
    .unwrap()
    .into();
    assert_eq!(
        read(&bytes, "target").unwrap().edit,
        json!({"cornerRadius":4})
    );
    assert!(read(&bytes, "missing").is_err());
    assert_eq!(
        read(&source(Value::Null), "target").unwrap().edit,
        Value::Null
    );
}
#[test]
fn projection_target_keeps_fractional_hint_for_live_geometry_conversion() {
    let source = br#"{"stickers":[{"id":"target","x":-20.25,"y":10.125,"w":80.25,"h":60.125}]}"#
        .to_vec()
        .into();
    let target = read(&source, "target").unwrap();
    let hint = target.hint.unwrap();
    let space = crate::session_placement::Space {
        origin: (-1920, -200),
        scale: 1.5,
    };
    let geometry = Geometry::Window {
        frame: space.physical(hint).unwrap(),
        space,
    };
    let size = geometry.logical(target.hint).unwrap();
    assert_eq!((size.width, size.height), (80.25, 60.125));
}
#[test]
fn projection_target_rejects_ambiguous_invalid_and_unbounded_known_appearance() {
    for raw in [
        r#"{"stickers":[{"id":"target"},{"id":"target"}]}"#,
        r#"{"stickers":[{"id":"target","imageEditState":false}]}"#,
        r#"{"stickers":[{"id":"target","imageEditState":{"borderWidth":1,"borderWidth":2}}]}"#,
    ] {
        assert!(read(&raw.as_bytes().to_vec().into(), "target").is_err());
    }
    assert!(read(
        &source(json!({"beautify":{"unknown":"x".repeat(16 * 1024)}})),
        "target"
    )
    .is_err());
    let raw = json!({"stickers": (0..=crate::workspace::MAX_UNITS).map(|n| json!({"id":n.to_string()})).collect::<Vec<_>>()});
    assert!(read(&serde_json::to_vec(&raw).unwrap().into(), "target").is_err());
}
#[test]
fn projection_target_rounding_clips_overlay_and_border_is_not_faded_with_base() {
    let _com = Apartment::new().unwrap();
    let image = Image {
        width: 32,
        height: 16,
        pixels: [255, 0, 0, 255].repeat(32 * 16).into(),
    };
    let editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (32, 16),
        base: image.pixels.clone(),
        annotation_size: None,
        annotations: Some([0, 255, 255, 255].repeat(32 * 16).into()),
    })
    .unwrap();
    let output = target_frame(
        &source(json!({"cornerRadius":8})),
        "target",
        &image,
        &editor,
        Presentation {
            opacity: 0.0,
            kind: crate::projection_delivery::sender::Kind::Sticker,
            geometry: None,
        },
        &AtomicBool::new(false),
    )
    .unwrap();
    let decoded = super::super::super::decode_png(output.png.to_vec());
    assert_eq!(&decoded.pixels[..4], &[0, 0, 0, 0]);
    assert_eq!(
        &decoded.pixels[(8 * 32 + 16) * 4..(8 * 32 + 16) * 4 + 4],
        &[0, 255, 255, 255]
    );
    let output = target_frame(
        &source(json!({"borderWidth":2,"borderColor":"#ff0000"})),
        "target",
        &image,
        &Editor::default(),
        Presentation {
            opacity: 0.0,
            kind: crate::projection_delivery::sender::Kind::Art,
            geometry: None,
        },
        &AtomicBool::new(false),
    )
    .unwrap();
    let decoded = super::super::super::decode_png(output.png.to_vec());
    assert_eq!(
        &decoded.pixels[(8 * 32) * 4..(8 * 32) * 4 + 4],
        &[0, 0, 255, 255]
    );
    assert_eq!(
        &decoded.pixels[(8 * 32 + 16) * 4..(8 * 32 + 16) * 4 + 4],
        &[0, 0, 0, 0]
    );
}
#[test]
fn projection_target_without_local_raster_uses_own_logical_frame_not_natural_pixels() {
    let _com = Apartment::new().unwrap();
    let image = Image {
        width: 64,
        height: 32,
        pixels: vec![255; 64 * 32 * 4].into(),
    };
    let output = target_frame(
        &source(json!({"borderWidth":2,"borderColor":"#ff0000"})),
        "target",
        &image,
        &Editor::default(),
        Presentation {
            opacity: 0.0,
            kind: crate::projection_delivery::sender::Kind::Art,
            geometry: None,
        },
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!((output.width, output.height), (64, 32));
    let decoded = super::super::super::decode_png(output.png.to_vec());
    let pixel = |x: usize| &decoded.pixels[(16 * 64 + x) * 4..(16 * 64 + x) * 4 + 4];
    assert_eq!(
        pixel(2),
        &[0, 0, 255, 255],
        "2 logical border pixels become 4 natural pixels"
    );
    assert_eq!(pixel(5), &[0, 0, 0, 0]);
    let bytes = br#"{"stickers":[{"id":"target","w":8,"h":4,"minified":true,"savedRect":{"x":0,"y":0,"w":32,"h":16}}]}"#;
    let logical = read(&bytes.to_vec().into(), "target")
        .unwrap()
        .logical
        .unwrap();
    assert_eq!((logical.width, logical.height), (32.0, 16.0));
}
#[test]
fn projection_target_checks_final_beautify_size_and_invalid_styles_before_encoding() {
    let image = Image {
        width: 1,
        height: 1,
        pixels: vec![255; 4].into(),
    };
    for edit in [
        json!({"cornerRadius":-1}),
        json!({"borderWidth":"2"}),
        json!({"beautify":{"enabled":true,"padding":4096}}),
    ] {
        assert!(target_frame(
            &source(edit),
            "target",
            &image,
            &Editor::default(),
            Presentation {
                opacity: 1.0,
                kind: crate::projection_delivery::sender::Kind::Art,
                geometry: None
            },
            &AtomicBool::new(false)
        )
        .is_err());
    }
}
