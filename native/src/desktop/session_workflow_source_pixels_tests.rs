//! 真实 WIC/原生合成验证底图身份；像素不同，避免只证明选择字符串的假阳性。
use super::*;
use serde_json::{json, Value};
#[path = "session_workflow_blur_pixels_tests.rs"]
mod blur_pixels;
#[path = "session_workflow_content_pixels_tests.rs"]
mod content_pixels;
fn inline(pixel: [u8; 4]) -> String {
    let image = crate::desktop::image::Image {
        width: 2,
        height: 1,
        pixels: pixel.repeat(2).into(),
    };
    let png = crate::desktop::export::png_raw(&image, &AtomicBool::new(false)).unwrap();
    crate::loom_base64::encode_png_data_url(&png, &AtomicBool::new(false))
        .unwrap()
        .as_str()
        .into()
}
fn bake(extra: Value, upstream: bool) -> crate::desktop::image::Image {
    let cancel = AtomicBool::new(false);
    let mut target = json!({"id":"b","x":0,"y":0,"w":2,"h":1,"opacityNormal":0.5,"src":inline([200,0,0,255]),"previewSrc":inline([0,200,0,255])});
    target
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    let links = if upstream {
        json!([{"id":"edge","fromUnitId":"a","toUnitId":"b","fromPortId":"output","toPortId":"image"}])
    } else {
        json!([])
    };
    let saved = serde_json::to_vec(&json!({"documentRevision":1,"stickers":[
        {"id":"a","x":0,"y":0,"w":2,"h":1,"src":inline([0,0,200,255])},target],"links":links}))
    .unwrap();
    let candidate =
        VectorCandidate::prepare(prepare(saved, None, &cancel).unwrap(), &cancel).unwrap();
    let prepared = candidate.prepare_workflow_snapshot(&[], &cancel).unwrap();
    crate::desktop::session_image::inline(
        prepared.snapshot().value()["nodes"][1]["data"]["src"]
            .as_str()
            .unwrap(),
        &cancel,
    )
    .unwrap()
}
#[test]
fn workflow_bake_uses_preview_upstream_manual_and_raster_override_pixels() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for (extra, upstream, channel) in [
        (json!({}), false, 1usize),
        (json!({}), true, 2),
        (
            json!({"stickerEditPropagation":{"acceptUpstream":false}}),
            true,
            1,
        ),
        (
            json!({"params":{"image_path":inline([0,0,200,255])}}),
            false,
            2,
        ),
        (
            json!({"rasterizedAnnotationLayerSrc":inline([0,0,0,0])}),
            true,
            0,
        ),
    ] {
        let image = bake(extra, upstream);
        assert!(
            image.pixels[channel] > 80,
            "unexpected pixel {:?}",
            &image.pixels[..4]
        );
        for other in 0..3 {
            if other != channel {
                assert_eq!(image.pixels[other], 0);
            }
        }
        assert!((127..=128).contains(&image.pixels[3]));
    }
}

#[test]
fn workflow_bake_rejects_unadmitted_formal_output_file() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let cancel = AtomicBool::new(false);
    let saved = serde_json::to_vec(&json!({"documentRevision":1,"stickers":[
        {"id":"a","type":"art","artId":"test","x":0,"y":0,"w":2,"h":1,"outputs":{"output":"C:/not-admitted.png"}},
        {"id":"b","x":0,"y":0,"w":2,"h":1,"opacityNormal":0.5,"src":inline([200,0,0,255])}
    ],"links":[{"id":"edge","fromUnitId":"a","toUnitId":"b","fromPortId":"output","toPortId":"image"}]})).unwrap();
    let candidate =
        VectorCandidate::prepare(prepare(saved, None, &cancel).unwrap(), &cancel).unwrap();
    assert!(candidate
        .prepare_workflow_snapshot(&[json!({"id":"test"})], &cancel)
        .is_err());
}

#[test]
fn workflow_bake_reads_admitted_output_file_and_applies_target_flip() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let cancel = AtomicBool::new(false);
    let directory =
        std::path::PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi").join(format!(
            "otter-workflow-source-{}",
            crate::desktop::document_state::new_unit_id().unwrap()
        ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("formal.png");
    let image = crate::desktop::image::Image {
        width: 2,
        height: 1,
        pixels: vec![200, 0, 0, 255, 0, 200, 0, 255].into(),
    };
    std::fs::write(
        &path,
        crate::desktop::export::png_raw(&image, &cancel)
            .unwrap()
            .as_ref(),
    )
    .unwrap();
    let root = crate::desktop::session_asset_root::Root::open(&directory).unwrap();
    let saved = serde_json::to_vec(&json!({"documentRevision":1,"stickers":[
        {"id":"a","type":"art","artId":"test","x":0,"y":0,"w":2,"h":1,"outputs":{"output":{"path":path.to_str().unwrap()}}},
        {"id":"b","x":0,"y":0,"w":2,"h":1,"opacityNormal":0.5,"src":inline([0,0,200,255]),"imageEditState":{"flippedX":true}}
    ],"links":[{"id":"edge","fromUnitId":"a","toUnitId":"b","fromPortId":"output","toPortId":"image"}]})).unwrap();
    let candidate =
        VectorCandidate::prepare(prepare(saved, Some(&root), &cancel).unwrap(), &cancel).unwrap();
    let result = candidate
        .prepare_workflow_snapshot_with_root(&[json!({"id":"test"})], Some(&root), &cancel)
        .unwrap();
    let baked = crate::desktop::session_image::inline(
        result.snapshot().value()["nodes"][1]["data"]["src"]
            .as_str()
            .unwrap(),
        &cancel,
    )
    .unwrap();
    assert_eq!((baked.width, baked.height), (2, 1));
    assert_eq!(
        (
            baked.pixels[0],
            baked.pixels[2],
            baked.pixels[5],
            baked.pixels[6]
        ),
        (0, 0, 0, 0)
    );
    assert!(baked.pixels[1] > 80 && baked.pixels[4] > 80);
    assert!((127..=128).contains(&baked.pixels[3]) && (127..=128).contains(&baked.pixels[7]));
    // 原资源必须保持原来的像素次序，目标翻转不能写回来源文件。
    assert_eq!(
        crate::desktop::export::decode_png(std::fs::read(&path).unwrap())
            .pixels
            .as_ref(),
        image.pixels.as_ref()
    );
    drop((result, candidate, root));
    std::fs::remove_dir_all(directory).unwrap();
}
