//! 极窄旧裁剪的实际 D2D/WIC 回归；大比例放大不能退回巨大整图坐标。
use super::*;

fn narrow(vertical: bool, flipped: bool, erase: bool) -> crate::desktop::image::Image {
    let cancel = AtomicBool::new(false);
    let (width, height) = if vertical { (1, 8192) } else { (8192, 1) };
    let mut pixels = [0, 200, 0, 255].repeat(8192);
    pixels[1234 * 4..1234 * 4 + 4].copy_from_slice(&[200, 0, 0, 255]);
    let original = crate::desktop::image::Image {
        width,
        height,
        pixels: pixels.into(),
    };
    let png = crate::desktop::export::png_raw(&original, &cancel).unwrap();
    let url = crate::loom_base64::encode_png_data_url(&png, &cancel).unwrap();
    let mut edit = json!({"cropRect":{"x":if vertical {0} else {1234},"y":if vertical {1234} else {0},"w":1,"h":1},
        "flippedX":!vertical && flipped,"flippedY":vertical && flipped});
    if erase {
        edit["contentEraseStrokes"] = json!([{"width":2,"points":[{
            "x":if vertical {0.5} else {4096.5},"y":if vertical {4096.5} else {0.5}}]}]);
    }
    let saved=serde_json::to_vec(&json!({"documentRevision":1,"stickers":[{
        "id":"narrow","x":0,"y":0,"w":width,"h":height,"src":url.as_str(),"opacityNormal":0.5,"imageEditState":edit
    }],"links":[]})).unwrap();
    let candidate =
        VectorCandidate::prepare(prepare(saved, None, &cancel).unwrap(), &cancel).unwrap();
    let prepared = candidate.prepare_workflow_snapshot(&[], &cancel).unwrap();
    let image = crate::desktop::session_image::inline(
        prepared.snapshot().value()["nodes"][0]["data"]["src"]
            .as_str()
            .unwrap(),
        &cancel,
    )
    .unwrap();
    assert_eq!((image.width, image.height), (width, height));
    assert_eq!(&original.pixels[1234 * 4..1234 * 4 + 4], &[200, 0, 0, 255]);
    image
}

#[test]
fn one_pixel_crop_expands_to_8192_without_giant_placement_for_either_axis() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for vertical in [false, true] {
        for flipped in [false, true] {
            let image = narrow(vertical, flipped, false);
            assert!(image
                .pixels
                .chunks_exact(4)
                .all(|p| p[0] > 80 && p[1] == 0 && p[3] > 120));
        }
    }
}

#[test]
fn magnified_single_pixel_can_be_erased_locally_without_erasing_entire_output() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for vertical in [false, true] {
        for flipped in [false, true] {
            let image = narrow(vertical, flipped, true);
            assert_eq!(image.pixels[4096 * 4 + 3], 0);
            assert!(image.pixels[4092 * 4 + 3] > 120);
            assert!(image.pixels[4100 * 4 + 3] > 120);
            assert!(image.pixels[3] > 120 && image.pixels[8191 * 4 + 3] > 120);
        }
    }
}

#[test]
fn fractional_crop_with_noop_or_empty_stroke_keeps_the_same_d2d_edge_pixels() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let edit = json!({"cropRect":{"x":0.25,"y":0.5,"w":3.5,"h":2.25},"flippedX":true});
    let state = propagation(0.25, 0.5, 4.5, 2.5);
    let expected = render(edit.clone(), state.clone());
    for stroke in [
        json!({"points":[]}),
        json!({"width":1,"points":[{"x":-50,"y":-50}]}),
    ] {
        let mut extra = edit.clone();
        extra["contentEraseStrokes"] = json!([stroke]);
        assert_eq!(
            render(extra, state.clone()).pixels.as_ref(),
            expected.pixels.as_ref()
        );
    }
}
