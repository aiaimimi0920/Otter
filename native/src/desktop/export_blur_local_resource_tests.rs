//! 生产局部准备路径的资源尺寸、缓存和失败原子性；不保留旧整框实现作为测试替身。
use super::*;

#[test]
fn local_blur_convolves_the_smoothed_projection_not_nearest_pixels() {
    let image = Image {
        width: 2,
        height: 9,
        pixels: [0, 0, 0, 255, 255, 255, 255, 255].repeat(9).into(),
    };
    let mut parent = Layout::for_frame(
        &image,
        &Size {
            width: 2.0,
            height: 9.0,
        }
        .into(),
        None,
    )
    .unwrap();
    parent.output = (8, 9);
    let cancel = AtomicBool::new(false);
    let frame = Rect {
        x: 0.0,
        y: 0.0,
        width: 2.0,
        height: 9.0,
    };
    let blur = local::prepare(&image, &parent, frame, None, 1.0, &cancel).unwrap();
    let reference = |values: &[u8]| {
        let bytes = values
            .iter()
            .flat_map(|v| [*v, *v, *v, 255])
            .collect::<Vec<_>>()
            .repeat(9);
        crate::blur::region_scaled(
            &bytes.into(),
            (8, 9),
            crate::blur::Region {
                x: 0,
                y: 0,
                width: 8,
                height: 9,
            },
            1.0,
            (4.0, 1.0),
            || false,
        )
        .unwrap()
    };
    let smooth = reference(&[0, 0, 32, 96, 159, 223, 255, 255]);
    let nearest = reference(&[0, 0, 0, 0, 255, 255, 255, 255]);
    for (actual, expected) in blur.pixels().iter().zip(smooth.iter()) {
        assert!(
            actual.abs_diff(*expected) <= 1,
            "blur did not use smooth source: {actual} vs {expected}"
        );
    }
    assert!(blur
        .pixels()
        .iter()
        .zip(nearest.iter())
        .any(|(a, b)| a.abs_diff(*b) > 1));
}

fn source() -> Image {
    Image {
        width: 16,
        height: 16,
        pixels: [20, 30, 40, 255].repeat(256).into(),
    }
}
fn effect(id: &str, width: f64, strength: f32) -> serde_json::Value {
    serde_json::json!({"id":id,"type":"blur","zIndex":0,"x":2,"y":2,"w":width,"h":4,
        "points":[{"x":4,"y":4}],"brushWidth":4,"strength":strength,"style":{"width":1,"color":"#fff"}})
}
fn document(elements: serde_json::Value) -> String {
    serde_json::json!({"id":"test","x":0,"y":0,"w":8,"h":8,
        "annotationState":{"serialCounter":0,"elements":elements}})
    .to_string()
}
fn restored(window: &crate::session_window::Window<'_>, image: &Image) -> Editor {
    crate::session_vectors::restore_with_resources(
        window.annotation_state,
        |_| Err(crate::session_vectors::Error::Unsupported),
        |strength| Ok(BlurImage::with_strength(16, 16, image.pixels.clone(), strength).unwrap()),
    )
    .unwrap()
    .editor
}

#[test]
fn identical_local_boxes_share_rounded_resources_without_quantizing_originals() {
    let _com = Apartment::new().unwrap();
    let image = source();
    let json = document(serde_json::json!([
        effect("a", 4.0, 4.1),
        effect("b", 4.0, 4.4)
    ]));
    let window = serde_json::from_str(&json).unwrap();
    let editor = restored(&window, &image);
    let frame = Size {
        width: 8.0,
        height: 8.0,
    }
    .into();
    let prepared = prepare_restored(
        &image,
        &editor,
        &frame,
        &window,
        None,
        &AtomicBool::new(false),
    )
    .unwrap();
    let a = prepared.elements()[0].blur.as_ref().unwrap();
    let b = prepared.elements()[1].blur.as_ref().unwrap();
    assert_eq!((a.width(), a.height()), (8, 8));
    assert_eq!(
        a.frame(),
        Rect {
            x: 2.0,
            y: 2.0,
            width: 4.0,
            height: 4.0
        }
    );
    assert!(a.pixels().same_storage(b.pixels()));
    assert_eq!(a.strength(), 4.0);
    assert_eq!(editor.elements()[0].blur.as_ref().unwrap().strength(), 4.1);
    assert_eq!(editor.elements()[1].blur.as_ref().unwrap().strength(), 4.4);
    assert_eq!(editor.revision(), 0);
}

#[test]
fn later_local_allocation_failure_and_cancellation_leave_source_untouched() {
    let _com = Apartment::new().unwrap();
    let image = source();
    let mut oversized = effect("b", 8192.0, 4.0);
    oversized["h"] = serde_json::json!(8192);
    let json = document(serde_json::json!([effect("a", 4.0, 4.0), oversized]));
    let window = serde_json::from_str(&json).unwrap();
    let editor = restored(&window, &image);
    let frame = Size {
        width: 8.0,
        height: 8.0,
    }
    .into();
    let original = editor.elements()[0].blur.as_ref().unwrap().pixels().clone();
    for cancelled in [false, true] {
        assert!(prepare_restored(
            &image,
            &editor,
            &frame,
            &window,
            None,
            &AtomicBool::new(cancelled)
        )
        .is_err());
        assert!(editor.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .pixels()
            .same_storage(&original));
        assert_eq!(editor.revision(), 0);
    }
}

#[test]
fn local_texture_outside_source_has_empty_color_clip_but_keeps_wash_frame() {
    let _com = Apartment::new().unwrap();
    let image = source();
    let parent = Layout::for_frame(
        &image,
        &Size {
            width: 8.0,
            height: 8.0,
        }
        .into(),
        None,
    )
    .unwrap();
    let frame = Rect {
        x: 10.0,
        y: 2.0,
        width: 4.0,
        height: 4.0,
    };
    let prepared =
        local::prepare(&image, &parent, frame, None, 4.0, &AtomicBool::new(false)).unwrap();
    assert_eq!(prepared.frame(), frame);
    assert_eq!(prepared.effect_clip().unwrap().width, 0.0);
    assert!(prepared.pixels().iter().all(|byte| *byte == 0));
}

#[test]
fn fractional_local_mask_scale_includes_canvas_and_output_rounding_per_axis() {
    let _com = Apartment::new().unwrap();
    let image = source();
    let parent = Layout::for_frame(
        &image,
        &Size {
            width: 12.0,
            height: 16.0,
        }
        .into(),
        None,
    )
    .unwrap();
    // 输出 16×16，密度 4/3 与 1；逻辑局部框 ceil 为 2×3，再 ceil 到 3×3 像素。
    let frame = Rect {
        x: 2.0,
        y: 2.0,
        width: 1.25,
        height: 2.5,
    };
    let prepared =
        local::prepare(&image, &parent, frame, None, 4.0, &AtomicBool::new(false)).unwrap();
    assert_eq!((prepared.width(), prepared.height()), (3, 3));
    let scale = prepared.effect_mask_scale();
    assert!((scale.0 - (1.25 / 2.25) as f32).abs() < 1e-6);
    assert!((scale.1 - (2.5 / 3.0) as f32).abs() < 1e-6);
    for invalid in [(0.0, 1.0), (1.1, 1.0), (1.0, f32::NAN)] {
        assert!(prepared.clone().with_effect_mask_scale(invalid).is_none());
    }
}
