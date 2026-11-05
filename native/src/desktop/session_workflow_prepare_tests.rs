//! 真正的会话解码、源分辨率合成、参数过滤和工作流编码贯通测试。
use super::*;
use serde_json::json;
fn fixture(extra: serde_json::Value) -> (VectorCandidate, Vec<serde_json::Value>) {
    let cancel = AtomicBool::new(false);
    let source = crate::desktop::image::Image {
        width: 40,
        height: 20,
        pixels: [10, 20, 30, 255].repeat(800).into(),
    };
    let png = crate::desktop::export::png_raw(&source, &cancel).unwrap();
    let url = crate::loom_base64::encode_png_data_url(&png, &cancel).unwrap();
    let mut sticker = json!({"id":"s","type":"sticker","x":0,"y":0,"w":20,"h":10,"opacityNormal":0.5,"src":url.as_str()});
    sticker
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    let bytes=serde_json::to_vec(&json!({"documentRevision":5,"stickers":[sticker,
        {"id":"art","type":"art","artId":"art:test","x":0,"y":0,"w":1,"h":1,"params":{"secret":"do-not-send","value":false}}
    ],"links":[]})).unwrap();
    let candidate =
        VectorCandidate::prepare(prepare(bytes, None, &cancel).unwrap(), &cancel).unwrap();
    (
        candidate,
        vec![json!({"id":"art:test","parameters":[{"id":"secret","secret":true},{"id":"value"}]})],
    )
}
#[test]
fn real_bake_and_parameter_filter_feed_global_and_component_ready_snapshot() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (candidate, catalog) = fixture(json!({"originWorkflowId":"wf","originNodeId":"remote"}));
    let prepared = candidate
        .prepare_workflow_snapshot(&catalog, &AtomicBool::new(false))
        .unwrap();
    assert_eq!(prepared._leases.len(), 1);
    let snapshot = prepared.snapshot();
    let nodes = &snapshot.value()["nodes"];
    let decoded = crate::desktop::session_image::inline(
        nodes[0]["data"]["src"].as_str().unwrap(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!((decoded.width, decoded.height), (40, 20));
    assert!((127..=128).contains(&decoded.pixels[3]));
    assert_eq!(nodes[1]["data"]["params"], json!({"value":false}));
    assert_eq!(snapshot.revision(), 5);
    let components = workflow_snapshot::components::prepare(
        snapshot,
        candidate.candidate.original_json(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(components.len(), 1);
    let payload = components[0]
        .encode_payload(1024 * 1024, &AtomicBool::new(false))
        .unwrap();
    assert!(!String::from_utf8(payload).unwrap().contains("do-not-send"));
}
#[test]
fn invalid_style_missing_capability_and_cancel_return_no_prepared_snapshot() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (candidate, catalog) =
        fixture(json!({"imageEditState":{"borderWidth":2,"borderColor":"url(secret)"}}));
    assert!(candidate
        .prepare_workflow_snapshot(&catalog, &AtomicBool::new(false))
        .is_err());
    let (candidate, catalog) = fixture(json!({}));
    assert!(candidate
        .prepare_workflow_snapshot(&[], &AtomicBool::new(false))
        .is_err());
    assert!(candidate
        .prepare_workflow_snapshot(&catalog, &AtomicBool::new(true))
        .is_err());
}
#[test]
fn border_bakes_above_transparent_base_at_source_density() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (candidate, catalog) = fixture(json!({"opacityNormal":0,
        "imageEditState":{"borderWidth":2,"borderColor":"#ff0000"}}));
    let prepared = candidate
        .prepare_workflow_snapshot(&catalog, &AtomicBool::new(false))
        .unwrap();
    let image = crate::desktop::session_image::inline(
        prepared.snapshot().value()["nodes"][0]["data"]["src"]
            .as_str()
            .unwrap(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!((image.width, image.height), (40, 20));
    // 两倍自然像素密度下逻辑 2px 边框为 4px；中心保留全透明底图。
    for (x, y) in [(1, 10), (3, 10), (38, 10), (20, 1), (20, 18)] {
        let start = (y * 40 + x) * 4;
        assert_eq!(&image.pixels[start..start + 4], &[0, 0, 255, 255]);
    }
    assert_eq!(
        &image.pixels[(10 * 40 + 20) * 4..(10 * 40 + 20) * 4 + 4],
        &[0, 0, 0, 0]
    );
}

#[test]
fn rounded_frame_clips_source_and_border_together() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for radius in [4, 100] {
        let (candidate, catalog) = fixture(json!({"opacityNormal":1,
            "imageEditState":{"cornerRadius":radius,"borderWidth":2,"borderColor":"red"}}));
        let prepared = candidate
            .prepare_workflow_snapshot(&catalog, &AtomicBool::new(false))
            .unwrap();
        let image = crate::desktop::session_image::inline(
            prepared.snapshot().value()["nodes"][0]["data"]["src"]
                .as_str()
                .unwrap(),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!((image.width, image.height), (40, 20));
        for (x, y) in [(0, 0), (39, 0), (0, 19), (39, 19)] {
            assert_eq!(image.pixels[(y * 40 + x) * 4 + 3], 0);
        }
        assert_eq!(
            &image.pixels[(10 * 40 + 20) * 4..(10 * 40 + 20) * 4 + 4],
            &[10, 20, 30, 255]
        );
        assert_eq!(
            &image.pixels[(40 + 20) * 4..(40 + 20) * 4 + 4],
            &[0, 0, 255, 255]
        );
    }
}

#[test]
fn historical_erase_changes_only_base_before_border_and_raster_annotation() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let cancel = AtomicBool::new(false);
    let overlay = crate::desktop::image::Image {
        width: 1,
        height: 1,
        pixels: vec![0, 255, 0, 255].into(),
    };
    let png = crate::desktop::export::png_raw(&overlay, &cancel).unwrap();
    let url = crate::loom_base64::encode_png_data_url(&png, &cancel).unwrap();
    for annotation in [false, true] {
        let mut extra = json!({"opacityNormal":1,"imageEditState":{
            "borderWidth":2,"borderColor":"red","contentEraseStrokes":[
                {"width":4,"points":[{"x":10,"y":5}]},
                {"width":4,"points":[{"x":0,"y":5}]}]}});
        if annotation {
            extra["rasterizedAnnotationLayerSrc"] = json!(url.as_str());
        }
        let (candidate, catalog) = fixture(extra);
        let prepared = candidate
            .prepare_workflow_snapshot(&catalog, &cancel)
            .unwrap();
        let image = crate::desktop::session_image::inline(
            prepared.snapshot().value()["nodes"][0]["data"]["src"]
                .as_str()
                .unwrap(),
            &cancel,
        )
        .unwrap();
        let pixel = |x: usize, y: usize| &image.pixels[(y * 40 + x) * 4..(y * 40 + x) * 4 + 4];
        assert_eq!(
            pixel(20, 10),
            if annotation {
                &[0, 255, 0, 255]
            } else {
                &[0, 0, 0, 0]
            }
        );
        assert_eq!(
            pixel(1, 10),
            if annotation {
                &[0, 255, 0, 255]
            } else {
                &[0, 0, 255, 255]
            }
        );
    }
}

#[test]
fn batch_bakes_keep_each_parsed_node_style_and_identity() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let cancel = AtomicBool::new(false);
    let (original, catalog) = fixture(json!({}));
    let mut document: serde_json::Value =
        serde_json::from_slice(original.candidate.original_json()).unwrap();
    let mut second = document["stickers"][0].clone();
    second["id"] = json!("second");
    second["opacityNormal"] = json!(0.25);
    second["imageEditState"] = json!({"cornerRadius":4});
    document["stickers"].as_array_mut().unwrap().push(second);
    let candidate = VectorCandidate::prepare(
        prepare(serde_json::to_vec(&document).unwrap(), None, &cancel).unwrap(),
        &cancel,
    )
    .unwrap();
    let prepared = candidate
        .prepare_workflow_snapshot(&catalog, &cancel)
        .unwrap();
    assert_eq!(prepared._leases.len(), 2);
    let nodes = prepared.snapshot().value()["nodes"].as_array().unwrap();
    for (id, alpha, corner) in [("s", 128, 128), ("second", 64, 0)] {
        let node = nodes.iter().find(|node| node["id"] == id).unwrap();
        let image =
            crate::desktop::session_image::inline(node["data"]["src"].as_str().unwrap(), &cancel)
                .unwrap();
        assert_eq!((image.width, image.height), (40, 20));
        assert!((image.pixels[(10 * 40 + 20) * 4 + 3] as i32 - alpha).abs() <= 1);
        assert!((image.pixels[3] as i32 - corner).abs() <= 1);
    }
    // 保留单张公共入口的身份校验；不存在的节点不能复用批量路径中的其它图片。
    assert!(candidate
        .bake_source_png(&UnitId::parse("missing").unwrap(), &cancel)
        .is_err());
}

#[test]
fn workflow_blur_is_prepared_at_natural_density_including_cropped_frame() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let cancel = AtomicBool::new(false);
    for cropped in [false, true] {
        let mut extra = json!({"opacityNormal":0,"annotationState":{"serialCounter":0,
            "elements":[{"id":"blur","type":"blur","zIndex":0,"x":0,"y":0,"w":20,"h":10,
                "points":[{"x":10,"y":5}],"brushWidth":4,"strength":1,
                "style":{"width":1,"color":"#fff"}}]}});
        if cropped {
            extra["imageEditState"] = json!({"cropRect":{"x":5,"y":2,"w":10,"h":6},
                "sourceSize":{"w":20,"h":10}});
        }
        let (candidate, catalog) = fixture(extra);
        let prepared = candidate
            .prepare_workflow_snapshot(&catalog, &cancel)
            .unwrap();
        let image = crate::desktop::session_image::inline(
            prepared.snapshot().value()["nodes"][0]["data"]["src"]
                .as_str()
                .unwrap(),
            &cancel,
        )
        .unwrap();
        let (width, height) = if cropped { (20, 12) } else { (40, 20) };
        assert_eq!((image.width, image.height), (width, height));
        let at = ((height / 2 * width + width / 2) * 4) as usize;
        for (actual, expected) in image.pixels[at..at + 4].iter().zip([30u8, 39, 48, 255]) {
            assert!(
                actual.abs_diff(expected) <= 1,
                "cropped={cropped} actual={actual} expected={expected}"
            );
        }
        assert_eq!(image.pixels[3], 0);
        assert_eq!(candidate.vectors[0].1.editor.revision(), 0);
    }
}

