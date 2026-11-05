use super::*;
use crate::crop_geometry::Size;
fn crop() -> Crop {
    Crop {
        rect: Rect {
            x: 10.25,
            y: 5.5,
            width: 30.5,
            height: 15.25,
        },
        source: Size {
            width: 80.5,
            height: 40.25,
        },
    }
}
fn read(state: Value) -> Option<crate::session_crop::Restored> {
    let json =
        json!({"id":"u","x":-100.25,"y":20.5,"w":61,"h":30.5,"imageEditState":state}).to_string();
    let window: crate::session_window::Window = serde_json::from_str(&json).unwrap();
    crate::session_crop::restore(&window).unwrap()
}
#[test]
fn fractional_source_round_trips_without_using_window_or_pixel_dimensions() {
    let original = json!({"cropRect":{"x":0,"y":0,"w":1,"h":1,"future":123},
        "sourceSize":{"w":2,"h":2,"future":456}, "flippedX":true,
        "extension":{"x":999},"contentEraseStrokes":[{"keep":true}]});
    let saved = merge(Some(original.clone()), Some(crop())).unwrap();
    assert_eq!(saved["cropRect"]["future"], 123);
    assert_eq!(saved["sourceSize"]["future"], 456);
    for key in ["flippedX", "extension", "contentEraseStrokes"] {
        assert_eq!(saved[key], original[key]);
    }
    let restored = read(saved).unwrap();
    assert_eq!(restored.crop, crop());
    assert!(restored.flipped_x);
    let normalized = restored.editor_crop().unwrap();
    assert_eq!(normalized.rect.x, 20.5);
    assert_eq!(normalized.rect.y, 11.0);
    assert_eq!(normalized.source.width, 161.0);
}
#[test]
fn reset_removes_both_crop_fields_and_recrop_uses_new_source() {
    let first = merge(Some(json!({"extension":"kept"})), Some(crop())).unwrap();
    let reset = merge(Some(first), None).unwrap();
    assert!(reset.get("cropRect").is_none());
    assert!(reset.get("sourceSize").is_none());
    assert!(read(reset.clone()).is_none());
    let next = Crop {
        source: Size {
            width: 200.0,
            height: 100.0,
        },
        ..crop()
    };
    let saved = merge(Some(reset), Some(next)).unwrap();
    assert_eq!(saved["extension"], "kept");
    assert_eq!(read(saved).unwrap().crop, next);
}
#[test]
fn invalid_metadata_and_nonfinite_geometry_are_rejected() {
    assert_eq!(merge(Some(json!([])), Some(crop())), Err(Error::Shape));
    assert_eq!(
        merge(Some(json!({"cropRect":17})), Some(crop())),
        Err(Error::Shape)
    );
    for width in [0.0, -1.0, f64::NAN, f64::INFINITY, 1_000_001.0] {
        let mut bad = crop();
        bad.source.width = width;
        assert_eq!(merge(None, Some(bad)), Err(Error::Geometry));
    }
}
