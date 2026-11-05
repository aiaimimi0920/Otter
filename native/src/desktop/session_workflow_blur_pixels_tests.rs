//! 模糊读取原始方向及独立裁剪坐标，不能误用已翻转/contain 后的底图。
use super::*;
#[path = "session_workflow_blur_local_tests.rs"]
mod local;

fn render(
    vertical: bool,
    flipped: bool,
    cropped: bool,
    propagated: bool,
    strength: f32,
) -> crate::desktop::image::Image {
    let cancel = AtomicBool::new(false);
    let (width, height) = if vertical { (64, 128) } else { (128, 64) };
    let mut pixels = Vec::new();
    for y in 0..height {
        for x in 0..width {
            pixels.extend_from_slice(if (if vertical { y } else { x }) < 64 {
                &[200, 0, 0, 255]
            } else {
                &[0, 200, 0, 255]
            });
        }
    }
    let image = crate::desktop::image::Image {
        width,
        height,
        pixels: pixels.into(),
    };
    let png = crate::desktop::export::png_raw(&image, &cancel).unwrap();
    let url = crate::loom_base64::encode_png_data_url(&png, &cancel).unwrap();
    let mut edit = json!({"flippedX":!vertical && flipped,"flippedY":vertical && flipped});
    if cropped {
        edit["cropRect"] = json!({"x":32,"y":0,"w":32,"h":32});
        edit["sourceSize"] = json!({"w":64,"h":32});
    }
    let mut sticker = json!({"id":"blur","x":0,"y":0,"w":if vertical {32} else {64},"h":if vertical {64} else {32},
        "src":url.as_str(),"opacityNormal":0,"imageEditState":edit,
        "annotationState":{"serialCounter":0,"elements":[{"id":"effect","type":"blur","zIndex":0,"x":0,"y":0,"w":32,"h":32,
            "points":[{"x":16,"y":16}],"brushWidth":8,"strength":strength,"style":{"width":1,"color":"#fff"}}]}});
    if propagated {
        sticker["stickerEditPropagation"] = json!({"upstreamSourceFrame":{"w":64,"h":32},"upstreamContentFrame":{"x":32,"y":0,"w":32,"h":32}});
    }
    let saved =
        serde_json::to_vec(&json!({"documentRevision":1,"stickers":[sticker],"links":[]})).unwrap();
    let candidate =
        VectorCandidate::prepare(prepare(saved, None, &cancel).unwrap(), &cancel).unwrap();
    let revision = candidate.vectors[0].1.editor.revision();
    let prepared = candidate.prepare_workflow_snapshot(&[], &cancel).unwrap();
    assert_eq!(candidate.vectors[0].1.editor.revision(), revision);
    // 绘制取整不能反向修改恢复后的参数或未来保存的数据。
    assert_eq!(
        candidate.vectors[0].1.editor.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .strength(),
        strength
    );
    crate::desktop::session_image::inline(
        prepared.snapshot().value()["nodes"][0]["data"]["src"]
            .as_str()
            .unwrap(),
        &cancel,
    )
    .unwrap()
}
fn center(image: &crate::desktop::image::Image, cropped: bool) -> &[u8] {
    let (x, y) = if cropped { (16, 32) } else { (32, 32) };
    let at = (y * image.width + x) as usize * 4;
    &image.pixels[at..at + 4]
}
#[test]
fn blur_uses_unflipped_source_on_both_axes() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for vertical in [false, true] {
        let original = render(vertical, false, false, false, 1.0);
        let flipped = render(vertical, true, false, false, 1.0);
        assert_eq!(original.pixels.as_ref(), flipped.pixels.as_ref());
        let p = center(&flipped, false);
        assert!(
            p[0] > 150 && p[1] < 80 && p[3] > 240,
            "unexpected blur pixel {p:?}"
        );
    }
}
#[test]
fn cropped_blur_uses_crop_pixel_offset_not_base_density_or_propagated_frame() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let expected = render(false, false, true, false, 1.0);
    assert_eq!((expected.width, expected.height), (64, 64));
    for (flipped, propagated) in [(true, false), (false, true), (true, true)] {
        let image = render(false, flipped, true, propagated, 1.0);
        assert_eq!(image.pixels.as_ref(), expected.pixels.as_ref());
        // 标注 x=16 对应原图 x=32+16=48（蓝），不是底图 contain 后的 x=96（绿）。
        let p = center(&image, true);
        assert!(
            p[0] > 150 && p[1] < 80 && p[3] > 240,
            "unexpected crop blur pixel {p:?}"
        );
    }
}

#[test]
fn blur_rounds_fractional_strength_only_when_baking() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for cropped in [false, true] {
        let lower = render(false, false, cropped, false, 7.0);
        let upper = render(false, false, cropped, false, 8.0);
        let below_half = render(false, false, cropped, false, 7.49);
        let half = render(false, false, cropped, false, 7.5);
        assert_eq!(below_half.pixels.as_ref(), lower.pixels.as_ref());
        assert_eq!(half.pixels.as_ref(), upper.pixels.as_ref());
        assert_ne!(lower.pixels.as_ref(), upper.pixels.as_ref());
    }
}
