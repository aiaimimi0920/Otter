//! 非对称像素通过真实读取/翻转、PNG 写出和再次读取，防止重复物化翻转。
use super::*;
use serde_json::json;

#[test]
fn saving_materialized_base_does_not_flip_it_again_or_flip_annotation_layer() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let cancel = AtomicBool::new(false);
    let pixels = vec![
        0, 0, 255, 255, 0, 255, 0, 255, 255, 0, 0, 255, 255, 255, 255, 255,
    ];
    let seed = Editor::with_layers(crate::erase_layers::Layers {
        size: (2, 2),
        base: pixels.clone().into(),
        annotation_size: None,
        annotations: None,
    })
    .unwrap();
    let seed = encode(&seed, &cancel)
        .unwrap()
        .unwrap()
        .into_data_urls(&cancel)
        .unwrap();
    for (x, y) in [(false, false), (true, false), (false, true), (true, true)] {
        let source = serde_json::to_vec(&json!({"stickers":[{"id":"one","x":0,"y":0,"w":2,"h":2,
            "src":seed.base.url.as_str(),"rasterizedAnnotationLayerSrc":seed.base.url.as_str(),
            "imageEditState":{"flippedX":x,"flippedY":y,"extension":123}}],"links":[]}))
        .unwrap();
        let first =
            crate::desktop::session_candidate::prepare(source.clone(), None, &cancel).unwrap();
        let base = first.images[0].source.as_ref().unwrap();
        let overlay = first.images[0].rasterized_annotations.as_ref().unwrap();
        assert_eq!(overlay.pixels.as_ref(), pixels.as_slice());
        if x || y {
            assert_ne!(base.pixels.as_ref(), pixels.as_slice());
        }
        let editor = Editor::with_layers(crate::erase_layers::Layers {
            size: (base.width, base.height),
            base: base.pixels.clone(),
            annotation_size: Some((overlay.width, overlay.height)),
            annotations: Some(overlay.pixels.clone()),
        })
        .unwrap();
        let mut encoded = Some(
            encode(&editor, &cancel)
                .unwrap()
                .unwrap()
                .into_data_urls(&cancel)
                .unwrap(),
        );
        let topology = crate::workspace_session::Topology::capture(&first.workspace);
        let saved =
            crate::session_write::assemble_owned(&source, &topology, &cancel, |_, fields| {
                encoded.take().unwrap().install(fields)
            })
            .unwrap();
        let state: serde_json::Value = serde_json::from_slice(&saved.bytes).unwrap();
        assert_eq!(
            state["stickers"][0]["imageEditState"],
            json!({"flippedX":false,"flippedY":false,"extension":123})
        );
        let second =
            crate::desktop::session_candidate::prepare(saved.bytes.to_vec(), None, &cancel)
                .unwrap();
        assert_eq!(
            second.images[0].source.as_ref().unwrap().pixels,
            base.pixels
        );
        assert_eq!(
            second.images[0]
                .rasterized_annotations
                .as_ref()
                .unwrap()
                .pixels,
            overlay.pixels
        );
    }
}

#[test]
fn invalid_image_state_is_not_silently_replaced() {
    let editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (1, 1),
        base: vec![255; 4].into(),
        annotation_size: None,
        annotations: None,
    })
    .unwrap();
    let cancel = AtomicBool::new(false);
    let images = encode(&editor, &cancel)
        .unwrap()
        .unwrap()
        .into_data_urls(&cancel)
        .unwrap();
    let fields = json!({"imageEditState":[]}).as_object().unwrap().clone();
    assert!(matches!(
        images.install(fields),
        Err(crate::session_write::Error::Window)
    ));
}
