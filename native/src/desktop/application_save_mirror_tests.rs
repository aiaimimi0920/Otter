//! 从真实 PNG/旧主干文档恢复并重复保存：朝向保留，像素只物化一次。
use super::*;
use crate::{desktop::session_candidate, editor::MirrorState, raster_flip::Axis};

fn prepare(bytes: Vec<u8>) -> session_candidate::VectorCandidate {
    let cancel = AtomicBool::new(false);
    let candidate = session_candidate::prepare(bytes, None, &cancel).unwrap();
    session_candidate::VectorCandidate::prepare(candidate, &cancel).unwrap()
}
fn save(editor: Editor, original: Vec<u8>) -> Vec<u8> {
    let mut document = doc("mirror");
    document.frame.width = 4;
    document.frame.height = 2;
    document.editor = editor;
    let frame = document.editor.visible_frame().unwrap();
    document.viewport = crate::viewport::Viewport {
        scale: 1.0,
        origin: crate::viewport::Point {
            x: -frame.x as f32,
            y: -frame.y as f32,
        },
    };
    let mut snapshot = data(vec![document]);
    // 重复保存必须保留同一工作区 scope，和活动 Application 的快照合同一致。
    if let Some(scope) = crate::session_document::decode(&original)
        .unwrap()
        .projection_scope
    {
        snapshot.projection_scope = scope;
    }
    snapshot.original = original.into();
    snapshot
        .encode(&AtomicBool::new(false))
        .unwrap()
        .bytes
        .to_vec()
}
#[test]
fn legacy_and_native_mirror_pressed_survive_repeated_real_save_restore_and_workflow_bake() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let cancel = AtomicBool::new(false);
    let pixels: Vec<_> = (1..=8).flat_map(|v| [v, v, v, 255]).collect();
    let image = crate::desktop::image::Image {
        width: 4,
        height: 2,
        pixels: pixels.clone().into(),
    };
    let png = crate::desktop::export::png_raw(&image, &cancel).unwrap();
    let url = crate::loom_base64::encode_png_data_url(&png, &cancel).unwrap();
    for (x, y) in [(true, false), (false, true), (true, true)] {
        let mut bytes = serde_json::to_vec(&serde_json::json!({"stickers":[{
            "id":"mirror","x":10,"y":20,"w":4,"h":2,"src":url.as_str(),
            "rasterizedAnnotationLayerSrc":url.as_str(),
            "imageEditState":{"flippedX":x,"flippedY":y,"future":42},
            "annotationState":{"serialCounter":0,"elements":[{"id":"line","type":"line","zIndex":0,
                "points":[{"x":1,"y":1},{"x":2,"y":1}],"style":{"width":1,"color":"#fff"}}]}
        }],"links":[]}))
        .unwrap();
        let mut prepared = prepare(bytes.clone());
        let mut expected = MirrorState { x, y };
        assert_eq!(prepared.vectors[0].1.editor.mirror_state(), expected);
        let mut source = prepared.vectors.remove(0).1.editor;
        let plan = source
            .prepare_flip(Axis::Horizontal, |_| unreachable!())
            .unwrap()
            .unwrap()
            .compute(|| false)
            .unwrap();
        assert!(source.commit_raster_refresh(plan).unwrap());
        expected.x = !expected.x;
        let base = source.layers().unwrap().base.clone();
        let annotations = source.layers().unwrap().annotations.clone();
        let points = source.elements()[0].points.clone();
        bytes = save(source, bytes);
        for _ in 0..3 {
            let value: Value = serde_json::from_slice(&bytes).unwrap();
            let state = &value["stickers"][0]["imageEditState"];
            assert_eq!(state["flippedX"], false);
            assert_eq!(state["flippedY"], false);
            assert_eq!(state["future"], 42);
            assert_eq!(
                state["otterNativeMaterializedMirror"],
                serde_json::to_value(expected).unwrap()
            );
            let mut prepared = prepare(bytes.clone());
            let editor = &prepared.vectors[0].1.editor;
            assert_eq!(editor.mirror_state(), expected);
            assert_eq!(editor.layers().unwrap().base, base);
            assert_eq!(editor.layers().unwrap().annotations, annotations);
            assert_eq!(editor.elements()[0].points, points);
            let baked = prepared.bake_source_png(&id("mirror"), &cancel).unwrap();
            let actual = crate::desktop::export::decode_png(baked.to_vec());
            assert_eq!((actual.width, actual.height), (4, 2));
            // 独立栅格层覆盖底图，首像素不得被新 UI 元数据再次翻转。
            assert_eq!(&actual.pixels[..4], &annotations.as_ref().unwrap()[..4]);
            bytes = save(prepared.vectors.remove(0).1.editor, bytes);
        }
    }
}
#[test]
fn invalid_mirror_record_is_rejected_before_publishing_a_session() {
    let source = serde_json::to_vec(&serde_json::json!({"stickers":[{
        "id":"mirror","x":10,"y":20,"w":4,"h":2,
        "imageEditState":{"otterNativeMaterializedMirror":{"x":"true","y":false}}
    }],"links":[]}))
    .unwrap();
    assert!(session_candidate::prepare(source, None, &AtomicBool::new(false)).is_err());
}
