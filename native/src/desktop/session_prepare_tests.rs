//! 整批图片准备回归；合成内联图像，不读取会话中的任意路径或网络。
use super::*;
fn uri() -> String {
    let input = super::super::loom_inline::prepare(
        1,
        1,
        vec![10, 20, 30, 255].into(),
        &crate::editor::Editor::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    let crate::loom_art_wire::Input::InlinePng { base64, .. } = input.descriptor() else {
        panic!("PNG expected");
    };
    format!("data:image/png;base64,{base64}")
}
fn document(source: &str, second: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"stickers":[
        {"id":"one","type":"art","x":0,"y":0,"w":1,"h":1,"src":source,"previewSrc":source},
        {"id":"two","x":0,"y":0,"w":1,"h":1,"src":second,"rasterizedAnnotationLayerSrc":source,"filePath":"never-open-original"}
    ],"links":[]})).unwrap()
}
#[test]
fn repeated_references_share_pixels_and_result_outlives_session_bytes() {
    let reference = uri();
    let bytes = document(&reference, &reference);
    let prepared = std::thread::spawn(move || {
        let document = crate::session_document::decode(&bytes).unwrap();
        images(&document, &AtomicBool::new(false), |_| {
            panic!("inline images must not open files")
        })
        .unwrap()
    })
    .join()
    .unwrap();
    assert_eq!(prepared.len(), 2);
    let first = prepared[0].source.as_ref().unwrap();
    assert!(first
        .pixels
        .same_storage(&prepared[1].source.as_ref().unwrap().pixels));
    assert!(first
        .pixels
        .same_storage(&prepared[1].rasterized_annotations.as_ref().unwrap().pixels));
    assert!(prepared[0].preview.is_none());
    assert!(!prepared[0].restored_preview_locked);
    assert_eq!(
        prepared[0].display().unwrap().pixels.as_ref(),
        &[10, 20, 30, 255]
    );
    let weak = first.pixels.downgrade();
    drop(prepared);
    assert!(weak.upgrade().is_none());
}
#[test]
fn persisted_flip_changes_source_but_does_not_double_flip_saved_annotation_texture() {
    let input = super::super::loom_inline::prepare(
        2,
        1,
        vec![10, 20, 30, 255, 40, 50, 60, 255].into(),
        &crate::editor::Editor::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    let crate::loom_art_wire::Input::InlinePng { base64, .. } = input.descriptor() else {
        panic!("PNG")
    };
    let reference = format!("data:image/png;base64,{base64}");
    let bytes = serde_json::to_vec(&serde_json::json!({"stickers":[
        {"id":"one","x":0,"y":0,"w":2,"h":1,"src":reference,"rasterizedAnnotationLayerSrc":reference,
         "imageEditState":{"flippedX":true}},
        {"id":"two","x":0,"y":0,"w":2,"h":1,"src":reference,"imageEditState":{"flippedX":true}}
    ],"links":[]})).unwrap();
    let prepared = std::thread::spawn(move || {
        let document = crate::session_document::decode(&bytes).unwrap();
        images(&document, &AtomicBool::new(false), |_| panic!("no files")).unwrap()
    })
    .join()
    .unwrap();
    let source = prepared[0].source.as_ref().unwrap();
    assert_eq!(&source.pixels[..], &[40, 50, 60, 255, 10, 20, 30, 255]);
    assert_eq!(
        &prepared[0].rasterized_annotations.as_ref().unwrap().pixels[..],
        &[10, 20, 30, 255, 40, 50, 60, 255]
    );
    assert!(source
        .pixels
        .same_storage(&prepared[1].source.as_ref().unwrap().pixels));
}
#[test]
fn failure_or_cancel_does_not_return_partial_batch_or_fallback_to_original_file() {
    let bytes = document(&uri(), "missing-resource");
    let document = crate::session_document::decode(&bytes).unwrap();
    let mut opened = Vec::new();
    assert!(images(&document, &AtomicBool::new(false), |reference| {
        opened.push(reference.to_owned());
        Err(Error::new(E_INVALIDARG, "not admitted"))
    })
    .is_err());
    assert_eq!(opened, ["missing-resource"]);
    assert_eq!(
        images(&document, &AtomicBool::new(true), |_| panic!(
            "cancel must precede file opening"
        ))
        .err()
        .unwrap()
        .code(),
        E_ABORT
    );
}
#[test]
fn admitted_managed_file_is_opened_once_for_the_entire_batch() {
    use base64::Engine;
    let directory =
        std::path::PathBuf::from("C:/Users/Public/nas_home/AI/GameEditor/linshi").join(format!(
            "otter-session-batch-{}",
            super::super::document_state::new_unit_id()
                .unwrap()
                .as_str()
        ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("source.png");
    let image = uri();
    let (_, encoded) = image.split_once(',').unwrap();
    std::fs::write(
        &path,
        base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap(),
    )
    .unwrap();
    let bytes = document(path.to_str().unwrap(), path.to_str().unwrap());
    let policy = super::super::session_asset_root::Root::open(&directory).unwrap();
    let prepared = std::thread::spawn(move || {
        let document = crate::session_document::decode(&bytes).unwrap();
        let mut count = 0;
        let prepared = images(&document, &AtomicBool::new(false), |reference| {
            count += 1;
            policy.open_image(reference)
        })
        .unwrap();
        assert_eq!(count, 1);
        prepared
    })
    .join()
    .unwrap();
    assert_eq!(
        prepared[0].display().unwrap().pixels.as_ref(),
        &[10, 20, 30, 255]
    );
    assert!(prepared[0]
        .source
        .as_ref()
        .unwrap()
        .pixels
        .same_storage(&prepared[1].source.as_ref().unwrap().pixels));
    drop(prepared);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(directory).unwrap();
}
