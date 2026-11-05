//! legacy子矩形先按目标密度物化再擦除；独立4×4子像素oracle及多笔全局预算。
use super::*;

#[test]
fn projection_mapped_history_keeps_two_strokes_in_the_same_space_after_region_materialization() {
    let _com = Apartment::new().unwrap();
    let natural = image(8, 8);
    let edit = json!({"cropRect":{"x":2,"y":0,"w":2,"h":8},"contentEraseStrokes":[
        {"width":2,"points":[{"x":2.5,"y":2.5}]},
        {"width":2,"points":[{"x":6.5,"y":6.5}]}]});
    let old = record("upstream", 8.0, 8.0, edit.clone(), json!(null));
    let target = record("target", 8.0, 8.0, json!({}), json!(null));
    let output = render(
        &target,
        &natural,
        &Editor::default(),
        Input {
            baseline: Some(baseline(old)),
            historical_erase: true,
            content: true,
            legacy_crop: true,
            ..Input::default()
        },
    );
    assert_eq!((output.width, output.height), (8, 8));
    for y in 0..8usize {
        for x in 0..8usize {
            let mut removed = 0;
            for sy in 0..4 {
                for sx in 0..4 {
                    let px = x as f64 + (sx as f64 + 0.5) / 4.0;
                    let py = y as f64 + (sy as f64 + 0.5) / 4.0;
                    if [(2.5, 2.5), (6.5, 6.5)]
                        .iter()
                        .any(|(cx, cy)| (px - cx).powi(2) + (py - cy).powi(2) <= 1.0)
                    {
                        removed += 1;
                    }
                }
            }
            for (actual, base) in pixel(&output, x, y).iter().zip([20u32, 40, 60, 255]) {
                let expected = ((base * (16 - removed) + 8) / 16) as u8;
                assert!(
                    actual.abs_diff(expected) <= 1,
                    "{x},{y}: {actual} vs {expected}"
                );
            }
        }
    }
    assert!(natural
        .pixels
        .chunks_exact(4)
        .all(|p| p == [20, 40, 60, 255]));
}

#[test]
fn projection_history_rejects_global_point_and_stroke_overflow_without_source_mutation() {
    let natural = image(4, 4);
    let points = vec![json!({"x":100,"y":100}); 32_769];
    for strokes in [
        json!([{"width":2,"points":points},{"width":2,"points":points}]),
        json!(vec![json!({"points":[]}); 4_097]),
    ] {
        let source = record(
            "target",
            4.0,
            4.0,
            json!({"contentEraseStrokes":strokes}),
            json!(null),
        );
        let editor = Editor::default();
        let stamp = editor.document_stamp();
        let error = target_frame_with_input(
            &source,
            "target",
            &natural,
            &editor,
            Presentation {
                opacity: 1.0,
                kind: crate::projection_delivery::sender::Kind::Sticker,
                geometry: None,
            },
            Input {
                historical_erase: true,
                ..Input::default()
            },
            &AtomicBool::new(false),
        )
        .err()
        .unwrap();
        assert_eq!(error.code(), E_INVALIDARG);
        assert!(stamp.matches(&editor));
        assert!(natural
            .pixels
            .chunks_exact(4)
            .all(|p| p == [20, 40, 60, 255]));
    }
}