#[test]
fn stretched_crop_keeps_source_density_but_contains_in_current_viewport() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let cancel = AtomicBool::new(false);
    let (candidate, catalog) = fixture(json!({"opacityNormal":1,"imageEditState":{
        "cropRect":{"x":0,"y":0,"w":10,"h":6},"sourceSize":{"w":20,"h":10}}}));
    let prepared = candidate
        .prepare_workflow_snapshot(&catalog, &cancel)
        .unwrap();
    let image = crate::desktop::session_image::inline(
        prepared.snapshot().value()["nodes"][0]["data"]["src"]
            .as_str()
            .unwrap(),
        &cancel,
    )
    .unwrap();
    assert_eq!((image.width, image.height), (20, 12));
    // 主干 viewport=40x(50/3)，自然图 contain 后宽100/3、左留白10/3。
    // 输出仍20x12；明确检查留白及内部像素，不绑定分数边缘的光栅覆盖方式。
    for row in image.pixels.chunks_exact(20 * 4) {
        assert!(row[..3 * 4].iter().all(|channel| *channel == 0));
        assert!(row[4 * 4..]
            .chunks_exact(4)
            .all(|pixel| pixel == [10, 20, 30, 255]));
    }
}

#[test]
fn workflow_beautify_wraps_composite_without_resizing_node_geometry() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let cancel = AtomicBool::new(false);
    for enabled in [false, true] {
        let (candidate, catalog) = fixture(json!({"opacityNormal":1,"imageEditState":{"beautify":{
            "enabled":enabled,"padding":4,"cornerRadius":2,"shadow":true,"backgroundId":"white"}}}));
        let prepared = candidate
            .prepare_workflow_snapshot(&catalog, &cancel)
            .unwrap();
        let node = &prepared.snapshot().value()["nodes"][0];
        let image =
            crate::desktop::session_image::inline(node["data"]["src"].as_str().unwrap(), &cancel)
                .unwrap();
        assert_eq!(
            (image.width, image.height),
            if enabled { (48, 28) } else { (40, 20) }
        );
        assert_eq!(node["data"]["w"].as_f64(), Some(20.0));
        assert_eq!(node["data"]["h"].as_f64(), Some(10.0));
        let at = ((image.height / 2 * image.width + image.width / 2) * 4) as usize;
        assert_eq!(&image.pixels[at..at + 4], &[10, 20, 30, 255]);
    }
}

