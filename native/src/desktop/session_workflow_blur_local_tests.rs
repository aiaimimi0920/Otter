//! 工作流实际 PNG：持久框裁剪、按身份匹配及透明邻域，不以笔迹 bounds 替代。
use super::*;

fn candidate(elements: serde_json::Value) -> VectorCandidate {
    let cancel = AtomicBool::new(false);
    let pixels = (0..256)
        .flat_map(|i| {
            if i % 16 < 8 {
                [200, 0, 0, 255]
            } else {
                [0, 200, 0, 255]
            }
        })
        .collect::<Vec<_>>();
    let image = crate::desktop::image::Image {
        width: 16,
        height: 16,
        pixels: pixels.into(),
    };
    let png = crate::desktop::export::png_raw(&image, &cancel).unwrap();
    let url = crate::loom_base64::encode_png_data_url(&png, &cancel).unwrap();
    let saved = serde_json::to_vec(
        &json!({"stickers":[{"id":"local","x":0,"y":0,"w":16,"h":16,"src":url.as_str(),
        "opacityNormal":0,"annotationState":{"serialCounter":0,"elements":elements}}],"links":[]}),
    )
    .unwrap();
    VectorCandidate::prepare(prepare(saved, None, &cancel).unwrap(), &cancel).unwrap()
}
fn effect(id: &str, x: f64, z: u32) -> serde_json::Value {
    json!({"id":id,"type":"blur","zIndex":z,"x":x,"y":2,"w":4,"h":12,
        "points":[{"x":x+2.0,"y":8}],"brushWidth":32,"strength":4,"style":{"width":1,"color":"#fff"}})
}
fn pixel(image: &crate::desktop::image::Image, x: usize, y: usize) -> &[u8] {
    let at = (y * image.width as usize + x) * 4;
    &image.pixels[at..at + 4]
}

#[test]
fn local_blur_uses_persisted_boxes_and_transparent_neighbours_after_z_sort() {
    let _com = crate::desktop::Apartment::new().unwrap();
    // JSON 顺序与恢复排序相反，同强度但不同框不能共享错误的局部纹理。
    let candidate = candidate(json!([effect("blue", 2.0, 10), effect("green", 10.0, 0)]));
    let editor = &candidate.vectors[0].1.editor;
    assert_eq!(editor.elements()[0].annotation_id(), "green");
    let before = editor.elements()[0].blur.as_ref().unwrap().pixels().clone();
    let png = candidate
        .bake_source_png(&UnitId::parse("local").unwrap(), &AtomicBool::new(false))
        .unwrap();
    let image = crate::desktop::export::decode_png(png.to_vec());
    for (x, channel) in [(4, 0), (12, 1)] {
        let p = pixel(&image, x, 8);
        assert!(
            p[channel] > 60 && p[1 - channel] <= 22,
            "local color leaked: {p:?}"
        );
        assert!(
            p[3] > 30 && p[3] < 180,
            "transparent neighbourhood lost: {p:?}"
        );
    }
    for (x, y) in [(0, 8), (7, 8), (8, 8), (15, 8), (4, 0), (12, 15)] {
        assert_eq!(
            pixel(&image, x, y),
            &[0, 0, 0, 0],
            "wash escaped persisted box at {x},{y}"
        );
    }
    assert_eq!(editor.revision(), 0);
    assert!(editor.elements()[0]
        .blur
        .as_ref()
        .unwrap()
        .pixels()
        .same_storage(&before));
}

#[test]
fn missing_or_invalid_local_box_fails_without_replacing_original_resources() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for invalid in [serde_json::Value::Null, json!(-1), json!(1e20)] {
        let mut e = effect("bad", 2.0, 0);
        e["w"] = invalid;
        let candidate = candidate(json!([e]));
        let editor = &candidate.vectors[0].1.editor;
        let before = editor.elements()[0].blur.as_ref().unwrap().pixels().clone();
        assert!(candidate
            .bake_source_png(&UnitId::parse("local").unwrap(), &AtomicBool::new(false))
            .is_err());
        assert_eq!(editor.revision(), 0);
        assert!(editor.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .pixels()
            .same_storage(&before));
    }
}

#[test]
fn local_blur_rotates_around_persisted_box_not_brush_center() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let mut e = effect("rotated", 4.0, 0);
    e["y"] = json!(4);
    e["w"] = json!(8);
    e["h"] = json!(4);
    e["points"] = json!([{"x":6,"y":6}]);
    e["rotation"] = json!(90);
    let candidate = candidate(json!([e]));
    let png = candidate
        .bake_source_png(&UnitId::parse("local").unwrap(), &AtomicBool::new(false))
        .unwrap();
    let image = crate::desktop::export::decode_png(png.to_vec());
    // 框中心 (8,6)，旋转后的框是 [6,10]×[2,10]；笔迹中心 (6,6) 会平移错位。
    assert!(pixel(&image, 9, 6)[3] > 20);
    assert_eq!(pixel(&image, 5, 8), &[0, 0, 0, 0]);
    assert_eq!(pixel(&image, 7, 11), &[0, 0, 0, 0]);
    assert_eq!(candidate.vectors[0].1.editor.revision(), 0);
}

#[test]
fn fractional_local_box_scales_the_stroke_mask_with_the_texture() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let mut e = effect("fractional", 4.0, 0);
    e["y"] = json!(4);
    e["w"] = json!(4.1);
    e["h"] = json!(8);
    e["points"] = json!([{"x":8,"y":8}]);
    e["brushWidth"] = json!(2);
    let candidate = candidate(json!([e]));
    let png = candidate
        .bake_source_png(&UnitId::parse("local").unwrap(), &AtomicBool::new(false))
        .unwrap();
    let image = crate::desktop::export::decode_png(png.to_vec());
    // x 方向 4.1/ceil(4.1)=0.82：圆心从 8 移到 7.28，半径从 1 变为 0.82。
    // 未缩放的旧遮罩完全不覆盖 x=6 的像素列。
    assert!(
        pixel(&image, 6, 8)[3] > 5,
        "mask did not shrink into local coordinates"
    );
    assert_eq!(pixel(&image, 5, 8), &[0, 0, 0, 0]);
    assert_eq!(pixel(&image, 7, 6), &[0, 0, 0, 0]);
}

#[test]
fn rotated_source_clip_keeps_only_wash_outside_the_source_projection() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let mut e = effect("partial", -4.0, 0);
    e["w"] = json!(12);
    e["rotation"] = json!(45);
    e["points"] = json!([{"x":2,"y":8}]);
    let candidate = candidate(json!([e]));
    let png = candidate
        .bake_source_png(&UnitId::parse("local").unwrap(), &AtomicBool::new(false))
        .unwrap();
    let image = crate::desktop::export::decode_png(png.to_vec());
    // 此像素逆旋转后全部位于 x<0：在效果框内、源投影外，只能有 8% 白色覆盖。
    let p = pixel(&image, 0, 5);
    assert!(
        (19..=22).contains(&p[3]),
        "rotated source clip leaked blur: {p:?}"
    );
    assert!(p[..3].iter().all(|v| (19..=22).contains(v)));
}
