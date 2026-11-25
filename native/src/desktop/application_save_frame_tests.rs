//! Save reads the same mainline fields without baking styles into the base image; later restoration and workflows still compose only once.
use super::*;
use crate::{
    desktop::{session_binding::Bound, session_candidate},
    pin_frame_style::Style,
};
#[test]
fn style_save_restore_preserves_source_crop_extensions_and_source_resolution_bake() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let cancel = AtomicBool::new(false);
    let image = crate::desktop::image::Image {
        width: 40,
        height: 20,
        pixels: [255, 255, 255, 255].repeat(800).into(),
    };
    let png = crate::desktop::export::png_raw(&image, &cancel).unwrap();
    let url = crate::loom_base64::encode_png_data_url(&png, &cancel).unwrap();
    let original = serde_json::json!({"stickers":[{"id":"styled","x":10,"y":20,"w":40,"h":20,"src":url.as_str(),"imageEditState":{"future":{"keep":true},"borderWidth":1,"borderColor":"blue"}}],"links":[]});
    let expected =
        Style::read(&serde_json::json!({"borderWidth":4,"borderColor":"red","cornerRadius":5}))
            .unwrap();
    let mut document = doc("styled");
    document.frame.height = 20;
    document.frame_style = Some(expected);
    document.editor = Editor::with_layers(crate::erase_layers::Layers {
        size: (40, 20),
        base: image.pixels.clone(),
        annotations: None,
        annotation_size: None,
    })
    .unwrap();
    let mut snapshot = data(vec![document]);
    snapshot.original = serde_json::to_vec(&original).unwrap().into();
    let saved = snapshot.assemble(&cancel, plain).unwrap();
    let read = crate::session_document::decode(&saved.bytes).unwrap();
    let edit: Value =
        serde_json::from_str(read.windows[0].image_edit_state.unwrap().get()).unwrap();
    assert_eq!(edit["future"]["keep"], true);
    assert_eq!(Style::read(&edit).unwrap(), expected);
    let candidate = session_candidate::prepare(saved.bytes.to_vec(), None, &cancel).unwrap();
    let vectors = session_candidate::VectorCandidate::prepare(candidate, &cancel).unwrap();
    let baked = vectors.bake_source_png(&id("styled"), &cancel).unwrap();
    let baked = crate::desktop::export::decode_png(baked.to_vec());
    assert_eq!(baked.pixels[3], 0);
    assert_eq!(&baked.pixels[20 * 4..21 * 4], &[0, 0, 255, 255]);
    let prepared = Bound::prepare_for(vectors, None, &cancel)
        .ok()
        .unwrap()
        .into_parts_for(None, &cancel)
        .ok()
        .unwrap()
        .prepare_states(&cancel)
        .unwrap();
    let canvas = prepared.states[0].canvas.borrow();
    assert_eq!(canvas.frame_style, expected);
    assert!(canvas
        .editor
        .layers()
        .unwrap()
        .base
        .iter()
        .all(|p| *p == 255));
    assert!(!canvas.editor.has_edit_state());
}
#[test]
fn removing_frame_style_clears_only_owned_fields_and_retains_other_image_edit_state() {
    let mut document = doc("plain");
    document.frame_style = Some(Style::default());
    let mut snapshot = data(vec![document]);
    snapshot.original=br#"{"stickers":[{"id":"plain","x":10,"y":20,"w":40,"h":30,"imageEditState":{"borderWidth":4,"borderColor":"red","cornerRadius":20,"flippedX":true,"future":42}}],"links":[]}"#.to_vec().into();
    let saved = snapshot.assemble(&AtomicBool::new(false), plain).unwrap();
    let read = crate::session_document::decode(&saved.bytes).unwrap();
    let edit: Value =
        serde_json::from_str(read.windows[0].image_edit_state.unwrap().get()).unwrap();
    assert_eq!(edit["future"], 42);
    assert_eq!(edit["flippedX"], true);
    assert_eq!(edit["borderWidth"], 0.0);
    assert_eq!(edit["cornerRadius"], 0.0);
    assert!(edit.get("borderColor").is_none());
}