#[test]
fn legacy_image_rotation_does_not_reject_or_rotate_workflow_pixels() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let cancel = AtomicBool::new(false);
    let source = crate::desktop::image::Image {
        width: 40,
        height: 20,
        pixels: (0..800)
            .flat_map(|i| [(i % 40 * 5) as u8, (i / 40 * 11) as u8, 60, 255])
            .collect::<Vec<_>>()
            .into(),
    };
    let png = crate::desktop::export::png_raw(&source, &cancel).unwrap();
    let url = crate::loom_base64::encode_png_data_url(&png, &cancel).unwrap();
    for rotation in [0, 90, 180, 270] {
        let (candidate, catalog) = fixture(json!({"src":url.as_str(),"opacityNormal":1,
            "imageEditState":{"rotation":rotation}}));
        let original = candidate.candidate.original_json().to_vec();
        let prepared = candidate
            .prepare_workflow_snapshot(&catalog, &cancel)
            .unwrap();
        let image = crate::desktop::session_image::inline(
            prepared.snapshot().value()["nodes"][0]["data"]["src"]
                .as_str()
                .unwrap(),
            &cancel,
        )
        .unwrap();
        assert_eq!((image.width, image.height), (40, 20));
        assert_eq!(image.pixels, source.pixels, "rotation={rotation}");
        assert_eq!(candidate.candidate.original_json(), original);
    }
}

#[test]
fn edited_candidate_cannot_mix_new_editor_with_old_saved_snapshot() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (mut candidate, catalog) = fixture(json!({}));
    candidate.vectors[0]
        .1
        .editor
        .crop_to(crate::crop_geometry::Rect {
            x: 1.0,
            y: 1.0,
            width: 5.0,
            height: 5.0,
        })
        .unwrap();
    assert!(candidate
        .prepare_workflow_snapshot(&catalog, &AtomicBool::new(false))
        .is_err());
}
