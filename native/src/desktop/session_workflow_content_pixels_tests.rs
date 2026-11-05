//! 工作流实际 PNG 回归：传播留白、整框翻转、裁剪 clip 与目标坐标擦除。
use super::*;
#[path = "session_workflow_crop_projection_tests.rs"]
mod crop_projection;

fn render(edit: Value, propagation: Value) -> crate::desktop::image::Image {
    let mut pixels = Vec::new();
    for _ in 0..4 {
        pixels.extend([200, 0, 0, 255].repeat(4));
        pixels.extend([0, 200, 0, 255].repeat(4));
    }
    render_pixels(edit, propagation, pixels)
}
fn render_pixels(edit: Value, propagation: Value, pixels: Vec<u8>) -> crate::desktop::image::Image {
    let cancel = AtomicBool::new(false);
    let image = crate::desktop::image::Image {
        width: 8,
        height: 4,
        pixels: pixels.into(),
    };
    let png = crate::desktop::export::png_raw(&image, &cancel).unwrap();
    let url = crate::loom_base64::encode_png_data_url(&png, &cancel).unwrap();
    let saved = serde_json::to_vec(&json!({"documentRevision":1,"stickers":[{
        "id":"s","x":0,"y":0,"w":8,"h":4,"src":url.as_str(),
        "opacityNormal":0.5,"imageEditState":edit,"stickerEditPropagation":propagation
    }],"links":[]}))
    .unwrap();
    let candidate =
        VectorCandidate::prepare(prepare(saved, None, &cancel).unwrap(), &cancel).unwrap();
    let prepared = candidate.prepare_workflow_snapshot(&[], &cancel).unwrap();
    crate::desktop::session_image::inline(
        prepared.snapshot().value()["nodes"][0]["data"]["src"]
            .as_str()
            .unwrap(),
        &cancel,
    )
    .unwrap()
}
fn propagation(x: f64, y: f64, w: f64, h: f64) -> Value {
    // source frame 的真实 wire 形状只有 w/h，不要求不存在的 x/y。
    json!({"upstreamSourceFrame":{"w":8,"h":4},"upstreamContentFrame":{"x":x,"y":y,"w":w,"h":h}})
}
fn pixel(image: &crate::desktop::image::Image, x: u32, y: u32) -> &[u8] {
    let at = ((y * image.width + x) * 4) as usize;
    &image.pixels[at..at + 4]
}

#[test]
fn propagated_content_contains_twice_and_whole_frame_flip_moves_placement() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for flipped in [false, true] {
        let image = render(json!({"flippedX":flipped}), propagation(0.0, 0.0, 4.0, 4.0));
        assert_eq!((image.width, image.height), (8, 4));
        for y in 0..4 {
            for x in 0..8 {
                let occupied = (1..3).contains(&y) && if flipped { x >= 4 } else { x < 4 };
                assert_eq!(
                    pixel(&image, x, y)[3] > 120,
                    occupied,
                    "{flipped} ({x},{y})"
                );
            }
        }
        let x = if flipped { 4 } else { 0 };
        assert!(pixel(&image, x, 1)[usize::from(flipped)] > 80);
    }
    let image = render(json!({"flippedY":true}), propagation(0.0, 0.0, 4.0, 2.0));
    assert_eq!(pixel(&image, 1, 0)[3], 0);
    assert!(pixel(&image, 1, 3)[3] > 120);
}

#[test]
fn propagated_logical_crop_clips_only_base_and_keeps_natural_output_size() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for flipped in [false, true] {
        let image = render(
            json!({"cropRect":{"x":2,"y":0,"w":4,"h":4},"sourceSize":{"w":8,"h":4},"flippedX":flipped}),
            propagation(4.0, 0.0, 4.0, 4.0),
        );
        assert_eq!((image.width, image.height), (4, 4));
        for y in 0..4 {
            assert_eq!(pixel(&image, 0, y), [0; 4]);
            assert_eq!(pixel(&image, 1, y), [0; 4]);
            assert!(pixel(&image, 2, y)[usize::from(flipped)] > 80);
            assert!(pixel(&image, 3, y)[usize::from(!flipped)] > 80);
        }
    }
}

#[test]
fn nonuniform_crop_contains_in_viewport_instead_of_stretching_old_source() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let image = render(
        json!({"cropRect":{"x":2,"y":0,"w":4,"h":4},"sourceSize":{"w":8,"h":4}}),
        Value::Null,
    );
    assert_eq!((image.width, image.height), (4, 4));
    // viewport 16x4，8x4 图片居中于 x=4；裁剪 x=4，恰好保留整图，无拉伸。
    for y in 0..4 {
        assert!(pixel(&image, 0, y)[0] > 80);
        assert!(pixel(&image, 3, y)[1] > 80);
    }
}

#[test]
fn propagated_erase_uses_visible_coordinates_after_flip() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let image = render(
        json!({"flippedX":true,"contentEraseStrokes":[{"width":2,"points":[{"x":5,"y":2}]}]}),
        propagation(0.0, 0.0, 4.0, 4.0),
    );
    assert_eq!(pixel(&image, 4, 1)[3], 0);
    assert!(pixel(&image, 7, 1)[3] > 120);
}

