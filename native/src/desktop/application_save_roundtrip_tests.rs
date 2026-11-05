//! 候选编码必须经过实际恢复与合成回读，而不只比 JSON 字段。
use super::*;
use crate::desktop::{
    image::Image,
    session_candidate::{self, VectorCandidate},
};
use crate::session_source::Source;
fn url(image: &Image, cancel: &AtomicBool) -> String {
    let png = crate::desktop::export::png_raw(image, cancel).unwrap();
    crate::loom_base64::encode_png_data_url(&png, cancel)
        .unwrap()
        .as_str()
        .to_owned()
}

#[test]
fn cropped_source_overlay_and_vectors_keep_composed_pixels_after_save_restore() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let cancel = AtomicBool::new(false);
    let base = Image {
        width: 8,
        height: 4,
        pixels: (0u8..32)
            .flat_map(|index| [index * 5, index * 3, index * 7, 255])
            .collect::<Vec<_>>()
            .into(),
    };
    let overlay = Image {
        width: 2,
        height: 2,
        pixels: vec![0, 0, 128, 128, 0, 128, 0, 128, 128, 0, 0, 128, 0, 0, 0, 0].into(),
    };
    let base_url = url(&base, &cancel);
    let overlay_url = url(&overlay, &cancel);
    for flip in [false, true] {
        let source = serde_json::to_vec(&serde_json::json!({"stickers":[{
            "id":"one","x":10,"y":20,"w":4,"h":2,"src":base_url,"rasterizedAnnotationLayerSrc":overlay_url,
            "imageEditState":{"cropRect":{"x":2,"y":1,"w":4,"h":2},"sourceSize":{"w":8,"h":4},"flippedX":flip},
            "annotationState":{"serialCounter":0,"elements":[{"id":"line","type":"line","zIndex":0,"points":[{"x":1,"y":1},{"x":3,"y":1}],"style":{"width":1,"color":"#fff"}}]}
        }],"links":[]})).unwrap();
        let first = VectorCandidate::prepare(
            session_candidate::prepare(source.clone(), None, &cancel).unwrap(),
            &cancel,
        )
        .unwrap();
        let editor = &first.vectors[0].1.editor;
        let source_image = first.candidate.images[0].source.as_ref().unwrap();
        let before =
            crate::desktop::display_composition::compose(source_image, editor, &cancel).unwrap();
        let mut doc = super::tests::doc("one");
        doc.editor = editor.export_snapshot();
        doc.frame = crate::pin_geometry::PinRect {
            x: 10,
            y: 20,
            width: 4,
            height: 2,
        };
        let frame = doc.editor.visible_frame().unwrap();
        doc.viewport = crate::viewport::Viewport {
            scale: 1.0,
            origin: crate::viewport::Point {
                x: -frame.x as f32,
                y: -frame.y as f32,
            },
        };
        doc.restored_source = Some(source_image.clone());
        let mut data = super::tests::data(vec![doc]);
        data.original = Source::from(source);
        let saved = data.encode(&cancel).unwrap();
        let second = VectorCandidate::prepare(
            session_candidate::prepare(saved.bytes.to_vec(), None, &cancel).unwrap(),
            &cancel,
        )
        .unwrap();
        let restored = &second.vectors[0].1.editor;
        let after = crate::desktop::display_composition::compose(
            second.candidate.images[0].source.as_ref().unwrap(),
            restored,
            &cancel,
        )
        .unwrap();
        assert_eq!((before.width, before.height), (4, 2));
        assert_eq!((after.width, after.height), (4, 2));
        assert_eq!(before.pixels, after.pixels, "flip={flip}");
        assert_eq!(
            editor.layers().unwrap().annotations,
            restored.layers().unwrap().annotations
        );
        assert_eq!(editor.elements()[0].points, restored.elements()[0].points);
        assert_eq!(
            editor.elements()[0].annotation_id(),
            restored.elements()[0].annotation_id()
        );
    }
}
