use super::*;
use crate::crop_geometry::{Crop, Size};

fn crop() -> Crop {
    Crop {
        rect: Rect {
            x: 1.25,
            y: 0.5,
            width: 5.5,
            height: 3.25,
        },
        source: Size {
            width: 8.0,
            height: 6.0,
        },
    }
}
fn document() -> (Editor, Box<RawValue>) {
    let raw = RawValue::from_string(r##"{"extension":{"x":42},"serialCounter":0,"elements":[{"id":"box","zIndex":0,"type":"rect","x":-2,"y":1,"w":3,"h":2,"style":{"width":2,"color":"#000000"}},{"id":"path","zIndex":1,"type":"brush","points":[{"x":1,"y":2},{"x":3,"y":4}],"style":{"width":2,"color":"#000000"}}]}"##.into()).unwrap();
    let restored = crate::session_vectors::restore(Some(&raw)).unwrap();
    let editor = restored
        .editor
        .with_restored_offset(Point { x: 1.25, y: 0.5 })
        .unwrap()
        .with_restored_layers(crate::erase_layers::Layers {
            size: (8, 6),
            base: vec![255; 8 * 6 * 4].into(),
            annotation_size: None,
            annotations: None,
        })
        .unwrap()
        .with_restored_crop(crop())
        .unwrap();
    (editor, raw)
}

#[test]
fn cropped_round_trip_undoes_only_the_import_offset() {
    let (editor, raw) = document();
    let before = editor.document_stamp();
    let output = encode_visible(&editor, Some(&raw), |_| Err(Error::Unsupported)).unwrap();
    assert_eq!(output.frame, Some(crop().rect));
    assert_eq!(output.crop.unwrap().source, crop());
    let saved = RawValue::from_string(String::from_utf8(output.annotations).unwrap()).unwrap();
    let value: Value = serde_json::from_str(saved.get()).unwrap();
    assert_eq!(value["extension"]["x"], 42);
    assert_eq!(value["elements"][0]["x"], -2.0);
    let restored = crate::session_vectors::restore(Some(&saved))
        .unwrap()
        .editor
        .with_restored_offset(Point { x: 1.25, y: 0.5 })
        .unwrap();
    for (a, b) in editor.elements().iter().zip(restored.elements()) {
        assert_eq!(a.points, b.points);
        assert_eq!(a.annotation_id(), b.annotation_id());
    }
    assert!(before.matches(&editor));
}

#[test]
fn reset_crop_switches_output_origin_without_mutating_vectors() {
    let (mut editor, raw) = document();
    let source = editor.elements()[0].points.clone();
    assert!(editor.reset_crop());
    let output = encode_visible(&editor, Some(&raw), |_| Err(Error::Unsupported)).unwrap();
    assert!(output.crop.is_none());
    assert_eq!(
        output.frame.unwrap(),
        Rect {
            x: 0.0,
            y: 0.0,
            width: 8.0,
            height: 6.0
        }
    );
    let value: Value = serde_json::from_slice(&output.annotations).unwrap();
    assert_eq!(value["elements"][0]["x"], -0.75);
    assert_eq!(editor.elements()[0].points, source);
    assert!(editor.undo());
    let output = encode_visible(&editor, Some(&raw), |_| Err(Error::Unsupported)).unwrap();
    let value: Value = serde_json::from_slice(&output.annotations).unwrap();
    assert_eq!(value["elements"][0]["x"], -2.0);
}

#[test]
fn translation_keeps_extent_and_unknown_fields_and_rejects_overflow() {
    let mut value =
        json!({"x":10,"y":20,"w":30,"h":40,"extension":{"x":99},"points":[{"x":-3,"y":4}]});
    translate(&mut value, Point { x: 2.0, y: 3.0 }).unwrap();
    assert_eq!(value["x"], 8.0);
    assert_eq!(value["y"], 17.0);
    assert_eq!(value["w"], 30);
    assert_eq!(value["extension"]["x"], 99);
    assert_eq!(value["points"][0]["x"], -5.0);
    assert!(translate(
        &mut value,
        Point {
            x: -1_000_000.0,
            y: 0.0
        }
    )
    .is_err());
}