#[test]
fn legacy_pixel_crop_samples_before_whole_frame_flip_without_changing_output_size() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for flipped in [false, true] {
        // 原图左半蓝色，右半绿色；只取左侧，不能对已翻转图误取左侧绿色。
        let image = render(
            json!({"cropRect":{"x":0,"y":0,"w":4,"h":4},"flippedX":flipped}),
            Value::Null,
        );
        assert_eq!((image.width, image.height), (8, 4));
        assert!(image
            .pixels
            .chunks_exact(4)
            .all(|p| p[0] > 80 && p[1] == 0 && p[3] > 120));
    }
}

#[test]
fn legacy_crop_moves_content_clip_with_both_flip_axes() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for (flip_x, flip_y) in [(false, false), (true, false), (false, true), (true, true)] {
        let image = render(
            json!({"cropRect":{"x":2,"y":0,"w":4,"h":4},"flippedX":flip_x,"flippedY":flip_y}),
            propagation(0.0, 0.0, 4.0, 2.0),
        );
        for y in 0..4 {
            for x in 0..8 {
                let occupied =
                    (if flip_x { x >= 4 } else { x < 4 }) && (if flip_y { y >= 2 } else { y < 2 });
                assert_eq!(
                    pixel(&image, x, y)[3] > 120,
                    occupied,
                    "({flip_x},{flip_y}) ({x},{y})"
                );
            }
        }
        let (x, y) = (if flip_x { 4 } else { 0 }, if flip_y { 2 } else { 0 });
        assert!(pixel(&image, x, y)[usize::from(flip_x)] > 80);
        assert!(pixel(&image, x + 3, y)[usize::from(!flip_x)] > 80);
    }
}

#[test]
fn legacy_outside_source_preserves_transparent_area_and_visible_erase_coordinates() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for flipped in [false, true] {
        let image = render(
            json!({"cropRect":{"x":-4,"y":0,"w":8,"h":4},"flippedX":flipped}),
            Value::Null,
        );
        for x in 0..8 {
            assert_eq!(
                pixel(&image, x, 1)[3] > 120,
                if flipped { x < 4 } else { x >= 4 }
            );
        }
    }
    let image = render(
        json!({"cropRect":{"x":0,"y":0,"w":4,"h":4},"flippedX":true,
        "contentEraseStrokes":[{"width":2,"points":[{"x":5,"y":1}]}]}),
        propagation(0.0, 0.0, 4.0, 2.0),
    );
    // 输出坐标 4×4 覆盖：圆形笔迹在该角像素仅覆盖 13/16，不应提前在源图抹平软边。
    assert!((23..=25).contains(&pixel(&image, 4, 0)[3]));
    assert!(pixel(&image, 7, 0)[0] > 80);
}

#[test]
fn legacy_vertical_crop_selects_original_rows_before_flip() {
    let _com = crate::desktop::Apartment::new().unwrap();
    // 上下不同色；仅检查占用矩形不足以证明 Y 轴取样区域正确。
    let pixels = [[200, 0, 0, 255].repeat(16), [0, 200, 0, 255].repeat(16)].concat();
    for flipped in [false, true] {
        let image = render_pixels(
            json!({"cropRect":{"x":0,"y":1,"w":8,"h":1},"flippedY":flipped}),
            Value::Null,
            pixels.clone(),
        );
        assert!(image
            .pixels
            .chunks_exact(4)
            .all(|p| p[0] > 80 && p[1] == 0 && p[3] > 120));
    }
}

#[test]
fn legacy_source_overflow_on_each_side_and_no_intersection_remain_transparent() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for (cx, cy) in [(-4, 0), (4, 0), (0, -2), (0, 2), (8, 0), (0, 4), (-8, -4)] {
        for (fx, fy) in [(false, false), (true, true)] {
            let image = render(
                json!({"cropRect":{"x":cx,"y":cy,"w":8,"h":4},"flippedX":fx,"flippedY":fy}),
                Value::Null,
            );
            for y in 0..4 {
                for x in 0..8 {
                    let sx = (if fx { 7 - x } else { x }) as i32 + cx;
                    let sy = (if fy { 3 - y } else { y }) as i32 + cy;
                    let occupied = (0..8).contains(&sx) && (0..4).contains(&sy);
                    assert_eq!(
                        pixel(&image, x, y)[3] > 120,
                        occupied,
                        "crop=({cx},{cy}) flip=({fx},{fy}) ({x},{y})"
                    );
                }
            }
        }
    }
}

#[test]
fn legacy_crop_missing_or_nonpositive_propagation_source_uses_full_target() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let edit = json!({"cropRect":{"x":0,"y":0,"w":4,"h":4}});
    let expected = render(edit.clone(), Value::Null);
    for state in [
        json!({"upstreamSourceFrame":{"w":8,"h":4}}),
        json!({"upstreamContentFrame":{"x":0,"y":0,"w":4,"h":2}}),
        json!({"upstreamSourceFrame":{"w":0,"h":4},"upstreamContentFrame":{"x":0,"y":0,"w":4,"h":2}}),
        json!({"upstreamSourceFrame":{"w":8,"h":-1},"upstreamContentFrame":{"x":0,"y":0,"w":4,"h":2}}),
    ] {
        assert_eq!(
            render(edit.clone(), state).pixels.as_ref(),
            expected.pixels.as_ref()
        );
    }
}
