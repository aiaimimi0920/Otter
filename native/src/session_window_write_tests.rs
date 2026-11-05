use super::*;
fn space() -> Space {
    Space {
        origin: (-1920, -200),
        scale: 1.5,
    }
}
fn original() -> Value {
    json!({"id":"u","x":10.25,"y":20.125,"w":100.25,"h":60.125,
    "extension":{"keep":true},"src":"opaque","params":{"secret":"retained"}})
}
#[test]
fn normal_window_preserves_fractional_geometry_and_unowned_fields() {
    let old = original();
    let frame = space().physical(rect(&old).unwrap()).unwrap();
    let saved = merge(
        old.clone(),
        Geometry {
            frame,
            expanded: None,
            minified: false,
            normal_opacity: 0.4,
            mini_opacity: 0.8,
        },
        space(),
    )
    .unwrap();
    assert_eq!(rect(&saved), rect(&old));
    for key in ["extension", "src", "params", "id"] {
        assert_eq!(saved[key], old[key]);
    }
    let text = saved.to_string();
    let window: crate::session_window::Window<'_> = serde_json::from_str(&text).unwrap();
    assert!(window.valid());
    assert!(!window.is_minified());
    assert_eq!(space().physical(window.rect()).unwrap(), frame);
}
#[test]
fn moved_minified_window_reopens_and_expands_to_the_same_physical_frames() {
    let old = json!({"id":"u","x":30.25,"y":40.125,"w":20,"h":20,"minified":true,
        "savedRect":{"x":10.25,"y":20.125,"w":100.25,"h":60.125},"cropOffset":{"x":20,"y":20}});
    let initial = space().physical(rect(&old).unwrap()).unwrap();
    let full = space().physical(rect(&old["savedRect"]).unwrap()).unwrap();
    let current = PinRect {
        x: initial.x + 17,
        y: initial.y - 9,
        ..initial
    };
    let expanded = PinRect {
        x: full.x + 17,
        y: full.y - 9,
        ..full
    };
    let saved = merge(
        old,
        Geometry {
            frame: current,
            expanded: Some(expanded),
            minified: true,
            normal_opacity: 1.0,
            mini_opacity: 0.3,
        },
        space(),
    )
    .unwrap();
    let text = saved.to_string();
    let window: crate::session_window::Window<'_> = serde_json::from_str(&text).unwrap();
    assert!(window.valid());
    assert!(window.is_minified());
    assert_eq!(space().physical(window.rect()).unwrap(), current);
    assert_eq!(
        space().physical(window.expanded_rect().unwrap()).unwrap(),
        expanded
    );
}
#[test]
fn missing_expanded_frame_does_not_resurrect_old_saved_geometry() {
    let mut old = original();
    old["savedRect"] = json!({"x":0,"y":0,"w":10,"h":10});
    let frame = space().physical(rect(&old).unwrap()).unwrap();
    let state = || Geometry {
        frame,
        expanded: None,
        minified: true,
        normal_opacity: 1.0,
        mini_opacity: 0.9,
    };
    let saved = merge(old.clone(), state(), space()).unwrap();
    assert!(saved.get("savedRect").is_none());
    assert!(saved.get("cropOffset").is_none());
    assert!(merge(
        old,
        Geometry {
            normal_opacity: f32::NAN,
            ..state()
        },
        space()
    )
    .is_err());
}

#[test]
fn absent_saved_rect_keeps_view_offset_and_moves_image_storage_without_copy() {
    let mut old = original();
    old["src"] = Value::String("x".repeat(1024));
    old["cropOffset"] = json!({"x":12.25,"y":-4});
    let storage = old["src"].as_str().unwrap().as_ptr();
    let frame = space().physical(rect(&old).unwrap()).unwrap();
    let saved = merge(
        old,
        Geometry {
            frame,
            expanded: None,
            minified: true,
            normal_opacity: 1.0,
            mini_opacity: 0.9,
        },
        space(),
    )
    .unwrap();
    assert_eq!(saved["cropOffset"], json!({"x":12.25,"y":-4}));
    assert_eq!(saved["src"].as_str().unwrap().as_ptr(), storage);
    assert!(saved.get("savedRect").is_none());
}
